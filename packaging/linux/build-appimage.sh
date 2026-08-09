#!/usr/bin/env bash
# Build an AppImage for edt.
#
# Usage: ./build-appimage.sh [output-path]
#
# Prerequisites:
#   - cargo build --release has been run
#   - appimagetool is on PATH (https://github.com/AppImage/AppImageKit)
set -euo pipefail

OUTPUT="${1:-edt-x86_64.AppImage}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

APPDIR="$SCRIPT_DIR/AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" \
         "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps" \
         "$APPDIR/usr/share/mime/packages"

cp "$ROOT_DIR/target/release/edt" "$APPDIR/usr/bin/edt"
cp "$SCRIPT_DIR/edt.desktop" "$APPDIR/usr/share/applications/edt.desktop"
cp "$SCRIPT_DIR/edt-mime.xml" "$APPDIR/usr/share/mime/packages/edt.xml"

# Icon (placeholder: use a 256x256 PNG with the edt logo)
# For real builds, generate this from a vector source.
if [ -f "$SCRIPT_DIR/edt.png" ]; then
  cp "$SCRIPT_DIR/edt.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/edt.png"
fi

# Top-level .desktop file (AppImage convention)
cp "$SCRIPT_DIR/edt.desktop" "$APPDIR/edt.desktop"

# AppRun: use the binary itself as AppRun (it works because edt is a
# single executable with no shared libraries beyond system ones).
cp "$ROOT_DIR/target/release/edt" "$APPDIR/AppRun"
chmod +x "$APPDIR/AppRun"

# Build the AppImage
appimagetool "$APPDIR" "$OUTPUT"
echo "Built $OUTPUT"
