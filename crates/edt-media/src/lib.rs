//! edt-media — Media I/O for edt.
//!
//! All media operations are performed by shelling out to the system
//! `ffmpeg`/`ffprobe` binaries. This avoids the pain of linking libav*
//! development packages on every target platform (especially Windows and
//! macOS, where they are not available from system package managers).
//!
//! ## Why shell out instead of using `ffmpeg-next`?
//!
//! The `ffmpeg-next` Rust crate binds to libavcodec/libavformat/libavutil
//! via FFI. Linking it requires:
//! - On Linux: `apt install libavcodec-dev libavformat-dev libavutil-dev libswscale-dev libswresample-dev`
//! - On macOS: `brew install ffmpeg` (keg-only, requires explicit PKG_CONFIG_PATH)
//! - On Windows: downloading pre-built FFmpeg shared libraries and setting
//!   `FFMPEG_DIR` (not available from any system package manager)
//!
//! Each of these is a CI configuration landmine. Shelling out to a
//! pre-installed `ffmpeg` binary is dramatically more reliable, at the
//! cost of:
//! - Process-spawn overhead per frame extraction (mitigated by batching
//!   for export via `concat` filters).
//! - No in-process decode of arbitrary frame formats (we use PNG output
//!   from ffmpeg for frame extraction, which the `image` crate reads).
//!
//! For an MVP this trade-off is clearly worth it. See
//! `docs/adr/0002-media-backend.md` for the full rationale.

pub mod ffmpeg;
pub mod frame;
pub mod probe;
pub mod thumb;

pub use ffmpeg::{find_ffmpeg, FfmpegError, FfmpegPaths};
pub use frame::extract_frame;
pub use probe::{probe, ProbeResult};
pub use thumb::generate_thumbnail;
