//! Audio mixing — combine active audio clips into a single stereo buffer.

use edt_core::media::MediaKind;
use edt_core::project::Project;
use edt_core::time::Time;
use edt_core::timeline::{Clip, TrackKind};

/// A planar stereo audio buffer (left and right channels separate).
/// Sample values are f32 in [-1.0, 1.0].
#[derive(Debug, Clone)]
pub struct AudioMixOutput {
    pub sample_rate: u32,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

impl AudioMixOutput {
    pub fn silence(sample_rate: u32, frames: usize) -> Self {
        Self {
            sample_rate,
            left: vec![0.0; frames],
            right: vec![0.0; frames],
        }
    }

    pub fn frames(&self) -> usize {
        self.left.len()
    }
}

/// Mix all active audio clips at time `t` into a stereo buffer of
/// `frames` samples.
///
/// `clip_samples` is a closure that, given an active audio clip, returns
/// its interleaved-or-planar stereo samples at the project's sample rate.
/// Returning `None` for a clip is fine — it just contributes silence.
///
/// For MVP, mixing is a simple sum with per-track and per-clip level
/// gains applied. No limiter is on the master bus, so heavy clipping is
/// possible — the export pipeline applies a soft clip in the ffmpeg
/// filter chain.
pub fn mix_audio<F>(
    project: &Project,
    t: Time,
    frames: usize,
    mut clip_samples: F,
) -> AudioMixOutput
where
    F: FnMut(&Clip) -> Option<(Vec<f32>, Vec<f32>)>,
{
    let sr = project.settings.audio_sample_rate;
    let mut out = AudioMixOutput::silence(sr, frames);

    for (track, clip) in project.timeline.active_clips_at(t) {
        if track.kind != TrackKind::Audio {
            continue;
        }
        if track.muted || clip.muted {
            continue;
        }
        if let Some((mut left, mut right)) = clip_samples(clip) {
            // If the source is mono, replicate to both channels.
            if left.len() != right.len() {
                if right.is_empty() {
                    right = left.clone();
                } else if left.is_empty() {
                    left = right.clone();
                } else {
                    // Length mismatch — truncate to shorter.
                    let n = left.len().min(right.len());
                    left.truncate(n);
                    right.truncate(n);
                }
            }
            let track_level = track.level;
            let clip_level = clip.level;
            let gain = track_level * clip_level;
            let n = left.len().min(out.frames());
            for i in 0..n {
                out.left[i] += left[i] * gain;
                out.right[i] += right[i] * gain;
            }
        }
    }

    // Suppress unused import warning when MediaKind::Video branch never fires.
    let _ = MediaKind::Video;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use edt_core::id::IdGenerator;
    use edt_core::project::Project;
    use edt_core::time::Time;
    use edt_core::timeline::{Clip, ClipSource};

    #[test]
    fn silence_returns_zero_buffer() {
        let out = AudioMixOutput::silence(48000, 100);
        assert_eq!(out.frames(), 100);
        assert!(out.left.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn single_audio_clip_is_summed_into_output() {
        let gen = IdGenerator::new();
        let (mut p, _) = Project::new();
        let asset_id = gen.next();
        let clip = Clip::new(
            gen.next(),
            "a",
            ClipSource {
                asset_id,
                source_start: Time::ZERO,
                source_end: Time(10.0),
            },
            Time::ZERO,
        );
        // Insert on A1 (index 2 in the default timeline).
        p.timeline.tracks[2].insert_clip(clip);
        let out = mix_audio(&p, Time(1.0), 100, |_| {
            Some((vec![0.5; 100], vec![0.5; 100]))
        });
        assert!(out.left.iter().all(|&s| (s - 0.5).abs() < 1e-6));
        assert!(out.right.iter().all(|&s| (s - 0.5).abs() < 1e-6));
    }

    #[test]
    fn muted_audio_track_silenced() {
        let gen = IdGenerator::new();
        let (mut p, _) = Project::new();
        let asset_id = gen.next();
        let clip = Clip::new(
            gen.next(),
            "a",
            ClipSource {
                asset_id,
                source_start: Time::ZERO,
                source_end: Time(10.0),
            },
            Time::ZERO,
        );
        p.timeline.tracks[2].muted = true;
        p.timeline.tracks[2].insert_clip(clip);
        let out = mix_audio(&p, Time(1.0), 100, |_| {
            Some((vec![0.5; 100], vec![0.5; 100]))
        });
        assert!(out.left.iter().all(|&s| s == 0.0));
    }
}
