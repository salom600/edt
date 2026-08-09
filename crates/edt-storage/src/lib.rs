//! edt-storage — Project save/load.
//!
//! Projects are persisted as pretty-printed JSON wrapped in a
//! [`ProjectFile`] envelope that carries a `format_version` field. This
//! allows future versions of edt to detect old project files and migrate
//! them (or refuse to load them with a clear error).
//!
//! ## Why JSON and not SQLite / TOML?
//!
//! - **JSON** is human-readable and diffable — useful for debugging and
//!   for version-control-friendly project files. `serde_json` is also
//!   already a transitive dependency via many other crates.
//! - **SQLite** would add complexity for very little gain at MVP scale
//!   (a project rarely exceeds a few hundred clips). It also complicates
//!   cross-platform builds slightly (bundled SQLite is fine, but still).
//! - **TOML** is great for config but degrades badly for deeply-nested
//!   data like a full timeline.
//!
//! ## Atomic writes
//!
//! Saves go to a sibling `.tmp` file first, then are atomically renamed
//! over the destination. This prevents corruption if the process is
//! killed mid-write or the disk fills up.

use edt_core::project::{Project, ProjectFile};
use edt_core::PROJECT_FORMAT_VERSION;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("project file format version {found} is not supported (expected {supported})")]
    UnsupportedVersion { found: u32, supported: u32 },
    #[error("project file is malformed: {0}")]
    Malformed(String),
}

/// Save `project` to `path` atomically. The file is written as
/// pretty-printed JSON with a trailing newline.
pub fn save_project(project: &Project, path: &Path) -> Result<(), StorageError> {
    let wrapper = ProjectFile::wrap(project.clone());
    let json = serde_json::to_string_pretty(&wrapper)?;
    let json = format!("{json}\n");

    let mut tmp_path = path.to_path_buf();
    let stem = tmp_path
        .file_name()
        .map(|s| s.to_owned())
        .unwrap_or_default();
    let mut tmp_name = std::ffi::OsString::from(".");
    tmp_name.push(stem);
    tmp_name.push(".tmp");
    tmp_path.set_file_name(tmp_name);

    std::fs::write(&tmp_path, json.as_bytes())?;
    // fs::rename is atomic on the same filesystem on POSIX, and on Windows
    // since Windows 10 1709 (long obsolete by 2026).
    std::fs::rename(&tmp_path, path)?;
    tracing::info!(path = %path.display(), "project saved");
    Ok(())
}

/// Load a project from `path`. Returns an error if the file does not
/// exist, is not valid JSON, or has an unsupported format version.
pub fn load_project(path: &Path) -> Result<Project, StorageError> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        StorageError::Io(std::io::Error::new(
            e.kind(),
            format!("reading project file {}: {}", path.display(), e),
        ))
    })?;
    let wrapper: ProjectFile = serde_json::from_str(&raw)?;
    if wrapper.format_version > PROJECT_FORMAT_VERSION {
        return Err(StorageError::UnsupportedVersion {
            found: wrapper.format_version,
            supported: PROJECT_FORMAT_VERSION,
        });
    }
    if wrapper.format_version < PROJECT_FORMAT_VERSION {
        // For 0.1.0 there is no older version, but the branch is here for
        // future migrations.
        tracing::warn!(
            found = wrapper.format_version,
            supported = PROJECT_FORMAT_VERSION,
            "loading older project format — applying migrations"
        );
    }
    let mut project = wrapper.project;
    project.last_save_path = Some(path.to_path_buf());
    Ok(project)
}

/// Default location for autosave files: `<cache_dir>/edt/autosave/<name>.json`.
pub fn autosave_path(project_name: &str) -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("dev", "edt", "edt")?;
    let mut p = dirs.cache_dir().to_path_buf();
    p.push("autosave");
    p.push(project_name);
    p.set_extension("json");
    Some(p)
}

/// Write a project to the autosave location. Silently no-ops if the
/// cache directory cannot be determined (e.g. on weird embedded systems).
pub fn autosave(project: &Project) -> Result<PathBuf, StorageError> {
    let path = autosave_path(&project.settings.name)
        .ok_or_else(|| StorageError::Malformed("cannot determine autosave directory".into()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    save_project(project, &path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edt_core::project::Project;
    use tempfile::NamedTempFile;

    #[test]
    fn save_and_load_roundtrips() {
        let (p, _) = Project::new();
        let f = NamedTempFile::new().unwrap();
        save_project(&p, f.path()).unwrap();
        let loaded = load_project(f.path()).unwrap();
        assert_eq!(loaded.settings.name, p.settings.name);
        assert_eq!(loaded.timeline.tracks.len(), p.timeline.tracks.len());
    }

    #[test]
    fn load_rejects_future_versions() {
        let raw = format!(
            r#"{{"format_version": {}, "settings": {{"name":"x","fps":30.0,"width":1,"height":1,"background":[0,0,0],"audio_sample_rate":48000,"audio_channels":2}}, "assets": [], "timeline": {{"tracks":[],"transitions":[]}}, "export": {{"format":"mp4","video_codec":"h264","audio_codec":"aac","width":1,"height":1,"fps":30.0,"video_bitrate":0,"crf":20,"audio_bitrate":192000,"audio_sample_rate":48000,"audio_channels":2,"hardware_accel":false}} }}"#,
            PROJECT_FORMAT_VERSION + 1
        );
        let f = NamedTempFile::new().unwrap();
        std::fs::write(f.path(), raw).unwrap();
        let err = load_project(f.path()).unwrap_err();
        assert!(matches!(err, StorageError::UnsupportedVersion { .. }));
    }
}
