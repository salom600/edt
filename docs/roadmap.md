# Roadmap

This document tracks planned work for edt beyond v0.1.0. Items are
grouped by theme and tagged with the same IDs used in
[known-issues.md](known-issues.md).

## v0.2 — Real frame compositor

The headline feature for v0.2 is a real-time, multi-track frame
compositor that decodes source frames and blends them according to
each clip's opacity, transforms, and effects.

- **E-002**: FramePipe export calls `edt_media::extract_frame` per
  active clip and composites the result. Performance target: 4×
  real-time on 1080p / 30fps with a single video track on a 2024
  laptop CPU.
- **E-001**: Cross-track alpha blending. Walk tracks bottom-to-top,
  apply each track's `level` as an alpha multiplier.
- **A-001**: Audio playback during preview using `cpal` or
  `rodio`. Buffer 200ms ahead, sync to playhead.
- **A-002**: Audio in FramePipe exports via a second ffmpeg input
  stream.
- **E-003**: Apply `ColorGrade` and basic `Effect`s (blur, flip,
  rotate, crop, scale, opacity, text overlay) during composition.
- **E-004**: Apply `Transition`s at clip boundaries (dissolve,
  dip-to-black, wipe, audio crossfade).

## v0.3 — Performance & GPU

- **P-003**: Proxy editing. Generate low-bitrate proxy copies on
  import; use them for editing; switch to originals at export.
- **GPU**: Switch the egui backend from `glow` to `wgpu`. Render
  the compositor's output texture directly to an egui image, skipping
  a CPU-side copy.
- **GPU effects**: Implement the effect stack as a wgpu render
  pipeline. Each effect becomes a fragment shader pass.
- **P-002**: Bounded LRU preview frame cache with explicit
  eviction policy.
- **P-001**: Configurable autosave interval in settings.

## v0.4 — Editing power features

- **U-001**: Undoable trim operations.
- **U-002**: Ripple delete / ripple insert.
- **U-003**: Keyframes for effect parameters. Linear / hold /
  bezier interpolation.
- **Multi-camera**: Sync multiple video clips by audio waveform,
  switch between them on the fly.
- **LUT support**: Apply `.cube` LUTs as a color grade effect.
- **Color wheels**: Lift / Gamma / Gain / Offset wheels in the
  inspector.

## v0.5 — Plugin & scripting API

- **Plugin API**: Define a stable Rust trait (`edt_plugin::Plugin`)
  that third-party crates can implement. Load plugins via
  `libloading` at startup.
- **Scripting**: Embed a Lua or Rhai interpreter; expose project
  mutation APIs.
- **Open effects API**: Allow plugins to register custom effect
  kinds that show up in the inspector and are applied during
  render.

## v0.6 — AI-assisted features (optional, opt-in)

- **Speech-to-text subtitles**: Run whisper.cpp on the audio track
  to generate subtitle clips.
- **Auto scene detection**: Detect cut points by frame histogram
  differencing.
- **Smart trim**: Detect silent / low-motion segments and offer to
  remove them.

These features will be opt-in and will run entirely locally — no
cloud APIs.

## v0.7 — Polish & distribution

- **PL-001**: Code signing for macOS (Developer ID) and Windows
  (Authenticode). Document how contributors can reproduce builds.
- **PL-002**: Produce .app bundles, .dmg images, .deb packages,
  .rpm packages, and .msi installers in CI. Auto-update via
  `self_update` or platform-native mechanisms.
- **S-001**: Surface background-job panics as UI error toasts.
- **Accessibility**: Audit egui's keyboard navigation, add ARIA-like
  labels to custom widgets.
- **Localization**: Extract all user-facing strings; provide
  translations for at least English, Spanish, Mandarin, French.

## Long-term / unclear

- **Node-based effects editor**: Alternative to the stack-based
  effects model for users who want more control. Would live
  alongside the stack UI, not replace it.
- **Collaborative editing**: Real-time multi-user editing via CRDTs.
  Major undertaking; would require a server component.
- **Mobile companion**: A read-only viewer for project files on
  iOS/Android. No editing on mobile.
