use crate::binding::Binding;
use crate::capability::Capability;
use crate::config;
use crate::input_processor::{InputProcessor, LogicalEvent};
use anyhow::{Context, Result};
use elgato_streamdeck::{list_devices, StreamDeck, StreamDeckInput};
use hidapi::HidApi;
use std::process::Command;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub fn run(app: AppHandle) -> Result<()> {
    let hid = HidApi::new().context("hid init failed")?;
    let devices = list_devices(&hid);

    if devices.is_empty() {
        anyhow::bail!("No Stream Deck found");
    }

    let (kind, serial) = &devices[0];
    let deck = StreamDeck::connect(&hid, *kind, serial)?;

    let mut processor = InputProcessor::default();

    let bindings = config::load_bindings().context("Failed to load bindings")?;

    loop {
        let input = deck.read_input(Some(Duration::from_millis(50)))?;

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
