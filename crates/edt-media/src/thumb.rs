//! Thumbnail generation for the media pool.

use crate::frame::extract_frame;
use crate::ffmpeg::FfmpegError;
use edt_core::media::MediaAsset;
use image::RgbaImage;
use std::path::Path;

/// Default thumbnail width. Height is derived from the asset's aspect
/// ratio. 240px is wide enough to look crisp in a typical media pool
/// while keeping memory usage low.
pub const THUMB_W: u32 = 240;

/// Generate a thumbnail for a media asset. For video assets, extracts a
/// frame at 10% into the duration. For images, decodes and scales the
/// entire image. For audio, returns `Ok(None)` (no thumbnail).
pub fn generate_thumbnail(asset: &MediaAsset) -> Result<Option<RgbaImage>, FfmpegError> {
    let kind = asset.kind();
    let path = asset.editing_path();
    match kind {
        edt_core::media::MediaKind::Video => {
            let dur = asset.duration().as_secs().max(0.0);
            let t = if dur > 0.0 { dur * 0.1 } else { 0.0 };
            let img = extract_frame(path, t, Some(THUMB_W))?;
            Ok(Some(img))
        }
        edt_core::media::MediaKind::Image => {
            // For images we decode the whole file then scale.
            let reader = image::ImageReader::open(path)?;
            let reader = reader.with_guessed_format()?;
            let img = reader.decode()?;
            let rgba = img.to_rgba8();
            let aspect = rgba.height() as f32 / rgba.width() as f32;
            let h = (THUMB_W as f32 * aspect).round() as u32;
            let h = h.max(1);
            let thumb = image::imageops::resize(&rgba, THUMB_W, h, image::imageops::FilterType::Triangle);
            Ok(Some(thumb))
        }
        edt_core::media::MediaKind::Audio => Ok(None),
        edt_core::media::MediaKind::Unknown => Ok(None),
    }
}

/// Save a thumbnail as a PNG file. The caller (typically the UI) caches
/// thumbnails in the app's cache directory keyed by asset id + mtime.
pub fn save_thumbnail(img: &RgbaImage, out: &Path) -> Result<(), FfmpegError> {
    img.save_with_format(out, image::ImageFormat::Png)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_width_constant() {
        assert_eq!(THUMB_W, 240);
    }
}
