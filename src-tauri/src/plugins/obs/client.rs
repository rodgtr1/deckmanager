//! OBS WebSocket API client
//!
//! Controls OBS Studio via the obs-websocket 5.x protocol (built into OBS 28+).
//! Default endpoint: ws://{host}:{port}
//!
//! Uses connection pooling to reuse authenticated WebSocket connections.

use anyhow::{Context, Result};
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};

/// Number of retry attempts for network operations
const MAX_RETRIES: u32 = 2;

/// Delay between retry attempts
const RETRY_DELAY: Duration = Duration::from_millis(100);

/// Maximum age for pooled connections (30 seconds)
const CONNECTION_MAX_AGE: Duration = Duration::from_secs(30);

/// Global request ID counter
static REQUEST_ID: AtomicU32 = AtomicU32::new(1);

/// Generate a unique request ID
fn next_request_id() -> String {
    REQUEST_ID.fetch_add(1, Ordering::SeqCst).to_string()
}

// ─────────────────────────────────────────────────────────────────
// Connection Pool
// ─────────────────────────────────────────────────────────────────

type OBSWebSocket = WebSocket<MaybeTlsStream<std::net::TcpStream>>;

/// Pooled connection entry
struct PooledConnection {
    socket: OBSWebSocket,
    created_at: Instant,
}

/// Global connection pool for OBS WebSocket connections
static CONNECTION_POOL: Mutex<Option<HashMap<String, PooledConnection>>> = Mutex::new(None);

/// Get or initialize the connection pool
fn with_pool<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, PooledConnection>) -> R,
{
    let mut guard = CONNECTION_POOL.lock().unwrap_or_else(|e| e.into_inner());
    let pool = guard.get_or_insert_with(HashMap::new);
    f(pool)
}

/// Take a connection from the pool if available and not expired
fn take_pooled_connection(key: &str) -> Option<OBSWebSocket> {
    with_pool(|pool| {
        if let Some(mut entry) = pool.remove(key) {
            // Check if connection is still fresh
            if entry.created_at.elapsed() < CONNECTION_MAX_AGE {
                return Some(entry.socket);
            }
            // Connection expired, close it
            let _ = entry.socket.close(None);
        }
        None
    })
}

/// Return a connection to the pool for reuse
fn return_pooled_connection(key: String, socket: OBSWebSocket) {
    with_pool(|pool| {
        // Clean up any stale connections first
        pool.retain(|_, entry| entry.created_at.elapsed() < CONNECTION_MAX_AGE);

        // Only pool if we have room (limit to 5 connections)
        if pool.len() < 5 {
            pool.insert(
                key,
                PooledConnection {
                    socket,
                    created_at: Instant::now(),
                },
            );
        }
    });
}

