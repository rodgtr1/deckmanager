//! OBS Studio plugin for Deck Manager.
//!
//! This plugin provides control for OBS Studio via WebSocket (obs-websocket 5.x protocol):
//! - Scene switching
//! - Stream/Record control
//! - Source visibility
//! - Audio volume and mute
//! - Studio Mode
//! - Replay Buffer
//! - Virtual Camera
//!
//! Enable with feature flag: `plugin-obs`

pub mod client;
pub mod controller;
pub mod plugin;

pub use plugin::OBSPlugin;
