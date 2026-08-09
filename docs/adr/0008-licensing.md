# ADR 0008 — Licensing: dual MIT / Apache-2.0

**Date:** 2026-08-09  
**Status:** Accepted

## Context

edt is intended to be open-source and reusable. We need to pick a
license that:

- Is permissive (allows proprietary forks and commercial use).
- Is well-understood by the Rust ecosystem.
- Is compatible with all of our dependencies.
- Does not impose copyleft on derivative works.

## Decision

Dual-license edt under **MIT OR Apache-2.0**.

## Consequences

### Positive

- **Maximum compatibility.** Every Rust crate we depend on (egui,
  serde, image, rfd, etc.) is licensed under MIT, Apache-2.0, or
  both. Dual-licensing means we can be a dependency of either MIT
  or Apache-2.0 projects without forcing a license change.
- **Standard Rust ecosystem practice.** This is the license pair
  recommended by the Rust API guidelines.
- **No copyleft surprise.** Users can fork edt into a proprietary
  product without legal friction.

### Negative

- **No copyleft protection.** A company can take edt, improve it,
  and not share their improvements. We accept this — the goal is
  maximum adoption, not forcing derivatives to open up.

### FFmpeg interaction

edt shells out to ffmpeg at runtime but does not link or distribute
ffmpeg. Therefore:

- edt's MIT/Apache-2.0 license is unaffected by ffmpeg's LGPL/GPL
  license.
- Users must install ffmpeg separately and comply with ffmpeg's
  license for their use of ffmpeg.
- If a future edt release bundles a ffmpeg binary, that distribution
  must comply with ffmpeg's license (LGPL or GPL, depending on the
  build). We would document this clearly and provide ffmpeg's
  source code as required.
