# ADR 0002 — Media backend: shell out to ffmpeg

**Date:** 2026-08-09  
**Status:** Accepted

## Context

A video editor needs to:

- Probe media files (resolution, duration, codec, etc.).
- Decode individual frames for preview and composition.
- Encode the final output file.

The main approaches:

1. **FFI to libavcodec / libavformat / libavutil** (via the
   `ffmpeg-next` Rust crate).
2. **GStreamer** (via `gstreamer` Rust crate).
3. **Shelling out to the `ffmpeg` / `ffprobe` binaries** via
   `std::process::Command`.

## Decision

**Shell out to ffmpeg / ffprobe binaries.** Do not link libav* via FFI.

## Consequences

### Positive

- **Cross-platform build simplicity.** Linking libav* requires:
  - Linux: `apt install libavcodec-dev libavformat-dev ...` (easy)
  - macOS: `brew install ffmpeg` + setting `PKG_CONFIG_PATH` to the
    keg-only install path (fiddly)
  - Windows: downloading pre-built FFmpeg shared libraries and
    setting `FFMPEG_DIR` (no system package manager)
  Each of these is a CI landmine. Shelling out works on all three
  platforms with zero native deps.
- **Smaller dependency tree.** `ffmpeg-next` pulls in a lot of
  build-time machinery.
- **Decoupled versioning.** Users can upgrade FFmpeg independently
  of edt.
- **Easier debugging.** A failing ffmpeg invocation can be
  copy-pasted from the editor's logs and run manually.

### Negative

- **Process-spawn overhead** per frame extraction (~10-30ms). For
  preview, this is hidden behind a 32-frame cache and async
  extraction. For export, we mitigate by using the `concat` filter
  strategy when possible (single ffmpeg invocation for the whole
  timeline).
- **No in-process decode of arbitrary frame formats.** We rely on
  ffmpeg's PNG output for frame extraction, which the `image` crate
  reads. This adds a PNG encode/decode round-trip per frame.
- **Users must install FFmpeg separately.** This is documented in
  the README. A future v0.7 release could bundle a static ffmpeg
  binary per platform.

### Neutral

- We do not use GStreamer because it has an even larger native
  dependency footprint (glib, gobject-introspection, etc.) and is
  less commonly pre-installed than ffmpeg.
- If we later need GPU-accelerated decode (NVDEC, VideoToolbox),
  we can switch to `ffmpeg-next` with hardware-accel features. The
  `edt_media` API surface is small enough that this is a localized
  change.
