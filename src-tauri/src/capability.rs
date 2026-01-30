#[derive(Debug, Clone, PartialEq)]
pub enum Capability {
    SystemVolume { step: f32 },
    ToggleMute,
}

#[derive(Debug, PartialEq)]
pub enum CapabilityEffect {
    VolumeDelta(f32),
    ToggleMute,
}

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
            Capability::ToggleMute if pressed => {
                // only fire on press, not release
                Some(CapabilityEffect::ToggleMute)
            }
            _ => None,
        }
    }
}
