use tauri::Builder;

mod binding;
mod capability;
mod config;
mod events;
mod hid;
mod input_processor;
mod streamdeck;

pub fn run() {
    Builder::default()
        .setup(|app| {
            // 👇 grab a CLONED AppHandle immediately
            let handle = app.handle().clone();

            // 👇 spawn backend thread immediately
            std::thread::spawn(move || {
                if let Err(e) = crate::streamdeck::run(handle) {
                    eprintln!("Stream Deck error: {:?}", e);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
