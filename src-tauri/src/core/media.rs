//! Media control capabilities: PlayPause, Next, Previous, Stop.

use crate::binding::Binding;
use crate::capability::Capability;
use crate::input_processor::LogicalEvent;
use crate::plugin::CapabilityMetadata;
use crate::state_manager::{self, SystemState};
use std::process::Command;
use std::sync::{Arc, Mutex};

/// Get capability metadata for all media capabilities.
pub fn capabilities() -> Vec<CapabilityMetadata> {
    vec![
        CapabilityMetadata {
            id: "MediaPlayPause",
            name: "Play/Pause",
            description: "Toggle media playback",
            plugin_id: "core",
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![],
        },
        CapabilityMetadata {
            id: "MediaNext",
            name: "Next Track",
            description: "Skip to next track",
            plugin_id: "core",
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![],
        },
        CapabilityMetadata {
            id: "MediaPrevious",
            name: "Previous Track",
            description: "Go to previous track",
            plugin_id: "core",
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![],
        },
        CapabilityMetadata {
            id: "MediaStop",
            name: "Stop",
            description: "Stop media playback",
            plugin_id: "core",
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![],
        },
    ]
}

/// Handle media-related events.
///
/// Returns `true` if the event was handled.
pub fn handle_event(
    event: &LogicalEvent,
    binding: &Binding,
    _system_state: &Arc<Mutex<SystemState>>,
) -> bool {
    match (&binding.capability, event) {
        (Capability::MediaPlayPause, LogicalEvent::EncoderPress(e)) if e.pressed => {
            media_play_pause();
            state_manager::request_state_check();
            true
        }

        (Capability::MediaPlayPause, LogicalEvent::Button(e)) if e.pressed => {
            media_play_pause();
            state_manager::request_state_check();
            true
        }

        (Capability::MediaNext, LogicalEvent::EncoderPress(e)) if e.pressed => {
            media_next();
            true
        }

        (Capability::MediaNext, LogicalEvent::Button(e)) if e.pressed => {
            media_next();
            true
        }

        (Capability::MediaPrevious, LogicalEvent::EncoderPress(e)) if e.pressed => {
            media_previous();
            true
        }

        (Capability::MediaPrevious, LogicalEvent::Button(e)) if e.pressed => {
            media_previous();
            true
        }

        (Capability::MediaStop, LogicalEvent::EncoderPress(e)) if e.pressed => {
            media_stop();
            true
        }

        (Capability::MediaStop, LogicalEvent::Button(e)) if e.pressed => {
            media_stop();
            true
        }

        _ => false,
    }
}

/// Check if a media binding is in an active state.
pub fn is_active(binding: &Binding, state: &SystemState) -> bool {
    matches!(&binding.capability, Capability::MediaPlayPause if state.is_playing)
}

// ─────────────────────────────────────────────────────────────────
// Media control functions (using playerctl)
// ─────────────────────────────────────────────────────────────────

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
