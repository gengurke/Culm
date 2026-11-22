//! TOC parsing and Forever TOC selection (F2.1).

use anyhow::{Context, Result};
use std::path::Path;

/// The fields of one `.toc` file we care about.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Toc {
    pub file_name: String,
    pub title: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub notes: Option<String>,
    /// Raw `## Interface` numbers, e.g. ["11509", "20506", "16001"].
    pub interface: Vec<String>,
    pub curse_project_id: Option<String>,
    pub wowi_id: Option<String>,
    pub wago_id: Option<String>,
    /// Required dependencies (`Dependencies`, `RequiredDeps`, any `Dep*` key).
    pub dependencies: Vec<String>,
}

impl Toc {
    /// Game versions of all interface numbers, e.g. ["1.15.9", "2.5.6", "1.60.1"].
    pub fn game_versions(&self) -> Vec<String> {
        self.interface
            .iter()
            .filter_map(|i| game_version(i))
            .collect()
    }

    /// The Forever game version if listed (1.60+ on major 1), else the first one.
    pub fn forever_game_version(&self) -> Option<String> {
        let all = self.game_versions();
        all.iter()
            .find(|v| {
                let mut p = v.split('.');
                p.next() == Some("1") && p.next().and_then(|m| m.parse::<u32>().ok()) >= Some(60)
            })
            .or(all.first())
            .cloned()
    }

    /// Provider ids from TOC metadata, keyed by our provider names.
    pub fn provider_ids(&self) -> Vec<(String, String)> {
        [
            (crate::providers::curseforge::NAME, &self.curse_project_id),
            (crate::providers::wowinterface::NAME, &self.wowi_id),
            // shortcut: no Wago provider yet; becomes its `NAME` when one lands.
            ("Wago", &self.wago_id),
        ]
        .into_iter()
        .filter_map(|(p, id)| id.as_ref().map(|id| (p.to_string(), id.clone())))
        .collect()
    }
}

/// Interface number -> game version: "16001" -> "1.60.1", "120105" -> "12.1.5".
pub fn game_version(interface: &str) -> Option<String> {
    let s = interface.trim();
    if s.len() < 5 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n = s.len();
    let num = |a: &str| a.parse::<u32>().ok();
    Some(format!(
        "{}.{}.{}",
        num(&s[..n - 4])?,
        num(&s[n - 4..n - 2])?,
        num(&s[n - 2..])?
    ))
}

