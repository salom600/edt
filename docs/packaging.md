# Packaging

This document describes how to package edt for distribution on each
platform. The GitHub Actions release workflow handles this
automatically; this document is for manual packaging.

## Linux

### .tar.gz (default)

The CI workflow produces `edt-linux-x86_64.tar.gz` containing the
`edt` binary plus README and LICENSE files.

### .deb (manual)

Use `cargo-deb`:

```sh
cargo install cargo-deb
cargo deb -p edt-app
# Output: target/debian/edt_0.1.0_amd64.deb
```

The `.desktop` entry and MIME type association are in
[`packaging/linux/`](../packaging/linux/).

### AppImage (manual)

Use `appimagetool`:

```sh
# Build edt
cargo build --release

# Stage the AppDir
mkdir -p AppDir/usr/bin
cp target/release/edt AppDir/usr/bin/edt
cp packaging/linux/edt.desktop AppDir/edt.desktop
cp packaging/linux/edt.png AppDir/edt.png
cp AppDir/usr/bin/edt AppDir/AppRun

# Build the AppImage
appimagetool AppDir edt-x86_64.AppImage
```

## macOS

### .app bundle (manual)

```sh
cargo build --release

mkdir -p "edt.app/Contents/MacOS"
mkdir -p "edt.app/Contents/Resources"
cp target/release/edt "edt.app/Contents/MacOS/edt"
cp packaging/macos/Info.plist "edt.app/Contents/Info.plist"
cp packaging/macos/edt.icns "edt.app/Contents/Resources/edt.icns" 2>/dev/null || true
```

### .dmg (manual)

Use `create-dmg`:

```sh
brew install create-dmg
create-dmg --volname "edt" --window-size 600 400 --icon-size 100 \
  --icon "edt.app" 150 110 --app-drop-link 450 110 \
  edt-0.1.0.dmg edt.app
```

### Code signing

v0.1.0 releases are **unsigned**. To sign a build you need a
Developer ID Application certificate. Then:

```sh
codesign --deep --force --options runtime \
  --sign "Developer ID Application: Your Name (TEAMID)" \
  --entitlements packaging/macos/edt.entitlements \
  edt.app

# Notarize
xcrun notarytool submit edt.dmg --apple-id you@example.com \
  --team-id TEAMID --password app-specific-password --wait
xcrun stapler staple edt.dmg
```

## Windows

### .zip (default)

The CI workflow produces `edt-windows-x86_64.zip` containing
`edt.exe` plus README and LICENSE files.

### .msi installer (manual)

Use `cargo-wix`:

```sh
cargo install cargo-wix
cargo wix -p edt-app
# Output: target/wix/edt-0.1.0-x86_64.msi
```

The WiX configuration is in
[`packaging/windows/edt.wxs`](../packaging/windows/edt.wxs).

### NSIS installer (manual)

Use `makensis`:

```sh
cargo build --release
makensis packaging/windows/edt.nsis
# Output: packaging/windows/edt-0.1.0-setup.exe
```

### Code signing

v0.1.0 releases are **unsigned**. To sign a build you need an
Authenticode certificate. Then:

```sh
signtool sign /fd SHA256 /a /tr http://timestamp.digicert.com \
  /td SHA256 /sha1 CERTIFICATE_THUMBPRINT \
  target/release/edt.exe
```

## Verification

Each release includes `.sha256` checksum files. Verify a download:

```sh
sha256sum -c edt-linux-x86_64.tar.gz.sha256
```

On macOS:

```sh
shasum -a 256 -c edt-macos-arm64.tar.gz.sha256
```

On Windows (PowerShell):

```powershell
$expected = (Get-Content edt-windows-x86_64.zip.sha256).Split(" ")[0]
$actual = (Get-FileHash edt-windows-x86_64.zip -Algorithm SHA256).Hash.ToLower()
if ($expected -eq $actual) { "OK" } else { "MISMATCH" }
```
