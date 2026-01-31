use crate::binding::{Binding, InputRef};
use crate::button_renderer::{button_size_for_kind, encoder_lcd_size_for_kind, ButtonRenderer, LcdRenderer};
use crate::capability::{Capability, KeyLightAction, KEY_LIGHT_BRIGHTNESS_STEP};
use crate::device::DeviceInfo;
use crate::elgato_key_light;
use crate::events::{ConnectionStatusEvent, PageChangeEvent};
use crate::input_processor::{detect_swipe_direction, InputProcessor, LogicalEvent, SwipeDirection};
use crate::key_light_controller::KeyLightController;
use crate::state_manager::{self, SystemState};
use anyhow::{Context, Result};
use elgato_streamdeck::{images::ImageRect, info::Kind, list_devices, StreamDeck, StreamDeckInput};
use hidapi::HidApi;
use std::collections::HashMap;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Global debounced Key Light controller
static KEY_LIGHT_CONTROLLER: OnceLock<KeyLightController> = OnceLock::new();

/// Get or initialize the Key Light controller
fn get_key_light_controller() -> &'static KeyLightController {
    KEY_LIGHT_CONTROLLER.get_or_init(KeyLightController::new)
}

/// Interval for checking device reconnection
const RECONNECT_INTERVAL: Duration = Duration::from_secs(2);

/// Timeout for reading input from Stream Deck (affects responsiveness)
const INPUT_POLL_TIMEOUT: Duration = Duration::from_millis(50);

/// Minimum interval between command executions (prevents rapid-fire from stuck buttons)
const MIN_COMMAND_INTERVAL: Duration = Duration::from_millis(200);

/// Track last execution time for rate limiting
static COMMAND_RATE_LIMITER: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

/// Get or initialize the rate limiter
fn get_rate_limiter() -> &'static Mutex<HashMap<String, Instant>> {
    COMMAND_RATE_LIMITER.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Check if a command can be executed (rate limiting)
/// Returns true if enough time has passed since the last execution
fn check_rate_limit(key: &str) -> bool {
    let rate_limiter = get_rate_limiter();
    let mut times = rate_limiter.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(last_time) = times.get(key) {
        if last_time.elapsed() < MIN_COMMAND_INTERVAL {
            return false; // Rate limited
        }
    }

    times.insert(key.to_string(), Instant::now());
    true
}

/// Flag to signal when button images need to be re-synced.
pub static SYNC_IMAGES_FLAG: AtomicBool = AtomicBool::new(false);

/// Request a sync of button images to hardware.
pub fn request_image_sync() {
    SYNC_IMAGES_FLAG.store(true, Ordering::SeqCst);
}

pub fn run(
    app: AppHandle,
    device_info_state: Arc<Mutex<Option<DeviceInfo>>>,
    bindings_state: Arc<Mutex<Vec<Binding>>>,
    system_state: Arc<Mutex<SystemState>>,
    current_page: Arc<Mutex<usize>>,
) -> Result<()> {
    // Outer loop handles connection/reconnection
    loop {
        // Try to connect to a Stream Deck
        let connection = try_connect(&app, &device_info_state);

        match connection {
            Ok((mut deck, device_kind, button_renderer, lcd_renderer)) => {
                // Initial image sync on connect
                {
                    let bindings = bindings_state.lock().ok();
                    let state = system_state.lock().ok();
                    let page = *current_page.lock().unwrap_or_else(|e| e.into_inner());
                    if let (Some(bindings), Some(state)) = (bindings, state) {
                        if let Some(ref renderer) = button_renderer {
                            sync_button_images(&mut deck, &bindings, renderer, device_kind, &state, page);
                        }
                        if let Some(ref renderer) = lcd_renderer {
                            sync_lcd_images(&mut deck, &bindings, renderer, device_kind, &state, page);
                        }
                    }
                }

                // Run the event loop until disconnected
                let disconnect_reason = run_event_loop(
                    &app,
                    &mut deck,
                    device_kind,
                    &button_renderer,
                    &lcd_renderer,
                    &bindings_state,
                    &system_state,
                    &current_page,
                );

                // Device disconnected
                eprintln!("Stream Deck disconnected: {}", disconnect_reason);
                emit_connection_status(&app, false, None);

                // Clear device info
                if let Ok(mut state) = device_info_state.lock() {
                    *state = None;
                }
            }
            Err(_) => {
                // No device found, wait before retrying
            }
        }

        // Wait before trying to reconnect
        std::thread::sleep(RECONNECT_INTERVAL);
    }
}

