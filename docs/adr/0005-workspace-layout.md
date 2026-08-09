# ADR 0005 — Workspace layout: multi-crate

**Date:** 2026-08-09  
**Status:** Accepted

## Context

edt has distinct concerns: pure data model, media I/O, storage,
rendering, export, and UI. We need to decide whether to keep these
in a single crate with modules, or split into multiple crates in a
Cargo workspace.

## Decision

Use a **Cargo workspace with six crates**:

- `edt-core` — pure data model, no I/O, no UI.
- `edt-media` — ffmpeg probing + frame extraction.
- `edt-storage` — JSON project save/load.
- `edt-render` — frame composition + audio mixing.
- `edt-export` — drives ffmpeg to render the final file.
- `edt-app` — egui UI shell.

## Consequences

### Positive

- **`edt-core` has zero non-`serde` dependencies.** This means core
  data types can be loaded in tests, scripting hosts, or headless
  tools without dragging in ffmpeg or egui.
- **Compile parallelism.** Cargo can compile independent crates in
  parallel.
- **Clear ownership boundaries.** Each crate has a focused
  responsibility, which makes refactoring safer.
- **Future plugin API.** A plugin crate can depend on `edt-core`
  (and maybe `edt-render`) without pulling in the UI.

### Negative

- **More boilerplate.** Each crate has its own `Cargo.toml` with
  workspace dep references. This is a minor inconvenience.
- **Cross-crate refactoring** is slightly slower than within a
  single crate, because each crate must recompile.

### Neutral

- The workspace root `Cargo.toml` defines shared dependency versions
  via `[workspace.dependencies]`, so all crates use the same version
  of `serde`, `image`, etc.
