//! Media asset model — describes an imported file on disk plus its
//! probed metadata (resolution, duration, codec, etc.).

use crate::id::Id;
use crate::time::Time;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Broad classification of a media asset. Drives UI grouping and which
/// tracks the asset can be dropped onto.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Video,
    Audio,
    Image,
    Unknown,
}

impl MediaKind {
    pub fn is_video(self) -> bool {
        matches!(self, MediaKind::Video)
    }
    pub fn is_audio(self) -> bool {
        matches!(self, MediaKind::Audio)
    }
}

/// Probed metadata for a video stream.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VideoInfo {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Average frames per second (e.g. 25.0, 29.97, 60.0).
    pub fps: f64,
    /// Codec reported by ffprobe (e.g. "h264", "hevc", "av1").
    pub codec: String,
    /// Pixel format (e.g. "yuv420p").
    pub pixel_format: String,
}

/// Probed metadata for an audio stream.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AudioInfo {
    /// Sample rate in Hz (e.g. 48000).
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u32,
    /// Codec reported by ffprobe (e.g. "aac", "pcm_s16le").
    pub codec: String,
}

/// Aggregate metadata for a media asset, as returned by [`edt_media::probe`].
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MediaMetadata {
    /// Total duration of the asset in seconds.
    /// For images this is `None` (still images have no intrinsic duration).
    pub duration: Option<f64>,
    /// Video stream info, if any.
    pub video: Option<VideoInfo>,
    /// Audio stream info, if any.
    pub audio: Option<AudioInfo>,
    /// Best-effort overall bitrate in bits/second.
    pub bitrate: Option<u64>,
    /// Container format reported by ffprobe (e.g. "mov,mp4,m4a,3gp,3g2,mj2").
    pub format: String,
}

impl MediaMetadata {
    pub fn duration_or_default(&self) -> f64 {
        self.duration.unwrap_or(0.0)
    }

    pub fn kind(&self) -> MediaKind {
        match (self.video.is_some(), self.audio.is_some(), self.duration) {
            // No video stream and no audio stream but has a duration is
            // a container we couldn't parse streams out of.
            (false, false, Some(_)) => MediaKind::Unknown,
            (false, false, None) => MediaKind::Unknown,
            // Has video stream + duration -> video.
            // Has video stream but no duration -> still image (single frame).
            (true, _, Some(_)) => MediaKind::Video,
            (true, _, None) => MediaKind::Image,
            // Audio-only stream -> audio.
            (false, true, _) => MediaKind::Audio,
        }
    }
}

/// An imported media asset referenced by the project.
///
/// The asset stores an absolute path (resolved at save time). If the file
/// is missing on load, the UI will mark it as `offline` and the user can
/// re-link it. We intentionally do **not** embed media bytes in the
/// project file — videos are too large.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: Id,
    /// Display name shown in the media pool (typically the file stem).
    pub name: String,
    /// Absolute path to the source file on disk.
    pub path: PathBuf,
    /// Probed metadata. May be `None` if the asset was added without
    /// probing (e.g. offline at import time).
    #[serde(default)]
    pub metadata: Option<MediaMetadata>,
    /// User-facing color label (0..7), matches Resolve-style labels.
    #[serde(default)]
    pub label: u8,
    /// True if the file was missing on last load. Cleared on re-link.
    #[serde(default)]
    pub offline: bool,
    /// Optional path to a proxy (transcoded low-bitrate copy) used for
    /// editing on slow machines. The original `path` is used at export.
    #[serde(default)]
    pub proxy_path: Option<PathBuf>,
}

impl MediaAsset {
    pub fn kind(&self) -> MediaKind {
        self.metadata
            .as_ref()
            .map(|m| m.kind())
            .unwrap_or(MediaKind::Unknown)
    }

    pub fn duration(&self) -> Time {
        Time::from_secs(
            self.metadata
                .as_ref()
                .and_then(|m| m.duration)
                .unwrap_or(0.0),
        )
    }

    pub fn video_info(&self) -> Option<&VideoInfo> {
        self.metadata.as_ref().and_then(|m| m.video.as_ref())
    }

    pub fn audio_info(&self) -> Option<&AudioInfo> {
        self.metadata.as_ref().and_then(|m| m.audio.as_ref())
    }

    /// Effective path used for editing operations (proxy if set, else original).
    pub fn editing_path(&self) -> &std::path::Path {
        self.proxy_path
            .as_deref()
            .unwrap_or_else(|| self.path.as_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_video_asset() -> MediaAsset {
        MediaAsset {
            id: Id(1),
            name: "clip.mp4".into(),
            path: "/tmp/clip.mp4".into(),
            metadata: Some(MediaMetadata {
                duration: Some(10.0),
                video: Some(VideoInfo {
                    width: 1920,
                    height: 1080,
                    fps: 25.0,
                    codec: "h264".into(),
                    pixel_format: "yuv420p".into(),
                }),
                audio: Some(AudioInfo {
                    sample_rate: 48000,
                    channels: 2,
                    codec: "aac".into(),
                }),
                bitrate: Some(5_000_000),
                format: "mov,mp4,m4a,3gp,3g2,mj2".into(),
            }),
            label: 0,
            offline: false,
            proxy_path: None,
        }
    }

    #[test]
    fn asset_kind_is_video() {
        assert_eq!(fake_video_asset().kind(), MediaKind::Video);
    }

    #[test]
    fn asset_duration_uses_metadata() {
        assert_eq!(fake_video_asset().duration().as_secs(), 10.0);
    }

    #[test]
    fn image_asset_has_zero_duration() {
        let mut a = fake_video_asset();
        a.metadata.as_mut().unwrap().duration = None;
        a.metadata.as_mut().unwrap().audio = None;
        assert_eq!(a.kind(), MediaKind::Image);
    }
}
