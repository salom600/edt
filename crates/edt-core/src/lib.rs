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

pub mod effect;
pub mod export;
pub mod id;
pub mod media;
pub mod project;
pub mod time;
pub mod timeline;
pub mod transition;

pub use effect::{ColorGrade, Effect, EffectKind};
pub use export::{ExportAudioCodec, ExportFormat, ExportSettings, ExportVideoCodec};
pub use id::{Id, IdGenerator};
pub use media::{AudioInfo, MediaAsset, MediaKind, MediaMetadata, VideoInfo};
pub use project::{Project, ProjectSettings};
pub use timeline::{Clip, ClipBounds, ClipSource, Timeline, Track, TrackKind};
pub use transition::{Transition, TransitionKind};

pub use time::{Time, TimeRange};

/// Re-export of the current project file format version.
///
/// Bump this when the on-disk schema changes in a breaking way.
/// `edt_storage` will refuse to load projects from incompatible major
/// versions and surface a clear error.
pub const PROJECT_FORMAT_VERSION: u32 = 1;
