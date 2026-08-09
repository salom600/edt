//! Media pool panel — list of imported assets.

use crate::app::EdtApp;
use crate::state::Selection;
use crate::ui::{label_color, panel_header, ACCENT, TEXT_DIM, WIDGET_BG};
use eframe::egui;
use egui::{Color32, Context, Response, Sense, Ui};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Cached thumbnails keyed by asset id. The app owns this so it survives
/// panel rebuilds.
pub type ThumbCache = HashMap<edt_core::id::Id, egui::TextureHandle>;

pub fn render(app: &mut EdtApp, _ctx: &Context, ui: &mut Ui) {
    panel_header(ui, "Media Pool");

    // Toolbar: import button + asset count.
    ui.horizontal(|ui| {
        if ui.button("＋ Import").clicked() {
            app.import_media_dialog();
        }
        ui.separator();
        let n = app.state.read().project.assets.len();
        ui.label(format!("{n} asset(s)"));
    });
    ui.separator();

    // Asset list.
    let assets: Vec<(edt_core::id::Id, String, Option<String>, u8, bool)> = {
        let s = app.state.read();
        s.project
            .assets
            .iter()
            .map(|a| {
                let meta_summary = match a.kind() {
                    edt_core::media::MediaKind::Video => a
                        .video_info()
                        .map(|v| format!("{}×{} · {:.2}fps", v.width, v.height, v.fps)),
                    edt_core::media::MediaKind::Audio => a
                        .audio_info()
                        .map(|a| format!("{}Hz · {}ch", a.sample_rate, a.channels)),
                    edt_core::media::MediaKind::Image => Some("Image".into()),
                    edt_core::media::MediaKind::Unknown => Some("Unknown".into()),
                };
                (a.id, a.name.clone(), meta_summary, a.label, a.offline)
            })
            .collect()
    };

    if assets.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.label(egui::RichText::new("No media imported").color(TEXT_DIM));
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Click \"Import\" above")
                    .color(TEXT_DIM)
                    .small(),
            );
        });
    }

    let mut clicked_id: Option<edt_core::id::Id> = None;
    let mut double_clicked_id: Option<edt_core::id::Id> = None;

    let scroll_area = egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for (id, name, meta, label, offline) in &assets {
                let label_color = label_color(*label);
                let row_resp = ui.horizontal(|ui| {
                    // Label color stripe.
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(4.0, 56.0), Sense::hover());
                    ui.painter().rect_filled(rect, 0.0, label_color);

                    // Thumbnail (or placeholder).
                    let thumb_size = egui::vec2(80.0, 56.0);
                    let (thumb_rect, _) = ui.allocate_exact_size(thumb_size, Sense::hover());
                    if let Some(handle) = app.thumbs.get(id) {
                        ui.painter().image(
                            handle.id(),
                            thumb_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    } else {
                        ui.painter().rect_filled(thumb_rect, 2.0, WIDGET_BG);
                        ui.painter().text(
                            thumb_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "…",
                            egui::FontId::proportional(20.0),
                            TEXT_DIM,
                        );
                    }

                    ui.vertical(|ui| {
                        ui.add_space(2.0);
                        let name_color = if *offline {
                            Color32::from_rgb(248, 113, 113)
                        } else {
                            Color32::from_rgb(220, 224, 230)
                        };
                        ui.label(egui::RichText::new(name).color(name_color).strong());
                        if let Some(m) = meta {
                            ui.label(egui::RichText::new(m).color(TEXT_DIM).small());
                        }
                        if *offline {
                            ui.label(
                                egui::RichText::new("⚠ offline")
                                    .color(Color32::from_rgb(248, 113, 113))
                                    .small(),
                            );
                        }
                    });
                });

                let resp = row_resp.response.interact(Sense::click());
                if resp.clicked() {
                    clicked_id = Some(*id);
                }
                if resp.double_clicked() {
                    double_clicked_id = Some(*id);
                }
                ui.separator();
            }
        });

    // Apply selection changes after the iteration to avoid borrow issues.
    if let Some(id) = clicked_id {
        app.state.write().selection = Selection::None;
        app.pending_drag_asset = Some(id);
    }
    if let Some(id) = double_clicked_id {
        // Double-click: append to first video track at end of timeline.
        let track_id = app
            .state
            .read()
            .first_track_of_kind(edt_core::timeline::TrackKind::Video);
        if let Some(track_id) = track_id {
            let start = {
                let s = app.state.read();
                s.project
                    .timeline
                    .track(track_id)
                    .map(|t| t.duration())
                    .unwrap_or_default()
            };
            let next_id = app.state.next_id();
            app.state
                .write()
                .add_clip_from_asset(id, track_id, start, next_id);
        }
    }

    let _ = scroll_area;
}

/// Helper to convert an `RgbaImage` into an `egui::TextureHandle`.
pub fn upload_texture(ctx: &Context, image: &image::RgbaImage) -> egui::TextureHandle {
    let color_image = egui::ColorImage {
        size: [image.width() as usize, image.height() as usize],
        pixels: image
            .pixels()
            .map(|p| egui::Color32::from_rgba_unmultiplied(p.0[0], p.0[1], p.0[2], p.0[3]))
            .collect(),
    };
    ctx.load_texture("thumb", color_image, egui::TextureOptions::LINEAR)
}
