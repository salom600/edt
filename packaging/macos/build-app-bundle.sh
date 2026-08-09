#!/usr/bin/env bash
# Build a macOS .app bundle for edt.
#
# Usage: ./build-app-bundle.sh [output-path]
#
# Prerequisites:
#   - cargo build --release has been run
set -euo pipefail

OUTPUT="${1:-edt.app}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

APP="$SCRIPT_DIR/$OUTPUT"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"

cp "$ROOT_DIR/target/release/edt" "$APP/Contents/MacOS/edt"
cp "$SCRIPT_DIR/Info.plist" "$APP/Contents/Info.plist"

# Icon (optional). If edt.icns exists in this directory, use it.
if [ -f "$SCRIPT_DIR/edt.icns" ]; then
  cp "$SCRIPT_DIR/edt.icns" "$APP/Contents/Resources/edt.icns"
fi

# Copy README and LICENSE into the bundle for easy access.
cp "$ROOT_DIR/README.md" "$APP/Contents/Resources/" 2>/dev/null || true
cp "$ROOT_DIR/LICENSE-MIT" "$APP/Contents/Resources/" 2>/dev/null || true

echo "Built $APP"
echo ""
echo "To open: open $APP"
echo "To create a DMG: create-dmg --volname edt $OUTPUT.dmg $APP"
