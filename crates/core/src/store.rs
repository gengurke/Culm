//! Persistence of installations and per-installation addon lists as JSON (F1.3, F2.5).
//! Layout: `<data>/installations.json`, `<data>/addons/<installation_id>.json`.

use crate::model::{Addon, IgnoreReason, Installation};
use crate::util::write_atomic;
use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};

fn addons_path(data_dir: &Path, installation_id: &str) -> PathBuf {
    data_dir
        .join("addons")
        .join(format!("{installation_id}.json"))
}

fn load<T: DeserializeOwned + Default>(path: &Path) -> Result<T> {
    match std::fs::read(path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

fn save<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    write_atomic(path, &serde_json::to_vec_pretty(value)?)
}

/// Stored addons of an installation (empty if never saved).
pub fn load_addons(data_dir: &Path, installation_id: &str) -> Result<Vec<Addon>> {
    load(&addons_path(data_dir, installation_id))
}

pub fn save_addons(data_dir: &Path, installation_id: &str, addons: &[Addon]) -> Result<()> {
    save(&addons_path(data_dir, installation_id), addons)
}

pub fn load_installations(data_dir: &Path) -> Result<Vec<Installation>> {
    load(&data_dir.join("installations.json"))
}

pub fn save_installations(data_dir: &Path, installations: &[Installation]) -> Result<()> {
    save(&data_dir.join("installations.json"), installations)
}

/// Combines a fresh scan with the previous records:
/// - a fresh provider match takes over the previous record of the same addon (id, channel,
///   auto-update, user ignore, install date; installed file if the provider could not tell);
/// - previous provider-linked records whose folders all still exist are kept (so links survive an
///   offline scan, a missing API key or an import); see below for shared folders;
/// - fresh unmatched records lose folders claimed above and keep the id / user ignore of a previous
///   record with the same folders.
pub fn merge_rescan(previous: &[Addon], fresh: Vec<Addon>) -> Vec<Addon> {
    let all_folders: Vec<String> = fresh.iter().flat_map(|a| a.folders.clone()).collect();
    let (matched, unmatched): (Vec<Addon>, Vec<Addon>) =
        fresh.into_iter().partition(|a| a.provider.is_some());
    let mut out: Vec<Addon> = Vec::new();

    for mut a in matched {
        if let Some(p) = previous
            .iter()
            .find(|p| p.provider == a.provider && p.external_id == a.external_id)
        {
            carry_user_choices(p, &mut a);
            a.installed_at = p.installed_at.clone();
            if a.installed_file_id.is_none() {
                a.installed_file_id = p.installed_file_id.clone();
                a.installed_version = p.installed_version.clone().or(a.installed_version);
            }
            if a.latest.is_none() {
                a.latest = p.latest.clone();
            }
        }
        out.push(a);
    }

    // Previous links whose folders all still exist are kept unless the same addon matched again.
    // On a shared folder, a fresh match of the *same* provider wins it (fresh data), but a link
    // to another provider stays with the previous record: without a CurseForge key, an imported
    // CurseForge record must not be taken over by a TOC `X-WoWI-ID` match, whose WowInterface
    // file may be a build for another game version. Overlaps among old records are resolved at the end.
    let fresh_matches = out.len();
    for p in previous.iter().filter(|p| p.provider.is_some()) {
        let same_addon = out[..fresh_matches]
            .iter()
            .any(|a| a.provider == p.provider && a.external_id == p.external_id);
        let present = !p.folders.is_empty() && p.folders.iter().all(|f| all_folders.contains(f));
        if same_addon || !present {
            continue;
        }
        let mut kept = p.clone();
        for a in &mut out[..fresh_matches] {
            if a.provider == kept.provider {
                kept.folders.retain(|f| !a.folders.contains(f));
            } else {
                a.folders.retain(|f| !kept.folders.contains(f));
            }
        }
        if !kept.folders.is_empty() {
            out.push(kept);
        }
    }
    out.retain(|a| !a.folders.is_empty());

    let claimed: Vec<String> = out.iter().flat_map(|a| a.folders.clone()).collect();
    for mut a in unmatched {
        a.folders.retain(|f| !claimed.contains(f));
        if a.folders.is_empty() {
            continue;
        }
        if let Some(p) = previous
            .iter()
            .find(|p| p.provider.is_none() && p.folders == a.folders)
        {
            carry_user_choices(p, &mut a);
        }
        out.push(a);
    }
    resolve_overlaps(&mut out);
    out
}

/// Makes every folder belong to exactly one addon. A folder claimed by several records stays
/// with the record that lists the fewest folders (its most specific owner: an import may list
/// `QuestieTrace` under Questie *and* as its own addon) and is dropped from the bigger ones;
/// ties keep the first record. Records left without folders are dropped. Sorted by name.
pub fn resolve_overlaps(addons: &mut Vec<Addon>) {
    addons.sort_by_key(|a| a.name.to_lowercase());
    let owners: Vec<Vec<String>> = addons.iter().map(|a| a.folders.clone()).collect();
    for (i, a) in addons.iter_mut().enumerate() {
        a.folders.retain(|f| {
            !owners.iter().enumerate().any(|(j, other)| {
                j != i
                    && other.contains(f)
                    && (other.len() < owners[i].len() || (other.len() == owners[i].len() && j < i))
            })
        });
    }
    addons.retain(|a| !a.folders.is_empty());
}

fn carry_user_choices(prev: &Addon, a: &mut Addon) {
    a.id = prev.id.clone();
    a.channel = prev.channel;
    a.auto_update = prev.auto_update;
    if prev.ignored == Some(IgnoreReason::User) {
        a.ignored = Some(IgnoreReason::User);
    }
}

/// Inserts or replaces `addon` (by id). It now owns its folders: other records lose them, and
/// records left without folders are dropped (e.g. the `Unknown` record of a folder that was just
/// installed). A record that only shared a library folder keeps its own folders and settings.
pub fn upsert_addon(addons: &mut Vec<Addon>, addon: Addon) {
    addons.retain(|a| a.id != addon.id);
    for a in addons.iter_mut() {
        a.folders.retain(|f| !addon.folders.contains(f));
    }
    addons.retain(|a| !a.folders.is_empty());
    addons.push(addon);
    addons.sort_by_key(|a| a.name.to_lowercase());
}

/// The folders `addon` may delete or replace: its stored folders (by id, else its own list)
/// minus any folder another stored record also lists.
pub fn owned_folders(addons: &[Addon], addon: &Addon) -> Vec<String> {
    let mine = addons.iter().find(|a| a.id == addon.id).unwrap_or(addon);
    mine.folders
        .iter()
        .filter(|f| !addons.iter().any(|o| o.id != addon.id && o.folders.contains(f)))
        .cloned()
        .collect()
}
