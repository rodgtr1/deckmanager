//! USB hotplug monitoring using udev.
//!
//! Monitors for Elgato Stream Deck device connection/disconnection events
//! and signals the streamdeck module to attempt reconnection.

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

/// Elgato vendor ID
const ELGATO_VENDOR_ID: &str = "0fd9";

/// Flag to signal that a device was just connected and we should try to reconnect.
pub static DEVICE_CONNECTED_FLAG: AtomicBool = AtomicBool::new(false);

/// Check and clear the device connected flag.
/// Returns true if a device was recently connected.
pub fn check_device_connected() -> bool {
    DEVICE_CONNECTED_FLAG.swap(false, Ordering::SeqCst)
}

/// Start the hotplug monitor in a background thread.
/// This monitors udev events for hidraw devices with the Elgato vendor ID.
pub fn start_hotplug_monitor() {
    thread::spawn(|| {
        if let Err(e) = run_monitor_loop() {
            eprintln!("Hotplug monitor error: {}", e);
        }
    });
}

fn run_monitor_loop() -> Result<(), Box<dyn std::error::Error>> {
    let socket = udev::MonitorBuilder::new()?
        .match_subsystem("hidraw")?
        .listen()?;

    eprintln!("Hotplug monitor started, watching for Elgato devices...");

    for event in socket.iter() {
        let device = event.device();

        // Check if this is an Elgato device
        if !is_elgato_device(&device) {
            continue;
        }

        match event.event_type() {
            udev::EventType::Add => {
                eprintln!("Elgato device connected via hotplug");
                DEVICE_CONNECTED_FLAG.store(true, Ordering::SeqCst);
            }
            udev::EventType::Remove => {
                eprintln!("Elgato device disconnected");
                // The streamdeck module handles disconnection via read errors
            }
            _ => {}
        }
    }

    Ok(())
}

/// Check if the device belongs to Elgato by looking at the parent USB device.
fn is_elgato_device(device: &udev::Device) -> bool {
    // Walk up to the USB device to check the vendor ID
    let mut current = device.parent();
    while let Some(parent) = current {
        if let Some(vendor) = parent.attribute_value("idVendor") {
            if vendor.to_string_lossy().to_lowercase() == ELGATO_VENDOR_ID {
                return true;
            }
        }
        current = parent.parent();
    }
    false
}
