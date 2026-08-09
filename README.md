# edt — Modern cross-platform video editor

[![CI](https://github.com/salom600/edt/actions/workflows/ci.yml/badge.svg)](https://github.com/salom600/edt/actions/workflows/ci.yml)
[![Release](https://github.com/salom600/edt/actions/workflows/release.yml/badge.svg)](https://github.com/salom600/edt/releases)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)

A modern, professional, non-linear video editor built primarily in Rust.
Runs natively on **Windows**, **Linux**, and **macOS**.

> **Status:** v0.1.0 — MVP. See [docs/known-issues.md](docs/known-issues.md)
> and [docs/roadmap.md](docs/roadmap.md) for what's working and what's next.

---

## Features

### Working in v0.1.0

- **Media management**: import video / audio / images, thumbnails, metadata display
- **Multi-track timeline**: 2 video + 2 audio tracks out of the box, more can be added
- **Editing**: trim, split, move clips with snap-to-edge, label colors, per-clip mute
- **Preview player**: transport controls, frame stepping, scrub slider
- **Project system**: JSON save/load with atomic writes, 60-second autosave
- **Export**: MP4 / MKV / MOV / WebM via ffmpeg, H.264/H.265/AV1/VP9/ProRes video codecs, AAC/Opus/PCM audio
- **Undo/redo**: add / delete / split / move clip operations
- **Dark theme**: modern dark UI built on egui

### Roadmap (v0.2+)

See [docs/roadmap.md](docs/roadmap.md). Highlights:

- Real frame-composited preview/export (currently uses solid color per clip)
- Cross-track opacity blending
- Color grade application during render
- Audio playback during preview
- LUT support, color wheels
- GPU-accelerated effects via wgpu
- Plugin / scripting API

---

## Quick start

### Prerequisites

**FFmpeg** must be installed separately — edt shells out to `ffmpeg` and
`ffprobe` on your PATH.

| OS | Install command |
|---|---|
| Linux (Debian/Ubuntu) | `sudo apt install ffmpeg` |
| Linux (Fedora) | `sudo dnf install ffmpeg` |
| Linux (Arch) | `sudo pacman -S ffmpeg` |
| macOS | `brew install ffmpeg` |
| Windows | `choco install ffmpeg` or download from <https://ffmpeg.org/download.html> |

Verify:
```sh
ffmpeg -version
ffprobe -version
```

### Build from source

```sh
git clone https://github.com/salom600/edt.git
cd edt
cargo run --release
```

### Download a prebuilt binary

See the [Releases page](https://github.com/salom600/edt/releases) for
prebuilt binaries for Windows, Linux, and macOS (both Apple Silicon and
Intel). Each release includes SHA-256 checksums.

---

## Keyboard shortcuts

| Action | Shortcut |
|---|---|
| Play / pause | `Space` |
| Step forward one frame | `→` |
| Step backward one frame | `←` |
| Split clip at playhead | `S` |
| Delete selected clip | `Delete` |
| Undo | `Ctrl+Z` |
| Redo | `Ctrl+Shift+Z` |
| New project | (File menu) |
| Open project | `Ctrl+O` |
| Save project | `Ctrl+S` |
| Save project as | `Ctrl+Shift+S` |
| Import media | `Ctrl+I` |
| Export… | `Ctrl+E` |
| Quit | (File menu) |

---

## Architecture

```
+---------------------------------------------------------------+
|                        edt-app (egui UI)                       |
+---------------------------------------------------------------+
|  menubar | media_pool | preview | timeline | inspector | export|
+---------------------------------------------------------------+
|                       edt-export (pipeline)                    |
+---------------------------------------------------------------+
|                       edt-render (compose)                     |
+---------------------------------------------------------------+
|     edt-storage (JSON)        |        edt-media (ffmpeg)      |
+---------------------------------------------------------------+
|                          edt-core (model)                      |
+---------------------------------------------------------------+
```

| Crate | Responsibility |
|---|---|
| [`edt-core`](crates/edt-core) | Project / timeline / clip data model. Pure, no I/O. |
| [`edt-media`](crates/edt-media) | ffmpeg probing, thumbnail + frame extraction |
| [`edt-storage`](crates/edt-storage) | JSON project save/load with atomic writes |
| [`edt-render`](crates/edt-render) | Frame composition + audio mixing from timeline state |
| [`edt-export`](crates/edt-export) | Drives ffmpeg to render the final output file |
| [`edt-app`](crates/edt-app) | egui UI shell, background jobs, undo/redo |

See [docs/architecture.md](docs/architecture.md) for the full write-up,
and [docs/adr/](docs/adr/) for architecture decision records.

---

## Build & development

### Run tests

```sh
cargo test --workspace
```

### Lint

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

### Build a release binary

```sh
cargo build --release
# Binary: target/release/edt (or edt.exe on Windows)
```

### CI

GitHub Actions workflows live in [`.github/workflows/`](.github/workflows/):

- `ci.yml` — runs on every push / PR. Lints + tests on Linux, then
  builds release binaries on Linux / Windows / macOS.
- `release.yml` — runs on tag push (`v*`). Builds artifacts for all
  platforms and publishes a GitHub Release.

---

## Packaging

Platform-specific packaging scripts are in [`packaging/`](packaging/):

- **Linux**: `.desktop` entry, MIME type association, AppImage script
- **macOS**: `Info.plist`, `.app` bundle script
- **Windows**: NSIS installer script

See [docs/packaging.md](docs/packaging.md) for full instructions.

---

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Third-party notices

This project uses the following open-source components:

- **FFmpeg** — LGPL 2.1+ (binary distribution; users install separately).
  See <https://ffmpeg.org/legal.html>.
- **egui / eframe** — MIT/Apache-2.0
- **image** crate — MIT/Apache-2.0
- **rfd** (rusty-fork-dialer) — MIT/Apache-2.0
- Many other Rust crates — see [Cargo.lock](Cargo.lock) for the full list.

See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for details.

---

## Contributing

PRs welcome! Please run `cargo fmt`, `cargo clippy`, and `cargo test`
before submitting. See [docs/development.md](docs/development.md) for
local development tips.

## Code of conduct

Standard Rust Code of Conduct applies. See
<https://www.rust-lang.org/policies/code-of-conduct>.
