//! WoW Forever installation detection (F1.1, F1.2) and the "is the game running" safety check.

use crate::model::{Flavor, Installation};
use crate::util::fnv1a64;
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

const CLIENT_DIR: &str = "_forever_";
const PRODUCT: &str = "wowf";

/// Wine/Proton prefixes (and similar) worth probing, derived from `$HOME`. Cheap: at most one
/// `read_dir` per glob-like parent, no recursive walks.
pub fn default_search_roots() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return vec![];
    };
    let mut roots = vec![home.join(".wine"), home.join("Games")];
    // Parents whose children are prefixes (one level), with an optional suffix inside each child.
    let parents: [(PathBuf, &str); 7] = [
        (home.join("Faugus"), ""),
        (home.join("Games"), ""),
        (home.join(".local/share/bottles/bottles"), ""),
        (
            home.join(".var/app/com.usebottles.bottles/data/bottles/bottles"),
            "",
        ),
        (home.join(".local/share/Steam/steamapps/compatdata"), "pfx"),
        (home.join(".steam/steam/steamapps/compatdata"), "pfx"),
        (
            home.join(".var/app/com.valvesoftware.Steam/data/Steam/steamapps/compatdata"),
            "pfx",
        ),
    ];
    for (parent, suffix) in parents {
        let Ok(rd) = std::fs::read_dir(&parent) else {
            continue;
        };
        let mut kids: Vec<PathBuf> = rd
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| e.path().join(suffix))
            .collect();
        kids.sort();
        roots.extend(kids);
    }
    roots
}

/// Finds Forever clients under the given roots. A root may be a Wine prefix, a `drive_c`,
/// a folder containing `World of Warcraft`, or a WoW root itself. Only clients whose
/// `.build.info` lists product `wowf` count. Results are de-duplicated by canonical path.
pub fn detect_installations(search_roots: &[PathBuf]) -> Vec<Installation> {
    let mut found: Vec<Installation> = Vec::new();
    for root in search_roots {
        let candidates = [
            root.clone(),
            root.join("World of Warcraft"),
            root.join("Program Files (x86)/World of Warcraft"),
            root.join("Program Files/World of Warcraft"),
            root.join("drive_c/Program Files (x86)/World of Warcraft"),
            root.join("drive_c/Program Files/World of Warcraft"),
        ];
        for wow_root in candidates {
            let client = wow_root.join(CLIENT_DIR);
            if !client.is_dir() {
                continue;
            }
            let Some(version) = read_build_version(&wow_root) else {
                continue;
            };
            let inst = make_installation(&client, Some(version));
            if !found.iter().any(|i| i.id == inst.id) {
                found.push(inst);
            }
        }
    }
    found
}

/// Accepts the WoW root, the `_forever_` folder, its `Interface`/`Interface/AddOns`, or `Wow.exe`,
/// and normalizes to the client folder. The version is read from `.build.info` when present.
pub fn installation_from_path(p: &Path) -> Result<Installation> {
    let mut p = p.to_path_buf();
    if p.is_file() {
        p.pop(); // Wow.exe -> client folder
    }
    let name_is = |p: &Path, n: &str| {
        p.file_name()
            .is_some_and(|f| f.to_string_lossy().eq_ignore_ascii_case(n))
    };
    if name_is(&p, "AddOns") {
        p.pop();
    }
    if name_is(&p, "Interface") {
        p.pop();
    }
    let client = if name_is(&p, CLIENT_DIR) {
        p
    } else if p.join(CLIENT_DIR).is_dir() {
        p.join(CLIENT_DIR)
    } else {
        bail!(
            "no WoW Forever client (`{CLIENT_DIR}` folder) found at {}",
            p.display()
        );
    };
    if !client.is_dir() {
        bail!("{} is not a folder", client.display());
    }
    let version = client.parent().and_then(read_build_version);
    Ok(make_installation(&client, version))
}

fn make_installation(client: &Path, version: Option<String>) -> Installation {
    let root = std::fs::canonicalize(client).map_or_else(|_| client.to_path_buf(), plain);
    Installation {
        id: format!("{:016x}", fnv1a64(root.to_string_lossy().as_bytes())),
        label: "WoW Forever".to_string(),
        flavor: Flavor::Forever,
        root,
        version,
    }
}

/// `p` without the `\\?\` prefix `canonicalize` puts on Windows paths: such a path takes no `/`
/// in joins, and Explorer can't open it.
fn plain(p: PathBuf) -> PathBuf {
    match p.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest),
        _ => p,
    }
}

/// The Wine prefix holding a client (the folder that holds its `drive_c`, e.g. `battlenet`):
/// what tells two installations of the same client apart. Read from the path alone.
pub fn prefix_name(root: &Path) -> Option<String> {
    root.ancestors()
        .find(|a| a.file_name().is_some_and(|f| f == "drive_c"))?
        .parent()?
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
}

