//! GitHub releases (public API; optional token raises the 60 req/h limit).
//! An addon is a repo "owner/repo"; its Forever build is found per release via the BigWigs packager
//! `release.json`, else a `*-forever.zip`/`*-camelot.zip` asset, else a plain (untagged) zip.

use super::{Provider, newest_per_channel};
use crate::http;
use crate::model::{Channel, Flavor, RemoteAddon, RemoteFile};
use anyhow::{Result, bail};
use serde::Deserialize;

pub const NAME: &str = "GitHub";
pub const DEFAULT_API_BASE: &str = "https://api.github.com";
/// Releases considered per repo (newest first).
const RELEASES: usize = 5;

pub struct GitHub {
    pub token: Option<String>,
    pub api_base: String,
}

impl GitHub {
    fn get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let auth = self
            .token
            .as_deref()
            .filter(|t| !t.is_empty())
            .map(|t| format!("Bearer {t}"));
        let mut headers = vec![("Accept", "application/vnd.github+json")];
        if let Some(auth) = &auth {
            headers.push(("Authorization", auth));
        }
        http::get_json(url, &[], &headers)
    }

    fn fetch(&self, repo: &str) -> Result<RemoteAddon> {
        let info: Repo = self.get(&format!("{}/repos/{repo}", self.api_base))?;
        let mut releases: Vec<Release> = self.get(&format!(
            "{}/repos/{repo}/releases?per_page=20",
            self.api_base
        ))?;
        releases.retain(|r| !r.draft);
        releases.sort_by(|a, b| b.published_at.cmp(&a.published_at));
        releases.truncate(RELEASES);
        let files: Vec<RemoteFile> = releases
            .iter()
            .filter_map(|r| {
                let asset = forever_asset(r)?;
                Some(RemoteFile {
                    id: asset.id.to_string(),
                    version: r.tag_name.clone(),
                    channel: if r.prerelease {
                        Channel::Beta
                    } else {
                        Channel::Stable
                    },
                    download_url: asset.browser_download_url.clone(),
                    released_at: r.published_at.clone(),
                    game_versions: vec![],
                    folders: vec![],
                    dependencies: vec![],
                })
            })
            .collect();
        Ok(RemoteAddon {
            provider: NAME.into(),
            external_id: info.full_name.clone(),
            name: info.name,
            author: info.owner.login,
            summary: info.description.unwrap_or_default(),
            url: Some(info.html_url.clone()),
            source_url: Some(info.html_url),
            thumbnail_url: info.owner.avatar_url,
            download_count: releases
                .iter()
                .flat_map(|r| &r.assets)
                .map(|a| a.download_count)
                .sum(),
            updated_at: files.iter().filter_map(|f| f.released_at.clone()).max(),
            files: newest_per_channel(files.into_iter()),
            categories: vec![],
        })
    }
}

impl Provider for GitHub {
    fn name(&self) -> &'static str {
        NAME
    }

    fn search(&self, _flavor: Flavor, _query: &str) -> Result<Vec<RemoteAddon>> {
        Ok(vec![])
    }

    /// One request pair per repo: a failing repo (renamed, deleted, rate limit) is skipped so
    /// the others still get checked; only a batch where every repo fails is an error.
    fn get_many(&self, _flavor: Flavor, external_ids: &[String]) -> Result<Vec<RemoteAddon>> {
        let mut out = Vec::new();
        let mut errors = Vec::new();
        for id in external_ids {
            match self.fetch(id) {
                Ok(a) => out.push(a),
                Err(e) => errors.push(format!("{id}: {e:#}")),
            }
        }
        if out.is_empty() && !errors.is_empty() {
            bail!("{}", errors.join("; "));
        }
        for e in &errors {
            eprintln!("GitHub update check skipped {e}");
        }
        Ok(out)
    }

    fn from_url(&self, _flavor: Flavor, url: &str) -> Result<Option<RemoteAddon>> {
        let Some(repo) = parse_url(url) else {
            return Ok(None);
        };
        let addon = self.fetch(&repo)?;
        if addon.files.is_empty() {
            bail!("{repo} has no World of Warcraft: Forever release asset");
        }
        Ok(Some(addon))
    }
}

