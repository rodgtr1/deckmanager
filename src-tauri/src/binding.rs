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
