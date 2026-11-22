//! Install / update / reinstall / remove (F3.2, F4.3, F5.1, F5.2).
//!
//! Folder swap: the zip is validated and extracted to a staging dir next to `AddOns` (same
//! filesystem, so moves are renames), the addon's old folders and every folder about to be
//! replaced are moved to a backup dir, the new folders are moved in, and on any failure the new
//! folders are removed and the backups restored. Old folders absent from the new zip are gone.
//!
//! Work dirs next to AddOns, by name: `.addons-staging-*` (the extracted zip), `.addons-backup-*`
//! (folders moved aside during a swap), `.addons-trash-*` (a finished swap's backup being deleted)
//! and `.addons-kept-*` (a rollback that couldn't put everything back keeps the old folders there).
//! [`recover`] cleans up after a crash; `.addons-lock` keeps it away from installs in other processes.

use crate::model::{Addon, Channel, Installation, RemoteAddon, RemoteFile};
use crate::toc::{Toc, read_folder_toc};
use crate::util::{new_id, now_rfc3339, now_stamp};
use anyhow::{Context, Result, bail};
use std::collections::BTreeSet;
use std::fs::File;
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
    let staging = work.join(format!("{STAGING}{tag}"));
    let backup = work.join(format!("{BACKUP}{tag}"));
    let kept = work.join(format!("{KEPT}{tag}"));
    // A filesystem without locks (some network shares) only loses the guard against a second process.
    let _lock = lock_file(work)
        .and_then(|f| {
            f.lock_shared()
                .with_context(|| format!("locking {}", work.join(LOCK).display()))?;
            Ok(f)
        })
        .inspect_err(|e| crate::log::info(&format!("installing without the lock: {e:#}")))
        .ok();

    let extracted = extract(zip, &staging);
    let result = extracted
        .and_then(|new| swap(&addons, &staging, &backup, &kept, &new, old_folders).map(|_| new));
    let _ = std::fs::remove_dir_all(&staging);
    if result.is_ok() {
        // Renamed first: after a crash mid-delete, `recover` must not put dropped folders back.
        let trash = work.join(format!("{TRASH}{tag}"));
        let doomed = if std::fs::rename(&backup, &trash).is_ok() {
            trash
        } else {
            backup
        };
        let _ = std::fs::remove_dir_all(doomed);
    }
    result
}

const STAGING: &str = ".addons-staging-";
const BACKUP: &str = ".addons-backup-";
const TRASH: &str = ".addons-trash-";
const KEPT: &str = ".addons-kept-";
const LOCK: &str = ".addons-lock";

/// The lock file of an installation's work dirs: installs hold it shared, [`recover`] exclusive,
/// across processes (two Culm windows, or one Culm stuck while another starts).
fn lock_file(work: &Path) -> Result<File> {
    let path = work.join(LOCK);
    std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .with_context(|| format!("opening {}", path.display()))
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
    kept: &Path,
    new: &[String],
    old: &[String],
) -> Result<()> {
    // A developer's checkout of a folder the addon doesn't own may hold unsaved work.
    if let Some(git) = new
        .iter()
        .find(|n| !old.contains(n) && addons.join(n).join(".git").exists())
    {
        bail!("{git} is a git checkout (a developer's copy) - not replacing it");
    }
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
        // `recover` never deletes a kept copy whose folder is in AddOns: that one may be broken.
        let keep = if std::fs::rename(backup, kept).is_ok() {
            kept
        } else {
            backup
        };
        Err(err.context(format!(
            "could not restore {} - the old folders are kept in {}",
            lost.join(", "),
            keep.display()
        )))
    }
}

/// What [`recover`] left undone.
#[derive(Debug, Default)]
pub struct Leftovers {
    /// Folders missing in AddOns whose copy is still in a backup: not gone, only not back yet.
    pub folders: Vec<String>,
    pub errors: Vec<String>,
}

