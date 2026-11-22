//! What the UI needs from the outside world. Every call is blocking and runs on a worker
//! (`worker.rs`), never on the UI thread. The real implementation wraps `addon-core`;
//! [`crate::mock::MockBackend`] serves canned Forever data for snapshots and tests.

use crate::theme::ThemeId;
use addon_core::discover::Entry;
use addon_core::self_update::Release;
use addon_core::{Addon, Category, Channel, Installation, RemoteAddon, RemoteFile, SearchFilter};
use anyhow::Result;
use std::path::{Path, PathBuf};

/// User settings the backend needs (providers read the API key).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
    pub curseforge_key: Option<String>,
    pub default_channel: Channel,
    /// Id of the installation shown last.
    pub installation: Option<String>,
    pub theme: ThemeId,
    /// My Addons' row order (F2.6).
    pub addon_order: AddonOrder,
}

/// A My Addons column the list can be ordered by (F2.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortBy {
    #[default]
    Name,
    Game,
    Status,
}

/// My Addons' row order: a column, ascending unless `desc`. Status ascending puts what needs the
/// user first (errors, updates), Game ascending the oldest game version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AddonOrder {
    pub by: SortBy,
    pub desc: bool,
}

impl AddonOrder {
    const BY: [(SortBy, &str); 3] = [
        (SortBy::Name, "name"),
        (SortBy::Game, "game"),
        (SortBy::Status, "status"),
    ];

    /// The id kept in prefs (`status-desc`).
    pub fn id(self) -> String {
        let by = Self::BY
            .iter()
            .find(|(b, _)| *b == self.by)
            .map_or("name", |(_, n)| n);
        format!("{by}-{}", if self.desc { "desc" } else { "asc" })
    }

    /// The order stored as `id`; anything unknown is the default.
    pub fn from_id(id: &str) -> AddonOrder {
        let (by, dir) = id.split_once('-').unwrap_or((id, "asc"));
        match Self::BY.iter().find(|(_, n)| *n == by) {
            Some(&(by, _)) => AddonOrder {
                by,
                desc: dir == "desc",
            },
            None => AddonOrder::default(),
        }
    }
}

/// A result plus non-fatal problems to show (e.g. one provider offline).
pub type Warned<T> = (T, Vec<String>);

/// Search or landing results from several sources (F4.8): one entry per addon, plus the sources
/// that failed as `(provider, error)` while the others answered.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Found {
    pub entries: Vec<Entry>,
    pub failed: Vec<(String, String)>,
}

// The search policy and source labels live with the sources in core; re-exported for the views.
pub use addon_core::discover::{curseforge_only, sort_entries};
pub use addon_core::providers::label as source_label;

/// A housekeeping job on the selected installation (F6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Maintenance {
    /// Delete `Cache` (or only `Cache/WDB`).
    ClearCache { wdb_only: bool },
    /// Delete `WTF/**/*.lua.bak` and `WTF/*.old`.
    CleanBackups,
    /// Zip `WTF` into the app's data dir.
    BackupWtf,
}

/// A WTF backup zip.
#[derive(Debug, Clone, PartialEq)]
pub struct Backup {
    pub name: String,
    /// RFC 3339.
    pub created_at: String,
    pub size_bytes: u64,
}

