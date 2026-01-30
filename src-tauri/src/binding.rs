use crate::capability::Capability;
use crate::input_processor::LogicalEvent;

#[derive(Debug, Clone)]
pub enum InputRef {
    Encoder { index: usize },
    Button { index: usize },
    EncoderPress { index: usize },
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

            _ => false,
        }
    }
}
