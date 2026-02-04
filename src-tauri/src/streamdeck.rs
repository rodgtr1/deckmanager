use crate::binding::{Binding, InputRef};
use crate::button_renderer::{button_size_for_kind, encoder_lcd_size_for_kind, ButtonRenderer, LcdRenderer};
use crate::device::DeviceInfo;
use crate::events::{ConnectionStatusEvent, PageChangeEvent};
use crate::hotplug;
use crate::input_processor::{detect_swipe_direction, InputProcessor, LogicalEvent, SwipeDirection};
use crate::plugin::PluginRegistry;
use crate::state_manager::SystemState;
use anyhow::{Context, Result};
use elgato_streamdeck::{images::ImageRect, info::Kind, list_devices, StreamDeck, StreamDeckInput};
use hidapi::HidApi;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Interval for checking device reconnection when polling
const RECONNECT_POLL_INTERVAL: Duration = Duration::from_millis(100);
/// Maximum time to wait before retrying connection
const RECONNECT_MAX_WAIT: Duration = Duration::from_secs(2);

/// Timeout for reading input from Stream Deck (affects responsiveness)
const INPUT_POLL_TIMEOUT: Duration = Duration::from_millis(50);

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
    plugin_registry: Arc<PluginRegistry>,
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
                            sync_button_images(&mut deck, &bindings, renderer, device_kind, &state, page, &plugin_registry);
                        }
                        if let Some(ref renderer) = lcd_renderer {
                            sync_lcd_images(&mut deck, &bindings, renderer, device_kind, &state, page, &plugin_registry);
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
                    &plugin_registry,
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

        // Wait before trying to reconnect, but check hotplug flag frequently
        // to respond quickly when a device is connected
        wait_for_device_or_timeout();
    }
}

