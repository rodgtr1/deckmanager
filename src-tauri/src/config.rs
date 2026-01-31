use crate::binding::{Binding, InputRef};
use crate::capability::Capability;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    #[serde(rename = "bindings")]
    bindings: Vec<Binding>,
}

/// Returns the path to the config file: ~/.config/archdeck/bindings.toml
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("archdeck").join("bindings.toml"))
}

/// Save bindings to the config file.
pub fn save_bindings(bindings: &[Binding]) -> Result<()> {
    let Some(path) = config_path() else {
        anyhow::bail!("Could not determine config directory");
    };

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let config = Config {
        bindings: bindings.to_vec(),
    };

    let contents = toml::to_string_pretty(&config)
        .context("Failed to serialize bindings to TOML")?;

    fs::write(&path, contents)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    Ok(())
}

/// Load bindings from the config file, or return defaults if it doesn't exist.
pub fn load_bindings() -> Result<Vec<Binding>> {
    let Some(path) = config_path() else {
        return Ok(default_bindings());
    };

    if !path.exists() {
        return Ok(default_bindings());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: Config = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    Ok(config.bindings)
}

/// Default bindings when no config file exists.
pub fn default_bindings() -> Vec<Binding> {
    vec![
        Binding {
            input: InputRef::Encoder { index: 0 },
            capability: Capability::SystemVolume { step: 0.02 },
            icon: None,
            label: None,
            button_image: None,
            show_label: None,
        },
        Binding {
            input: InputRef::EncoderPress { index: 0 },
            capability: Capability::ToggleMute,
            icon: None,
            label: None,
            button_image: None,
            show_label: None,
        },
        Binding {
            input: InputRef::EncoderPress { index: 1 },
            capability: Capability::MediaPlayPause,
            icon: None,
            label: None,
            button_image: None,
            show_label: None,
        },
        Binding {
            input: InputRef::Button { index: 0 },
            capability: Capability::MediaPlayPause,
            icon: None,
            label: None,
            button_image: None,
            show_label: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bindings_exist() {
        let bindings = default_bindings();
        assert_eq!(bindings.len(), 4);
    }

    #[test]
    fn toml_roundtrip() {
        let bindings = default_bindings();
        let config = Config {
            bindings: bindings.clone(),
        };

        let toml_str = toml::to_string_pretty(&config).expect("serialize");
        let parsed: Config = toml::from_str(&toml_str).expect("deserialize");

        assert_eq!(parsed.bindings.len(), bindings.len());
    }

    #[test]
    fn parse_volume_binding() {
        let toml = r#"
[[bindings]]
[bindings.input]
type = "Encoder"
index = 0

[bindings.capability]
type = "SystemVolume"
step = 0.05
"#;

        let config: Config = toml::from_str(toml).expect("parse");
        assert_eq!(config.bindings.len(), 1);

        match &config.bindings[0].capability {
            Capability::SystemVolume { step } => assert_eq!(*step, 0.05),
            _ => panic!("expected SystemVolume"),
        }
    }

    #[test]
    fn parse_mute_binding() {
        let toml = r#"
[[bindings]]
[bindings.input]
type = "EncoderPress"
index = 0

[bindings.capability]
type = "ToggleMute"
"#;

        let config: Config = toml::from_str(toml).expect("parse");
        assert_eq!(config.bindings.len(), 1);
        assert_eq!(config.bindings[0].capability, Capability::ToggleMute);
    }

    #[test]
    fn parse_multiple_bindings() {
        let toml = r#"
[[bindings]]
[bindings.input]
type = "Encoder"
index = 0
[bindings.capability]
type = "SystemVolume"
step = 0.02

[[bindings]]
[bindings.input]
type = "EncoderPress"
index = 0
[bindings.capability]
type = "ToggleMute"
"#;

        let config: Config = toml::from_str(toml).expect("parse");
        assert_eq!(config.bindings.len(), 2);
    }

    #[test]
    fn invalid_toml_produces_error() {
        let toml = "this is not valid toml [[[";
        let result: Result<Config, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn parse_example_file() {
        let example = include_str!("../examples/bindings.toml");
        let config: Config = toml::from_str(example).expect("parse example file");
        assert_eq!(config.bindings.len(), 3);
    }
}
