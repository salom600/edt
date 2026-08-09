# Development guide

## Setting up a local dev environment

```sh
git clone https://github.com/salom600/edt.git
cd edt
cargo build
cargo test --workspace
```

You also need FFmpeg installed (see [build-instructions.md](build-instructions.md)).

## Project structure

See [architecture.md](architecture.md) for the high-level overview.
Key files:

- `Cargo.toml` — workspace root.
- `crates/edt-core/src/` — data model.
- `crates/edt-app/src/app.rs` — top-level UI shell.
- `crates/edt-app/src/ui/` — UI panels.
- `.github/workflows/` — CI/CD.

## Coding standards

- Run `cargo fmt --all` before committing.
- Run `cargo clippy --workspace --all-targets -- -D warnings` before
  committing. CI enforces this.
- Every public function has a doc comment.
- Every module has a `//!` doc comment at the top.
- Tests live in `#[cfg(test)] mod tests { ... }` at the bottom of
  each file.
- Use `tracing::info!` / `warn!` / `error!` for logging, not `println!`.

## Commit message conventions

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add ripple-delete
fix: handle negative clip durations on trim
docs: update build instructions
ci: cache ffmpeg install
refactor: extract clip splitting into a command
test: add property tests for time arithmetic
chore: bump egui to 0.30
```

## Adding a new effect kind

1. Add a variant to `edt_core::effect::EffectKind`.
2. Add a default instance to `EffectKind::default()` if appropriate.
3. Update the inspector UI in `crates/edt-app/src/ui/inspector.rs`
   to show the effect's parameters.
4. Update the render pipeline in `crates/edt-render/src/compose.rs`
   to apply the effect.
5. Add a test in `edt-render` that verifies the effect changes the
   output frame.

## Adding a new command (undo/redo)

1. Define a struct implementing `crate::commands::Command`.
2. Implement `apply` and `revert`.
3. Push the command onto the undo stack from the UI handler:
   ```rust
   self.undo.push(Box::new(MyCmd { ... }), &self.state);
   ```
4. Add a test in `crates/edt-app/src/commands.rs`.

## Debugging

### Logging

edt uses `tracing`. Set the `RUST_LOG` env var to control verbosity:

```sh
RUST_LOG=debug cargo run
RUST_LOG=edt_media=trace,edt_app=debug cargo run
```

### Panic hook

A panic hook in `main.rs` logs panics via `tracing::error` before
the process dies. This makes panics visible in CI logs.

### Inspecting project files

Project files are pretty-printed JSON. Open them in your editor to
inspect the structure.

### Autosave files

Autosave files live in `<cache_dir>/edt/autosave/`. On Linux this
is `~/.cache/edt/autosave/`; on macOS `~/Library/Caches/edt/autosave/`;
on Windows `%LOCALAPPDATA%\edt\autosave\`.

## Profiling

For performance work, use `cargo flamegraph`:

```sh
cargo install flamegraph
cargo flamegraph --release --bin edt
```

Or use `perf` directly:

```sh
cargo build --release
perf record -g target/release/edt
perf report
```

## CI debugging

To reproduce a CI failure locally:

1. Note the OS and Rust version from the failing job's log.
2. `rustup default stable` and `rustup update` to match.
3. Run the exact commands from the workflow's `steps:` section.

To re-run a failed workflow:

- Push an empty commit: `git commit --allow-empty -m "ci: retry" && git push`
- Or use the "Re-run jobs" button in the Actions UI.
