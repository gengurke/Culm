//! Scanning `Interface/AddOns` (F2.1), provider matching (F2.3) and grouping folders into addons (F2.2).

use crate::fingerprint::folder_fingerprints;
use crate::model::{Addon, Channel, Flavor, IgnoreReason, Installation};
use crate::providers::{FolderMatch, Provider, ScannedFolder};
use crate::toc::{Toc, read_folder_toc, toc_names};
use crate::util::new_id;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// One addon folder as found on disk.
#[derive(Debug, Clone)]
pub struct LocalFolder {
    pub name: String,
    /// The Forever TOC, None if the folder only has TOCs for other flavors.
    pub toc: Option<Toc>,
    pub fingerprint: u32,
    /// md5 fingerprint (WowUp Hub).
    pub hub_fingerprint: String,
    /// The folder is a git checkout (developer addon).
    pub is_git: bool,
}

impl LocalFolder {
    pub fn scanned(&self) -> ScannedFolder {
        ScannedFolder {
            name: self.name.clone(),
            fingerprint: self.fingerprint,
            hub_fingerprint: self.hub_fingerprint.clone(),
            toc_ids: self.toc.as_ref().map(Toc::provider_ids).unwrap_or_default(),
        }
    }
}

/// Result of matching: the addon list plus non-fatal provider errors (e.g. offline).
#[derive(Debug, Default)]
pub struct ScanResult {
    pub addons: Vec<Addon>,
    pub errors: Vec<String>,
}

/// Reads every addon folder (a folder with at least one `.toc`) in the installation's AddOns dir.
/// Folders are fingerprinted in parallel. Sorted by name.
pub fn scan_folders(inst: &Installation) -> Result<Vec<LocalFolder>> {
    let dir = inst.addons_dir();
    let rd = std::fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))?;
    let mut paths: Vec<PathBuf> = rd
        .flatten()
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    paths.sort();
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(8);
    let chunk = paths.len().div_ceil(threads).max(1);
    let results: Vec<Result<Option<LocalFolder>>> = std::thread::scope(|s| {
        let handles: Vec<_> = paths
            .chunks(chunk)
            .map(|c| s.spawn(move || c.iter().map(|p| scan_folder(p)).collect::<Vec<_>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| {
                h.join()
                    .unwrap_or_else(|_| vec![Err(anyhow::anyhow!("scan thread panicked"))])
            })
            .collect()
    });
    let mut out = Vec::new();
    for r in results {
        if let Some(f) = r? {
            out.push(f);
        }
    }
    Ok(out)
}