/// Cleans up after an install that was cut off (crash, power loss): folders in a backup dir whose
/// name is missing in AddOns go back. Then a crash's backup and leftover staging and trash dirs
/// are deleted; a kept dir only once it is empty, since its folders may be the only good copies.
/// A dir that can't be put back fully stays for the next try. Skipped while an install runs
/// (in any process).
pub fn recover(inst: &Installation) -> Leftovers {
    let mut left = Leftovers::default();
    let addons = inst.addons_dir();
    let Some(work) = addons.parent() else {
        return left;
    };
    let work_dirs = || -> Vec<(String, PathBuf)> {
        let Ok(entries) = std::fs::read_dir(work) else {
            return vec![]; // no Interface folder yet
        };
        entries
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
            .filter(|(n, _)| {
                [STAGING, BACKUP, TRASH, KEPT]
                    .iter()
                    .any(|p| n.starts_with(p))
            })
            .collect()
    };
    if work_dirs().is_empty() {
        return left;
    }
    // Busy: an install runs (here or in another process), its dirs are live. Without locks (a
    // filesystem that has none) cleanup goes ahead, as installs do.
    let lock = lock_file(work);
    let busy = match &lock {
        Ok(f) => match f.try_lock() {
            Ok(()) => false,
            Err(std::fs::TryLockError::WouldBlock) => true,
            Err(std::fs::TryLockError::Error(e)) => {
                crate::log::info(&format!("cleaning up without the lock: {e}"));
                false
            }
        },
        Err(e) => {
            crate::log::info(&format!("cleaning up without the lock: {e:#}"));
            false
        }
    };
    // Listed under the lock: no install can finish or start meanwhile.
    for (name, path) in work_dirs().iter().filter(|_| !busy) {
        let res = if name.starts_with(BACKUP) {
            restore(path, &addons).and_then(|()| Ok(std::fs::remove_dir_all(path)?))
        } else if name.starts_with(KEPT) {
            restore(path, &addons).map(|()| {
                let _ = std::fs::remove_dir(path); // only if empty
            })
        } else {
            std::fs::remove_dir_all(path).map_err(Into::into)
        };
        if let Err(e) = res {
            left.errors.push(format!(
                "could not clean up an interrupted install: {}: {e:#}",
                path.display()
            ));
        }
    }
    for (name, path) in &work_dirs() {
        if !(name.starts_with(BACKUP) || name.starts_with(KEPT)) {
            continue;
        }
        for e in std::fs::read_dir(path).into_iter().flatten().flatten() {
            if addons.join(e.file_name()).symlink_metadata().is_err() {
                left.folders
                    .push(e.file_name().to_string_lossy().into_owned());
            }
        }
    }
    left.folders.sort();
    left.folders.dedup();
    left
}

/// Moves each folder in `backup` whose name is missing in `addons` back.
fn restore(backup: &Path, addons: &Path) -> Result<()> {
    for e in std::fs::read_dir(backup)? {
        let e = e?;
        let dst = addons.join(e.file_name());
        if dst.symlink_metadata().is_err() {
            std::fs::create_dir_all(addons)?;
            std::fs::rename(e.path(), &dst)
                .with_context(|| format!("putting back {}", dst.display()))?;
            crate::log::info(&format!(
                "put back {} after an interrupted install",
                dst.display()
            ));
        }
    }
    Ok(())
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
        game_versions: vec![],
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
    let toc = main.and_then(|m| read_folder_toc(&inst.addons_dir().join(m)).ok().flatten());
    a.game_version = toc.as_ref().and_then(Toc::forever_game_version);
    a.game_versions = toc.as_ref().map(Toc::game_versions).unwrap_or_default();
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

    /// F3.2: a rollback that can't put everything back marks its backup kept, so `recover` never
    /// deletes the only good copy.
    #[cfg(unix)]
    #[test]
    fn f3_2_failed_rollback_keeps_its_backup() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let (addons, staging) = (tmp.path().join("AddOns"), tmp.path().join("staging"));
        let (backup, kept) = (tmp.path().join("backup"), tmp.path().join("kept"));
        std::fs::create_dir_all(addons.join("Foo")).unwrap();
        std::fs::write(addons.join("Foo/old.lua"), "old").unwrap();
        // The new Foo moves in, then Foo_Lib is missing from staging: the swap fails. The new Foo
        // can't be deleted (a read-only folder inside), so the old one can't go back.
        let sub = staging.join("Foo/sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("x.lua"), "new").unwrap();
        std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o555)).unwrap();
        let probe = std::fs::File::create(sub.join("probe")).is_ok();
        let new = ["Foo".to_string(), "Foo_Lib".to_string()];
        let res = (!probe).then(|| swap(&addons, &staging, &backup, &kept, &new, &["Foo".into()]));
        let _ = std::fs::set_permissions(
            addons.join("Foo/sub"),
            std::fs::Permissions::from_mode(0o755),
        );
        let _ = std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o755));
        let Some(res) = res else {
            eprintln!("skipped: running as root, folders stay writable");
            return;
        };
        let err = format!("{:#}", res.unwrap_err());
        assert!(err.contains("could not restore Foo"), "{err}");
        assert!(err.contains(&kept.display().to_string()), "{err}");
        assert!(!backup.exists());
        assert_eq!(
            std::fs::read_to_string(kept.join("Foo/old.lua")).unwrap(),
            "old"
        );
    }
}
