//! edt-render — Frame composition from timeline state.
//!
//! Given a [`Project`] and a timeline time `t`, the render pipeline
//! figures out which clips are active, where they sit on the canvas, and
//! produces a single composited RGBA frame ready for display (preview) or
//! encoding (export).
//!
//! ## MVP scope
//!
//! For 0.1.0 the pipeline handles:
//! - Topmost-video-clip-wins compositing (no opacity blending across tracks).
//! - Audio mixing by simple summing across non-muted audio tracks.
//! - Identity color transform (color grade application is a roadmap item).
//!
//! Advanced features (cross-track opacity, transitions, GPU shaders,
//! LUTs) are documented in `docs/adr/0006-render-pipeline.md` and
//! tracked on the v0.2 roadmap.

pub mod audio;
pub mod compose;

pub use audio::{mix_audio, AudioMixOutput};
pub use compose::{compose_frame, ActiveClip, CompositeFrame};
