//! Shared domain types. This is the contract between core modules and the UI.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Game flavor. Only Forever is supported for now; the enum keeps the door open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Flavor {
    /// "World of Warcraft: Forever" — folder `_forever_`, product `wowf`, interface 160xx.
    Forever,
}

/// One WoW client installation (e.g. `.../World of Warcraft/_forever_`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Installation {
    /// Stable id (derived from the path), used to key the addon database.
    pub id: String,
    pub label: String,
    pub flavor: Flavor,
    /// The client folder, e.g. `.../World of Warcraft/_forever_`.
    pub root: PathBuf,
    /// Client version from `.build.info`, e.g. "1.60.1.70938".
    pub version: Option<String>,
}

impl Installation {
    pub fn addons_dir(&self) -> PathBuf {
        self.root.join("Interface").join("AddOns")
    }
}

/// Release channel. Ordering matters: Stable < Beta < Alpha
/// ("Beta" channel accepts stable and beta files, etc.).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum Channel {
    #[default]
    Stable,
    Beta,
    Alpha,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyKind {
    Embedded,
    Required,
    Optional,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dependency {
    pub external_id: String,
    pub kind: DependencyKind,
}

/// A downloadable release of a remote addon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteFile {
    /// Provider release id (CurseForge file id, GitHub asset id, ...).
    pub id: String,
    pub version: String,
    pub channel: Channel,
    pub download_url: String,
    /// RFC 3339 timestamp.
    pub released_at: Option<String>,
    /// Game versions this file targets, e.g. ["1.60.1"].
    pub game_versions: Vec<String>,
    /// Top-level folders the zip contains (CurseForge `modules[].name`), if known.
    pub folders: Vec<String>,
    pub dependencies: Vec<Dependency>,
}

/// An addon as known by a provider (search result, featured list, sync result).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteAddon {
    pub provider: String,
    pub external_id: String,
    pub name: String,
    pub author: String,
    pub summary: String,
    pub url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub download_count: u64,
    /// Latest files for this flavor, newest first, at most one per channel is enough.
    pub files: Vec<RemoteFile>,
}

impl RemoteAddon {
    /// Newest file allowed by `channel` (Beta accepts Stable+Beta, Alpha accepts all).
    pub fn latest_for(&self, channel: Channel) -> Option<&RemoteFile> {
        self.files
            .iter()
            .filter(|f| f.channel <= channel)
            .max_by(|a, b| {
                a.released_at
                    .cmp(&b.released_at)
                    .then_with(|| a.id.cmp(&b.id))
            })
    }
}

/// Why an installed addon is not managed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IgnoreReason {
    /// The user chose to ignore it.
    User,
    /// No provider match.
    Unknown,
    /// Folder is a git checkout (developer addon).
    GitRepo,
}

/// An installed addon (one or more folders in `Interface/AddOns`), persisted per installation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Addon {
    /// Local uuid-like id.
    pub id: String,
    pub name: String,
    pub author: String,
    pub summary: String,
    /// Provider name ("CurseForge", "GitHub", "WowInterface") or None if unmatched.
    pub provider: Option<String>,
    pub external_id: Option<String>,
    pub url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub folders: Vec<String>,
    pub installed_version: Option<String>,
    pub installed_file_id: Option<String>,
    pub installed_at: Option<String>,
    /// TOC `## Interface` of the main folder, e.g. "1.60.1".
    pub game_version: Option<String>,
    pub channel: Channel,
    /// Latest known remote file for `channel` (filled by update checks).
    pub latest: Option<RemoteFile>,
    pub ignored: Option<IgnoreReason>,
    pub auto_update: bool,
    pub dependencies: Vec<Dependency>,
}

impl Addon {
    pub fn update_available(&self) -> bool {
        if self.ignored.is_some() {
            return false;
        }
        match (&self.latest, &self.installed_file_id) {
            (Some(latest), Some(installed)) => &latest.id != installed,
            (Some(latest), None) => Some(&latest.version) != self.installed_version.as_ref(),
            _ => false,
        }
    }
}
