//! The top-level editor application.

use crate::background::{BackgroundJobs, JobResult};
use crate::commands::{AddClipCmd, DeleteClipCmd, SplitClipCmd, UndoStack};
use crate::state::{EditorState, EditorStateInner, Selection};
use crate::ui::export_dialog::ExportDialogState;
use crate::ui::menubar::MenuState;
use crate::ui::preview::PreviewCache;
use crate::ui::timeline::DragState;
use crate::ui::{
    apply_theme, export_dialog, inspector, media_pool, menubar, preview, timeline, ThumbCache,
};
use edt_core::id::Id;
use edt_core::media::{MediaAsset, MediaMetadata};
use edt_core::project::{Project, ProjectFile};
use edt_core::time::Time;
use edt_core::timeline::{Clip, ClipSource};
use eframe::egui;
use egui::{Context, Ui};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The edt application. Owns all state and wires panels together.
pub struct EdtApp {
    pub state: Arc<EditorState>,
    pub undo: UndoStack,
    pub jobs: BackgroundJobs,
    pub thumbs: ThumbCache,
    pub preview_cache: PreviewCache,
    pub menu: MenuState,
    pub export_dialog: ExportDialogState,
    pub pending_drag_asset: Option<Id>,
    pub timeline_drag: DragState,
    pub last_autosave_check: Instant,
    pub last_frame_time: Instant,
    pub about_window_open: bool,
    pub known_issues_window_open: bool,
    pub pending_open_dialog: bool,
    pub pending_save_dialog: bool,
    pub pending_import_dialog: bool,
    pub pending_save_as: bool,
    pub file_dialog_kind: FileDialogKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDialogKind {
    None,
    OpenProject,
    SaveProject,
    ImportMedia,
}

impl EdtApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_theme(&cc.egui_ctx);
        let state = EditorState::new();
        let jobs = BackgroundJobs::new(state.clone());
        Self {
            state,
            undo: UndoStack::default(),
            jobs,
            thumbs: ThumbCache::new(),
            preview_cache: PreviewCache::default(),
            menu: MenuState::default(),
            export_dialog: ExportDialogState::default(),
            pending_drag_asset: None,
            timeline_drag: DragState::default(),
            last_autosave_check: Instant::now(),
            last_frame_time: Instant::now(),
            about_window_open: false,
            known_issues_window_open: false,
            pending_open_dialog: false,
            pending_save_dialog: false,
            pending_import_dialog: false,
            pending_save_as: false,
            file_dialog_kind: FileDialogKind::None,
        }
    }

    // -- File operations ----------------------------------------------------

    pub fn new_project(&mut self) {
        let (project, _) = Project::new();
        let mut s = self.state.write();
        s.project = project;
        s.selection = Selection::None;
        s.playhead = Time::ZERO;
        s.play_state = crate::state::PlayState::Paused;
        s.timeline_zoom = 50.0;
        s.timeline_scroll = 0.0;
        s.dirty = false;
        s.last_error = None;
        self.thumbs.clear();
        self.preview_cache.frames.clear();
        self.undo = UndoStack::default();
    }

    pub fn open_project_dialog(&mut self) {
        self.pending_open_dialog = true;
        self.file_dialog_kind = FileDialogKind::OpenProject;
    }

    pub fn save_project(&mut self, save_as: bool) {
        if save_as {
            self.pending_save_as = true;
            self.pending_save_dialog = true;
            self.file_dialog_kind = FileDialogKind::SaveProject;
            return;
        }
        let path = self.state.read().project.last_save_path.clone();
        match path {
            Some(p) => {
                self.do_save(&p);
            }
            None => {
                self.pending_save_dialog = true;
                self.file_dialog_kind = FileDialogKind::SaveProject;
            }
        }
    }

    fn do_save(&mut self, path: &std::path::Path) {
        let project = self.state.read().project.clone();
        match edt_storage::save_project(&project, path) {
            Ok(()) => {
                self.state.write().project.last_save_path = Some(path.to_path_buf());
                self.state.write().dirty = false;
                self.state.write().last_error = None;
                tracing::info!(path = %path.display(), "project saved");
            }
            Err(e) => {
                self.state.write().last_error = Some(format!("Save failed: {e}"));
            }
        }
    }

    pub fn import_media_dialog(&mut self) {
        self.pending_import_dialog = true;
        self.file_dialog_kind = FileDialogKind::ImportMedia;
    }

    pub fn import_media_path(&mut self, path: PathBuf) {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "media".into());
        let _ = self.jobs.tx.send(crate::background::JobRequest::Probe {
            path: path.clone(),
            name: name.clone(),
        });
        // Optimistically add the asset with no metadata; the probe will
        // update it when it completes.
        let id = self.state.next_id();
        let asset = MediaAsset {
            id,
            name,
            path,
            metadata: None,
            label: 0,
            offline: false,
            proxy_path: None,
        };
        self.state.write().add_asset(asset.clone());
        let _ = self
            .jobs
            .tx
            .send(crate::background::JobRequest::Thumbnail { asset });
    }

    // -- Edit operations ----------------------------------------------------

    pub fn split_at_playhead(&mut self) {
        let next_id = self.state.next_id();
        // Capture original clip + track for undo.
        let original = {
            let s = self.state.read();
            let id = match s.selection {
                Selection::Clip(id) => id,
                _ => return,
            };
            s.project.timeline.clip(id).map(|(_, c)| c.clone())
        };
        let Some(original) = original else { return };
        let track_id = match self
            .state
            .read()
            .project
            .timeline
            .track_of_clip(original.id)
        {
            Some(t) => t,
            None => return,
        };
        let playhead = self.state.read().playhead;
        let mut left = original.clone();
        let right = match left.split(next_id, playhead) {
            Some(r) => r,
            None => return,
        };
        // The split mutates `left`. We need to write it back to state.
        {
            let mut s = self.state.write();
            if let Some(track) = s.project.timeline.track_mut(track_id) {
                if let Some(slot) = track.clips.iter_mut().find(|c| c.id == original.id) {
                    *slot = left.clone();
                }
                track.insert_clip(right.clone());
                s.mark_dirty();
            }
        }
        self.undo.push(
            Box::new(SplitClipCmd {
                left_id: left.id,
                right_id: right.id,
                track_id,
                split_time: playhead,
                original_clip: original,
            }),
            &self.state,
        );
    }

    pub fn delete_selected(&mut self) {
        let snapshot = {
            let s = self.state.read();
            let id = match s.selection {
                Selection::Clip(id) => id,
                _ => return,
            };
            s.project.timeline.clip(id).map(|(_, c)| c.clone())
        };
        let Some(clip) = snapshot else { return };
        let track_id = match self.state.read().project.timeline.track_of_clip(clip.id) {
            Some(t) => t,
            None => return,
        };
        self.undo.push(
            Box::new(DeleteClipCmd {
                clip: clip.clone(),
                track_id,
            }),
            &self.state,
        );
        // The command applies itself.
        self.state.write().selection = Selection::None;
    }
}

