//! Addons the user fetches by hand (F3.5, F4.3): when an author allows downloads only through the
//! source's own app or website, the user downloads the zip in the browser and Culm installs it.
//! A zip counts only when its size and SHA-1 are exactly the ones the source publishes, so a
//! wrong, partial or tampered file is never installed.

use crate::install::{self, install_zip_file, pick_file};
use crate::model::{Addon, Channel, FileCheck, Installation, RemoteAddon, RemoteFile};
use anyhow::{Context, Result, bail};
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

/// Whether the file at `path` is the one `check` describes.
pub fn matches(path: &Path, check: &FileCheck) -> bool {
    File::open(path).is_ok_and(|mut f| is_file(&mut f, check))
}

/// Whether the open file is the one `check` describes (size first, so most files cost one `stat`).
fn is_file(f: &mut File, check: &FileCheck) -> bool {
    let Ok(meta) = f.metadata() else {
        return false;
    };
    if !meta.is_file() || meta.len() != check.size {
        return false;
    }
    sha1_hex(f).is_ok_and(|h| h == check.sha1)
}

fn sha1_hex(mut f: impl Read) -> Result<String> {
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Every `.zip` directly in `dir` (the Downloads folder) that is exactly `check`'s file, by name:
/// a browser that fetched it twice leaves identical copies (`Foo.zip`, `Foo(1).zip`). Browsers
/// write to a temporary name (`.part`, `.crdownload`) and rename when done, so a match is always
/// complete.
pub fn find_in(dir: &Path, check: &FileCheck) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut zips: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("zip")))
        .filter(|p| matches(p, check))
        .collect();
    zips.sort();
    zips
}

/// A record for a new install of `remote` by hand: its `latest` is the file to fetch
/// ([`pick_file`]), which [`install`] then checks the zip against.
pub fn new_record(remote: &RemoteAddon, channel: Channel) -> Result<Addon> {
    let file = pick_file(remote, channel)?.clone();
    Ok(Addon {
        latest: Some(file),
        ..install::new_record(remote, channel)
    })
}

/// Installs the zip at `path` as `addon`'s latest file after checking it is exactly that file.
pub fn install(inst: &Installation, addon: &Addon, path: &Path) -> Result<Addon> {
    let file: &RemoteFile = addon
        .latest
        .as_ref()
        .with_context(|| format!("no update known for {}", addon.name))?;
    let Some(check) = &file.check else {
        bail!(
            "{}: check for updates first, so Culm can recognise the file",
            addon.name
        );
    };
    // Checked and installed from one open file: a file swapped in meanwhile is never installed.
    let mut zip = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    if !is_file(&mut zip, check) {
        bail!("this file is not {} {}", addon.name, file.version);
    }
    zip.rewind()?;
    install_zip_file(inst, addon, file, zip).with_context(|| format!("installing {}", addon.name))
}