/// Attempt to connect to a Stream Deck device
fn try_connect(
    app: &AppHandle,
    device_info_state: &Arc<Mutex<Option<DeviceInfo>>>,
) -> Result<(StreamDeck, Kind, Option<ButtonRenderer>, Option<LcdRenderer>)> {
    let hid = HidApi::new().context("hid init failed")?;
    let devices = list_devices(&hid);

    if devices.is_empty() {
        anyhow::bail!("No Stream Deck found");
    }

    let (kind, serial) = &devices[0];
    let deck = StreamDeck::connect(&hid, *kind, serial)?;
    let device_kind = *kind;

    // Update device info state
    let info = DeviceInfo::from_kind(device_kind);
    {
        if let Ok(mut state) = device_info_state.lock() {
            *state = Some(info.clone());
        }
    }

    // Emit connection event
    emit_connection_status(app, true, Some(info.model.clone()));

    // Create renderers
    let button_renderer = match create_button_renderer(device_kind) {
        Ok(r) => Some(r),
        Err(e) => {
            eprintln!("Failed to create button renderer: {e}");
            None
        }
    };

    let lcd_renderer = match create_lcd_renderer(device_kind) {
        Ok(Some(r)) => Some(r),
        Ok(None) => None,
        Err(e) => {
            eprintln!("Failed to create LCD renderer: {e}");
            None
        }
    };

    Ok((deck, device_kind, button_renderer, lcd_renderer))
}