/// Wait for either a hotplug event or timeout.
/// This allows us to respond quickly when a device is connected via hotplug
/// instead of waiting the full reconnect interval.
fn wait_for_device_or_timeout() {
    let mut elapsed = Duration::ZERO;
    while elapsed < RECONNECT_MAX_WAIT {
        // Check if hotplug detected a device connection
        if hotplug::check_device_connected() {
            eprintln!("Hotplug detected device, attempting immediate reconnection...");
            return;
        }
        std::thread::sleep(RECONNECT_POLL_INTERVAL);
        elapsed += RECONNECT_POLL_INTERVAL;
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
    plugin_registry: &Arc<PluginRegistry>,
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
                    sync_button_images(deck, &bindings, renderer, device_kind, &state, page, plugin_registry);
                }
                if let Some(ref renderer) = lcd_renderer {
                    sync_lcd_images(deck, &bindings, renderer, device_kind, &state, page, plugin_registry);
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

        // Get current page first (quick lock)
        let page = *current_page.lock().unwrap_or_else(|e| e.into_inner());

        // Get bindings for current page only (avoids cloning entire vector)
        // Only clone bindings that match the current page
        let page_bindings: Vec<Binding> = bindings_state
            .lock()
            .ok()
            .map(|b| b.iter().filter(|binding| binding.page == page).cloned().collect())
            .unwrap_or_default();

        match input {
            StreamDeckInput::ButtonStateChange(states) => {
                for event in processor.process_buttons(&states) {
                    emit_event(app, event.clone());
                    handle_logical_event(event, &page_bindings, system_state, plugin_registry);
                }
            }

            StreamDeckInput::EncoderTwist(deltas) => {
                for event in processor.process_encoders(&deltas) {
                    emit_event(app, event.clone());
                    handle_logical_event(event, &page_bindings, system_state, plugin_registry);
                }
            }

            StreamDeckInput::TouchScreenSwipe(start, end) => {
                let event = processor.process_swipe(start, end);
                emit_event(app, event.clone());

                #[cfg(debug_assertions)]
                eprintln!("Swipe detected: start={:?}, end={:?}", start, end);

                // Check for page navigation swipe
                if let Some(direction) = detect_swipe_direction(start, end) {
                    // Need all bindings for max page calculation (not just current page)
                    let max_binding_page = bindings_state
                        .lock()
                        .ok()
                        .map(|b| get_max_page(&b))
                        .unwrap_or(0);
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
                    handle_logical_event(event, &page_bindings, system_state, plugin_registry);
                }
            }

            StreamDeckInput::EncoderStateChange(states) => {
                #[cfg(debug_assertions)]
                println!("RAW encoder state: {:?}", states);

                for event in processor.process_encoder_presses(&states) {
                    emit_event(app, event.clone());
                    handle_logical_event(event, &page_bindings, system_state, plugin_registry);
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

/// Get the effective image and color for a binding based on current system state.
/// Uses the plugin registry to determine active state.
/// Returns (image_path, icon_color) - using alt variants when in active state.
fn get_effective_image_and_color<'a>(
    binding: &'a Binding,
    state: &SystemState,
    registry: &PluginRegistry
) -> (Option<&'a str>, Option<&'a str>) {
    // Check if this capability has an "active" state via plugin
    let is_active = registry.is_binding_active(binding, state);

    // If active and we have an alt image, use alt image and alt color
    if is_active {
        if let Some(ref alt) = binding.button_image_alt {
            let color = binding.icon_color_alt.as_deref()
                .or(binding.icon_color.as_deref()); // Fall back to default color if no alt color
            return (Some(alt.as_str()), color);
        }
    }

    // Otherwise use the default image and default color
    (binding.button_image.as_deref(), binding.icon_color.as_deref())
}

/// Sync all button images from bindings to hardware.
fn sync_button_images(
    deck: &mut StreamDeck,
    bindings: &[Binding],
    renderer: &ButtonRenderer,
    kind: Kind,
    state: &SystemState,
    current_page: usize,
    registry: &PluginRegistry,
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

            // Get effective image and color based on state
            let (effective_image, effective_color) = get_effective_image_and_color(binding, state, registry);

            // Create a modified binding with the effective image and color for rendering
            let render_binding = Binding {
                input: binding.input.clone(),
                capability: binding.capability.clone(),
                page: binding.page,
                icon: binding.icon.clone(),
                label: binding.label.clone(),
                button_image: effective_image.map(String::from),
                button_image_alt: None, // Not needed for rendering
                show_label: binding.show_label,
                icon_color: effective_color.map(String::from),
                icon_color_alt: None, // Not needed for rendering
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
    registry: &PluginRegistry,
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
        let (binding_to_use, effective_image, effective_color) = {
            // Try press binding first
            if let Some(b) = press_binding {
                let (img, color) = get_effective_image_and_color(b, state, registry);
                if img.is_some() {
                    (Some(b), img, color)
                } else if let Some(rb) = rotate_binding {
                    let (rimg, rcolor) = get_effective_image_and_color(rb, state, registry);
                    (Some(rb), rimg, rcolor)
                } else {
                    (None, None, None)
                }
            } else if let Some(rb) = rotate_binding {
                let (rimg, rcolor) = get_effective_image_and_color(rb, state, registry);
                (Some(rb), rimg, rcolor)
            } else {
                (None, None, None)
            }
        };

        match (binding_to_use, effective_image) {
            (Some(binding), Some(img_path)) => {
                // Create a modified binding with the effective image and color for rendering
                let render_binding = Binding {
                    input: binding.input.clone(),
                    capability: binding.capability.clone(),
                    page: binding.page,
                    icon: binding.icon.clone(),
                    label: binding.label.clone(),
                    button_image: Some(img_path.to_string()),
                    button_image_alt: None,
                    show_label: binding.show_label,
                    icon_color: effective_color.map(String::from),
                    icon_color_alt: None,
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

/// Handle a logical event by dispatching to the plugin registry.
/// Plugin handlers are spawned in separate threads to avoid blocking the event loop
/// (important for network-dependent plugins like OBS/Elgato that may timeout).
fn handle_logical_event(
    event: LogicalEvent,
    bindings: &[Binding],
    system_state: &Arc<Mutex<SystemState>>,
    plugin_registry: &Arc<PluginRegistry>,
) {
    #[cfg(debug_assertions)]
    eprintln!("handle_logical_event: {:?}, {} bindings on page", event, bindings.len());

    for binding in bindings {
        if !binding.matches(&event) {
            continue;
        }

        #[cfg(debug_assertions)]
        eprintln!("  -> matched binding: {:?}", binding.capability);

        // Clone what we need for the spawned thread
        let event = event.clone();
        let binding = binding.clone();
        let system_state = Arc::clone(system_state);
        let plugin_registry = Arc::clone(plugin_registry);

        // Spawn handler in separate thread to avoid blocking event loop
        // This is critical for network-dependent plugins (OBS, Elgato) that may timeout
        std::thread::spawn(move || {
            let _handled = plugin_registry.handle_event(&event, &binding, &system_state);

            #[cfg(debug_assertions)]
            eprintln!("  -> handled: {}", _handled);
        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;

    #[test]
    fn test_get_max_page_empty() {
        let bindings: Vec<Binding> = vec![];
        assert_eq!(get_max_page(&bindings), 0);
    }

    #[test]
    fn test_get_max_page_single_page() {
        let bindings = vec![
            Binding {
                input: InputRef::Button { index: 0 },
                capability: Capability::MediaPlayPause,
                page: 0,
                icon: None,
                label: None,
                button_image: None,
                button_image_alt: None,
                show_label: None,
                icon_color: None,
                icon_color_alt: None,
            },
        ];
        assert_eq!(get_max_page(&bindings), 0);
    }

    #[test]
    fn test_get_max_page_multiple_pages() {
        let bindings = vec![
            Binding {
                input: InputRef::Button { index: 0 },
                capability: Capability::MediaPlayPause,
                page: 0,
                icon: None,
                label: None,
                button_image: None,
                button_image_alt: None,
                show_label: None,
                icon_color: None,
                icon_color_alt: None,
            },
            Binding {
                input: InputRef::Button { index: 1 },
                capability: Capability::MediaNext,
                page: 2,
                icon: None,
                label: None,
                button_image: None,
                button_image_alt: None,
                show_label: None,
                icon_color: None,
                icon_color_alt: None,
            },
            Binding {
                input: InputRef::Button { index: 2 },
                capability: Capability::MediaPrevious,
                page: 1,
                icon: None,
                label: None,
                button_image: None,
                button_image_alt: None,
                show_label: None,
                icon_color: None,
                icon_color_alt: None,
            },
        ];
        assert_eq!(get_max_page(&bindings), 2);
    }

    #[test]
    fn test_sync_images_flag_initial_state() {
        // Flag should be false initially (or after being consumed)
        SYNC_IMAGES_FLAG.store(false, Ordering::SeqCst);
        assert!(!SYNC_IMAGES_FLAG.load(Ordering::SeqCst));
    }

    #[test]
    fn test_request_image_sync_sets_flag() {
        SYNC_IMAGES_FLAG.store(false, Ordering::SeqCst);
        request_image_sync();
        assert!(SYNC_IMAGES_FLAG.load(Ordering::SeqCst));
        // Clean up
        SYNC_IMAGES_FLAG.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_sync_images_flag_swap_clears() {
        SYNC_IMAGES_FLAG.store(true, Ordering::SeqCst);
        let was_set = SYNC_IMAGES_FLAG.swap(false, Ordering::SeqCst);
        assert!(was_set);
        assert!(!SYNC_IMAGES_FLAG.load(Ordering::SeqCst));
    }

    #[test]
    fn test_reconnect_constants() {
        // Reconnect poll should be reasonably fast (50-500ms)
        assert!(RECONNECT_POLL_INTERVAL >= Duration::from_millis(50));
        assert!(RECONNECT_POLL_INTERVAL <= Duration::from_millis(500));

        // Max wait should be reasonable (1-10 seconds)
        assert!(RECONNECT_MAX_WAIT >= Duration::from_secs(1));
        assert!(RECONNECT_MAX_WAIT <= Duration::from_secs(10));
    }

    #[test]
    fn test_input_poll_timeout_constant() {
        // Poll timeout should be responsive (10-200ms)
        assert!(INPUT_POLL_TIMEOUT >= Duration::from_millis(10));
        assert!(INPUT_POLL_TIMEOUT <= Duration::from_millis(200));
    }
}
