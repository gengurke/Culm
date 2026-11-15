//! CurseForge (api.curseforge.com, needs a user API key). Forever = gameVersionTypeId 88568.

use super::{FolderMatch, Provider, ScannedFolder, newest_per_channel};
use crate::html;
use crate::http;
use crate::model::{
    Category, Channel, Dependency, DependencyKind, Flavor, RemoteAddon, RemoteFile, SearchFilter,
    Sort,
};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;

pub const NAME: &str = "CurseForge";
pub const DEFAULT_BASE_URL: &str = "https://api.curseforge.com";
const GAME_ID: u32 = 1;
const FOREVER_TYPE_ID: u32 = 88568;
/// The "Addons" class of game 1: the root of the addon category tree.
const ADDONS_CLASS_ID: u32 = 1;

pub struct CurseForge {
    pub api_key: Option<String>,
    pub base_url: String,
}

impl CurseForge {
    fn key(&self) -> Result<&str> {
        self.api_key
            .as_deref()
            .filter(|k| !k.trim().is_empty())
            .context("CurseForge needs an API key (Settings)")
    }

    fn get<T: for<'de> Deserialize<'de>>(&self, path: &str, query: &[(&str, &str)]) -> Result<T> {
        let url = format!("{}{path}", self.base_url);
        let env: Envelope<T> = http::get_json(&url, query, &[("x-api-key", self.key()?)])?;
        Ok(env.data)
    }

    fn post<T: for<'de> Deserialize<'de>>(&self, path: &str, body: serde_json::Value) -> Result<T> {
        let url = format!("{}{path}", self.base_url);
        let env: Envelope<T> = http::post_json(&url, &body, &[("x-api-key", self.key()?)])?;
        Ok(env.data)
    }

    /// Forever addons matching `filter` (text, category, order), mods without a Forever file dropped.
    pub fn search_filtered(&self, filter: &SearchFilter) -> Result<Vec<RemoteAddon>> {
        let (game, ty) = (GAME_ID.to_string(), FOREVER_TYPE_ID.to_string());
        // docs.curseforge.com ModsSearchSortField: 2 Popularity, 3 LastUpdated, 4 Name, 6 TotalDownloads.
        let (field, order) = match filter.sort {
            Sort::Popularity => ("2", "desc"),
            Sort::LastUpdated => ("3", "desc"),
            Sort::Name => ("4", "asc"),
            Sort::TotalDownloads => ("6", "desc"),
        };
        let category = filter.category.map(|c| c.to_string());
        let mut query = vec![
            ("gameId", game.as_str()),
            ("gameVersionTypeId", ty.as_str()),
            ("sortField", field),
            ("sortOrder", order),
            ("pageSize", "50"),
        ];
        let text = filter.text.trim();
        if !text.is_empty() {
            query.push(("searchFilter", text));
        }
        if let Some(c) = &category {
            query.push(("categoryId", c));
        }
        let mods: Vec<CfMod> = self.get("/v1/mods/search", &query)?;
        Ok(mods
            .iter()
            .map(to_addon)
            .filter(|a| !a.files.is_empty())
            .collect())
    }

    /// WoW addon categories (`GET /v1/categories?gameId=1`), sorted by name; top-level ones have
    /// no parent.
    pub fn categories(&self) -> Result<Vec<Category>> {
        let game = GAME_ID.to_string();
        let all: Vec<CfCategory> = self.get("/v1/categories", &[("gameId", &game)])?;
        let mut out: Vec<Category> = all
            .into_iter()
            .filter(|c| !c.is_class.unwrap_or(false) && c.class_id == Some(ADDONS_CLASS_ID))
            .map(|c| Category {
                id: c.id,
                name: c.name,
                slug: c.slug,
                parent: c.parent_category_id.filter(|p| *p != ADDONS_CLASS_ID),
            })
            .collect();
        out.sort_by_key(|c| c.name.to_lowercase());
        Ok(out)
    }

    /// The addon's long description (`GET /v1/mods/{id}/description`, HTML) as plain text.
    pub fn description(&self, id: &str) -> Result<String> {
        let id: u64 = id.parse().context("not a CurseForge addon id")?;
        let html: String = self.get(&format!("/v1/mods/{id}/description"), &[])?;
        Ok(html::to_text(&html, html::DESCRIPTION_MAX))
    }

    fn mods(&self, ids: &[u64]) -> Result<Vec<CfMod>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        self.post("/v1/mods", json!({ "modIds": ids }))
    }
}

