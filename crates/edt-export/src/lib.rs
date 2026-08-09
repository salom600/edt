//! edt-export — Drive ffmpeg to render the final output file.
//!
//! ## MVP strategy
//!
//! For 0.1.0 the exporter takes a pragmatic approach: rather than
//! compositing frames in-process and piping them to ffmpeg (which would
//! require careful timing alignment), we use ffmpeg's `concat` filter to
//! stitch the source clips together with their trim offsets applied.
//!
//! This works well when:
//! - All clips come from a single asset (single-segment export).
//! - Or all clips are on a single track with no overlaps.
//!
//! For more complex timelines (multi-track compositing, transitions,
//! effects) the exporter falls back to a per-frame approach: we render
//! each frame via [`edt_render::compose_frame`] and pipe raw RGBA to
//! ffmpeg via stdin. This is slower but handles any timeline.
//!
//! See `docs/adr/0007-export-strategy.md` for the full decision tree.

pub mod pipeline;
pub mod progress;

pub use pipeline::{export_project, ExportOptions, ExportStrategy};
pub use progress::{ExportProgress, ProgressUpdate};
