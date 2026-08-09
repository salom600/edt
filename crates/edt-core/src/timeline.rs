//! Timeline model — tracks, clips, and the timeline container.

use crate::id::{Id, IdGenerator};
use crate::media::MediaAsset;
use crate::time::{Time, TimeRange};
use crate::transition::Transition;
use serde::{Deserialize, Serialize};

/// Distinguishes video and audio tracks. The timeline keeps video tracks
/// above audio tracks (Resolve/Premiere convention) — see [`Timeline::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Video,
    Audio,
}

/// A timeline track. Holds an ordered list of clips.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: Id,
    pub name: String,
    pub kind: TrackKind,
    /// True if the track's output is muted (video: hidden; audio: silent).
    #[serde(default)]
    pub muted: bool,
    /// True if the track is the only one audible/visible during preview.
    #[serde(default)]
    pub solo: bool,
    /// True if the track is locked against edits.
    #[serde(default)]
    pub locked: bool,
    /// 0.0..=1.0 — opacity for video, gain for audio.
    #[serde(default = "default_track_level")]
    pub level: f32,
    /// Clips, ordered by start time ascending.
    #[serde(default)]
    pub clips: Vec<Clip>,
}

fn default_track_level() -> f32 {
    1.0
}

impl Track {
    pub fn new(id: Id, name: impl Into<String>, kind: TrackKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            muted: false,
            solo: false,
            locked: false,
            level: 1.0,
            clips: Vec::new(),
        }
    }

    /// Returns the clip whose timeline range contains `t`, if any.
    pub fn clip_at(&self, t: Time) -> Option<&Clip> {
        self.clips.iter().find(|c| c.timeline_range().contains(t))
    }

    /// Returns the clip whose timeline range contains `t`, mutably.
    pub fn clip_at_mut(&mut self, t: Time) -> Option<&mut Clip> {
        self.clips
            .iter_mut()
            .find(|c| c.timeline_range().contains(t))
    }

    /// Total duration of the track (end time of its last clip).
    pub fn duration(&self) -> Time {
        self.clips
            .iter()
            .map(|c| c.timeline_range().end)
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(Time::ZERO)
    }

    /// Insert a clip, keeping clips sorted by start time.
    /// Does **not** detect overlaps — that is the editor's responsibility.
    pub fn insert_clip(&mut self, clip: Clip) {
        let pos = self
            .clips
            .iter()
            .position(|c| c.timeline_start.0 > clip.timeline_start.0)
            .unwrap_or(self.clips.len());
        self.clips.insert(pos, clip);
    }

    pub fn remove_clip(&mut self, id: Id) -> Option<Clip> {
        self.clips
            .iter()
            .position(|c| c.id == id)
            .map(|i| self.clips.remove(i))
    }
}

/// Which portion of the source asset this clip uses.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClipSource {
    pub asset_id: Id,
    /// Start time within the source asset, in seconds.
    pub source_start: Time,
    /// End time within the source asset, in seconds.
    pub source_end: Time,
}

impl ClipSource {
    pub fn duration(&self) -> Time {
        Time(self.source_end.0 - self.source_start.0)
    }

    /// Default source range covering the entire asset.
    pub fn from_asset(asset: &MediaAsset) -> Self {
        let dur = asset.duration();
        Self {
            asset_id: asset.id,
            source_start: Time::ZERO,
            source_end: dur,
        }
    }
}

/// Playback speed multiplier. 1.0 = normal, 2.0 = 2x fast, 0.5 = half speed.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClipSpeed(pub f64);

impl Default for ClipSpeed {
    fn default() -> Self {
        ClipSpeed(1.0)
    }
}

/// A clip on the timeline — a placed, possibly trimmed, slice of a media asset.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clip {
    pub id: Id,
    pub name: String,
    pub source: ClipSource,
    /// Where the clip starts on the project timeline, in seconds.
    pub timeline_start: Time,
    /// Where the clip ends on the project timeline, in seconds.
    /// For speed=1.0 this equals `timeline_start + source.duration()`.
    /// For speed!=1.0 it equals `timeline_start + source.duration() / speed`.
    pub timeline_end: Time,
    #[serde(default)]
    pub speed: ClipSpeed,
    /// 0.0..=1.0 — opacity for video, gain for audio.
    #[serde(default = "default_track_level")]
    pub level: f32,
    /// True if clip is muted during preview/export.
    #[serde(default)]
    pub muted: bool,
    /// Optional label color (0..7).
    #[serde(default)]
    pub label: u8,
}

impl Clip {
    pub fn new(id: Id, name: impl Into<String>, source: ClipSource, timeline_start: Time) -> Self {
        let timeline_end = timeline_start + source.duration();
        Self {
            id,
            name: name.into(),
            source,
            timeline_start,
            timeline_end,
            speed: ClipSpeed::default(),
            level: 1.0,
            muted: false,
            label: 0,
        }
    }

