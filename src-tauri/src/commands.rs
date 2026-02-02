use crate::binding::{Binding, InputRef};
use crate::capability::Capability;
use crate::config;
use crate::device::DeviceInfo;
use crate::plugin::{PluginInfo, PluginRegistry};
use crate::state_manager::{self, SystemState};
use crate::streamdeck;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::State;

/// Shared application state accessible from commands.
pub struct AppState {
    pub device_info: Arc<Mutex<Option<DeviceInfo>>>,
    pub bindings: Arc<Mutex<Vec<Binding>>>,
    pub system_state: Arc<Mutex<SystemState>>,
    pub current_page: Arc<Mutex<usize>>,
    pub plugin_registry: Arc<PluginRegistry>,
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

/// Get available capabilities from all enabled plugins.
#[tauri::command]
pub fn get_capabilities(state: State<AppState>) -> Vec<CapabilityInfo> {
    state.plugin_registry.get_capability_infos()
}

/// Parameters for set_binding command - using a struct ensures proper deserialization
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SetBindingParams {
    pub input: InputRef,
    pub capability: Capability,
    #[serde(default)]
    pub page: usize,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub button_image: Option<String>,
    #[serde(default)]
    pub button_image_alt: Option<String>,
    #[serde(default)]
    pub show_label: Option<bool>,
    #[serde(default)]
    pub icon_color: Option<String>,
    #[serde(default)]
    pub icon_color_alt: Option<String>,
}

/// Add or update a binding.
#[tauri::command]
pub fn set_binding(state: State<AppState>, params: SetBindingParams) -> Result<(), String> {
    #[cfg(debug_assertions)]
    eprintln!(
        "set_binding called: page={}, button_image={:?}, show_label={:?}",
        params.page, params.button_image, params.show_label
    );

    let mut bindings = state.bindings.lock().map_err(|e| e.to_string())?;

    // Remove existing binding for this input AND page if present
    bindings.retain(|b| !(inputs_match(&b.input, &params.input) && b.page == params.page));

    // Add new binding
    bindings.push(Binding {
        input: params.input,
        capability: params.capability,
        page: params.page,
        icon: params.icon,
        label: params.label,
        button_image: params.button_image,
        button_image_alt: params.button_image_alt,
        show_label: params.show_label,
        icon_color: params.icon_color,
        icon_color_alt: params.icon_color_alt,
    });

    // Request button image sync to hardware
    streamdeck::request_image_sync();

    Ok(())
}

/// Remove a binding for an input on a specific page.
#[tauri::command]
pub fn remove_binding(state: State<AppState>, input: InputRef, page: Option<usize>) -> Result<(), String> {
    let current_page = *state.current_page.lock().map_err(|e| e.to_string())?;
    let target_page = page.unwrap_or(current_page);

    let mut bindings = state.bindings.lock().map_err(|e| e.to_string())?;
    bindings.retain(|b| !(inputs_match(&b.input, &input) && b.page == target_page));

    // Request button image sync to clear the removed button
    streamdeck::request_image_sync();

    Ok(())
}

/// Get the current page number.
#[tauri::command]
pub fn get_current_page(state: State<AppState>) -> usize {
    *state.current_page.lock().unwrap_or_else(|e| e.into_inner())
}

/// Set the current page number.
#[tauri::command]
pub fn set_current_page(state: State<AppState>, page: usize) {
    if let Ok(mut current) = state.current_page.lock() {
        *current = page;
    }
    // Sync hardware to show the new page's bindings
    streamdeck::request_image_sync();
}

/// Get the total number of pages (based on max page in bindings + 1).
#[tauri::command]
pub fn get_page_count(state: State<AppState>) -> usize {
    let bindings = state.bindings.lock().ok();
    match bindings {
        Some(b) => b.iter().map(|binding| binding.page).max().unwrap_or(0) + 1,
        None => 1,
    }
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

/// Get current system state (mute, playback).
#[derive(Debug, Clone, Serialize)]
pub struct SystemStateResponse {
    pub is_muted: bool,
    pub is_playing: bool,
}

#[tauri::command]
pub fn get_system_state(state: State<AppState>) -> SystemStateResponse {
    // Also request a fresh state check
    state_manager::request_state_check();

    let current = state.system_state.lock().unwrap();
    SystemStateResponse {
        is_muted: current.is_muted,
        is_playing: current.is_playing,
    }
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

/// Get information about all plugins.
#[tauri::command]
pub fn get_plugins(state: State<AppState>) -> Vec<PluginInfo> {
    state.plugin_registry.get_plugins()
}

/// Enable or disable a plugin.
#[tauri::command]
pub fn set_plugin_enabled(
    state: State<AppState>,
    plugin_id: String,
    enabled: bool,
) -> Result<(), String> {
    // Check if plugin is core (cannot be disabled)
    let plugins = state.plugin_registry.get_plugins();
    if let Some(plugin) = plugins.iter().find(|p| p.id == plugin_id) {
        if plugin.is_core && !enabled {
            return Err("Core plugins cannot be disabled".to_string());
        }
    } else {
        return Err(format!("Plugin '{}' not found", plugin_id));
    }

    // Update the enabled state in the registry
    if !state.plugin_registry.set_plugin_enabled(&plugin_id, enabled) {
        return Err(format!("Failed to update plugin '{}' state", plugin_id));
    }

    // Persist the state to disk
    if let Err(e) = config::save_plugin_state(&plugin_id, enabled) {
        eprintln!("Failed to persist plugin state: {}", e);
        // Don't fail the command, state is still updated in memory
    }

    Ok(())
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

}
