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
/// - previous provider-linked records whose folders all still exist and are not claimed by a fresh
///   match are kept (so links survive an offline scan, a missing API key or an import);
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

    // Only fresh matches displace old links; overlaps among old records (imports have some) stay.
    let kept: Vec<Addon> = previous
        .iter()
        .filter(|p| p.provider.is_some())
        .filter(|p| {
            let taken = out.iter().any(|a| {
                (a.provider == p.provider && a.external_id == p.external_id)
                    || a.folders.iter().any(|f| p.folders.contains(f))
            });
            let present =
                !p.folders.is_empty() && p.folders.iter().all(|f| all_folders.contains(f));
            !taken && present
        })
        .cloned()
        .collect();
    out.extend(kept);

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
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

fn carry_user_choices(prev: &Addon, a: &mut Addon) {
    a.id = prev.id.clone();
    a.channel = prev.channel;
    a.auto_update = prev.auto_update;
    if prev.ignored == Some(IgnoreReason::User) {
        a.ignored = Some(IgnoreReason::User);
    }
}

/// Inserts or replaces `addon` (by id) and drops other records that share one of its folders
/// (e.g. the `Unknown` record of a folder that was just installed from a provider).
pub fn upsert_addon(addons: &mut Vec<Addon>, addon: Addon) {
    addons.retain(|a| a.id != addon.id && !a.folders.iter().any(|f| addon.folders.contains(f)));
    addons.push(addon);
    addons.sort_by_key(|a| a.name.to_lowercase());
}