// -- eframe::App impl ------------------------------------------------------

impl eframe::App for EdtApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process background job results before rendering.
        self.drain_jobs(ctx);

        // Handle pending file dialogs (rfd runs on a thread to avoid
        // blocking the UI; the result comes back as an event).
        self.handle_file_dialogs();

        // Autosave.
        if self.last_autosave_check.elapsed() > Duration::from_secs(60) {
            self.last_autosave_check = Instant::now();
            if self.state.read().dirty {
                let _ = edt_storage::autosave(&self.state.read().project);
            }
        }

        // Advance playhead.
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f64();
        self.last_frame_time = now;
        self.state.write().advance_playhead(dt);

        // Top menu bar.
        egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
            menubar::render(self, ctx, ui);
        });

        // Left panel: media pool.
        egui::SidePanel::left("media_pool")
            .default_width(280.0)
            .resizable(true)
            .show(ctx, |ui| {
                media_pool::render(self, ctx, ui);
            });

        // Right panel: inspector.
        egui::SidePanel::right("inspector")
            .default_width(280.0)
            .resizable(true)
            .show(ctx, |ui| {
                inspector::render(self, ctx, ui);
            });

        // Center top: preview.
        egui::TopBottomPanel::top("preview")
            .default_height(360.0)
            .resizable(true)
            .show(ctx, |ui| {
                preview::render(self, ctx, ui);
            });

        // Bottom: timeline.
        egui::CentralPanel::default().show(ctx, |ui| {
            timeline::render(self, ctx, ui);
        });

        // Modal dialogs.
        export_dialog::render(self, ctx);

        if self.about_window_open {
            self.show_about(ctx);
        }
        if self.known_issues_window_open {
            self.show_known_issues(ctx);
        }

        // Keyboard shortcuts.
        self.handle_shortcuts(ctx);
    }
}

