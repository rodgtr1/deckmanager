//! Command execution capabilities: RunCommand, LaunchApp, OpenURL.
//!
//! Security notes:
//! - RunCommand uses shlex for safe argument parsing (no shell injection)
//! - LaunchApp validates against shell metacharacters
//! - OpenURL only allows whitelisted schemes

use crate::binding::{Binding, InputRef};
use crate::capability::Capability;
use crate::input_processor::LogicalEvent;
use crate::plugin::{CapabilityMetadata, ParameterDef, ParameterType};
use crate::state_manager::SystemState;
use crate::streamdeck::request_image_sync;
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Minimum interval between command executions (prevents rapid-fire from stuck buttons)
const MIN_COMMAND_INTERVAL: Duration = Duration::from_millis(200);

/// Track last execution time for rate limiting
static COMMAND_RATE_LIMITER: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

/// Get or initialize the rate limiter
fn get_rate_limiter() -> &'static Mutex<HashMap<String, Instant>> {
    COMMAND_RATE_LIMITER.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Check if a command can be executed (rate limiting)
/// Returns true if enough time has passed since the last execution
fn check_rate_limit(key: &str) -> bool {
    let rate_limiter = get_rate_limiter();
    let mut times = rate_limiter.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(last_time) = times.get(key) {
        if last_time.elapsed() < MIN_COMMAND_INTERVAL {
            return false; // Rate limited
        }
    }

    times.insert(key.to_string(), Instant::now());
    true
}

/// Get capability metadata for all command capabilities.
pub fn capabilities() -> Vec<CapabilityMetadata> {
    vec![
        CapabilityMetadata {
            id: "RunCommand",
            name: "Run Command",
            description: "Execute a shell command. Enable toggle mode for commands that flip between states (e.g., start/stop dictation)",
            plugin_id: "core",
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![
                ParameterDef {
                    name: "command",
                    param_type: ParameterType::String,
                    default_value: "",
                    description: "Shell command to execute",
                },
                ParameterDef {
                    name: "toggle",
                    param_type: ParameterType::Bool,
                    default_value: "false",
                    description: "Toggle mode: alternate between default and active image on each press",
                },
            ],
        },
        CapabilityMetadata {
            id: "LaunchApp",
            name: "Launch App",
            description: "Launch an application",
            plugin_id: "core",
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![ParameterDef {
                name: "command",
                param_type: ParameterType::String,
                default_value: "",
                description: "Application to launch (e.g., firefox, code)",
            }],
        },
        CapabilityMetadata {
            id: "OpenURL",
            name: "Open URL",
            description: "Open a URL in your default browser",
            plugin_id: "core",
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: true,
            parameters: vec![ParameterDef {
                name: "url",
                param_type: ParameterType::String,
                default_value: "https://",
                description: "URL to open",
            }],
        },
    ]
}

/// Handle command-related events.
///
/// Returns `true` if the event was handled.
pub fn handle_event(
    event: &LogicalEvent,
    binding: &Binding,
    system_state: &Arc<Mutex<SystemState>>,
) -> bool {
    match (&binding.capability, event) {
        (Capability::RunCommand { command, toggle }, LogicalEvent::EncoderPress(e)) if e.pressed => {
            run_shell_command(command);
            if *toggle {
                flip_toggle_state(binding, system_state);
            }
            true
        }

        (Capability::RunCommand { command, toggle }, LogicalEvent::Button(e)) if e.pressed => {
            run_shell_command(command);
            if *toggle {
                flip_toggle_state(binding, system_state);
            }
            true
        }

        (Capability::LaunchApp { command }, LogicalEvent::EncoderPress(e)) if e.pressed => {
            launch_app(command);
            true
        }

        (Capability::LaunchApp { command }, LogicalEvent::Button(e)) if e.pressed => {
            launch_app(command);
            true
        }

        (Capability::OpenURL { url }, LogicalEvent::EncoderPress(e)) if e.pressed => {
            open_url(url);
            true
        }

        (Capability::OpenURL { url }, LogicalEvent::Button(e)) if e.pressed => {
            open_url(url);
            true
        }

        _ => false,
    }
}

/// Check if a command binding is in an active state.
pub fn is_active(binding: &Binding, state: &SystemState) -> bool {
    if let Capability::RunCommand { toggle: true, .. } = &binding.capability {
        // Check toggle state for this binding
        let key = binding_key(binding);
        state.toggle_states.get(&key).copied().unwrap_or(false)
    } else {
        false
    }
}

