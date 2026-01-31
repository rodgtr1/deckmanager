use crate::binding::{Binding, InputRef};
use crate::capability::Capability;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Current config schema version for future migration support
const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    /// Schema version for migration support
    #[serde(default = "default_version")]
    version: u32,
    #[serde(rename = "bindings")]
    bindings: Vec<Binding>,
}

fn default_version() -> u32 {
    1
}

/// Returns the path to the config file: ~/.config/{app_name}/bindings.toml
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join(crate::app_constants::APP_NAME_LOWER).join("bindings.toml"))
}

/// Save bindings to the config file.
/// Uses atomic writes (write to temp, then rename) to prevent corruption.
/// Keeps a .bak backup of the previous config.
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
        version: CONFIG_VERSION,
        bindings: bindings.to_vec(),
    };

    let contents = toml::to_string_pretty(&config)
        .context("Failed to serialize bindings to TOML")?;

    // Atomic write: write to temp file, then rename
    let tmp_path = path.with_extension("toml.tmp");
    let bak_path = path.with_extension("toml.bak");

    // Write to temporary file first
    fs::write(&tmp_path, &contents)
        .with_context(|| format!("Failed to write temp config file: {}", tmp_path.display()))?;

    // Backup existing config if it exists
    if path.exists() {
        // Remove old backup if exists (ignore errors)
        let _ = fs::remove_file(&bak_path);
        // Rename current to backup
        fs::rename(&path, &bak_path)
            .with_context(|| format!("Failed to backup config file: {}", path.display()))?;
    }

    // Atomic rename temp to final
    fs::rename(&tmp_path, &path)
        .with_context(|| format!("Failed to finalize config file: {}", path.display()))?;

    Ok(())
}

/// Load bindings from the config file, or return defaults if it doesn't exist.
/// If the main config is corrupted, attempts to load from backup.
pub fn load_bindings() -> Result<Vec<Binding>> {
    let Some(path) = config_path() else {
        return Ok(default_bindings());
    };

    if !path.exists() {
        // Try backup if main config doesn't exist
        let bak_path = path.with_extension("toml.bak");
        if bak_path.exists() {
            eprintln!("Main config missing, loading from backup: {}", bak_path.display());
            return load_from_path(&bak_path);
        }
        return Ok(default_bindings());
    }

    // Try to load main config
    match load_from_path(&path) {
        Ok(bindings) => Ok(bindings),
        Err(e) => {
            // Main config corrupted, try backup
            let bak_path = path.with_extension("toml.bak");
            if bak_path.exists() {
                eprintln!("Main config corrupted ({}), loading from backup", e);
                return load_from_path(&bak_path);
            }
            Err(e)
        }
    }
}

/// Load bindings from a specific path
fn load_from_path(path: &PathBuf) -> Result<Vec<Binding>> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: Config = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    // Future: handle migrations based on config.version
    if config.version > CONFIG_VERSION {
        eprintln!(
            "Warning: Config version {} is newer than supported version {}",
            config.version, CONFIG_VERSION
        );
    }

    Ok(config.bindings)
}

/// Default bindings when no config file exists.
pub fn default_bindings() -> Vec<Binding> {
    vec![
        // System Audio on encoder 0 - rotation for volume, press for mute
        Binding {
            input: InputRef::Encoder { index: 0 },
            capability: Capability::SystemAudio { step: 0.02 },
            page: 0,
            icon: None,
            label: None,
            button_image: None,
            button_image_alt: None,
            show_label: None,
        },
        Binding {
            input: InputRef::EncoderPress { index: 0 },
            capability: Capability::SystemAudio { step: 0.02 },
            page: 0,
            icon: None,
            label: None,
            button_image: None,
            button_image_alt: None,
            show_label: None,
        },
        Binding {
            input: InputRef::EncoderPress { index: 1 },
            capability: Capability::MediaPlayPause,
            page: 0,
            icon: None,
            label: None,
            button_image: None,
            button_image_alt: None,
            show_label: None,
        },
        Binding {
            input: InputRef::Button { index: 0 },
            capability: Capability::MediaPlayPause,
            page: 0,
            icon: None,
            label: None,
            button_image: None,
            button_image_alt: None,
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
            version: CONFIG_VERSION,
            bindings: bindings.clone(),
        };

        let toml_str = toml::to_string_pretty(&config).expect("serialize");
        let parsed: Config = toml::from_str(&toml_str).expect("deserialize");

        assert_eq!(parsed.version, CONFIG_VERSION);
        assert_eq!(parsed.bindings.len(), bindings.len());
    }

    #[test]
    fn version_defaults_to_one() {
        // Old configs without version field should default to 1
        let toml = r#"
[[bindings]]
[bindings.input]
type = "Button"
index = 0
[bindings.capability]
type = "MediaPlayPause"
"#;
        let config: Config = toml::from_str(toml).expect("parse");
        assert_eq!(config.version, 1);
    }

    #[test]
    fn parse_system_audio_binding() {
        let toml = r#"
[[bindings]]
[bindings.input]
type = "Encoder"
index = 0

[bindings.capability]
type = "SystemAudio"
step = 0.05
"#;

        let config: Config = toml::from_str(toml).expect("parse");
        assert_eq!(config.bindings.len(), 1);

        match &config.bindings[0].capability {
            Capability::SystemAudio { step } => assert_eq!(*step, 0.05),
            _ => panic!("expected SystemAudio"),
        }
    }

    #[test]
    fn parse_microphone_binding() {
        let toml = r#"
[[bindings]]
[bindings.input]
type = "EncoderPress"
index = 0

[bindings.capability]
type = "Microphone"
step = 0.02
"#;

        let config: Config = toml::from_str(toml).expect("parse");
        assert_eq!(config.bindings.len(), 1);
        assert_eq!(config.bindings[0].capability, Capability::Microphone { step: 0.02 });
    }

    #[test]
    fn parse_multiple_bindings() {
        let toml = r#"
[[bindings]]
[bindings.input]
type = "Encoder"
index = 0
[bindings.capability]
type = "SystemAudio"
step = 0.02

[[bindings]]
[bindings.input]
type = "EncoderPress"
index = 0
[bindings.capability]
type = "Microphone"
step = 0.02
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
