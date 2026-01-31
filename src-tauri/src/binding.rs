use crate::capability::Capability;
use crate::input_processor::LogicalEvent;
use serde::{Deserialize, Serialize};

/// Reference to a specific input on the Stream Deck.
///
/// Used in bindings to specify which input triggers a capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum InputRef {
    Button { index: usize },
    Encoder { index: usize },
    EncoderPress { index: usize },
    Swipe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    pub input: InputRef,
    pub capability: Capability,
    /// Which page this binding belongs to (0-indexed)
    #[serde(default)]
    pub page: usize,
    /// Custom emoji or icon name for this binding (UI only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Custom display text for this binding (UI only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// File path or URL for hardware button image (default state)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button_image: Option<String>,
    /// Alternate image shown when state is "active" (e.g., muted, playing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button_image_alt: Option<String>,
    /// Whether to render label on hardware button
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_label: Option<bool>,
}

impl Binding {
    pub fn matches(&self, event: &LogicalEvent) -> bool {
        match (&self.input, event) {
            (InputRef::Button { index }, LogicalEvent::Button(e)) => e.index == *index,
            (InputRef::Encoder { index }, LogicalEvent::Encoder(e)) => e.index == *index,
            (InputRef::EncoderPress { index }, LogicalEvent::EncoderPress(e)) => e.index == *index,
            (InputRef::Swipe, LogicalEvent::Swipe(_)) => true,
            _ => false,
        }
    }
}
