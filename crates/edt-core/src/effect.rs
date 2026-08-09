//! Effect model. Effects are non-destructive operations applied to a
//! clip's frames or audio samples during preview and export.

use crate::id::Id;
use serde::{Deserialize, Serialize};

/// A color grade applied to a clip. All values are linear multipliers or
/// offsets in the [0,1] range (lift) or [-1,1] range (offset).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorGrade {
    /// Per-channel lift (shadows): -1.0..=1.0.
    pub lift_r: f32,
    pub lift_g: f32,
    pub lift_b: f32,
    /// Per-channel gamma (midtones): 0.1..=4.0, 1.0 = neutral.
    pub gamma_r: f32,
    pub gamma_g: f32,
    pub gamma_b: f32,
    /// Per-channel gain (highlights): 0.0..=4.0, 1.0 = neutral.
    pub gain_r: f32,
    pub gain_g: f32,
    pub gain_b: f32,
    /// Overall saturation: 0.0..=2.0, 1.0 = neutral.
    pub saturation: f32,
    /// Overall brightness offset: -1.0..=1.0.
    pub brightness: f32,
    /// Overall contrast multiplier: 0.0..=2.0, 1.0 = neutral.
    pub contrast: f32,
}

impl Default for ColorGrade {
    fn default() -> Self {
        Self {
            lift_r: 0.0, lift_g: 0.0, lift_b: 0.0,
            gamma_r: 1.0, gamma_g: 1.0, gamma_b: 1.0,
            gain_r: 1.0, gain_g: 1.0, gain_b: 1.0,
            saturation: 1.0,
            brightness: 0.0,
            contrast: 1.0,
        }
    }
}

/// Concrete effect variants. New variants should be added at the end to
/// keep deserialization of older project files stable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EffectKind {
    ColorGrade(ColorGrade),
    /// Gaussian blur, radius in pixels.
    Blur { radius: f32 },
    /// Simple horizontal/vertical flip.
    Flip { horizontal: bool, vertical: bool },
    /// Rotation in degrees. Must be a multiple of 90 for the MVP.
    Rotate { degrees: i32 },
    /// Crop rectangle as fractions of source size (0.0..=1.0).
    Crop { x: f32, y: f32, w: f32, h: f32 },
    /// Scale factor relative to source size.
    Scale { factor: f32 },
    /// Opacity multiplier (0.0..=1.0). Composed with the clip's own opacity.
    Opacity { value: f32 },
    /// Audio volume gain in dB. 0.0 = unity.
    VolumeGain { db: f32 },
    /// Audio fade in over N seconds at clip start.
    FadeIn { duration: f32 },
    /// Audio fade out over N seconds at clip end.
    FadeOut { duration: f32 },
    /// Text overlay (MVP: fixed position, no keyframes).
    Text {
        text: String,
        font_size: f32,
        x: f32,
        y: f32,
        color: [f32; 4],
    },
}

/// An effect instance attached to a clip. Effects form an ordered stack
/// applied bottom-to-top during render.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Effect {
    pub id: Id,
    pub enabled: bool,
    pub kind: EffectKind,
}

impl Effect {
    pub fn new(id: Id, kind: EffectKind) -> Self {
        Self { id, enabled: true, kind }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_grade_default_is_neutral() {
        let g = ColorGrade::default();
        assert_eq!(g.saturation, 1.0);
        assert_eq!(g.brightness, 0.0);
        assert_eq!(g.contrast, 1.0);
    }

    #[test]
    fn effect_roundtrips_through_json() {
        let e = Effect::new(Id(1), EffectKind::Blur { radius: 2.5 });
        let s = serde_json::to_string(&e).unwrap();
        let back: Effect = serde_json::from_str(&s).unwrap();
        assert_eq!(e.kind, back.kind);
    }
}
