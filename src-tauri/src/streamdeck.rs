use crate::binding::{Binding, InputRef};
use crate::button_renderer::{button_size_for_kind, encoder_lcd_size_for_kind, ButtonRenderer, LcdRenderer};
use crate::capability::{Capability, KeyLightAction};
use crate::device::DeviceInfo;
use crate::elgato_key_light;
use crate::input_processor::{InputProcessor, LogicalEvent};
use crate::state_manager::{self, SystemState};
use anyhow::{Context, Result};
use elgato_streamdeck::{images::ImageRect, info::Kind, list_devices, StreamDeck, StreamDeckInput};
use hidapi::HidApi;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

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
) -> Result<()> {
    let hid = HidApi::new().context("hid init failed")?;
    let devices = list_devices(&hid);

    if devices.is_empty() {
        anyhow::bail!("No Stream Deck found");
    }

    let (kind, serial) = &devices[0];
    let mut deck = StreamDeck::connect(&hid, *kind, serial)?;
    let device_kind = *kind;

    // Capture device info into shared state
    {
        let info = DeviceInfo::from_kind(device_kind);
        if let Ok(mut state) = device_info_state.lock() {
            *state = Some(info);
        }
    }

    // Create button renderer
    let button_renderer = match create_button_renderer(device_kind) {
        Ok(r) => Some(r),
        Err(e) => {
            eprintln!("Failed to create button renderer: {e}");
            None
        }
    };

    // Create LCD renderer if device has LCD strip
    let lcd_renderer = match create_lcd_renderer(device_kind) {
        Ok(Some(r)) => Some(r),
        Ok(None) => None,
        Err(e) => {
            eprintln!("Failed to create LCD renderer: {e}");
            None
        }
    };

    // Initial image sync on startup
    {
        let bindings = bindings_state.lock().ok();
        let state = system_state.lock().ok();
        if let (Some(bindings), Some(state)) = (bindings, state) {
            if let Some(ref renderer) = button_renderer {
                sync_button_images(&mut deck, &bindings, renderer, device_kind, &state);
            }
            if let Some(ref renderer) = lcd_renderer {
                sync_lcd_images(&mut deck, &bindings, renderer, device_kind, &state);
            }
        }
    }

    let mut processor = InputProcessor::default();

    loop {
        // Check for image sync requests
        if SYNC_IMAGES_FLAG.swap(false, Ordering::SeqCst) {
            let bindings = bindings_state.lock().ok();
            let state = system_state.lock().ok();
            if let (Some(bindings), Some(state)) = (bindings, state) {
                if let Some(ref renderer) = button_renderer {
                    sync_button_images(&mut deck, &bindings, renderer, device_kind, &state);
                }
                if let Some(ref renderer) = lcd_renderer {
                    sync_lcd_images(&mut deck, &bindings, renderer, device_kind, &state);
                }
            }
        }

        let input = deck.read_input(Some(Duration::from_millis(50)))?;

        // Get current bindings snapshot for this iteration
        let bindings = bindings_state
            .lock()
            .ok()
            .map(|b| b.clone())
            .unwrap_or_default();

        match input {
            StreamDeckInput::ButtonStateChange(states) => {
                for event in processor.process_buttons(&states) {
                    emit_event(&app, event.clone());
                    handle_logical_event(event, &bindings);
                }
            }

            StreamDeckInput::EncoderTwist(deltas) => {
                for event in processor.process_encoders(&deltas) {
                    emit_event(&app, event.clone());
                    handle_logical_event(event, &bindings);
                }
            }

            StreamDeckInput::TouchScreenSwipe(start, end) => {
                let event = processor.process_swipe(start, end);
                emit_event(&app, event.clone());
                handle_logical_event(event, &bindings);
            }

            StreamDeckInput::EncoderStateChange(states) => {
                #[cfg(debug_assertions)]
                println!("RAW encoder state: {:?}", states);

                for event in processor.process_encoder_presses(&states) {
                    emit_event(&app, event.clone());
                    handle_logical_event(event, &bindings);
                }
            }

            _ => {}
        }
    }
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

/// Get the effective image for a binding based on current system state.
/// Returns (image_to_use, has_image)
fn get_effective_image<'a>(binding: &'a Binding, state: &SystemState) -> Option<&'a str> {
    // Check if this capability has an "active" state
    let is_active = match &binding.capability {
        Capability::ToggleMute => state.is_muted,
        Capability::MediaPlayPause => state.is_playing,
        Capability::ElgatoKeyLight { ip, port, .. } => {
            // Check if key light is on
            state.key_lights.get(&format!("{}:{}", ip, port))
                .map(|s| s.on)
                .unwrap_or(false)
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
) {
    let button_count = kind.key_count();

    // Track which buttons have been set
    let mut buttons_set = vec![false; button_count as usize];

    for binding in bindings {
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
) {
    let encoder_count = kind.encoder_count();
    if encoder_count == 0 {
        return;
    }

    let Some((section_w, _section_h)) = encoder_lcd_size_for_kind(kind) else {
        return;
    };

    for encoder_idx in 0..encoder_count {
        // Find the EncoderPress binding for this encoder (primary)
        let press_binding = bindings.iter().find(|b| {
            matches!(&b.input, InputRef::EncoderPress { index } if *index == encoder_idx as usize)
        });

        // Find the Encoder (rotation) binding as fallback
        let rotate_binding = bindings.iter().find(|b| {
            matches!(&b.input, InputRef::Encoder { index } if *index == encoder_idx as usize)
        });

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

fn handle_logical_event(event: LogicalEvent, bindings: &[Binding]) {
    for binding in bindings {
        if !binding.matches(&event) {
            continue;
        }

        match (&binding.capability, &event) {
            (Capability::ToggleMute, LogicalEvent::EncoderPress(e)) if e.pressed => {
                toggle_mute();
                // Request immediate state check to update icon
                state_manager::request_state_check();
            }

            (Capability::ToggleMute, LogicalEvent::Button(e)) if e.pressed => {
                toggle_mute();
                state_manager::request_state_check();
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

            (Capability::RunCommand { command }, LogicalEvent::EncoderPress(e)) if e.pressed => {
                run_shell_command(command);
            }

            (Capability::RunCommand { command }, LogicalEvent::Button(e)) if e.pressed => {
                run_shell_command(command);
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

            (Capability::SystemVolume { step }, LogicalEvent::Encoder(e)) => {
                apply_volume_delta(e.delta as f32 * step);
            }

            // Elgato Key Light controls
            (Capability::ElgatoKeyLight { ip, port, action }, LogicalEvent::Button(e)) if e.pressed => {
                handle_key_light_button(ip, *port, action);
                state_manager::request_state_check();
            }

            (Capability::ElgatoKeyLight { ip, port, action }, LogicalEvent::EncoderPress(e)) if e.pressed => {
                handle_key_light_button(ip, *port, action);
                state_manager::request_state_check();
            }

            (Capability::ElgatoKeyLight { ip, port, action: KeyLightAction::SetBrightness }, LogicalEvent::Encoder(e)) => {
                handle_key_light_brightness(ip, *port, e.delta);
                state_manager::request_state_check();
            }

            _ => {}
        }
    }
}

fn handle_key_light_button(ip: &str, port: u16, action: &KeyLightAction) {
    let result = match action {
        KeyLightAction::Toggle => elgato_key_light::toggle(ip, port).map(|_| ()),
        KeyLightAction::On => elgato_key_light::turn_on(ip, port),
        KeyLightAction::Off => elgato_key_light::turn_off(ip, port),
        KeyLightAction::SetBrightness => Ok(()), // Handled by encoder
    };

    if let Err(e) = result {
        eprintln!("Key Light error: {e}");
    }
}

fn handle_key_light_brightness(ip: &str, port: u16, delta: i8) {
    let brightness_delta = delta as i32 * 5; // 5% per tick
    if let Err(e) = elgato_key_light::adjust_brightness(ip, port, brightness_delta) {
        eprintln!("Key Light brightness error: {e}");
    }
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
    let _ = Command::new("wpctl")
        .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
        .status();
}

fn media_play_pause() {
    let _ = Command::new("playerctl").arg("play-pause").status();
}

fn media_next() {
    let _ = Command::new("playerctl").arg("next").status();
}

fn media_previous() {
    let _ = Command::new("playerctl").arg("previous").status();
}

fn media_stop() {
    let _ = Command::new("playerctl").arg("stop").status();
}

fn run_shell_command(cmd: &str) {
    let _ = Command::new("sh").args(["-c", cmd]).spawn();
}

fn launch_app(app: &str) {
    let _ = Command::new(app).spawn();
}

fn open_url(url: &str) {
    let _ = Command::new("xdg-open").arg(url).spawn();
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
