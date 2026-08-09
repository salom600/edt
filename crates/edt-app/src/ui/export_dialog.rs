//! Export dialog — modal window for configuring and running an export.

use crate::app::EdtApp;
use crate::ui::{ACCENT, TEXT, TEXT_DIM, WIDGET_BG};
use eframe::egui;
use egui::{Color32, Context, Ui};
use edt_core::export::{ExportAudioCodec, ExportFormat, ExportSettings, ExportVideoCodec};
use edt_export::{ExportOptions, ExportStrategy, ProgressUpdate};
use std::path::PathBuf;
use std::sync::Arc;

pub struct ExportDialogState {
    pub settings: ExportSettings,
    pub output_path: String,
    pub in_progress: bool,
    pub progress: Option<ProgressUpdate>,
    pub last_result: Option<Result<(), String>>,
    pub show: bool,
}

impl Default for ExportDialogState {
    fn default() -> Self {
        let mut path = std::env::current_dir().unwrap_or_default();
        path.push("export.mp4");
        Self {
            settings: ExportSettings::default(),
            output_path: path.to_string_lossy().into_owned(),
            in_progress: false,
            progress: None,
            last_result: None,
            show: false,
        }
    }
}

pub fn render(app: &mut EdtApp, ctx: &Context) {
    if !app.export_dialog.show {
        return;
    }
    let mut open = true;
    let title = if app.export_dialog.in_progress {
        "Exporting…"
    } else {
        "Export"
    };
    egui::Window::new(title)
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(480.0)
        .show(ctx, |ui| {
            if app.export_dialog.in_progress {
                render_progress(app, ui);
            } else {
                render_form(app, ui);
            }
        });
    app.export_dialog.show = open;
}

fn render_form(app: &mut EdtApp, ui: &mut Ui) {
    let s = &mut app.export_dialog.settings;
    ui.label(egui::RichText::new("Output").color(TEXT).strong());
    ui.horizontal(|ui| {
        ui.label("Path");
        ui.text_edit_singleline(&mut app.export_dialog.output_path);
        if ui.button("Browse…").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("MP4", &["mp4"])
                .add_filter("MKV", &["mkv"])
                .add_filter("MOV", &["mov"])
                .add_filter("WebM", &["webm"])
                .set_file_name("export.mp4")
                .save_file()
            {
                app.export_dialog.output_path = path.to_string_lossy().into_owned();
            }
        }
    });
    ui.separator();

    ui.label(egui::RichText::new("Video").color(TEXT).strong());
    egui::Grid::new("export_video_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Format");
            let mut fmt = s.format;
            egui::ComboBox::from_id_source("fmt")
                .selected_text(format!("{:?}", fmt))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut fmt, ExportFormat::Mp4, "MP4");
                    ui.selectable_value(&mut fmt, ExportFormat::Mkv, "MKV");
                    ui.selectable_value(&mut fmt, ExportFormat::Mov, "MOV");
                    ui.selectable_value(&mut fmt, ExportFormat::Webm, "WebM");
                });
            s.format = fmt;
            ui.end_row();

            ui.label("Codec");
            egui::ComboBox::from_id_source("vcodec")
                .selected_text(format!("{:?}", s.video_codec))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut s.video_codec, ExportVideoCodec::H264, "H.264 (libx264)");
                    ui.selectable_value(&mut s.video_codec, ExportVideoCodec::H265, "H.265 (libx265)");
                    ui.selectable_value(&mut s.video_codec, ExportVideoCodec::Av1, "AV1 (libsvtav1)");
                    ui.selectable_value(&mut s.video_codec, ExportVideoCodec::Vp9, "VP9 (libvpx-vp9)");
                    ui.selectable_value(&mut s.video_codec, ExportVideoCodec::Prores, "ProRes");
                });
            ui.end_row();

            ui.label("Resolution");
            let mut res = format!("{}×{}", s.width, s.height);
            ui.text_edit_singleline(&mut res);
            if let Some((w, h)) = parse_resolution(&res) {
                s.width = w;
                s.height = h;
            }
            ui.end_row();

            ui.label("Framerate");
            ui.add(egui::Slider::new(&mut s.fps, 1.0..=120.0).step_by(1.0).text("fps"));
            ui.end_row();

            ui.label("CRF (quality)");
            ui.add(egui::Slider::new(&mut s.crf, 0..=51).text("lower = better"));
            ui.end_row();
        });
    ui.separator();

    ui.label(egui::RichText::new("Audio").color(TEXT).strong());
    egui::Grid::new("export_audio_grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Codec");
            egui::ComboBox::from_id_source("acodec")
                .selected_text(format!("{:?}", s.audio_codec))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut s.audio_codec, ExportAudioCodec::Aac, "AAC");
                    ui.selectable_value(&mut s.audio_codec, ExportAudioCodec::Opus, "Opus");
                    ui.selectable_value(&mut s.audio_codec, ExportAudioCodec::PcmS16le, "PCM S16 LE");
                });
            ui.end_row();

            ui.label("Bitrate");
            let mut br = (s.audio_bitrate as f64) / 1000.0;
            ui.add(egui::Slider::new(&mut br, 32.0..=512.0).text("kbps"));
            s.audio_bitrate = (br * 1000.0) as u64;
            ui.end_row();

            ui.label("Sample rate");
            let sr_opts = [44100, 48000, 96000];
            egui::ComboBox::from_id_source("sr")
                .selected_text(format!("{} Hz", s.audio_sample_rate))
                .show_ui(ui, |ui| {
                    for &sr in &sr_opts {
                        ui.selectable_value(&mut s.audio_sample_rate, sr, format!("{} Hz", sr));
                    }
                });
            ui.end_row();

            ui.label("Channels");
            let ch_opts = [(1, "Mono"), (2, "Stereo"), (6, "5.1"), (8, "7.1")];
            egui::ComboBox::from_id_source("ch")
                .selected_text(format!("{}", s.audio_channels))
                .show_ui(ui, |ui| {
                    for (n, label) in ch_opts {
                        ui.selectable_value(&mut s.audio_channels, n, format!("{} ({})", n, label));
                    }
                });
            ui.end_row();
        });
    ui.separator();

    ui.label(egui::RichText::new("Presets").color(TEXT).strong());
    ui.horizontal(|ui| {
        if ui.button("720p H.264").clicked() {
            *s = ExportSettings::preset_720p_h264();
        }
        if ui.button("1080p H.264").clicked() {
            *s = ExportSettings::preset_1080p_h264();
        }
        if ui.button("4K H.265").clicked() {
            *s = ExportSettings::preset_4k_h265();
        }
    });

    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("Cancel").clicked() {
            app.export_dialog.show = false;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(egui::Button::new("Start Export").fill(ACCENT)).clicked() {
                start_export(app);
            }
        });
    });

    // Last result message.
    if let Some(res) = &app.export_dialog.last_result {
        ui.separator();
        match res {
            Ok(()) => {
                ui.colored_label(Color32::from_rgb(132, 204, 22), "✓ Export succeeded");
            }
            Err(e) => {
                ui.colored_label(Color32::from_rgb(248, 113, 113), format!("✗ Export failed: {e}"));
            }
        }
    }
}

