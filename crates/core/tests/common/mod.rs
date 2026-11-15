//! Shared fixtures for the feature tests: fake WoW trees, zips, a fake provider.
#![allow(dead_code)]

use addon_core::providers::{FolderMatch, Provider, ScannedFolder};
use addon_core::{Channel, Flavor, Installation, RemoteAddon, RemoteFile};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const BUILD_INFO: &str = "Branch!STRING:0|Active!DEC:1|Build Key!HEX:16|CDN Key!HEX:16|Install Key!HEX:16|IM Size!DEC:4|CDN Path!STRING:0|CDN Hosts!STRING:0|CDN Servers!STRING:0|Tags!STRING:0|Armadillo!STRING:0|Last Activated!STRING:0|Version!STRING:0|KeyRing!HEX:16|Product!STRING:0\n\
eu|1|3627b099867bfde3e37a17cf44f69cdc|3363ac656da5f261cdc675ab28b2744c|||tpr/wow|level3.blizzard.com|http://level3.blizzard.com/?maxhosts=8|Windows x86_64 EU? enUS speech?:Windows x86_64 EU? enUS text?|||1.60.1.70938||wowf\n";

pub fn fixture_toc(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tocs")
            .join(name),
    )
    .unwrap()
}

pub fn write(path: &Path, content: impl AsRef<[u8]>) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

/// `<tmp>/<prefix>/drive_c/Program Files (x86)/World of Warcraft/_forever_` with `.build.info`.
pub struct FakeWow {
    pub tmp: tempfile::TempDir,
    pub wow_root: PathBuf,
    pub inst: Installation,
}

pub fn fake_wow() -> FakeWow {
    let tmp = tempfile::tempdir().unwrap();
    let wow_root = tmp
        .path()
        .join("battlenet/drive_c/Program Files (x86)/World of Warcraft");
    write(&wow_root.join(".build.info"), BUILD_INFO);
    std::fs::create_dir_all(wow_root.join("_forever_/Interface/AddOns")).unwrap();
    let inst = addon_core::wow::installation_from_path(&wow_root).unwrap();
    FakeWow {
        tmp,
        wow_root,
        inst,
    }
}

impl FakeWow {
    pub fn addons(&self) -> PathBuf {
        self.inst.addons_dir()
    }
    /// An addon folder with one TOC (`toc_name`) and some files.
    pub fn add_folder(&self, folder: &str, toc_name: &str, toc: &str, files: &[(&str, &str)]) {
        let dir = self.addons().join(folder);
        write(&dir.join(toc_name), toc);
        for (f, c) in files {
            write(&dir.join(f), c);
        }
    }
    pub fn folder_names(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(self.addons())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }
}

pub fn simple_toc(title: &str, version: &str) -> String {
    format!("## Interface: 16001\r\n## Title: {title}\r\n## Version: {version}\r\n\r\nCore.lua\r\n")
}

pub fn zip_bytes(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default();
    for (name, content) in entries {
        w.start_file(*name, opts).unwrap();
        w.write_all(content.as_bytes()).unwrap();
    }
    w.finish().unwrap().into_inner()
}

pub fn remote_file(id: &str, version: &str, channel: Channel, folders: &[&str]) -> RemoteFile {
    RemoteFile {
        id: id.into(),
        version: version.into(),
        channel,
        download_url: format!("https://example.invalid/{id}.zip"),
        released_at: Some(format!("2026-01-{:02}T00:00:00Z", id.len() % 28 + 1)),
        game_versions: vec!["1.60.1".into()],
        folders: folders.iter().map(|s| s.to_string()).collect(),
        dependencies: vec![],
    }
}

pub fn remote_addon(id: &str, name: &str, files: Vec<RemoteFile>) -> RemoteAddon {
    RemoteAddon {
        provider: "Fake".into(),
        external_id: id.into(),
        name: name.into(),
        author: "Someone".into(),
        summary: format!("{name} summary"),
        url: None,
        thumbnail_url: None,
        download_count: 1,
        files,
    }
}

/// Matches folders by name: (folder, addon, installed file).
pub struct FakeProvider {
    pub matches: Vec<(String, RemoteAddon, Option<RemoteFile>)>,
    pub fail: bool,
}

impl Provider for FakeProvider {
    fn name(&self) -> &'static str {
        "Fake"
    }
    fn search(&self, _: Flavor, _: &str) -> anyhow::Result<Vec<RemoteAddon>> {
        Ok(vec![])
    }
    fn get_many(&self, _: Flavor, _: &[String]) -> anyhow::Result<Vec<RemoteAddon>> {
        Ok(vec![])
    }
    fn match_folders(
        &self,
        _: Flavor,
        folders: &[ScannedFolder],
    ) -> anyhow::Result<Vec<FolderMatch>> {
        if self.fail {
            anyhow::bail!("offline");
        }
        Ok(self
            .matches
            .iter()
            .filter(|(f, ..)| folders.iter().any(|s| &s.name == f))
            .map(|(f, a, file)| FolderMatch {
                folder: f.clone(),
                addon: a.clone(),
                installed_file: file.clone(),
            })
            .collect())
    }
}
