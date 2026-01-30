use hidapi::HidDevice;

/// Stream Deck Plus encoder press detector
///
/// This reads raw HID reports and extracts encoder press bits.
/// It ONLY cares about press state — nothing else.
pub struct EncoderPressReader {
    device: HidDevice,
    last_state: Vec<bool>,
}

impl EncoderPressReader {
    pub fn new(device: HidDevice, encoder_count: usize) -> Self {
        Self {
            device,
            last_state: vec![false; encoder_count],
        }
    }

    /// Polls HID and returns encoder press/release events
    pub fn poll(&mut self) -> Vec<(usize, bool)> {
        let mut buf = [0u8; 64];

        let Ok(len) = self.device.read_timeout(&mut buf, 0) else {
            return vec![];
        };

        if len == 0 {
            return vec![];
        }

        // 🔎 Based on your hidraw dump:
        // byte 4 (0-based) contains encoder press bit(s)
        let press_byte = buf[4];

        let mut events = vec![];

        for i in 0..self.last_state.len() {
            let pressed = (press_byte & (1 << i)) != 0;
            let prev = self.last_state[i];

            if pressed != prev {
                events.push((i, pressed));
                self.last_state[i] = pressed;
            }
        }

        events
    }
}