/// Run the main event loop, returns error message when disconnected
fn run_event_loop(
    app: &AppHandle,
    deck: &mut StreamDeck,
    device_kind: Kind,
    button_renderer: &Option<ButtonRenderer>,
    lcd_renderer: &Option<LcdRenderer>,
    bindings_state: &Arc<Mutex<Vec<Binding>>>,
    system_state: &Arc<Mutex<SystemState>>,
    current_page: &Arc<Mutex<usize>>,
) -> String {
    let mut processor = InputProcessor::default();

    loop {
        // Check for image sync requests
        if SYNC_IMAGES_FLAG.swap(false, Ordering::SeqCst) {
            let bindings = bindings_state.lock().ok();
            let state = system_state.lock().ok();
            let page = *current_page.lock().unwrap_or_else(|e| e.into_inner());
            if let (Some(bindings), Some(state)) = (bindings, state) {
                if let Some(ref renderer) = button_renderer {
                    sync_button_images(deck, &bindings, renderer, device_kind, &state, page);
                }
                if let Some(ref renderer) = lcd_renderer {
                    sync_lcd_images(deck, &bindings, renderer, device_kind, &state, page);
                }
            }
        }

        // Read input with timeout
        let input = match deck.read_input(Some(INPUT_POLL_TIMEOUT)) {
            Ok(input) => input,
            Err(e) => {
                // Device disconnected or error
                return format!("{}", e);
            }
        };

        // Get current bindings snapshot and page
        let bindings = bindings_state
            .lock()
            .ok()
            .map(|b| b.clone())
            .unwrap_or_default();
        let page = *current_page.lock().unwrap_or_else(|e| e.into_inner());

        // Filter bindings to current page for event handling
        let page_bindings: Vec<_> = bindings.iter().filter(|b| b.page == page).cloned().collect();

        match input {
            StreamDeckInput::ButtonStateChange(states) => {
                for event in processor.process_buttons(&states) {
                    emit_event(app, event.clone());
                    handle_logical_event(event, &page_bindings, system_state);
                }
            }

            StreamDeckInput::EncoderTwist(deltas) => {
                for event in processor.process_encoders(&deltas) {
                    emit_event(app, event.clone());
                    handle_logical_event(event, &page_bindings, system_state);
                }
            }

            StreamDeckInput::TouchScreenSwipe(start, end) => {
                let event = processor.process_swipe(start, end);
                emit_event(app, event.clone());

                #[cfg(debug_assertions)]
                eprintln!("Swipe detected: start={:?}, end={:?}", start, end);

                // Check for page navigation swipe
                if let Some(direction) = detect_swipe_direction(start, end) {
                    let max_binding_page = get_max_page(&bindings);
                    // Allow navigation to one empty page beyond the last page with bindings
                    // e.g., if bindings on pages 0,1 -> can navigate to 0, 1, 2 (empty)
                    let max_allowed_page = max_binding_page + 1;
                    let page_count = max_allowed_page + 1; // Total pages for display

                    #[cfg(debug_assertions)]
                    eprintln!("Swipe direction: {:?}, current_page={}, max_binding_page={}, max_allowed={}", direction, page, max_binding_page, max_allowed_page);

                    // Linear navigation - no wrapping
                    let new_page = match direction {
                        SwipeDirection::Left => {
                            // Swipe left = next page (capped at max)
                            if page >= max_allowed_page { page } else { page + 1 }
                        }
                        SwipeDirection::Right => {
                            // Swipe right = previous page (capped at 0)
                            if page == 0 { 0 } else { page - 1 }
                        }
                    };

                    #[cfg(debug_assertions)]
                    eprintln!("Page change: {} -> {}", page, new_page);

                    // Only update if page actually changed
                    if new_page != page {
                        if let Ok(mut p) = current_page.lock() {
                            *p = new_page;
                        }
                        emit_page_change(app, new_page, page_count);
                        request_image_sync();
                    }
                } else {
                    #[cfg(debug_assertions)]
                    eprintln!("Swipe too short or vertical, not a page change");
                    // Not a page navigation swipe, handle normally
                    handle_logical_event(event, &page_bindings, system_state);
                }
            }

            StreamDeckInput::EncoderStateChange(states) => {
                #[cfg(debug_assertions)]
                println!("RAW encoder state: {:?}", states);

                for event in processor.process_encoder_presses(&states) {
                    emit_event(app, event.clone());
                    handle_logical_event(event, &page_bindings, system_state);
                }
            }

            _ => {}
        }
    }
}

/// Get the maximum page number from bindings (0 if no bindings)
fn get_max_page(bindings: &[Binding]) -> usize {
    bindings.iter().map(|b| b.page).max().unwrap_or(0)
}

/// Emit page change event to frontend
fn emit_page_change(app: &AppHandle, page: usize, page_count: usize) {
    let _ = app.emit("streamdeck:page", PageChangeEvent { page, page_count });
}

/// Emit connection status event to frontend
fn emit_connection_status(app: &AppHandle, connected: bool, model: Option<String>) {
    let _ = app.emit(
        "streamdeck:connection",
        ConnectionStatusEvent { connected, model },
    );
}

/// Create a button renderer for the given device kind.
fn create_button_renderer(kind: Kind) -> Result<ButtonRenderer> {
    let (w, h) = button_size_for_kind(kind);
    ButtonRenderer::new(w, h)
}

/// Create an LCD renderer for the given device kind (if it has an LCD strip).
fn create_lcd_renderer(kind: Kind) -> Result<Option<LcdRenderer>> {
    match encoder_lcd_size_for_kind(kind) {
        Some((w, h)) => Ok(Some(LcdRenderer::new(w, h)?)),
        None => Ok(None),
    }
}

/// Get a unique key for a binding (used for toggle state tracking)
fn binding_key(binding: &Binding) -> String {
    let input_key = match &binding.input {
        InputRef::Button { index } => format!("btn:{}", index),
        InputRef::Encoder { index } => format!("enc:{}", index),
        InputRef::EncoderPress { index } => format!("encp:{}", index),
        InputRef::Swipe => "swipe".to_string(),
    };
    format!("{}:{}", input_key, binding.page)
}

