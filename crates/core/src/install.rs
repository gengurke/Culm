//! Install / update / reinstall / remove (F3.2, F4.3, F5.1, F5.2).
//!
//! Folder swap: the zip is validated and extracted to a staging dir next to `AddOns` (same
//! filesystem, so moves are renames), the addon's old folders and every folder about to be
//! replaced are moved to a backup dir, the new folders are moved in, and on any failure the new
//! folders are removed and the backups restored. Old folders absent from the new zip are gone.

use crate::model::{Addon, Channel, Installation, RemoteAddon, RemoteFile};
use crate::toc::read_folder_toc;
use crate::util::{new_id, now_rfc3339, now_stamp};
use anyhow::{Context, Result, bail};
use std::collections::BTreeSet;
use std::io::{Cursor, Read, Seek};
use std::path::{Path, PathBuf};

/// A downloaded zip: a temp file from the app, or bytes in memory ([`in_memory`]).
pub trait Zip: Read + Seek {}
impl<T: Read + Seek> Zip for T {}

/// Fetches a URL (the app injects its HTTP client and streams to a temp file).
pub type Download<'a> = &'a dyn Fn(&str) -> Result<Box<dyn Zip>>;

/// Wraps bytes already in memory as a [`Zip`].
pub fn in_memory(bytes: Vec<u8>) -> Box<dyn Zip> {
    Box::new(Cursor::new(bytes))
}

/// Refuse archives that inflate beyond this (zip bombs).
const MAX_UNPACKED: u64 = 16 << 30;

/// Installs the zip's top-level folders into AddOns, replacing `old_folders` (the addon's current
/// folders) and any existing folder of the same name. Returns the new folder list.
pub fn install_zip(
    inst: &Installation,
    zip: impl Read + Seek,
    old_folders: &[String],
) -> Result<Vec<String>> {
    for f in old_folders {
        check_folder_name(f)?;
    }
    let addons = inst.addons_dir();
    std::fs::create_dir_all(&addons).with_context(|| format!("creating {}", addons.display()))?;
    let work = addons.parent().context("AddOns folder has no parent")?;
    let tag = format!("{}-{}", now_stamp(), &new_id()[..8]);
    let staging = work.join(format!(".addons-staging-{tag}"));
    let backup = work.join(format!(".addons-backup-{tag}"));

    let extracted = extract(zip, &staging);
    let result =
        extracted.and_then(|new| swap(&addons, &staging, &backup, &new, old_folders).map(|_| new));
    let _ = std::fs::remove_dir_all(&staging);
    if result.is_ok() {
        let _ = std::fs::remove_dir_all(&backup);
    }
    result
}

