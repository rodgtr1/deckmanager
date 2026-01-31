//! Debounced Key Light controller for smooth brightness adjustments.
//!
//! Accumulates encoder deltas and sends a single HTTP request after a debounce window,
//! preventing lag when turning the encoder quickly.

use crate::elgato_key_light::{self, KeyLightState};
use crate::streamdeck::request_image_sync;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Debounce window - accumulate deltas for this long before sending
const DEBOUNCE_MS: u64 = 80;

/// Pending brightness adjustment for a specific light
#[derive(Debug)]
struct PendingAdjustment {
    /// Accumulated brightness delta
    delta: i32,
    /// When the first delta in this batch was received
    first_delta_at: Instant,
    /// Last known brightness (to avoid GET requests)
    cached_brightness: Option<u8>,
    /// Last known on/off state
    cached_on: Option<bool>,
}

/// Thread-safe controller for debounced Key Light brightness adjustments
pub struct KeyLightController {
    /// Pending adjustments per light ("ip:port" -> adjustment)
    pending: Arc<Mutex<HashMap<String, PendingAdjustment>>>,
    /// Flag to signal the worker thread to process pending adjustments
    has_pending: Arc<Mutex<bool>>,
}

impl KeyLightController {
    /// Create a new controller and start the background worker thread
    pub fn new() -> Self {
        let pending: Arc<Mutex<HashMap<String, PendingAdjustment>>> = Arc::new(Mutex::new(HashMap::new()));
        let has_pending = Arc::new(Mutex::new(false));

        // Start background worker
        let pending_clone = pending.clone();
        let has_pending_clone = has_pending.clone();
        thread::spawn(move || {
            worker_loop(pending_clone, has_pending_clone);
        });

        Self { pending, has_pending }
    }

    /// Queue a brightness adjustment (will be debounced and sent in batch)
    pub fn queue_brightness_delta(&self, ip: &str, port: u16, delta: i32) {
        let key = format!("{}:{}", ip, port);

        if let Ok(mut pending) = self.pending.lock() {
            let entry = pending.entry(key.clone()).or_insert_with(|| PendingAdjustment {
                delta: 0,
                first_delta_at: Instant::now(),
                cached_brightness: None,
                cached_on: None,
            });

            entry.delta += delta;

            // If this is the first delta in the batch, record the time
            if entry.delta == delta {
                entry.first_delta_at = Instant::now();
            }
        }

        // Signal worker that there's work to do
        if let Ok(mut has_pending) = self.has_pending.lock() {
            *has_pending = true;
        }
    }

    /// Update cached state for a light (call after toggle/on/off)
    pub fn update_cached_state(&self, ip: &str, port: u16, state: &KeyLightState) {
        let key = format!("{}:{}", ip, port);

        if let Ok(mut pending) = self.pending.lock() {
            if let Some(entry) = pending.get_mut(&key) {
                entry.cached_brightness = Some(state.brightness);
                entry.cached_on = Some(state.on);
            } else {
                // Create an entry just for caching
                pending.insert(key, PendingAdjustment {
                    delta: 0,
                    first_delta_at: Instant::now(),
                    cached_brightness: Some(state.brightness),
                    cached_on: Some(state.on),
                });
            }
        }
    }

    /// Get cached brightness for a light (if available)
    #[allow(dead_code)]
    pub fn get_cached_brightness(&self, ip: &str, port: u16) -> Option<u8> {
        let key = format!("{}:{}", ip, port);
        self.pending.lock().ok()?.get(&key)?.cached_brightness
    }
}

impl Default for KeyLightController {
    fn default() -> Self {
        Self::new()
    }
}

