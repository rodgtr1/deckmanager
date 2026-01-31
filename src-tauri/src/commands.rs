use crate::binding::{Binding, InputRef};
use crate::capability::Capability;
use crate::config;
use crate::device::DeviceInfo;
use crate::streamdeck;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::State;

/// Shared application state accessible from commands.
pub struct AppState {
    pub device_info: Arc<Mutex<Option<DeviceInfo>>>,
    pub bindings: Arc<Mutex<Vec<Binding>>>,
}

/// Information about an available capability for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Which input types this capability supports.
    pub supports_button: bool,
    pub supports_encoder: bool,
    pub supports_encoder_press: bool,
    /// Parameters this capability accepts.
    pub parameters: Vec<CapabilityParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityParameter {
    pub name: String,
    pub param_type: String,
    pub default_value: String,
    pub description: String,
}

/// Get connected device information.
#[tauri::command]
pub fn get_device_info(state: State<AppState>) -> Option<DeviceInfo> {
    state.device_info.lock().ok()?.clone()
}

/// Get current bindings.
#[tauri::command]
pub fn get_bindings(state: State<AppState>) -> Vec<Binding> {
    state.bindings.lock().ok().map(|b| b.clone()).unwrap_or_default()
}

/// Get available capabilities.
#[tauri::command]
pub fn get_capabilities() -> Vec<CapabilityInfo> {
    vec![
        CapabilityInfo {
            id: "SystemVolume".to_string(),
            name: "System Volume".to_string(),
            description: "Adjust system volume with encoder rotation".to_string(),
            supports_button: false,
            supports_encoder: true,
            supports_encoder_press: false,
            parameters: vec![CapabilityParameter {
                name: "step".to_string(),
                param_type: "f32".to_string(),
                default_value: "0.02".to_string(),
                description: "Volume change per encoder tick (0.0-1.0)".to_string(),
            }],
        },
        CapabilityInfo {
            id: "ToggleMute".to_string(),
            name: "Toggle Mute".to_string(),
            description: "Toggle system audio mute on/off".to_string(),
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![],
        },
        CapabilityInfo {
            id: "MediaPlayPause".to_string(),
            name: "Play/Pause".to_string(),
            description: "Toggle media playback".to_string(),
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![],
        },
        CapabilityInfo {
            id: "MediaNext".to_string(),
            name: "Next Track".to_string(),
            description: "Skip to next track".to_string(),
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![],
        },
        CapabilityInfo {
            id: "MediaPrevious".to_string(),
            name: "Previous Track".to_string(),
            description: "Go to previous track".to_string(),
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![],
        },
        CapabilityInfo {
            id: "MediaStop".to_string(),
            name: "Stop".to_string(),
            description: "Stop media playback".to_string(),
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![],
        },
        CapabilityInfo {
            id: "RunCommand".to_string(),
            name: "Run Command".to_string(),
            description: "Execute a shell command".to_string(),
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![CapabilityParameter {
                name: "command".to_string(),
                param_type: "string".to_string(),
                default_value: "".to_string(),
                description: "Shell command to execute".to_string(),
            }],
        },
        CapabilityInfo {
            id: "LaunchApp".to_string(),
            name: "Launch App".to_string(),
            description: "Launch an application".to_string(),
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![CapabilityParameter {
                name: "command".to_string(),
                param_type: "string".to_string(),
                default_value: "".to_string(),
                description: "Application to launch (e.g., firefox, code)".to_string(),
            }],
        },
        CapabilityInfo {
            id: "OpenURL".to_string(),
            name: "Open URL".to_string(),
            description: "Open a URL in your default browser".to_string(),
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![CapabilityParameter {
                name: "url".to_string(),
                param_type: "string".to_string(),
                default_value: "https://".to_string(),
                description: "URL to open".to_string(),
            }],
        },
    ]
}

/// Parameters for set_binding command - using a struct ensures proper deserialization
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SetBindingParams {
    pub input: InputRef,
    pub capability: Capability,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub button_image: Option<String>,
    #[serde(default)]
    pub show_label: Option<bool>,
}

/// Add or update a binding.
#[tauri::command]
pub fn set_binding(state: State<AppState>, params: SetBindingParams) -> Result<(), String> {
    #[cfg(debug_assertions)]
    eprintln!(
        "set_binding called: button_image={:?}, show_label={:?}",
        params.button_image, params.show_label
    );

    let mut bindings = state.bindings.lock().map_err(|e| e.to_string())?;

    // Remove existing binding for this input if present
    bindings.retain(|b| !inputs_match(&b.input, &params.input));

    // Add new binding
    bindings.push(Binding {
        input: params.input,
        capability: params.capability,
        icon: params.icon,
        label: params.label,
        button_image: params.button_image,
        show_label: params.show_label,
    });

    // Request button image sync to hardware
    streamdeck::request_image_sync();

    Ok(())
}

/// Remove a binding for an input.
#[tauri::command]
pub fn remove_binding(state: State<AppState>, input: InputRef) -> Result<(), String> {
    let mut bindings = state.bindings.lock().map_err(|e| e.to_string())?;
    bindings.retain(|b| !inputs_match(&b.input, &input));

    // Request button image sync to clear the removed button
    streamdeck::request_image_sync();

    Ok(())
}

/// Sync button images to hardware.
#[tauri::command]
pub fn sync_button_images() {
    streamdeck::request_image_sync();
}

/// Save bindings to config file.
#[tauri::command]
pub fn save_bindings(state: State<AppState>) -> Result<(), String> {
    let bindings = state.bindings.lock().map_err(|e| e.to_string())?;
    config::save_bindings(&bindings).map_err(|e| e.to_string())
}

/// Check if two InputRefs refer to the same input.
fn inputs_match(a: &InputRef, b: &InputRef) -> bool {
    match (a, b) {
        (InputRef::Button { index: i1 }, InputRef::Button { index: i2 }) => i1 == i2,
        (InputRef::Encoder { index: i1 }, InputRef::Encoder { index: i2 }) => i1 == i2,
        (InputRef::EncoderPress { index: i1 }, InputRef::EncoderPress { index: i2 }) => i1 == i2,
        (InputRef::Swipe, InputRef::Swipe) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs_match_same_button() {
        let a = InputRef::Button { index: 0 };
        let b = InputRef::Button { index: 0 };
        assert!(inputs_match(&a, &b));
    }

    #[test]
    fn inputs_match_different_buttons() {
        let a = InputRef::Button { index: 0 };
        let b = InputRef::Button { index: 1 };
        assert!(!inputs_match(&a, &b));
    }

    #[test]
    fn inputs_match_different_types() {
        let a = InputRef::Button { index: 0 };
        let b = InputRef::Encoder { index: 0 };
        assert!(!inputs_match(&a, &b));
    }

    #[test]
    fn capabilities_list_not_empty() {
        let caps = get_capabilities();
        assert!(!caps.is_empty());
        assert!(caps.iter().any(|c| c.id == "SystemVolume"));
        assert!(caps.iter().any(|c| c.id == "ToggleMute"));
        assert!(caps.iter().any(|c| c.id == "MediaPlayPause"));
        assert!(caps.iter().any(|c| c.id == "MediaNext"));
        assert!(caps.iter().any(|c| c.id == "MediaPrevious"));
        assert!(caps.iter().any(|c| c.id == "MediaStop"));
        assert!(caps.iter().any(|c| c.id == "RunCommand"));
        assert!(caps.iter().any(|c| c.id == "LaunchApp"));
        assert!(caps.iter().any(|c| c.id == "OpenURL"));
    }
}