/// Validates every entry, then extracts. Returns the top-level folder names.
fn extract(zip: impl Read + Seek, dest: &Path) -> Result<Vec<String>> {
    let mut ar = zip::ZipArchive::new(zip).context("the download is not a valid zip file")?;
    let mut entries: Vec<(usize, Vec<String>, bool)> = Vec::new();
    let mut tops: BTreeSet<String> = BTreeSet::new();
    let mut with_toc: BTreeSet<String> = BTreeSet::new();
    for i in 0..ar.len() {
        let f = ar.by_index(i).context("reading zip entry")?;
        let raw = f.name().context("zip entry name")?.to_string();
        let parts = safe_parts(&raw)?;
        if parts.is_empty() || parts[0] == "__MACOSX" {
            continue;
        }
        if f.is_symlink() {
            bail!("zip entry {raw:?} is a symlink");
        }
        let is_dir = f.is_dir();
        if parts.len() == 1 && !is_dir {
            continue; // loose top-level files (README etc.) are not addons
        }
        if parts.len() == 2 && !is_dir && parts[1].to_ascii_lowercase().ends_with(".toc") {
            with_toc.insert(parts[0].clone());
        }
        tops.insert(parts[0].clone());
        entries.push((i, parts, is_dir));
    }
    // A zip with only a wrapper folder (`Repo-v1/MyAddon/...`) would replace the addon with a
    // folder WoW never loads: refuse it before anything is touched.
    if with_toc.is_empty() {
        bail!("the zip contains no addon folders (no top-level folder has a .toc file)");
    }
    std::fs::create_dir_all(dest).with_context(|| format!("creating {}", dest.display()))?;
    let mut budget = MAX_UNPACKED;
    for (i, parts, is_dir) in entries {
        let path = parts.iter().fold(dest.to_path_buf(), |p, s| p.join(s));
        if is_dir {
            std::fs::create_dir_all(&path)?;
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut f = ar.by_index(i)?;
        let mut out =
            std::fs::File::create(&path).with_context(|| format!("creating {}", path.display()))?;
        let n = std::io::copy(&mut (&mut f).take(budget + 1), &mut out)
            .with_context(|| format!("extracting {}", parts.join("/")))?;
        if n > budget {
            bail!("the zip unpacks to more than {} GiB", MAX_UNPACKED >> 30);
        }
        budget -= n;
    }
    Ok(tops.into_iter().collect())
}

/// Splits a zip entry name into safe components; rejects absolute paths, `..`, drive letters.
fn safe_parts(name: &str) -> Result<Vec<String>> {
    let name = name.replace('\\', "/");
    if name.starts_with('/') {
        bail!("zip entry {name:?} has an absolute path");
    }
    let mut parts = Vec::new();
    for seg in name.split('/') {
        match seg {
            "" | "." => {}
            ".." => bail!("zip entry {name:?} points outside the addon folder"),
            s if s.contains(':') || s.contains('\0') => {
                bail!("zip entry {name:?} has an invalid name")
            }
            s => parts.push(s.to_string()),
        }
    }
    Ok(parts)
}

fn check_folder_name(name: &str) -> Result<()> {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', '\0']) {
        bail!("invalid addon folder name {name:?}");
    }
    Ok(())
}

fn swap(
    addons: &Path,
    staging: &Path,
    backup: &Path,
    new: &[String],
    old: &[String],
) -> Result<()> {
    std::fs::create_dir(backup).with_context(|| format!("creating {}", backup.display()))?;
    let mut backed_up: Vec<&String> = Vec::new();
    let mut moved_in: Vec<&String> = Vec::new();
    let targets: BTreeSet<&String> = old.iter().chain(new).collect();
    let run = || -> Result<()> {
        for name in targets {
            let src = addons.join(name);
            if src.symlink_metadata().is_ok() {
                std::fs::rename(&src, backup.join(name))
                    .with_context(|| format!("backing up {}", src.display()))?;
                backed_up.push(name);
            }
        }
        for name in new {
            let dst = addons.join(name);
            std::fs::rename(staging.join(name), &dst)
                .with_context(|| format!("installing {}", dst.display()))?;
            moved_in.push(name);
        }
        Ok(())
    };
    let Err(err) = run() else { return Ok(()) };
    for name in moved_in {
        let _ = std::fs::remove_dir_all(addons.join(name));
    }
    let mut lost = Vec::new();
    for name in backed_up {
        if std::fs::rename(backup.join(name), addons.join(name)).is_err() {
            lost.push(name.as_str());
        }
    }
    if lost.is_empty() {
        let _ = std::fs::remove_dir_all(backup);
        Err(err)
    } else {
        Err(err.context(format!(
            "could not restore {} - the old folders are kept in {}",
            lost.join(", "),
            backup.display()
        )))
    }
}

/// Installs a provider addon ([`pick_file`]) and returns its new record. Existing folders with the
/// same names are replaced; use [`crate::store::upsert_addon`] to drop records that owned them.
pub fn install_remote(
    inst: &Installation,
    remote: &RemoteAddon,
    channel: Channel,
    download: Download,
) -> Result<Addon> {
    let file = pick_file(remote, channel)?;
    install_file(inst, &new_record(remote, channel), file, download)
}