impl Provider for CurseForge {
    fn name(&self) -> &'static str {
        NAME
    }

    fn available(&self) -> bool {
        self.key().is_ok()
    }

    fn search(&self, _flavor: Flavor, query: &str) -> Result<Vec<RemoteAddon>> {
        self.search_filtered(&SearchFilter {
            text: query.into(),
            ..SearchFilter::default()
        })
    }

    fn featured(&self, _flavor: Flavor) -> Result<Vec<RemoteAddon>> {
        let f: Featured = self.post(
            "/v1/mods/featured",
            json!({ "gameId": GAME_ID, "gameVersionTypeId": FOREVER_TYPE_ID, "excludedModIds": [] }),
        )?;
        let mut seen = HashSet::new();
        Ok(f.popular
            .iter()
            .chain(&f.recently_updated)
            .filter(|m| seen.insert(m.id))
            .map(to_addon)
            .filter(|a| !a.files.is_empty())
            .collect())
    }

    fn get_many(&self, _flavor: Flavor, external_ids: &[String]) -> Result<Vec<RemoteAddon>> {
        let ids: Vec<u64> = external_ids
            .iter()
            .filter_map(|id| id.parse().ok())
            .collect();
        Ok(self.mods(&ids)?.iter().map(to_addon).collect())
    }

    fn match_folders(
        &self,
        _flavor: Flavor,
        folders: &[ScannedFolder],
    ) -> Result<Vec<FolderMatch>> {
        if folders.is_empty() {
            return Ok(vec![]);
        }
        let fps: Vec<u32> = folders.iter().map(|f| f.fingerprint).collect();
        let res: FingerprintResult = self.post(
            &format!("/v1/fingerprints/{GAME_ID}"),
            json!({ "fingerprints": fps }),
        )?;
        let has_module =
            |m: &&FpMatch, fp: u32| m.file.modules.iter().any(|md| md.fingerprint == fp);
        let matched: Vec<(&ScannedFolder, &FpMatch)> = folders
            .iter()
            .filter_map(|folder| {
                let fp = folder.fingerprint;
                // Exact matches must be a Forever file; partial ones (locally modified) are taken as-is.
                res.exact_matches
                    .iter()
                    .find(|m| is_forever(&m.file) && has_module(m, fp))
                    .or_else(|| res.partial_matches.iter().find(|m| has_module(m, fp)))
                    .map(|m| (folder, m))
            })
            .collect();
        let mut ids: Vec<u64> = matched.iter().map(|(_, m)| m.id).collect();
        ids.sort_unstable();
        ids.dedup();
        let addons: Vec<RemoteAddon> = self.mods(&ids)?.iter().map(to_addon).collect();
        Ok(matched
            .into_iter()
            .filter_map(|(folder, m)| {
                let addon = addons.iter().find(|a| a.external_id == m.id.to_string())?;
                Some(FolderMatch {
                    folder: folder.name.clone(),
                    addon: addon.clone(),
                    installed_file: Some(to_file(&m.file)),
                })
            })
            .collect())
    }

    fn from_url(&self, _flavor: Flavor, url: &str) -> Result<Option<RemoteAddon>> {
        let Some(slug) = parse_url(url) else {
            return Ok(None);
        };
        // No gameVersionTypeId filter: we want to say "no Forever release" rather than "not found".
        let game = GAME_ID.to_string();
        let mods: Vec<CfMod> = self.get("/v1/mods/search", &[("gameId", &game), ("slug", slug)])?;
        let Some(m) = mods.iter().find(|m| m.slug.eq_ignore_ascii_case(slug)) else {
            bail!("CurseForge addon '{slug}' not found");
        };
        let addon = to_addon(m);
        if addon.files.is_empty() {
            bail!("{} has no World of Warcraft: Forever release", addon.name);
        }
        Ok(Some(addon))
    }
}

/// `https://www.curseforge.com/wow/addons/<slug>[/...]` → slug.
pub(super) fn parse_url(url: &str) -> Option<&str> {
    let rest = url.trim().split_once("://").map_or(url.trim(), |(_, r)| r);
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    let slug = rest
        .strip_prefix("curseforge.com/wow/addons/")?
        .split(['/', '?', '#'])
        .next()?;
    (!slug.is_empty()).then_some(slug)
}

fn is_forever(f: &CfFile) -> bool {
    f.sortable_game_versions
        .iter()
        .any(|v| v.game_version_type_id == FOREVER_TYPE_ID)
}

