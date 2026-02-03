//! OBS Studio plugin implementation.

use super::client::{self, OBSConnection};
use super::controller::OBSAudioController;
use crate::binding::Binding;
use crate::capability::{
    Capability, OBSRecordAction, OBSReplayAction, OBSStreamAction,
};
use crate::impl_owns_capability;
use crate::input_processor::LogicalEvent;
use crate::plugin::{CapabilityMetadata, ParameterDef, ParameterType, Plugin, PluginConfig};
use crate::state_manager::{OBSState, SystemState};
use crate::streamdeck::request_image_sync;
use std::any::Any;
use std::sync::{Arc, Mutex, OnceLock};

/// Check if an event is a press event (button down or encoder press down)
fn is_press_event(event: &LogicalEvent) -> bool {
    matches!(
        event,
        LogicalEvent::Button(e) if e.pressed
    ) || matches!(
        event,
        LogicalEvent::EncoderPress(e) if e.pressed
    )
}

/// Global debounced OBS audio controller
static OBS_AUDIO_CONTROLLER: OnceLock<OBSAudioController> = OnceLock::new();

/// Get or initialize the OBS audio controller
fn get_audio_controller(system_state: &Arc<Mutex<SystemState>>) -> &'static OBSAudioController {
    OBS_AUDIO_CONTROLLER.get_or_init(|| OBSAudioController::new(Arc::clone(system_state)))
}

/// OBS Studio plugin
pub struct OBSPlugin;

