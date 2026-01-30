use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Capability {
    SystemVolume { step: f32 },
    ToggleMute,
    MediaPlayPause,
}

/// Effects produced when a capability is triggered.
///
/// These are the concrete actions to be executed by the effect handler.
#[derive(Debug, PartialEq)]
#[allow(dead_code)] // Reserved for future effect-based dispatch
pub enum CapabilityEffect {
    VolumeDelta(f32),
    ToggleMute,
    MediaPlayPause,
}

#[allow(dead_code)] // Reserved for future effect-based dispatch
impl Capability {
    pub fn apply_encoder(&self, delta: i8) -> Option<CapabilityEffect> {
        match self {
            Capability::SystemVolume { step } => {
                if delta == 0 {
                    None
                } else {
                    Some(CapabilityEffect::VolumeDelta(*step * delta as f32))
                }
            }
            _ => None,
        }
    }

    pub fn apply_button(&self, pressed: bool) -> Option<CapabilityEffect> {
        match self {
            Capability::ToggleMute if pressed => Some(CapabilityEffect::ToggleMute),
            Capability::MediaPlayPause if pressed => Some(CapabilityEffect::MediaPlayPause),
            _ => None,
        }
    }
}

/// Clamps a volume value to the valid range [0.0, 1.0].
#[allow(dead_code)] // Available for future volume handling
pub fn clamp_volume(volume: f32) -> f32 {
    volume.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────
    // ToggleMute capability tests
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn toggle_mute_produces_effect_on_press() {
        let cap = Capability::ToggleMute;
        let effect = cap.apply_button(true);
        assert_eq!(effect, Some(CapabilityEffect::ToggleMute));
    }

    #[test]
    fn toggle_mute_no_effect_on_release() {
        let cap = Capability::ToggleMute;
        let effect = cap.apply_button(false);
        assert_eq!(effect, None);
    }

    // ─────────────────────────────────────────────────────────────────
    // SystemVolume capability tests
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn volume_encoder_positive_delta() {
        let cap = Capability::SystemVolume { step: 0.02 };
        let effect = cap.apply_encoder(1);
        assert_eq!(effect, Some(CapabilityEffect::VolumeDelta(0.02)));
    }

    #[test]
    fn volume_encoder_negative_delta() {
        let cap = Capability::SystemVolume { step: 0.02 };
        let effect = cap.apply_encoder(-1);
        assert_eq!(effect, Some(CapabilityEffect::VolumeDelta(-0.02)));
    }

    #[test]
    fn volume_encoder_zero_delta_no_effect() {
        let cap = Capability::SystemVolume { step: 0.02 };
        let effect = cap.apply_encoder(0);
        assert_eq!(effect, None);
    }

    #[test]
    fn volume_encoder_scales_with_delta() {
        let cap = Capability::SystemVolume { step: 0.05 };
        let effect = cap.apply_encoder(3);
        assert_eq!(effect, Some(CapabilityEffect::VolumeDelta(0.15)));
    }

    // ─────────────────────────────────────────────────────────────────
    // Volume clamping tests
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn clamp_volume_within_range() {
        assert_eq!(clamp_volume(0.5), 0.5);
    }

    #[test]
    fn clamp_volume_at_boundaries() {
        assert_eq!(clamp_volume(0.0), 0.0);
        assert_eq!(clamp_volume(1.0), 1.0);
    }

    #[test]
    fn clamp_volume_below_zero() {
        assert_eq!(clamp_volume(-0.1), 0.0);
        assert_eq!(clamp_volume(-1.0), 0.0);
    }

    #[test]
    fn clamp_volume_above_one() {
        assert_eq!(clamp_volume(1.1), 1.0);
        assert_eq!(clamp_volume(2.0), 1.0);
    }

    // ─────────────────────────────────────────────────────────────────
    // Capability-event type mismatch tests
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn toggle_mute_ignores_encoder_input() {
        let cap = Capability::ToggleMute;
        let effect = cap.apply_encoder(1);
        assert_eq!(effect, None);
    }

    #[test]
    fn volume_ignores_button_input() {
        let cap = Capability::SystemVolume { step: 0.02 };
        let effect = cap.apply_button(true);
        assert_eq!(effect, None);
    }

    // ─────────────────────────────────────────────────────────────────
    // MediaPlayPause capability tests
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn media_play_pause_produces_effect_on_press() {
        let cap = Capability::MediaPlayPause;
        assert_eq!(cap.apply_button(true), Some(CapabilityEffect::MediaPlayPause));
    }

    #[test]
    fn media_play_pause_no_effect_on_release() {
        let cap = Capability::MediaPlayPause;
        assert_eq!(cap.apply_button(false), None);
    }

    #[test]
    fn media_play_pause_ignores_encoder_input() {
        let cap = Capability::MediaPlayPause;
        assert_eq!(cap.apply_encoder(1), None);
    }
}