fn to_addon(m: &CfMod) -> RemoteAddon {
    let mut files: Vec<&CfFile> = m
        .latest_files
        .iter()
        .filter(|f| !f.expose_as_alternative.unwrap_or(false) && is_forever(f))
        .collect();
    files.sort_by_key(|f| std::cmp::Reverse(f.id));
    RemoteAddon {
        provider: NAME.into(),
        external_id: m.id.to_string(),
        name: m.name.clone(),
        author: m
            .authors
            .iter()
            .map(|a| a.name.as_str())
            .filter(|n| !n.to_lowercase().starts_with("_forgeuser"))
            .collect::<Vec<_>>()
            .join(", "),
        summary: m.summary.clone(),
        url: m.links.website_url.clone(),
        thumbnail_url: m.logo.as_ref().and_then(|l| l.thumbnail_url.clone()),
        download_count: m.download_count as u64,
        files: newest_per_channel(files.into_iter().map(to_file)),
        categories: m.categories.iter().map(|c| c.name.clone()).collect(),
        updated_at: m.date_modified.clone(),
    }
}

fn to_file(f: &CfFile) -> RemoteFile {
    RemoteFile {
        id: f.id.to_string(),
        // Some authors upload with the zip name as the display name ("ForeverQuestTint-0.7.3.zip").
        version: f
            .display_name
            .strip_suffix(".zip")
            .unwrap_or(&f.display_name)
            .to_string(),
        channel: match f.release_type {
            2 => Channel::Beta,
            3 => Channel::Alpha,
            _ => Channel::Stable,
        },
        // shortcut: null downloadUrl (distribution disabled) → the public CDN path pattern, which
        // works today; switch to GET /v1/mods/{id}/files/{fileId}/download-url if it stops working.
        download_url: f.download_url.clone().unwrap_or_else(|| {
            format!(
                "https://edge.forgecdn.net/files/{}/{}/{}",
                f.id / 1000,
                f.id % 1000,
                f.file_name.replace(' ', "%20")
            )
        }),
        released_at: f.file_date.clone(),
        game_versions: f
            .sortable_game_versions
            .iter()
            .filter(|v| v.game_version_type_id == FOREVER_TYPE_ID)
            .map(|v| {
                if v.game_version.is_empty() {
                    v.game_version_name.clone()
                } else {
                    v.game_version.clone()
                }
            })
            .collect(),
        folders: f.modules.iter().map(|m| m.name.clone()).collect(),
        dependencies: f
            .dependencies
            .iter()
            .map(|d| Dependency {
                external_id: d.mod_id.to_string(),
                kind: match d.relation_type {
                    1 => DependencyKind::Embedded,
                    2 => DependencyKind::Optional,
                    3 => DependencyKind::Required,
                    _ => DependencyKind::Other, // 4 Tool, 5 Incompatible, 6 Include
                },
            })
            .collect(),
    }
}

// --- API shapes (only the fields we use; everything defaults so schema drift doesn't break us) ---

#[derive(Deserialize)]
struct Envelope<T> {
    data: T,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Featured {
    popular: Vec<CfMod>,
    recently_updated: Vec<CfMod>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct FingerprintResult {
    exact_matches: Vec<FpMatch>,
    partial_matches: Vec<FpMatch>,
}

#[derive(Deserialize)]
struct FpMatch {
    id: u64,
    file: CfFile,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct CfMod {
    id: u64,
    name: String,
    slug: String,
    summary: String,
    download_count: f64,
    links: Links,
    authors: Vec<Author>,
    logo: Option<Logo>,
    latest_files: Vec<CfFile>,
    categories: Vec<CfCategory>,
    date_modified: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct CfCategory {
    id: u32,
    name: String,
    slug: String,
    class_id: Option<u32>,
    parent_category_id: Option<u32>,
    is_class: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Links {
    website_url: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Author {
    name: String,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Logo {
    thumbnail_url: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct CfFile {
    id: u64,
    display_name: String,
    file_name: String,
    release_type: u8,
    file_date: Option<String>,
    download_url: Option<String>,
    expose_as_alternative: Option<bool>,
    sortable_game_versions: Vec<SortableVersion>,
    dependencies: Vec<CfDependency>,
    modules: Vec<Module>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct SortableVersion {
    game_version_name: String,
    game_version: String,
    game_version_type_id: u32,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct CfDependency {
    mod_id: u64,
    relation_type: u8,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Module {
    name: String,
    fingerprint: u32,
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_addon_urls() {
        assert_eq!(
            super::parse_url("https://www.curseforge.com/wow/addons/details"),
            Some("details")
        );
        assert_eq!(
            super::parse_url("curseforge.com/wow/addons/dbm-core/files?x=1"),
            Some("dbm-core")
        );
        assert_eq!(super::parse_url("https://github.com/a/b"), None);
        assert_eq!(
            super::parse_url("https://www.curseforge.com/wow/addons/"),
            None
        );
    }
}