impl OBSPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OBSPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for OBSPlugin {
    fn id(&self) -> &'static str {
        "obs"
    }

    fn name(&self) -> &'static str {
        "OBS Studio"
    }

    fn category(&self) -> &'static str {
        "Streaming"
    }

    fn capabilities(&self) -> Vec<CapabilityMetadata> {
        vec![
            CapabilityMetadata {
                id: "OBSScene",
                name: "Scene",
                description: "Switch to a specific OBS scene",
                plugin_id: "obs",
                supports_button: true,
                supports_encoder: false,
                supports_encoder_press: true,
                parameters: vec![
                    obs_host_param(),
                    obs_port_param(),
                    obs_password_param(),
                    ParameterDef {
                        name: "scene",
                        param_type: ParameterType::String,
                        default_value: "Scene",
                        description: "Name of the scene to switch to",
                    },
                ],
            },
            CapabilityMetadata {
                id: "OBSStream",
                name: "Stream",
                description: "Control OBS streaming (toggle/start/stop)",
                plugin_id: "obs",
                supports_button: true,
                supports_encoder: false,
                supports_encoder_press: true,
                parameters: vec![
                    obs_host_param(),
                    obs_port_param(),
                    obs_password_param(),
                    ParameterDef {
                        name: "action",
                        param_type: ParameterType::String,
                        default_value: "Toggle",
                        description: "Action: Toggle, Start, or Stop",
                    },
                ],
            },
            CapabilityMetadata {
                id: "OBSRecord",
                name: "Record",
                description: "Control OBS recording (toggle/start/stop/pause)",
                plugin_id: "obs",
                supports_button: true,
                supports_encoder: false,
                supports_encoder_press: true,
                parameters: vec![
                    obs_host_param(),
                    obs_port_param(),
                    obs_password_param(),
                    ParameterDef {
                        name: "action",
                        param_type: ParameterType::String,
                        default_value: "Toggle",
                        description: "Action: Toggle, Start, Stop, or TogglePause",
                    },
                ],
            },
            CapabilityMetadata {
                id: "OBSSourceVisibility",
                name: "Source Visibility",
                description: "Toggle visibility of a source in a scene",
                plugin_id: "obs",
                supports_button: true,
                supports_encoder: false,
                supports_encoder_press: true,
                parameters: vec![
                    obs_host_param(),
                    obs_port_param(),
                    obs_password_param(),
                    ParameterDef {
                        name: "scene",
                        param_type: ParameterType::String,
                        default_value: "Scene",
                        description: "Name of the scene containing the source",
                    },
                    ParameterDef {
                        name: "source",
                        param_type: ParameterType::String,
                        default_value: "Source",
                        description: "Name of the source to toggle",
                    },
                ],
            },
            CapabilityMetadata {
                id: "OBSAudio",
                name: "Audio",
                description: "Control OBS audio - rotate for volume, press to mute",
                plugin_id: "obs",
                supports_button: true,
                supports_encoder: true,
                supports_encoder_press: true,
                parameters: vec![
                    obs_host_param(),
                    obs_port_param(),
                    obs_password_param(),
                    ParameterDef {
                        name: "input_name",
                        param_type: ParameterType::String,
                        default_value: "Mic/Aux",
                        description: "Name of the audio input in OBS",
                    },
                    ParameterDef {
                        name: "step",
                        param_type: ParameterType::Float,
                        default_value: "0.02",
                        description: "Volume change per encoder tick (0.0-1.0)",
                    },
                ],
            },
            CapabilityMetadata {
                id: "OBSStudioMode",
                name: "Studio Mode",
                description: "Toggle OBS Studio Mode",
                plugin_id: "obs",
                supports_button: true,
                supports_encoder: false,
                supports_encoder_press: true,
                parameters: vec![
                    obs_host_param(),
                    obs_port_param(),
                    obs_password_param(),
                ],
            },
            CapabilityMetadata {
                id: "OBSReplayBuffer",
                name: "Replay Buffer",
                description: "Control OBS Replay Buffer (toggle/start/stop/save)",
                plugin_id: "obs",
                supports_button: true,
                supports_encoder: false,
                supports_encoder_press: true,
                parameters: vec![
                    obs_host_param(),
                    obs_port_param(),
                    obs_password_param(),
                    ParameterDef {
                        name: "action",
                        param_type: ParameterType::String,
                        default_value: "Save",
                        description: "Action: Toggle, Start, Stop, or Save",
                    },
                ],
            },
            CapabilityMetadata {
                id: "OBSVirtualCam",
                name: "Virtual Camera",
                description: "Toggle OBS Virtual Camera",
                plugin_id: "obs",
                supports_button: true,
                supports_encoder: false,
                supports_encoder_press: true,
                parameters: vec![
                    obs_host_param(),
                    obs_port_param(),
                    obs_password_param(),
                ],
            },
            CapabilityMetadata {
                id: "OBSTransition",
                name: "Transition",
                description: "Trigger Studio Mode transition (preview to program)",
                plugin_id: "obs",
                supports_button: true,
                supports_encoder: false,
                supports_encoder_press: true,
                parameters: vec![
                    obs_host_param(),
                    obs_port_param(),
                    obs_password_param(),
                ],
            },
        ]
    }

    fn handle_event(
        &self,
        event: &LogicalEvent,
        binding: &Binding,
        system_state: &Arc<Mutex<SystemState>>,
    ) -> bool {
        match &binding.capability {
            // OBSScene - press to switch scene
            Capability::OBSScene { host, port, password, scene } if is_press_event(event) => {
                handle_scene_switch(host, *port, password.clone(), scene, system_state);
                true
            }

            // OBSStream - press to control streaming
            Capability::OBSStream { host, port, password, action } if is_press_event(event) => {
                handle_stream_action(host, *port, password.clone(), action, system_state);
                true
            }

            // OBSRecord - press to control recording
            Capability::OBSRecord { host, port, password, action } if is_press_event(event) => {
                handle_record_action(host, *port, password.clone(), action, system_state);
                true
            }

            // OBSSourceVisibility - press to toggle source visibility
            Capability::OBSSourceVisibility { host, port, password, scene, source } if is_press_event(event) => {
                handle_source_visibility(host, *port, password.clone(), scene, source, system_state);
                true
            }

            // OBSAudio - encoder rotation for volume
            Capability::OBSAudio { host, port, password, input_name, step } => {
                match event {
                    LogicalEvent::Encoder(e) => {
                        handle_audio_volume(host, *port, password.clone(), input_name, *step, e.delta, system_state);
                        true
                    }
                    _ if is_press_event(event) => {
                        handle_audio_mute(host, *port, password.clone(), input_name, system_state);
                        true
                    }
                    _ => false,
                }
            }

            // OBSStudioMode - press to toggle studio mode
            Capability::OBSStudioMode { host, port, password } if is_press_event(event) => {
                handle_studio_mode(host, *port, password.clone(), system_state);
                true
            }

            // OBSReplayBuffer - press to control replay buffer
            Capability::OBSReplayBuffer { host, port, password, action } if is_press_event(event) => {
                handle_replay_action(host, *port, password.clone(), action, system_state);
                true
            }

            // OBSVirtualCam - press to toggle virtual camera
            Capability::OBSVirtualCam { host, port, password } if is_press_event(event) => {
                handle_virtual_cam(host, *port, password.clone(), system_state);
                true
            }

            // OBSTransition - press to trigger transition
            Capability::OBSTransition { host, port, password } if is_press_event(event) => {
                handle_transition(host, *port, password.clone());
                true
            }

            _ => false,
        }
    }

    impl_owns_capability!(
        "OBSScene",
        "OBSStream",
        "OBSRecord",
        "OBSSourceVisibility",
        "OBSAudio",
        "OBSStudioMode",
        "OBSReplayBuffer",
        "OBSVirtualCam",
        "OBSTransition"
    );

    fn is_active(&self, binding: &Binding, system_state: &SystemState) -> bool {
        match &binding.capability {
            Capability::OBSScene { host, port, scene, .. } => {
                let key = format!("{}:{}", host, port);
                system_state
                    .obs_states
                    .get(&key)
                    .map(|s| s.current_scene == *scene)
                    .unwrap_or(false)
            }
            Capability::OBSStream { host, port, .. } => {
                let key = format!("{}:{}", host, port);
                system_state
                    .obs_states
                    .get(&key)
                    .map(|s| s.streaming)
                    .unwrap_or(false)
            }
            Capability::OBSRecord { host, port, .. } => {
                let key = format!("{}:{}", host, port);
                system_state
                    .obs_states
                    .get(&key)
                    .map(|s| s.recording)
                    .unwrap_or(false)
            }
            Capability::OBSSourceVisibility { host, port, scene, source, .. } => {
                let key = format!("{}:{}", host, port);
                let source_key = format!("{}:{}", scene, source);
                system_state
                    .obs_states
                    .get(&key)
                    .and_then(|s| s.source_visibility.get(&source_key))
                    .copied()
                    .unwrap_or(false)
            }
            Capability::OBSAudio { host, port, input_name, .. } => {
                let key = format!("{}:{}", host, port);
                system_state
                    .obs_states
                    .get(&key)
                    .and_then(|s| s.muted_inputs.get(input_name))
                    .copied()
                    .unwrap_or(false)
            }
            Capability::OBSStudioMode { host, port, .. } => {
                let key = format!("{}:{}", host, port);
                system_state
                    .obs_states
                    .get(&key)
                    .map(|s| s.studio_mode)
                    .unwrap_or(false)
            }
            Capability::OBSReplayBuffer { host, port, .. } => {
                let key = format!("{}:{}", host, port);
                system_state
                    .obs_states
                    .get(&key)
                    .map(|s| s.replay_buffer)
                    .unwrap_or(false)
            }
            Capability::OBSVirtualCam { host, port, .. } => {
                let key = format!("{}:{}", host, port);
                system_state
                    .obs_states
                    .get(&key)
                    .map(|s| s.virtual_cam)
                    .unwrap_or(false)
            }
            _ => false,
        }
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
        "Control OBS Studio via WebSocket"
    }

    fn documentation(&self) -> &'static str {
        include_str!("../../../docs/plugins/obs.md")
    }

    fn icon(&self) -> &'static str {
        "obs"
    }
}

