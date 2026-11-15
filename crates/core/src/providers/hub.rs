//! WowUp Hub (hub.wowup.io): a public, keyless index of GitHub-hosted addons that publish
//! release metadata. Forever = gameType `forever`; a release counts when one of its
//! `game_versions` is for `forever`. Folders are matched by the Hub's md5 fingerprint.

use super::{FolderMatch, Provider, ScannedFolder, newest_per_channel};
use crate::http;
use crate::model::{Channel, Flavor, RemoteAddon, RemoteFile};
use crate::toc;
use anyhow::Result;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;

pub const NAME: &str = "WowUpHub";
pub const DEFAULT_BASE_URL: &str = "https://hub.wowup.io";

pub struct Hub {
    pub base_url: String,
}

fn game_type(flavor: Flavor) -> &'static str {
    match flavor {
        Flavor::Forever => "forever",
    }
}

impl Hub {
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    fn batch(&self, flavor: Flavor, ids: &[u64]) -> Result<Vec<HubAddon>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let res: AddonList = http::post_json(
            &self.url(&format!("/addons/batch/{}", game_type(flavor))),
            &json!({ "addonIds": ids }),
            &[],
        )?;
        Ok(res.addons)
    }
}

impl Provider for Hub {
    fn name(&self) -> &'static str {
        NAME
    }

    fn search(&self, flavor: Flavor, query: &str) -> Result<Vec<RemoteAddon>> {
        let res: AddonList = http::get_json(
            &self.url(&format!("/addons/search/{}", game_type(flavor))),
            &[("query", query), ("limit", "50")],
            &[],
        )?;
        Ok(to_addons(flavor, &res.addons))
    }

    fn featured(&self, flavor: Flavor) -> Result<Vec<RemoteAddon>> {
        let res: Featured = http::get_json(
            &self.url(&format!("/addons/featured/{}", game_type(flavor))),
            &[("count", "60"), ("recent", "30")],
            &[],
        )?;
        let mut seen = HashSet::new();
        let unique: Vec<HubAddon> = res
            .addons
            .into_iter()
            .chain(res.recent)
            .filter(|a| seen.insert(a.id))
            .collect();
        Ok(to_addons(flavor, &unique))
    }

    fn get_many(&self, flavor: Flavor, external_ids: &[String]) -> Result<Vec<RemoteAddon>> {
        let ids: Vec<u64> = external_ids.iter().filter_map(|i| i.parse().ok()).collect();
        Ok(self
            .batch(flavor, &ids)?
            .iter()
            .map(|a| to_addon(flavor, a))
            .collect())
    }

    /// `POST /addons/fingerprint` with the folders' md5 fingerprints. Only exact matches whose
    /// matched release is a Forever release count; latest releases come from one batch call.
    fn match_folders(&self, flavor: Flavor, folders: &[ScannedFolder]) -> Result<Vec<FolderMatch>> {
        let fps: Vec<&str> = folders
            .iter()
            .map(|f| f.hub_fingerprint.as_str())
            .filter(|f| !f.is_empty())
            .collect();
        if fps.is_empty() {
            return Ok(vec![]);
        }
        let res: FingerprintResult = http::post_json(
            &self.url("/addons/fingerprint"),
            &json!({ "fingerprints": fps }),
            &[],
        )?;
        let matches: Vec<(&ScannedFolder, &FpMatch, RemoteFile)> = folders
            .iter()
            .filter(|f| !f.hub_fingerprint.is_empty())
            .filter_map(|folder| {
                res.exact_matches.iter().find_map(|m| {
                    let rel = &m.matched_release;
                    let hit = rel.addon_folders.iter().any(|af| {
                        af.fingerprint == folder.hub_fingerprint
                            && af.folder_name.eq_ignore_ascii_case(&folder.name)
                    });
                    if !hit {
                        return None;
                    }
                    Some((folder, m, to_file(flavor, rel)?))
                })
            })
            .collect();
        let mut ids: Vec<u64> = matches.iter().map(|(_, m, _)| m.id).collect();
        ids.sort_unstable();
        ids.dedup();
        let latest = self.batch(flavor, &ids)?;
        Ok(matches
            .into_iter()
            .filter_map(|(folder, m, installed)| {
                let a = latest.iter().find(|a| a.id == m.id)?;
                Some(FolderMatch {
                    folder: folder.name.clone(),
                    addon: to_addon(flavor, a),
                    installed_file: Some(installed),
                })
            })
            .collect())
    }

    fn from_url(&self, flavor: Flavor, url: &str) -> Result<Option<RemoteAddon>> {
        let Some(id) = parse_url(url) else {
            return Ok(None);
        };
        let res: Single = http::get_json(&self.url(&format!("/addons/{id}")), &[], &[])?;
        let addon = to_addon(flavor, &res.addon);
        anyhow::ensure!(
            !addon.files.is_empty(),
            "{} has no World of Warcraft: Forever release",
            addon.name
        );
        Ok(Some(addon))
    }
}

