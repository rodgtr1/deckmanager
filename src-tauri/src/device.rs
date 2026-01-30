use elgato_streamdeck::info::Kind;
use serde::{Deserialize, Serialize};

/// Device information exposed to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub model: String,
    pub button_count: u8,
    pub encoder_count: u8,
    pub rows: u8,
    pub columns: u8,
    pub has_touch_strip: bool,
}

impl DeviceInfo {
    /// Create DeviceInfo from a Stream Deck Kind.
    pub fn from_kind(kind: Kind) -> Self {
        Self {
            model: kind_to_model_name(kind),
            button_count: kind.key_count(),
            encoder_count: kind.encoder_count(),
            rows: kind.row_count(),
            columns: kind.column_count(),
            has_touch_strip: kind.lcd_strip_size().is_some(),
        }
    }
}

/// Map Kind to human-readable model name.
fn kind_to_model_name(kind: Kind) -> String {
    match kind {
        Kind::Original => "Stream Deck Original".to_string(),
        Kind::OriginalV2 => "Stream Deck Original V2".to_string(),
        Kind::Mini => "Stream Deck Mini".to_string(),
        Kind::MiniMk2 => "Stream Deck Mini MK2".to_string(),
        Kind::Xl => "Stream Deck XL".to_string(),
        Kind::XlV2 => "Stream Deck XL V2".to_string(),
        Kind::Mk2 => "Stream Deck MK2".to_string(),
        Kind::Pedal => "Stream Deck Pedal".to_string(),
        Kind::Plus => "Stream Deck Plus".to_string(),
        Kind::Neo => "Stream Deck Neo".to_string(),
        _ => "Unknown Stream Deck".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_deck_plus_info() {
        let info = DeviceInfo::from_kind(Kind::Plus);
        assert_eq!(info.model, "Stream Deck Plus");
        assert_eq!(info.button_count, 8);
        assert_eq!(info.encoder_count, 4);
        assert_eq!(info.rows, 2);
        assert_eq!(info.columns, 4);
        assert!(info.has_touch_strip);
    }

    #[test]
    fn stream_deck_xl_info() {
        let info = DeviceInfo::from_kind(Kind::Xl);
        assert_eq!(info.model, "Stream Deck XL");
        assert_eq!(info.button_count, 32);
        assert_eq!(info.encoder_count, 0);
        assert_eq!(info.rows, 4);
        assert_eq!(info.columns, 8);
        assert!(!info.has_touch_strip);
    }

    #[test]
    fn stream_deck_mini_info() {
        let info = DeviceInfo::from_kind(Kind::Mini);
        assert_eq!(info.model, "Stream Deck Mini");
        assert_eq!(info.button_count, 6);
        assert_eq!(info.encoder_count, 0);
        assert_eq!(info.rows, 2);
        assert_eq!(info.columns, 3);
        assert!(!info.has_touch_strip);
    }
}
