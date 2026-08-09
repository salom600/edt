# ADR 0007 — Export strategy: concat vs frame-pipe

**Date:** 2026-08-09  
**Status:** Accepted

## Context

Exporting a timeline to a video file requires deciding how to feed
frame data to the encoder. Two main approaches:

1. **Single ffmpeg invocation with `concat` filter**: tell ffmpeg
   about all the source clips, apply trims via `-ss`/`-t`, and use
   the `concat` filter to stitch them. ffmpeg does all decoding and
   encoding internally. Fast.
2. **Per-frame in-process composition**: render each frame in
   edt via `edt_render::compose_frame`, pipe raw RGBA to ffmpeg via
   stdin, let ffmpeg encode. Slower (per-frame process overhead and
   PNG round-trip), but handles any timeline shape.

## Decision

Support **both** strategies. Pick automatically based on the
timeline shape:

- If there is exactly one non-empty track and all its clips come
  from a single asset, use **concat**.
- Otherwise, use **frame-pipe**.

The user can override the choice in the export dialog (v0.2 feature;
v0.1 always auto-picks).

## Consequences

### Positive

- **Best of both worlds.** Single-asset timelines export at near
  ffmpeg-native speed. Multi-track timelines still work, just
  slower.
- **No user configuration needed** for the common case.
- **`pick_strategy` is a pure function** of the project state, so
  it can be unit-tested without touching ffmpeg.

### Negative

- **Two code paths to maintain.** The concat path is a single
  `Command` builder; the frame-pipe path is a streaming loop with
  stdin piping. Both have their own failure modes.
- **Frame-pipe is currently a placeholder** (renders solid colors
  per clip, not actual decoded video). This is a known limitation
  (known-issues E-002); the wiring to call
  `edt_media::extract_frame` per clip exists but is gated behind
  performance work.
- **Concat strategy cannot apply effects** — it just stitches
  source frames. Users who want effects must use the frame-pipe
  strategy.

### Neutral

- The concat filter syntax is finicky (`concat=n=2:v=1:a=0`), but
  we generate it programmatically and unit-test the generation.
- A future v0.3 "hybrid" strategy could render each clip to a
  temporary file via concat, then concat the temp files. This would
  give us effects support at near-concat speed.