/// `Version` of the `wowf` row in `<wow_root>/.build.info` (pipe table with `Name!TYPE:len` header).
pub fn read_build_version(wow_root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(wow_root.join(".build.info")).ok()?;
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header: Vec<String> = lines
        .next()?
        .split('|')
        .map(|h| h.split('!').next().unwrap_or("").trim().to_string())
        .collect();
    let col = |name: &str| header.iter().position(|h| h == name);
    let (product, version) = (col("Product")?, col("Version")?);
    lines
        .map(|l| l.split('|').map(str::trim).collect::<Vec<_>>())
        .find(|row| row.get(product) == Some(&PRODUCT))
        .and_then(|row| row.get(version).map(|v| v.to_string()))
        .filter(|v| !v.is_empty())
}

/// True if the game of this installation runs; a game running from another client folder doesn't
/// count. Linux reads `/proc` (Wine games, see [`runs_in`]); Windows and macOS ask the OS which
/// programs run from inside the client folder.
pub fn is_game_running(inst: &Installation) -> bool {
    let root = std::fs::canonicalize(&inst.root).map_or_else(|_| inst.root.clone(), plain);
    #[cfg(target_os = "linux")]
    return wine_game_running(&root);
    #[cfg(not(target_os = "linux"))]
    return native_game_running(&root);
}

/// Windows and macOS: a program started from inside the client folder (`Wow.exe`, the macOS app
/// bundle). One that looks like the game but whose path we may not read counts too: we can't tell,
/// so maintenance stays locked rather than guess.
#[cfg(not(target_os = "linux"))]
fn native_game_running(root: &Path) -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let exe_only = ProcessRefreshKind::nothing().with_exe(UpdateKind::Always);
    let Ok(mut sys) = System::new() else {
        return true;
    };
    // Not even this process seen: the OS didn't tell.
    if sys.refresh_processes_specifics(ProcessesToUpdate::All, true, exe_only) == 0 {
        return true;
    }
    // Paths compare case-insensitively, as both file systems usually do.
    let lower = |p: &Path| PathBuf::from(p.to_string_lossy().to_lowercase());
    let root = lower(root);
    sys.processes().values().any(|p| match p.exe() {
        Some(exe) => lower(exe).starts_with(&root),
        None => {
            let name = p.name().to_string_lossy().to_lowercase();
            name == "wow.exe" || name.starts_with("world of warcraft")
        }
    })
}

/// Linux: a `Wow.exe` process (under Wine) that belongs to the client at `root` (see [`runs_in`]).
// shortcut: heuristic, Wine passes Windows paths. Upgrade if users report maintenance running while the game is open.
#[cfg(target_os = "linux")]
fn wine_game_running(root: &Path) -> bool {
    let Ok(rd) = std::fs::read_dir("/proc") else {
        return false;
    };
    rd.flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .bytes()
                .all(|b| b.is_ascii_digit())
        })
        .any(|e| {
            let dir = e.path();
            let Ok(cmdline) = std::fs::read(dir.join("cmdline")) else {
                return false;
            };
            let cmd = String::from_utf8_lossy(&cmdline)
                .to_lowercase()
                .replace('\\', "/");
            let comm = std::fs::read_to_string(dir.join("comm")).unwrap_or_default();
            let argv0 = cmd.split('\0').next().unwrap_or("");
            let is_wow = comm.trim().eq_ignore_ascii_case("wow.exe")
                || argv0.rsplit('/').next() == Some("wow.exe");
            is_wow && runs_in(&dir, root, &cmd)
        })
}

/// Whether the Wow.exe process at `/proc/<pid>` (`dir`) belongs to the client at `root`: its
/// working directory (Wine sets it to the client's Unix path) or an open file is inside `root`.
/// Only when neither can be read (another user's process) does a `_forever_` command line count,
/// so we stay on the safe side without letting every Forever client lock every other one.
#[cfg(target_os = "linux")]
fn runs_in(dir: &Path, root: &Path, cmd: &str) -> bool {
    let cwd = std::fs::read_link(dir.join("cwd"));
    if cwd.as_ref().is_ok_and(|cwd| cwd.starts_with(root)) {
        return true;
    }
    match std::fs::read_dir(dir.join("fd")) {
        Ok(fds) => fds
            .flatten()
            .any(|fd| std::fs::read_link(fd.path()).is_ok_and(|t| t.starts_with(root))),
        Err(_) => cwd.is_err() && cmd.contains("/_forever_/"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_info_picks_wowf_row() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".build.info"),
            "Branch!STRING:0|Active!DEC:1|Version!STRING:0|Product!STRING:0\n\
             eu|1|11.2.7.1|wow\neu|1|1.60.1.70938|wowf\n",
        )
        .unwrap();
        assert_eq!(
            read_build_version(dir.path()).as_deref(),
            Some("1.60.1.70938")
        );
    }
}
