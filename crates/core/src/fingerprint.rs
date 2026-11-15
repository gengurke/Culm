//! CurseForge folder fingerprint (murmur2) (F2.3).
//! Only the TOC/XML include tree (plus a top-level `Bindings.xml`) is hashed, never media files.

use crate::util::walk_files;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// CurseForge's MurmurHash2 variant: seed 1, whitespace bytes (9, 10, 13, 32) are skipped.
pub fn murmur2(data: &[u8]) -> u32 {
    const M: u32 = 0x5bd1e995;
    let is_ws = |b: &u8| matches!(b, 9 | 10 | 13 | 32);
    let len = data.iter().filter(|b| !is_ws(b)).count() as u32;
    let mut h: u32 = 1 ^ len;
    let (mut k, mut shift) = (0u32, 0u32);
    for &b in data.iter().filter(|b| !is_ws(b)) {
        k |= (b as u32) << shift;
        shift += 8;
        if shift == 32 {
            let mut m = k.wrapping_mul(M);
            m = (m ^ (m >> 24)).wrapping_mul(M);
            h = h.wrapping_mul(M) ^ m;
            k = 0;
            shift = 0;
        }
    }
    if shift > 0 {
        h = (h ^ k).wrapping_mul(M);
    }
    let h = (h ^ (h >> 13)).wrapping_mul(M);
    h ^ (h >> 15)
}

/// The fingerprint CurseForge knows the folder by.
pub fn folder_fingerprint(folder: &Path) -> Result<u32> {
    Ok(folder_fingerprints(folder)?.0)
}

/// The CurseForge fingerprint and the Hub's md5 fingerprint, reading each file once.
/// md5: hex md5 per file, hex strings sorted and concatenated, md5 of that.
// shortcut: the md5 uses the CurseForge file selection; the Hub's scanner takes every XML include of
// a line, not just the first. Same result unless one XML line has two includes; port it if Hub misses show up.
pub fn folder_fingerprints(folder: &Path) -> Result<(u32, String)> {
    let files: Vec<Vec<u8>> = fingerprint_files(folder)?
        .iter()
        .filter_map(|f| std::fs::read(f).ok()) // unreadable files are skipped
        .collect();
    let mut hashes: Vec<u32> = files.iter().map(|b| murmur2(b)).collect();
    hashes.sort_unstable();
    let concat: String = hashes.iter().map(u32::to_string).collect();
    let mut md5s: Vec<String> = files.iter().map(|b| md5_hex(b)).collect();
    md5s.sort_unstable();
    Ok((
        murmur2(concat.as_bytes()),
        md5_hex(md5s.concat().as_bytes()),
    ))
}

/// MD5 (RFC 1321) as lowercase hex. Not for security, only to match the Hub's fingerprints.
pub fn md5_hex(data: &[u8]) -> String {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let k: Vec<u32> = (0..64)
        .map(|i| ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32)
        .collect();
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64).wrapping_mul(8)).to_le_bytes());
    let mut h: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
    for chunk in msg.chunks(64) {
        let m: Vec<u32> = chunk
            .chunks(4)
            .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
            .collect();
        let [mut a, mut b, mut c, mut d] = h;
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d]) {
            *x = x.wrapping_add(y);
        }
    }
    h.iter()
        .flat_map(|x| x.to_le_bytes())
        .map(|b| format!("{b:02x}"))
        .collect()
}

const TOC_SUFFIXES: [&str; 11] = [
    "mainline", "bcc", "tbc", "classic", "vanilla", "wrath", "wotlkc", "cata", "mists", "forever",
    "camelot",
];

/// The files that go into the fingerprint (absolute paths, sorted case-insensitively).
pub fn fingerprint_files(folder: &Path) -> Result<Vec<PathBuf>> {
    // Lowercase relative path -> real relative path: TOCs reference files with sloppy casing.
    let mut map: HashMap<String, String> = HashMap::new();
    for f in walk_files(folder)? {
        if let Ok(rel) = f.strip_prefix(folder) {
            let rel = rel.to_string_lossy().replace('\\', "/");
            map.insert(rel.to_lowercase(), rel);
        }
    }
    let folder_lc = folder
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let mut matched: Vec<String> = Vec::new();
    let mut roots: Vec<String> = Vec::new();
    let mut top: Vec<&String> = map.keys().filter(|k| !k.contains('/')).collect();
    top.sort();
    for lc in top {
        if lc == "bindings.xml" {
            matched.push(map[lc].clone());
        } else if is_root_toc(&folder_lc, lc) {
            roots.push(lc.clone());
        }
    }
    for r in roots {
        process_include(folder, &map, &mut matched, &r);
    }
    matched.sort_by_key(|p| p.to_lowercase());
    Ok(matched.into_iter().map(|rel| folder.join(rel)).collect())
}

/// `<folder>.toc` or `<folder>[-_|]<flavor>.toc` (lowercase input).
fn is_root_toc(folder_lc: &str, name_lc: &str) -> bool {
    let Some(rest) = name_lc.strip_prefix(folder_lc) else {
        return false;
    };
    let Some(rest) = rest.strip_suffix(".toc") else {
        return false;
    };
    rest.is_empty()
        || rest
            .strip_prefix(['-', '_', '|'])
            .is_some_and(|s| TOC_SUFFIXES.contains(&s))
}