// ─────────────────────────────────────────────────────────────────
// Helper functions for common parameters
// ─────────────────────────────────────────────────────────────────

fn obs_host_param() -> ParameterDef {
    ParameterDef {
        name: "host",
        param_type: ParameterType::IpAddress,
        default_value: "127.0.0.1",
        description: "OBS WebSocket host address",
    }
}

fn obs_port_param() -> ParameterDef {
    ParameterDef {
        name: "port",
        param_type: ParameterType::Integer,
        default_value: "4455",
        description: "OBS WebSocket port (default: 4455)",
    }
}

fn obs_password_param() -> ParameterDef {
    ParameterDef {
        name: "password",
        param_type: ParameterType::String,
        default_value: "",
        description: "OBS WebSocket password (if enabled)",
    }
}

// ─────────────────────────────────────────────────────────────────
// Event handlers (spawn background threads)
// ─────────────────────────────────────────────────────────────────

fn handle_scene_switch(
    host: &str,
    port: u16,
    password: Option<String>,
    scene: &str,
    system_state: &Arc<Mutex<SystemState>>,
) {
    let conn = OBSConnection::new(host, port, password);
    let scene = scene.to_string();
    let state = Arc::clone(system_state);

    std::thread::spawn(move || {
        if let Err(e) = client::set_current_scene(&conn, &scene) {
            eprintln!("OBS scene switch error: {e}");
            return;
        }

        // Update state
        if let Ok(mut s) = state.lock() {
            let key = conn.key();
            let obs_state = s.obs_states.entry(key).or_insert_with(OBSState::default);
            obs_state.current_scene = scene;
        }

        request_image_sync();
    });
}

