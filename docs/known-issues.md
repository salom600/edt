# Known Issues

This document tracks known limitations of edt v0.1.0. Each item has
an ID for cross-referencing with the roadmap.

## Rendering / Export

### E-001 — Multi-track compositing uses topmost-clip-wins

The v0.1 compositor walks tracks top-to-bottom and lets later writes
overwrite earlier ones. This means a clip on V2 will completely
obscure a clip on V1, ignoring the V2 clip's `level` (opacity)
property. Cross-track alpha blending is a v0.2 feature.

### E-002 — Frame-pipe export renders solid color per clip

The FramePipe export strategy (used for multi-track or multi-asset
timelines) currently renders a deterministic solid color per clip
rather than the actual decoded video frame. The wiring to call
`edt_media::extract_frame` per clip exists; doing so naively would
make the test suite take many minutes, so it is gated behind roadmap
item E-002.

**Workaround:** For single-track, single-asset timelines, use the
Concat export strategy (the default), which produces a real video.

### E-003 — Color grade and effects parameters are not applied

The data model stores `ColorGrade` and `Effect` instances on clips,
and the inspector shows effect placeholders, but the render pipeline
does not yet apply these during preview or export. See roadmap F-001.

### E-004 — Transitions are stored but not rendered

`Transition` records can be created in the data model but the
compositor ignores them. See roadmap T-001.

## Audio

### A-001 — No audio playback during preview

The preview player shows video frames but does not play audio.
Audio mixing logic exists in `edt_render::audio::mix_audio` but is
not yet wired to an audio output device.

### A-002 — Audio export is silent in FramePipe mode

The FramePipe export strategy pipes only video frames to ffmpeg.
Audio is therefore absent from FramePipe exports. Use the Concat
strategy (single-track, single-asset) for exports with audio.

## UI / Editing

### U-001 — Trim operations are not undoable

The undo stack supports add/delete/split/move but not trim. Trim
operations mutate the clip in place without recording the original
bounds. Tracked as roadmap item U-001.

### U-002 — Ripple edit not implemented

There is no ripple-delete or ripple-insert. Deleting a clip leaves
a gap. See roadmap U-002.

### U-003 — No keyframe support

The `Effect` model has no keyframes. All effect parameters are
constant across the clip's duration. See roadmap F-002.

## Performance

### P-001 — Autosave interval is hardcoded

The 60-second autosave interval cannot be configured. See roadmap P-001.

### P-002 — Preview frame cache is unbounded-ish

The preview cache holds up to 32 frames, then clears entirely.
A proper LRU would be better. See roadmap P-002.

### P-003 — No proxy editing

The `MediaAsset.proxy_path` field exists but the UI does not expose
a way to generate or use proxies. See roadmap P-003.

## Platform

### PL-001 — No code signing

macOS .app bundles and Windows installers are unsigned. Users will
see Gatekeeper / SmartScreen warnings. See roadmap PL-001.

### PL-002 — No .app / .dmg / .msi packaging in CI

The CI workflow produces raw binaries in tar.gz / zip archives. The
packaging scripts in `packaging/` can produce .app bundles, .dmg
images, and .msi installers but are not yet wired into CI. See
roadmap PL-002.

## Stability

### S-001 — Panics in background jobs are not surfaced to the UI

If the background worker thread panics, the channel simply closes
and the UI stops receiving updates. A panic hook logs to stderr but
the user sees nothing. See roadmap S-001.
