//! What the UI needs from the outside world. Every call is blocking and runs on a worker
//! (`worker.rs`), never on the UI thread. The real implementation wraps `addon-core`;
//! [`crate::mock::MockBackend`] serves canned Forever data for snapshots and tests.

use crate::theme::ThemeId;
use addon_core::discover::Entry;
use addon_core::{Addon, Category, Channel, Installation, RemoteAddon, SearchFilter, Sort};
use anyhow::Result;
use std::path::Path;

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

/// Whether a search can only ask CurseForge: a category (or class) filter, or browsing without
/// text, which only CurseForge supports.
pub fn curseforge_only(filter: &SearchFilter) -> bool {
    filter.category.is_some() || filter.text.trim().is_empty()
}

/// Orders grouped results for a non-default sort (the sources rank on their own; popularity keeps
/// the grouped order). Stable, so ties keep it too.
pub fn sort_entries(entries: &mut [Entry], sort: Sort) {
    let newest = |e: &Entry| e.sources.iter().filter_map(|r| r.updated_at.clone()).max();
    let downloads = |e: &Entry| e.sources.iter().map(|r| r.download_count).max();
    match sort {
        Sort::Popularity => {}
        Sort::LastUpdated => entries.sort_by_key(|e| std::cmp::Reverse(newest(e))),
        Sort::Name => entries.sort_by_key(|e| e.preferred().name.to_lowercase()),
        Sort::TotalDownloads => entries.sort_by_key(|e| std::cmp::Reverse(downloads(e))),
    }
}

/// A provider's name as people read it ("WowUpHub" → "WowUp Hub").
pub fn source_label(provider: &str) -> &str {
    match provider {
        addon_core::providers::hub::NAME => "WowUp Hub",
        other => other,
    }
}

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
    /// Install (or re-download, `reinstall`) the latest file for the addon's channel.
    fn update(
        &self,
        inst: &Installation,
        addon: &Addon,
        reinstall: bool,
        progress: &dyn Fn(f32),
    ) -> Result<Addon>;
    fn install(
        &self,
        inst: &Installation,
        remote: &RemoteAddon,
        channel: Channel,
        progress: &dyn Fn(f32),
    ) -> Result<Addon>;
    fn remove(&self, inst: &Installation, addon: &Addon) -> Result<()>;
    /// Persist record changes (channel, ignore).
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
}
