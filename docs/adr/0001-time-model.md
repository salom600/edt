# ADR 0001 — Time model: f64 seconds

**Date:** 2026-08-09  
**Status:** Accepted

## Context

A video editor needs a time representation that supports:

- Frame-accurate seeking at common framerates (23.976, 24, 25, 29.97,
  30, 50, 59.94, 60, 120).
- Math with audio sample positions (48000 Hz, 96000 Hz).
- Reasonable precision over long projects (hours).
- JSON serialization.
- Easy mental arithmetic for users (SMPTE timecode conversion).

The main candidates are:

1. **Integer frames** at the project's native framerate.
2. **Rational time** (`num_rational::Ratio<i64>` or similar).
3. **Floating-point seconds** (`f64`).

## Decision

Use **`f64` seconds** as the canonical time type, exposed via the
newtype `edt_core::time::Time`.

## Consequences

### Positive

- Trivial math: `start + duration`, `t * speed`, `t * sample_rate`.
- Natural JSON serialization (just a number).
- Sufficient precision: at 60 fps, one frame is ~16.6 ms; f64 has
  ~15 significant decimal digits, so even a 24-hour project has
  sub-microsecond precision.
- Easy conversion to/from frames: `frame = round(t * fps)`.

### Negative

- 29.97 fps (NTSC) is not exactly representable — `30000/1001` is
  a repeating fraction. We accept the ~1e-16 relative error, which
  is far below any perceptual threshold.
- Comparisons need epsilon: `t1 == t2` should use `(t1 - t2).abs() < eps`.
  We document this in `edt_core::time::Time` and use it in tests.
- Cannot exactly represent drop-frame timecode. SMPTE timecode
  display is a v0.2 feature; when added, it will live in the UI
  layer, not in the core time type.

### Neutral

- The project also stores a `fps: f64` field on `ProjectSettings`
  so frame indices can be derived when needed.
