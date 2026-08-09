//! edt-app — UI shell crate root.
//!
//! The shell is organized around a single [`EdtApp`] struct that owns
//! all editor state and renders the four main panels (media pool,
//! preview, timeline, inspector) plus modal dialogs (export, settings).
//!
//! ## Threading model
//!
//! The UI runs on the eframe render thread. All ffmpeg calls happen on
//! background std::thread jobs spawned by [`BackgroundJobs`]; results
//! flow back via channels polled on each frame.

mod app;
mod background;
mod commands;
mod state;
mod ui;

pub use app::EdtApp;
