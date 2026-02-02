//! Optional plugins for Deck Manager.
//!
//! Plugins are conditionally compiled based on Cargo feature flags.

#[cfg(feature = "plugin-elgato")]
pub mod elgato;

#[cfg(feature = "plugin-obs")]
pub mod obs;
