use serde::{Deserialize, Serialize};

/// Brightness change percentage per encoder tick for Key Lights
pub const KEY_LIGHT_BRIGHTNESS_STEP: i32 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KeyLightAction {
    Toggle,
    On,
    Off,
    SetBrightness, // Uses encoder delta
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Capability {
    /// System audio control (for encoders)
    /// - Encoder rotation: adjust volume
    /// - Encoder press: toggle mute
    SystemAudio { step: f32 },
    /// Toggle system mute (for buttons)
    Mute,
    /// Increase system volume (for buttons)
    VolumeUp { step: f32 },
    /// Decrease system volume (for buttons)
    VolumeDown { step: f32 },
    /// Microphone control (for encoders)
    /// - Encoder rotation: adjust mic volume
    /// - Encoder press: toggle mic mute
    Microphone { step: f32 },
    /// Toggle mic mute (for buttons)
    MicMute,
    /// Increase mic volume (for buttons)
    MicVolumeUp { step: f32 },
    /// Decrease mic volume (for buttons)
    MicVolumeDown { step: f32 },
    MediaPlayPause,
    MediaNext,
    MediaPrevious,
    MediaStop,
    RunCommand {
        command: String,
        #[serde(default)]
        toggle: bool,
    },
    LaunchApp { command: String },
    OpenURL { url: String },
    ElgatoKeyLight {
        ip: String,
        #[serde(default = "default_key_light_port")]
        port: u16,
        action: KeyLightAction,
    },
}

fn default_key_light_port() -> u16 {
    9123
}

/// Effects produced when a capability is triggered.
///
/// These are the concrete actions to be executed by the effect handler.
#[derive(Debug, PartialEq)]
#[allow(dead_code)] // Reserved for future effect-based dispatch
pub enum CapabilityEffect {
    VolumeDelta(f32),
    ToggleMute,
    MicVolumeDelta(f32),
    ToggleMicMute,
    MediaPlayPause,
    MediaNext,
    MediaPrevious,
    MediaStop,
    RunCommand(String),
    LaunchApp(String),
    OpenURL(String),
    KeyLightToggle { ip: String, port: u16 },
    KeyLightOn { ip: String, port: u16 },
    KeyLightOff { ip: String, port: u16 },
    KeyLightBrightness { ip: String, port: u16, delta: i32 },
}

#[allow(dead_code)] // Reserved for future effect-based dispatch
impl Capability {
    pub fn apply_encoder(&self, delta: i8) -> Option<CapabilityEffect> {
        match self {
            Capability::SystemAudio { step } => {
                if delta == 0 {
                    None
                } else {
                    Some(CapabilityEffect::VolumeDelta(*step * delta as f32))
                }
            }
            Capability::Microphone { step } => {
                if delta == 0 {
                    None
                } else {
                    Some(CapabilityEffect::MicVolumeDelta(*step * delta as f32))
                }
            }
            Capability::ElgatoKeyLight { ip, port, action: KeyLightAction::SetBrightness } => {
                if delta == 0 {
                    None
                } else {
                    Some(CapabilityEffect::KeyLightBrightness {
                        ip: ip.clone(),
                        port: *port,
                        delta: delta as i32 * KEY_LIGHT_BRIGHTNESS_STEP,
                    })
                }
            }
            _ => None,
        }
    }

    pub fn apply_button(&self, pressed: bool) -> Option<CapabilityEffect> {
        match self {
            Capability::SystemAudio { .. } if pressed => Some(CapabilityEffect::ToggleMute),
            Capability::Mute if pressed => Some(CapabilityEffect::ToggleMute),
            Capability::VolumeUp { step } if pressed => Some(CapabilityEffect::VolumeDelta(*step)),
            Capability::VolumeDown { step } if pressed => Some(CapabilityEffect::VolumeDelta(-*step)),
            Capability::Microphone { .. } if pressed => Some(CapabilityEffect::ToggleMicMute),
            Capability::MicMute if pressed => Some(CapabilityEffect::ToggleMicMute),
            Capability::MicVolumeUp { step } if pressed => Some(CapabilityEffect::MicVolumeDelta(*step)),
            Capability::MicVolumeDown { step } if pressed => Some(CapabilityEffect::MicVolumeDelta(-*step)),
            Capability::MediaPlayPause if pressed => Some(CapabilityEffect::MediaPlayPause),
            Capability::MediaNext if pressed => Some(CapabilityEffect::MediaNext),
            Capability::MediaPrevious if pressed => Some(CapabilityEffect::MediaPrevious),
            Capability::MediaStop if pressed => Some(CapabilityEffect::MediaStop),
            Capability::RunCommand { command, .. } if pressed => {
                Some(CapabilityEffect::RunCommand(command.clone()))
            }
            Capability::LaunchApp { command } if pressed => {
                Some(CapabilityEffect::LaunchApp(command.clone()))
            }
            Capability::OpenURL { url } if pressed => Some(CapabilityEffect::OpenURL(url.clone())),
            Capability::ElgatoKeyLight { ip, port, action } if pressed => {
                match action {
                    KeyLightAction::Toggle => Some(CapabilityEffect::KeyLightToggle {
                        ip: ip.clone(),
                        port: *port,
                    }),
                    KeyLightAction::On => Some(CapabilityEffect::KeyLightOn {
                        ip: ip.clone(),
                        port: *port,
                    }),
                    KeyLightAction::Off => Some(CapabilityEffect::KeyLightOff {
                        ip: ip.clone(),
                        port: *port,
                    }),
                    KeyLightAction::SetBrightness => None, // Handled by encoder
                }
            }
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
    // SystemAudio capability tests
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn system_audio_mute_on_button_press() {
        let cap = Capability::SystemAudio { step: 0.02 };
        let effect = cap.apply_button(true);
        assert_eq!(effect, Some(CapabilityEffect::ToggleMute));
    }

    #[test]
    fn system_audio_no_effect_on_button_release() {
        let cap = Capability::SystemAudio { step: 0.02 };
        let effect = cap.apply_button(false);
        assert_eq!(effect, None);
    }

    #[test]
    fn system_audio_volume_on_encoder_positive() {
        let cap = Capability::SystemAudio { step: 0.02 };
        let effect = cap.apply_encoder(1);
        assert_eq!(effect, Some(CapabilityEffect::VolumeDelta(0.02)));
    }

    #[test]
    fn system_audio_volume_on_encoder_negative() {
        let cap = Capability::SystemAudio { step: 0.02 };
        let effect = cap.apply_encoder(-1);
        assert_eq!(effect, Some(CapabilityEffect::VolumeDelta(-0.02)));
    }

    #[test]
    fn system_audio_no_effect_on_encoder_zero() {
        let cap = Capability::SystemAudio { step: 0.02 };
        let effect = cap.apply_encoder(0);
        assert_eq!(effect, None);
    }

    #[test]
    fn system_audio_encoder_scales_with_delta() {
        let cap = Capability::SystemAudio { step: 0.05 };
        let effect = cap.apply_encoder(3);
        assert_eq!(effect, Some(CapabilityEffect::VolumeDelta(0.15)));
    }

    // ─────────────────────────────────────────────────────────────────
    // Microphone capability tests
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn microphone_mute_on_button_press() {
        let cap = Capability::Microphone { step: 0.02 };
        let effect = cap.apply_button(true);
        assert_eq!(effect, Some(CapabilityEffect::ToggleMicMute));
    }

    #[test]
    fn microphone_no_effect_on_button_release() {
        let cap = Capability::Microphone { step: 0.02 };
        let effect = cap.apply_button(false);
        assert_eq!(effect, None);
    }

    #[test]
    fn microphone_volume_on_encoder_positive() {
        let cap = Capability::Microphone { step: 0.02 };
        let effect = cap.apply_encoder(1);
        assert_eq!(effect, Some(CapabilityEffect::MicVolumeDelta(0.02)));
    }

    #[test]
    fn microphone_volume_on_encoder_negative() {
        let cap = Capability::Microphone { step: 0.02 };
        let effect = cap.apply_encoder(-1);
        assert_eq!(effect, Some(CapabilityEffect::MicVolumeDelta(-0.02)));
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
            toggle: false,
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
            toggle: false,
        };
        assert_eq!(cap.apply_button(false), None);
    }

    #[test]
    fn run_command_toggle_produces_effect_on_press() {
        let cap = Capability::RunCommand {
            command: "dictation-toggle".to_string(),
            toggle: true,
        };
        assert_eq!(
            cap.apply_button(true),
            Some(CapabilityEffect::RunCommand("dictation-toggle".to_string()))
        );
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
