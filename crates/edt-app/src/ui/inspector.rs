//! Inspector / properties panel.
//!
//! When a clip is selected, shows its properties (name, source range,
//! level, speed, label color). When a track is selected (or nothing),
//! shows project settings + transport info.

use crate::app::EdtApp;
use crate::state::Selection;
use crate::ui::{label_color, panel_header, TEXT, TEXT_DIM};
use eframe::egui;
use egui::{Context, Ui};

pub fn render(app: &mut EdtApp, _ctx: &Context, ui: &mut Ui) {
    panel_header(ui, "Inspector");

    let selection = app.state.read().selection;
    match selection {
        Selection::Clip(clip_id) => {
            render_clip_props(app, ui, clip_id);
        }
        Selection::Track(track_id) => {
            render_track_props(app, ui, track_id);
        }
        Selection::None => {
            render_project_props(app, ui);
        }
    }
}

fn render_clip_props(app: &mut EdtApp, ui: &mut Ui, clip_id: edt_core::id::Id) {
    // Snapshot the clip data we need.
    let snapshot = {
        let s = app.state.read();
        let (_, clip) = match s.project.timeline.clip(clip_id) {
            Some(x) => x,
            None => return,
        };
        ClipSnapshot {
            id: clip.id,
            name: clip.name.clone(),
            timeline_start: clip.timeline_start.0,
            timeline_end: clip.timeline_end.0,
            source_start: clip.source.source_start.0,
            source_end: clip.source.source_end.0,
            speed: clip.speed.0,
            level: clip.level,
            muted: clip.muted,
            label: clip.label,
        }
    };

    ui.label(egui::RichText::new("Clip").strong().color(TEXT).size(13.0));
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Name");
        let mut name = snapshot.name.clone();
        ui.text_edit_singleline(&mut name);
        if name != snapshot.name {
            app.state
                .write()
                .project
                .timeline
                .track_of_clip(clip_id)
                .and_then(|tid| {
                    app.state
                        .write()
                        .project
                        .timeline
                        .track_mut(tid)?
                        .clips
                        .iter_mut()
                        .find(|c| c.id == clip_id)?
                        .name = name;
                    Some(())
                });
        }
    });

    ui.horizontal(|ui| {
        ui.label("Label");
        for i in 0..8u8 {
            let color = label_color(i);
            let selected = snapshot.label == i;
            let btn = egui::RadioButton::new(selected, "");
            if ui.add(btn).clicked() {
                let mut s = app.state.write();
                if let Some(tid) = s.project.timeline.track_of_clip(clip_id) {
                    if let Some(track) = s.project.timeline.track_mut(tid) {
                        if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                            clip.label = i;
                            s.mark_dirty();
                        }
                    }
                }
            }
            // Draw the color swatch beside the radio button.
            let cursor = ui.cursor();
            let swatch_rect = egui::Rect::from_min_size(
                egui::pos2(cursor.max.x - 10.0, cursor.min.y + 4.0),
                egui::vec2(10.0, 10.0),
            );
            ui.painter().rect_filled(swatch_rect, 2.0, color);
            ui.add_space(14.0);
        }
    });

    ui.separator();
    ui.label(egui::RichText::new("Timing").color(TEXT).size(12.0));
    ui.label(format!(
        "Timeline: {:.3}s → {:.3}s ({:.3}s)",
        snapshot.timeline_start,
        snapshot.timeline_end,
        snapshot.timeline_end - snapshot.timeline_start
    ));
    ui.label(format!(
        "Source:   {:.3}s → {:.3}s ({:.3}s)",
        snapshot.source_start,
        snapshot.source_end,
        snapshot.source_end - snapshot.source_start
    ));

    let mut speed = snapshot.speed;
    ui.horizontal(|ui| {
        ui.label("Speed");
        ui.add(
            egui::Slider::new(&mut speed, 0.25..=4.0)
                .step_by(0.05)
                .text("×"),
        );
    });
    if (speed - snapshot.speed).abs() > 1e-3 {
        let mut s = app.state.write();
        if let Some(tid) = s.project.timeline.track_of_clip(clip_id) {
            if let Some(track) = s.project.timeline.track_mut(tid) {
                if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                    clip.speed = edt_core::timeline::ClipSpeed(speed);
                    let src_dur = clip.source.duration().0;
                    clip.timeline_end =
                        edt_core::time::Time(clip.timeline_start.0 + src_dur / speed);
                    s.mark_dirty();
                }
            }
        }
    }

    let mut level = snapshot.level;
    ui.horizontal(|ui| {
        ui.label("Level");
        ui.add(egui::Slider::new(&mut level, 0.0..=1.0).text(""));
    });
    if (level - snapshot.level).abs() > 1e-3 {
        let mut s = app.state.write();
        if let Some(tid) = s.project.timeline.track_of_clip(clip_id) {
            if let Some(track) = s.project.timeline.track_mut(tid) {
                if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                    clip.level = level;
                    s.mark_dirty();
                }
            }
        }
    }

    let mut muted = snapshot.muted;
    ui.checkbox(&mut muted, "Muted");
    if muted != snapshot.muted {
        let mut s = app.state.write();
        if let Some(tid) = s.project.timeline.track_of_clip(clip_id) {
            if let Some(track) = s.project.timeline.track_mut(tid) {
                if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                    clip.muted = muted;
                    s.mark_dirty();
                }
            }
        }
    }

    ui.separator();
    ui.label(
        egui::RichText::new("Effects (roadmap)")
            .color(TEXT_DIM)
            .size(11.0),
    );
    ui.label(
        egui::RichText::new("Color grade, blur, text — coming in v0.2.")
            .color(TEXT_DIM)
            .small(),
    );

    ui.separator();
    ui.label(egui::RichText::new("Actions").color(TEXT).size(12.0));
    if ui.button("Split at playhead").clicked() {
        app.split_at_playhead();
    }
    if ui.button("Delete clip").clicked() {
        app.delete_selected();
    }
}

