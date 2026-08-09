//! Timeline panel — multi-track clip display with editing interactions.
//!
//! The timeline is rendered manually with `egui::Painter` for performance
//! and to support custom interactions (drag, trim, snap). Layout:
//!
//! ```text
//! +------------------------------------------------------+
//! | ruler:  0s    1s    2s    3s    4s    5s    6s       |
//! +------------------------------------------------------+
//! | track header | clip clip        clip                 |
//! | track header | clip                                  |
//! +------------------------------------------------------+
//! ```
//!
//! The left track-header column is fixed at `HEADER_W = 110px`. The
//! remaining width is the timeline canvas.

use crate::app::EdtApp;
use crate::state::Selection;
use crate::ui::{
    panel_header, PLAYHEAD, SELECTION, TEXT_DIM, TRACK_AUDIO, TRACK_VIDEO, WIDGET_BG,
};
use eframe::egui;
use egui::{Color32, Context, PointerButton, Rect, Response, Sense, Ui, Vec2};
use std::collections::HashSet;

const HEADER_W: f32 = 110.0;
const TRACK_H: f32 = 56.0;
const RULER_H: f32 = 22.0;
const SNAP_THRESHOLD_PX: f64 = 8.0;

/// State of an in-progress timeline drag (move, trim, or scrub).
#[derive(Debug, Clone, Copy)]
pub struct DragState {
    pub kind: DragKind,
    pub clip_id: Option<edt_core::id::Id>,
    pub start_pointer_x: f32,
    pub start_value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragKind {
    None,
    MoveClip,
    TrimLeft,
    TrimRight,
    Scrub,
}

impl Default for DragState {
    fn default() -> Self {
        Self {
            kind: DragKind::None,
            clip_id: None,
            start_pointer_x: 0.0,
            start_value: 0.0,
        }
    }
}

pub fn render(app: &mut EdtApp, ctx: &Context, ui: &mut Ui) {
    panel_header(ui, "Timeline");

    // Snapshot of timeline data we need for rendering.
    let snapshot = {
        let s = app.state.read();
        let tracks: Vec<TrackSnap> = s
            .project
            .timeline
            .tracks
            .iter()
            .map(|t| TrackSnap {
                id: t.id,
                name: t.name.clone(),
                kind: t.kind,
                muted: t.muted,
                locked: t.locked,
                clips: t
                    .clips
                    .iter()
                    .map(|c| ClipSnap {
                        id: c.id,
                        name: c.name.clone(),
                        start: c.timeline_start.0,
                        end: c.timeline_end.0,
                        label: c.label,
                    })
                    .collect(),
            })
            .collect();
        TimelineSnap {
            tracks,
            zoom: s.timeline_zoom,
            scroll: s.timeline_scroll,
            playhead: s.playhead.0,
            selection: s.selection,
            duration: s.project.duration().0,
        }
    };

    // Top toolbar: zoom controls + time display.
    ui.horizontal(|ui| {
        if ui.button("⟵").on_hover_text("Scroll left").clicked() {
            app.state.write().timeline_scroll = (app.state.read().timeline_scroll - 1.0).max(0.0);
        }
        if ui.button("⟶").on_hover_text("Scroll right").clicked() {
            app.state.write().timeline_scroll += 1.0;
        }
        ui.separator();
        if ui.button("−").on_hover_text("Zoom out").clicked() {
            let mut s = app.state.write();
            s.timeline_zoom = (s.timeline_zoom * 0.8).max(2.0);
        }
        if ui.button("＋").on_hover_text("Zoom in").clicked() {
            let mut s = app.state.write();
            s.timeline_zoom = (s.timeline_zoom * 1.25).min(500.0);
        }
        ui.separator();
        ui.label(format!("Zoom: {:.0} px/s", snapshot.zoom));
        ui.separator();
        ui.label(format!("Playhead: {:.3}s", snapshot.playhead));
        ui.separator();
        ui.label(format!("Duration: {:.3}s", snapshot.duration));
    });
    ui.separator();

    // Allocate the canvas.
    let available = ui.available_size();
    let canvas_w = available.x - HEADER_W;
    let canvas_w = canvas_w.max(200.0);
    let canvas_h = available.y.min(snapshot.tracks.len() as f32 * TRACK_H + RULER_H + 8.0);
    let canvas_w_total = canvas_w + HEADER_W;

    let (canvas_rect, canvas_resp) =
        ui.allocate_exact_size(egui::vec2(canvas_w_total, canvas_h), Sense::drag());
    let painter = ui.painter().with_clip_rect(canvas_rect);

    let header_rect = Rect::from_min_size(canvas_rect.min, egui::vec2(HEADER_W, canvas_h));
    let ruler_rect = Rect::from_min_size(
        egui::pos2(canvas_rect.min.x + HEADER_W, canvas_rect.min.y),
        egui::vec2(canvas_w, RULER_H),
    );
    let body_rect = Rect::from_min_size(
        egui::pos2(ruler_rect.min.x, ruler_rect.min.y + RULER_H),
        egui::vec2(canvas_w, canvas_h - RULER_H),
    );

    // Ruler background + ticks.
    painter.rect_filled(ruler_rect, 0.0, WIDGET_BG);
    draw_ruler(&painter, ruler_rect, snapshot.zoom, snapshot.scroll);

    // Track header backgrounds.
    painter.rect_filled(header_rect, 0.0, Color32::from_rgb(24, 26, 32));

    // Body background.
    painter.rect_filled(body_rect, 0.0, Color32::from_rgb(16, 18, 22));

    // Draw each track row.
    let mut clip_rects: Vec<(edt_core::id::Id, Rect, bool)> = Vec::new();
    for (i, track) in snapshot.tracks.iter().enumerate() {
        let track_y = body_rect.min.y + i as f32 * TRACK_H;
        let track_rect = Rect::from_min_size(
            egui::pos2(body_rect.min.x, track_y),
            egui::vec2(canvas_w, TRACK_H),
        );
        // Alternating row stripe.
        if i % 2 == 0 {
            painter.rect_filled(track_rect, 0.0, Color32::from_rgb(20, 22, 28));
        }

        // Track header.
        let header_track_rect = Rect::from_min_size(
            egui::pos2(header_rect.min.x, track_y),
            egui::vec2(HEADER_W, TRACK_H),
        );
        let track_color = if track.kind == edt_core::timeline::TrackKind::Video {
            TRACK_VIDEO
        } else {
            TRACK_AUDIO
        };
        painter.rect_filled(
            Rect::from_min_size(header_track_rect.min, egui::vec2(4.0, TRACK_H)),
            0.0,
            track_color,
        );
        painter.text(
            egui::pos2(header_track_rect.min.x + 10.0, header_track_rect.min.y + 6.0),
            egui::Align2::LEFT_TOP,
            &track.name,
            egui::FontId::proportional(12.0),
            Color32::from_rgb(220, 224, 230),
        );
        if track.muted {
            painter.text(
                egui::pos2(header_track_rect.min.x + 10.0, header_track_rect.min.y + 22.0),
                egui::Align2::LEFT_TOP,
                "(muted)",
                egui::FontId::proportional(10.0),
                TEXT_DIM,
            );
        }
        if track.locked {
            painter.text(
                egui::pos2(header_track_rect.min.x + 10.0, header_track_rect.min.y + 36.0),
                egui::Align2::LEFT_TOP,
                "(locked)",
                egui::FontId::proportional(10.0),
                Color32::from_rgb(248, 113, 113),
            );
        }

        // Clips.
        for clip in &track.clips {
            let x_start = body_rect.min.x + ((clip.start - snapshot.scroll) * snapshot.zoom) as f32;
            let x_end = body_rect.min.x + ((clip.end - snapshot.scroll) * snapshot.zoom) as f32;
            let clip_rect = Rect::from_min_max(
                egui::pos2(x_start.max(body_rect.min.x), track_y + 4.0),
                egui::pos2(x_end.min(body_rect.max.x), track_y + TRACK_H - 4.0),
            );
            if clip_rect.width() < 1.0 {
                continue;
            }
            let is_video = track.kind == edt_core::timeline::TrackKind::Video;
            let mut color = if is_video { TRACK_VIDEO } else { TRACK_AUDIO };
            let is_selected = matches!(snapshot.selection, Selection::Clip(id) if id == clip.id);
            if is_selected {
                color = color.lighten(0.4);
            }
            painter.rect_filled(clip_rect, 2.0, color);
            // Label color stripe.
            let stripe_color = crate::ui::styles::label_color(clip.label);
            painter.rect_filled(
                Rect::from_min_size(clip_rect.min, egui::vec2(3.0, clip_rect.height())),
                0.0,
                stripe_color,
            );
            // Clip name.
            let text = truncate(&clip.name, (clip_rect.width() / 7.0) as usize);
            painter.text(
                egui::pos2(clip_rect.min.x + 8.0, clip_rect.center().y),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::proportional(11.0),
                Color32::from_rgb(20, 22, 26),
            );
            if is_selected {
                painter.rect_stroke(clip_rect, 2.0, egui::Stroke::new(2.0, SELECTION));
            }
            // Trim handles (4px on each edge).
            let left_handle = Rect::from_min_size(clip_rect.min, egui::vec2(4.0, clip_rect.height()));
            let right_handle = Rect::from_min_size(
                egui::pos2(clip_rect.max.x - 4.0, clip_rect.min.y),
                egui::vec2(4.0, clip_rect.height()),
            );
            painter.rect_filled(left_handle, 0.0, Color32::from_rgb(255, 255, 255).linear_multiply(0.5));
            painter.rect_filled(right_handle, 0.0, Color32::from_rgb(255, 255, 255).linear_multiply(0.5));

            clip_rects.push((clip.id, clip_rect, is_video));
        }

        // Track separator.
        painter.line_segment(
            [
                egui::pos2(body_rect.min.x, track_y + TRACK_H),
                egui::pos2(body_rect.max.x, track_y + TRACK_H),
            ],
            egui::Stroke::new(1.0, Color32::from_rgb(40, 42, 50)),
        );
    }

    // Playhead.
    let playhead_x = body_rect.min.x + ((snapshot.playhead - snapshot.scroll) * snapshot.zoom) as f32;
    if playhead_x >= body_rect.min.x && playhead_x <= body_rect.max.x {
        painter.line_segment(
            [egui::pos2(playhead_x, ruler_rect.min.y), egui::pos2(playhead_x, body_rect.max.y)],
            egui::Stroke::new(2.0, PLAYHEAD),
        );
        // Triangle at top.
        let tri = vec![
            egui::pos2(playhead_x - 5.0, ruler_rect.min.y),
            egui::pos2(playhead_x + 5.0, ruler_rect.min.y),
            egui::pos2(playhead_x, ruler_rect.min.y + 6.0),
        ];
        painter.add(egui::Shape::convex_polygon {
            points: tri,
            fill: PLAYHEAD,
            stroke: egui::Stroke::NONE,
        });
    }

    // ---- Interaction ----
    let drag = &mut app.timeline_drag;
    let pointer = ctx.input(|i| i.pointer.latest_pos());
    let clicked = canvas_resp.drag_started_by(PointerButton::Primary)
        || canvas_resp.drag_started_by(PointerButton::Secondary);
    let dragging = canvas_resp.is_pointer_button_down_on() && ctx.input(|i| i.pointer.is_decidedly_dragging());

    if let Some(p) = pointer {
        if body_rect.contains(p) {
            // Determine what's under the pointer.
            let mut found_clip: Option<(edt_core::id::Id, DragKind)> = None;
            for (id, rect, _) in &clip_rects {
                if rect.contains(p) {
                    let left_handle = Rect::from_min_size(rect.min, egui::vec2(5.0, rect.height()));
                    let right_handle = Rect::from_min_size(
                        egui::pos2(rect.max.x - 5.0, rect.min.y),
                        egui::vec2(5.0, rect.height()),
                    );
                    let kind = if left_handle.contains(p) {
                        DragKind::TrimLeft
                    } else if right_handle.contains(p) {
                        DragKind::TrimRight
                    } else {
                        DragKind::MoveClip
                    };
                    found_clip = Some((*id, kind));
                    break;
                }
            }

            // Cursor hint.
            let cursor = match found_clip.map(|(_, k)| k) {
                Some(DragKind::TrimLeft) | Some(DragKind::TrimRight) => egui::CursorIcon::ResizeHorizontal,
                Some(DragKind::MoveClip) => egui::CursorIcon::Move,
                _ => egui::CursorIcon::default(),
            };
            canvas_resp.on_hover_cursor(cursor);

            if clicked {
                if let Some((id, kind)) = found_clip {
                    app.state.write().selection = Selection::Clip(id);
                    if kind == DragKind::MoveClip {
                        let start = snapshot
                            .tracks
                            .iter()
                            .flat_map(|t| t.clips.iter().find(|c| c.id == id).cloned())
                            .next()
                            .map(|c| c.start)
                            .unwrap_or(0.0);
                        *drag = DragState {
                            kind,
                            clip_id: Some(id),
                            start_pointer_x: p.x,
                            start_value: start,
                        };
                    } else {
                        let clip = snapshot
                            .tracks
                            .iter()
                            .flat_map(|t| t.clips.iter().find(|c| c.id == id).cloned())
                            .next();
                        if let Some(c) = clip {
                            *drag = DragState {
                                kind,
                                clip_id: Some(id),
                                start_pointer_x: p.x,
                                start_value: if kind == DragKind::TrimLeft {
                                    c.start
                                } else {
                                    c.end
                                },
                            };
                        }
                    }
                } else {
                    // Click on empty canvas: scrub playhead.
                    let t = snapshot.scroll + ((p.x - body_rect.min.x) / snapshot.zoom) as f64;
                    let mut s = app.state.write();
                    s.playhead = edt_core::time::Time(t.max(0.0));
                    s.selection = Selection::None;
                    *drag = DragState {
                        kind: DragKind::Scrub,
                        clip_id: None,
                        start_pointer_x: p.x,
                        start_value: t,
                    };
                }
            } else if dragging && drag.kind != DragKind::None {
                let dx = (p.x - drag.start_pointer_x) / snapshot.zoom as f32;
                let new_t = (drag.start_value + dx as f64).max(0.0);

                // Snap to other clips' edges and to playhead.
                let snapped = snap_to_edges(new_t, &snapshot, drag.clip_id, snapshot.zoom);
                let snapped_t = snapped.unwrap_or(new_t);

                match drag.kind {
                    DragKind::MoveClip => {
                        if let Some(id) = drag.clip_id {
                            let track_id = app.state.read().project.timeline.track_of_clip(id);
                            if let Some(track_id) = track_id {
                                let mut s = app.state.write();
                                if let Some(track) = s.project.timeline.track_mut(track_id) {
                                    if let Some(clip) = track.clips.iter_mut().find(|c| c.id == id) {
                                        let dur = clip.timeline_duration();
                                        clip.timeline_start = edt_core::time::Time(snapped_t);
                                        clip.timeline_end = edt_core::time::Time(snapped_t + dur.0);
                                        s.mark_dirty();
                                    }
                                }
                            }
                        }
                    }
                    DragKind::TrimLeft => {
                        if let Some(id) = drag.clip_id {
                            let mut s = app.state.write();
                            if let Some(track_id) = s.project.timeline.track_of_clip(id) {
                                if let Some(track) = s.project.timeline.track_mut(track_id) {
                                    if let Some(clip) = track.clips.iter_mut().find(|c| c.id == id) {
                                        if snapped_t < clip.timeline_end.0 - 0.1 {
                                            clip.trim_left(edt_core::time::Time(snapped_t));
                                            s.mark_dirty();
                                        }
                                    }
                                }
                            }
                        }
                    }
                    DragKind::TrimRight => {
                        if let Some(id) = drag.clip_id {
                            let mut s = app.state.write();
                            if let Some(track_id) = s.project.timeline.track_of_clip(id) {
                                if let Some(track) = s.project.timeline.track_mut(track_id) {
                                    if let Some(clip) = track.clips.iter_mut().find(|c| c.id == id) {
                                        if snapped_t > clip.timeline_start.0 + 0.1 {
                                            clip.trim_right(edt_core::time::Time(snapped_t));
                                            s.mark_dirty();
                                        }
                                    }
                                }
                            }
                        }
                    }
                    DragKind::Scrub => {
                        let t = snapshot.scroll + ((p.x - body_rect.min.x) / snapshot.zoom) as f64;
                        app.state.write().playhead = edt_core::time::Time(t.max(0.0));
                    }
                    DragKind::None => {}
                }
            }
        }
    }

    if !dragging {
        *drag = DragState::default();
    }

    // Drop target: if dragging an asset from the media pool onto a track.
    if let Some(asset_id) = app.pending_drag_asset.take() {
        if let Some(p) = pointer {
            // Figure out which track was hit.
            for (i, track) in snapshot.tracks.iter().enumerate() {
                let track_y = body_rect.min.y + i as f32 * TRACK_H;
                let track_rect = Rect::from_min_size(
                    egui::pos2(body_rect.min.x, track_y),
                    egui::vec2(canvas_w, TRACK_H),
                );
                if track_rect.contains(p) && !track.locked {
                    let t = snapshot.scroll + ((p.x - body_rect.min.x) / snapshot.zoom) as f64;
                    let next_id = app.state.next_id();
                    app.state
                        .write()
                        .add_clip_from_asset(asset_id, track.id, edt_core::time::Time(t.max(0.0)), next_id);
                    break;
                }
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn draw_ruler(painter: &egui::Painter, rect: Rect, zoom: f64, scroll: f64) {
    // Choose a tick interval so that ticks are ~80px apart.
    let target_px = 80.0;
    let target_secs = target_px / zoom;
    let interval = nice_interval(target_secs);
    let start_t = (scroll / interval).floor() * interval;
    let end_t = scroll + (rect.width() as f64 / zoom);
    let mut t = start_t;
    while t <= end_t {
        let x = rect.min.x + ((t - scroll) * zoom) as f32;
        if x >= rect.min.x && x <= rect.max.x {
            painter.line_segment(
                [egui::pos2(x, rect.min.y + 8.0), egui::pos2(x, rect.max.y)],
                egui::Stroke::new(1.0, Color32::from_rgb(70, 75, 85)),
            );
            painter.text(
                egui::pos2(x + 3.0, rect.min.y + 4.0),
                egui::Align2::LEFT_TOP,
                format_secs(t),
                egui::FontId::proportional(10.0),
                Color32::from_rgb(160, 165, 175),
            );
        }
        t += interval;
    }
}

fn nice_interval(target: f64) -> f64 {
    // Round to 1, 2, 5, 10, 15, 30, 60, 120, 300, 600, ...
    const STEPS: &[f64] = &[1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0, 3600.0];
    for &s in STEPS {
        if s >= target {
            return s;
        }
    }
    *STEPS.last().unwrap()
}

fn format_secs(s: f64) -> String {
    let total = s.max(0.0);
    let m = (total / 60.0).floor() as u64;
    let sec = total - (m as f64) * 60.0;
    if m > 0 {
        format!("{}m{:.0}s", m, sec)
    } else {
        format!("{:.0}s", sec)
    }
}

fn snap_to_edges(
    t: f64,
    snapshot: &TimelineSnap,
    exclude_id: Option<edt_core::id::Id>,
    zoom: f64,
) -> Option<f64> {
    let threshold = SNAP_THRESHOLD_PX / zoom;
    let mut candidates: Vec<f64> = Vec::new();
    candidates.push(0.0);
    candidates.push(snapshot.playhead);
    for track in &snapshot.tracks {
        for clip in &track.clips {
            if Some(clip.id) == exclude_id {
                continue;
            }
            candidates.push(clip.start);
            candidates.push(clip.end);
        }
    }
    let mut best: Option<(f64, f64)> = None;
    for &c in &candidates {
        let dist = (c - t).abs();
        if dist < threshold {
            if best.map(|(_, bd)| dist < bd).unwrap_or(true) {
                best = Some((c, dist));
            }
        }
    }
    best.map(|(v, _)| v)
}

// Snapshot types — copy the bits we need so we don't hold a lock during paint.
#[derive(Debug, Clone)]
struct TrackSnap {
    id: edt_core::id::Id,
    name: String,
    kind: edt_core::timeline::TrackKind,
    muted: bool,
    locked: bool,
    clips: Vec<ClipSnap>,
}

#[derive(Debug, Clone)]
struct ClipSnap {
    id: edt_core::id::Id,
    name: String,
    start: f64,
    end: f64,
    label: u8,
}

#[derive(Debug, Clone)]
struct TimelineSnap {
    tracks: Vec<TrackSnap>,
    zoom: f64,
    scroll: f64,
    playhead: f64,
    selection: Selection,
    duration: f64,
}
