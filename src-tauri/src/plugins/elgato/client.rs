//! Elgato Key Light API client
//!
//! Controls Elgato Key Light devices via their HTTP API.
//! API endpoint: http://{ip}:{port}/elgato/lights

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::LazyLock;
use std::thread;
use std::time::Duration;

/// Shared HTTP client with connection pooling for better performance
static HTTP_CLIENT: LazyLock<reqwest::blocking::Client> = LazyLock::new(|| {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .pool_max_idle_per_host(5)
        .build()
        .expect("Failed to create HTTP client")
});

/// Number of retry attempts for network operations
const MAX_RETRIES: u32 = 2;

/// Delay between retry attempts
const RETRY_DELAY: Duration = Duration::from_millis(100);

/// Validate that an IP address is safe to connect to (private/local network only)
fn validate_ip(ip: &str) -> Result<()> {
    let addr: IpAddr = ip.parse().context("Invalid IP address format")?;

    let is_safe = match addr {
        IpAddr::V4(v4) => {
            v4.is_private()      // 10.x.x.x, 172.16-31.x.x, 192.168.x.x
                || v4.is_loopback()  // 127.x.x.x
                || v4.is_link_local() // 169.254.x.x
        }
        IpAddr::V6(v6) => {
            v6.is_loopback() // ::1
        }
    };

    if !is_safe {
        anyhow::bail!(
            "Key Light IP address must be on a private/local network, got: {}",
            ip
        );
    }

    Ok(())
}

/// Execute a fallible operation with retries
fn with_retry<T, F>(mut operation: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    let mut last_error = None;

    for attempt in 0..=MAX_RETRIES {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < MAX_RETRIES {
                    thread::sleep(RETRY_DELAY);
                }
            }
        }
    }

    Err(last_error.unwrap())
}

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

/// Get the current state of a Key Light (with retry)
pub fn get_state(ip: &str, port: u16) -> Result<KeyLightState> {
    validate_ip(ip)?;
    let url = format!("http://{}:{}/elgato/lights", ip, port);

    with_retry(|| {
        let response: LightsResponse = HTTP_CLIENT
            .get(&url)
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
    })
}

/// Set the state of a Key Light (with retry)
pub fn set_state(ip: &str, port: u16, on: bool, brightness: u8) -> Result<()> {
    validate_ip(ip)?;
    let url = format!("http://{}:{}/elgato/lights", ip, port);

    let request = LightsRequest {
        lights: vec![LightRequestData {
            on: if on { 1 } else { 0 },
            brightness: brightness.clamp(0, 100),
            temperature: None, // Keep current temperature
        }],
    };

    with_retry(|| {
        HTTP_CLIENT
            .put(&url)
            .json(&request)
            .send()
            .context("Failed to send command to Key Light")?;

        Ok(())
    })
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
#[allow(dead_code)]
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
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_retry_constants() {
        // Retry count should be reasonable (1-5)
        assert!(MAX_RETRIES >= 1);
        assert!(MAX_RETRIES <= 5);

        // Retry delay should be reasonable (50-500ms)
        assert!(RETRY_DELAY >= Duration::from_millis(50));
        assert!(RETRY_DELAY <= Duration::from_millis(500));
    }

    #[test]
    fn test_with_retry_succeeds_first_try() {
        let call_count = AtomicU32::new(0);

        let result = with_retry(|| {
            call_count.fetch_add(1, Ordering::SeqCst);
            Ok::<_, anyhow::Error>(42)
        });

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_with_retry_succeeds_after_failures() {
        let call_count = AtomicU32::new(0);

        let result = with_retry(|| {
            let count = call_count.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Err(anyhow::anyhow!("temporary failure"))
            } else {
                Ok(42)
            }
        });

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 3); // 2 failures + 1 success
    }

    #[test]
    fn test_with_retry_fails_after_max_retries() {
        let call_count = AtomicU32::new(0);

        let result: Result<i32> = with_retry(|| {
            call_count.fetch_add(1, Ordering::SeqCst);
            Err(anyhow::anyhow!("persistent failure"))
        });

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), MAX_RETRIES + 1);
    }

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

    #[test]
    fn test_key_light_state_serialization() {
        // Test KeyLightState serialization round-trip
        let state = KeyLightState {
            on: true,
            brightness: 75,
            temperature: 200,
        };

        let json = serde_json::to_string(&state).unwrap();
        let parsed: KeyLightState = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.on, state.on);
        assert_eq!(parsed.brightness, state.brightness);
        assert_eq!(parsed.temperature, state.temperature);
    }
}