/// Get a unique key for a binding (used for toggle state tracking)
fn binding_key(binding: &Binding) -> String {
    let input_key = match &binding.input {
        InputRef::Button { index } => format!("btn:{}", index),
        InputRef::Encoder { index } => format!("enc:{}", index),
        InputRef::EncoderPress { index } => format!("encp:{}", index),
        InputRef::Swipe => "swipe".to_string(),
    };
    format!("{}:{}", input_key, binding.page)
}

/// Flip the toggle state for a binding and request image sync
fn flip_toggle_state(binding: &Binding, system_state: &Arc<Mutex<SystemState>>) {
    let key = binding_key(binding);
    if let Ok(mut state) = system_state.lock() {
        let current = state.toggle_states.get(&key).copied().unwrap_or(false);
        state.toggle_states.insert(key, !current);
    }
    request_image_sync();
}

// ─────────────────────────────────────────────────────────────────
// Command execution functions
// ─────────────────────────────────────────────────────────────────

fn run_shell_command(cmd: &str) {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        eprintln!("Warning: Attempted to run empty command");
        return;
    }

    // Rate limit to prevent rapid-fire execution
    let rate_key = format!("cmd:{}", cmd);
    if !check_rate_limit(&rate_key) {
        #[cfg(debug_assertions)]
        eprintln!("Rate limited: {}", cmd);
        return;
    }

    // Parse command using shlex for safe argument handling (prevents shell injection)
    let args = match shlex::split(cmd) {
        Some(args) if !args.is_empty() => args,
        Some(_) => {
            eprintln!("Warning: Command parsed to empty arguments: {}", cmd);
            return;
        }
        None => {
            eprintln!("Warning: Failed to parse command (invalid quoting): {}", cmd);
            return;
        }
    };

    #[cfg(debug_assertions)]
    eprintln!("Executing command: {:?}", args);

    // Execute directly without shell (no injection possible)
    match Command::new(&args[0]).args(&args[1..]).spawn() {
        Ok(_) => {}
        Err(e) => eprintln!("Failed to execute command '{}': {}", cmd, e),
    }
}

/// Characters that could be used for shell injection or command chaining
const DANGEROUS_CHARS: &[char] = &['$', '`', ';', '|', '&', '>', '<', '(', ')', '{', '}', '[', ']', '!', '\n', '\r'];

fn launch_app(app: &str) {
    let app = app.trim();
    if app.is_empty() {
        eprintln!("Warning: Attempted to launch empty application name");
        return;
    }

    // Reject any shell metacharacters that could be used for injection
    if app.chars().any(|c| DANGEROUS_CHARS.contains(&c)) {
        eprintln!("Warning: Application name contains dangerous characters, rejected: {}", app);
        return;
    }

    // Reject path traversal attempts
    if app.contains("..") {
        eprintln!("Warning: Path traversal rejected: {}", app);
        return;
    }

    // For absolute paths, only allow known safe directories
    if app.starts_with('/') {
        let allowed_prefixes = ["/usr/bin/", "/usr/local/bin/", "/bin/", "/opt/"];
        if !allowed_prefixes.iter().any(|prefix| app.starts_with(prefix)) {
            eprintln!("Warning: Absolute path not in allowed directories: {}", app);
            return;
        }
    }

    // Rate limit to prevent rapid-fire execution
    let rate_key = format!("app:{}", app);
    if !check_rate_limit(&rate_key) {
        #[cfg(debug_assertions)]
        eprintln!("Rate limited: {}", app);
        return;
    }

    #[cfg(debug_assertions)]
    eprintln!("Launching application: {}", app);

    match Command::new(app).spawn() {
        Ok(_) => {}
        Err(e) => eprintln!("Failed to launch '{}': {}", app, e),
    }
}

fn open_url(url: &str) {
    let url = url.trim();
    if url.is_empty() {
        eprintln!("Warning: Attempted to open empty URL");
        return;
    }

    // Validate URL scheme - only allow http, https, and common safe schemes
    let valid_schemes = ["http://", "https://", "mailto:", "tel:"];
    if !valid_schemes.iter().any(|scheme| url.starts_with(scheme)) {
        eprintln!("Warning: Rejected URL with unsupported scheme: {}", url);
        return;
    }

    // Rate limit to prevent rapid-fire execution
    let rate_key = format!("url:{}", url);
    if !check_rate_limit(&rate_key) {
        #[cfg(debug_assertions)]
        eprintln!("Rate limited: {}", url);
        return;
    }

    #[cfg(debug_assertions)]
    eprintln!("Opening URL: {}", url);

    match Command::new("xdg-open").arg(url).spawn() {
        Ok(_) => {}
        Err(e) => eprintln!("Failed to open URL '{}': {}", url, e),
    }
}
