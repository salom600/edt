//! edt-core — Core data model for the edt video editor.
//!
//! This crate is intentionally free of I/O and UI concerns. It owns the
//! pure data structures that describe a project: media assets, the
//! timeline (tracks + clips), effects, transitions, and export settings.
//!
//! All types are `Serialize`/`Deserialize` so the entire project state can
//! be persisted to JSON via [`edt_storage`].
//!
//! ## Time model
//!
//! All time values are stored as `f64` seconds in the project's native
//! framerate. This avoids integer-overflow issues with long projects and
//! makes math with audio sample positions straightforward. A project also
//! stores a `fps: f64` field so frames can be derived when needed:
//! `frame_idx = round(t * fps)`.

pub mod id;
pub mod media;
pub mod project;
pub mod timeline;
pub mod effect;
pub mod transition;
pub mod export;
pub mod time;

pub use id::{Id, IdGenerator};
pub use media::{MediaAsset, MediaKind, MediaMetadata, VideoInfo, AudioInfo};
pub use project::{Project, ProjectSettings};
pub use timeline::{Timeline, Track, TrackKind, Clip, ClipSource, ClipBounds};
pub use effect::{Effect, EffectKind, ColorGrade};
pub use transition::{Transition, TransitionKind};
pub use export::{ExportSettings, ExportFormat, ExportVideoCodec, ExportAudioCodec};

pub use time::{Time, TimeRange};

/// Re-export of the current project file format version.
///
/// Bump this when the on-disk schema changes in a breaking way.
/// `edt_storage` will refuse to load projects from incompatible major
/// versions and surface a clear error.
pub const PROJECT_FORMAT_VERSION: u32 = 1;