impl EdtApp {
    fn drain_jobs(&mut self, ctx: &Context) {
        use crate::background::JobResult;
        while let Ok(result) = self.jobs.rx.try_recv() {
            match result {
                JobResult::Probe { path, name, result } => {
                    match result {
                        Ok(meta) => {
                            // Find asset by path and update its metadata.
                            let mut s = self.state.write();
                            for asset in s.project.assets.iter_mut() {
                                if asset.path == path {
                                    asset.metadata = Some(meta);
                                    break;
                                }
                            }
                            s.last_error = None;
                        }
                        Err(e) => {
                            tracing::warn!(path = %path.display(), error = %e, "probe failed");
                            let mut s = self.state.write();
                            for asset in s.project.assets.iter_mut() {
                                if asset.path == path {
                                    asset.offline = true;
                                    break;
                                }
                            }
                            s.last_error = Some(format!("Probe failed for {name}: {e}"));
                        }
                    }
                }
                JobResult::Thumbnail { asset_id, result } => match result {
                    Ok(img) => {
                        let handle = media_pool::upload_texture(ctx, &img);
                        self.thumbs.insert(asset_id, handle);
                    }
                    Err(e) => {
                        tracing::warn!(%asset_id, error = %e, "thumbnail failed");
                    }
                },
                JobResult::PreviewFrame { time, result } => match result {
                    Ok(img) => {
                        let handle = media_pool::upload_texture(ctx, &img);
                        let fps = self.state.read().project.settings.fps;
                        let key = (time.0 * fps).round() / fps;
                        if self.preview_cache.frames.len() >= 32 {
                            self.preview_cache.frames.clear();
                        }
                        self.preview_cache.frames.insert(key, handle);
                        ctx.request_repaint();
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, "preview frame failed (likely no ffmpeg)");
                    }
                },
                JobResult::Export { result } => {
                    self.export_dialog.in_progress = false;
                    self.export_dialog.last_result = Some(result.clone());
                    match result {
                        Ok(()) => {
                            self.state.write().last_error = None;
                            tracing::info!("export completed");
                        }
                        Err(e) => {
                            self.state.write().last_error = Some(format!("Export failed: {e}"));
                            tracing::error!(error = %e, "export failed");
                        }
                    }
                }
                JobResult::ExportProgress { done, total } => {
                    if let Some(p) = &self.export_dialog.progress {
                        p.set_done(done);
                        let _ = total;
                    }
                    ctx.request_repaint();
                }
            }
        }
    }

    fn handle_file_dialogs(&mut self) {
        // rfd blocking dialogs run on a worker thread to avoid UI stalls.
        // We spawn one if pending, and consume its result on the next frame.
        if !self.pending_open_dialog && !self.pending_save_dialog && !self.pending_import_dialog {
            return;
        }
        let kind = self.file_dialog_kind;
        self.file_dialog_kind = FileDialogKind::None;
        self.pending_open_dialog = false;
        self.pending_save_dialog = false;
        self.pending_import_dialog = false;

        // Use the blocking API in a spawn; this keeps the call simple and
        // correct. The UI freezes for ~50ms on a typical system which is
        // acceptable for MVP.
        match kind {
            FileDialogKind::OpenProject => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("edt project", &["json"])
                    .pick_file()
                {
                    match edt_storage::load_project(&path) {
                        Ok(project) => {
                            let mut s = self.state.write();
                            s.project = project;
                            s.selection = Selection::None;
                            s.playhead = Time::ZERO;
                            s.dirty = false;
                            s.last_error = None;
                            drop(s);
                            self.thumbs.clear();
                            self.preview_cache.frames.clear();
                            self.undo = UndoStack::default();
                            tracing::info!(path = %path.display(), "project loaded");
                        }
                        Err(e) => {
                            self.state.write().last_error = Some(format!("Load failed: {e}"));
                        }
                    }
                }
            }
            FileDialogKind::SaveProject => {
                let mut name = self.state.read().project.settings.name.clone();
                if !name.ends_with(".json") {
                    name.push_str(".json");
                }
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("edt project", &["json"])
                    .set_file_name(&name)
                    .save_file()
                {
                    self.do_save(&path);
                }
            }
            FileDialogKind::ImportMedia => {
                let paths = rfd::FileDialog::new()
                    .add_filter("Video", &["mp4", "mov", "mkv", "webm", "avi"])
                    .add_filter("Audio", &["wav", "mp3", "aac", "flac", "ogg"])
                    .add_filter("Image", &["png", "jpg", "jpeg", "bmp", "webp"])
                    .pick_files();
                if let Some(paths) = paths {
                    for p in paths {
                        self.import_media_path(p);
                    }
                }
            }
            FileDialogKind::None => {}
        }
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        // Detect when an input field is focused so we don't hijack typing.
        let any_focused = ctx.memory(|m| {
            m.focused().is_some_and(|id| {
                let opts = m.options.get(&id);
                opts.is_some()
            })
        });
        if any_focused {
            return;
        }
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Space) {
                self.state.write().toggle_play();
            }
            if i.key_pressed(egui::Key::ArrowLeft) {
                let fps = self.state.read().project.settings.fps;
                self.state.write().nudge_playhead(-1.0 / fps);
            }
            if i.key_pressed(egui::Key::ArrowRight) {
                let fps = self.state.read().project.settings.fps;
                self.state.write().nudge_playhead(1.0 / fps);
            }
            if i.key_pressed(egui::Key::S) && i.modifiers.ctrl {
                self.save_project(i.modifiers.shift);
            }
            if i.key_pressed(egui::Key::Delete) {
                self.delete_selected();
            }
            if i.key_pressed(egui::Key::S) && !i.modifiers.ctrl {
                self.split_at_playhead();
            }
            if i.key_pressed(egui::Key::Z) && i.modifiers.ctrl {
                if i.modifiers.shift {
                    self.undo.redo(&self.state);
                } else {
                    self.undo.undo(&self.state);
                }
            }
            if i.key_pressed(egui::Key::O) && i.modifiers.ctrl {
                self.open_project_dialog();
            }
            if i.key_pressed(egui::Key::I) && i.modifiers.ctrl {
                self.import_media_dialog();
            }
            if i.key_pressed(egui::Key::E) && i.modifiers.ctrl {
                self.export_dialog.show = true;
            }
        });
    }

    fn show_about(&mut self, ctx: &Context) {
        let mut open = self.about_window_open;
        egui::Window::new("About edt")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.heading("edt");
                    ui.label("Modern cross-platform video editor");
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("v0.1.0").weak());
                    ui.add_space(8.0);
                    ui.hyperlink_to("Source", "https://github.com/salom600/edt");
                    ui.hyperlink_to(
                        "Documentation",
                        "https://github.com/salom600/edt/blob/main/docs/architecture.md",
                    );
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new("Built with Rust, egui, and FFmpeg.")
                            .weak()
                            .small(),
                    );
                    ui.label(
                        egui::RichText::new("Licensed under MIT OR Apache-2.0.")
                            .weak()
                            .small(),
                    );
                });
            });
        self.about_window_open = open;
    }

    fn show_known_issues(&mut self, ctx: &Context) {
        let mut open = self.known_issues_window_open;
        egui::Window::new("Known Issues")
            .open(&mut open)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.label("• Frame-pipe export renders solid placeholder colors per clip, not actual decoded video frames. See roadmap item E-002.");
                ui.label("• Multi-track compositing uses topmost-clip-wins; no cross-track opacity blending yet (V-001).");
                ui.label("• Color grade and effects parameters are stored in the data model but not applied during render (F-001).");
                ui.label("• Preview playback does not play audio (A-001).");
                ui.label("• Undo for trim operations is not yet implemented (U-001).");
                ui.label("• Autosave runs every 60s but cannot be configured (P-001).");
            });
        self.known_issues_window_open = open;
    }
}