/// The file a new install of `remote` takes: the newest allowed by `channel`, else the newest at all.
pub fn pick_file(remote: &RemoteAddon, channel: Channel) -> Result<&RemoteFile> {
    remote
        .latest_for(channel)
        .or_else(|| remote.latest_for(Channel::Alpha))
        .with_context(|| format!("{} has no release for this game version", remote.name))
}

/// A record for `remote` with nothing installed yet.
pub fn new_record(remote: &RemoteAddon, channel: Channel) -> Addon {
    Addon {
        id: new_id(),
        name: remote.name.clone(),
        author: remote.author.clone(),
        summary: remote.summary.clone(),
        provider: Some(remote.provider.clone()),
        external_id: Some(remote.external_id.clone()),
        url: remote.url.clone(),
        thumbnail_url: remote.thumbnail_url.clone(),
        folders: vec![],
        installed_version: None,
        installed_file_id: None,
        installed_at: None,
        game_version: None,
        channel,
        latest: remote.latest_for(channel).cloned(),
        ignored: None,
        auto_update: false,
        dependencies: vec![],
    }
}

/// Installs `file` over `addon` (update, reinstall, or a channel switch) and returns the updated record.
pub fn install_file(
    inst: &Installation,
    addon: &Addon,
    file: &RemoteFile,
    download: Download,
) -> Result<Addon> {
    if file.download_url.is_empty() {
        bail!(
            "{}: the author only allows downloads through the source's own app or website",
            addon.name
        );
    }
    let zip =
        download(&file.download_url).with_context(|| format!("downloading {}", addon.name))?;
    install_zip_file(inst, addon, file, zip)
}

/// Installs `zip`, the archive of `file`, over `addon` and returns the updated record.
pub fn install_zip_file(
    inst: &Installation,
    addon: &Addon,
    file: &RemoteFile,
    zip: impl Read + Seek,
) -> Result<Addon> {
    let folders = install_zip(inst, zip, &addon.folders)
        .with_context(|| format!("installing {}", addon.name))?;
    let mut a = addon.clone();
    let main = folders
        .iter()
        .min_by(|x, y| x.len().cmp(&y.len()).then(x.cmp(y)));
    a.game_version = main
        .and_then(|m| read_folder_toc(&inst.addons_dir().join(m)).ok().flatten())
        .and_then(|t| t.forever_game_version());
    a.folders = folders;
    a.installed_version = Some(file.version.clone());
    a.installed_file_id = Some(file.id.clone());
    a.installed_at = Some(now_rfc3339());
    a.dependencies = file.dependencies.clone();
    Ok(a)
}

/// Updates to the latest known file (`addon.latest`, filled by update checks).
pub fn update_addon(inst: &Installation, addon: &Addon, download: Download) -> Result<Addon> {
    let file = addon
        .latest
        .as_ref()
        .with_context(|| format!("no update known for {}", addon.name))?;
    install_file(inst, addon, file, download)
}

/// Deletes the addon's folders (missing ones are fine).
pub fn remove_addon(inst: &Installation, addon: &Addon) -> Result<()> {
    let mut failed = Vec::new();
    for name in &addon.folders {
        check_folder_name(name)?;
        let path: PathBuf = inst.addons_dir().join(name);
        let res = match path.symlink_metadata() {
            Ok(m) if m.is_dir() => std::fs::remove_dir_all(&path),
            Ok(_) => std::fs::remove_file(&path),
            Err(_) => continue,
        };
        if let Err(e) = res {
            failed.push(format!("{name}: {e}"));
        }
    }
    if !failed.is_empty() {
        bail!("could not remove {}", failed.join("; "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_entry_names() {
        assert!(safe_parts("../evil/x.lua").is_err());
        assert!(safe_parts("/etc/passwd").is_err());
        assert!(safe_parts("A\\..\\..\\x").is_err());
        assert!(safe_parts("C:/x").is_err());
        assert_eq!(safe_parts("A/./b.lua").unwrap(), ["A", "b.lua"]);
    }
}
