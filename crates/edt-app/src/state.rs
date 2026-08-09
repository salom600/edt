//! Editor state — the model the UI mutates.
//!
//! This is a thin wrapper around [`edt_core::Project`] plus UI-only
//! state (current selection, playback position, undo stack). The actual
//! timeline data lives in the project; this struct adds the bits that
//! never need to be persisted.

use edt_core::id::Id;
use edt_core::media::MediaAsset;
use edt_core::project::Project;
use edt_core::time::Time;
use edt_core::timeline::{Clip, TrackKind};
use parking_lot::RwLock;
use std::sync::Arc;

/// A timeline selection. Either nothing, a clip, or a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Selection {
    #[default]
    None,
    Clip(Id),
    Track(Id),
}

/// Playback state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayState {
    #[default]
    Paused,
    Playing,
}

/// The full editor state, wrapped in a `RwLock` so background jobs can
/// read it without blocking the UI thread.
#[derive(Default)]
pub struct EditorState {
    pub inner: RwLock<EditorStateInner>,
}

/// Inner mutable state.
#[derive(Clone, Debug, Default)]
pub struct EditorStateInner {
    pub project: Project,
    pub id_gen_seq: u64,
    pub selection: Selection,
    pub playhead: Time,
    pub play_state: PlayState,
    pub timeline_zoom: f64,   // pixels per second
    pub timeline_scroll: f64, // seconds at left edge
    pub dirty: bool,
    pub last_autosave: Option<std::time::Instant>,
    /// Pending ffmpeg error from a background job, surfaced to the UI.
    pub last_error: Option<String>,
}

impl EditorState {
    pub fn new() -> Arc<Self> {
        let (project, _gen) = Project::new();
        let inner = EditorStateInner {
            project,
            id_gen_seq: 0,
            selection: Selection::None,
            playhead: Time::ZERO,
            play_state: PlayState::Paused,
            timeline_zoom: 50.0,
            timeline_scroll: 0.0,
            dirty: false,
            last_autosave: None,
            last_error: None,
        };
        Arc::new(Self {
            inner: RwLock::new(inner),
        })
    }

    /// Generate the next id from the project's id generator.
    /// We don't keep a persistent IdGenerator because it isn't Serialize;
    /// instead we derive ids from a monotonic counter seeded at load
    /// time. This is fine because ids only need to be unique within a
    /// single session.
    pub fn next_id(&self) -> Id {
        let mut g = self.inner.write();
        g.id_gen_seq += 1;
        Id(g.id_gen_seq as u128)
    }

    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, EditorStateInner> {
        self.inner.read()
    }

    pub fn write(&self) -> parking_lot::RwLockWriteGuard<'_, EditorStateInner> {
        self.inner.write()
    }
}

/// Helper functions for the inner state. These mutate the project in
/// ways that should be undoable — they're typically called from
/// command handlers in `commands.rs`.
impl EditorStateInner {
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn add_asset(&mut self, asset: MediaAsset) {
        self.project.add_asset(asset);
        self.mark_dirty();
    }

    pub fn add_clip_from_asset(
        &mut self,
        asset_id: Id,
        track_id: Id,
        timeline_start: Time,
        next_id: Id,
    ) -> Option<Id> {
        let asset = self.project.asset(asset_id)?.clone();
        let track = self.project.timeline.track_mut(track_id)?;
        if track.locked {
            return None;
        }
        let source = edt_core::timeline::ClipSource::from_asset(&asset);
        let name = asset.name.clone();
        let clip = Clip::new(next_id, name, source, timeline_start);
        let clip_id = clip.id;
        track.insert_clip(clip);
        self.mark_dirty();
        Some(clip_id)
    }

    pub fn split_selected_clip_at_playhead(&mut self, next_id: Id) -> Option<Id> {
        let sel = match self.selection {
            Selection::Clip(id) => id,
            _ => return None,
        };
        let track_id = self.project.timeline.track_of_clip(sel)?;
        let track = self.project.timeline.track_mut(track_id)?;
        if track.locked {
            return None;
        }
        let pos = track.clips.iter().position(|c| c.id == sel)?;
        let new_clip = {
            let clip = &mut track.clips[pos];
            clip.split(next_id, self.playhead)?
        };
        let new_clip_id = new_clip.id;
        track.insert_clip(new_clip);
        self.mark_dirty();
        Some(new_clip_id)
    }

