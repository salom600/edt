//! Frame composition.

use edt_core::media::MediaKind;
use edt_core::project::Project;
use edt_core::time::Time;
use edt_core::timeline::Clip;
use image::RgbaImage;

/// Description of a clip that is active at a given timeline time, with
/// the resolved source time and an optional pre-decoded frame.
#[derive(Debug)]
pub struct ActiveClip<'a> {
    pub clip: &'a Clip,
    pub track_name: &'a str,
    pub source_time: Time,
    /// Decoded RGBA frame for this clip, if available. `None` if the
    /// caller has not yet extracted it (e.g. the preview player caches
    /// frames asynchronously).
    pub frame: Option<RgbaImage>,
}

/// The result of compositing one timeline frame.
#[derive(Debug)]
pub struct CompositeFrame {
    /// The final composited RGBA image at project resolution.
    pub image: RgbaImage,
    /// The timeline time this frame represents.
    pub time: Time,
}

/// Compose a single timeline frame at time `t`.
///
/// `clip_frames` is a closure that, given an active clip, returns its
/// decoded RGBA frame (or `None` if not yet available). This decouples
/// the compositor from the actual decode mechanism — the caller decides
/// whether to call ffmpeg, hit a cache, or return a placeholder.
///
/// For MVP: the topmost non-muted video track's clip wins. If no clip is
/// active, the canvas is filled with the project's background color.
pub fn compose_frame<F>(project: &Project, t: Time, mut clip_frames: F) -> CompositeFrame
where
    F: FnMut(&Clip) -> Option<RgbaImage>,
{
    let w = project.settings.width.max(1);
    let h = project.settings.height.max(1);
    let mut canvas = RgbaImage::from_pixel(w, h, background_pixel(project));

    // Walk tracks top-to-bottom. For video tracks, the *topmost* track in
    // the list should appear on top of the canvas. We iterate in
    // top-to-bottom order and let later writes overwrite earlier ones,
    // which gives us the correct stacking.
    //
    // Note: This means audio tracks contribute nothing to the canvas,
    // which is exactly what we want.
    for (track, clip) in project.timeline.active_clips_at(t) {
        if track.kind != MediaKind::Video.cast_to_track_kind() {
            continue;
        }
        if track.muted || clip.muted {
            continue;
        }
        if let Some(frame) = clip_frames(clip) {
            // Scale frame to fit canvas (contain mode).
            blit_fit(&mut canvas, &frame);
        }
    }

    CompositeFrame {
        image: canvas,
        time: t,
    }
}

/// Blit `src` onto `dst` such that `src` fits within `dst` while
/// preserving aspect ratio (letterbox). For MVP this is sufficient;
/// scale/position transforms live in the inspector as effect parameters
/// but are not yet wired to the compositor.
fn blit_fit(dst: &mut RgbaImage, src: &RgbaImage) {
    let dw = dst.width() as f32;
    let dh = dst.height() as f32;
    let sw = src.width() as f32;
    let sh = src.height() as f32;
    if sw == 0.0 || sh == 0.0 {
        return;
    }
    let scale = (dw / sw).min(dh / sh);
    let new_w = (sw * scale).round() as u32;
    let new_h = (sh * scale).round() as u32;
    let new_w = new_w.max(1).min(dst.width());
    let new_h = new_h.max(1).min(dst.height());
    let scaled = image::imageops::resize(src, new_w, new_h, image::imageops::FilterType::Triangle);
    let off_x = (dw as u32 - new_w) / 2;
    let off_y = (dh as u32 - new_h) / 2;
    image::imageops::overlay(dst, &scaled, off_x as i64, off_y as i64);
}

fn background_pixel(project: &Project) -> image::Rgba<u8> {
    let [r, g, b] = project.settings.background;
    image::Rgba([r, g, b, 255])
}

// Helpers to bridge MediaKind and TrackKind without circular deps.
trait TrackKindCast {
    fn cast_to_track_kind(self) -> edt_core::timeline::TrackKind;
}

impl TrackKindCast for MediaKind {
    fn cast_to_track_kind(self) -> edt_core::timeline::TrackKind {
        match self {
            MediaKind::Video | MediaKind::Image => edt_core::timeline::TrackKind::Video,
            _ => edt_core::timeline::TrackKind::Audio,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edt_core::id::IdGenerator;
    use edt_core::project::Project;
    use edt_core::timeline::{Clip, ClipSource, TrackKind};

    #[test]
    fn empty_timeline_yields_background_color() {
        let (mut p, _) = Project::new();
        p.settings.background = [10, 20, 30];
        let frame = compose_frame(&p, Time::ZERO, |_| None);
        assert_eq!(frame.image.width(), p.settings.width);
        let px = frame.image.get_pixel(0, 0);
        assert_eq!(px.0, [10, 20, 30, 255]);
    }

    #[test]
    fn compose_with_one_clip_blits_into_canvas() {
        let gen = IdGenerator::new();
        let (mut p, _) = Project::new();
        let asset_id = gen.next();
        // Add a 10s clip on V1 starting at t=0.
        let clip = Clip::new(
            gen.next(),
            "c",
            ClipSource {
                asset_id,
                source_start: Time::ZERO,
                source_end: Time(10.0),
            },
            Time::ZERO,
        );
        p.timeline.tracks[0].insert_clip(clip);
        // Provide a tiny 2x2 red frame for any clip.
        let frame = compose_frame(&p, Time(5.0), |_| {
            Some(RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255])))
        });
        // Some pixel should now be red (blit fit centered, so for a 1920x1080
        // canvas the 2x2 image gets upscaled to fill, all pixels are red).
        let non_bg = frame.image.pixels().any(|p| p.0 == [255, 0, 0, 255]);
        assert!(non_bg, "expected at least one red pixel after blit");
    }

    #[test]
    fn muted_track_skipped() {
        let gen = IdGenerator::new();
        let (mut p, _) = Project::new();
        let asset_id = gen.next();
        let clip = Clip::new(
            gen.next(),
            "c",
            ClipSource {
                asset_id,
                source_start: Time::ZERO,
                source_end: Time(10.0),
            },
            Time::ZERO,
        );
        p.timeline.tracks[0].muted = true;
        p.timeline.tracks[0].insert_clip(clip);
        p.settings.background = [5, 5, 5];
        let frame = compose_frame(&p, Time(5.0), |_| {
            Some(RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255])))
        });
        // No red pixels because the track is muted.
        let any_red = frame.image.pixels().any(|p| p.0 == [255, 0, 0, 255]);
        assert!(!any_red);
        // First pixel should be the background.
        assert_eq!(frame.image.get_pixel(0, 0).0, [5, 5, 5, 255]);
        let _ = TrackKind::Video; // silence unused import warning if any
    }
}
