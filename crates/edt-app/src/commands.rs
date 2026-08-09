//! Undo/redo command stack.
//!
//! Each command is an object that knows how to apply and revert itself.
//! For MVP the command set is small (add clip, delete clip, split clip,
//! move clip, trim clip). Adding new commands is mechanical.

use crate::state::EditorState;
use edt_core::id::Id;
use edt_core::time::Time;
use edt_core::timeline::Clip;
use std::sync::Arc;

/// A reversible edit operation.
pub trait Command: std::fmt::Debug + Send + Sync {
    fn apply(&self, state: &Arc<EditorState>);
    fn revert(&self, state: &Arc<EditorState>);
    fn describe(&self) -> &'static str;
}

/// The undo stack. Holds up to `MAX_UNDO` commands; older commands are
/// dropped (FIFO eviction).
pub struct UndoStack {
    undo: Vec<Box<dyn Command>>,
    redo: Vec<Box<dyn Command>>,
    pub max: usize,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            max: 100,
        }
    }
}

impl UndoStack {
    pub fn push(&mut self, cmd: Box<dyn Command>, state: &Arc<EditorState>) {
        if self.undo.len() >= self.max {
            self.undo.remove(0);
        }
        cmd.apply(state);
        self.undo.push(cmd);
        self.redo.clear();
    }

    pub fn undo(&mut self, state: &Arc<EditorState>) -> bool {
        let cmd = match self.undo.pop() {
            Some(c) => c,
            None => return false,
        };
        cmd.revert(state);
        self.redo.push(cmd);
        true
    }

    pub fn redo(&mut self, state: &Arc<EditorState>) -> bool {
        let cmd = match self.redo.pop() {
            Some(c) => c,
            None => return false,
        };
        cmd.apply(state);
        self.undo.push(cmd);
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Concrete commands
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct AddClipCmd {
    pub clip: Clip,
    pub track_id: Id,
}

impl Command for AddClipCmd {
    fn apply(&self, state: &Arc<EditorState>) {
        let mut s = state.write();
        if let Some(track) = s.project.timeline.track_mut(self.track_id) {
            track.insert_clip(self.clip.clone());
            s.mark_dirty();
        }
    }
    fn revert(&self, state: &Arc<EditorState>) {
        let mut s = state.write();
        if let Some(track) = s.project.timeline.track_mut(self.track_id) {
            track.remove_clip(self.clip.id);
            s.mark_dirty();
        }
    }
    fn describe(&self) -> &'static str {
        "Add clip"
    }
}

#[derive(Debug)]
pub struct DeleteClipCmd {
    pub clip: Clip,
    pub track_id: Id,
}

impl Command for DeleteClipCmd {
    fn apply(&self, state: &Arc<EditorState>) {
        let mut s = state.write();
        if let Some(track) = s.project.timeline.track_mut(self.track_id) {
            track.remove_clip(self.clip.id);
            s.mark_dirty();
        }
    }
    fn revert(&self, state: &Arc<EditorState>) {
        let mut s = state.write();
        if let Some(track) = s.project.timeline.track_mut(self.track_id) {
            track.insert_clip(self.clip.clone());
            s.mark_dirty();
        }
    }
    fn describe(&self) -> &'static str {
        "Delete clip"
    }
}

#[derive(Debug, Clone)]
pub struct MoveClipCmd {
    pub clip_id: Id,
    pub track_id: Id,
    pub old_timeline_start: Time,
    pub new_timeline_start: Time,
}

impl Command for MoveClipCmd {
    fn apply(&self, state: &Arc<EditorState>) {
        let mut s = state.write();
        if let Some(track) = s.project.timeline.track_mut(self.track_id) {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == self.clip_id) {
                let dur = clip.timeline_duration();
                clip.timeline_start = self.new_timeline_start;
                clip.timeline_end = self.new_timeline_start + dur;
                s.mark_dirty();
            }
        }
    }
    fn revert(&self, state: &Arc<EditorState>) {
        let mut s = state.write();
        if let Some(track) = s.project.timeline.track_mut(self.track_id) {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == self.clip_id) {
                let dur = clip.timeline_duration();
                clip.timeline_start = self.old_timeline_start;
                clip.timeline_end = self.old_timeline_start + dur;
                s.mark_dirty();
            }
        }
    }
    fn describe(&self) -> &'static str {
        "Move clip"
    }
}

