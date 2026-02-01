//! Core plugin providing built-in capabilities.
//!
//! This plugin is always compiled and cannot be disabled.
//! It provides:
//! - Audio control (SystemAudio, Mute, Volume, Microphone)
//! - Media control (PlayPause, Next, Previous, Stop)
//! - Command execution (RunCommand, LaunchApp, OpenURL)

pub mod audio;
pub mod commands;
pub mod media;

use crate::binding::Binding;
use crate::impl_owns_capability;
use crate::input_processor::LogicalEvent;
use crate::plugin::{CapabilityMetadata, Plugin, PluginConfig};
use crate::state_manager::SystemState;
use std::any::Any;
use std::sync::{Arc, Mutex};

/// The core plugin providing built-in capabilities.
pub struct CorePlugin;

impl CorePlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CorePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for CorePlugin {
    fn id(&self) -> &'static str {
        "core"
    }

    fn name(&self) -> &'static str {
        "Core"
    }

    fn category(&self) -> &'static str {
        "Core"
    }

    fn capabilities(&self) -> Vec<CapabilityMetadata> {
        let mut caps = Vec::new();
        caps.extend(audio::capabilities());
        caps.extend(media::capabilities());
        caps.extend(commands::capabilities());
        caps
    }

    fn handle_event(
        &self,
        event: &LogicalEvent,
        binding: &Binding,
        system_state: &Arc<Mutex<SystemState>>,
    ) -> bool {
        // Try each module in order
        if audio::handle_event(event, binding, system_state) {
            return true;
        }
        if media::handle_event(event, binding, system_state) {
            return true;
        }
        if commands::handle_event(event, binding, system_state) {
            return true;
        }
        false
    }

    impl_owns_capability!(
        "SystemAudio",
        "Mute",
        "VolumeUp",
        "VolumeDown",
        "Microphone",
        "MicMute",
        "MicVolumeUp",
        "MicVolumeDown",
        "MediaPlayPause",
        "MediaNext",
        "MediaPrevious",
        "MediaStop",
        "RunCommand",
        "LaunchApp",
        "OpenURL"
    );

    fn is_active(&self, binding: &Binding, system_state: &SystemState) -> bool {
        // Check each module
        if audio::is_active(binding, system_state) {
            return true;
        }
        if media::is_active(binding, system_state) {
            return true;
        }
        if commands::is_active(binding, system_state) {
            return true;
        }
        false
    }

    fn initialize(&mut self, _config: &PluginConfig) -> anyhow::Result<()> {
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
        "Audio, media controls, and shell commands"
    }

    fn documentation(&self) -> &'static str {
        include_str!("../../docs/plugins/core.md")
    }

    fn icon(&self) -> &'static str {
        "⚡"
    }

    fn is_core(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_plugin_has_all_capabilities() {
        let plugin = CorePlugin::new();
        let caps = plugin.capabilities();

        // Should have all core capabilities
        let ids: Vec<_> = caps.iter().map(|c| c.id).collect();
        assert!(ids.contains(&"SystemAudio"));
        assert!(ids.contains(&"Mute"));
        assert!(ids.contains(&"MediaPlayPause"));
        assert!(ids.contains(&"RunCommand"));
        assert!(ids.contains(&"LaunchApp"));
        assert!(ids.contains(&"OpenURL"));
    }

    #[test]
    fn core_plugin_owns_its_capabilities() {
        let plugin = CorePlugin::new();
        assert!(plugin.owns_capability("SystemAudio"));
        assert!(plugin.owns_capability("Mute"));
        assert!(plugin.owns_capability("MediaPlayPause"));
        assert!(plugin.owns_capability("RunCommand"));
        assert!(!plugin.owns_capability("ElgatoKeyLight"));
    }

    #[test]
    fn core_plugin_metadata_id() {
        let plugin = CorePlugin::new();
        assert_eq!(plugin.id(), "core");
    }

    #[test]
    fn core_plugin_metadata_name() {
        let plugin = CorePlugin::new();
        assert_eq!(plugin.name(), "Core");
    }

    #[test]
    fn core_plugin_metadata_category() {
        let plugin = CorePlugin::new();
        assert_eq!(plugin.category(), "Core");
    }

    #[test]
    fn core_plugin_metadata_version() {
        let plugin = CorePlugin::new();
        assert_eq!(plugin.version(), "1.0.0");
    }

    #[test]
    fn core_plugin_metadata_description() {
        let plugin = CorePlugin::new();
        assert_eq!(plugin.description(), "Audio, media controls, and shell commands");
    }

    #[test]
    fn core_plugin_metadata_icon() {
        let plugin = CorePlugin::new();
        assert_eq!(plugin.icon(), "⚡");
    }

    #[test]
    fn core_plugin_is_core() {
        let plugin = CorePlugin::new();
        assert!(plugin.is_core());
    }

    #[test]
    fn core_plugin_documentation_not_empty() {
        let plugin = CorePlugin::new();
        let docs = plugin.documentation();
        assert!(!docs.is_empty());
        assert!(docs.contains("Core Plugin"));
    }
}
