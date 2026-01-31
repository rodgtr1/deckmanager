//! Elgato Key Light API client
//!
//! Controls Elgato Key Light devices via their HTTP API.
//! API endpoint: http://{ip}:{port}/elgato/lights

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// State of a single Key Light
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyLightState {
    pub on: bool,
    pub brightness: u8,     // 0-100
    pub temperature: u16,   // 143-344 (2900K-7000K)
}

/// Response from the Key Light API
#[derive(Debug, Deserialize)]
struct LightsResponse {
    lights: Vec<LightData>,
}

/// Individual light data from API
#[derive(Debug, Deserialize)]
struct LightData {
    on: u8,         // 0 or 1
    brightness: u8, // 0-100
    temperature: u16,
}

/// Request body for setting light state
#[derive(Debug, Serialize)]
struct LightsRequest {
    lights: Vec<LightRequestData>,
}

#[derive(Debug, Serialize)]
struct LightRequestData {
    on: u8,
    brightness: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<u16>,
}

/// Get the current state of a Key Light
pub fn get_state(ip: &str, port: u16) -> Result<KeyLightState> {
    let url = format!("http://{}:{}/elgato/lights", ip, port);

    let response: LightsResponse = reqwest::blocking::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .context("Failed to connect to Key Light")?
        .json()
        .context("Failed to parse Key Light response")?;

    let light = response
        .lights
        .first()
        .context("No lights found in response")?;

    Ok(KeyLightState {
        on: light.on == 1,
        brightness: light.brightness,
        temperature: light.temperature,
    })
}

/// Set the state of a Key Light
pub fn set_state(ip: &str, port: u16, on: bool, brightness: u8) -> Result<()> {
    let url = format!("http://{}:{}/elgato/lights", ip, port);

    let request = LightsRequest {
        lights: vec![LightRequestData {
            on: if on { 1 } else { 0 },
            brightness: brightness.clamp(0, 100),
            temperature: None, // Keep current temperature
        }],
    };

    reqwest::blocking::Client::new()
        .put(&url)
        .timeout(std::time::Duration::from_secs(2))
        .json(&request)
        .send()
        .context("Failed to send command to Key Light")?;

    Ok(())
}

/// Toggle the Key Light on/off
/// Returns the new on state
pub fn toggle(ip: &str, port: u16) -> Result<bool> {
    let current = get_state(ip, port)?;
    let new_on = !current.on;

    // When turning on, use full brightness if it was 0
    let brightness = if new_on && current.brightness == 0 {
        100
    } else {
        current.brightness
    };

    set_state(ip, port, new_on, brightness)?;
    Ok(new_on)
}

/// Turn the Key Light on
pub fn turn_on(ip: &str, port: u16) -> Result<()> {
    let current = get_state(ip, port)?;
    let brightness = if current.brightness == 0 { 100 } else { current.brightness };
    set_state(ip, port, true, brightness)
}

/// Turn the Key Light off
pub fn turn_off(ip: &str, port: u16) -> Result<()> {
    set_state(ip, port, false, 0)
}

/// Adjust brightness by delta (-100 to +100)
/// Returns the new brightness level
pub fn adjust_brightness(ip: &str, port: u16, delta: i32) -> Result<u8> {
    let current = get_state(ip, port)?;

    // If light is off and we're increasing brightness, turn it on
    let should_be_on = current.on || delta > 0;

    let new_brightness = ((current.brightness as i32) + delta).clamp(0, 100) as u8;

    // If brightness drops to 0, turn off the light
    let final_on = should_be_on && new_brightness > 0;

    set_state(ip, port, final_on, new_brightness)?;
    Ok(new_brightness)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brightness_clamping() {
        // Test that brightness is clamped to 0-100
        let request = LightRequestData {
            on: 1,
            brightness: 150u8.clamp(0, 100),
            temperature: None,
        };
        assert_eq!(request.brightness, 100);
    }
}