fn handle_stream_action(
    host: &str,
    port: u16,
    password: Option<String>,
    action: &OBSStreamAction,
    system_state: &Arc<Mutex<SystemState>>,
) {
    let conn = OBSConnection::new(host, port, password);
    let action = action.clone();
    let state = Arc::clone(system_state);

    std::thread::spawn(move || {
        let result = match action {
            OBSStreamAction::Toggle => client::toggle_stream(&conn).map(Some),
            OBSStreamAction::Start => client::start_stream(&conn).map(|_| None),
            OBSStreamAction::Stop => client::stop_stream(&conn).map(|_| None),
        };

        match result {
            Ok(new_state) => {
                // Get actual state if not returned by the action
                let streaming = new_state.unwrap_or_else(|| {
                    client::get_stream_status(&conn).unwrap_or(false)
                });

                // Update state
                if let Ok(mut s) = state.lock() {
                    let key = conn.key();
                    let obs_state = s.obs_states.entry(key).or_insert_with(OBSState::default);
                    obs_state.streaming = streaming;
                }

                request_image_sync();
            }
            Err(e) => eprintln!("OBS stream error: {e}"),
        }
    });
}

fn handle_record_action(
    host: &str,
    port: u16,
    password: Option<String>,
    action: &OBSRecordAction,
    system_state: &Arc<Mutex<SystemState>>,
) {
    let conn = OBSConnection::new(host, port, password);
    let action = action.clone();
    let state = Arc::clone(system_state);

    std::thread::spawn(move || {
        let result = match action {
            OBSRecordAction::Toggle => client::toggle_record(&conn),
            OBSRecordAction::Start => client::start_record(&conn),
            OBSRecordAction::Stop => client::stop_record(&conn),
            OBSRecordAction::TogglePause => client::toggle_record_pause(&conn),
        };

        if let Err(e) = result {
            eprintln!("OBS record error: {e}");
            return;
        }

        // Fetch actual state
        if let Ok(status) = client::get_record_status(&conn) {
            if let Ok(mut s) = state.lock() {
                let key = conn.key();
                let obs_state = s.obs_states.entry(key).or_insert_with(OBSState::default);
                obs_state.recording = status.active;
                obs_state.recording_paused = status.paused;
            }
        }

        request_image_sync();
    });
}

