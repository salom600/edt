//! Export settings — what the final render looks like.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Mp4,
    Mkv,
    Mov,
    Webm,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Mp4 => "mp4",
            ExportFormat::Mkv => "mkv",
            ExportFormat::Mov => "mov",
            ExportFormat::Webm => "webm",
        }
    }

    pub fn ffmpeg_muxer(self) -> &'static str {
        match self {
            ExportFormat::Mp4 => "mp4",
            ExportFormat::Mkv => "matroska",
            ExportFormat::Mov => "mov",
            ExportFormat::Webm => "webm",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportVideoCodec {
    H264,
    H265,
    Av1,
    Vp9,
    Prores,
}

impl ExportVideoCodec {
    pub fn ffmpeg_encoder(self) -> &'static str {
        match self {
            ExportVideoCodec::H264 => "libx264",
            ExportVideoCodec::H265 => "libx265",
            ExportVideoCodec::Av1 => "libsvtav1",
            ExportVideoCodec::Vp9 => "libvpx-vp9",
            ExportVideoCodec::Prores => "prores_ks",
        }
    }

    pub fn pix_fmt(self) -> &'static str {
        match self {
            ExportVideoCodec::Prores => "yuv422p10le",
            _ => "yuv420p",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportAudioCodec {
    Aac,
    Opus,
    PcmS16le,
}

impl ExportAudioCodec {
    pub fn ffmpeg_encoder(self) -> &'static str {
        match self {
            ExportAudioCodec::Aac => "aac",
            ExportAudioCodec::Opus => "libopus",
            ExportAudioCodec::PcmS16le => "pcm_s16le",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportSettings {
    pub format: ExportFormat,
    pub video_codec: ExportVideoCodec,
    pub audio_codec: ExportAudioCodec,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    /// Target video bitrate in bits/second. 0 means use CRF.
    pub video_bitrate: u64,
    /// CRF value for libx264/libx265 (0..=51, lower = higher quality).
    /// Used when `video_bitrate == 0`.
    pub crf: u32,
    /// Audio bitrate in bits/second.
    pub audio_bitrate: u64,
    /// Audio sample rate in Hz.
    pub audio_sample_rate: u32,
    /// Number of audio channels.
    pub audio_channels: u32,
    /// Whether to use hardware acceleration if available.
    #[serde(default)]
    pub hardware_accel: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            format: ExportFormat::Mp4,
            video_codec: ExportVideoCodec::H264,
            audio_codec: ExportAudioCodec::Aac,
            width: 1920,
            height: 1080,
            fps: 30.0,
            video_bitrate: 0,
            crf: 20,
            audio_bitrate: 192_000,
            audio_sample_rate: 48_000,
            audio_channels: 2,
            hardware_accel: false,
        }
    }
}

impl ExportSettings {
    /// Common preset: 1080p H.264 MP4.
    pub fn preset_1080p_h264() -> Self {
        Self::default()
    }

    /// Common preset: 4K H.265 MP4.
    pub fn preset_4k_h265() -> Self {
        Self {
            width: 3840,
            height: 2160,
            fps: 30.0,
            video_codec: ExportVideoCodec::H265,
            crf: 22,
            ..Self::default()
        }
    }

    /// Common preset: 720p H.264 MP4 (good for web previews).
    pub fn preset_720p_h264() -> Self {
        Self {
            width: 1280,
            height: 720,
            fps: 30.0,
            crf: 23,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_defaults_are_sane() {
        let p = ExportSettings::default();
        assert_eq!(p.width, 1920);
        assert_eq!(p.height, 1080);
        assert_eq!(p.format, ExportFormat::Mp4);
    }

    #[test]
    fn codec_to_ffmpeg_mapping() {
        assert_eq!(ExportVideoCodec::H264.ffmpeg_encoder(), "libx264");
        assert_eq!(ExportVideoCodec::H265.ffmpeg_encoder(), "libx265");
        assert_eq!(ExportAudioCodec::Aac.ffmpeg_encoder(), "aac");
    }
}
