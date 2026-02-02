//! Debounced OBS audio volume controller for smooth adjustments.
//!
//! Accumulates encoder deltas and sends a single WebSocket request after a debounce window,
//! preventing lag when turning the encoder quickly.

use super::client::{self, OBSConnection};
use crate::state_manager::{OBSState, SystemState};
use crate::streamdeck::request_image_sync;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Debounce window - accumulate deltas for this long before sending
const DEBOUNCE_MS: u64 = 80;

/// Pending volume adjustment for a specific input
#[derive(Debug)]
struct PendingAdjustment {
    /// OBS connection parameters
    conn: OBSConnection,
    /// Input name to adjust
    input_name: String,
    /// Accumulated volume delta (multiplier, 0.0-1.0)
    delta: f32,
    /// When the first delta in this batch was received
    first_delta_at: Instant,
    /// Last known volume (to avoid GET requests)
    cached_volume: Option<f32>,
}

/// Thread-safe controller for debounced OBS volume adjustments
pub struct OBSAudioController {
    /// Pending adjustments per input ("host:port:input_name" -> adjustment)
    pending: Arc<Mutex<HashMap<String, PendingAdjustment>>>,
    /// Flag to signal the worker thread to process pending adjustments
    has_pending: Arc<Mutex<bool>>,
    /// Reference to system state for updating OBS state
    #[allow(dead_code)]
    system_state: Arc<Mutex<SystemState>>,
}

impl OBSAudioController {
    /// Create a new controller and start the background worker thread
    pub fn new(system_state: Arc<Mutex<SystemState>>) -> Self {
        let pending: Arc<Mutex<HashMap<String, PendingAdjustment>>> = Arc::new(Mutex::new(HashMap::new()));
        let has_pending = Arc::new(Mutex::new(false));

        // Start background worker
        let pending_clone = pending.clone();
        let has_pending_clone = has_pending.clone();
        let state_clone = system_state.clone();
        thread::spawn(move || {
            worker_loop(pending_clone, has_pending_clone, state_clone);
        });

        Self { pending, has_pending, system_state }
    }

    /// Queue a volume adjustment (will be debounced and sent in batch)
    pub fn queue_volume_delta(&self, conn: &OBSConnection, input_name: &str, delta: f32) {
        let key = format!("{}:{}:{}", conn.host, conn.port, input_name);

        if let Ok(mut pending) = self.pending.lock() {
            let entry = pending.entry(key.clone()).or_insert_with(|| PendingAdjustment {
                conn: conn.clone(),
                input_name: input_name.to_string(),
                delta: 0.0,
                first_delta_at: Instant::now(),
                cached_volume: None,
            });

            entry.delta += delta;

            // If this is the first delta in the batch, record the time
            if (entry.delta - delta).abs() < f32::EPSILON {
                entry.first_delta_at = Instant::now();
            }
        }

        // Signal worker that there's work to do
        if let Ok(mut has_pending) = self.has_pending.lock() {
            *has_pending = true;
        }
    }

    /// Update cached volume for an input (call after mute toggle)
    #[allow(dead_code)]
    pub fn update_cached_volume(&self, conn: &OBSConnection, input_name: &str, volume: f32) {
        let key = format!("{}:{}:{}", conn.host, conn.port, input_name);

        if let Ok(mut pending) = self.pending.lock() {
            if let Some(entry) = pending.get_mut(&key) {
                entry.cached_volume = Some(volume);
            } else {
                // Create an entry just for caching
                pending.insert(key, PendingAdjustment {
                    conn: conn.clone(),
                    input_name: input_name.to_string(),
                    delta: 0.0,
                    first_delta_at: Instant::now(),
                    cached_volume: Some(volume),
                });
            }
        }
    }

    /// Get cached volume for an input (if available)
    #[allow(dead_code)]
    pub fn get_cached_volume(&self, conn: &OBSConnection, input_name: &str) -> Option<f32> {
        let key = format!("{}:{}:{}", conn.host, conn.port, input_name);
        self.pending.lock().ok()?.get(&key)?.cached_volume
    }

