use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Standard Tauri build
    tauri_build::build();

    // Generate app constants from tauri.conf.json
    generate_app_constants();
}

fn generate_app_constants() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let config_path = Path::new(&manifest_dir).join("tauri.conf.json");

    let config_content =
        fs::read_to_string(&config_path).expect("Failed to read tauri.conf.json");

    // Simple JSON parsing for productName (avoid adding serde_json as build dep)
    let product_name =
        extract_json_string(&config_content, "productName").unwrap_or_else(|| "archdeck".to_string());

    // Convert to lowercase for paths/service names
    let app_name_lower = product_name.to_lowercase();

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("app_constants.rs");

    let constants = format!(
        r#"/// Auto-generated from tauri.conf.json - do not edit manually

/// Application display name (e.g., "ArchDeck")
pub const APP_NAME: &str = "{product_name}";

/// Lowercase name for paths, services, etc. (e.g., "archdeck")
pub const APP_NAME_LOWER: &str = "{app_name_lower}";
"#
    );

    fs::write(&dest_path, constants).expect("Failed to write app_constants.rs");

    // Tell Cargo to rerun if tauri.conf.json changes
    println!("cargo:rerun-if-changed=tauri.conf.json");
}

/// Simple extraction of a string value from JSON without full parsing
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    // Find "key": "value" pattern
    let key_pattern = format!("\"{}\"", key);
    let key_pos = json.find(&key_pattern)?;

    // Find the colon after the key
    let after_key = &json[key_pos + key_pattern.len()..];
    let colon_pos = after_key.find(':')?;

    // Find the opening quote of the value
    let after_colon = &after_key[colon_pos + 1..];
    let quote_start = after_colon.find('"')?;

    // Find the closing quote
    let value_start = quote_start + 1;
    let value_content = &after_colon[value_start..];
    let quote_end = value_content.find('"')?;

    Some(value_content[..quote_end].to_string())
}