pub fn parse_toc(file_name: &str, text: &str) -> Toc {
    let mut toc = Toc {
        file_name: file_name.to_string(),
        ..Toc::default()
    };
    let list = |v: &str| -> Vec<String> {
        v.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    let set = |slot: &mut Option<String>, v: &str| {
        if slot.is_none() && !v.is_empty() {
            *slot = Some(v.to_string());
        }
    };
    for raw in text.trim_start_matches('\u{feff}').lines() {
        let line = raw.trim();
        if let Some(meta) = line.strip_prefix("##") {
            let Some((key, value)) = meta.split_once(':') else {
                continue;
            };
            let key = key.trim();
            let value = clean_value(value);
            match key.to_ascii_lowercase().as_str() {
                "title" => set(&mut toc.title, &value),
                "version" => set(&mut toc.version, &value),
                "author" => set(&mut toc.author, &value),
                "notes" => set(&mut toc.notes, &value),
                "interface" if toc.interface.is_empty() => toc.interface = list(&value),
                "x-curse-project-id" => set(&mut toc.curse_project_id, &value),
                "x-wowi-id" => set(&mut toc.wowi_id, &value),
                "x-wago-id" => set(&mut toc.wago_id, &value),
                k if k == "requireddeps" || k.starts_with("dep") => {
                    for d in list(&value) {
                        if !toc.dependencies.contains(&d) {
                            toc.dependencies.push(d);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    toc
}

/// Strips load conditions (`[AllowLoad...]`), colour codes (`|cAARRGGBB`, `|r`) and textures (`|T...|t`).
fn clean_value(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut rest = v;
    while let Some(i) = rest.find(['|', '[']) {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let skip = if tail.starts_with("[AllowLoad") || tail.starts_with("[ExcludeLoad") {
            tail.find(']').map_or(tail.len(), |j| j + 1)
        } else if tail.starts_with("|c")
            && tail.len() >= 10
            && tail.as_bytes()[2..10].iter().all(u8::is_ascii_hexdigit)
        {
            10
        } else if tail.starts_with("|r") {
            2
        } else if tail.starts_with("|T") {
            tail.find("|t").map_or(tail.len(), |j| j + 2)
        } else {
            out.push_str(&tail[..1]);
            1
        };
        rest = &tail[skip..];
    }
    out.push_str(rest);
    out.trim().to_string()
}

pub(crate) const FLAVOR_SUFFIXES: [&str; 11] = [
    "classic", "vanilla", "bcc", "tbc", "mainline", "wrath", "wotlkc", "cata", "mists", "forever",
    "camelot",
];

/// Flavor suffix of a TOC file name: "Details_Camelot.toc" -> Some("camelot").
fn toc_suffix(name: &str) -> Option<&'static str> {
    let stem = name.get(..name.len().checked_sub(4)?)?.to_ascii_lowercase();
    FLAVOR_SUFFIXES.into_iter().find(|s| {
        stem.len() > s.len() + 1
            && stem.ends_with(s)
            && matches!(stem.as_bytes()[stem.len() - s.len() - 1], b'-' | b'_')
    })
}

/// The Forever TOC: `*[-_]Forever.toc` / `*[-_]Camelot.toc`, then a TOC without a flavor
/// suffix (preferring `<folder>.toc`).
pub fn select_toc<'a>(folder: &str, toc_names: &'a [String]) -> Option<&'a str> {
    let mut names: Vec<&str> = toc_names.iter().map(String::as_str).collect();
    names.sort_unstable();
    let by_suffix = |s: &str| names.iter().find(|n| toc_suffix(n) == Some(s)).copied();
    by_suffix("forever")
        .or_else(|| by_suffix("camelot"))
        .or_else(|| {
            let folder_toc = format!("{folder}.toc");
            names
                .iter()
                .find(|n| n.eq_ignore_ascii_case(&folder_toc))
                .copied()
        })
        .or_else(|| names.iter().find(|n| toc_suffix(n).is_none()).copied())
}

/// Names of the `*.toc` files directly inside `folder`.
pub fn toc_names(folder: &Path) -> Result<Vec<String>> {
    let rd = std::fs::read_dir(folder).with_context(|| format!("reading {}", folder.display()))?;
    Ok(rd
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        // Byte-wise: slicing a name like "Über" at len-4 would split a char and panic.
        .filter(|n| n.len() > 4 && n.as_bytes()[n.len() - 4..].eq_ignore_ascii_case(b".toc"))
        .collect())
}

pub fn read_toc(path: &Path) -> Result<Toc> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(parse_toc(&name, &String::from_utf8_lossy(&bytes)))
}

/// The Forever TOC of an addon folder, or None if the folder has no usable TOC.
pub fn read_folder_toc(folder: &Path) -> Result<Option<Toc>> {
    let names = toc_names(folder)?;
    let folder_name = folder
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    match select_toc(&folder_name, &names) {
        Some(n) => read_toc(&folder.join(n)).map(Some),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_crlf_bom_colors_and_lists() {
        let t = parse_toc(
            "X.toc",
            "\u{feff}## Interface: 11509, 20506, 16001\r\n## Title: Questie|cFF00FF00 v12.9.0|r\r\n\
             ## RequiredDeps: Details\r\n## Dependencies: Ace3, Details\r\n## LoadOnDemand: 1\r\n\
             # comment\r\nCore\\Init.lua\r\n\r\n",
        );
        assert_eq!(t.title.as_deref(), Some("Questie v12.9.0"));
        assert_eq!(t.game_versions(), ["1.15.9", "2.5.6", "1.60.1"]);
        assert_eq!(t.forever_game_version().as_deref(), Some("1.60.1"));
        assert_eq!(t.dependencies, ["Details", "Ace3"]);
    }

    #[test]
    fn selection_order() {
        let n = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let names = n(&["Q.toc", "Q_Camelot.toc", "Q_Forever.toc", "Q_Mists.toc"]);
        assert_eq!(select_toc("Q", &names), Some("Q_Forever.toc"));
        let names = n(&["Q.toc", "Q-Camelot.toc"]);
        assert_eq!(select_toc("Q", &names), Some("Q-Camelot.toc"));
        let names = n(&["Q_Mainline.toc", "Q.toc"]);
        assert_eq!(select_toc("Q", &names), Some("Q.toc"));
        assert_eq!(select_toc("Q", &n(&["Q_Mainline.toc"])), None);
    }

    #[test]
    fn toc_names_survive_non_ascii_file_names() {
        let dir = tempfile::tempdir().unwrap();
        for n in ["Über", "€ab", "A.toc", "B.TOC", "x.toc.bak"] {
            std::fs::write(dir.path().join(n), "").unwrap();
        }
        let mut names = toc_names(dir.path()).unwrap();
        names.sort();
        assert_eq!(names, ["A.toc", "B.TOC"]);
    }

    #[test]
    fn interface_versions() {
        assert_eq!(game_version("120105").as_deref(), Some("12.1.5"));
        assert_eq!(game_version("abc"), None);
    }
}
