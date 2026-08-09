//! Top menu bar (File / Edit / View / Help) and status bar.

use crate::app::EdtApp;
use crate::state::Selection;
use eframe::egui;
use egui::{Context, Ui};

#[derive(Debug, Clone, Default)]
pub struct MenuState {
    pub show_export: bool,
    pub show_about: bool,
    pub show_settings: bool,
    pub show_known_issues: bool,
}

pub fn render(app: &mut EdtApp, ctx: &Context, ui: &mut Ui) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("New Project").clicked() {
                app.new_project();
                ui.close_menu();
            }
            if ui.button("Open Project…").clicked() {
                app.open_project_dialog();
                ui.close_menu();
            }
            if ui.button("Save Project").clicked() {
                app.save_project(false);
                ui.close_menu();
            }
            if ui.button("Save Project As…").clicked() {
                app.save_project(true);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Import Media…").clicked() {
                app.import_media_dialog();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Export…").clicked() {
                app.menu.show_export = true;
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                ui.close_menu();
            }
        });

        ui.menu_button("Edit", |ui| {
            if ui
                .add_enabled(app.undo.can_undo(), egui::Button::new("Undo"))
                .clicked()
            {
                app.undo.undo(&app.state);
                ui.close_menu();
            }
            if ui
                .add_enabled(app.undo.can_redo(), egui::Button::new("Redo"))
                .clicked()
            {
                app.undo.redo(&app.state);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Split at Playhead (S)").clicked() {
                app.split_at_playhead();
                ui.close_menu();
            }
            if ui.button("Delete Selected (Del)").clicked() {
                app.delete_selected();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Deselect").clicked() {
                app.state.write().selection = Selection::None;
                ui.close_menu();
            }
        });

        ui.menu_button("View", |ui| {
            ui.label("Zoom controls are in the timeline toolbar.");
            ui.separator();
            if ui.button("Fit Timeline").clicked() {
                let mut s = app.state.write();
                let dur = s.project.duration().0.max(1.0);
                s.timeline_zoom = 800.0 / dur;
                s.timeline_scroll = 0.0;
                drop(s);
                ui.close_menu();
            }
            if ui.button("Reset Zoom").clicked() {
                app.state.write().timeline_zoom = 50.0;
                ui.close_menu();
            }
        });

        ui.menu_button("Playback", |ui| {
            if ui.button("Play/Pause (Space)").clicked() {
                app.state.write().toggle_play();
                ui.close_menu();
            }
            if ui.button("Stop").clicked() {
                app.state.write().stop();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Step Forward (→)").clicked() {
                let fps = app.state.read().project.settings.fps;
                app.state.write().nudge_playhead(1.0 / fps);
                ui.close_menu();
            }
            if ui.button("Step Backward (←)").clicked() {
                let fps = app.state.read().project.settings.fps;
                app.state.write().nudge_playhead(-1.0 / fps);
                ui.close_menu();
            }
        });

        ui.menu_button("Help", |ui| {
            if ui.button("About edt").clicked() {
                app.menu.show_about = true;
                ui.close_menu();
            }
            if ui.button("Known Issues").clicked() {
                app.menu.show_known_issues = true;
                ui.close_menu();
            }
            ui.separator();
            ui.hyperlink_to(
                "Documentation",
                "https://github.com/salom600/edt/blob/main/docs/architecture.md",
            );
            ui.hyperlink_to("Report a Bug", "https://github.com/salom600/edt/issues");
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let s = app.state.read();
            let dirty = if s.dirty { " ●" } else { "" };
            let proj_name = s.project.settings.name.clone();
            ui.label(format!("{proj_name}{dirty}"));
            ui.separator();
            let playhead = s.playhead.0;
            let dur = s.project.duration().0;
            ui.label(format!("{playhead:.2}s / {dur:.2}s"));
            ui.separator();
            if let Some(err) = &s.last_error {
                ui.colored_label(egui::Color32::from_rgb(248, 113, 113), format!("⚠ {err}"));
            }
        });
    });
}
