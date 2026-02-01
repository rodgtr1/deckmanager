//! Plugin system types for capability metadata and parameters.

use serde::{Deserialize, Serialize};

/// Metadata describing a capability provided by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMetadata {
    /// Unique identifier (e.g., "SystemAudio", "ElgatoKeyLight")
    pub id: &'static str,
    /// Human-readable name
    pub name: &'static str,
    /// Description of what this capability does
    pub description: &'static str,
    /// Plugin ID that provides this capability
    pub plugin_id: &'static str,
    /// Whether this capability can be bound to buttons
    pub supports_button: bool,
    /// Whether this capability can be bound to encoder rotation
    pub supports_encoder: bool,
    /// Whether this capability can be bound to encoder press
    pub supports_encoder_press: bool,
    /// Parameters this capability accepts
    pub parameters: Vec<ParameterDef>,
}

/// Definition of a capability parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    /// Parameter name (e.g., "step", "command", "ip")
    pub name: &'static str,
    /// Parameter type for UI rendering
    pub param_type: ParameterType,
    /// Default value as string
    pub default_value: &'static str,
    /// Description for UI tooltip
    pub description: &'static str,
}

/// Parameter types for capability configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    /// Floating point number (e.g., volume step)
    Float,
    /// Integer number
    Integer,
    /// Text string (e.g., command, URL)
    String,
    /// Boolean flag
    Bool,
    /// IP address
    IpAddress,
}

impl ParameterType {
    /// Get the string representation for frontend compatibility
    pub fn as_str(&self) -> &'static str {
        match self {
            ParameterType::Float => "f32",
            ParameterType::Integer => "i32",
            ParameterType::String => "string",
            ParameterType::Bool => "bool",
            ParameterType::IpAddress => "string",
        }
    }
}

/// Plugin configuration from settings file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Whether this plugin is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Plugin-specific settings (e.g., WebSocket URL for OBS)
    #[serde(flatten)]
    pub settings: std::collections::HashMap<String, toml::Value>,
}

fn default_enabled() -> bool {
    true
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            settings: std::collections::HashMap::new(),
        }
    }
}

/// Information about a plugin for the frontend.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Unique plugin identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Plugin category (e.g., "Core", "Lighting", "Streaming")
    pub category: String,
    /// Whether the plugin is currently enabled
    pub enabled: bool,
    /// Number of capabilities provided
    pub capability_count: usize,
    /// Plugin version string
    pub version: String,
    /// Short description of what the plugin does
    pub description: String,
    /// Markdown documentation for the plugin
    pub documentation: String,
    /// Icon/emoji for the plugin
    pub icon: String,
    /// Whether this is a core plugin (cannot be disabled)
    pub is_core: bool,
}
