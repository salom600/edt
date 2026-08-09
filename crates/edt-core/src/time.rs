//! Time types for the timeline.
//!
//! All times are `f64` seconds. We provide [`Time`] (a typed alias) and
//! [`TimeRange`] for clarity in signatures, and helper conversions for
//! frames and audio samples.

use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

/// A point in time, in seconds, within the project timeline.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Time(pub f64);

impl Time {
    pub const ZERO: Time = Time(0.0);

    pub fn from_secs(s: f64) -> Self {
        Time(s)
    }

    pub fn from_frames(frame: i64, fps: f64) -> Self {
        Time(frame as f64 / fps)
    }

    pub fn as_secs(self) -> f64 {
        self.0
    }

    pub fn as_millis(self) -> f64 {
        self.0 * 1000.0
    }

    pub fn to_frame(self, fps: f64) -> i64 {
        (self.0 * fps).round() as i64
    }

    pub fn max(self, other: Time) -> Time {
        Time(self.0.max(other.0))
    }

    pub fn min(self, other: Time) -> Time {
        Time(self.0.min(other.0))
    }
}

impl Add for Time {
    type Output = Time;
    fn add(self, rhs: Time) -> Time {
        Time(self.0 + rhs.0)
    }
}

impl Sub for Time {
    type Output = Time;
    fn sub(self, rhs: Time) -> Time {
        Time(self.0 - rhs.0)
    }
}

impl Default for Time {
    fn default() -> Self {
        Time::ZERO
    }
}

/// A half-open interval `[start, end)` on the project timeline.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: Time,
    pub end: Time,
}

impl TimeRange {
    pub fn new(start: Time, end: Time) -> Self {
        debug_assert!(
            end.0 >= start.0,
            "TimeRange end must be >= start (got {start:?}..{end:?})"
        );
        Self { start, end }
    }

    pub fn from_start_duration(start: Time, dur: Time) -> Self {
        Self::new(start, start + dur)
    }

    pub fn duration(self) -> Time {
        Time(self.end.0 - self.start.0)
    }

    pub fn contains(self, t: Time) -> bool {
        t.0 >= self.start.0 && t.0 < self.end.0
    }

    pub fn intersects(self, other: Self) -> bool {
        self.start.0 < other.end.0 && other.start.0 < self.end.0
    }
}

impl Default for TimeRange {
    fn default() -> Self {
        Self::new(Time::ZERO, Time::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_arithmetic() {
        let a = Time::from_secs(1.0);
        let b = Time::from_secs(2.5);
        assert_eq!((a + b).as_secs(), 3.5);
        assert_eq!((b - a).as_secs(), 1.5);
    }

    #[test]
    fn frame_roundtrip() {
        let t = Time::from_frames(100, 25.0);
        assert!((t.as_secs() - 4.0).abs() < 1e-9);
        assert_eq!(t.to_frame(25.0), 100);
    }

    #[test]
    fn range_contains_and_intersects() {
        let r = TimeRange::new(Time(0.0), Time(10.0));
        assert!(r.contains(Time(5.0)));
        assert!(!r.contains(Time(10.0)));
        assert!(r.intersects(TimeRange::new(Time(5.0), Time(15.0))));
        assert!(!r.intersects(TimeRange::new(Time(10.0), Time(15.0))));
    }
}
