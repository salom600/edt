//! Media probing — calls `ffprobe` to extract metadata about a file.

use crate::ffmpeg::{ensure_exists, parse_fraction, run_ffprobe, FfmpegError, FfprobeOutput};
use edt_core::media::{AudioInfo, MediaMetadata, VideoInfo};
use std::path::Path;

/// Result of probing a media file.
#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub metadata: MediaMetadata,
}

/// Probe a media file at `path` using `ffprobe`.
///
/// Returns an error if `ffprobe` cannot be found, if the file does not
/// exist, or if `ffprobe` exits non-zero (typically: corrupt or
/// unrecognized media).
pub fn probe(path: &Path) -> Result<ProbeResult, FfmpegError> {
    ensure_exists(path)?;
    let stdout = run_ffprobe(&[
        "-v",
        "error",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
        path.to_str().expect("path is utf-8"),
    ])?;
    let parsed: FfprobeOutput = serde_json::from_slice(&stdout)
        .map_err(|e| FfmpegError::ParseFailed(format!("JSON decode failed: {e}")))?;

    let metadata = build_metadata(parsed);
    Ok(ProbeResult { metadata })
}

fn build_metadata(parsed: FfprobeOutput) -> MediaMetadata {
    let mut video: Option<VideoInfo> = None;
    let mut audio: Option<AudioInfo> = None;

    if let Some(streams) = parsed.streams {
        for s in streams {
            match s.codec_type.as_str() {
                "video" if video.is_none() => {
                    // Prefer avg_frame_rate, fall back to r_frame_rate.
                    let fps = s
                        .avg_frame_rate
                        .as_deref()
                        .and_then(parse_fraction)
                        .or_else(|| s.r_frame_rate.as_deref().and_then(parse_fraction))
                        .unwrap_or(0.0);
                    video = Some(VideoInfo {
                        width: s.width.unwrap_or(0),
                        height: s.height.unwrap_or(0),
                        fps,
                        codec: s.codec_name.unwrap_or_default(),
                        pixel_format: s.pix_fmt.unwrap_or_default(),
                    });
                }
                "audio" if audio.is_none() => {
                    audio = Some(AudioInfo {
                        sample_rate: s.sample_rate.unwrap_or(0),
                        channels: s.channels.unwrap_or(0),
                        codec: s.codec_name.unwrap_or_default(),
                    });
                }
                _ => {}
            }
        }
    }

    let format = parsed.format;
    let duration = format
        .as_ref()
        .and_then(|f| f.duration.as_deref())
        .and_then(|s| s.parse::<f64>().ok());

    let bitrate = format
        .as_ref()
        .and_then(|f| f.bit_rate.as_deref())
        .and_then(|s| s.parse::<u64>().ok());

    let format_name = format
        .as_ref()
        .and_then(|f| f.format_name.clone())
        .unwrap_or_default();

    MediaMetadata {
        duration,
        video,
        audio,
        bitrate,
        format: format_name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffmpeg::{FfprobeFormat, FfprobeStream};
    use edt_core::media::MediaMetadata;

    fn build_with(streams: Vec<FfprobeStream>) -> MediaMetadata {
        build_metadata(FfprobeOutput {
            format: None,
            streams: Some(streams),
        })
    }

    fn build_with_format_and_streams(
        format: Option<FfprobeFormat>,
        streams: Vec<FfprobeStream>,
    ) -> MediaMetadata {
        build_metadata(FfprobeOutput {
            format,
            streams: Some(streams),
        })
    }

    #[test]
    fn empty_input_yields_unknown_kind() {
        let m = build_with(vec![]);
        assert_eq!(m.kind(), edt_core::media::MediaKind::Unknown);
    }

    #[test]
    fn video_stream_classifies_as_video() {
        let m = build_with_format_and_streams(
            Some(FfprobeFormat {
                duration: Some("10.0".into()),
                bit_rate: None,
                format_name: Some("mp4".into()),
            }),
            vec![FfprobeStream {
                codec_type: "video".into(),
                codec_name: Some("h264".into()),
                width: Some(1920),
                height: Some(1080),
                pix_fmt: Some("yuv420p".into()),
                sample_rate: None,
                channels: None,
                avg_frame_rate: Some("30/1".into()),
                r_frame_rate: Some("30/1".into()),
            }],
        );
        assert_eq!(m.kind(), edt_core::media::MediaKind::Video);
        let v = m.video.expect("video info");
        assert_eq!(v.width, 1920);
        assert!((v.fps - 30.0).abs() < 1e-9);
    }

    #[test]
    fn audio_only_classifies_as_audio() {
        let m = build_with(vec![FfprobeStream {
            codec_type: "audio".into(),
            codec_name: Some("aac".into()),
            width: None,
            height: None,
            pix_fmt: None,
            sample_rate: Some(48000),
            channels: Some(2),
            avg_frame_rate: None,
            r_frame_rate: None,
        }]);
        assert_eq!(m.kind(), edt_core::media::MediaKind::Audio);
    }
}