    pub fn timeline_range(&self) -> TimeRange {
        TimeRange::new(self.timeline_start, self.timeline_end)
    }

    pub fn timeline_duration(&self) -> Time {
        Time(self.timeline_end.0 - self.timeline_start.0)
    }

    pub fn source_duration(&self) -> Time {
        self.source.duration()
    }

    /// Map a timeline time to a source time. Returns `None` if `t` is
    /// outside the clip's timeline range.
    pub fn timeline_to_source(&self, t: Time) -> Option<Time> {
        if !self.timeline_range().contains(t) {
            return None;
        }
        let offset = t.0 - self.timeline_start.0;
        let src_offset = offset * self.speed.0;
        Some(Time(self.source.source_start.0 + src_offset))
    }

    /// Adjust the clip's left edge. `new_start` is the new timeline start.
    /// The source start is moved correspondingly to keep the right edge
    /// visually fixed. Caller is responsible for clamping.
    pub fn trim_left(&mut self, new_start: Time) {
        let delta_timeline = new_start.0 - self.timeline_start.0;
        let delta_source = delta_timeline * self.speed.0;
        self.timeline_start = new_start;
        self.source.source_start = Time(self.source.source_start.0 + delta_source);
    }

    /// Adjust the clip's right edge.
    pub fn trim_right(&mut self, new_end: Time) {
        self.timeline_end = new_end;
        let new_dur_timeline = new_end.0 - self.timeline_start.0;
        let new_dur_source = new_dur_timeline * self.speed.0;
        self.source.source_end = Time(self.source.source_start.0 + new_dur_source);
    }

    /// Split the clip at timeline time `t`. Returns a new clip for the
    /// right half. The original clip becomes the left half.
    /// `t` must be strictly inside the clip's timeline range.
    pub fn split(&mut self, new_id: Id, t: Time) -> Option<Clip> {
        if !self.timeline_range().contains(t) {
            return None;
        }
        let src_t = self.timeline_to_source(t)?;
        let mut right = self.clone();
        right.id = new_id;
        right.timeline_start = t;
        right.source.source_start = src_t;
        self.timeline_end = t;
        self.source.source_end = src_t;
        Some(right)
    }
}

/// Helper newtype for callers that want a typed bounds tuple.
#[derive(Clone, Copy, Debug)]
pub struct ClipBounds {
    pub timeline: TimeRange,
    pub source: TimeRange,
}

impl From<&Clip> for ClipBounds {
    fn from(c: &Clip) -> Self {
        ClipBounds {
            timeline: c.timeline_range(),
            source: TimeRange::new(c.source.source_start, c.source.source_end),
        }
    }
}

// ---------------------------------------------------------------------------
// Timeline container
// ---------------------------------------------------------------------------

/// The timeline owns all tracks (video + audio) plus transitions.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Timeline {
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub transitions: Vec<Transition>,
}

impl Timeline {
    /// Create a timeline pre-seeded with two video and two audio tracks.
    /// This matches the UX of most NLEs (Resolve, Premiere) where the
    /// user starts editing immediately without having to add tracks.
    pub fn with_default_tracks(gen: &IdGenerator) -> Self {
        let mut t = Self::default();
        t.add_track(Track::new(gen.next(), "V1", TrackKind::Video));
        t.add_track(Track::new(gen.next(), "V2", TrackKind::Video));
        t.add_track(Track::new(gen.next(), "A1", TrackKind::Audio));
        t.add_track(Track::new(gen.next(), "A2", TrackKind::Audio));
        t
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn track(&self, id: Id) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }

    pub fn track_mut(&mut self, id: Id) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.id == id)
    }

    /// Iterate tracks top-to-bottom (video first, then audio), matching
    /// how the UI renders them. Video tracks render top-down so that the
    /// topmost track in the list visually appears on top of the canvas.
    pub fn tracks_top_to_bottom(&self) -> impl Iterator<Item = &Track> {
        let (video, audio): (Vec<_>, Vec<_>) =
            self.tracks.iter().partition(|t| t.kind == TrackKind::Video);
        video.into_iter().chain(audio)
    }

    /// Find the clip with the given id across all tracks.
    pub fn clip(&self, id: Id) -> Option<(&Track, &Clip)> {
        self.tracks
            .iter()
            .find_map(|t| t.clips.iter().find(|c| c.id == id).map(|c| (t, c)))
    }

    /// Find the track that owns a clip with the given id.
    pub fn track_of_clip(&self, clip_id: Id) -> Option<Id> {
        self.tracks
            .iter()
            .find(|t| t.clips.iter().any(|c| c.id == clip_id))
            .map(|t| t.id)
    }

    /// End time of the last clip across all tracks.
    pub fn duration(&self) -> Time {
        self.tracks
            .iter()
            .map(|t| t.duration())
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(Time::ZERO)
    }

    /// Collect all clips active at time `t` across all tracks, in top-to-bottom order.
    pub fn active_clips_at(&self, t: Time) -> Vec<(&Track, &Clip)> {
        self.tracks_top_to_bottom()
            .filter_map(|track| track.clip_at(t).map(|c| (track, c)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_clip(start: f64, end: f64, t_start: f64) -> Clip {
        Clip::new(
            Id(1),
            "c",
            ClipSource {
                asset_id: Id(99),
                source_start: Time(start),
                source_end: Time(end),
            },
            Time(t_start),
        )
    }

    #[test]
    fn clip_timeline_range() {
        let c = make_clip(0.0, 10.0, 5.0);
        let r = c.timeline_range();
        assert_eq!(r.start.0, 5.0);
        assert_eq!(r.end.0, 15.0);
    }

    #[test]
    fn split_clip_keeps_total_length() {
        let mut c = make_clip(0.0, 10.0, 5.0);
        let original_dur = c.timeline_duration().0;
        let right = c.split(Id(2), Time(10.0)).expect("split");
        assert_eq!(c.timeline_range().end.0, 10.0);
        assert_eq!(right.timeline_range().start.0, 10.0);
        assert_eq!(right.timeline_range().end.0, 15.0);
        let total = c.timeline_duration().0 + right.timeline_duration().0;
        assert!((total - original_dur).abs() < 1e-9);
    }

    #[test]
    fn trim_left_preserves_right_edge() {
        let mut c = make_clip(0.0, 10.0, 5.0);
        let right_before = c.timeline_end.0;
        c.trim_left(Time(7.0));
        assert_eq!(c.timeline_start.0, 7.0);
        assert_eq!(c.timeline_end.0, right_before);
        assert_eq!(c.source.source_start.0, 2.0);
    }

    #[test]
    fn timeline_to_source_respects_speed() {
        let mut c = make_clip(0.0, 10.0, 0.0);
        c.speed = ClipSpeed(2.0);
        c.timeline_end = Time(5.0);
        let src_t = c.timeline_to_source(Time(2.5)).unwrap();
        assert!((src_t.0 - 5.0).abs() < 1e-9);
    }

    #[test]
    fn track_insert_keeps_sorted() {
        let mut t = Track::new(Id(1), "v1", TrackKind::Video);
        t.insert_clip(make_clip(0.0, 5.0, 10.0));
        t.insert_clip(make_clip(0.0, 5.0, 0.0));
        t.insert_clip(make_clip(0.0, 5.0, 5.0));
        let starts: Vec<f64> = t.clips.iter().map(|c| c.timeline_start.0).collect();
        assert_eq!(starts, vec![0.0, 5.0, 10.0]);
    }

    #[test]
    fn default_timeline_has_four_tracks() {
        let gen = IdGenerator::new();
        let t = Timeline::with_default_tracks(&gen);
        assert_eq!(t.tracks.len(), 4);
        assert_eq!(t.tracks[0].name, "V1");
    }

    #[test]
    fn duration_is_max_of_track_durations() {
        let gen = IdGenerator::new();
        let mut t = Timeline::with_default_tracks(&gen);
        let clip = Clip::new(
            gen.next(),
            "c",
            ClipSource {
                asset_id: gen.next(),
                source_start: Time::ZERO,
                source_end: Time(5.0),
            },
            Time(10.0),
        );
        t.tracks[0].insert_clip(clip);
        assert_eq!(t.duration().0, 15.0);
    }

    #[test]
    fn active_clips_at_returns_top_to_bottom() {
        let gen = IdGenerator::new();
        let mut t = Timeline::with_default_tracks(&gen);
        let c1 = Clip::new(
            gen.next(),
            "v1",
            ClipSource {
                asset_id: gen.next(),
                source_start: Time::ZERO,
                source_end: Time(10.0),
            },
            Time::ZERO,
        );
        let c2 = Clip::new(
            gen.next(),
            "v2",
            ClipSource {
                asset_id: gen.next(),
                source_start: Time::ZERO,
                source_end: Time(10.0),
            },
            Time::ZERO,
        );
        t.tracks[0].insert_clip(c1);
        t.tracks[1].insert_clip(c2);
        let active = t.active_clips_at(Time(5.0));
        assert_eq!(active.len(), 2);
        assert_eq!(active[0].1.name, "v1");
        assert_eq!(active[1].1.name, "v2");
    }
}