#[derive(Debug, Clone)]
pub struct SplitClipCmd {
    pub left_id: Id,
    pub right_id: Id,
    pub track_id: Id,
    pub split_time: Time,
    pub original_clip: Clip,
}

impl Command for SplitClipCmd {
    fn apply(&self, state: &Arc<EditorState>) {
        let mut s = state.write();
        if let Some(track) = s.project.timeline.track_mut(self.track_id) {
            // Replace original with left half and insert right half.
            if let Some(pos) = track.clips.iter().position(|c| c.id == self.left_id) {
                let mut right = self.original_clip.clone();
                right.id = self.right_id;
                let _ = track.clips[pos].split(self.right_id, self.split_time);
                track.insert_clip(right);
                s.mark_dirty();
            }
        }
    }
    fn revert(&self, state: &Arc<EditorState>) {
        let mut s = state.write();
        if let Some(track) = s.project.timeline.track_mut(self.track_id) {
            track.remove_clip(self.right_id);
            if let Some(pos) = track.clips.iter().position(|c| c.id == self.left_id) {
                track.clips[pos] = self.original_clip.clone();
            }
            // Re-sort.
            track.clips.sort_by(|a, b| {
                a.timeline_start
                    .0
                    .partial_cmp(&b.timeline_start.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            s.mark_dirty();
        }
    }
    fn describe(&self) -> &'static str {
        "Split clip"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::EditorState;
    use edt_core::media::{MediaAsset, MediaMetadata, VideoInfo};
    use edt_core::timeline::{ClipSource, TrackKind};

    fn setup_with_clip() -> (Arc<EditorState>, Id, Id) {
        let state = EditorState::new();
        let asset_id = state.next_id();
        let track_id = state.read().first_track_of_kind(TrackKind::Video).unwrap();
        let asset = MediaAsset {
            id: asset_id,
            name: "clip".into(),
            path: "/tmp/clip.mp4".into(),
            metadata: Some(MediaMetadata {
                duration: Some(10.0),
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
        };
        state.write().add_asset(asset);
        let next_id = state.next_id();
        let clip_id = state
            .write()
            .add_clip_from_asset(asset_id, track_id, Time::ZERO, next_id)
            .unwrap();
        (state, track_id, clip_id)
    }

    #[test]
    fn add_and_undo_restores_state() {
        let (state, _track, _clip) = setup_with_clip();
        // The setup already added one clip. Test a second add via command.
        let asset_id = state.read().project.assets[0].id;
        let track_id = state.read().first_track_of_kind(TrackKind::Video).unwrap();
        let next_id = state.next_id();
        let source = ClipSource::from_asset(&state.read().project.assets[0]);
        let new_clip = Clip::new(next_id, "clip2", source, Time(15.0));
        let mut undo = UndoStack::default();
        let count_before = state
            .read()
            .project
            .timeline
            .track(track_id)
            .unwrap()
            .clips
            .len();
        undo.push(
            Box::new(AddClipCmd {
                clip: new_clip,
                track_id,
            }),
            &state,
        );
        let count_after = state
            .read()
            .project
            .timeline
            .track(track_id)
            .unwrap()
            .clips
            .len();
        assert_eq!(count_after, count_before + 1);
        undo.undo(&state);
        let count_undo = state
            .read()
            .project
            .timeline
            .track(track_id)
            .unwrap()
            .clips
            .len();
        assert_eq!(count_undo, count_before);
        let _ = asset_id; // suppress unused
    }

    #[test]
    fn move_clip_command_roundtrips() {
        let (state, track_id, clip_id) = setup_with_clip();
        let old_start =
            state.read().project.timeline.track(track_id).unwrap().clips[0].timeline_start;
        let mut undo = UndoStack::default();
        undo.push(
            Box::new(MoveClipCmd {
                clip_id,
                track_id,
                old_timeline_start: old_start,
                new_timeline_start: Time(5.0),
            }),
            &state,
        );
        let new_start =
            state.read().project.timeline.track(track_id).unwrap().clips[0].timeline_start;
        assert_eq!(new_start.0, 5.0);
        undo.undo(&state);
        let restored =
            state.read().project.timeline.track(track_id).unwrap().clips[0].timeline_start;
        assert_eq!(restored.0, old_start.0);
    }
}
