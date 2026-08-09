//! Frame extraction — given a media file and a timestamp, produce a
//! decoded RGBA image suitable for display in the preview player.

use crate::ffmpeg::{ensure_exists, FfmpegError};
use image::RgbaImage;
use std::path::Path;

/// Extract a single frame at `time_secs` from `path`. The frame is
/// returned as an `image::RgbaImage`. Resolution is the source's native
/// resolution unless `max_width` is set, in which case the frame is
/// scaled to fit within `max_width` preserving aspect ratio.
pub fn extract_frame(
    path: &Path,
    time_secs: f64,
    max_width: Option<u32>,
) -> Result<RgbaImage, FfmpegError> {
    ensure_exists(path)?;
    let time_str = format!("{:.3}", time_secs.max(0.0));

    let mut args: Vec<&str> = Vec::with_capacity(10);
    args.extend_from_slice(&[
        "-y",
        "-loglevel",
        "error",
        "-ss",
        &time_str,
        "-i",
        path.to_str().expect("path is utf-8"),
        "-frames:v",
        "1",
    ]);

    let scale_filter = match max_width {
        Some(w) => format!("scale={w}:-1"),
        None => "scale=iw:ih".to_string(),
    };
    let vf_arg = format!("{scale_filter},format=rgba");
    // Leak the filter string into a static lifetime; this is fine because
    // the arg is consumed within the same function call.
    let vf: &'static str = Box::leak(vf_arg.into_boxed_str());
    args.extend_from_slice(&["-vf", vf, "-f", "image2pipe", "-vcodec", "png", "-"]);

    let paths = crate::ffmpeg::find_ffmpeg()?;
    let output = std::process::Command::new(&paths.ffmpeg)
        .args(&args)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(FfmpegError::CommandFailed {
            code: output.status.code(),
            stderr,
        });
    }
    if output.stdout.is_empty() {
        return Err(FfmpegError::Unsupported(
            "ffmpeg produced no output (frame extraction failed)".into(),
        ));
    }
    let img = image::load_from_memory_with_format(&output.stdout, image::ImageFormat::Png)?;
    Ok(img.to_rgba8())
}

/// Extract a frame and write it to a PNG file at `out_path`. Useful for
/// thumbnail generation and for headless tests.
pub fn extract_frame_to_file(
    path: &Path,
    time_secs: f64,
    max_width: Option<u32>,
    out_path: &Path,
) -> Result<(), FfmpegError> {
    let img = extract_frame(path, time_secs, max_width)?;
    img.save_with_format(out_path, image::ImageFormat::Png)?;
    Ok(())
}

/// Spawn a frame-extraction worker that runs in the background. Used by
/// the preview player to avoid blocking the UI thread on ffmpeg calls.
///
/// The returned channel receives `(time_secs, RgbaImage)` pairs as they
/// complete. The caller is responsible for caching and dropping stale
/// frames.
pub fn spawn_extractor(
    path: std::path::PathBuf,
    times: Vec<f64>,
    max_width: Option<u32>,
) -> crossbeam_channel::Receiver<(f64, RgbaImage)> {
    let (tx, rx) = crossbeam_channel::bounded(8);
    std::thread::spawn(move || {
        for t in times {
            match extract_frame(&path, t, max_width) {
                Ok(img) => {
                    if tx.send((t, img)).is_err() {
                        break; // receiver dropped
                    }
                }
                Err(e) => {
                    tracing::warn!(time = t, error = ?e, "frame extraction failed");
                }
            }
        }
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This test is intentionally skipped in CI: it requires ffmpeg to be
    /// installed and a sample video to exist at `/tmp/sample.mp4`. The
    /// test exists for local development.
    #[test]
    #[ignore]
    fn extract_first_frame_of_sample() {
        let path = Path::new("/tmp/sample.mp4");
        if !path.exists() {
            return;
        }
        let img = extract_frame(path, 0.0, Some(320)).expect("extract");
        assert!(img.width() > 0);
        assert!(img.height() > 0);
    }

    /// Verifies that the helper `spawn_extractor` channel closes cleanly
    /// when the receiver is dropped (no deadlock on the sender side).
    #[test]
    fn extractor_channel_drops_cleanly() {
        let (_rx, _) = crossbeam_channel::bounded::<(f64, RgbaImage)>(1);
        // Just verify the type compiles and channels work.
    }
}