/// `https://wowup.io/addons/<id>[/...]` → id.
pub(super) fn parse_url(url: &str) -> Option<u64> {
    let rest = url.trim().split_once("://").map_or(url.trim(), |(_, r)| r);
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    rest.strip_prefix("wowup.io/addons/")?
        .split(['/', '?', '#'])
        .next()?
        .parse()
        .ok()
}

/// Addons with at least one Forever release (search lists hold other flavors too).
fn to_addons(flavor: Flavor, addons: &[HubAddon]) -> Vec<RemoteAddon> {
    addons
        .iter()
        .map(|a| to_addon(flavor, a))
        .filter(|a| !a.files.is_empty())
        .collect()
}

fn to_addon(flavor: Flavor, a: &HubAddon) -> RemoteAddon {
    let mut releases: Vec<&HubRelease> = a.releases.iter().collect();
    releases.sort_by(|x, y| y.published_at.cmp(&x.published_at));
    RemoteAddon {
        provider: NAME.into(),
        external_id: a.id.to_string(),
        name: a.repository_name.clone(),
        author: a.owner_name.clone(),
        summary: summary(&a.description),
        url: Some(format!("https://wowup.io/addons/{}", a.id)),
        thumbnail_url: a.image_url.clone().or_else(|| a.owner_image_url.clone()),
        download_count: a.total_download_count,
        files: newest_per_channel(releases.into_iter().filter_map(|r| to_file(flavor, r))),
        source_url: Some(a.repository.clone()).filter(|r| !r.is_empty()),
        categories: vec![],
        updated_at: a
            .releases
            .iter()
            .filter_map(|r| r.published_at.clone())
            .max(),
    }
}

/// None unless the release targets `flavor` and has an http(s) download URL.
fn to_file(flavor: Flavor, r: &HubRelease) -> Option<RemoteFile> {
    let versions: Vec<String> = r
        .game_versions
        .iter()
        .filter(|g| g.game_type == game_type(flavor))
        .filter_map(|g| toc::game_version(&g.interface))
        .collect();
    let any = r
        .game_versions
        .iter()
        .any(|g| g.game_type == game_type(flavor));
    let url_ok = r.download_url.starts_with("https://") || r.download_url.starts_with("http://");
    (any && url_ok).then(|| RemoteFile {
        // The GitHub asset id, so a Hub install and a GitHub install of the same release agree.
        id: if r.external_id.is_empty() {
            r.id.to_string()
        } else {
            r.external_id.clone()
        },
        version: r.tag_name.clone(),
        channel: if r.prerelease {
            Channel::Beta
        } else {
            Channel::Stable
        },
        download_url: r.download_url.clone(),
        released_at: r.published_at.clone(),
        game_versions: versions,
        folders: r
            .addon_folders
            .iter()
            .map(|f| f.folder_name.clone())
            .collect(),
        dependencies: vec![],
    })
}

/// The HTML description as one short line of text.
fn summary(html: &str) -> String {
    let text = crate::html::to_text(html, 240);
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

// --- API shapes (only the fields we use; defaults so schema drift doesn't break us) ---

#[derive(Deserialize, Default)]
#[serde(default)]
struct AddonList {
    addons: Vec<HubAddon>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Featured {
    addons: Vec<HubAddon>,
    recent: Vec<HubAddon>,
}

#[derive(Deserialize)]
struct Single {
    addon: HubAddon,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct FingerprintResult {
    exact_matches: Vec<FpMatch>,
}

#[derive(Deserialize)]
struct FpMatch {
    id: u64,
    matched_release: HubRelease,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct HubAddon {
    id: u64,
    repository: String,
    repository_name: String,
    owner_name: String,
    description: String,
    image_url: Option<String>,
    owner_image_url: Option<String>,
    total_download_count: u64,
    releases: Vec<HubRelease>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct HubRelease {
    id: u64,
    external_id: String,
    tag_name: String,
    download_url: String,
    published_at: Option<String>,
    prerelease: bool,
    game_versions: Vec<GameVersion>,
    #[serde(rename = "addonFolders")]
    addon_folders: Vec<AddonFolder>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct GameVersion {
    game_type: String,
    interface: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct AddonFolder {
    folder_name: String,
    fingerprint: String,
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_hub_urls_and_summaries() {
        assert_eq!(
            super::parse_url("https://wowup.io/addons/47671"),
            Some(47671)
        );
        assert_eq!(super::parse_url("wowup.io/addons/5/questie"), Some(5));
        assert_eq!(super::parse_url("https://github.com/a/b"), None);
        assert_eq!(
            super::summary("<h1>Questie</h1><p>Quests &amp; more</p>"),
            "Questie Quests & more"
        );
    }
}