fn scan_folder(path: &Path) -> Result<Option<LocalFolder>> {
    if toc_names(path)?.is_empty() {
        return Ok(None);
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (fingerprint, hub_fingerprint) =
        folder_fingerprints(path).with_context(|| format!("fingerprinting {}", path.display()))?;
    Ok(Some(LocalFolder {
        toc: read_folder_toc(path)?,
        fingerprint,
        hub_fingerprint,
        is_git: path.join(".git").exists(),
        name,
    }))
}

/// Scan + match in one go.
pub fn scan_installation(
    inst: &Installation,
    providers: &[Box<dyn Provider>],
) -> Result<ScanResult> {
    Ok(build_addons(inst.flavor, &scan_folders(inst)?, providers))
}

/// Matches folders against the available providers (in order; a folder matched by one provider is
/// not offered to the next) and groups them into addons:
/// - folders matched to the same provider addon form one addon, which also adopts unmatched local
///   folders listed in the installed file's folder list;
/// - unmatched folders join an unmatched folder they require whose name is their prefix
///   (`Details_TinyThreat` -> `Details`), else stand alone. They are ignored as `Unknown`.
pub fn build_addons(
    flavor: Flavor,
    folders: &[LocalFolder],
    providers: &[Box<dyn Provider>],
) -> ScanResult {
    let mut result = ScanResult::default();
    let mut matched: HashMap<String, (&'static str, FolderMatch)> = HashMap::new();
    for p in providers.iter().filter(|p| p.available()) {
        let pending: Vec<ScannedFolder> = folders
            .iter()
            .filter(|f| !matched.contains_key(&f.name))
            .map(LocalFolder::scanned)
            .collect();
        if pending.is_empty() {
            break;
        }
        match p.match_folders(flavor, &pending) {
            Ok(ms) => {
                for m in ms {
                    if pending.iter().any(|f| f.name == m.folder)
                        && !matched.contains_key(&m.folder)
                    {
                        matched.insert(m.folder.clone(), (p.name(), m));
                    }
                }
            }
            Err(e) => result.errors.push(format!("{}: {e:#}", p.name())),
        }
    }

    let by_name: HashMap<&str, &LocalFolder> =
        folders.iter().map(|f| (f.name.as_str(), f)).collect();
    let mut claimed: Vec<String> = matched.keys().cloned().collect();

    // Matched groups, keyed by provider + external id.
    let mut groups: BTreeMap<(&str, String), Vec<&FolderMatch>> = BTreeMap::new();
    for (provider, m) in matched.values() {
        groups
            .entry((provider, m.addon.external_id.clone()))
            .or_default()
            .push(m);
    }
    for ((provider, _), ms) in groups {
        let mut names: Vec<String> = ms.iter().map(|m| m.folder.clone()).collect();
        let best = ms
            .iter()
            .find(|m| m.installed_file.is_some())
            .unwrap_or(&ms[0]);
        if let Some(file) = &best.installed_file {
            for f in folders {
                let listed = file.folders.iter().any(|n| n.eq_ignore_ascii_case(&f.name));
                if listed && !claimed.contains(&f.name) {
                    claimed.push(f.name.clone());
                    names.push(f.name.clone());
                }
            }
        }
        names.sort();
        let locals: Vec<&LocalFolder> = names
            .iter()
            .filter_map(|n| by_name.get(n.as_str()).copied())
            .collect();
        let main = main_folder(&locals);
        let file = best.installed_file.as_ref();
        let channel = file.map_or(Channel::Stable, |f| f.channel);
        let a = &best.addon;
        result.addons.push(Addon {
            id: new_id(),
            name: a.name.clone(),
            author: a.author.clone(),
            summary: a.summary.clone(),
            provider: Some(provider.to_string()),
            external_id: Some(a.external_id.clone()),
            url: a.url.clone(),
            thumbnail_url: a.thumbnail_url.clone(),
            installed_version: file
                .map(|f| f.version.clone())
                .or_else(|| main.and_then(|m| m.toc.as_ref()?.version.clone())),
            installed_file_id: file.map(|f| f.id.clone()),
            installed_at: None,
            game_version: main.and_then(|m| m.toc.as_ref()?.forever_game_version()),
            game_versions: main
                .and_then(|m| Some(m.toc.as_ref()?.game_versions()))
                .unwrap_or_default(),
            channel,
            latest: a.latest_for(channel).cloned(),
            ignored: locals
                .iter()
                .any(|f| f.is_git)
                .then_some(IgnoreReason::GitRepo),
            auto_update: false,
            dependencies: file.map(|f| f.dependencies.clone()).unwrap_or_default(),
            folders: names,
        });
    }

    // Unmatched: group dependents under their prefix parent.
    let unmatched: Vec<&LocalFolder> = folders
        .iter()
        .filter(|f| !claimed.contains(&f.name))
        .collect();
    let lc: HashMap<String, &str> = unmatched
        .iter()
        .map(|f| (f.name.to_lowercase(), f.name.as_str()))
        .collect();
    let parent = |f: &LocalFolder| -> Option<&str> {
        let name = f.name.to_lowercase();
        f.toc.as_ref()?.dependencies.iter().find_map(|d| {
            let d = d.to_lowercase();
            (d != name && name.starts_with(&d))
                .then(|| lc.get(&d).copied())
                .flatten()
        })
    };
    let mut ugroups: BTreeMap<&str, Vec<&LocalFolder>> = BTreeMap::new();
    for f in &unmatched {
        let mut root = f.name.as_str();
        // Prefix parents are strictly shorter, so this terminates.
        while let Some(p) = by_name.get(root).and_then(|r| parent(r)) {
            root = p;
        }
        ugroups.entry(root).or_default().push(f);
    }
    for (_, mut locals) in ugroups {
        locals.sort_by(|a, b| a.name.cmp(&b.name));
        let toc = main_folder(&locals).and_then(|m| m.toc.as_ref());
        let fallback = main_folder(&locals)
            .map(|m| m.name.clone())
            .unwrap_or_default();
        result.addons.push(Addon {
            id: new_id(),
            name: toc
                .and_then(|t| t.title.clone())
                .filter(|t| t.chars().any(char::is_alphabetic))
                .unwrap_or(fallback),
            author: toc.and_then(|t| t.author.clone()).unwrap_or_default(),
            summary: toc.and_then(|t| t.notes.clone()).unwrap_or_default(),
            provider: None,
            external_id: None,
            url: None,
            thumbnail_url: None,
            installed_version: toc.and_then(|t| t.version.clone()),
            installed_file_id: None,
            installed_at: None,
            game_version: toc.and_then(Toc::forever_game_version),
            game_versions: toc.map(Toc::game_versions).unwrap_or_default(),
            channel: Channel::Stable,
            latest: None,
            ignored: Some(if locals.iter().any(|f| f.is_git) {
                IgnoreReason::GitRepo
            } else {
                IgnoreReason::Unknown
            }),
            auto_update: false,
            dependencies: vec![],
            folders: locals.iter().map(|f| f.name.clone()).collect(),
        });
    }
    result.addons.sort_by_key(|a| a.name.to_lowercase());
    result
}

/// The folder that represents a group: the shortest name (the parent of `Foo_Options` etc.).
fn main_folder<'a>(locals: &[&'a LocalFolder]) -> Option<&'a LocalFolder> {
    locals
        .iter()
        .copied()
        .min_by(|a, b| a.name.len().cmp(&b.name.len()).then(a.name.cmp(&b.name)))
}
