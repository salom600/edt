# ADR 0009 — CI: GitHub Actions with per-platform matrix

**Date:** 2026-08-09  
**Status:** Accepted

## Context

edt must build and produce installable artifacts on Windows, Linux,
and macOS. We need a CI system that:

- Runs on all three operating systems.
- Is free for public open-source repositories.
- Supports artifact upload and release publishing.
- Has good Rust ecosystem support (pre-built actions for toolchain,
  caching).

## Decision

Use **GitHub Actions** with two workflows:

- `ci.yml` — lint + test on Linux, then build release binaries on a
  3-OS matrix (ubuntu-latest, windows-latest, macos-latest).
- `release.yml` — on tag push (`v*`), build artifacts on a 4-target
  matrix (adds macOS Intel via `macos-13`) and publish a GitHub
  Release with checksums.

## Consequences

### Positive

- **Free for public repos.** No usage limits we're likely to hit.
- **Per-platform matrix** gives us native builds on each OS without
  cross-compilation pain.
- **`Swatinem/rust-cache@v2`** caches the cargo registry and target
  directory, cutting rebuild times dramatically.
- **`softprops/action-gh-release@v2`** handles release creation and
  artifact attachment in a single step.
- **`actions/upload-artifact@v4`** lets us download artifacts from
  the Actions UI for manual testing.

### Negative

- **macOS runners are slower to start** (sometimes 5+ minutes queue
  time on the free tier). Acceptable for a release workflow that
  runs occasionally.
- **No self-hosted runners.** We rely on GitHub-hosted runners,
  which means we can't customize the OS image beyond what GitHub
  provides. For ffmpeg, we install it via the platform's package
  manager on each runner.
- **Windows builds are the slowest** in the matrix (MSVC + large
  dep tree). We mitigate with caching.

### Neutral

- We use `dtolnay/rust-toolchain@stable` rather than `rustup`
  directly because it's faster and handles component installation
  cleanly.
- The `RUSTFLAGS: "-D warnings"` env var turns clippy warnings into
  errors, enforcing code quality on every push.