fn handle_source_visibility(
    host: &str,
    port: u16,
    password: Option<String>,
    scene: &str,
    source: &str,
    system_state: &Arc<Mutex<SystemState>>,
) {
    let conn = OBSConnection::new(host, port, password);
    let scene = scene.to_string();
    let source = source.to_string();
    let state = Arc::clone(system_state);

    std::thread::spawn(move || {
        match client::toggle_source_visibility(&conn, &scene, &source) {
            Ok(new_visible) => {
                if let Ok(mut s) = state.lock() {
                    let key = conn.key();
                    let obs_state = s.obs_states.entry(key).or_insert_with(OBSState::default);
                    let source_key = format!("{}:{}", scene, source);
                    obs_state.source_visibility.insert(source_key, new_visible);
                }
                request_image_sync();
            }
            Err(e) => eprintln!("OBS source visibility error: {e}"),
        }
    });
}

fn handle_audio_volume(
    host: &str,
    port: u16,
    password: Option<String>,
    input_name: &str,
    step: f32,
    delta: i8,
    system_state: &Arc<Mutex<SystemState>>,
) {
    if delta == 0 {
        return;
    }

    let conn = OBSConnection::new(host, port, password);
    let volume_delta = step * delta as f32;

    // Use the debounced controller
    let controller = get_audio_controller(system_state);
    controller.queue_volume_delta(&conn, input_name, volume_delta);
}

fn handle_audio_mute(
    host: &str,
    port: u16,
    password: Option<String>,
    input_name: &str,
    system_state: &Arc<Mutex<SystemState>>,
) {
    let conn = OBSConnection::new(host, port, password);
    let input_name = input_name.to_string();
    let state = Arc::clone(system_state);

    std::thread::spawn(move || {
        match client::toggle_input_mute(&conn, &input_name) {
            Ok(muted) => {
                if let Ok(mut s) = state.lock() {
                    let key = conn.key();
                    let obs_state = s.obs_states.entry(key).or_insert_with(OBSState::default);
                    obs_state.muted_inputs.insert(input_name.clone(), muted);
                }
                request_image_sync();
            }
            Err(e) => eprintln!("OBS mute error: {e}"),
        }
    });
}

fn handle_studio_mode(
    host: &str,
    port: u16,
    password: Option<String>,
    system_state: &Arc<Mutex<SystemState>>,
) {
    let conn = OBSConnection::new(host, port, password);
    let state = Arc::clone(system_state);

    std::thread::spawn(move || {
        match client::toggle_studio_mode(&conn) {
            Ok(enabled) => {
                if let Ok(mut s) = state.lock() {
                    let key = conn.key();
                    let obs_state = s.obs_states.entry(key).or_insert_with(OBSState::default);
                    obs_state.studio_mode = enabled;
                }
                request_image_sync();
            }
            Err(e) => eprintln!("OBS studio mode error: {e}"),
        }
    });
}

fn handle_replay_action(
    host: &str,
    port: u16,
    password: Option<String>,
    action: &OBSReplayAction,
    system_state: &Arc<Mutex<SystemState>>,
) {
    let conn = OBSConnection::new(host, port, password);
    let action = action.clone();
    let state = Arc::clone(system_state);

    std::thread::spawn(move || {
        let result = match action {
            OBSReplayAction::Toggle => client::toggle_replay_buffer(&conn).map(Some),
            OBSReplayAction::Start => client::start_replay_buffer(&conn).map(|_| None),
            OBSReplayAction::Stop => client::stop_replay_buffer(&conn).map(|_| None),
            OBSReplayAction::Save => client::save_replay_buffer(&conn).map(|_| None),
        };

        match result {
            Ok(new_state) => {
                // Get actual state if not returned
                let active = new_state.unwrap_or_else(|| {
                    client::get_replay_buffer_status(&conn).unwrap_or(false)
                });

                if let Ok(mut s) = state.lock() {
                    let key = conn.key();
                    let obs_state = s.obs_states.entry(key).or_insert_with(OBSState::default);
                    obs_state.replay_buffer = active;
                }

                request_image_sync();
            }
            Err(e) => eprintln!("OBS replay buffer error: {e}"),
        }
    });
}