fn render_track_props(app: &mut EdtApp, ui: &mut Ui, track_id: edt_core::id::Id) {
    ui.label(egui::RichText::new("Track").strong().color(TEXT).size(13.0));
    ui.separator();

    let snapshot = {
        let s = app.state.read();
        let track = match s.project.timeline.track(track_id) {
            Some(t) => t,
            None => return,
        };
        TrackSnapshot {
            name: track.name.clone(),
            kind: track.kind,
            muted: track.muted,
            solo: track.solo,
            locked: track.locked,
            level: track.level,
            clips_count: track.clips.len() as u32,
            duration: track.duration().0,
        }
    };

    ui.label(format!("Name: {}", snapshot.name));
    ui.label(format!("Kind: {:?}", snapshot.kind));
    ui.label(format!("Clips: {}", snapshot.clips_count));
    ui.label(format!("Duration: {:.3}s", snapshot.duration));

    let mut muted = snapshot.muted;
    ui.checkbox(&mut muted, "Muted");
    if muted != snapshot.muted {
        let mut s = app.state.write();
        if let Some(track) = s.project.timeline.track_mut(track_id) {
            track.muted = muted;
            s.mark_dirty();
        }
    }

    let mut locked = snapshot.locked;
    ui.checkbox(&mut locked, "Locked");
    if locked != snapshot.locked {
        let mut s = app.state.write();
        if let Some(track) = s.project.timeline.track_mut(track_id) {
            track.locked = locked;
            s.mark_dirty();
        }
    }

    let mut level = snapshot.level;
    ui.add(egui::Slider::new(&mut level, 0.0..=1.0).text("Level"));
    if (level - snapshot.level).abs() > 1e-3 {
        let mut s = app.state.write();
        if let Some(track) = s.project.timeline.track_mut(track_id) {
            track.level = level;
            s.mark_dirty();
        }
    }
}

fn render_project_props(app: &mut EdtApp, ui: &mut Ui) {
    ui.label(
        egui::RichText::new("Project")
            .strong()
            .color(TEXT)
            .size(13.0),
    );
    ui.separator();

    let snapshot = {
        let s = app.state.read();
        ProjectSnapshot {
            name: s.project.settings.name.clone(),
            fps: s.project.settings.fps,
            width: s.project.settings.width,
            height: s.project.settings.height,
            audio_sample_rate: s.project.settings.audio_sample_rate,
            audio_channels: s.project.settings.audio_channels,
            asset_count: s.project.assets.len() as u32,
            track_count: s.project.timeline.tracks.len() as u32,
            duration: s.project.duration().0,
        }
    };

    ui.label(format!("Name: {}", snapshot.name));
    ui.label(format!(
        "Resolution: {}×{}",
        snapshot.width, snapshot.height
    ));
    ui.label(format!("Framerate: {:.2} fps", snapshot.fps));
    ui.label(format!(
        "Audio: {}Hz · {}ch",
        snapshot.audio_sample_rate, snapshot.audio_channels
    ));
    ui.label(format!("Assets: {}", snapshot.asset_count));
    ui.label(format!("Tracks: {}", snapshot.track_count));
    ui.label(format!("Duration: {:.3}s", snapshot.duration));

    ui.separator();
    ui.label(
        egui::RichText::new("Nothing selected")
            .color(TEXT_DIM)
            .small(),
    );
    ui.label(
        egui::RichText::new("Click a clip in the timeline to edit its properties.")
            .color(TEXT_DIM)
            .small(),
    );
}

#[derive(Debug, Clone)]
struct ClipSnapshot {
    id: edt_core::id::Id,
    name: String,
    timeline_start: f64,
    timeline_end: f64,
    source_start: f64,
    source_end: f64,
    speed: f64,
    level: f32,
    muted: bool,
    label: u8,
}

#[derive(Debug, Clone)]
struct TrackSnapshot {
    name: String,
    kind: edt_core::timeline::TrackKind,
    muted: bool,
    solo: bool,
    locked: bool,
    level: f32,
    clips_count: u32,
    duration: f64,
}

#[derive(Debug, Clone)]
struct ProjectSnapshot {
    name: String,
    fps: f64,
    width: u32,
    height: u32,
    audio_sample_rate: u32,
    audio_channels: u32,
    asset_count: u32,
    track_count: u32,
    duration: f64,
}
