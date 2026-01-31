use std::sync::{Arc, Mutex};
use tauri::Builder;

mod binding;
mod button_renderer;
mod capability;
mod commands;
mod config;
mod device;
mod elgato_key_light;
mod events;
mod hid;
mod input_processor;
mod state_manager;
mod streamdeck;

use commands::AppState;

pub fn run() {
    // Shared state for device info and bindings
    let device_info = Arc::new(Mutex::new(None));
    let bindings = Arc::new(Mutex::new(
        config::load_bindings().unwrap_or_else(|_| config::default_bindings()),
    ));
    let system_state = Arc::new(Mutex::new(state_manager::SystemState::default()));

    // Clone for the streamdeck thread
    let device_info_clone = Arc::clone(&device_info);
    let bindings_clone = Arc::clone(&bindings);
    let system_state_clone = Arc::clone(&system_state);

    // Clone for the state poller thread
    let system_state_poller = Arc::clone(&system_state);

    Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState {
            device_info: Arc::clone(&device_info),
            bindings: Arc::clone(&bindings),
            system_state: Arc::clone(&system_state),
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
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let state_handle = app.handle().clone();

            // Start Stream Deck thread
            std::thread::spawn(move || {
                if let Err(e) =
                    crate::streamdeck::run(handle, device_info_clone, bindings_clone, system_state_clone)
                {
                    eprintln!("Stream Deck error: {:?}", e);
                }
            });

            // Start state poller thread
            std::thread::spawn(move || {
                state_manager::run_state_poller(state_handle, system_state_poller);
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