fn handle_virtual_cam(
    host: &str,
    port: u16,
    password: Option<String>,
    system_state: &Arc<Mutex<SystemState>>,
) {
    let conn = OBSConnection::new(host, port, password);
    let state = Arc::clone(system_state);

    std::thread::spawn(move || {
        match client::toggle_virtual_cam(&conn) {
            Ok(active) => {
                if let Ok(mut s) = state.lock() {
                    let key = conn.key();
                    let obs_state = s.obs_states.entry(key).or_insert_with(OBSState::default);
                    obs_state.virtual_cam = active;
                }
                request_image_sync();
            }
            Err(e) => eprintln!("OBS virtual cam error: {e}"),
        }
    });
}

fn handle_transition(host: &str, port: u16, password: Option<String>) {
    let conn = OBSConnection::new(host, port, password);

    std::thread::spawn(move || {
        if let Err(e) = client::trigger_transition(&conn) {
            eprintln!("OBS transition error: {e}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obs_plugin_has_correct_id() {
        let plugin = OBSPlugin::new();
        assert_eq!(plugin.id(), "obs");
    }

    #[test]
    fn obs_plugin_has_all_capabilities() {
        let plugin = OBSPlugin::new();
        let caps = plugin.capabilities();
        assert_eq!(caps.len(), 9);

        let cap_ids: Vec<_> = caps.iter().map(|c| c.id).collect();
        assert!(cap_ids.contains(&"OBSScene"));
        assert!(cap_ids.contains(&"OBSStream"));
        assert!(cap_ids.contains(&"OBSRecord"));
        assert!(cap_ids.contains(&"OBSSourceVisibility"));
        assert!(cap_ids.contains(&"OBSAudio"));
        assert!(cap_ids.contains(&"OBSStudioMode"));
        assert!(cap_ids.contains(&"OBSReplayBuffer"));
        assert!(cap_ids.contains(&"OBSVirtualCam"));
        assert!(cap_ids.contains(&"OBSTransition"));
    }

    #[test]
    fn obs_plugin_owns_its_capabilities() {
        let plugin = OBSPlugin::new();
        assert!(plugin.owns_capability("OBSScene"));
        assert!(plugin.owns_capability("OBSStream"));
        assert!(plugin.owns_capability("OBSRecord"));
        assert!(plugin.owns_capability("OBSSourceVisibility"));
        assert!(plugin.owns_capability("OBSAudio"));
        assert!(plugin.owns_capability("OBSStudioMode"));
        assert!(plugin.owns_capability("OBSReplayBuffer"));
        assert!(plugin.owns_capability("OBSVirtualCam"));
        assert!(plugin.owns_capability("OBSTransition"));
        assert!(!plugin.owns_capability("SystemAudio"));
    }

    #[test]
    fn obs_plugin_metadata_name() {
        let plugin = OBSPlugin::new();
        assert_eq!(plugin.name(), "OBS Studio");
    }

    #[test]
    fn obs_plugin_metadata_category() {
        let plugin = OBSPlugin::new();
        assert_eq!(plugin.category(), "Streaming");
    }

    #[test]
    fn obs_plugin_metadata_version() {
        let plugin = OBSPlugin::new();
        assert_eq!(plugin.version(), "1.0.0");
    }

    #[test]
    fn obs_plugin_is_not_core() {
        let plugin = OBSPlugin::new();
        assert!(!plugin.is_core());
    }
}