fn process_include(
    folder: &Path,
    map: &HashMap<String, String>,
    matched: &mut Vec<String>,
    lc: &str,
) {
    let Some(real) = map.get(lc) else { return };
    if matched.contains(real) {
        return;
    }
    matched.push(real.clone());
    // The extension check is case-sensitive, as in the reference fingerprints.
    let is_toc = real.ends_with(".toc");
    if !is_toc && !real.ends_with(".xml") {
        return;
    }
    let Ok(bytes) = std::fs::read(folder.join(real)) else {
        return;
    };
    let text = String::from_utf8_lossy(&bytes);
    let includes = if is_toc {
        toc_includes(&text)
    } else {
        xml_includes(&text)
    };
    let dir = real.rsplit_once('/').map_or("", |(d, _)| d);
    for inc in includes {
        if inc.chars().any(|c| c == '|' || (c as u32) < 0x20) {
            break;
        }
        if let Some(path) = normalize(&format!("{dir}/{}", inc.replace('\\', "/"))) {
            process_include(folder, map, matched, &path.to_lowercase());
        }
    }
}

/// `path.normalize` for a folder-relative path; None if it escapes the folder.
fn normalize(p: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            s => parts.push(s),
        }
    }
    Some(parts.join("/"))
}

/// JS `String.prototype.trim` (also strips the BOM).
fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// Lines matching `^\s*((?:(?<!\.\.).)+\.(?:xml|lua))\s*$` (case-insensitive) after `#` comments are cut.
fn toc_includes(text: &str) -> Vec<String> {
    text.split('\n')
        .filter_map(|line| {
            let line = js_trim(line.split('#').next().unwrap_or(""));
            let b = line.as_bytes();
            let n = b.len();
            let ext_ok = n >= 5
                && (b[n - 4..].eq_ignore_ascii_case(b".xml")
                    || b[n - 4..].eq_ignore_ascii_case(b".lua"));
            (ext_ok && !b[..n - 5].windows(2).any(|w| w == b"..")).then(|| line.to_string())
        })
        .collect()
}

/// Per trimmed line, the first `<(?:Include|Script)\s+file=["']((?:(?<!\.\.).)+)["']\s*\/>`
/// (case-insensitive, greedy group), after `<!-- -->` comments are removed.
fn xml_includes(text: &str) -> Vec<String> {
    let mut clean = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find("<!--") {
        clean.push_str(&rest[..i]);
        match rest[i + 4..].find("-->") {
            Some(j) => rest = &rest[i + 4 + j + 3..],
            None => {
                // No closing tag: the regex does not match, keep the text as is.
                clean.push_str(&rest[i..]);
                rest = "";
            }
        }
    }
    clean.push_str(rest);
    clean
        .split('\n')
        .filter_map(|l| xml_line_include(js_trim(l)).map(|s| js_trim(s).to_string()))
        .collect()
}

fn xml_line_include(line: &str) -> Option<&str> {
    let b = line.as_bytes();
    let n = b.len();
    let starts_ci = |at: usize, word: &str| {
        b.len() >= at + word.len() && b[at..at + word.len()].eq_ignore_ascii_case(word.as_bytes())
    };
    for start in (0..n).filter(|&i| b[i] == b'<') {
        let mut i = start + 1;
        if starts_ci(i, "include") {
            i += 7;
        } else if starts_ci(i, "script") {
            i += 6;
        } else {
            continue;
        }
        let ws = b[i..]
            .iter()
            .take_while(|c| c.is_ascii_whitespace())
            .count();
        if ws == 0 || !starts_ci(i + ws, "file=") {
            continue;
        }
        i += ws + 5;
        if i >= n || !matches!(b[i], b'"' | b'\'') {
            continue;
        }
        let g0 = i + 1;
        // Greedy extent: a char may not follow "..".
        let mut end = g0;
        while end < n && !(b[end - 2] == b'.' && b[end - 1] == b'.') {
            end += 1;
        }
        for k in (g0 + 1..=end.min(n.saturating_sub(1))).rev() {
            if !matches!(b[k], b'"' | b'\'') {
                continue;
            }
            let after = &b[k + 1..];
            let ws = after.iter().take_while(|c| c.is_ascii_whitespace()).count();
            if after[ws..].starts_with(b"/>") {
                return Some(&line[g0..k]);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn murmur_known_values() {
        // Whitespace is ignored entirely.
        assert_eq!(murmur2(b"a b\r\n"), murmur2(b"ab"));
        assert_ne!(murmur2(b"ab"), murmur2(b"ba"));
    }

    #[test]
    fn md5_rfc_vectors() {
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(
            md5_hex(b"The quick brown fox jumps over the lazy dog"),
            "9e107d9d372bb6826bd81d3542a419d6"
        );
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            md5_hex("1234567890".repeat(8).as_bytes()),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
    }

    #[test]
    fn xml_include_regex_semantics() {
        assert_eq!(
            xml_line_include(r#"<Script file="Panel.lua"/>"#),
            Some("Panel.lua")
        );
        assert_eq!(
            xml_line_include(r#"<include  FILE='a\b.xml' />"#),
            Some(r"a\b.xml")
        );
        assert_eq!(xml_line_include(r#"<Script file="../x.lua"/>"#), None);
        // Greedy group swallows a second include on the same line.
        assert_eq!(
            xml_line_include(r#"<Script file="a.lua"/><Script file="b.lua"/>"#),
            Some(r#"a.lua"/><Script file="b.lua"#)
        );
        assert_eq!(xml_line_include("<Ui/>"), None);
    }

    #[test]
    fn toc_include_regex_semantics() {
        let got =
            toc_includes("## Title: x\r\nCore.lua\r\n  Mod\\A.XML # c\r\n..\\x.lua\r\nimg.tga\r\n");
        assert_eq!(got, ["Core.lua", "Mod\\A.XML"]);
    }
}
