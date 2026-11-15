//! WowInterface (public mmoui API, no key, no search, no flavor info: files are trusted as-is).

use super::{FolderMatch, Provider, ScannedFolder};
use crate::http;
use crate::model::{Channel, Flavor, RemoteAddon, RemoteFile};
use anyhow::{Result, bail};
use serde::Deserialize;

pub const NAME: &str = "WowInterface";
pub const DEFAULT_BASE_URL: &str = "https://api.mmoui.com/v4/game/WOW";

pub struct WowInterface {
    pub base_url: String,
}

impl Provider for WowInterface {
    fn name(&self) -> &'static str {
        NAME
    }

    fn search(&self, _flavor: Flavor, _query: &str) -> Result<Vec<RemoteAddon>> {
        Ok(vec![])
    }

    fn get_many(&self, _flavor: Flavor, external_ids: &[String]) -> Result<Vec<RemoteAddon>> {
        // Ids end up in the URL path: only digits are valid.
        let ids: Vec<&str> = external_ids
            .iter()
            .map(String::as_str)
            .filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
            .collect();
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let url = format!("{}/filedetails/{}.json", self.base_url, ids.join(","));
        let details: Vec<Details> = http::get_json(&url, &[], &[])?;
        Ok(details.into_iter().map(to_addon).collect())
    }

    fn match_folders(&self, flavor: Flavor, folders: &[ScannedFolder]) -> Result<Vec<FolderMatch>> {
        let by_folder: Vec<(&str, &str)> = folders
            .iter()
            .filter_map(|f| {
                let (_, id) = f.toc_ids.iter().find(|(p, _)| p == NAME)?;
                Some((f.name.as_str(), id.as_str()))
            })
            .collect();
        let ids: Vec<String> = by_folder.iter().map(|(_, id)| id.to_string()).collect();
        let addons = self.get_many(flavor, &ids)?;
        Ok(by_folder
            .into_iter()
            .filter_map(|(folder, id)| {
                let addon = addons.iter().find(|a| a.external_id == id)?.clone();
                Some(FolderMatch {
                    folder: folder.into(),
                    addon,
                    installed_file: None,
                })
            })
            .collect())
    }

    fn from_url(&self, flavor: Flavor, url: &str) -> Result<Option<RemoteAddon>> {
        let Some(id) = parse_url(url) else {
            return Ok(None);
        };
        match self
            .get_many(flavor, std::slice::from_ref(&id))?
            .into_iter()
            .next()
        {
            Some(a) => Ok(Some(a)),
            None => bail!("WowInterface addon {id} not found"),
        }
    }
}

/// `https://www.wowinterface.com/downloads/info<ID>-Name.html` (or `download<ID>-...`) → ID.
fn parse_url(url: &str) -> Option<String> {
    let rest = url.split_once("wowinterface.com/downloads/")?.1;
    let rest = rest
        .strip_prefix("info")
        .or_else(|| rest.strip_prefix("download"))?;
    let id: String = rest.chars().take_while(char::is_ascii_digit).collect();
    (!id.is_empty()).then_some(id)
}

fn to_addon(d: Details) -> RemoteAddon {
    RemoteAddon {
        provider: NAME.into(),
        external_id: d.id.to_string(),
        name: d.title,
        author: d.author,
        summary: String::new(), // description is long BBCode
        url: Some(format!(
            "https://www.wowinterface.com/downloads/info{}",
            d.id
        )),
        thumbnail_url: d.images.first().and_then(|i| i.thumb_url.clone()),
        download_count: d.downloads,
        files: vec![RemoteFile {
            id: format!("{}-{}", d.id, d.last_update),
            version: d.version,
            channel: Channel::Stable,
            download_url: d.download_uri,
            released_at: None, // shortcut: lastUpdate is ms epoch; convert once the UI shows dates
            game_versions: vec![],
            folders: vec![],
            dependencies: vec![],
        }],
        source_url: None,
        categories: vec![],
        updated_at: None,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Details {
    id: u64,
    title: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    last_update: u64,
    download_uri: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    images: Vec<Image>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Image {
    thumb_url: Option<String>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_urls() {
        let p = super::parse_url;
        assert_eq!(
            p("https://www.wowinterface.com/downloads/info5108-Clique.html").as_deref(),
            Some("5108")
        );
        assert_eq!(
            p("wowinterface.com/downloads/download5108-Clique").as_deref(),
            Some("5108")
        );
        assert_eq!(p("https://github.com/a/b"), None);
    }
}
