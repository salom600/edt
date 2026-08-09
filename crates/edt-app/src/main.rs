//! edt — a modern cross-platform video editor built in Rust.
//!
//! This binary is the UI shell. All editing logic lives in the
//! `edt-core` / `edt-media` / `edt-storage` / `edt-render` / `edt-export`
//! crates. The shell wires those into an [`eframe`] / [`egui`] UI.

use edt_app::EdtApp;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() -> eframe::Result<()> {
    init_tracing();
    install_panic_hook();

    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 600.0])
            .with_title("edt — video editor"),
        ..Default::default()
    };

    eframe::run_native(
        "edt",
        opts,
        Box::new(move |cc| {
            Ok(Box::new(EdtApp::new(cc)))
        }),
    )
}

static TRACING_INSTALLED: AtomicBool = AtomicBool::new(false);

fn init_tracing() {
    if TRACING_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,edt=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .with_ansi(true)
        .init();
}

/// Install a panic hook that logs panics via `tracing::error` before
/// the process dies. This makes panics visible in CI logs instead of
/// being swallowed by the GUI runtime.
fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "<unknown>".into());
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("<non-string panic payload>");
        tracing::error!(location = %location, payload = %payload, "PANIC");
        prev(info);
    }));
}
