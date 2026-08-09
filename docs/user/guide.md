# User guide

## Installing edt

1. Download the appropriate archive for your OS from the
   [Releases page](https://github.com/salom600/edt/releases).
2. Verify the checksum (see [packaging.md](../packaging.md#verification)).
3. Extract the archive.
4. Install [FFmpeg](https://ffmpeg.org/download.html) if you don't
   already have it — edt requires `ffmpeg` and `ffprobe` on your PATH.

## Starting edt

### Linux

```sh
tar -xzf edt-linux-x86_64.tar.gz
./edt
```

### macOS

```sh
tar -xzf edt-macos-arm64.tar.gz
./edt
```

(On first launch, macOS Gatekeeper will warn that edt is unsigned.
Right-click → Open → Open Anyway to bypass.)

### Windows

Extract `edt-windows-x86_64.zip` and double-click `edt.exe`.
(SmartScreen will warn that edt is unsigned. Click "More info" →
"Run anyway".)

## The editor layout

edt's window is divided into four panels:

```
┌──────────────────────────────────────────────────────────────┐
│  Menu bar: File / Edit / View / Playback / Help              │
├────────────┬──────────────────────────────────┬──────────────┤
│            │                                  │              │
│  Media     │         Preview player           │  Inspector   │
│  pool      │                                  │              │
│            │                                  │              │
│            ├──────────────────────────────────┤              │
│            │                                  │              │
│            │         Timeline                 │              │
│            │                                  │              │
│            │                                  │              │
└────────────┴──────────────────────────────────┴──────────────┘
```

- **Media pool** (left): imported video, audio, and image files.
- **Preview player** (center top): current timeline frame with
  transport controls.
- **Timeline** (center bottom): multi-track clip layout.
- **Inspector** (right): properties of the currently selected
  clip, track, or the project itself.

## Importing media

1. Press `Ctrl+I` (or File → Import Media…).
2. Select one or more files.
3. The files appear in the media pool. Thumbnails generate in the
   background; you'll see them fill in over a few seconds.

Supported formats: anything ffmpeg supports — MP4, MOV, MKV, WebM,
AVI, WAV, MP3, AAC, FLAC, PNG, JPEG, BMP, WebP, and many more.

## Adding clips to the timeline

### Method 1: Double-click

Double-click a media asset in the pool. The asset is appended to
the end of the first video track at the timeline's current end
point.

### Method 2: Drag-and-drop (coming in v0.2)

For v0.1, drag-and-drop from the media pool to the timeline is
partially implemented — clicking an asset in the pool and then
clicking on a timeline track will place a clip there.

## Editing clips

### Selecting

Click a clip in the timeline to select it. The inspector shows
its properties.

### Moving

Click and drag a clip's body to move it. The clip snaps to other
clips' edges and to the playhead when within 8 pixels.

### Trimming

Click and drag a clip's left or right edge (the 4-pixel white
stripes) to trim. The clip's source range adjusts accordingly.

### Splitting

1. Click on the timeline where you want to split (this moves the
   playhead).
2. Select the clip you want to split.
3. Press `S` (or Edit → Split at Playhead).

### Deleting

Select a clip and press `Delete` (or Edit → Delete Selected).

### Changing properties

Select a clip and edit its properties in the inspector:

- **Name**: display name.
- **Label**: color label (8 colors).
- **Speed**: 0.25× to 4×.
- **Level**: opacity (video) or gain (audio), 0.0 to 1.0.
- **Muted**: silences the clip during preview/export.

## Playback

- **Play/Pause**: `Space` or the `▶`/`⏸` button.
- **Skip to start**: `⏮` button.
- **Skip to end**: `⏭` button.
- **Frame step**: `←` / `→` arrows, or the `◀|`/`|▶` buttons.
- **Scrub**: drag the slider in the transport bar, or click on the
  timeline ruler.

## Saving and opening projects

- **Save**: `Ctrl+S` (or File → Save Project).
- **Save As**: `Ctrl+Shift+S` (or File → Save Project As…).
- **Open**: `Ctrl+O` (or File → Open Project…).

Project files are JSON. They reference media files by absolute
path — if you move a media file, edt will mark it as `offline`
(red) in the media pool. (Re-linking is a v0.2 feature.)

edt autosaves every 60 seconds to your OS's cache directory.

## Exporting

1. Press `Ctrl+E` (or File → Export…).
2. Choose the output path, format, codec, resolution, framerate,
   and quality (CRF).
3. Click "Start Export".

Export runs in the background. A progress bar shows frames
completed. You can cancel mid-export.

For single-track, single-asset timelines, edt uses ffmpeg's
`concat` filter for fast export. For more complex timelines, it
falls back to per-frame composition (slower; v0.1 renders solid
colors per clip in this mode — see [known-issues.md](../docs/known-issues.md)
E-002).

## Keyboard shortcuts

See the [README](../README.md#keyboard-shortcuts).

## Troubleshooting

### "ffmpeg binary not found"

Install FFmpeg and ensure `ffmpeg` and `ffprobe` are on your PATH.
Verify with `ffmpeg -version` in a terminal.

### Probe fails / asset is "offline"

The file may have moved or be inaccessible. Check that the file
exists at the path shown in the media pool. (Re-linking is a v0.2
feature; for now, re-import the file from its new location.)

### Export produces an empty / corrupt file

Check the status bar at the top right for error messages. Common
causes:

- The output path is not writable.
- The selected codec is not supported by your ffmpeg build (e.g.
  AV1 requires `libsvtav1`).
- The timeline is empty.

### Performance is slow

- Use the **Concat** export strategy when possible (single track,
  single asset).
- Reduce preview resolution in the export dialog (doesn't affect
  the project, just the preview).
- Close other heavy applications.
