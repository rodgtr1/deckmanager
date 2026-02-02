//! State management for stateful capabilities (mute, media playback, etc.)
//!
//! Polls system state every 2 seconds and emits events when state changes.

#[cfg(feature = "plugin-elgato")]
use crate::plugins::elgato::client::{self as elgato_key_light, KeyLightState};
#[cfg(not(feature = "plugin-elgato"))]
use serde::{Deserialize, Serialize};
#[cfg(not(feature = "plugin-elgato"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyLightState {
    pub on: bool,
    pub brightness: u8,
    pub temperature: u16,
}
use std::collections::HashMap;

/// State of an OBS instance
#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // Fields used by plugin-obs feature
pub struct OBSState {
    /// Whether streaming is active
    pub streaming: bool,
    /// Whether recording is active
    pub recording: bool,
    /// Whether recording is paused
    pub recording_paused: bool,
    /// Whether Studio Mode is enabled
    pub studio_mode: bool,
    /// Whether virtual camera is active
    pub virtual_cam: bool,
    /// Whether replay buffer is active
    pub replay_buffer: bool,
    /// Current scene name
    pub current_scene: String,
    /// Muted state of audio inputs: input_name -> is_muted
    pub muted_inputs: HashMap<String, bool>,
    /// Source visibility: "scene:source" -> is_visible
    pub source_visibility: HashMap<String, bool>,
}
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Current state of stateful capabilities
#[derive(Debug, Clone, Default)]
pub struct SystemState {
    pub is_muted: bool,
    pub is_mic_muted: bool,
    pub is_playing: bool,
    /// Key light states: "ip:port" -> KeyLightState
    pub key_lights: HashMap<String, KeyLightState>,
    /// Toggle states for RunCommand with toggle=true: "input_key:page" -> is_active
    pub toggle_states: HashMap<String, bool>,
    /// OBS states: "host:port" -> OBSState
    #[allow(dead_code)] // Used by plugin-obs feature
    pub obs_states: HashMap<String, OBSState>,
}

/// Flag to request immediate state check (e.g., after button press)
pub static CHECK_STATE_NOW: AtomicBool = AtomicBool::new(false);

/// Request an immediate state check
pub fn request_state_check() {
    CHECK_STATE_NOW.store(true, Ordering::SeqCst);
}

/// Check if audio is currently muted via wpctl
pub fn check_mute_state() -> bool {
    let output = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Output looks like "Volume: 0.50" or "Volume: 0.50 [MUTED]"
            stdout.contains("[MUTED]")
        }
        Err(_) => false,
    }
}

/// Check if microphone is currently muted via wpctl
pub fn check_mic_mute_state() -> bool {
    let output = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SOURCE@"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Output looks like "Volume: 0.50" or "Volume: 0.50 [MUTED]"
            stdout.contains("[MUTED]")
        }
        Err(_) => false,
    }
}

/// Check if media is currently playing via playerctl
pub fn check_playing_state() -> bool {
    let output = Command::new("playerctl")
        .arg("status")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Output is "Playing", "Paused", or "Stopped"
            stdout.trim() == "Playing"
        }
        Err(_) => false,
    }
}

/// Get current system state (without key lights - those are checked separately)
pub fn get_current_state() -> SystemState {
    SystemState {
        is_muted: check_mute_state(),
        is_mic_muted: check_mic_mute_state(),
        is_playing: check_playing_state(),
        key_lights: HashMap::new(),
        toggle_states: HashMap::new(),
        obs_states: HashMap::new(),
    }
}

/// Check key light state for a specific light
/// (Reserved for future key light state polling)
#[allow(dead_code)]
#[cfg(feature = "plugin-elgato")]
pub fn check_key_light_state(ip: &str, port: u16) -> Option<KeyLightState> {
    elgato_key_light::get_state(ip, port).ok()
}

#[allow(dead_code)]
#[cfg(not(feature = "plugin-elgato"))]
pub fn check_key_light_state(_ip: &str, _port: u16) -> Option<KeyLightState> {
    None
}

/// State change event emitted to frontend
#[derive(Debug, Clone, serde::Serialize)]
pub struct StateChangeEvent {
    pub is_muted: bool,
    pub is_mic_muted: bool,
    pub is_playing: bool,
    pub key_lights: HashMap<String, KeyLightState>,
}

/// Run the state polling loop
pub fn run_state_poller(app: AppHandle, state: Arc<Mutex<SystemState>>) {
    let check_interval = Duration::from_millis(100);

    loop {
        // Check if immediate check requested
        let should_check = CHECK_STATE_NOW.swap(false, Ordering::SeqCst);

        if should_check {
            let mut new_state = get_current_state();
            let mut current = state.lock().unwrap();

            // Copy over existing key light states (they're checked on demand)
            new_state.key_lights = current.key_lights.clone();

            let state_changed = new_state.is_muted != current.is_muted
                || new_state.is_mic_muted != current.is_mic_muted
                || new_state.is_playing != current.is_playing;

            if state_changed {
                *current = new_state.clone();
                drop(current); // Release lock before emitting

                // Emit state change event
                let _ = app.emit("state:change", StateChangeEvent {
                    is_muted: new_state.is_muted,
                    is_mic_muted: new_state.is_mic_muted,
                    is_playing: new_state.is_playing,
                    key_lights: new_state.key_lights,
                });

                // Request image sync to update hardware
                crate::streamdeck::request_image_sync();
            }
        }

        std::thread::sleep(check_interval);

        // Every 2 seconds, do a regular poll
        static mut TICK_COUNT: u32 = 0;
        unsafe {
            TICK_COUNT += 1;
            if TICK_COUNT >= 20 { // 20 * 100ms = 2s
                TICK_COUNT = 0;

                let mut new_state = get_current_state();
                let mut current = state.lock().unwrap();

                // Copy over existing key light states
                new_state.key_lights = current.key_lights.clone();

                let state_changed = new_state.is_muted != current.is_muted
                    || new_state.is_mic_muted != current.is_mic_muted
                    || new_state.is_playing != current.is_playing;

                if state_changed {
                    *current = new_state.clone();
                    drop(current);

                    let _ = app.emit("state:change", StateChangeEvent {
                        is_muted: new_state.is_muted,
                        is_mic_muted: new_state.is_mic_muted,
                        is_playing: new_state.is_playing,
                        key_lights: new_state.key_lights,
                    });

                    crate::streamdeck::request_image_sync();
                }
            }
        }
    }
}

/// Update key light state in the system state
/// (Reserved for future key light state polling)
#[allow(dead_code)]
pub fn update_key_light_state(
    state: &Arc<Mutex<SystemState>>,
    ip: &str,
    port: u16,
) {
    if let Some(light_state) = check_key_light_state(ip, port) {
        if let Ok(mut current) = state.lock() {
            let key = format!("{}:{}", ip, port);
            current.key_lights.insert(key, light_state);
        }
    }
}
