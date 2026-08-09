//! Background job orchestration.
//!
//! The UI thread should never block on ffmpeg. Long-running operations
//! (probing, thumbnail generation, frame extraction for preview, export)
//! are dispatched to a worker thread and their results flow back via
//! channels polled on each UI frame.

use crate::state::EditorState;
use crossbeam_channel::{unbounded, Receiver, Sender};
use edt_core::id::Id;
use edt_core::media::{MediaAsset, MediaMetadata};
use edt_core::project::Project;
use edt_core::time::Time;
use edt_export::{ExportOptions, ExportStrategy, ProgressUpdate};
use image::RgbaImage;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

/// Messages from background jobs to the UI thread.
#[derive(Debug)]
pub enum JobResult {
    /// A media probe completed.
    Probe {
        path: PathBuf,
        name: String,
        result: Result<MediaMetadata, String>,
    },
    /// A thumbnail finished rendering.
    Thumbnail {
        asset_id: Id,
        result: Result<RgbaImage, String>,
    },
    /// A preview frame finished decoding.
    PreviewFrame {
        time: Time,
        result: Result<RgbaImage, String>,
    },
    /// An export finished.
    Export { result: Result<(), String> },
    /// Export progress tick.
    ExportProgress { done: u64, total: u64 },
}

/// Owns the worker thread and the inbound channel.
pub struct BackgroundJobs {
    pub tx: Sender<JobRequest>,
    pub rx: Receiver<JobResult>,
}

impl BackgroundJobs {
    pub fn new(state: Arc<EditorState>) -> Self {
        let (tx_req, rx_req) = unbounded::<JobRequest>();
        let (tx_res, rx_res) = unbounded::<JobResult>();
        thread::spawn(move || {
            worker_loop(rx_req, tx_res, state);
        });
        Self {
            tx: tx_req,
            rx: rx_res,
        }
    }
}

/// Requests dispatched to the worker.
#[derive(Debug)]
pub enum JobRequest {
    Probe {
        path: PathBuf,
        name: String,
    },
    Thumbnail {
        asset: MediaAsset,
    },
    PreviewFrame {
        path: PathBuf,
        time: Time,
        max_width: u32,
    },
    Export {
        project: Project,
        options: ExportOptions,
        strategy_hint: Option<ExportStrategy>,
        progress: ProgressUpdate,
    },
    Shutdown,
}

fn worker_loop(rx: Receiver<JobRequest>, tx: Sender<JobResult>, state: Arc<EditorState>) {
    while let Ok(req) = rx.recv() {
        match req {
            JobRequest::Shutdown => break,
            JobRequest::Probe { path, name } => {
                let result = edt_media::probe(&path)
                    .map(|r| r.metadata)
                    .map_err(|e| e.to_string());
                let _ = tx.send(JobResult::Probe { path, name, result });
            }
            JobRequest::Thumbnail { asset } => {
                let result = edt_media::generate_thumbnail(&asset).map_err(|e| e.to_string());
                let _ = tx.send(JobResult::Thumbnail {
                    asset_id: asset.id,
                    result: result.ok().flatten().ok_or_else(|| "no thumbnail".into()),
                });
            }
            JobRequest::PreviewFrame {
                path,
                time,
                max_width,
            } => {
                let result = edt_media::extract_frame(&path, time.0, Some(max_width))
                    .map_err(|e| e.to_string());
                let _ = tx.send(JobResult::PreviewFrame { time, result });
            }
            JobRequest::Export {
                project,
                options,
                strategy_hint: _,
                progress,
            } => {
                let p = progress.clone();
                let result =
                    edt_export::export_project(&project, &options, p).map_err(|e| e.to_string());
                let _ = tx.send(JobResult::Export { result });
                // Suppress unused state warning when not needed.
                let _ = &state;
            }
        }
    }
    tracing::info!("background worker exiting");
}