pub trait Backend: Send + Sync {
    fn load_settings(&self) -> Settings;
    fn save_settings(&self, settings: &Settings) -> Result<()>;
    /// Stored plus auto-detected installations.
    fn installations(&self) -> Result<Vec<Installation>>;
    /// Validate and store a folder the user picked (any of WoW root, `_forever_`, `Wow.exe` dir).
    fn add_installation(&self, path: &Path) -> Result<Installation>;
    /// Scan + provider matching, merged with the stored list (first scan: WowUp records too).
    fn scan(&self, inst: &Installation) -> Result<Warned<Vec<Addon>>>;
    /// Returns `addons` with `latest` refreshed.
    fn check_updates(&self, inst: &Installation, addons: &[Addon]) -> Result<Warned<Vec<Addon>>>;
    /// Install `addon.latest`, the newest file for its channel (update and reinstall alike).
    fn update(&self, inst: &Installation, addon: &Addon, progress: &dyn Fn(f32)) -> Result<Addon>;
    /// The zip of `file` the user downloaded by hand, if it sits in their Downloads folder with
    /// exactly the size and SHA-1 its source publishes (F3.5).
    fn find_download(&self, file: &RemoteFile) -> Option<PathBuf>;
    /// Where downloads land, for the "Install from file" dialog to open in.
    fn downloads_dir(&self) -> Option<PathBuf>;
    /// Installs the zip at `path` as the addon's latest file, after checking it is exactly that
    /// file; `remove` then deletes it and every identical copy in Downloads (F3.5).
    fn install_downloaded(
        &self,
        inst: &Installation,
        addon: &Addon,
        path: &Path,
        remove: bool,
    ) -> Result<Addon>;
    fn install(
        &self,
        inst: &Installation,
        remote: &RemoteAddon,
        channel: Channel,
        progress: &dyn Fn(f32),
    ) -> Result<Addon>;
    fn remove(&self, inst: &Installation, addon: &Addon) -> Result<()>;
    /// Store the user's choices on `addon`'s record (ignore, channel), nothing else: a save that
    /// waited behind an update must not put the old version and folders back (#136).
    fn save_addon(&self, inst: &Installation, addon: &Addon) -> Result<()>;
    /// Search every available source by text, grouped per addon (F4.1, F4.8); a category or
    /// browsing without text asks CurseForge alone (F4.6, see [`curseforge_only`]). Fails only
    /// when no source answered.
    fn search(&self, filter: &SearchFilter) -> Result<Found>;
    /// The sources' landing lists (popular / recently updated), grouped like `search`.
    fn featured(&self) -> Result<Found>;
    /// CurseForge's addon categories (cached on disk).
    fn categories(&self) -> Result<Vec<Category>>;
    /// The addon's long description as plain text, if its provider has one (F4.5).
    fn description(&self, remote: &RemoteAddon) -> Result<Option<String>>;
    /// Resolve a CurseForge page, WowUp Hub page or GitHub repo URL (F4.4).
    fn resolve_url(&self, url: &str) -> Result<RemoteAddon>;
    fn game_running(&self, inst: &Installation) -> bool;
    /// Runs a maintenance job; returns a one-line result ("Freed 412 MB").
    fn maintenance(&self, inst: &Installation, job: Maintenance) -> Result<String>;
    fn backups(&self, inst: &Installation) -> Result<Vec<Backup>>;
    /// Import WowUp's addon records for `inst`; returns how many were imported.
    fn import_wowup(&self, inst: &Installation) -> Result<usize>;
    /// Show a folder in the system file manager.
    fn open_folder(&self, path: &Path) -> Result<()>;
    /// Image bytes of an addon thumbnail (cached on disk).
    fn thumbnail(&self, url: &str) -> Result<Vec<u8>>;
    /// The original class icons for the class chips (F4.6), from the cache or downloaded once;
    /// `each(slug, None)` for a class whose icon could not be had (its glyph stays).
    fn class_icons(&self, each: &mut dyn FnMut(&str, Option<Vec<u8>>));
    /// The app's newest release if it is newer than this build (F8.4).
    fn app_release(&self) -> Result<Option<Release>>;
    /// Whether this build can replace itself (it runs as an AppImage); otherwise updates are by hand.
    fn self_updating(&self) -> bool;
    /// Whether to look for a new version at every start, not only when told.
    fn checks_app_updates(&self) -> bool;
    /// Replace the app's own file with `release` (checked against its checksums); `progress` 0..=1.
    fn install_app_release(&self, release: &Release, progress: &dyn Fn(f32)) -> Result<()>;
    /// Start the (updated) app as a new process; the caller quits.
    fn launch_app(&self) -> Result<()>;
}
