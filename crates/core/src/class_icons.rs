//! The original WoW class icons for the Get Addons class chips (F4.6).
//!
//! They are Blizzard's art, so they are never bundled with this GPL program: they are downloaded
//! at runtime, once per user, from the first source that serves a valid image, and kept in the
//! data dir for good (not the cache dir, which cleaners empty). Nothing is re-encoded or altered.
//! Without them the app draws its own class glyphs.

use crate::http;
use std::path::Path;

/// Forever's nine classes, by the slug in the icon file names.
pub const CLASSES: [&str; 9] = [
    "warrior", "mage", "rogue", "druid", "hunter", "shaman", "priest", "warlock", "paladin",
];

/// Where the icons are asked for, in order: `<base>classicon_<slug>.jpg`. Blizzard's render CDN
/// (Classic, then retail), so one moved URL doesn't lose them.
pub const DEFAULT_SOURCES: [&str; 2] = [
    "https://render.worldofwarcraft.com/classic-us/icons/56/",
    "https://render.worldofwarcraft.com/us/icons/56/",
];

/// The icons are 56 px JPGs of a few KB; anything far off is not one.
const MAX_BYTES: usize = 200 << 10;
const SIDE: std::ops::RangeInclusive<u32> = 32..=128;

/// Reports each class's icon bytes to `each` as they come (None: no valid copy in
/// `<data_dir>/class-icons/` and no source gave one). A valid cached copy is used without any
/// request; a missing or corrupt one is fetched from `sources` in order and kept. A source that
/// fails once is skipped for the rest of the call, so an offline start costs at most one request
/// per source, and nothing is retried until the next call (the app makes one per run).
pub fn load(data_dir: &Path, sources: &[String], each: &mut dyn FnMut(&str, Option<Vec<u8>>)) {
    let dir = data_dir.join("class-icons");
    let mut dead = vec![false; sources.len()];
    for slug in CLASSES {
        let path = dir.join(format!("{slug}.jpg"));
        if let Some(bytes) = std::fs::read(&path).ok().filter(|b| valid(b)) {
            each(slug, Some(bytes));
            continue;
        }
        let mut found = None;
        for (base, dead) in sources.iter().zip(&mut dead) {
            if *dead {
                continue;
            }
            let url = format!("{base}classicon_{slug}.jpg");
            match http::download(&url, &mut |_, _| {}) {
                Ok(bytes) if valid(&bytes) => {
                    found = Some(bytes);
                    break;
                }
                _ => *dead = true,
            }
        }
        if let Some(bytes) = &found {
            // Best effort: a failed write only costs a download next run.
            let tmp = path.with_extension("part");
            if std::fs::create_dir_all(&dir).is_ok() && std::fs::write(&tmp, bytes).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
        }
        each(slug, found);
    }
}

/// A small image that decodes, at a sane size.
fn valid(bytes: &[u8]) -> bool {
    bytes.len() < MAX_BYTES
        && image::load_from_memory(bytes)
            .is_ok_and(|img| SIDE.contains(&img.width()) && SIDE.contains(&img.height()))
}