/// Flip the toggle state for a binding and request image sync
fn flip_toggle_state(binding: &Binding, system_state: &Arc<Mutex<SystemState>>) {
    let key = binding_key(binding);
    if let Ok(mut state) = system_state.lock() {
        let current = state.toggle_states.get(&key).copied().unwrap_or(false);
        state.toggle_states.insert(key, !current);
    }
    request_image_sync();
}

/// Get the effective image for a binding based on current system state.
/// Returns (image_to_use, has_image)
fn get_effective_image<'a>(binding: &'a Binding, state: &SystemState) -> Option<&'a str> {
    // Check if this capability has an "active" state
    let is_active = match &binding.capability {
        Capability::SystemAudio { .. } | Capability::Mute => state.is_muted,
        Capability::Microphone { .. } | Capability::MicMute => state.is_mic_muted,
        Capability::MediaPlayPause => state.is_playing,
        Capability::ElgatoKeyLight { ip, port, .. } => {
            // Check if key light is on
            state.key_lights.get(&format!("{}:{}", ip, port))
                .map(|s| s.on)
                .unwrap_or(false)
        }
        Capability::RunCommand { toggle: true, .. } => {
            // Check toggle state for this binding
            let key = binding_key(binding);
            state.toggle_states.get(&key).copied().unwrap_or(false)
        }
        _ => false,
    };

    // If active and we have an alt image, use it
    if is_active {
        if let Some(ref alt) = binding.button_image_alt {
            return Some(alt.as_str());
        }
    }

    // Otherwise use the default image
    binding.button_image.as_deref()
}

/// Sync all button images from bindings to hardware.
fn sync_button_images(
    deck: &mut StreamDeck,
    bindings: &[Binding],
    renderer: &ButtonRenderer,
    kind: Kind,
    state: &SystemState,
    current_page: usize,
) {
    let button_count = kind.key_count();

    // Track which buttons have been set
    let mut buttons_set = vec![false; button_count as usize];

    // Filter to current page
    for binding in bindings.iter().filter(|b| b.page == current_page) {
        if let InputRef::Button { index } = &binding.input {
            let key = *index as u8;
            if key >= button_count {
                continue;
            }

            // Get effective image based on state
            let effective_image = get_effective_image(binding, state);

            // Create a modified binding with the effective image for rendering
            let render_binding = Binding {
                input: binding.input.clone(),
                capability: binding.capability.clone(),
                page: binding.page,
                icon: binding.icon.clone(),
                label: binding.label.clone(),
                button_image: effective_image.map(String::from),
                button_image_alt: None, // Not needed for rendering
                show_label: binding.show_label,
            };

            match renderer.render_binding(&render_binding) {
                Ok(Some(img)) => {
                    if let Err(e) = deck.set_button_image(key, img) {
                        eprintln!("Failed to set button {key} image: {e}");
                    } else {
                        buttons_set[*index] = true;
                    }
                }
                Ok(None) => {
                    // No button_image configured, clear the button
                    if let Err(e) = deck.clear_button_image(key) {
                        eprintln!("Failed to clear button {key}: {e}");
                    }
                    buttons_set[*index] = true;
                }
                Err(e) => {
                    eprintln!("Failed to render button {key} image: {e}");
                }
            }
        }
    }

    // Clear buttons that don't have bindings
    for (index, was_set) in buttons_set.iter().enumerate() {
        if !was_set {
            let key = index as u8;
            if let Err(e) = deck.clear_button_image(key) {
                eprintln!("Failed to clear unbound button {key}: {e}");
            }
        }
    }

    // Flush all changes to device
    if let Err(e) = deck.flush() {
        eprintln!("Failed to flush button images: {e}");
    }
}

