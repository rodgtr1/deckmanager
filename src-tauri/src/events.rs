use serde::Serialize;

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ButtonEvent {
    pub index: usize,
    pub pressed: bool,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct EncoderEvent {
    pub index: usize,
    pub delta: i8,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct TouchSwipeEvent {
    pub start: (u16, u16),
    pub end: (u16, u16),
}