/// Validate that an IP address is safe to connect to (private/local network only)
fn validate_ip(host: &str) -> Result<()> {
    let addr: IpAddr = host.parse().context("Invalid IP address format")?;

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
            "OBS WebSocket host must be on a private/local network, got: {}",
            host
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

/// OBS WebSocket Hello message (server -> client)
#[derive(Debug, Deserialize)]
struct Hello {
    #[serde(rename = "obsWebSocketVersion")]
    _obs_websocket_version: String,
    authentication: Option<AuthChallenge>,
}

/// Authentication challenge from server
#[derive(Debug, Deserialize)]
struct AuthChallenge {
    challenge: String,
    salt: String,
}

/// Identified response (after successful auth)
#[derive(Debug, Deserialize)]
struct Identified {
    #[serde(rename = "negotiatedRpcVersion")]
    _negotiated_rpc_version: u32,
}

/// OBS WebSocket message wrapper
#[derive(Debug, Deserialize)]
struct OBSMessage {
    op: u32,
    d: Value,
}

/// OBS WebSocket op codes
mod op {
    pub const HELLO: u32 = 0;
    pub const IDENTIFY: u32 = 1;
    pub const IDENTIFIED: u32 = 2;
    pub const REQUEST: u32 = 6;
    pub const REQUEST_RESPONSE: u32 = 7;
}

/// Generate authentication string per obs-websocket protocol
fn generate_auth_string(password: &str, challenge: &str, salt: &str) -> String {
    // Step 1: Concatenate password + salt, then SHA256
    let secret_string = format!("{}{}", password, salt);
    let secret_hash = Sha256::digest(secret_string.as_bytes());
    let secret_base64 = base64::engine::general_purpose::STANDARD.encode(secret_hash);

    // Step 2: Concatenate secret_base64 + challenge, then SHA256
    let auth_string = format!("{}{}", secret_base64, challenge);
    let auth_hash = Sha256::digest(auth_string.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(auth_hash)
}

/// Connection parameters for OBS
#[derive(Debug, Clone)]
pub struct OBSConnection {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
}

impl OBSConnection {
    pub fn new(host: &str, port: u16, password: Option<String>) -> Self {
        Self {
            host: host.to_string(),
            port,
            password,
        }
    }

    /// Create a connection key for state tracking
    pub fn key(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Create a new authenticated WebSocket connection to OBS
fn create_connection(conn: &OBSConnection) -> Result<OBSWebSocket> {
    let url = format!("ws://{}:{}", conn.host, conn.port);

    // Connect to OBS WebSocket
    let (mut socket, _response) = connect(&url)
        .context("Failed to connect to OBS WebSocket")?;

    // Step 1: Receive Hello
    let hello_msg = socket.read()
        .context("Failed to read Hello from OBS")?;
    let hello: OBSMessage = serde_json::from_str(&hello_msg.to_text()?)
        .context("Failed to parse Hello message")?;

    if hello.op != op::HELLO {
        anyhow::bail!("Expected Hello message, got op {}", hello.op);
    }

    let hello_data: Hello = serde_json::from_value(hello.d)
        .context("Failed to parse Hello data")?;

    // Step 2: Send Identify (with optional auth)
    let identify = if let Some(auth) = hello_data.authentication {
        if let Some(password) = &conn.password {
            let auth_string = generate_auth_string(password, &auth.challenge, &auth.salt);
            json!({
                "op": op::IDENTIFY,
                "d": {
                    "rpcVersion": 1,
                    "authentication": auth_string
                }
            })
        } else {
            anyhow::bail!("OBS requires authentication but no password provided");
        }
    } else {
        json!({
            "op": op::IDENTIFY,
            "d": {
                "rpcVersion": 1
            }
        })
    };

    socket.send(Message::Text(identify.to_string()))
        .context("Failed to send Identify")?;

    // Step 3: Receive Identified
    let identified_msg = socket.read()
        .context("Failed to read Identified from OBS")?;
    let identified: OBSMessage = serde_json::from_str(&identified_msg.to_text()?)
        .context("Failed to parse Identified message")?;

    if identified.op != op::IDENTIFIED {
        anyhow::bail!("Authentication failed or unexpected message (op {})", identified.op);
    }

    // Parse to verify it's valid
    let _: Identified = serde_json::from_value(identified.d)
        .context("Failed to parse Identified data")?;

    Ok(socket)
}

/// Send a request using a socket, returning the response
fn send_request_on_socket(socket: &mut OBSWebSocket, request_type: &str, request_data: Option<&Value>) -> Result<Value> {
    let request_id = next_request_id();
    let request = if let Some(data) = request_data {
        json!({
            "op": op::REQUEST,
            "d": {
                "requestType": request_type,
                "requestId": request_id,
                "requestData": data
            }
        })
    } else {
        json!({
            "op": op::REQUEST,
            "d": {
                "requestType": request_type,
                "requestId": request_id
            }
        })
    };

    socket.send(Message::Text(request.to_string()))
        .context("Failed to send request")?;

    // Receive Response
    let response_msg = socket.read()
        .context("Failed to read response from OBS")?;
    let response: OBSMessage = serde_json::from_str(&response_msg.to_text()?)
        .context("Failed to parse response message")?;

    if response.op != op::REQUEST_RESPONSE {
        anyhow::bail!("Expected RequestResponse, got op {}", response.op);
    }

    Ok(response.d)
}

/// Execute a single request to OBS and return the response.
/// Uses connection pooling to reuse authenticated WebSocket connections.
pub fn send_request(conn: &OBSConnection, request_type: &str, request_data: Option<Value>) -> Result<Value> {
    validate_ip(&conn.host)?;

    let pool_key = conn.key();

    // Try to use a pooled connection first
    if let Some(mut socket) = take_pooled_connection(&pool_key) {
        match send_request_on_socket(&mut socket, request_type, request_data.as_ref()) {
            Ok(response) => {
                // Success! Return connection to pool for reuse
                return_pooled_connection(pool_key, socket);
                return Ok(response);
            }
            Err(_) => {
                // Connection failed, close it and fall through to create new one
                let _ = socket.close(None);
            }
        }
    }

    // No pooled connection or it failed - create a new one with retry
    with_retry(|| {
        let mut socket = create_connection(conn)?;
        let response = send_request_on_socket(&mut socket, request_type, request_data.as_ref())?;

        // Return connection to pool for reuse
        return_pooled_connection(pool_key.clone(), socket);

        Ok(response)
    })
}

/// Response status from OBS
#[derive(Debug, Deserialize)]
struct RequestStatus {
    result: bool,
    code: u32,
    #[serde(default)]
    comment: Option<String>,
}

/// Parse response and check for success
fn check_response(response: &Value) -> Result<()> {
    if let Some(status) = response.get("requestStatus") {
        let status: RequestStatus = serde_json::from_value(status.clone())
            .context("Failed to parse request status")?;

        if !status.result {
            let msg = status.comment.unwrap_or_else(|| format!("Error code {}", status.code));
            anyhow::bail!("OBS request failed: {}", msg);
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// Scene Operations
// ─────────────────────────────────────────────────────────────────

/// Set the current program scene
pub fn set_current_scene(conn: &OBSConnection, scene_name: &str) -> Result<()> {
    let response = send_request(conn, "SetCurrentProgramScene", Some(json!({
        "sceneName": scene_name
    })))?;
    check_response(&response)
}

/// Get the current program scene name
#[allow(dead_code)]
pub fn get_current_scene(conn: &OBSConnection) -> Result<String> {
    let response = send_request(conn, "GetCurrentProgramScene", None)?;
    check_response(&response)?;

    response
        .get("responseData")
        .and_then(|d| d.get("currentProgramSceneName"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .context("Failed to get current scene name")
}

// ─────────────────────────────────────────────────────────────────
// Stream Operations
// ─────────────────────────────────────────────────────────────────

/// Get streaming status
pub fn get_stream_status(conn: &OBSConnection) -> Result<bool> {
    let response = send_request(conn, "GetStreamStatus", None)?;
    check_response(&response)?;

    response
        .get("responseData")
        .and_then(|d| d.get("outputActive"))
        .and_then(|v| v.as_bool())
        .context("Failed to get stream status")
}

/// Toggle streaming
pub fn toggle_stream(conn: &OBSConnection) -> Result<bool> {
    let response = send_request(conn, "ToggleStream", None)?;
    check_response(&response)?;

    response
        .get("responseData")
        .and_then(|d| d.get("outputActive"))
        .and_then(|v| v.as_bool())
        .context("Failed to get stream toggle result")
}

/// Start streaming
pub fn start_stream(conn: &OBSConnection) -> Result<()> {
    let response = send_request(conn, "StartStream", None)?;
    check_response(&response)
}

/// Stop streaming
pub fn stop_stream(conn: &OBSConnection) -> Result<()> {
    let response = send_request(conn, "StopStream", None)?;
    check_response(&response)
}

// ─────────────────────────────────────────────────────────────────
// Record Operations
// ─────────────────────────────────────────────────────────────────

/// Recording status response
#[derive(Debug, Clone)]
pub struct RecordStatus {
    pub active: bool,
    pub paused: bool,
}

/// Get recording status
pub fn get_record_status(conn: &OBSConnection) -> Result<RecordStatus> {
    let response = send_request(conn, "GetRecordStatus", None)?;
    check_response(&response)?;

    let data = response.get("responseData")
        .context("Missing responseData")?;

    Ok(RecordStatus {
        active: data.get("outputActive").and_then(|v| v.as_bool()).unwrap_or(false),
        paused: data.get("outputPaused").and_then(|v| v.as_bool()).unwrap_or(false),
    })
}

/// Toggle recording
pub fn toggle_record(conn: &OBSConnection) -> Result<()> {
    let response = send_request(conn, "ToggleRecord", None)?;
    check_response(&response)
}

/// Start recording
pub fn start_record(conn: &OBSConnection) -> Result<()> {
    let response = send_request(conn, "StartRecord", None)?;
    check_response(&response)
}

/// Stop recording
pub fn stop_record(conn: &OBSConnection) -> Result<()> {
    let response = send_request(conn, "StopRecord", None)?;
    check_response(&response)
}

/// Toggle recording pause
pub fn toggle_record_pause(conn: &OBSConnection) -> Result<()> {
    let response = send_request(conn, "ToggleRecordPause", None)?;
    check_response(&response)
}

// ─────────────────────────────────────────────────────────────────
// Source Visibility Operations
// ─────────────────────────────────────────────────────────────────

/// Get source visibility in a scene
#[allow(dead_code)]
pub fn get_source_visibility(conn: &OBSConnection, scene_name: &str, source_name: &str) -> Result<bool> {
    // First, get the scene item ID
    let response = send_request(conn, "GetSceneItemId", Some(json!({
        "sceneName": scene_name,
        "sourceName": source_name
    })))?;
    check_response(&response)?;

    let scene_item_id = response
        .get("responseData")
        .and_then(|d| d.get("sceneItemId"))
        .and_then(|v| v.as_i64())
        .context("Failed to get scene item ID")?;

    // Then get the enabled state
    let response = send_request(conn, "GetSceneItemEnabled", Some(json!({
        "sceneName": scene_name,
        "sceneItemId": scene_item_id
    })))?;
    check_response(&response)?;

    response
        .get("responseData")
        .and_then(|d| d.get("sceneItemEnabled"))
        .and_then(|v| v.as_bool())
        .context("Failed to get source visibility")
}

/// Toggle source visibility in a scene
pub fn toggle_source_visibility(conn: &OBSConnection, scene_name: &str, source_name: &str) -> Result<bool> {
    // First, get the scene item ID
    let response = send_request(conn, "GetSceneItemId", Some(json!({
        "sceneName": scene_name,
        "sourceName": source_name
    })))?;
    check_response(&response)?;

    let scene_item_id = response
        .get("responseData")
        .and_then(|d| d.get("sceneItemId"))
        .and_then(|v| v.as_i64())
        .context("Failed to get scene item ID")?;

    // Get current state
    let response = send_request(conn, "GetSceneItemEnabled", Some(json!({
        "sceneName": scene_name,
        "sceneItemId": scene_item_id
    })))?;
    check_response(&response)?;

    let current = response
        .get("responseData")
        .and_then(|d| d.get("sceneItemEnabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Toggle it
    let new_state = !current;
    let response = send_request(conn, "SetSceneItemEnabled", Some(json!({
        "sceneName": scene_name,
        "sceneItemId": scene_item_id,
        "sceneItemEnabled": new_state
    })))?;
    check_response(&response)?;

    Ok(new_state)
}

// ─────────────────────────────────────────────────────────────────
// Audio Operations
// ─────────────────────────────────────────────────────────────────

/// Get input mute state
#[allow(dead_code)]
pub fn get_input_mute(conn: &OBSConnection, input_name: &str) -> Result<bool> {
    let response = send_request(conn, "GetInputMute", Some(json!({
        "inputName": input_name
    })))?;
    check_response(&response)?;

    response
        .get("responseData")
        .and_then(|d| d.get("inputMuted"))
        .and_then(|v| v.as_bool())
        .context("Failed to get input mute state")
}

/// Toggle input mute
pub fn toggle_input_mute(conn: &OBSConnection, input_name: &str) -> Result<bool> {
    let response = send_request(conn, "ToggleInputMute", Some(json!({
        "inputName": input_name
    })))?;
    check_response(&response)?;

    response
        .get("responseData")
        .and_then(|d| d.get("inputMuted"))
        .and_then(|v| v.as_bool())
        .context("Failed to get mute toggle result")
}

/// Get input volume (returns value in dB, OBS uses mul internally)
pub fn get_input_volume(conn: &OBSConnection, input_name: &str) -> Result<f32> {
    let response = send_request(conn, "GetInputVolume", Some(json!({
        "inputName": input_name
    })))?;
    check_response(&response)?;

    // OBS returns inputVolumeMul (0.0-1.0) and inputVolumeDb
    response
        .get("responseData")
        .and_then(|d| d.get("inputVolumeMul"))
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .context("Failed to get input volume")
}

/// Set input volume (0.0-1.0 multiplier)
pub fn set_input_volume(conn: &OBSConnection, input_name: &str, volume_mul: f32) -> Result<()> {
    let volume = volume_mul.clamp(0.0, 1.0);
    let response = send_request(conn, "SetInputVolume", Some(json!({
        "inputName": input_name,
        "inputVolumeMul": volume
    })))?;
    check_response(&response)
}

// ─────────────────────────────────────────────────────────────────
// Studio Mode Operations
// ─────────────────────────────────────────────────────────────────

/// Get Studio Mode status
pub fn get_studio_mode(conn: &OBSConnection) -> Result<bool> {
    let response = send_request(conn, "GetStudioModeEnabled", None)?;
    check_response(&response)?;

    response
        .get("responseData")
        .and_then(|d| d.get("studioModeEnabled"))
        .and_then(|v| v.as_bool())
        .context("Failed to get studio mode status")
}

/// Toggle Studio Mode
pub fn toggle_studio_mode(conn: &OBSConnection) -> Result<bool> {
    let current = get_studio_mode(conn)?;
    let new_state = !current;

    let response = send_request(conn, "SetStudioModeEnabled", Some(json!({
        "studioModeEnabled": new_state
    })))?;
    check_response(&response)?;

    Ok(new_state)
}

/// Trigger Studio Mode transition
pub fn trigger_transition(conn: &OBSConnection) -> Result<()> {
    let response = send_request(conn, "TriggerStudioModeTransition", None)?;
    check_response(&response)
}

// ─────────────────────────────────────────────────────────────────
// Replay Buffer Operations
// ─────────────────────────────────────────────────────────────────

/// Get replay buffer status
pub fn get_replay_buffer_status(conn: &OBSConnection) -> Result<bool> {
    let response = send_request(conn, "GetReplayBufferStatus", None)?;
    check_response(&response)?;

    response
        .get("responseData")
        .and_then(|d| d.get("outputActive"))
        .and_then(|v| v.as_bool())
        .context("Failed to get replay buffer status")
}

/// Toggle replay buffer
pub fn toggle_replay_buffer(conn: &OBSConnection) -> Result<bool> {
    let response = send_request(conn, "ToggleReplayBuffer", None)?;
    check_response(&response)?;

    response
        .get("responseData")
        .and_then(|d| d.get("outputActive"))
        .and_then(|v| v.as_bool())
        .context("Failed to get replay buffer toggle result")
}

/// Start replay buffer
pub fn start_replay_buffer(conn: &OBSConnection) -> Result<()> {
    let response = send_request(conn, "StartReplayBuffer", None)?;
    check_response(&response)
}

/// Stop replay buffer
pub fn stop_replay_buffer(conn: &OBSConnection) -> Result<()> {
    let response = send_request(conn, "StopReplayBuffer", None)?;
    check_response(&response)
}

/// Save replay buffer
pub fn save_replay_buffer(conn: &OBSConnection) -> Result<()> {
    let response = send_request(conn, "SaveReplayBuffer", None)?;
    check_response(&response)
}

// ─────────────────────────────────────────────────────────────────
// Virtual Camera Operations
// ─────────────────────────────────────────────────────────────────

/// Get virtual camera status
#[allow(dead_code)]
pub fn get_virtual_cam_status(conn: &OBSConnection) -> Result<bool> {
    let response = send_request(conn, "GetVirtualCamStatus", None)?;
    check_response(&response)?;

    response
        .get("responseData")
        .and_then(|d| d.get("outputActive"))
        .and_then(|v| v.as_bool())
        .context("Failed to get virtual cam status")
}

/// Toggle virtual camera
pub fn toggle_virtual_cam(conn: &OBSConnection) -> Result<bool> {
    let response = send_request(conn, "ToggleVirtualCam", None)?;
    check_response(&response)?;

    response
        .get("responseData")
        .and_then(|d| d.get("outputActive"))
        .and_then(|v| v.as_bool())
        .context("Failed to get virtual cam toggle result")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_string_generation() {
        // Test that the auth string generation follows the obs-websocket protocol:
        // 1. secret = base64(sha256(password + salt))
        // 2. auth = base64(sha256(secret + challenge))
        let password = "supersecretpassword";
        let challenge = "ztTBnnuqrqaKDzRM3xcVdbYm";
        let salt = "PZVbYpvAnZut2SS6JNJytDm9";

        let auth = generate_auth_string(password, challenge, salt);

        // Verify the result is a valid base64 string of expected length (SHA256 = 32 bytes = 44 chars base64)
        assert_eq!(auth.len(), 44);
        assert!(base64::engine::general_purpose::STANDARD.decode(&auth).is_ok());

        // Verify deterministic output (same inputs = same output)
        let auth2 = generate_auth_string(password, challenge, salt);
        assert_eq!(auth, auth2);
    }

    #[test]
    fn test_request_id_increments() {
        let id1 = next_request_id();
        let id2 = next_request_id();

        let n1: u32 = id1.parse().unwrap();
        let n2: u32 = id2.parse().unwrap();

        assert_eq!(n2, n1 + 1);
    }

    #[test]
    fn test_validate_ip_allows_private() {
        assert!(validate_ip("192.168.1.100").is_ok());
        assert!(validate_ip("10.0.0.1").is_ok());
        assert!(validate_ip("172.16.0.1").is_ok());
        assert!(validate_ip("127.0.0.1").is_ok());
    }

    #[test]
    fn test_validate_ip_blocks_public() {
        assert!(validate_ip("8.8.8.8").is_err());
        assert!(validate_ip("1.1.1.1").is_err());
    }

    #[test]
    fn test_connection_key() {
        let conn = OBSConnection::new("192.168.1.50", 4455, None);
        assert_eq!(conn.key(), "192.168.1.50:4455");
    }
}