/// Sync encoder images to the LCD strip.
/// Priority: EncoderPress image > Encoder (rotation) image > empty
fn sync_lcd_images(
    deck: &mut StreamDeck,
    bindings: &[Binding],
    renderer: &LcdRenderer,
    kind: Kind,
    state: &SystemState,
    current_page: usize,
) {
    let encoder_count = kind.encoder_count();
    if encoder_count == 0 {
        return;
    }

    let Some((section_w, _section_h)) = encoder_lcd_size_for_kind(kind) else {
        return;
    };

    // Filter to current page
    let page_bindings: Vec<_> = bindings.iter().filter(|b| b.page == current_page).collect();

    for encoder_idx in 0..encoder_count {
        // Find the EncoderPress binding for this encoder (primary)
        let press_binding = page_bindings.iter().find(|b| {
            matches!(&b.input, InputRef::EncoderPress { index } if *index == encoder_idx as usize)
        }).copied();

        // Find the Encoder (rotation) binding as fallback
        let rotate_binding = page_bindings.iter().find(|b| {
            matches!(&b.input, InputRef::Encoder { index } if *index == encoder_idx as usize)
        }).copied();

        // Calculate X position for this encoder section
        let x = (encoder_idx as u32 * section_w) as u16;

        // Determine which binding has an image (considering state)
        let (binding_to_use, effective_image) = {
            // Try press binding first
            if let Some(b) = press_binding {
                let img = get_effective_image(b, state);
                if img.is_some() {
                    (Some(b), img)
                } else if let Some(rb) = rotate_binding {
                    let rimg = get_effective_image(rb, state);
                    (Some(rb), rimg)
                } else {
                    (None, None)
                }
            } else if let Some(rb) = rotate_binding {
                let rimg = get_effective_image(rb, state);
                (Some(rb), rimg)
            } else {
                (None, None)
            }
        };

        match (binding_to_use, effective_image) {
            (Some(binding), Some(img_path)) => {
                // Create a modified binding with the effective image for rendering
                let render_binding = Binding {
                    input: binding.input.clone(),
                    capability: binding.capability.clone(),
                    page: binding.page,
                    icon: binding.icon.clone(),
                    label: binding.label.clone(),
                    button_image: Some(img_path.to_string()),
                    button_image_alt: None,
                    show_label: binding.show_label,
                };

                match renderer.render_binding(&render_binding) {
                    Ok(Some(img)) => {
                        match ImageRect::from_image(img) {
                            Ok(rect) => {
                                if let Err(e) = deck.write_lcd(x, 0, &rect) {
                                    eprintln!("Failed to write LCD for encoder {encoder_idx}: {e}");
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to convert image to LCD rect for encoder {encoder_idx}: {e}");
                            }
                        }
                    }
                    Ok(None) => {
                        let empty = renderer.create_empty();
                        if let Ok(rect) = ImageRect::from_image(empty) {
                            let _ = deck.write_lcd(x, 0, &rect);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to render LCD image for encoder {encoder_idx}: {e}");
                    }
                }
            }
            _ => {
                // No binding with image, write empty section
                let empty = renderer.create_empty();
                if let Ok(rect) = ImageRect::from_image(empty) {
                    let _ = deck.write_lcd(x, 0, &rect);
                }
            }
        }
    }
}

fn handle_logical_event(event: LogicalEvent, bindings: &[Binding], system_state: &Arc<Mutex<SystemState>>) {
    for binding in bindings {
        if !binding.matches(&event) {
            continue;
        }

        match (&binding.capability, &event) {
            // SystemAudio: encoder rotation = volume, encoder press = mute
            (Capability::SystemAudio { .. }, LogicalEvent::EncoderPress(e)) if e.pressed => {
                toggle_mute();
                state_manager::request_state_check();
            }

            (Capability::SystemAudio { step }, LogicalEvent::Encoder(e)) => {
                apply_volume_delta(e.delta as f32 * step);
            }

            // Mute toggle (for buttons)
            (Capability::Mute, LogicalEvent::Button(e)) if e.pressed => {
                toggle_mute();
                state_manager::request_state_check();
            }

            (Capability::Mute, LogicalEvent::EncoderPress(e)) if e.pressed => {
                toggle_mute();
                state_manager::request_state_check();
            }

            // Volume Up (for buttons)
            (Capability::VolumeUp { step }, LogicalEvent::Button(e)) if e.pressed => {
                apply_volume_delta(*step);
            }

            (Capability::VolumeUp { step }, LogicalEvent::EncoderPress(e)) if e.pressed => {
                apply_volume_delta(*step);
            }

            // Volume Down (for buttons)
            (Capability::VolumeDown { step }, LogicalEvent::Button(e)) if e.pressed => {
                apply_volume_delta(-*step);
            }

            (Capability::VolumeDown { step }, LogicalEvent::EncoderPress(e)) if e.pressed => {
                apply_volume_delta(-*step);
            }

            // Microphone: encoder rotation = volume, encoder press = mute
            (Capability::Microphone { .. }, LogicalEvent::EncoderPress(e)) if e.pressed => {
                toggle_mic_mute();
                state_manager::request_state_check();
            }

            (Capability::Microphone { step }, LogicalEvent::Encoder(e)) => {
                apply_mic_volume_delta(e.delta as f32 * step);
            }

            // Mic Mute toggle (for buttons)
            (Capability::MicMute, LogicalEvent::Button(e)) if e.pressed => {
                toggle_mic_mute();
                state_manager::request_state_check();
            }

            (Capability::MicMute, LogicalEvent::EncoderPress(e)) if e.pressed => {
                toggle_mic_mute();
                state_manager::request_state_check();
            }

            // Mic Volume Up (for buttons)
            (Capability::MicVolumeUp { step }, LogicalEvent::Button(e)) if e.pressed => {
                apply_mic_volume_delta(*step);
            }

            (Capability::MicVolumeUp { step }, LogicalEvent::EncoderPress(e)) if e.pressed => {
                apply_mic_volume_delta(*step);
            }

            // Mic Volume Down (for buttons)
            (Capability::MicVolumeDown { step }, LogicalEvent::Button(e)) if e.pressed => {
                apply_mic_volume_delta(-*step);
            }

            (Capability::MicVolumeDown { step }, LogicalEvent::EncoderPress(e)) if e.pressed => {
                apply_mic_volume_delta(-*step);
            }

            (Capability::MediaPlayPause, LogicalEvent::EncoderPress(e)) if e.pressed => {
                media_play_pause();
                state_manager::request_state_check();
            }

            (Capability::MediaPlayPause, LogicalEvent::Button(e)) if e.pressed => {
                media_play_pause();
                state_manager::request_state_check();
            }

            (Capability::MediaNext, LogicalEvent::EncoderPress(e)) if e.pressed => {
                media_next();
            }

            (Capability::MediaNext, LogicalEvent::Button(e)) if e.pressed => {
                media_next();
            }

            (Capability::MediaPrevious, LogicalEvent::EncoderPress(e)) if e.pressed => {
                media_previous();
            }

            (Capability::MediaPrevious, LogicalEvent::Button(e)) if e.pressed => {
                media_previous();
            }

            (Capability::MediaStop, LogicalEvent::EncoderPress(e)) if e.pressed => {
                media_stop();
            }

            (Capability::MediaStop, LogicalEvent::Button(e)) if e.pressed => {
                media_stop();
            }

            (Capability::RunCommand { command, toggle }, LogicalEvent::EncoderPress(e)) if e.pressed => {
                run_shell_command(command);
                if *toggle {
                    flip_toggle_state(binding, system_state);
                }
            }

            (Capability::RunCommand { command, toggle }, LogicalEvent::Button(e)) if e.pressed => {
                run_shell_command(command);
                if *toggle {
                    flip_toggle_state(binding, system_state);
                }
            }

            (Capability::LaunchApp { command }, LogicalEvent::EncoderPress(e)) if e.pressed => {
                launch_app(command);
            }

            (Capability::LaunchApp { command }, LogicalEvent::Button(e)) if e.pressed => {
                launch_app(command);
            }

            (Capability::OpenURL { url }, LogicalEvent::EncoderPress(e)) if e.pressed => {
                open_url(url);
            }

            (Capability::OpenURL { url }, LogicalEvent::Button(e)) if e.pressed => {
                open_url(url);
            }

            // Elgato Key Light controls
            // Button press -> toggle
            (Capability::ElgatoKeyLight { ip, port, .. }, LogicalEvent::Button(e)) if e.pressed => {
                handle_key_light_button(ip, *port, &KeyLightAction::Toggle, system_state);
            }

            // Encoder press -> toggle
            (Capability::ElgatoKeyLight { ip, port, .. }, LogicalEvent::EncoderPress(e)) if e.pressed => {
                handle_key_light_button(ip, *port, &KeyLightAction::Toggle, system_state);
            }

            // Encoder rotation -> brightness
            (Capability::ElgatoKeyLight { ip, port, .. }, LogicalEvent::Encoder(e)) => {
                handle_key_light_brightness(ip, *port, e.delta, system_state);
            }

            _ => {}
        }
    }
}

fn handle_key_light_button(ip: &str, port: u16, action: &KeyLightAction, system_state: &Arc<Mutex<SystemState>>) {
    // Spawn background thread to avoid blocking the event loop
    let ip = ip.to_string();
    let port = port;
    let action = action.clone();
    let state = Arc::clone(system_state);

    std::thread::spawn(move || {
        let result = match action {
            KeyLightAction::Toggle => elgato_key_light::toggle(&ip, port).map(|_| ()),
            KeyLightAction::On => elgato_key_light::turn_on(&ip, port),
            KeyLightAction::Off => elgato_key_light::turn_off(&ip, port),
            KeyLightAction::SetBrightness => Ok(()), // Handled by encoder
        };

        if let Err(e) = result {
            eprintln!("Key Light error: {e}");
        }

        // Update key light state and trigger image sync
        match elgato_key_light::get_state(&ip, port) {
            Ok(light_state) => {
                // Update system state
                if let Ok(mut s) = state.lock() {
                    let key = format!("{}:{}", ip, port);
                    s.key_lights.insert(key, light_state.clone());
                }

                // Update controller's cache so brightness adjustments have accurate state
                get_key_light_controller().update_cached_state(&ip, port, &light_state);
            }
            Err(e) => {
                eprintln!("Failed to fetch Key Light state after action: {}", e);
            }
        }

        // Request image sync to update hardware display
        request_image_sync();
    });
}

fn handle_key_light_brightness(ip: &str, port: u16, delta: i8, _system_state: &Arc<Mutex<SystemState>>) {
    let brightness_delta = delta as i32 * KEY_LIGHT_BRIGHTNESS_STEP;

    // Queue the adjustment - will be debounced and sent in batch
    get_key_light_controller().queue_brightness_delta(ip, port, brightness_delta);

    // Note: State sync happens after debounce in the controller's background thread
    // We don't block the event loop waiting for HTTP responses
}

fn get_current_volume() -> Option<f32> {
    let output = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Expected: "Volume: 0.42"
    stdout
        .split_whitespace()
        .find_map(|word| word.parse::<f32>().ok())
}

fn apply_volume_delta(delta: f32) {
    // Read current volume
    let current = get_current_volume().unwrap_or(0.5);

    // Apply + clamp
    let new_volume = (current + delta).clamp(0.0, 1.0);

    let arg = format!("{:.3}", new_volume);

    let result = Command::new("wpctl")
        .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &arg])
        .status();

    if let Err(err) = result {
        eprintln!("Failed to set volume: {err}");
    }
}

