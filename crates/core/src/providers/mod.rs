//! Addon sources (CurseForge, GitHub, WowInterface, ...).
//! Each provider is a small blocking client behind this trait so the rest of
//! the app does not care where an addon comes from.

use crate::model::{Flavor, RemoteAddon};
use anyhow::Result;

/// One local addon folder, prepared for provider matching.
#[derive(Debug, Clone)]
pub struct ScannedFolder {
    pub name: String,
    /// CurseForge fingerprint (murmur2) of the folder.
    pub fingerprint: u32,
    /// Ids from TOC metadata, e.g. ("CurseForge", "1234") from `X-Curse-Project-ID`.
    pub toc_ids: Vec<(String, String)>,
}

/// A provider match for some scanned folders.
#[derive(Debug, Clone)]
pub struct FolderMatch {
    pub folder: String,
    pub addon: RemoteAddon,
    /// The release that is installed (if the provider could tell).
    pub installed_file: Option<crate::model::RemoteFile>,
}

pub trait Provider: Send + Sync {
    /// Stable provider name, persisted in `Addon::provider`.
    fn name(&self) -> &'static str;
    /// Whether the provider is usable (e.g. has an API key).
    fn available(&self) -> bool {
        true
    }
    fn search(&self, flavor: Flavor, query: &str) -> Result<Vec<RemoteAddon>>;
    /// Landing page list (popular / recently updated). Empty if unsupported.
    fn featured(&self, _flavor: Flavor) -> Result<Vec<RemoteAddon>> {
        Ok(vec![])
    }
    /// Fresh data for known addons (batch update check).
    fn get_many(&self, flavor: Flavor, external_ids: &[String]) -> Result<Vec<RemoteAddon>>;
    /// Match local folders. Providers that can't match return an empty list.
    fn match_folders(
        &self,
        _flavor: Flavor,
        _folders: &[ScannedFolder],
    ) -> Result<Vec<FolderMatch>> {
        Ok(vec![])
    }
    /// Resolve an addon page / repo URL (install-by-URL). None if not this provider's URL.
    fn from_url(&self, _flavor: Flavor, _url: &str) -> Result<Option<RemoteAddon>> {
        Ok(None)
    }
}
