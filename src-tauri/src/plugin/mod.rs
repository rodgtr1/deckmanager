//! Plugin system for ArchDeck.
//!
//! Provides a trait-based plugin architecture that allows capabilities to be
//! organized into independent modules that can be enabled/disabled via Cargo features.

pub mod registry;
pub mod types;

pub use registry::PluginRegistry;
pub use types::{CapabilityMetadata, ParameterDef, ParameterType, PluginConfig, PluginInfo};

use crate::binding::Binding;
use crate::input_processor::LogicalEvent;
use crate::state_manager::SystemState;
use std::any::Any;
use std::sync::{Arc, Mutex};

/// Trait implemented by all plugins.
///
/// Plugins provide capabilities that can be bound to Stream Deck inputs.
/// Each plugin manages its own state and handles events for its capabilities.
#[allow(dead_code)]
pub trait Plugin: Send + Sync {
    /// Unique identifier for this plugin (e.g., "core", "elgato", "obs")
    fn id(&self) -> &'static str;

    /// Human-readable name for UI display
    fn name(&self) -> &'static str;

    /// Category for grouping in UI (e.g., "Core", "Lighting", "Streaming")
    fn category(&self) -> &'static str;

    /// Get metadata for all capabilities this plugin provides.
    fn capabilities(&self) -> Vec<CapabilityMetadata>;

    /// Handle a logical event for a binding.
    ///
    /// Returns `true` if this plugin handled the event, `false` otherwise.
    /// The plugin should check if the binding's capability belongs to it.
    fn handle_event(
        &self,
        event: &LogicalEvent,
        binding: &Binding,
        system_state: &Arc<Mutex<SystemState>>,
    ) -> bool;

    /// Check if a capability ID belongs to this plugin.
    fn owns_capability(&self, capability_type: &str) -> bool;

    /// Query the active state for a binding (for button image selection).
    ///
    /// Returns `true` if the binding is in an "active" state (e.g., muted, playing).
    /// This is used to determine whether to show the alternate button image.
    fn is_active(&self, binding: &Binding, system_state: &SystemState) -> bool;

    /// Initialize the plugin with configuration.
    ///
    /// Called once when the plugin is loaded.
    fn initialize(&mut self, _config: &PluginConfig) -> anyhow::Result<()> {
        Ok(())
    }

    /// Shutdown the plugin.
    ///
    /// Called when the application is closing or the plugin is being disabled.
    fn shutdown(&mut self) {}

    /// Get plugin-specific state as Any for downcasting.
    ///
    /// Allows plugins to expose custom state that other parts of the system
    /// might need to access.
    fn as_any(&self) -> &dyn Any;

    /// Get plugin-specific state as mutable Any for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    // --- Optional metadata methods with sensible defaults ---

    /// Plugin version string (default: "1.0.0")
    fn version(&self) -> &'static str {
        "1.0.0"
    }

    /// Short description of what the plugin does
    fn description(&self) -> &'static str {
        ""
    }

    /// Markdown documentation for the plugin
    fn documentation(&self) -> &'static str {
        ""
    }

    /// Icon/emoji for the plugin (default: plug emoji)
    fn icon(&self) -> &'static str {
        "🔌"
    }

    /// Whether this is a core plugin that cannot be disabled
    fn is_core(&self) -> bool {
        false
    }
}

/// Helper macro to implement common capability ID ownership check.
#[macro_export]
macro_rules! impl_owns_capability {
    ($($cap:literal),+ $(,)?) => {
        fn owns_capability(&self, capability_type: &str) -> bool {
            matches!(capability_type, $($cap)|+)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal plugin implementation using trait defaults
    struct MinimalPlugin;

    impl Plugin for MinimalPlugin {
        fn id(&self) -> &'static str { "minimal" }
        fn name(&self) -> &'static str { "Minimal" }
        fn category(&self) -> &'static str { "Test" }
        fn capabilities(&self) -> Vec<CapabilityMetadata> { vec![] }
        fn handle_event(&self, _: &LogicalEvent, _: &Binding, _: &Arc<Mutex<SystemState>>) -> bool { false }
        fn owns_capability(&self, _: &str) -> bool { false }
        fn is_active(&self, _: &Binding, _: &SystemState) -> bool { false }
        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }
        // All other methods use defaults
    }

    #[test]
    fn trait_default_version() {
        let plugin = MinimalPlugin;
        assert_eq!(plugin.version(), "1.0.0");
    }

    #[test]
    fn trait_default_description() {
        let plugin = MinimalPlugin;
        assert_eq!(plugin.description(), "");
    }

    #[test]
    fn trait_default_documentation() {
        let plugin = MinimalPlugin;
        assert_eq!(plugin.documentation(), "");
    }

    #[test]
    fn trait_default_icon() {
        let plugin = MinimalPlugin;
        assert_eq!(plugin.icon(), "🔌");
    }

    #[test]
    fn trait_default_is_core() {
        let plugin = MinimalPlugin;
        assert!(!plugin.is_core());
    }

    #[test]
    fn trait_default_initialize_succeeds() {
        let mut plugin = MinimalPlugin;
        let config = PluginConfig::default();
        assert!(plugin.initialize(&config).is_ok());
    }
}