fn forever_asset(r: &Release) -> Option<&Asset> {
    let zips = || {
        r.assets
            .iter()
            .filter(|a| is_zip(a) && !a.name.to_lowercase().contains("-nolib"))
    };
    let by_name = || {
        zips().find(|a| {
            let n = a.name.to_lowercase();
            n.ends_with("-forever.zip")
                || n.ends_with("_forever.zip")
                || n.ends_with("-camelot.zip")
                || n.ends_with("_camelot.zip")
        })
    };
    if let Some(meta_asset) = r.assets.iter().find(|a| a.name == "release.json") {
        // A failed metadata fetch falls back to asset names. No token: the download
        // URL redirects to another host.
        let picked = http::get_json::<ReleaseJson>(&meta_asset.browser_download_url, &[], &[])
            .ok()
            .and_then(|meta| {
                let entry = meta
                    .releases
                    .iter()
                    .find(|e| !e.nolib && e.metadata.iter().any(|m| m.flavor == "forever"))?;
                r.assets.iter().find(|a| a.name == entry.filename)
            });
        // With metadata present, a release without a Forever entry has no Forever build.
        return picked.or_else(by_name);
    }
    // Convenience fallback for repos without packager metadata: first zip not tagged for another flavor.
    by_name().or_else(|| zips().find(|a| !tagged_other_flavor(&a.name)))
}

/// `https://github.com/<owner>/<repo>[/...][.git]` or bare `owner/repo` → "owner/repo".
pub(super) fn parse_url(url: &str) -> Option<String> {
    let url = url.trim();
    let (rest, bare) = match url.split_once("://") {
        Some((_, r)) => (
            r.strip_prefix("www.")
                .unwrap_or(r)
                .strip_prefix("github.com/")?,
            false,
        ),
        None => match url.strip_prefix("github.com/") {
            Some(r) => (r, false),
            None => (url, true),
        },
    };
    let mut parts = rest.split(['?', '#']).next()?.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    let repo = repo.strip_suffix(".git").unwrap_or(repo);
    let ok = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    // A bare "owner/repo" must be exactly that, so we don't claim other providers' input.
    if !ok(owner) || !ok(repo) || (bare && parts.next().is_some()) {
        return None;
    }
    Some(format!("{owner}/{repo}"))
}

fn is_zip(a: &Asset) -> bool {
    a.name.to_lowercase().ends_with(".zip")
        && matches!(
            a.content_type.as_str(),
            "application/zip"
                | "application/x-zip-compressed"
                | "application/x-zip"
                | "application/octet-stream"
                | ""
        )
}

/// Zip names tagged for a non-Forever client (BigWigs packager suffixes and common variants).
fn tagged_other_flavor(name: &str) -> bool {
    let stem = name.to_lowercase();
    let stem = stem.trim_end_matches(".zip");
    [
        "mainline", "retail", "classic", "vanilla", "bcc", "tbc", "wrath", "wotlk", "cata",
        "mists", "mop",
    ]
    .iter()
    .any(|f| stem.ends_with(&format!("-{f}")) || stem.ends_with(&format!("_{f}")))
}

// --- API shapes ---

#[derive(Deserialize)]
struct Repo {
    full_name: String,
    name: String,
    html_url: String,
    description: Option<String>,
    owner: Owner,
}

#[derive(Deserialize)]
struct Owner {
    login: String,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    id: u64,
    name: String,
    #[serde(default)]
    content_type: String,
    browser_download_url: String,
    #[serde(default)]
    download_count: u64,
}

#[derive(Deserialize)]
struct ReleaseJson {
    releases: Vec<ReleaseJsonEntry>,
}

#[derive(Deserialize)]
struct ReleaseJsonEntry {
    filename: String,
    #[serde(default)]
    nolib: bool,
    #[serde(default)]
    metadata: Vec<ReleaseJsonMeta>,
}

#[derive(Deserialize)]
struct ReleaseJsonMeta {
    flavor: String,
}

#[cfg(test)]
mod tests {
    use super::{parse_url, tagged_other_flavor};

    #[test]
    fn parses_repo_urls() {
        assert_eq!(
            parse_url("https://github.com/Questie/Questie").as_deref(),
            Some("Questie/Questie")
        );
        assert_eq!(
            parse_url("https://github.com/a/b.git").as_deref(),
            Some("a/b")
        );
        assert_eq!(parse_url("github.com/a/b/releases").as_deref(), Some("a/b"));
        assert_eq!(parse_url("a/b").as_deref(), Some("a/b"));
        assert_eq!(parse_url("a/b/c"), None);
        assert_eq!(parse_url("https://www.curseforge.com/wow/addons/x"), None);
        assert_eq!(parse_url("https://github.com/a"), None);
    }

    #[test]
    fn flavor_tags() {
        assert!(tagged_other_flavor("Addon-1.0-classic.zip"));
        assert!(!tagged_other_flavor("Addon-1.0.zip"));
    }
}
