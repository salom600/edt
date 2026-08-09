# ADR 0003 — UI framework: egui

**Date:** 2026-08-09  
**Status:** Accepted

## Context

A 2026-grade video editor needs a UI framework that supports:

- Cross-platform desktop (Windows, Linux, macOS).
- Custom rendering (the timeline panel is heavily custom-painted).
- Reasonable performance for 60fps interaction.
- Mature enough to not surprise us mid-project.
- Permissively licensed.

The main candidates:

1. **egui** (immediate mode, pure Rust, glow/wgpu backend).
2. **iced** (retained, Elm-inspired, wgpu backend).
3. **Slint** (custom DSL, optional wgpu backend).
4. **Tauri** (web frontend + Rust backend).
5. **Custom wgpu + DOM-like retained mode tree** (build from scratch).

## Decision

Use **egui** (with `eframe` for windowing).

## Consequences

### Positive

- **Immediate mode** is ideal for the timeline panel, which needs
  per-pixel hit testing, drag interactions, and custom painting.
  egui's `Painter` API gives us direct access to draw rectangles,
  text, and images without fighting a retained-mode widget tree.
- **Pure Rust.** No webview, no JavaScript, no system webview
  version drift.
- **Small binary size** compared to Tauri (which ships a webview
  frontend).
- **Mature.** egui has been stable since 2020 and is used by many
  desktop Rust apps (Rerun, Hyperion, etc.).
- **Permissive license** (MIT OR Apache-2.0).

### Negative

- **No native widgets.** egui's widgets look like egui, not like
  native Windows / macOS / GTK. For a video editor this is
  acceptable — professional NLEs (Resolve, Premiere) all use custom
  UI anyway.
- **Accessibility is weak.** egui's keyboard navigation and screen
  reader support are improving but not yet at native-toolkit level.
  Tracked as a v0.7 concern.
- **No declarative layout DSL** like Slint. Layout is expressed in
  Rust code, which is more flexible but more verbose.

### Neutral

- We default to the `glow` (OpenGL) backend for v0.1 to keep the
  build simple. The `wgpu` backend is a v0.2 upgrade target — it
  will let us render the compositor's output texture directly into
  an egui image without a CPU-side copy.
- We considered `iced` seriously. Its retained-mode model would
  have made the timeline panel harder to build (we'd need to
  implement a custom widget either way). egui's immediate mode is
  a better fit for our custom-painting needs.
