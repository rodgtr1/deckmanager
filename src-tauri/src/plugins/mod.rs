//! Optional plugins for ArchDeck.
//!
//! Plugins are conditionally compiled based on Cargo feature flags.

#[cfg(feature = "plugin-elgato")]
pub mod elgato;