fn render_progress(app: &mut EdtApp, ui: &mut Ui) {
    let prog = app
        .export_dialog
        .progress
        .as_ref()
        .map(|p| p.snapshot())
        .unwrap_or(edt_export::ExportProgress {
            frames_done: 0,
            frames_total: 0,
            current_time_secs: 0.0,
            total_time_secs: 0.0,
        });

    ui.label(format!("Exporting to: {}", app.export_dialog.output_path));
    ui.add_space(8.0);
    let frac = if prog.frames_total == 0 {
        0.0
    } else {
        prog.fraction()
    };
    ui.add(egui::ProgressBar::new(frac).text(format!(
        "{}%  ({}/{})",
        prog.percent(),
        prog.frames_done,
        prog.frames_total
    )));
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        if ui.button("Cancel").clicked() {
            if let Some(p) = &app.export_dialog.progress {
                p.cancel();
            }
        }
    });
}

fn start_export(app: &mut EdtApp) {
    let project = app.state.read().project.clone();
    let settings = app.export_dialog.settings.clone();
    let output_path = PathBuf::from(&app.export_dialog.output_path);
    let options = ExportOptions {
        settings: settings.clone(),
        output_path: output_path.clone(),
        strategy: ExportStrategy::Concat,
    };
    let total_frames = (project.duration().0 * settings.fps).ceil() as u64;
    let progress = ProgressUpdate::new(total_frames);
    app.export_dialog.progress = Some(progress.clone());
    app.export_dialog.in_progress = true;
    app.export_dialog.last_result = None;

    let _ = app.jobs.tx.send(crate::background::JobRequest::Export {
        project,
        options,
        strategy_hint: None,
        progress,
    });
}

fn parse_resolution(s: &str) -> Option<(u32, u32)> {
    let (w, h) = s.split_once('×').or_else(|| s.split_once('x'))?;
    let w: u32 = w.trim().parse().ok()?;
    let h: u32 = h.trim().parse().ok()?;
    if w == 0 || h == 0 || w > 16384 || h > 16384 {
        return None;
    }
    Some((w, h))
}