fn toggle_mute() {
    if let Err(e) = Command::new("wpctl")
        .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
        .status()
    {
        eprintln!("Failed to toggle mute (is wpctl installed?): {}", e);
    }
}

fn toggle_mic_mute() {
    if let Err(e) = Command::new("wpctl")
        .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])
        .status()
    {
        eprintln!("Failed to toggle mic mute (is wpctl installed?): {}", e);
    }
}

fn get_current_mic_volume() -> Option<f32> {
    let output = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SOURCE@"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Expected: "Volume: 0.42" or "Volume: 0.42 [MUTED]"
    stdout
        .split_whitespace()
        .find_map(|word| word.parse::<f32>().ok())
}

fn apply_mic_volume_delta(delta: f32) {
    // Read current volume
    let current = get_current_mic_volume().unwrap_or(0.5);

    // Apply + clamp
    let new_volume = (current + delta).clamp(0.0, 1.0);

    let arg = format!("{:.3}", new_volume);

    let result = Command::new("wpctl")
        .args(["set-volume", "@DEFAULT_AUDIO_SOURCE@", &arg])
        .status();

    if let Err(err) = result {
        eprintln!("Failed to set mic volume: {err}");
    }
}

fn media_play_pause() {
    if let Err(e) = Command::new("playerctl").arg("play-pause").status() {
        eprintln!("Failed to play/pause media (is playerctl installed?): {}", e);
    }
}

