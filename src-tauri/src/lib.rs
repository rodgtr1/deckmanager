use std::sync::{Arc, Mutex};
use tauri::Builder;

mod binding;
mod button_renderer;
mod capability;
mod commands;
mod config;
mod device;
mod events;
mod hid;
mod input_processor;
mod streamdeck;

use commands::AppState;

pub fn run() {
    // Shared state for device info and bindings
    let device_info = Arc::new(Mutex::new(None));
    let bindings = Arc::new(Mutex::new(
        config::load_bindings().unwrap_or_else(|_| config::default_bindings()),
    ));

    // Clone for the streamdeck thread
    let device_info_clone = Arc::clone(&device_info);
    let bindings_clone = Arc::clone(&bindings);

    Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState {
            device_info: Arc::clone(&device_info),
            bindings: Arc::clone(&bindings),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_device_info,
            commands::get_bindings,
            commands::get_capabilities,
            commands::set_binding,
            commands::remove_binding,
            commands::save_bindings,
            commands::sync_button_images,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            std::thread::spawn(move || {
                if let Err(e) =
                    crate::streamdeck::run(handle, device_info_clone, bindings_clone)
                {
                    eprintln!("Stream Deck error: {:?}", e);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
