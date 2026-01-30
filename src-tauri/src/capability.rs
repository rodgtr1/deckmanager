use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Capability {
    SystemVolume { step: f32 },
    ToggleMute,
    MediaPlayPause,
    MediaNext,
    MediaPrevious,
    MediaStop,
    RunCommand { command: String },
    LaunchApp { command: String },
    OpenURL { url: String },
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
    MediaNext,
    MediaPrevious,
    MediaStop,
    RunCommand(String),
    LaunchApp(String),
    OpenURL(String),
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
            Capability::MediaNext if pressed => Some(CapabilityEffect::MediaNext),
            Capability::MediaPrevious if pressed => Some(CapabilityEffect::MediaPrevious),
            Capability::MediaStop if pressed => Some(CapabilityEffect::MediaStop),
            Capability::RunCommand { command } if pressed => {
                Some(CapabilityEffect::RunCommand(command.clone()))
            }
            Capability::LaunchApp { command } if pressed => {
                Some(CapabilityEffect::LaunchApp(command.clone()))
            }
            Capability::OpenURL { url } if pressed => Some(CapabilityEffect::OpenURL(url.clone())),
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

    // ─────────────────────────────────────────────────────────────────
    // MediaNext capability tests
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn media_next_produces_effect_on_press() {
        let cap = Capability::MediaNext;
        assert_eq!(cap.apply_button(true), Some(CapabilityEffect::MediaNext));
    }

    #[test]
    fn media_next_no_effect_on_release() {
        let cap = Capability::MediaNext;
        assert_eq!(cap.apply_button(false), None);
    }

    // ─────────────────────────────────────────────────────────────────
    // MediaPrevious capability tests
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn media_previous_produces_effect_on_press() {
        let cap = Capability::MediaPrevious;
        assert_eq!(
            cap.apply_button(true),
            Some(CapabilityEffect::MediaPrevious)
        );
    }

    #[test]
    fn media_previous_no_effect_on_release() {
        let cap = Capability::MediaPrevious;
        assert_eq!(cap.apply_button(false), None);
    }

    // ─────────────────────────────────────────────────────────────────
    // MediaStop capability tests
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn media_stop_produces_effect_on_press() {
        let cap = Capability::MediaStop;
        assert_eq!(cap.apply_button(true), Some(CapabilityEffect::MediaStop));
    }

    #[test]
    fn media_stop_no_effect_on_release() {
        let cap = Capability::MediaStop;
        assert_eq!(cap.apply_button(false), None);
    }

    // ─────────────────────────────────────────────────────────────────
    // RunCommand capability tests
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn run_command_produces_effect_on_press() {
        let cap = Capability::RunCommand {
            command: "echo hello".to_string(),
        };
        assert_eq!(
            cap.apply_button(true),
            Some(CapabilityEffect::RunCommand("echo hello".to_string()))
        );
    }

    #[test]
    fn run_command_no_effect_on_release() {
        let cap = Capability::RunCommand {
            command: "echo hello".to_string(),
        };
        assert_eq!(cap.apply_button(false), None);
    }

    // ─────────────────────────────────────────────────────────────────
    // LaunchApp capability tests
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn launch_app_produces_effect_on_press() {
        let cap = Capability::LaunchApp {
            command: "firefox".to_string(),
        };
        assert_eq!(
            cap.apply_button(true),
            Some(CapabilityEffect::LaunchApp("firefox".to_string()))
        );
    }

    #[test]
    fn launch_app_no_effect_on_release() {
        let cap = Capability::LaunchApp {
            command: "firefox".to_string(),
        };
        assert_eq!(cap.apply_button(false), None);
    }

    // ─────────────────────────────────────────────────────────────────
    // OpenURL capability tests
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn open_url_produces_effect_on_press() {
        let cap = Capability::OpenURL {
            url: "https://github.com".to_string(),
        };
        assert_eq!(
            cap.apply_button(true),
            Some(CapabilityEffect::OpenURL("https://github.com".to_string()))
        );
    }

    #[test]
    fn open_url_no_effect_on_release() {
        let cap = Capability::OpenURL {
            url: "https://github.com".to_string(),
        };
        assert_eq!(cap.apply_button(false), None);
    }
}
