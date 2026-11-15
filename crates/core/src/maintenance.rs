//! Client maintenance (F6): cache clearing, WTF leftovers, WTF backups. Refused while WoW runs.

use crate::model::Installation;
use crate::util::{now_stamp, walk_files};
use crate::wow::is_game_running;
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// What a cleanup removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cleanup {
    pub files: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WtfBackup {
    pub path: PathBuf,
    pub size: u64,
    pub created: Option<SystemTime>,
}

impl WtfBackup {
    /// File name, e.g. `WTF-20261115-102030.zip`.
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Creation (modification) time as RFC 3339 UTC.
    pub fn created_rfc3339(&self) -> Option<String> {
        self.created.map(crate::util::rfc3339)
    }
}

/// Every tool deletes below `inst.root`: only ever act on a real Forever client folder (the stored
/// installation list is a plain JSON file), and never while the game runs.
fn refuse_unsafe(inst: &Installation) -> Result<()> {
    let is_client = inst
        .root
        .file_name()
        .is_some_and(|n| n.eq_ignore_ascii_case("_forever_"));
    if !is_client || !inst.root.is_dir() {
        bail!("{} is not a WoW Forever client folder", inst.root.display());
    }
    if is_game_running(inst) {
        bail!("World of Warcraft is running - close the game first");
    }
    Ok(())
}

fn remove_files(files: &[PathBuf]) -> Result<Cleanup> {
    let mut c = Cleanup::default();
    for f in files {
        let size = f.metadata().map(|m| m.len()).unwrap_or(0);
        std::fs::remove_file(f).with_context(|| format!("deleting {}", f.display()))?;
        c.files += 1;
        c.bytes += size;
    }
    Ok(c)
}

/// Deletes `Cache/` (or only `Cache/WDB`) of the client.
pub fn clear_cache(inst: &Installation, wdb_only: bool) -> Result<Cleanup> {
    refuse_unsafe(inst)?;
    let dir = if wdb_only {
        inst.root.join("Cache/WDB")
    } else {
        inst.root.join("Cache")
    };
    if dir
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        bail!("{} is a link - not following it", dir.display());
    }
    let c = remove_files(&walk_files(&dir)?)?;
    if dir.exists() {
        std::fs::remove_dir_all(&dir).with_context(|| format!("deleting {}", dir.display()))?;
    }
    Ok(c)
}

/// Deletes `WTF/**/*.lua.bak` (previous SavedVariables) and `WTF/*.old`.
pub fn clean_backup_files(inst: &Installation) -> Result<Cleanup> {
    refuse_unsafe(inst)?;
    let wtf = inst.root.join("WTF");
    let files: Vec<PathBuf> = walk_files(&wtf)?
        .into_iter()
        .filter(|f| {
            let name = f
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            name.ends_with(".lua.bak")
                || (name.ends_with(".old") && f.parent() == Some(wtf.as_path()))
        })
        .collect();
    remove_files(&files)
}

/// Zips the client's `WTF` folder into `backups_dir/WTF-<timestamp>.zip`.
pub fn backup_wtf(inst: &Installation, backups_dir: &Path) -> Result<PathBuf> {
    refuse_unsafe(inst)?;
    let wtf = inst.root.join("WTF");
    if !wtf.is_dir() {
        bail!("{} has no WTF folder", inst.root.display());
    }
    std::fs::create_dir_all(backups_dir)
        .with_context(|| format!("creating {}", backups_dir.display()))?;
    let mut target = backups_dir.join(format!("WTF-{}.zip", now_stamp()));
    let mut n = 1;
    while target.exists() {
        target = backups_dir.join(format!("WTF-{}-{n}.zip", now_stamp()));
        n += 1;
    }
    let tmp = target.with_extension("zip.part");
    let write = || -> Result<()> {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&tmp)?);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .large_file(true);
        let mut files = walk_files(&wtf)?;
        files.sort();
        for f in files {
            let rel = f
                .strip_prefix(&inst.root)?
                .to_string_lossy()
                .replace('\\', "/");
            zip.start_file(rel, opts)?;
            let mut src =
                std::fs::File::open(&f).with_context(|| format!("reading {}", f.display()))?;
            std::io::copy(&mut src, &mut zip)?;
        }
        zip.finish()?;
        Ok(())
    };
    if let Err(e) = write() {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.context("writing WTF backup"));
    }
    std::fs::rename(&tmp, &target)?;
    Ok(target)
}

/// WTF backups in `backups_dir`, newest first.
pub fn list_wtf_backups(backups_dir: &Path) -> Result<Vec<WtfBackup>> {
    let rd = match std::fs::read_dir(backups_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e).with_context(|| format!("reading {}", backups_dir.display())),
    };
    let mut out: Vec<WtfBackup> = rd
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.starts_with("WTF-") && n.ends_with(".zip")
        })
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            Some(WtfBackup {
                path: e.path(),
                size: m.len(),
                created: m.modified().ok(),
            })
        })
        .collect();
    // Names embed the timestamp, so name order is creation order.
    out.sort_by(|a, b| b.path.cmp(&a.path));
    Ok(out)
}
