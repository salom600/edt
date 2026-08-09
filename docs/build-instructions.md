# Build instructions

## Prerequisites

- **Rust** stable (1.78+). Install via <https://rustup.rs/>.
- **FFmpeg** installed and on your PATH (`ffmpeg` and `ffprobe`).

### OS-specific FFmpeg install

| OS | Command |
|---|---|
| Debian / Ubuntu | `sudo apt install ffmpeg` |
| Fedora | `sudo dnf install ffmpeg` |
| Arch | `sudo pacman -S ffmpeg` |
| macOS | `brew install ffmpeg` |
| Windows | `choco install ffmpeg` (or download from ffmpeg.org) |

Verify:
```sh
ffmpeg -version
ffprobe -version
```

## Build

### Debug build (fast compile, slow runtime)

```sh
cargo build
```

### Release build (slow compile, fast runtime)

```sh
cargo build --release
```

The binary is at `target/debug/edt` or `target/release/edt`
(`edt.exe` on Windows).

### Run

```sh
cargo run --release
```

## Test

```sh
cargo test --workspace
```

Some tests in `edt-media` are `#[ignore]`d because they require a
sample video at `/tmp/sample.mp4`. To run them:

```sh
# Generate a 10-second test video
ffmpeg -f lavfi -i testsrc=duration=10:size=640x360:rate=30 -c:v libx264 /tmp/sample.mp4

cargo test --workspace -- --ignored
```

## Lint

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

## Doc

```sh
cargo doc --workspace --no-deps --open
```

## Cross-compile

Cross-compilation is not officially supported in v0.1. Use the
GitHub Actions matrix builds for cross-platform binaries.

If you need to cross-compile locally, install the appropriate
target and a cross-linker:

```sh
# Example: Linux x86_64 → aarch64 (for Raspberry Pi 4 etc.)
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

## Reproducible CI builds

The exact build commands used by GitHub Actions are in
[`.github/workflows/ci.yml`](../.github/workflows/ci.yml). To
reproduce a CI build locally:

```sh
rustup default stable
rustup component add rustfmt clippy
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```