/// Background worker that processes pending adjustments after debounce window
fn worker_loop(
    pending: Arc<Mutex<HashMap<String, PendingAdjustment>>>,
    has_pending: Arc<Mutex<bool>>,
) {
    loop {
        thread::sleep(Duration::from_millis(20)); // Check every 20ms

        // Check if there's any pending work
        let should_process = {
            let has = has_pending.lock().ok().map(|h| *h).unwrap_or(false);
            has
        };

        if !should_process {
            continue;
        }

        // Find adjustments that are ready to send (debounce window elapsed)
        let ready_adjustments: Vec<(String, i32, Option<u8>, Option<bool>)> = {
            let mut pending_lock = match pending.lock() {
                Ok(p) => p,
                Err(_) => continue,
            };

            let now = Instant::now();
            let mut ready = Vec::new();

            for (key, adj) in pending_lock.iter_mut() {
                if adj.delta != 0 && now.duration_since(adj.first_delta_at) >= Duration::from_millis(DEBOUNCE_MS) {
                    ready.push((
                        key.clone(),
                        adj.delta,
                        adj.cached_brightness,
                        adj.cached_on,
                    ));
                    // Reset delta but keep cache
                    adj.delta = 0;
                }
            }

            // Clear has_pending flag if no more work
            let any_pending = pending_lock.values().any(|a| a.delta != 0);
            if !any_pending {
                if let Ok(mut h) = has_pending.lock() {
                    *h = false;
                }
            }

            ready
        };

        // Process ready adjustments (outside the lock)
        let mut any_applied = false;
        for (key, delta, cached_brightness, cached_on) in ready_adjustments {
            let parts: Vec<&str> = key.split(':').collect();
            if parts.len() != 2 {
                continue;
            }
            let ip = parts[0];
            let port: u16 = match parts[1].parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Apply the adjustment
            let result = apply_brightness_delta(ip, port, delta, cached_brightness, cached_on);

            // Update cache with result
            if let Ok((new_brightness, new_on)) = result {
                any_applied = true;
                if let Ok(mut pending_lock) = pending.lock() {
                    if let Some(adj) = pending_lock.get_mut(&key) {
                        adj.cached_brightness = Some(new_brightness);
                        adj.cached_on = Some(new_on);
                    }
                }
            }
        }

        // Trigger image sync if any adjustments were applied
        if any_applied {
            request_image_sync();
        }
    }
}

/// Apply accumulated brightness delta to a light
/// Returns (new_brightness, is_on) on success
fn apply_brightness_delta(
    ip: &str,
    port: u16,
    delta: i32,
    cached_brightness: Option<u8>,
    cached_on: Option<bool>,
) -> Result<(u8, bool), ()> {
    // Use cached values if available, otherwise fetch
    let (current_brightness, current_on) = match (cached_brightness, cached_on) {
        (Some(b), Some(on)) => (b, on),
        _ => {
            // Need to fetch current state
            match elgato_key_light::get_state(ip, port) {
                Ok(state) => (state.brightness, state.on),
                Err(_) => return Err(()),
            }
        }
    };

    // Calculate new brightness
    let new_brightness = ((current_brightness as i32) + delta).clamp(0, 100) as u8;

    // Determine on/off state
    let should_be_on = current_on || delta > 0;
    let final_on = should_be_on && new_brightness > 0;

    // Send the update
    if elgato_key_light::set_state(ip, port, final_on, new_brightness).is_err() {
        return Err(());
    }

    Ok((new_brightness, final_on))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debounce_window_constant() {
        // Debounce should be reasonable (50-150ms)
        assert!(DEBOUNCE_MS >= 50);
        assert!(DEBOUNCE_MS <= 150);
    }

    #[test]
    fn test_pending_adjustment_delta_accumulation() {
        let mut adj = PendingAdjustment {
            delta: 0,
            first_delta_at: Instant::now(),
            cached_brightness: Some(50),
            cached_on: Some(true),
        };

        adj.delta += 5;
        adj.delta += 3;
        adj.delta -= 2;

        assert_eq!(adj.delta, 6);
    }

    #[test]
    fn test_brightness_calculation() {
        // Test clamping at max
        let new = ((95i32) + 20).clamp(0, 100) as u8;
        assert_eq!(new, 100);

        // Test clamping at min
        let new = ((10i32) - 30).clamp(0, 100) as u8;
        assert_eq!(new, 0);

        // Test normal adjustment
        let new = ((50i32) + 10).clamp(0, 100) as u8;
        assert_eq!(new, 60);
    }
}
