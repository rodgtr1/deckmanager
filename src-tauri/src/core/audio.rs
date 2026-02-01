//! Audio control capabilities: SystemAudio, Mute, Volume, Microphone.

use crate::binding::Binding;
use crate::capability::Capability;
use crate::input_processor::LogicalEvent;
use crate::plugin::{CapabilityMetadata, ParameterDef, ParameterType};
use crate::state_manager::{self, SystemState};
use std::process::Command;
use std::sync::{Arc, Mutex};

/// Get capability metadata for all audio capabilities.
pub fn capabilities() -> Vec<CapabilityMetadata> {
    vec![
        CapabilityMetadata {
            id: "SystemAudio",
            name: "System Audio",
            description: "Full audio control for encoders. Rotation: volume, Press: mute toggle",
            plugin_id: "core",
            supports_button: false,
            supports_encoder: true,
            supports_encoder_press: true,
            parameters: vec![ParameterDef {
                name: "step",
                param_type: ParameterType::Float,
                default_value: "0.02",
                description: "Volume change per encoder tick (0.0-1.0)",
            }],
        },
        CapabilityMetadata {
            id: "Mute",
            name: "Mute",
            description: "Toggle system audio mute on/off",
            plugin_id: "core",
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![],
        },
        CapabilityMetadata {
            id: "VolumeUp",
            name: "Volume Up",
            description: "Increase system volume",
            plugin_id: "core",
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![ParameterDef {
                name: "step",
                param_type: ParameterType::Float,
                default_value: "0.05",
                description: "Volume increase per press (0.0-1.0)",
            }],
        },
        CapabilityMetadata {
            id: "VolumeDown",
            name: "Volume Down",
            description: "Decrease system volume",
            plugin_id: "core",
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![ParameterDef {
                name: "step",
                param_type: ParameterType::Float,
                default_value: "0.05",
                description: "Volume decrease per press (0.0-1.0)",
            }],
        },
        CapabilityMetadata {
            id: "Microphone",
            name: "Microphone",
            description: "Full mic control for encoders. Rotation: volume, Press: mute toggle",
            plugin_id: "core",
            supports_button: false,
            supports_encoder: true,
            supports_encoder_press: true,
            parameters: vec![ParameterDef {
                name: "step",
                param_type: ParameterType::Float,
                default_value: "0.02",
                description: "Volume change per encoder tick (0.0-1.0)",
            }],
        },
        CapabilityMetadata {
            id: "MicMute",
            name: "Mic Mute",
            description: "Toggle microphone mute on/off",
            plugin_id: "core",
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![],
        },
        CapabilityMetadata {
            id: "MicVolumeUp",
            name: "Mic Volume Up",
            description: "Increase microphone volume",
            plugin_id: "core",
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![ParameterDef {
                name: "step",
                param_type: ParameterType::Float,
                default_value: "0.05",
                description: "Volume increase per press (0.0-1.0)",
            }],
        },
        CapabilityMetadata {
            id: "MicVolumeDown",
            name: "Mic Volume Down",
            description: "Decrease microphone volume",
            plugin_id: "core",
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![ParameterDef {
                name: "step",
                param_type: ParameterType::Float,
                default_value: "0.05",
                description: "Volume decrease per press (0.0-1.0)",
            }],
        },
    ]
}

/// Handle audio-related events.
///
/// Returns `true` if the event was handled.
pub fn handle_event(
    event: &LogicalEvent,
    binding: &Binding,
    _system_state: &Arc<Mutex<SystemState>>,
) -> bool {
    match (&binding.capability, event) {
        // SystemAudio: encoder rotation = volume, encoder press = mute
        (Capability::SystemAudio { .. }, LogicalEvent::EncoderPress(e)) if e.pressed => {
            toggle_mute();
            state_manager::request_state_check();
            true
        }

        (Capability::SystemAudio { step }, LogicalEvent::Encoder(e)) => {
            apply_volume_delta(e.delta as f32 * step);
            true
        }

        // Mute toggle (for buttons)
        (Capability::Mute, LogicalEvent::Button(e)) if e.pressed => {
            toggle_mute();
            state_manager::request_state_check();
            true
        }

        (Capability::Mute, LogicalEvent::EncoderPress(e)) if e.pressed => {
            toggle_mute();
            state_manager::request_state_check();
            true
        }

        // Volume Up (for buttons)
        (Capability::VolumeUp { step }, LogicalEvent::Button(e)) if e.pressed => {
            apply_volume_delta(*step);
            true
        }

        (Capability::VolumeUp { step }, LogicalEvent::EncoderPress(e)) if e.pressed => {
            apply_volume_delta(*step);
            true
        }

        // Volume Down (for buttons)
        (Capability::VolumeDown { step }, LogicalEvent::Button(e)) if e.pressed => {
            apply_volume_delta(-*step);
            true
        }

        (Capability::VolumeDown { step }, LogicalEvent::EncoderPress(e)) if e.pressed => {
            apply_volume_delta(-*step);
            true
        }

        // Microphone: encoder rotation = volume, encoder press = mute
        (Capability::Microphone { .. }, LogicalEvent::EncoderPress(e)) if e.pressed => {
            toggle_mic_mute();
            state_manager::request_state_check();
            true
        }

        (Capability::Microphone { step }, LogicalEvent::Encoder(e)) => {
            apply_mic_volume_delta(e.delta as f32 * step);
            true
        }

        // Mic Mute toggle (for buttons)
        (Capability::MicMute, LogicalEvent::Button(e)) if e.pressed => {
            toggle_mic_mute();
            state_manager::request_state_check();
            true
        }

        (Capability::MicMute, LogicalEvent::EncoderPress(e)) if e.pressed => {
            toggle_mic_mute();
            state_manager::request_state_check();
            true
        }

        // Mic Volume Up (for buttons)
        (Capability::MicVolumeUp { step }, LogicalEvent::Button(e)) if e.pressed => {
            apply_mic_volume_delta(*step);
            true
        }

        (Capability::MicVolumeUp { step }, LogicalEvent::EncoderPress(e)) if e.pressed => {
            apply_mic_volume_delta(*step);
            true
        }

        // Mic Volume Down (for buttons)
        (Capability::MicVolumeDown { step }, LogicalEvent::Button(e)) if e.pressed => {
            apply_mic_volume_delta(-*step);
            true
        }

        (Capability::MicVolumeDown { step }, LogicalEvent::EncoderPress(e)) if e.pressed => {
            apply_mic_volume_delta(-*step);
            true
        }

        _ => false,
    }
}

/// Check if an audio binding is in an active state.
pub fn is_active(binding: &Binding, state: &SystemState) -> bool {
    match &binding.capability {
        Capability::SystemAudio { .. } | Capability::Mute => state.is_muted,
        Capability::Microphone { .. } | Capability::MicMute => state.is_mic_muted,
        _ => false,
    }
}

// ─────────────────────────────────────────────────────────────────
// Audio control functions (using wpctl/PipeWire)
// ─────────────────────────────────────────────────────────────────

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
