//! Small shared helpers (ids, timestamps, atomic writes, dir walking).

use anyhow::{Context, Result};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// FNV-1a 64: stable across Rust versions (unlike `DefaultHasher`), used for path-derived ids.
pub(crate) fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Random uuid-v4-shaped id (randomness from std's per-process SipHash keys plus time).
pub(crate) fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut words = [0u64; 2];
    for (i, w) in words.iter_mut().enumerate() {
        let mut h = RandomState::new().build_hasher();
        h.write_u128(nanos);
        h.write_usize(i);
        *w = h.finish();
    }
    let b: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-4{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[13..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Current time as RFC 3339 UTC, e.g. "2026-11-15T10:20:30Z".
pub(crate) fn now_rfc3339() -> String {
    rfc3339(SystemTime::now())
}

/// `t` as RFC 3339 UTC (whole seconds).
pub(crate) fn rfc3339(t: SystemTime) -> String {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, m, d, hh, mm, ss) = civil(secs);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Compact local-free timestamp for file names, e.g. "20261115-102030".
pub(crate) fn now_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, m, d, hh, mm, ss) = civil(secs);
    format!("{y:04}{m:02}{d:02}-{hh:02}{mm:02}{ss:02}")
}

/// Unix seconds -> (year, month, day, hour, min, sec) in UTC (Howard Hinnant's days_from_civil inverse).
fn civil(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (
        y,
        m,
        d,
        (rem / 3600) as u32,
        (rem % 3600 / 60) as u32,
        (rem % 60) as u32,
    )
}

/// Write via a temp file in the same directory + rename, so readers never see a half-written file.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path.parent().context("path has no parent")?;
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let name = path.file_name().context("path has no file name")?;
    // Unique per process *and* call, so two threads never share a temp file.
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let tmp = dir.join(format!(
        ".{}.tmp-{}-{}",
        name.to_string_lossy(),
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let write = || -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        // On disk before the rename, so a crash can't leave an empty list behind.
        f.sync_all()
    };
    if let Err(e) = write() {
        let _ = std::fs::remove_file(&tmp);
        return Err(e).with_context(|| format!("writing {}", tmp.display()));
    }
    std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
}

/// All regular files below `dir` (symlinks are not followed). Missing dir -> empty.
pub(crate) fn walk_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let rd = match std::fs::read_dir(&d) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e).with_context(|| format!("reading {}", d.display())),
        };
        for entry in rd {
            let entry = entry.with_context(|| format!("reading {}", d.display()))?;
            let ft = entry.file_type()?;
            if ft.is_dir() {
                stack.push(entry.path());
            } else if ft.is_file() {
                out.push(entry.path());
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(civil(1_763_202_030), (2025, 11, 15, 10, 20, 30));
        assert_eq!(civil(951_782_400), (2000, 2, 29, 0, 0, 0));
    }

    #[test]
    fn ids_are_unique_and_uuid_shaped() {
        let (a, b) = (new_id(), new_id());
        assert_ne!(a, b);
        assert_eq!(a.len(), 36);
    }
}
