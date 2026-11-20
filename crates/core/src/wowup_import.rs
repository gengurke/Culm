//! Import of WowUp's addon records for the same client (F2.4). Read-only on WowUp's files.

use crate::model::{
    Addon, Channel, Dependency, DependencyKind, IgnoreReason, Installation, RemoteFile,
};
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;

fn read_json(path: &Path) -> Result<Value> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))
}

fn same_dir(a: &Path, b: &Path) -> bool {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    canon(a) == canon(b)
}

/// Maps WowUp records of the WowUp installation(s) located in `inst.root` to addons.
/// Only providers we support are imported, and only records whose folders all exist.
pub fn import_wowup(wowup_dir: &Path, inst: &Installation) -> Result<Vec<Addon>> {
    let prefs = read_json(&wowup_dir.join("preferences.json"))?;
    // electron-store may keep nested values as JSON strings.
    let installs = match &prefs["wow_installations"] {
        Value::String(s) => serde_json::from_str(s).context("parsing wow_installations")?,
        v => v.clone(),
    };
    let ids: Vec<&str> = installs
        .as_array()
        .into_iter()
        .flatten()
        .filter(|i| {
            i["location"]
                .as_str()
                .and_then(|l| Path::new(l).parent())
                .is_some_and(|dir| same_dir(dir, &inst.root))
        })
        .filter_map(|i| i["id"].as_str())
        .collect();
    let records = read_json(&wowup_dir.join("addons.json"))?;
    let addons_dir = inst.addons_dir();
    let mut out: Vec<Addon> = records
        .as_object()
        .into_iter()
        .flat_map(|m| m.values())
        .filter(|r| {
            r["installationId"]
                .as_str()
                .is_some_and(|id| ids.contains(&id))
        })
        .filter_map(map_record)
        .filter(|a| !a.folders.is_empty() && a.folders.iter().all(|f| addons_dir.join(f).is_dir()))
        .collect();
    for a in &mut out {
        clean_file_name_version(a, &addons_dir);
    }
    crate::store::resolve_overlaps(&mut out);
    Ok(out)
}

/// WowUp sometimes stores the zip name as the version ("ForeverQuestTint-0.7.3.zip"): show the
/// main folder's TOC `## Version` instead (else the name without `.zip`).
fn clean_file_name_version(a: &mut Addon, addons_dir: &Path) {
    let is_file = |v: &str| v.to_ascii_lowercase().ends_with(".zip");
    let strip = |v: &str| v[..v.len() - 4].to_string();
    if let Some(v) = a.installed_version.clone().filter(|v| is_file(v)) {
        let main = a
            .folders
            .iter()
            .min_by(|x, y| x.len().cmp(&y.len()).then(x.cmp(y)));
        let toc = main
            .and_then(|m| {
                crate::toc::read_folder_toc(&addons_dir.join(m))
                    .ok()
                    .flatten()
            })
            .and_then(|t| t.version);
        a.installed_version = Some(toc.unwrap_or_else(|| strip(&v)));
    }
    if let Some(latest) = a.latest.as_mut().filter(|l| is_file(&l.version)) {
        latest.version = strip(&latest.version);
    }
}

fn map_record(r: &Value) -> Option<Addon> {
    let s = |k: &str| r[k].as_str().filter(|v| !v.is_empty()).map(str::to_string);
    let provider = match r["providerName"].as_str()? {
        "Curse" | "CurseV2" => "CurseForge",
        "WowInterface" => "WowInterface",
        "GitHub" => "GitHub",
        _ => return None,
    };
    let mut folders: Vec<String> = match r["installedFolderList"].as_array() {
        Some(list) => list
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        None => s("installedFolders")?
            .split(',')
            .map(|f| f.trim().to_string())
            .collect(),
    };
    folders.retain(|f| !f.is_empty() && !f.contains(['/', '\\']) && f != "..");
    let channel = |v: &Value| match v.as_u64() {
        Some(1) => Channel::Beta,
        Some(2) => Channel::Alpha,
        _ => Channel::Stable,
    };
    let channel_name = |v: &Value| match v.as_str() {
        Some("Beta") => Channel::Beta,
        Some("Alpha") => Channel::Alpha,
        _ => Channel::Stable,
    };
    let game_versions: Vec<String> = r["gameVersion"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    // Only a newer release WowUp knew of is kept (it can be installed without a key); "latest is
    // what's installed" was WowUp's check, not ours, so the addon shows "Not checked" until we check.
    let latest = match (s("externalLatestReleaseId"), s("latestVersion")) {
        (Some(id), _) if Some(&id) == s("installedExternalReleaseId").as_ref() => None,
        (Some(id), Some(version)) => Some(RemoteFile {
            id,
            version,
            channel: channel_name(&r["externalChannel"]),
            download_url: s("downloadUrl").unwrap_or_default(),
            released_at: s("releasedAt"),
            game_versions: vec![],
            folders: vec![],
            dependencies: vec![],
            check: None,
        }),
        _ => None,
    };
    let ignored = match (r["isIgnored"].as_bool(), r["ignoreReason"].as_str()) {
        (Some(true), Some("git_repo")) => Some(IgnoreReason::GitRepo),
        (Some(true), _) => Some(IgnoreReason::User),
        _ => None,
    };
    Some(Addon {
        id: s("id").unwrap_or_else(crate::util::new_id),
        name: s("name")?,
        author: s("author").unwrap_or_default(),
        summary: s("summary").unwrap_or_default(),
        provider: Some(provider.to_string()),
        external_id: Some(s("externalId")?),
        url: s("externalUrl"),
        thumbnail_url: s("thumbnailUrl"),
        folders,
        installed_version: s("installedVersion"),
        installed_file_id: s("installedExternalReleaseId"),
        installed_at: s("installedAt"),
        game_version: game_versions
            .iter()
            .find(|v| v.starts_with("1.6"))
            .or(game_versions.first())
            .cloned(),
        game_versions,
        channel: channel(&r["channelType"]),
        latest,
        ignored,
        auto_update: r["autoUpdateEnabled"].as_bool().unwrap_or(false),
        dependencies: r["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|d| {
                Some(Dependency {
                    external_id: d["externalAddonId"].as_str()?.to_string(),
                    kind: match d["type"].as_u64() {
                        Some(1) => DependencyKind::Embedded,
                        Some(2) => DependencyKind::Required,
                        Some(3) => DependencyKind::Optional,
                        _ => DependencyKind::Other,
                    },
                })
            })
            .collect(),
    })
}