fn media_next() {
    if let Err(e) = Command::new("playerctl").arg("next").status() {
        eprintln!("Failed to skip to next track: {}", e);
    }
}

fn media_previous() {
    if let Err(e) = Command::new("playerctl").arg("previous").status() {
        eprintln!("Failed to go to previous track: {}", e);
    }
}

fn media_stop() {
    if let Err(e) = Command::new("playerctl").arg("stop").status() {
        eprintln!("Failed to stop media: {}", e);
    }
}

fn run_shell_command(cmd: &str) {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        eprintln!("Warning: Attempted to run empty command");
        return;
    }

    // Rate limit to prevent rapid-fire execution
    let rate_key = format!("cmd:{}", cmd);
    if !check_rate_limit(&rate_key) {
        #[cfg(debug_assertions)]
        eprintln!("Rate limited: {}", cmd);
        return;
    }

    #[cfg(debug_assertions)]
    eprintln!("Executing shell command: {}", cmd);

    match Command::new("sh").args(["-c", cmd]).spawn() {
        Ok(_) => {}
        Err(e) => eprintln!("Failed to execute command '{}': {}", cmd, e),
    }
}

fn launch_app(app: &str) {
    let app = app.trim();
    if app.is_empty() {
        eprintln!("Warning: Attempted to launch empty application name");
        return;
    }

    // Basic validation: reject paths with suspicious patterns
    if app.contains("..") || (app.starts_with('/') && app.contains(' ')) {
        eprintln!("Warning: Suspicious application path rejected: {}", app);
        return;
    }

    // Rate limit to prevent rapid-fire execution
    let rate_key = format!("app:{}", app);
    if !check_rate_limit(&rate_key) {
        #[cfg(debug_assertions)]
        eprintln!("Rate limited: {}", app);
        return;
    }

    #[cfg(debug_assertions)]
    eprintln!("Launching application: {}", app);

    match Command::new(app).spawn() {
        Ok(_) => {}
        Err(e) => eprintln!("Failed to launch '{}': {}", app, e),
    }
}

