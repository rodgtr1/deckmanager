//! Elgato Key Light plugin for ArchDeck.
//!
//! This plugin provides control for Elgato Key Light devices:
//! - Toggle on/off with button press
//! - Adjust brightness with encoder rotation
//!
//! Enable with feature flag: `plugin-elgato`

pub mod client;
pub mod controller;
pub mod plugin;

// Re-export for state management
#[allow(unused_imports)]
pub use client::KeyLightState;
pub use plugin::ElgatoPlugin;
