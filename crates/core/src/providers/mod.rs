//! Addon sources (CurseForge, GitHub, WowInterface, ...).
//! Each provider is a small blocking client behind this trait so the rest of
//! the app does not care where an addon comes from.

use crate::model::{Flavor, RemoteAddon, RemoteFile};
use anyhow::Result;

pub mod curseforge;
pub mod github;
pub mod wowinterface;

pub use curseforge::CurseForge;
pub use github::GitHub;
pub use wowinterface::WowInterface;

/// Provider settings (from user prefs). `Default` points at the real services.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub curseforge_api_key: Option<String>,
    pub curseforge_base_url: String,
    pub github_token: Option<String>,
    pub github_base_url: String,
    pub wowinterface_base_url: String,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            curseforge_api_key: None,
            curseforge_base_url: curseforge::DEFAULT_BASE_URL.into(),
            github_token: None,
            github_base_url: github::DEFAULT_API_BASE.into(),
            wowinterface_base_url: wowinterface::DEFAULT_BASE_URL.into(),
        }
    }
}

/// All providers, in matching priority order (CurseForge fingerprints first).
pub fn all(config: &ProviderConfig) -> Vec<Box<dyn Provider>> {
    vec![
        Box::new(CurseForge {
            api_key: config.curseforge_api_key.clone(),
            base_url: config.curseforge_base_url.clone(),
        }),
        Box::new(WowInterface {
            base_url: config.wowinterface_base_url.clone(),
        }),
        Box::new(GitHub {
            token: config.github_token.clone(),
            api_base: config.github_base_url.clone(),
        }),
    ]
}

/// Keeps the first (newest) file of each channel from a newest-first list.
fn newest_per_channel(files: impl Iterator<Item = RemoteFile>) -> Vec<RemoteFile> {
    let mut out: Vec<RemoteFile> = Vec::new();
    for f in files {
        if !out.iter().any(|o| o.channel == f.channel) {
            out.push(f);
        }
    }
    out
}

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
    pub installed_file: Option<RemoteFile>,
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
    #[allow(clippy::wrong_self_convention)] // "from URL" reads better than any rename
    fn from_url(&self, _flavor: Flavor, _url: &str) -> Result<Option<RemoteAddon>> {
        Ok(None)
    }
}
