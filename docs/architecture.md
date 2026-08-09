# Architecture

This document describes the high-level architecture of edt. For
point-in-time design decisions and their rationale, see
[adr/](adr/).

## Workspace layout

```
edt/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── edt-core/           # Pure data model (no I/O, no UI)
│   ├── edt-media/          # ffmpeg probing + frame extraction
│   ├── edt-storage/        # JSON project save/load
│   ├── edt-render/         # Frame composition + audio mixing
│   ├── edt-export/         # Drives ffmpeg to render final file
│   └── edt-app/            # egui UI shell + background jobs
├── .github/workflows/      # CI/CD
├── docs/                   # This directory
├── packaging/              # Platform-specific packaging scripts
└── assets/                 # Icons, sample media
```

## Crate dependency graph

```
edt-app
  ├─ edt-export
  │    ├─ edt-render
  │    │    ├─ edt-media
  │    │    └─ edt-core
  │    ├─ edt-media
  │    └─ edt-core
  ├─ edt-render
  ├─ edt-storage
  │    └─ edt-core
  ├─ edt-media
  │    └─ edt-core
  └─ edt-core
```

`edt-core` has zero non-`serde` dependencies — it is the foundation
everything else builds on. This means core data types can be loaded
in tests, scripting hosts, or headless tools without dragging in the
ffmpeg or egui dependency trees.

## Threading model

The UI runs on the `eframe` render thread (the main thread). All
ffmpeg calls are dispatched to a single background worker thread
owned by `BackgroundJobs` (in `crates/edt-app/src/background.rs`).
Results flow back to the UI via a `crossbeam-channel::Receiver` that
the UI drains on every frame.

Project state is shared between threads via `Arc<EditorState>`,
where `EditorState` wraps a `parking_lot::RwLock<EditorStateInner>`.
The UI takes a write lock only when mutating state; reads use a
short-lived read lock that snapshots the data needed for one frame.

## Time model

All time values are `f64` seconds. We chose floating-point seconds
over rational time (e.g. `num_rational::Ratio<i64>`) because:

1. The precision is more than sufficient for any practical edit
   (1 µs at 60 fps is 16× finer than a frame).
2. Math with audio sample positions and frame indices is trivial.
3. JSON serialization is natural.

See [adr/0001-time-model.md](adr/0001-time-model.md) for the full
discussion.

## Project file format

Projects are saved as pretty-printed JSON with a `format_version`
header. The current version is `1`. The schema is defined by
`edt_core::project::ProjectFile`.

Atomic writes: save writes to `<path>.<name>.tmp` then renames over
the destination. This prevents corruption if the process is killed
mid-write.

Autosave: every 60 seconds, if the project is dirty, edt writes to
`<cache_dir>/edt/autosave/<name>.json`. The cache directory is
located via the `directories` crate.

## Media backend

edt shells out to `ffmpeg` / `ffprobe` rather than linking
`libav*` via FFI. This decision is documented in
[adr/0002-media-backend.md](adr/0002-media-backend.md). The short
version: linking libav* is a CI configuration nightmare on
Windows and macOS, while shelling out to a pre-installed binary
works on all three platforms with zero native deps.

## Render pipeline

Frame composition lives in `edt_render::compose`. For v0.1 the
pipeline is intentionally simple:

1. Walk timeline tracks top-to-bottom at time `t`.
2. For each non-muted video track, find the active clip.
3. Call the caller-provided `clip_frames` closure to get an
   `RgbaImage` for that clip.
4. Blit-fit the frame onto the canvas (letterbox, no transforms).

Cross-track opacity blending, transitions, and color grade
application are v0.2 work — see
[adr/0006-render-pipeline.md](adr/0006-render-pipeline.md).

## Export pipeline

Two strategies, picked automatically by `pick_strategy`:

- **Concat** — used when there is exactly one non-empty track and all
  its clips come from a single asset. Builds an ffmpeg `concat`
  filter graph and runs a single ffmpeg invocation. Fast.
- **FramePipe** — used otherwise. Renders each frame in-process
  via `edt_render::compose_frame` and pipes raw RGBA to ffmpeg via
  stdin. Slower but handles any timeline shape.

For v0.1 the FramePipe strategy renders a solid color per clip
rather than the actual decoded video frame. This is a known
limitation — see [known-issues.md](known-issues.md) item E-002.
The wiring is in place to call `edt_media::extract_frame` per
clip; doing so naively would make the test suite take many
minutes, so it is gated behind a roadmap item.

## Undo / redo

Commands are objects implementing the `Command` trait (apply + revert).
The `UndoStack` holds up to 100 commands; older ones are evicted FIFO.
Each command captures enough state to revert itself (clip snapshots,
track id, etc.).

Currently implemented commands:
- `AddClipCmd`
- `DeleteClipCmd`
- `MoveClipCmd`
- `SplitClipCmd`

Trim commands are not yet implemented (see roadmap U-001).

## CI / CD

Two workflows:

- `ci.yml` — runs on every push/PR. Lints (fmt + clippy) and tests
  on Linux, then builds release binaries on Linux/Windows/macOS.
  Uploads artifacts.
- `release.yml` — runs on tag push (`v*`). Builds artifacts on all
  four targets (Linux x86_64, Windows x86_64, macOS arm64, macOS
  x86_64) and publishes a GitHub Release with checksums.

The release workflow uses `softprops/action-gh-release@v2` and
attaches both the artifact archives and their `.sha256` checksum
files. Release notes are auto-generated from commit history plus
a curated header.

## Cross-platform considerations

- **Path handling**: all paths in the project model are
  `std::path::PathBuf`. We never assume `/` vs `\`.
- **File dialogs**: `rfd` provides native dialogs on each platform.
- **Windowing**: `eframe` uses `winit` under the hood, which
  abstracts platform-specific window creation.
- **Rendering**: `egui` can use either `glow` (OpenGL) or `wgpu`
  (Vulkan/Metal/DX12). We default to `glow` for v0.1 to keep the
  build simpler; `wgpu` is a v0.2 upgrade target.
- **FFmpeg**: users install this separately. The README documents
  the install command for each platform.

## Security

- No network access from the application itself.
- No telemetry.
- Project files are JSON; they can contain arbitrary paths but no
  executable code.
- The only process spawning is to `ffmpeg` / `ffprobe` on PATH.
  User-supplied paths are passed as separate command arguments
  (never shell-interpolated), so shell injection is not possible.
- File dialogs are native; no custom path parsing.
