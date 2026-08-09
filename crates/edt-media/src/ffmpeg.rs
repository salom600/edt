//! Locating and invoking `ffmpeg` / `ffprobe`.
//!
//! On all three target platforms we expect `ffmpeg` and `ffprobe` to be
//! on `PATH`. CI installs them via the platform's package manager
//! (`apt`, `brew`, and the [`bubbajoe/ffmpeg-action`](https://github.com/bubbajoe/ffmpeg-action)
//! GitHub Action for Windows). End users must install FFmpeg themselves;
//! the README documents this.

use anyhow::anyhow;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

/// All errors produced by media operations.
#[derive(Debug, Error)]
pub enum FfmpegError {
    #[error("ffmpeg binary not found on PATH. Install FFmpeg from https://ffmpeg.org/")]
    FfmpegNotFound,
    #[error("ffprobe binary not found on PATH. Install FFmpeg from https://ffmpeg.org/")]
    FfprobeNotFound,
    #[error("ffmpeg command failed (exit code {code:?}): {stderr}")]
    CommandFailed { code: Option<i32>, stderr: String },
    #[error("could not parse ffprobe output: {0}")]
    ParseFailed(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image decode error: {0}")]
    Image(#[from] image::ImageError),
    #[error("unsupported media: {0}")]
    Unsupported(String),
}

/// Resolved paths to ffmpeg + ffprobe binaries.
#[derive(Debug, Clone)]
pub struct FfmpegPaths {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

static PATHS: OnceCell<FfmpegPaths> = OnceCell::new();

/// Locate `ffmpeg` and `ffprobe` on PATH. The result is cached for the
/// lifetime of the process. Returns an error if either binary is missing.
pub fn find_ffmpeg() -> Result<&'static FfmpegPaths, FfmpegError> {
    if PATHS.get().is_none() {
        let paths = locate_paths()?;
        // set() returns Err if already set; we ignore that race.
        let _ = PATHS.set(paths);
    }
    Ok(PATHS.get().expect("ffmpeg paths cached"))
}

fn locate_paths() -> Result<FfmpegPaths, FfmpegError> {
    let ffmpeg = which("ffmpeg").ok_or(FfmpegError::FfmpegNotFound)?;
    let ffprobe = which("ffprobe").ok_or(FfmpegError::FfprobeNotFound)?;
    tracing::info!(?ffmpeg, ?ffprobe, "located ffmpeg binaries");
    Ok(FfmpegPaths { ffmpeg, ffprobe })
}

fn which(bin: &str) -> Option<PathBuf> {
    // We implement our own `which` to avoid pulling in the `which` crate
    // (lightweight, but every dependency counts in CI build time).
    let path_env = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_env) {
        let candidate = dir.join(bin);
        // On Windows, also try with .exe suffix.
        for suffix in &["", ".exe"] {
            let with_ext = {
                let mut c = candidate.clone();
                let mut name = c.file_name()?.to_owned();
                name.push(suffix);
                c.set_file_name(name);
                c
            };
            if let Ok(meta) = std::fs::metadata(&with_ext) {
                if meta.is_file() {
                    return Some(with_ext);
                }
            }
        }
        // Some platforms (notably Linux) may also have the binary at
        // /usr/bin/<bin> regardless of PATH.
        if let Ok(meta) = std::fs::metadata(&candidate) {
            if meta.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Run an ffmpeg command and check the exit status.
pub(crate) fn run_ffmpeg(args: &[&str]) -> Result<(), FfmpegError> {
    let paths = find_ffmpeg()?;
    let output = Command::new(&paths.ffmpeg).args(args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(FfmpegError::CommandFailed {
            code: output.status.code(),
            stderr,
        });
    }
    Ok(())
}

/// Run ffprobe and return its stdout as bytes.
pub(crate) fn run_ffprobe(args: &[&str]) -> Result<Vec<u8>, FfmpegError> {
    let paths = find_ffmpeg()?;
    let output = Command::new(&paths.ffprobe).args(args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(FfmpegError::CommandFailed {
            code: output.status.code(),
            stderr,
        });
    }
    Ok(output.stdout)
}

/// Subset of `ffprobe -show_format -show_streams -of json` output that we
/// actually consume. Fields not present in this struct are ignored.
#[derive(Debug, Deserialize)]
pub(crate) struct FfprobeOutput {
    pub format: Option<FfprobeFormat>,
    pub streams: Option<Vec<FfprobeStream>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FfprobeFormat {
    pub duration: Option<String>,
    pub bit_rate: Option<String>,
    pub format_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FfprobeStream {
    pub codec_type: String,
    pub codec_name: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub pix_fmt: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub avg_frame_rate: Option<String>,
    pub r_frame_rate: Option<String>,
}

/// Parse a fraction string like "30000/1001" into a float.
pub(crate) fn parse_fraction(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some((num, den)) = s.split_once('/') {
        let n: f64 = num.parse().ok()?;
        let d: f64 = den.parse().ok()?;
        if d == 0.0 {
            return None;
        }
        Some(n / d)
    } else {
        s.parse().ok()
    }
}

/// Run a function on a Path, returning a friendly error if the path doesn't exist.
pub(crate) fn ensure_exists(path: &Path) -> Result<(), FfmpegError> {
    if !path.exists() {
        return Err(FfmpegError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            anyhow!("file not found: {}", path.display()),
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fraction_handles_ntsc_framerates() {
        assert!((parse_fraction("30000/1001").unwrap() - 29.97002997).abs() < 1e-6);
        assert!((parse_fraction("25/1").unwrap() - 25.0).abs() < 1e-9);
        assert!((parse_fraction("60").unwrap() - 60.0).abs() < 1e-9);
        assert!(parse_fraction("0/0").is_none());
        assert!(parse_fraction("garbage").is_none());
    }
}