    /// Update OBS state after an action
    #[allow(dead_code)]
    pub fn update_obs_state<F>(&self, conn: &OBSConnection, update_fn: F)
    where
        F: FnOnce(&mut OBSState),
    {
        if let Ok(mut state) = self.system_state.lock() {
            let key = conn.key();
            let obs_state = state.obs_states.entry(key).or_insert_with(OBSState::default);
            update_fn(obs_state);
        }
    }
}

/// Background worker that processes pending adjustments after debounce window
fn worker_loop(
    pending: Arc<Mutex<HashMap<String, PendingAdjustment>>>,
    has_pending: Arc<Mutex<bool>>,
    system_state: Arc<Mutex<SystemState>>,
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
        let ready_adjustments: Vec<(String, OBSConnection, String, f32, Option<f32>)> = {
            let mut pending_lock = match pending.lock() {
                Ok(p) => p,
                Err(_) => continue,
            };

            let now = Instant::now();
            let mut ready = Vec::new();

            for (key, adj) in pending_lock.iter_mut() {
                if adj.delta.abs() > f32::EPSILON
                    && now.duration_since(adj.first_delta_at) >= Duration::from_millis(DEBOUNCE_MS)
                {
                    ready.push((
                        key.clone(),
                        adj.conn.clone(),
                        adj.input_name.clone(),
                        adj.delta,
                        adj.cached_volume,
                    ));
                    // Reset delta but keep cache
                    adj.delta = 0.0;
                }
            }

            // Clear has_pending flag if no more work
            let any_pending = pending_lock.values().any(|a| a.delta.abs() > f32::EPSILON);
            if !any_pending {
                if let Ok(mut h) = has_pending.lock() {
                    *h = false;
                }
            }

            ready
        };

        // Process ready adjustments (outside the lock)
        let mut any_applied = false;
        for (key, conn, input_name, delta, cached_volume) in ready_adjustments {
            // Apply the adjustment
            let result = apply_volume_delta(&conn, &input_name, delta, cached_volume);

            // Update cache with result
            if let Ok(new_volume) = result {
                any_applied = true;
                if let Ok(mut pending_lock) = pending.lock() {
                    if let Some(adj) = pending_lock.get_mut(&key) {
                        adj.cached_volume = Some(new_volume);
                    }
                }

                // Update system state
                if let Ok(mut state) = system_state.lock() {
                    let obs_key = conn.key();
                    // Ensure OBSState exists but don't modify mute state here
                    state.obs_states.entry(obs_key).or_insert_with(OBSState::default);
                }
            }
        }

        // Trigger image sync if any adjustments were applied
        if any_applied {
            request_image_sync();
        }
    }
}

/// Apply accumulated volume delta to an OBS input
/// Returns the new volume on success
fn apply_volume_delta(
    conn: &OBSConnection,
    input_name: &str,
    delta: f32,
    cached_volume: Option<f32>,
) -> Result<f32, ()> {
    // Use cached value if available, otherwise fetch
    let current_volume = match cached_volume {
        Some(v) => v,
        None => {
            // Need to fetch current volume
            match client::get_input_volume(conn, input_name) {
                Ok(vol) => vol,
                Err(_) => return Err(()),
            }
        }
    };

    // Calculate new volume (clamped to 0.0-1.0)
    let new_volume = (current_volume + delta).clamp(0.0, 1.0);

    // Send the update
    if client::set_input_volume(conn, input_name, new_volume).is_err() {
        return Err(());
    }

    Ok(new_volume)
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
    fn test_volume_calculation() {
        // Test clamping at max
        let new = (0.95f32 + 0.20).clamp(0.0, 1.0);
        assert!((new - 1.0).abs() < f32::EPSILON);

        // Test clamping at min
        let new = (0.10f32 - 0.30).clamp(0.0, 1.0);
        assert!((new - 0.0).abs() < f32::EPSILON);

        // Test normal adjustment
        let new = (0.50f32 + 0.10).clamp(0.0, 1.0);
        assert!((new - 0.60).abs() < f32::EPSILON);
    }

    #[test]
    fn test_pending_key_format() {
        let key = format!("{}:{}:{}", "192.168.1.50", 4455, "Mic/Aux");
        assert_eq!(key, "192.168.1.50:4455:Mic/Aux");
    }
}
