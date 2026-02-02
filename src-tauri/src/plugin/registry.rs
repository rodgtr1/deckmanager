//! Plugin registry for managing loaded plugins.

use super::{CapabilityMetadata, Plugin, PluginConfig, PluginInfo};
use crate::binding::Binding;
use crate::commands::CapabilityInfo;
use crate::input_processor::LogicalEvent;
use crate::state_manager::SystemState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

/// Central registry for managing plugins.
///
/// The registry:
/// - Stores all loaded plugins
/// - Maps capability IDs to their owning plugins
/// - Dispatches events to the appropriate plugin
/// - Tracks enabled/disabled state
pub struct PluginRegistry {
    /// Loaded plugins indexed by ID
    plugins: RwLock<HashMap<String, Box<dyn Plugin>>>,
    /// Capability ID -> Plugin ID mapping for fast dispatch
    capability_map: RwLock<HashMap<String, String>>,
    /// Plugin enabled states
    enabled: RwLock<HashMap<String, bool>>,
}

impl PluginRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            capability_map: RwLock::new(HashMap::new()),
            enabled: RwLock::new(HashMap::new()),
        }
    }

    /// Register a plugin with the registry.
    pub fn register(&self, mut plugin: Box<dyn Plugin>, config: Option<&PluginConfig>) {
        let plugin_id = plugin.id().to_string();

        // Initialize the plugin
        let cfg = config.cloned().unwrap_or_default();
        if let Err(e) = plugin.initialize(&cfg) {
            eprintln!("Failed to initialize plugin '{}': {}", plugin_id, e);
            return;
        }

        // Build capability map
        {
            let mut cap_map = self.capability_map.write().unwrap();
            for cap in plugin.capabilities() {
                cap_map.insert(cap.id.to_string(), plugin_id.clone());
            }
        }

        // Set enabled state
        {
            let mut enabled = self.enabled.write().unwrap();
            enabled.insert(plugin_id.clone(), cfg.enabled);
        }

        // Store the plugin
        {
            let mut plugins = self.plugins.write().unwrap();
            plugins.insert(plugin_id, plugin);
        }
    }

    /// Get all capabilities from enabled plugins.
    pub fn get_capabilities(&self) -> Vec<CapabilityMetadata> {
        let plugins = self.plugins.read().unwrap();
        let enabled = self.enabled.read().unwrap();

        let mut caps = Vec::new();
        for (id, plugin) in plugins.iter() {
            if *enabled.get(id).unwrap_or(&true) {
                caps.extend(plugin.capabilities());
            }
        }
        caps
    }

    /// Get capabilities formatted for frontend (CapabilityInfo).
    pub fn get_capability_infos(&self) -> Vec<CapabilityInfo> {
        self.get_capabilities()
            .into_iter()
            .map(|cap| CapabilityInfo {
                id: cap.id.to_string(),
                name: cap.name.to_string(),
                description: cap.description.to_string(),
                supports_button: cap.supports_button,
                supports_encoder: cap.supports_encoder,
                supports_encoder_press: cap.supports_encoder_press,
                parameters: cap
                    .parameters
                    .into_iter()
                    .map(|p| crate::commands::CapabilityParameter {
                        name: p.name.to_string(),
                        param_type: p.param_type.as_str().to_string(),
                        default_value: p.default_value.to_string(),
                        description: p.description.to_string(),
                    })
                    .collect(),
            })
            .collect()
    }

    /// Get information about all plugins.
    pub fn get_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().unwrap();
        let enabled = self.enabled.read().unwrap();

        plugins
            .iter()
            .map(|(id, plugin)| PluginInfo {
                id: id.clone(),
                name: plugin.name().to_string(),
                category: plugin.category().to_string(),
                enabled: *enabled.get(id).unwrap_or(&true),
                capability_count: plugin.capabilities().len(),
                version: plugin.version().to_string(),
                description: plugin.description().to_string(),
                documentation: plugin.documentation().to_string(),
                icon: plugin.icon().to_string(),
                is_core: plugin.is_core(),
            })
            .collect()
    }

    /// Set whether a plugin is enabled.
    #[allow(dead_code)]
    pub fn set_plugin_enabled(&self, plugin_id: &str, enabled_state: bool) -> bool {
        let mut enabled = self.enabled.write().unwrap();
        if enabled.contains_key(plugin_id) {
            enabled.insert(plugin_id.to_string(), enabled_state);
            true
        } else {
            false
        }
    }

    /// Check if a plugin is enabled.
    #[allow(dead_code)]
    pub fn is_plugin_enabled(&self, plugin_id: &str) -> bool {
        let enabled = self.enabled.read().unwrap();
        *enabled.get(plugin_id).unwrap_or(&false)
    }

    /// Handle a logical event by dispatching to the appropriate plugin.
    ///
    /// Returns `true` if a plugin handled the event.
    pub fn handle_event(
        &self,
        event: &LogicalEvent,
        binding: &Binding,
        system_state: &Arc<Mutex<SystemState>>,
    ) -> bool {
        // Get the capability type from the binding
        let capability_type = get_capability_type(&binding.capability);

        #[cfg(debug_assertions)]
        eprintln!("    registry.handle_event: capability_type={}", capability_type);

        // Look up which plugin owns this capability
        let plugin_id = {
            let cap_map = self.capability_map.read().unwrap();
            cap_map.get(capability_type).cloned()
        };

        #[cfg(debug_assertions)]
        eprintln!("    registry.handle_event: plugin_id={:?}", plugin_id);

        let Some(plugin_id) = plugin_id else {
            #[cfg(debug_assertions)]
            eprintln!("    registry.handle_event: NO PLUGIN FOUND for {}", capability_type);
            return false;
        };

        // Check if plugin is enabled
        {
            let enabled = self.enabled.read().unwrap();
            if !*enabled.get(&plugin_id).unwrap_or(&true) {
                return false;
            }
        }

        // Dispatch to plugin
        let plugins = self.plugins.read().unwrap();
        if let Some(plugin) = plugins.get(&plugin_id) {
            plugin.handle_event(event, binding, system_state)
        } else {
            false
        }
    }

    /// Check if a binding is in an "active" state.
    ///
    /// Used for determining which button image to display.
    pub fn is_binding_active(&self, binding: &Binding, system_state: &SystemState) -> bool {
        let capability_type = get_capability_type(&binding.capability);

        let plugin_id = {
            let cap_map = self.capability_map.read().unwrap();
            cap_map.get(capability_type).cloned()
        };

        let Some(plugin_id) = plugin_id else {
            return false;
        };

        // Check if plugin is enabled
        {
            let enabled = self.enabled.read().unwrap();
            if !*enabled.get(&plugin_id).unwrap_or(&true) {
                return false;
            }
        }

        let plugins = self.plugins.read().unwrap();
        if let Some(plugin) = plugins.get(&plugin_id) {
            plugin.is_active(binding, system_state)
        } else {
            false
        }
    }

    /// Shutdown all plugins.
    #[allow(dead_code)]
    pub fn shutdown(&self) {
        let mut plugins = self.plugins.write().unwrap();
        for (_, plugin) in plugins.iter_mut() {
            plugin.shutdown();
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract the capability type string from a Capability enum.
fn get_capability_type(capability: &crate::capability::Capability) -> &'static str {
    use crate::capability::Capability;
    match capability {
        Capability::SystemAudio { .. } => "SystemAudio",
        Capability::Mute => "Mute",
        Capability::VolumeUp { .. } => "VolumeUp",
        Capability::VolumeDown { .. } => "VolumeDown",
        Capability::Microphone { .. } => "Microphone",
        Capability::MicMute => "MicMute",
        Capability::MicVolumeUp { .. } => "MicVolumeUp",
        Capability::MicVolumeDown { .. } => "MicVolumeDown",
        Capability::MediaPlayPause => "MediaPlayPause",
        Capability::MediaNext => "MediaNext",
        Capability::MediaPrevious => "MediaPrevious",
        Capability::MediaStop => "MediaStop",
        Capability::RunCommand { .. } => "RunCommand",
        Capability::LaunchApp { .. } => "LaunchApp",
        Capability::OpenURL { .. } => "OpenURL",
        Capability::ElgatoKeyLight { .. } => "ElgatoKeyLight",
        // OBS capabilities
        Capability::OBSScene { .. } => "OBSScene",
        Capability::OBSStream { .. } => "OBSStream",
        Capability::OBSRecord { .. } => "OBSRecord",
        Capability::OBSSourceVisibility { .. } => "OBSSourceVisibility",
        Capability::OBSAudio { .. } => "OBSAudio",
        Capability::OBSStudioMode { .. } => "OBSStudioMode",
        Capability::OBSReplayBuffer { .. } => "OBSReplayBuffer",
        Capability::OBSVirtualCam { .. } => "OBSVirtualCam",
        Capability::OBSTransition { .. } => "OBSTransition",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;
    use std::any::Any;

    #[test]
    fn get_capability_type_returns_correct_strings() {
        assert_eq!(
            get_capability_type(&Capability::SystemAudio { step: 0.02 }),
            "SystemAudio"
        );
        assert_eq!(get_capability_type(&Capability::Mute), "Mute");
        assert_eq!(get_capability_type(&Capability::MediaPlayPause), "MediaPlayPause");
        assert_eq!(
            get_capability_type(&Capability::RunCommand {
                command: "test".to_string(),
                toggle: false
            }),
            "RunCommand"
        );
    }

    // Mock plugin for testing
    struct MockPlugin {
        id: &'static str,
        is_core: bool,
    }

    impl Plugin for MockPlugin {
        fn id(&self) -> &'static str { self.id }
        fn name(&self) -> &'static str { "Mock Plugin" }
        fn category(&self) -> &'static str { "Test" }
        fn capabilities(&self) -> Vec<CapabilityMetadata> { vec![] }
        fn handle_event(&self, _: &LogicalEvent, _: &Binding, _: &Arc<Mutex<SystemState>>) -> bool { false }
        fn owns_capability(&self, _: &str) -> bool { false }
        fn is_active(&self, _: &Binding, _: &SystemState) -> bool { false }
        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }
        fn version(&self) -> &'static str { "2.0.0" }
        fn description(&self) -> &'static str { "A mock plugin for testing" }
        fn documentation(&self) -> &'static str { "# Mock\n\nDocs here." }
        fn icon(&self) -> &'static str { "🧪" }
        fn is_core(&self) -> bool { self.is_core }
    }

    #[test]
    fn registry_new_is_empty() {
        let registry = PluginRegistry::new();
        assert!(registry.get_plugins().is_empty());
        assert!(registry.get_capabilities().is_empty());
    }

    #[test]
    fn registry_register_adds_plugin() {
        let registry = PluginRegistry::new();
        registry.register(Box::new(MockPlugin { id: "mock", is_core: false }), None);

        let plugins = registry.get_plugins();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].id, "mock");
    }

    #[test]
    fn get_plugins_includes_metadata_fields() {
        let registry = PluginRegistry::new();
        registry.register(Box::new(MockPlugin { id: "mock", is_core: false }), None);

        let plugins = registry.get_plugins();
        let plugin = &plugins[0];

        assert_eq!(plugin.id, "mock");
        assert_eq!(plugin.name, "Mock Plugin");
        assert_eq!(plugin.category, "Test");
        assert_eq!(plugin.version, "2.0.0");
        assert_eq!(plugin.description, "A mock plugin for testing");
        assert_eq!(plugin.documentation, "# Mock\n\nDocs here.");
        assert_eq!(plugin.icon, "🧪");
        assert!(!plugin.is_core);
        assert!(plugin.enabled); // Default is enabled
    }

    #[test]
    fn get_plugins_core_flag_set_correctly() {
        let registry = PluginRegistry::new();
        registry.register(Box::new(MockPlugin { id: "core_mock", is_core: true }), None);

        let plugins = registry.get_plugins();
        assert!(plugins[0].is_core);
    }

    #[test]
    fn set_plugin_enabled_updates_state() {
        let registry = PluginRegistry::new();
        registry.register(Box::new(MockPlugin { id: "mock", is_core: false }), None);

        // Initially enabled
        assert!(registry.is_plugin_enabled("mock"));

        // Disable it
        assert!(registry.set_plugin_enabled("mock", false));
        assert!(!registry.is_plugin_enabled("mock"));

        // Re-enable it
        assert!(registry.set_plugin_enabled("mock", true));
        assert!(registry.is_plugin_enabled("mock"));
    }

    #[test]
    fn set_plugin_enabled_returns_false_for_unknown_plugin() {
        let registry = PluginRegistry::new();
        assert!(!registry.set_plugin_enabled("nonexistent", true));
    }

    #[test]
    fn is_plugin_enabled_returns_false_for_unknown_plugin() {
        let registry = PluginRegistry::new();
        assert!(!registry.is_plugin_enabled("nonexistent"));
    }

    #[test]
    fn get_plugins_reflects_enabled_state() {
        let registry = PluginRegistry::new();
        registry.register(Box::new(MockPlugin { id: "mock", is_core: false }), None);

        // Initially enabled
        let plugins = registry.get_plugins();
        assert!(plugins[0].enabled);

        // Disable and check
        registry.set_plugin_enabled("mock", false);
        let plugins = registry.get_plugins();
        assert!(!plugins[0].enabled);
    }

    #[test]
    fn register_with_config_sets_enabled_state() {
        let registry = PluginRegistry::new();
        let config = PluginConfig {
            enabled: false,
            settings: std::collections::HashMap::new(),
        };
        registry.register(Box::new(MockPlugin { id: "mock", is_core: false }), Some(&config));

        assert!(!registry.is_plugin_enabled("mock"));
    }
}
