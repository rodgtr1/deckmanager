use std::sync::{Arc, Mutex};
use tauri::{Builder, Manager, RunEvent, WindowEvent};
use tauri_plugin_single_instance::init as single_instance_init;

mod binding;
mod button_renderer;
mod capability;
mod commands;
mod config;
mod core;
mod device;
mod events;
mod hid;
mod hotplug;
mod image_cache;
mod input_processor;
mod plugin;
mod plugins;
mod state_manager;
mod streamdeck;

// Re-export for backwards compatibility
#[cfg(feature = "plugin-elgato")]
pub use plugins::elgato::client as elgato_key_light;
#[cfg(feature = "plugin-elgato")]
pub use plugins::elgato::controller as key_light_controller;

// Include generated constants from build.rs
pub mod app_constants {
    include!(concat!(env!("OUT_DIR"), "/app_constants.rs"));
}

use commands::AppState;
use plugin::PluginRegistry;

/// Run the application with optional hidden mode
pub fn run() {
    run_with_options(false);
}

/// Initialize the plugin registry with all available plugins.
fn create_plugin_registry() -> Arc<PluginRegistry> {
    use plugin::PluginConfig;

    let registry = PluginRegistry::new();

    // Load persisted plugin states
    let plugin_states = config::load_plugin_states();

    // Helper to create config with persisted enabled state
    let make_config = |plugin_id: &str, default_enabled: bool| -> PluginConfig {
        PluginConfig {
            enabled: *plugin_states.get(plugin_id).unwrap_or(&default_enabled),
            settings: std::collections::HashMap::new(),
        }
    };

    // Register core plugin (always available, always enabled)
    registry.register(
        Box::new(core::CorePlugin::new()),
        Some(&PluginConfig { enabled: true, settings: std::collections::HashMap::new() }),
    );

    // Register optional plugins based on feature flags
    #[cfg(feature = "plugin-elgato")]
    registry.register(
        Box::new(plugins::elgato::ElgatoPlugin::new()),
        Some(&make_config("elgato", false)),
    );

    #[cfg(feature = "plugin-obs")]
    registry.register(
        Box::new(plugins::obs::OBSPlugin::new()),
        Some(&make_config("obs", false)),  // Default disabled until user enables
    );

    Arc::new(registry)
}

/// Run the application, optionally starting hidden (no window shown)
pub fn run_with_options(start_hidden: bool) {
    // Initialize plugin registry
    let plugin_registry = create_plugin_registry();

    // Shared state for device info and bindings
    let device_info = Arc::new(Mutex::new(None));
    let bindings = Arc::new(Mutex::new(
        config::load_bindings().unwrap_or_else(|_| config::default_bindings()),
    ));
    let system_state = Arc::new(Mutex::new(state_manager::SystemState::default()));
    let current_page = Arc::new(Mutex::new(0usize));

    // Clone for the streamdeck thread
    let device_info_clone = Arc::clone(&device_info);
    let bindings_clone = Arc::clone(&bindings);
    let system_state_clone = Arc::clone(&system_state);
    let current_page_clone = Arc::clone(&current_page);
    let registry_clone = Arc::clone(&plugin_registry);

    // Clone for the state poller thread
    let system_state_poller = Arc::clone(&system_state);

    let app = Builder::default()
        .plugin(single_instance_init(|app, _args, _cwd| {
            // When a second instance is launched, show and focus the existing window
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState {
            device_info: Arc::clone(&device_info),
            bindings: Arc::clone(&bindings),
            system_state: Arc::clone(&system_state),
            current_page: Arc::clone(&current_page),
            plugin_registry: Arc::clone(&plugin_registry),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_device_info,
            commands::get_bindings,
            commands::get_capabilities,
            commands::set_binding,
            commands::remove_binding,
            commands::save_bindings,
            commands::sync_button_images,
            commands::get_system_state,
            commands::get_current_page,
            commands::set_current_page,
            commands::get_page_count,
            commands::get_plugins,
            commands::set_plugin_enabled,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let state_handle = app.handle().clone();

            // Hide window if starting in hidden mode
            if start_hidden {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            // Start hotplug monitor for device connection events
            hotplug::start_hotplug_monitor();

            // Start Stream Deck thread
            std::thread::spawn(move || {
                if let Err(e) = crate::streamdeck::run(
                    handle,
                    device_info_clone,
                    bindings_clone,
                    system_state_clone,
                    current_page_clone,
                    registry_clone,
                ) {
                    eprintln!("Stream Deck error: {:?}", e);
                }
            });

            // Start state poller thread
            std::thread::spawn(move || {
                state_manager::run_state_poller(state_handle, system_state_poller);
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Run the app with custom event handling to prevent exit on window close
    app.run(|app_handle, event| {
        match event {
            RunEvent::WindowEvent { label, event: WindowEvent::CloseRequested { api, .. }, .. } => {
                // Prevent the window from closing - just hide it instead
                api.prevent_close();
                if let Some(window) = app_handle.get_webview_window(&label) {
                    let _ = window.hide();
                }
                eprintln!("Window hidden - {} continues running in background", app_constants::APP_NAME);
            }
            RunEvent::ExitRequested { api, .. } => {
                // Prevent automatic exit - keep running in background
                api.prevent_exit();
            }
            _ => {}
        }
    });
}
