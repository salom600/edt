//! Progress reporting for export.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Progress information reported by the export pipeline.
#[derive(Debug, Clone, Copy)]
pub struct ExportProgress {
    pub frames_done: u64,
    pub frames_total: u64,
    pub current_time_secs: f64,
    pub total_time_secs: f64,
}

impl ExportProgress {
    pub fn fraction(&self) -> f64 {
        if self.frames_total == 0 {
            return 0.0;
        }
        (self.frames_done as f64 / self.frames_total as f64).clamp(0.0, 1.0)
    }

    pub fn percent(&self) -> u32 {
        (self.fraction() * 100.0).round() as u32
    }
}

/// A thread-safe progress cell shared between the export worker and the UI.
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    inner: Arc<ProgressInner>,
}

#[derive(Debug)]
struct ProgressInner {
    frames_done: AtomicU64,
    frames_total: AtomicU64,
    cancelled: std::sync::atomic::AtomicBool,
}

impl ProgressUpdate {
    pub fn new(frames_total: u64) -> Self {
        Self {
            inner: Arc::new(ProgressInner {
                frames_done: AtomicU64::new(0),
                frames_total: AtomicU64::new(frames_total),
                cancelled: std::sync::atomic::AtomicBool::new(false),
            }),
        }
    }

    pub fn snapshot(&self) -> ExportProgress {
        let done = self.inner.frames_done.load(Ordering::Relaxed);
        let total = self.inner.frames_total.load(Ordering::Relaxed);
        ExportProgress {
            frames_done: done,
            frames_total: total,
            current_time_secs: 0.0,
            total_time_secs: 0.0,
        }
    }

    pub fn set_done(&self, n: u64) {
        self.inner.frames_done.store(n, Ordering::Relaxed);
    }

    pub fn inc_done(&self, by: u64) {
        self.inner.frames_done.fetch_add(by, Ordering::Relaxed);
    }

    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_fraction_clamps_to_one() {
        let p = ExportProgress {
            frames_done: 200,
            frames_total: 100,
            current_time_secs: 0.0,
            total_time_secs: 0.0,
        };
        assert!((p.fraction() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn progress_update_increments() {
        let u = ProgressUpdate::new(100);
        u.inc_done(30);
        assert_eq!(u.snapshot().frames_done, 30);
        u.inc_done(70);
        assert_eq!(u.snapshot().percent(), 100);
    }

    #[test]
    fn cancel_flag_round_trips() {
        let u = ProgressUpdate::new(10);
        assert!(!u.is_cancelled());
        u.cancel();
        assert!(u.is_cancelled());
    }
}
