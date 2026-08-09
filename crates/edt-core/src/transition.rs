//! Transition model — short effects applied at the boundary between two
//! adjacent clips on the same track.

use crate::id::Id;
use crate::time::Time;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKind {
    /// Simple hard cut. Stored for completeness; usually absent from data.
    Cut,
    /// Cross-fade between outgoing and incoming clip.
    Dissolve,
    /// Dip through black.
    DipToBlack,
    /// Wipe left-to-right.
    WipeLeft,
    /// Wipe right-to-left.
    WipeRight,
    /// Audio cross-fade.
    AudioCrossfade,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub id: Id,
    pub kind: TransitionKind,
    /// Outgoing clip.
    pub from_clip: Id,
    /// Incoming clip.
    pub to_clip: Id,
    /// Duration of the overlap region.
    pub duration: Time,
}

impl Transition {
    pub fn new(id: Id, kind: TransitionKind, from_clip: Id, to_clip: Id, duration: Time) -> Self {
        Self {
            id,
            kind,
            from_clip,
            to_clip,
            duration,
        }
    }
}
