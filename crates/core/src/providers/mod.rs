//! Addon sources (CurseForge, GitHub, WowInterface, ...).
//! Each provider is a small blocking client behind this trait so the rest of
//! the app does not care where an addon comes from.

use crate::model::{Flavor, RemoteAddon, RemoteFile, SearchFilter};
use anyhow::Result;
use std::sync::LazyLock;

pub mod curseforge;
pub mod github;
pub mod hub;
pub mod wowinterface;

pub use curseforge::CurseForge;
pub use github::GitHub;
pub use hub::Hub;
pub use wowinterface::WowInterface;

/// Provider settings (from user prefs). `Default` points at the real services.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub curseforge_api_key: Option<String>,
    pub curseforge_base_url: String,
    pub github_token: Option<String>,
    pub github_base_url: String,
    pub hub_base_url: String,
    pub wowinterface_base_url: String,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            curseforge_api_key: None,
            curseforge_base_url: curseforge::DEFAULT_BASE_URL.into(),
            github_token: None,
            github_base_url: github::DEFAULT_API_BASE.into(),
            hub_base_url: hub::DEFAULT_BASE_URL.into(),
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
        Box::new(Hub {
            base_url: config.hub_base_url.clone(),
            github: GitHub {
                token: config.github_token.clone(),
                api_base: config.github_base_url.clone(),
            },
        }),
        // shortcut: WowInterface is left out because its files carry no game-version info, so
        // for Forever it could offer a retail build. Re-add once it can be filtered per flavor.
        Box::new(GitHub {
            token: config.github_token.clone(),
            api_base: config.github_base_url.clone(),
        }),
    ]
}

/// Whether a link is one of a provider's addon pages.
type IsLink = fn(&str) -> bool;

/// The sources an install link can name, in the order links are tried: a CurseForge addon page,
/// a WowUp Hub page `wowup.io/addons/<id>`, a GitHub repo URL or bare `owner/repo`.
const LINKS: [(&str, IsLink); 3] = [
    (curseforge::NAME, |s| curseforge::parse_url(s).is_some()),
    (hub::NAME, |s| hub::parse_url(s).is_some()),
    (github::NAME, |s| github::parse_url(s).is_some()),
];

/// The provider an install link names ([`LINKS`]), or None for anything else (search text).
pub fn link_source(input: &str) -> Option<&'static str> {
    let input = input.trim();
    LINKS
        .iter()
        .find(|(_, is)| is(input))
        .map(|(name, _)| *name)
}

/// "CurseForge, WowUp Hub or GitHub": the sources install links can name, for messages.
pub fn link_sources() -> String {
    let labels: Vec<&str> = LINKS.iter().map(|(name, _)| label(name)).collect();
    match labels.split_last() {
        Some((last, [])) => last.to_string(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
        None => String::new(),
    }
}

/// A provider's name as people read it ([`Provider::label`], "WowUpHub" → "WowUp Hub"); a name no
/// provider has stays as it is.
pub fn label(name: &str) -> &str {
    static LABELS: LazyLock<Vec<(&'static str, &'static str)>> = LazyLock::new(|| {
        all(&ProviderConfig::default())
            .iter()
            .map(|p| (p.name(), p.label()))
            .collect()
    });
    LABELS
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(name, |(_, l)| l)
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
    /// md5 fingerprint (hex) of the folder, matched by the WowUp Hub.
    pub hub_fingerprint: String,
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
    /// The name as people read it ("WowUp Hub").
    fn label(&self) -> &'static str {
        self.name()
    }
    /// Whether the provider is usable (e.g. has an API key).
    fn available(&self) -> bool {
        true
    }
    /// Addons matching `filter`. Only CurseForge has categories and orders; the others search by
    /// text ([`crate::discover::search`] decides who is asked). Empty if unsupported.
    fn search(&self, _flavor: Flavor, _filter: &SearchFilter) -> Result<Vec<RemoteAddon>> {
        Ok(vec![])
    }
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
    /// The addon's long description as plain text, None if the source has none.
    fn description(&self, _addon: &RemoteAddon) -> Result<Option<String>> {
        Ok(None)
    }
    /// Resolve an addon page / repo URL (install-by-URL). None if not this provider's URL.
    #[allow(clippy::wrong_self_convention)] // "from URL" reads better than any rename
    fn from_url(&self, _flavor: Flavor, _url: &str) -> Result<Option<RemoteAddon>> {
        Ok(None)
    }
}
