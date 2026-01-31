use std::sync::{Arc, Mutex};
use tauri::{Builder, Manager, RunEvent, WindowEvent};

mod binding;
mod button_renderer;
mod capability;
mod commands;
mod config;
mod device;
mod elgato_key_light;
mod events;
mod hid;
mod image_cache;
mod input_processor;
mod key_light_controller;
mod state_manager;
mod streamdeck;

// Include generated constants from build.rs
pub mod app_constants {
    include!(concat!(env!("OUT_DIR"), "/app_constants.rs"));
}

use commands::AppState;

/// Run the application with optional hidden mode
pub fn run() {
    run_with_options(false);
}

/// Run the application, optionally starting hidden (no window shown)
pub fn run_with_options(start_hidden: bool) {
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

    // Clone for the state poller thread
    let system_state_poller = Arc::clone(&system_state);

    let app = Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState {
            device_info: Arc::clone(&device_info),
            bindings: Arc::clone(&bindings),
            system_state: Arc::clone(&system_state),
            current_page: Arc::clone(&current_page),
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

            // Start Stream Deck thread
            std::thread::spawn(move || {
                if let Err(e) = crate::streamdeck::run(
                    handle,
                    device_info_clone,
                    bindings_clone,
                    system_state_clone,
                    current_page_clone,
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