    pub fn delete_selected_clip(&mut self) -> bool {
        let sel = match self.selection {
            Selection::Clip(id) => id,
            _ => return false,
        };
        let track_id = match self.project.timeline.track_of_clip(sel) {
            Some(t) => t,
            None => return false,
        };
        let track = self.project.timeline.track_mut(track_id).unwrap();
        if track.locked {
            return false;
        }
        if track.remove_clip(sel).is_some() {
            self.selection = Selection::None;
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    pub fn nudge_playhead(&mut self, delta_secs: f64) {
        let mut t = self.playhead.0 + delta_secs;
        if t < 0.0 {
            t = 0.0;
        }
        let dur = self.project.duration().0;
        if t > dur {
            t = dur;
        }
        self.playhead = Time(t);
    }

    pub fn toggle_play(&mut self) {
        self.play_state = match self.play_state {
            PlayState::Paused => PlayState::Playing,
            PlayState::Playing => PlayState::Paused,
        };
    }

    pub fn stop(&mut self) {
        self.play_state = PlayState::Paused;
        self.playhead = Time::ZERO;
    }

    /// Advance the playhead by `dt_secs` seconds. Returns true if playback
    /// should continue (i.e. playhead < project duration).
    pub fn advance_playhead(&mut self, dt_secs: f64) -> bool {
        if self.play_state != PlayState::Playing {
            return false;
        }
        let new_t = self.playhead.0 + dt_secs;
        let dur = self.project.duration().0;
        if new_t >= dur {
            self.playhead = Time(dur);
            self.play_state = PlayState::Paused;
            false
        } else {
            self.playhead = Time(new_t);
            true
        }
    }

    pub fn selected_clip(&self) -> Option<&Clip> {
        let id = match self.selection {
            Selection::Clip(id) => id,
            _ => return None,
        };
        let (_, clip) = self.project.timeline.clip(id)?;
        Some(clip)
    }

    pub fn selected_clip_mut(&mut self) -> Option<&mut Clip> {
        let id = match self.selection {
            Selection::Clip(id) => id,
            _ => return None,
        };
        let track_id = self.project.timeline.track_of_clip(id)?;
        let track = self.project.timeline.track_mut(track_id)?;
        track.clips.iter_mut().find(|c| c.id == id)
    }

    /// First track of the given kind (top-most video or bottom-most audio).
    pub fn first_track_of_kind(&self, kind: TrackKind) -> Option<Id> {
        self.project
            .timeline
            .tracks
            .iter()
            .find(|t| t.kind == kind)
            .map(|t| t.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edt_core::media::{MediaAsset, MediaMetadata, VideoInfo};

    fn fake_asset(id: Id, name: &str, dur: f64) -> MediaAsset {
        MediaAsset {
            id,
            name: name.into(),
            path: format!("/tmp/{name}.mp4").into(),
            metadata: Some(MediaMetadata {
                duration: Some(dur),
                video: Some(VideoInfo {
                    width: 1920,
                    height: 1080,
                    fps: 30.0,
                    codec: "h264".into(),
                    pixel_format: "yuv420p".into(),
                }),
                audio: None,
                bitrate: None,
                format: "mp4".into(),
            }),
            label: 0,
            offline: false,
            proxy_path: None,
        }
    }

    #[test]
    fn add_clip_from_asset_creates_clip() {
        let state = EditorState::new();
        let asset_id = state.next_id();
        let track_id = state.read().first_track_of_kind(TrackKind::Video).unwrap();
        state.write().add_asset(fake_asset(asset_id, "clip", 10.0));
        let next_id = state.next_id();
        let clip_id = state
            .write()
            .add_clip_from_asset(asset_id, track_id, Time::ZERO, next_id);
        assert!(clip_id.is_some());
        let s = state.read();
        assert_eq!(s.project.duration().0, 10.0);
        assert!(s.dirty);
    }

    #[test]
    fn split_at_playhead_creates_two_clips() {
        let state = EditorState::new();
        let asset_id = state.next_id();
        let track_id = state.read().first_track_of_kind(TrackKind::Video).unwrap();
        state.write().add_asset(fake_asset(asset_id, "clip", 10.0));
        let next_id = state.next_id();
        let clip_id = state
            .write()
            .add_clip_from_asset(asset_id, track_id, Time::ZERO, next_id)
            .unwrap();
        state.write().selection = Selection::Clip(clip_id);
        state.write().playhead = Time(5.0);
        let split_id = state.next_id();
        let new_id = state.write().split_selected_clip_at_playhead(split_id);
        assert!(new_id.is_some());
        let s = state.read();
        let track = s.project.timeline.track(track_id).unwrap();
        assert_eq!(track.clips.len(), 2);
    }

    #[test]
    fn delete_selected_clip_removes_it() {
        let state = EditorState::new();
        let asset_id = state.next_id();
        let track_id = state.read().first_track_of_kind(TrackKind::Video).unwrap();
        state.write().add_asset(fake_asset(asset_id, "clip", 10.0));
        let next_id = state.next_id();
        let clip_id = state
            .write()
            .add_clip_from_asset(asset_id, track_id, Time::ZERO, next_id)
            .unwrap();
        state.write().selection = Selection::Clip(clip_id);
        let deleted = state.write().delete_selected_clip();
        assert!(deleted);
        let s = state.read();
        let track = s.project.timeline.track(track_id).unwrap();
        assert_eq!(track.clips.len(), 0);
        assert_eq!(s.selection, Selection::None);
    }

    #[test]
    fn playhead_clamps_to_duration() {
        let state = EditorState::new();
        state.write().nudge_playhead(100.0);
        assert_eq!(state.read().playhead.0, 0.0);
    }

    #[test]
    fn advance_playhead_stops_at_end() {
        let state = EditorState::new();
        let asset_id = state.next_id();
        let track_id = state.read().first_track_of_kind(TrackKind::Video).unwrap();
        state.write().add_asset(fake_asset(asset_id, "clip", 2.0));
        let next_id = state.next_id();
        state
            .write()
            .add_clip_from_asset(asset_id, track_id, Time::ZERO, next_id);
        state.write().toggle_play();
        assert!(state.write().advance_playhead(1.0));
        assert!(!state.write().advance_playhead(5.0));
        assert_eq!(state.read().play_state, PlayState::Paused);
    }
}
