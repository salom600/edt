//! Project model — top-level container that ties together media assets,
//! the timeline, and project-wide settings.

use crate::export::ExportSettings;
use crate::id::{Id, IdGenerator};
use crate::media::MediaAsset;
use crate::timeline::Timeline;
use crate::PROJECT_FORMAT_VERSION;
use serde::{Deserialize, Serialize};

/// Top-level project settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// Project name (also used as default file name).
    pub name: String,
    /// Edit framerate in frames per second.
    pub fps: f64,
    /// Edit canvas width.
    pub width: u32,
    /// Edit canvas height.
    pub height: u32,
    /// Background color (RGB) for areas not covered by a clip.
    pub background: [u8; 3],
    /// Sample rate for the project's audio bus, in Hz.
    pub audio_sample_rate: u32,
    /// Number of channels on the project's audio bus.
    pub audio_channels: u32,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            name: "Untitled Project".into(),
            fps: 30.0,
            width: 1920,
            height: 1080,
            background: [0, 0, 0],
            audio_sample_rate: 48_000,
            audio_channels: 2,
        }
    }
}

/// On-disk wrapper for a project file. Adds a version header so future
/// loaders can detect incompatibilities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectFile {
    pub format_version: u32,
    #[serde(flatten)]
    pub project: Project,
}

impl ProjectFile {
    pub fn wrap(project: Project) -> Self {
        Self {
            format_version: PROJECT_FORMAT_VERSION,
            project,
        }
    }
}

/// The project — the root of all editor state.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Project {
    pub settings: ProjectSettings,
    #[serde(default)]
    pub assets: Vec<MediaAsset>,
    #[serde(default)]
    pub timeline: Timeline,
    /// Default export settings used by the export dialog.
    #[serde(default)]
    pub export: ExportSettings,
    /// Absolute path the project was last saved to. `None` if never saved.
    #[serde(skip)]
    pub last_save_path: Option<std::path::PathBuf>,
}

impl Project {
    /// Create a new empty project with a fresh id generator.
    pub fn new() -> (Self, IdGenerator) {
        let gen = IdGenerator::new();
        let mut p = Self::default();
        // Pre-seed the timeline with two video and two audio tracks so the
        // user has somewhere to drop clips immediately.
        p.timeline = Timeline::with_default_tracks(&gen);
        (p, gen)
    }

    pub fn add_asset(&mut self, asset: MediaAsset) {
        self.assets.push(asset);
    }

    pub fn asset(&self, id: Id) -> Option<&MediaAsset> {
        self.assets.iter().find(|a| a.id == id)
    }

    pub fn asset_mut(&mut self, id: Id) -> Option<&mut MediaAsset> {
        self.assets.iter_mut().find(|a| a.id == id)
    }

    /// Total timeline duration across all tracks.
    pub fn duration(&self) -> crate::time::Time {
        self.timeline.duration()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_project_has_default_tracks() {
        let (p, _) = Project::new();
        assert!(p.timeline.tracks.len() >= 4, "expected seeded tracks");
        assert!(p
            .timeline
            .tracks
            .iter()
            .any(|t| t.kind == crate::timeline::TrackKind::Video));
        assert!(p
            .timeline
            .tracks
            .iter()
            .any(|t| t.kind == crate::timeline::TrackKind::Audio));
    }

    #[test]
    fn project_file_roundtrips() {
        let (p, _) = Project::new();
        let pf = ProjectFile::wrap(p);
        let s = serde_json::to_string_pretty(&pf).unwrap();
        let back: ProjectFile = serde_json::from_str(&s).unwrap();
        assert_eq!(back.format_version, PROJECT_FORMAT_VERSION);
    }
}