fn open_url(url: &str) {
    let url = url.trim();
    if url.is_empty() {
        eprintln!("Warning: Attempted to open empty URL");
        return;
    }

    // Validate URL scheme - only allow http, https, and common safe schemes
    let valid_schemes = ["http://", "https://", "mailto:", "tel:"];
    if !valid_schemes.iter().any(|scheme| url.starts_with(scheme)) {
        eprintln!("Warning: Rejected URL with unsupported scheme: {}", url);
        return;
    }

    // Rate limit to prevent rapid-fire execution
    let rate_key = format!("url:{}", url);
    if !check_rate_limit(&rate_key) {
        #[cfg(debug_assertions)]
        eprintln!("Rate limited: {}", url);
        return;
    }

    #[cfg(debug_assertions)]
    eprintln!("Opening URL: {}", url);

    match Command::new("xdg-open").arg(url).spawn() {
        Ok(_) => {}
        Err(e) => eprintln!("Failed to open URL '{}': {}", url, e),
    }
}

fn emit_event(app: &AppHandle, event: LogicalEvent) {
    match event {
        LogicalEvent::Button(e) => {
            app.emit("streamdeck:button", e).ok();
        }
        LogicalEvent::Encoder(e) => {
            app.emit("streamdeck:encoder", e).ok();
        }
        LogicalEvent::EncoderPress(e) => {
            app.emit("streamdeck:encoder-press", e).ok();
        }
        LogicalEvent::Swipe(e) => {
            app.emit("streamdeck:swipe", e).ok();
        }
    }
}
