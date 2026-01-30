use crate::capability::Capability;
use crate::input_processor::LogicalEvent;

/// Reference to a specific input on the Stream Deck.
///
/// Used in bindings to specify which input triggers a capability.
#[derive(Debug, Clone)]
#[non_exhaustive]
#[allow(dead_code)] // Variants reserved for future config-driven bindings
pub enum InputRef {
    Button { index: usize },
    Encoder { index: usize },
    EncoderPress { index: usize },
    Swipe,
}

#[derive(Debug, Clone)]
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
