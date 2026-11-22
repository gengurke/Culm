//! Shared core fixtures for the feature tests: fake WoW trees, a fake provider, remote files and addons.
#![allow(dead_code)]

use addon_core::providers::{FolderMatch, Provider, ScannedFolder};
use addon_core::{Channel, Flavor, Installation, RemoteAddon, RemoteFile};
use addon_testkit::{BUILD_INFO_REAL, write};
use std::path::{Path, PathBuf};

pub fn fixture_toc(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tocs")
            .join(name),
    )
    .unwrap()
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
    write(&wow_root.join(".build.info"), BUILD_INFO_REAL);
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
        check: None,
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
        source_url: None,
        categories: vec![],
        updated_at: None,
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
