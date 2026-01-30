use crate::binding::Binding;
use crate::capability::Capability;
use crate::device::DeviceInfo;
use crate::input_processor::{InputProcessor, LogicalEvent};
use anyhow::{Context, Result};
use elgato_streamdeck::{list_devices, StreamDeck, StreamDeckInput};
use hidapi::HidApi;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub fn run(
    app: AppHandle,
    device_info_state: Arc<Mutex<Option<DeviceInfo>>>,
    bindings_state: Arc<Mutex<Vec<Binding>>>,
) -> Result<()> {
    let hid = HidApi::new().context("hid init failed")?;
    let devices = list_devices(&hid);

    if devices.is_empty() {
        anyhow::bail!("No Stream Deck found");
    }

    let (kind, serial) = &devices[0];
    let deck = StreamDeck::connect(&hid, *kind, serial)?;

    // Capture device info into shared state
    {
        let info = DeviceInfo::from_kind(*kind);
        if let Ok(mut state) = device_info_state.lock() {
            *state = Some(info);
        }
    }

    let mut processor = InputProcessor::default();

    loop {
        let input = deck.read_input(Some(Duration::from_millis(50)))?;

        // Get current bindings snapshot for this iteration
        let bindings = bindings_state.lock().ok().map(|b| b.clone()).unwrap_or_default();

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

fn handle_logical_event(event: LogicalEvent, bindings: &[Binding]) {
    for binding in bindings {
        if !binding.matches(&event) {
            continue;
        }

        match (&binding.capability, &event) {
            (Capability::ToggleMute, LogicalEvent::EncoderPress(e)) if e.pressed => {
                toggle_mute();
            }

            (Capability::ToggleMute, LogicalEvent::Button(e)) if e.pressed => {
                toggle_mute();
            }

            (Capability::MediaPlayPause, LogicalEvent::EncoderPress(e)) if e.pressed => {
                media_play_pause();
            }

            (Capability::MediaPlayPause, LogicalEvent::Button(e)) if e.pressed => {
                media_play_pause();
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

            _ => {}
        }
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
