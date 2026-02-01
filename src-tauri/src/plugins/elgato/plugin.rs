//! Elgato Key Light plugin implementation.

use super::client;
use super::controller::KeyLightController;
use crate::binding::Binding;
use crate::capability::{Capability, KeyLightAction, KEY_LIGHT_BRIGHTNESS_STEP};
use crate::impl_owns_capability;
use crate::input_processor::LogicalEvent;
use crate::plugin::{CapabilityMetadata, ParameterDef, ParameterType, Plugin, PluginConfig};
use crate::state_manager::SystemState;
use crate::streamdeck::request_image_sync;
use std::any::Any;
use std::sync::{Arc, Mutex, OnceLock};

/// Global debounced Key Light controller
static KEY_LIGHT_CONTROLLER: OnceLock<KeyLightController> = OnceLock::new();

/// Get or initialize the Key Light controller
fn get_key_light_controller() -> &'static KeyLightController {
    KEY_LIGHT_CONTROLLER.get_or_init(KeyLightController::new)
}

/// Elgato Key Light plugin
pub struct ElgatoPlugin;

impl ElgatoPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ElgatoPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ElgatoPlugin {
    fn id(&self) -> &'static str {
        "elgato"
    }

    fn name(&self) -> &'static str {
        "Elgato Key Light"
    }

    fn category(&self) -> &'static str {
        "Lighting"
    }

    fn capabilities(&self) -> Vec<CapabilityMetadata> {
        vec![CapabilityMetadata {
            id: "ElgatoKeyLight",
            name: "Key Light",
            description: "Control Elgato Key Light - rotate for brightness, press to toggle",
            plugin_id: "elgato",
            supports_button: true,
            supports_encoder: true,
            supports_encoder_press: true,
            parameters: vec![ParameterDef {
                name: "ip",
                param_type: ParameterType::IpAddress,
                default_value: "192.168.1.100",
                description: "IP address of the Key Light",
            }],
        }]
    }

    fn handle_event(
        &self,
        event: &LogicalEvent,
        binding: &Binding,
        system_state: &Arc<Mutex<SystemState>>,
    ) -> bool {
        match (&binding.capability, event) {
            // Button press -> toggle
            (Capability::ElgatoKeyLight { ip, port, .. }, LogicalEvent::Button(e)) if e.pressed => {
                handle_key_light_button(ip, *port, &KeyLightAction::Toggle, system_state);
                true
            }

            // Encoder press -> toggle
            (Capability::ElgatoKeyLight { ip, port, .. }, LogicalEvent::EncoderPress(e)) if e.pressed => {
                handle_key_light_button(ip, *port, &KeyLightAction::Toggle, system_state);
                true
            }

            // Encoder rotation -> brightness
            (Capability::ElgatoKeyLight { ip, port, .. }, LogicalEvent::Encoder(e)) => {
                handle_key_light_brightness(ip, *port, e.delta);
                true
            }

            _ => false,
        }
    }

    impl_owns_capability!("ElgatoKeyLight");

    fn is_active(&self, binding: &Binding, system_state: &SystemState) -> bool {
        if let Capability::ElgatoKeyLight { ip, port, .. } = &binding.capability {
            // Check if key light is on
            system_state
                .key_lights
                .get(&format!("{}:{}", ip, port))
                .map(|s| s.on)
                .unwrap_or(false)
        } else {
            false
        }
    }

    fn initialize(&mut self, _config: &PluginConfig) -> anyhow::Result<()> {
        // Initialize the controller lazily
        let _ = get_key_light_controller();
        Ok(())
    }

    fn shutdown(&mut self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn description(&self) -> &'static str {
        "Control Elgato Key Lights over your network"
    }

    fn documentation(&self) -> &'static str {
        include_str!("../../../docs/plugins/elgato.md")
    }

    fn icon(&self) -> &'static str {
        "💡"
    }
}

fn handle_key_light_button(ip: &str, port: u16, action: &KeyLightAction, system_state: &Arc<Mutex<SystemState>>) {
    // Spawn background thread to avoid blocking the event loop
    let ip = ip.to_string();
    let action = action.clone();
    let state = Arc::clone(system_state);

    std::thread::spawn(move || {
        let result = match action {
            KeyLightAction::Toggle => client::toggle(&ip, port).map(|_| ()),
            KeyLightAction::On => client::turn_on(&ip, port),
            KeyLightAction::Off => client::turn_off(&ip, port),
            KeyLightAction::SetBrightness => Ok(()), // Handled by encoder
        };

        if let Err(e) = result {
            eprintln!("Key Light error: {e}");
        }

        // Update key light state and trigger image sync
        match client::get_state(&ip, port) {
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

fn handle_key_light_brightness(ip: &str, port: u16, delta: i8) {
    let brightness_delta = delta as i32 * KEY_LIGHT_BRIGHTNESS_STEP;

    // Queue the adjustment - will be debounced and sent in batch
    get_key_light_controller().queue_brightness_delta(ip, port, brightness_delta);

    // Note: State sync happens after debounce in the controller's background thread
    // We don't block the event loop waiting for HTTP responses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elgato_plugin_has_correct_id() {
        let plugin = ElgatoPlugin::new();
        assert_eq!(plugin.id(), "elgato");
    }

    #[test]
    fn elgato_plugin_has_key_light_capability() {
        let plugin = ElgatoPlugin::new();
        let caps = plugin.capabilities();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].id, "ElgatoKeyLight");
    }

    #[test]
    fn elgato_plugin_owns_its_capability() {
        let plugin = ElgatoPlugin::new();
        assert!(plugin.owns_capability("ElgatoKeyLight"));
        assert!(!plugin.owns_capability("SystemAudio"));
    }

    #[test]
    fn elgato_plugin_metadata_name() {
        let plugin = ElgatoPlugin::new();
        assert_eq!(plugin.name(), "Elgato Key Light");
    }

    #[test]
    fn elgato_plugin_metadata_category() {
        let plugin = ElgatoPlugin::new();
        assert_eq!(plugin.category(), "Lighting");
    }

    #[test]
    fn elgato_plugin_metadata_version() {
        let plugin = ElgatoPlugin::new();
        assert_eq!(plugin.version(), "1.0.0");
    }

    #[test]
    fn elgato_plugin_metadata_description() {
        let plugin = ElgatoPlugin::new();
        assert_eq!(plugin.description(), "Control Elgato Key Lights over your network");
    }

    #[test]
    fn elgato_plugin_metadata_icon() {
        let plugin = ElgatoPlugin::new();
        assert_eq!(plugin.icon(), "💡");
    }

    #[test]
    fn elgato_plugin_is_not_core() {
        let plugin = ElgatoPlugin::new();
        assert!(!plugin.is_core());
    }

    #[test]
    fn elgato_plugin_documentation_not_empty() {
        let plugin = ElgatoPlugin::new();
        let docs = plugin.documentation();
        assert!(!docs.is_empty());
        assert!(docs.contains("Elgato Key Light"));
    }
}
