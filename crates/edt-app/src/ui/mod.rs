//! UI panels for the edt editor shell.

pub mod menubar;
pub mod media_pool;
pub mod timeline;
pub mod preview;
pub mod inspector;
pub mod export_dialog;
pub mod styles;

use crate::app::EdtApp;
use egui::{Color32, Context, Ui};

/// The edt dark theme palette.
pub const BG: Color32 = Color32::from_rgb(20, 22, 26);
pub const PANEL_BG: Color32 = Color32::from_rgb(28, 30, 36);
pub const WIDGET_BG: Color32 = Color32::from_rgb(38, 40, 48);
pub const WIDGET_HOVER: Color32 = Color32::from_rgb(48, 50, 60);
pub const ACCENT: Color32 = Color32::from_rgb(96, 165, 250);
pub const TEXT: Color32 = Color32::from_rgb(220, 224, 230);
pub const TEXT_DIM: Color32 = Color32::from_rgb(140, 145, 155);
pub const TRACK_VIDEO: Color32 = Color32::from_rgb(80, 130, 200);
pub const TRACK_AUDIO: Color32 = Color32::from_rgb(120, 180, 100);
pub const PLAYHEAD: Color32 = Color32::from_rgb(250, 200, 80);
pub const SELECTION: Color32 = Color32::from_rgb(250, 250, 100);

/// Apply the edt dark theme to an egui context.
pub fn apply_theme(ctx: &Context) {
    let mut style: egui::Style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(8);
    ctx.set_style(style);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = PANEL_BG;
    visuals.extreme_bg_color = Color32::from_rgb(12, 14, 18);
    visuals.faint_bg_color = WIDGET_BG;
    visuals.widgets.noninteractive.bg_fill = WIDGET_BG;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_DIM);
    visuals.widgets.inactive.bg_fill = WIDGET_BG;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT);
    visuals.widgets.hovered.bg_fill = WIDGET_HOVER;
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TEXT);
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.selection.bg_fill = ACCENT.linear_multiply(0.4);
    visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT;
    ctx.set_visuals(visuals);
}

/// Trait helper: render a top-aligned panel header.
pub fn panel_header(ui: &mut Ui, title: &str) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).strong().color(TEXT).size(13.0));
    });
    ui.separator();
}

/// Reference to the EdtApp for panel rendering.
pub trait AppRef {
    fn app(&self) -> &EdtApp;
}
