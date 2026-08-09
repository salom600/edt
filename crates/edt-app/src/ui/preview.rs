//! Preview player panel — shows the current timeline frame with
//! transport controls (play/pause, scrub, frame step).

use crate::app::EdtApp;
use crate::ui::{panel_header, TEXT_DIM, WIDGET_BG};
use eframe::egui;
use egui::{Color32, Context, Sense, Ui};
use std::collections::HashMap;
use std::time::Instant;

/// Cache of decoded preview frames keyed by their timeline frame index.
/// The cache holds at most `CAP` entries (LRU eviction is approximate —
/// we just drop everything when full for simplicity).
#[derive(Default)]
pub struct PreviewCache {
    pub frames: HashMap<u64, egui::TextureHandle>,
    pub last_request: Option<u64>,
    pub last_request_at: Option<Instant>,
}

#[allow(dead_code)]
const CAP: usize = 32;

pub fn render(app: &mut EdtApp, ctx: &Context, ui: &mut Ui) {
    panel_header(ui, "Preview");

    // Snapshot what we need from state.
    let (playhead, dur, fps, w, h, is_playing, has_clips, top_clip) = {
        let s = app.state.read();
        let playhead = s.playhead.0;
        let dur = s.project.duration().0;
        let fps = s.project.settings.fps;
        let w = s.project.settings.width;
        let h = s.project.settings.height;
        let is_playing = s.play_state == crate::state::PlayState::Playing;
        let has_clips = dur > 0.0;
        // Find the topmost active video clip and resolve its source file.
        let mut top: Option<(std::path::PathBuf, f64)> = None;
        for (track, clip) in s.project.timeline.active_clips_at(s.playhead) {
            if track.kind != edt_core::timeline::TrackKind::Video || track.muted || clip.muted {
                continue;
            }
            if let Some(asset) = s.project.asset(clip.source.asset_id) {
                if let Some(src_t) = clip.timeline_to_source(s.playhead) {
                    top = Some((asset.editing_path().to_path_buf(), src_t.0));
                    break;
                }
            }
        }
        (playhead, dur, fps, w, h, is_playing, has_clips, top)
    };
    let top_clip_path = top_clip.as_ref().map(|(p, _)| p.clone());
    let top_clip_source_t = top_clip.as_ref().map(|(_, t)| *t);

    // ---- Preview canvas ----
    let available = ui.available_size();
    let transport_h = 56.0;
    let canvas_h = (available.y - transport_h).max(120.0);
    let canvas_w = available.x;

    let (canvas_rect, _) = ui.allocate_exact_size(egui::vec2(canvas_w, canvas_h), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(canvas_rect, 0.0, Color32::from_rgb(8, 10, 14));

    // Compute letterboxed destination rect.
    let aspect_src = w as f32 / h.max(1) as f32;
    let aspect_dst = canvas_w / canvas_h;
    let (dst_w, dst_h) = if aspect_src > aspect_dst {
        (canvas_w, canvas_w / aspect_src)
    } else {
        (canvas_h * aspect_src, canvas_h)
    };
    let dst_rect = egui::Rect::from_center_size(canvas_rect.center(), egui::vec2(dst_w, dst_h));

    // Frame to display: look up cached frame for current playhead.
    let frame_idx = (playhead * fps).round() as u64;
    if let Some(handle) = app.preview_cache.frames.get(&frame_idx) {
        painter.image(
            handle.id(),
            dst_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else if !has_clips {
        painter.text(
            canvas_rect.center(),
            egui::Align2::CENTER_CENTER,
            "No clips on timeline",
            egui::FontId::proportional(14.0),
            TEXT_DIM,
        );
    } else {
        painter.text(
            canvas_rect.center(),
            egui::Align2::CENTER_CENTER,
            "Decoding frame…",
            egui::FontId::proportional(13.0),
            TEXT_DIM,
        );
        // Throttle: only request a frame if we haven't requested one for
        // this time in the last 250ms.
        let now = Instant::now();
        let should_request = match app.preview_cache.last_request {
            Some(t) if t == frame_idx => false,
            _ => match app.preview_cache.last_request_at {
                Some(prev) => now.duration_since(prev).as_millis() > 250,
                None => true,
            },
        };
        if should_request {
            if let Some(path) = top_clip_path {
                let src_t = top_clip_source_t.unwrap_or(0.0);
                let _ = app
                    .jobs
                    .tx
                    .send(crate::background::JobRequest::PreviewFrame {
                        path,
                        time: edt_core::time::Time(src_t),
                        max_width: 640,
                    });
                app.preview_cache.last_request = Some(frame_idx);
                app.preview_cache.last_request_at = Some(now);
            }
        }
    }

    // Time overlay.
    let time_label = format!("{:.2}s / {:.2}s", playhead, dur);
    painter.text(
        egui::pos2(canvas_rect.max.x - 8.0, canvas_rect.min.y + 8.0),
        egui::Align2::RIGHT_TOP,
        &time_label,
        egui::FontId::proportional(11.0),
        Color32::from_rgb(180, 185, 195),
    );

    // ---- Transport bar ----
    let (transport_rect, _) =
        ui.allocate_exact_size(egui::vec2(canvas_w, transport_h), Sense::click());
    let _transport = ui.painter().with_clip_rect(transport_rect);

    ui.painter().rect_filled(transport_rect, 4.0, WIDGET_BG);

    #[allow(deprecated)]
    ui.allocate_ui_at_rect(transport_rect.shrink2(egui::vec2(8.0, 4.0)), |ui| {
        ui.horizontal_centered(|ui| {
            // Skip to start.
            if ui.button("⏮").on_hover_text("Skip to start").clicked() {
                app.state.write().playhead = edt_core::time::Time::ZERO;
            }
            // Frame back.
            if ui.button("◀|").on_hover_text("Previous frame").clicked() {
                app.state.write().nudge_playhead(-1.0 / fps);
            }
            // Play/pause.
            let play_label = if is_playing { "⏸" } else { "▶" };
            if ui
                .button(play_label)
                .on_hover_text("Play/Pause (Space)")
                .clicked()
            {
                app.state.write().toggle_play();
            }
            // Frame forward.
            if ui.button("|▶").on_hover_text("Next frame").clicked() {
                app.state.write().nudge_playhead(1.0 / fps);
            }
            // Skip to end.
            if ui.button("⏭").on_hover_text("Skip to end").clicked() {
                let mut s = app.state.write();
                s.playhead = edt_core::time::Time(s.project.duration().0);
            }
            ui.separator();

            // Scrub slider.
            let mut t = playhead;
            let slider = egui::Slider::new(&mut t, 0.0..=dur.max(0.001))
                .show_value(true)
                .text("s")
                .clamping(egui::SliderClamping::Always);
            if ui.add(slider).changed() {
                app.state.write().playhead = edt_core::time::Time(t);
            }
        });
    });

    // If playing, schedule a repaint so the playhead advances.
    if is_playing {
        ctx.request_repaint();
    }

    let _ = top_clip_source_t;
}
