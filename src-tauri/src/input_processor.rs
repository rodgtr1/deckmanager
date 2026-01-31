use crate::events::{ButtonEvent, EncoderEvent, TouchSwipeEvent};

/// Direction of a swipe gesture on the touch strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeDirection {
    Left,
    Right,
}

/// Minimum horizontal distance (in pixels) to consider a swipe intentional.
const SWIPE_THRESHOLD: i32 = 50;

/// Detect swipe direction from start and end coordinates.
/// Returns None if the swipe was too short or primarily vertical.
pub fn detect_swipe_direction(start: (u16, u16), end: (u16, u16)) -> Option<SwipeDirection> {
    let dx = end.0 as i32 - start.0 as i32;
    let dy = (end.1 as i32 - start.1 as i32).abs();

    // Ignore if vertical movement is greater than horizontal (not a page swipe)
    if dy > dx.abs() {
        return None;
    }

    // Check if swipe exceeds threshold
    if dx > SWIPE_THRESHOLD {
        Some(SwipeDirection::Right)
    } else if dx < -SWIPE_THRESHOLD {
        Some(SwipeDirection::Left)
    } else {
        None
    }
}

#[derive(Default)]
pub struct InputProcessor {
    last_buttons: Vec<bool>,
    last_encoders: Vec<bool>,
}

/// Normalized input events from the Stream Deck.
///
/// Each variant represents a distinct input type:
/// - `Button`: Physical button press/release (index + pressed state)
/// - `Encoder`: Rotary encoder twist (index + delta)
/// - `EncoderPress`: Encoder push button (index + pressed state)
/// - `Swipe`: Touch screen swipe gesture (start + end coordinates)
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum LogicalEvent {
    Button(ButtonEvent),
    Encoder(EncoderEvent),
    EncoderPress(ButtonEvent),
    Swipe(TouchSwipeEvent),
}

impl InputProcessor {
    pub fn process_buttons(&mut self, states: &[bool]) -> Vec<LogicalEvent> {
        let mut events = Vec::new();

        // First frame: emit DOWN for any pressed buttons
        if self.last_buttons.is_empty() {
            for (i, &pressed) in states.iter().enumerate() {
                if pressed {
                    events.push(LogicalEvent::Button(ButtonEvent {
                        index: i,
                        pressed: true,
                    }));
                }
            }
            self.last_buttons = states.to_vec();
            return events;
        }

        for (i, (&prev, &curr)) in self.last_buttons.iter().zip(states).enumerate() {
            if prev != curr {
                events.push(LogicalEvent::Button(ButtonEvent {
                    index: i,
                    pressed: curr,
                }));
            }
        }

        self.last_buttons = states.to_vec();
        events
    }

    pub fn process_encoders(&self, deltas: &[i8]) -> Vec<LogicalEvent> {
        deltas
            .iter()
            .enumerate()
            .filter(|(_, &d)| d != 0)
            .map(|(i, &d)| LogicalEvent::Encoder(EncoderEvent { index: i, delta: d }))
            .collect()
    }

    pub fn process_encoder_presses(&mut self, states: &[bool]) -> Vec<LogicalEvent> {
        let mut events = Vec::new();

        // First frame: emit PRESS for any encoders already pressed
        if self.last_encoders.is_empty() {
            for (i, &pressed) in states.iter().enumerate() {
                if pressed {
                    events.push(LogicalEvent::EncoderPress(ButtonEvent {
                        index: i,
                        pressed: true,
                    }));
                }
            }

            self.last_encoders = states.to_vec();
            return events;
        }

        for (i, (&prev, &curr)) in self.last_encoders.iter().zip(states).enumerate() {
            if prev != curr {
                events.push(LogicalEvent::EncoderPress(ButtonEvent {
                    index: i,
                    pressed: curr,
                }));
            }
        }

        self.last_encoders = states.to_vec();
        events
    }

    pub fn process_swipe(&self, start: (u16, u16), end: (u16, u16)) -> LogicalEvent {
        LogicalEvent::Swipe(TouchSwipeEvent { start, end })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_button_press_emits_down() {
        let mut p = InputProcessor::default();

        let events = p.process_buttons(&[true, false, false]);

        assert_eq!(events.len(), 1);
        match &events[0] {
            LogicalEvent::Button(e) => {
                assert_eq!(e.index, 0);
                assert!(e.pressed);
            }
            _ => panic!("wrong event"),
        }
    }

    #[test]
    fn button_up_and_down_detected() {
        let mut p = InputProcessor::default();

        p.process_buttons(&[false, false]);
        let events = p.process_buttons(&[true, false]);

        assert_eq!(events.len(), 1);
        match &events[0] {
            LogicalEvent::Button(e) => {
                assert_eq!(e.index, 0);
                assert!(e.pressed);
            }
            _ => panic!("wrong event"),
        }

        let events = p.process_buttons(&[false, false]);
        match &events[0] {
            LogicalEvent::Button(e) => {
                assert!(!e.pressed);
            }
            _ => panic!("wrong event"),
        }
    }

    #[test]
    fn encoder_zero_deltas_ignored() {
        let p = InputProcessor::default();

        let events = p.process_encoders(&[0, 0, 0]);
        assert!(events.is_empty());
    }

    #[test]
    fn encoder_deltas_preserved() {
        let p = InputProcessor::default();

        let events = p.process_encoders(&[1, -1, 0]);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn swipe_event_is_forwarded() {
        let p = InputProcessor::default();

        let event = p.process_swipe((10, 20), (100, 20));

        match event {
            LogicalEvent::Swipe(e) => {
                assert_eq!(e.start, (10, 20));
                assert_eq!(e.end, (100, 20));
            }
            _ => panic!("expected swipe event"),
        }
    }
    #[test]
    fn initial_encoder_press_emits_down() {
        let mut p = InputProcessor::default();

        let events = p.process_encoder_presses(&[true, false]);

        assert_eq!(events.len(), 1);
        match &events[0] {
            LogicalEvent::EncoderPress(e) => {
                assert_eq!(e.index, 0);
                assert!(e.pressed);
            }
            _ => panic!("wrong event"),
        }
    }

    #[test]
    fn swipe_right_detected() {
        let direction = detect_swipe_direction((100, 50), (200, 50));
        assert_eq!(direction, Some(SwipeDirection::Right));
    }

    #[test]
    fn swipe_left_detected() {
        let direction = detect_swipe_direction((200, 50), (100, 50));
        assert_eq!(direction, Some(SwipeDirection::Left));
    }

    #[test]
    fn short_swipe_ignored() {
        let direction = detect_swipe_direction((100, 50), (130, 50));
        assert_eq!(direction, None);
    }

    #[test]
    fn vertical_swipe_ignored() {
        let direction = detect_swipe_direction((100, 50), (200, 200));
        assert_eq!(direction, None);
    }
}
