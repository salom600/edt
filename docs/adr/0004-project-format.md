# ADR 0004 — Project file format: JSON

**Date:** 2026-08-09  
**Status:** Accepted

## Context

A project file format needs to:

- Store the full project state (settings, assets, timeline, effects,
  transitions, export settings).
- Be human-readable for debugging and version-control friendliness.
- Be forward-compatible (old projects load in new edt versions, with
  or without migration).
- Be fast to read/write (projects rarely exceed a few MB).

Candidates:

1. **JSON** via `serde_json`.
2. **TOML** via `toml`.
3. **SQLite** via `rusqlite`.
4. **MessagePack** via `rmp-serde`.
5. **Custom binary format**.

## Decision

Use **pretty-printed JSON** wrapped in a `ProjectFile` envelope that
carries a `format_version` field.

## Consequences

### Positive

- **Human-readable and diffable.** Users can `git diff` their
  project files, which is great for version control of edit
  decisions.
- **`serde_json` is already a transitive dependency** of many crates
  we use — no new deps.
- **Forward compatibility** is straightforward: bump
  `PROJECT_FORMAT_VERSION`, add a migration function that reads
  old fields and writes new ones.
- **Atomic writes** are trivial: write to `.<name>.tmp`, then
  `rename` over the destination.

### Negative

- **Slower than binary** for very large projects. A 10k-clip project
  might take 100ms to serialize. Acceptable for v0.1; can be revisited
  if real users hit this.
- **No random access.** Loading a project reads the entire file into
  memory. Fine for v0.1; would matter if we added streaming load of
  huge projects.
- **No schema validation** beyond what `serde` provides. A
  hand-edited project file with a typo produces a `serde_json::Error`
  rather than a friendly message. We can improve error messages in
  the storage layer.

### Neutral

- We considered **SQLite**. It would give us random access,
  transactions, and better performance for huge projects, but adds
  complexity (bundled C library, schema migrations, etc.) for very
  little gain at MVP scale. A future v1.0 might switch if needed.
- **TOML** is great for config but degrades badly for deeply-nested
  data like a full timeline with effects. Not suitable here.
