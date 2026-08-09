# ADR 0006 — Render pipeline: topmost-clip-wins for v0.1

**Date:** 2026-08-09  
**Status:** Accepted (for v0.1; will be revisited in v0.2)

## Context

A multi-track video compositor must decide how to combine multiple
video clips that are active at the same timeline time. Standard
approaches:

1. **Topmost-clip-wins**: the topmost non-muted track's clip is
   drawn opaquely; lower tracks are ignored.
2. **Alpha compositing**: each track contributes a layer; the
   track's `level` is used as alpha; layers are composited
   bottom-to-top with the standard "over" operator.
3. **Node graph**: each track feeds into a user-defined node graph
   that produces the final frame.

## Decision

For v0.1, use **topmost-clip-wins**. Walk tracks top-to-bottom,
blit-fit each active clip's frame onto the canvas, letting later
writes overwrite earlier ones.

## Consequences

### Positive

- **Trivial to implement.** A single `image::imageops::overlay` call
  per clip.
- **Predictable behavior** for users coming from a "video track =
  opaque layer" mental model.
- **No alpha math bugs** in v0.1.

### Negative

- **`level` is ignored for video tracks.** Setting a clip's opacity
  to 50% has no visible effect. This is a known limitation
  (known-issues E-001) and is documented in the inspector.
- **No transitions.** Cross-fades require alpha compositing. v0.1
  stores `Transition` records but does not render them (E-004).

### Neutral

- For v0.2, we will switch to **alpha compositing** with the
  standard "over" operator. The change is localized to
  `edt_render::compose::compose_frame`. The data model already
  stores `level` per track and per clip, so no schema migration is
  needed.
- The node-graph approach is a longer-term possibility (v0.5+)
  for users who want more control. It would live alongside the
  stack model, not replace it.
