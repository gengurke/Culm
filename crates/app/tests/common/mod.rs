//! End-to-end harness: the real UI (`addon_app::runtime`) over the real core backend, against a
//! temp World of Warcraft: Forever tree, temp app folders, an optional fake WowUp config and a
//! local FakeServer that speaks CurseForge, WowUp Hub (under `/hub`) and GitHub. Nothing leaves the temp dir or 127.0.0.1.
#![allow(dead_code)]

use addon_app::core_backend::{Config, CoreBackend};
use addon_app::state::{Op, Status};
use addon_app::{Job, State, runtime};
use addon_core::fingerprint::folder_fingerprint;
use addon_core::providers::ProviderConfig;
use addon_testkit::{
    FakeServer, cf_envelope, cf_file, cf_mod_with_files, fixture, fixture_at, hub_addon, make_zip,
};
use rune::prefs::Prefs;
use rune::{Key, KeyEvent, Mods, Runtime};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const BUILD_INFO: &str = "Branch!STRING:0|Active!DEC:1|Version!STRING:0|Product!STRING:0\n\
eu|1|11.2.7.64000|wow\neu|1|1.60.1.70938|wowf\n";

/// A tiny SVG served as every addon's thumbnail.
pub const THUMB_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8" fill="#c90"/></svg>"##;

pub fn write(path: &Path, content: impl AsRef<[u8]>) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

/// `Foo_Options` requires `Foo`, so unmatched it still groups under `Foo` (F2.2).
fn toc(title: &str, version: &str) -> String {
    let deps = title
        .split_once('_')
        .map(|(parent, _)| format!("## RequiredDeps: {parent}\n"))
        .unwrap_or_default();
    format!("## Interface: 16001\n## Title: {title}\n## Version: {version}\n{deps}\nCore.lua\n")
}

/// Points the CDN URLs of provider JSON at the fake server.
fn local(v: Value, base: &str) -> Value {
    let s = v
        .to_string()
        .replace("https://edge.forgecdn.net", base)
        .replace("https://media.forgecdn.net", &format!("{base}/media"));
    serde_json::from_str(&s).unwrap()
}

/// The disk and the network the app sees.
///
/// On disk: Ace 1.0 (CurseForge 1, 1.1 out), Bee + Bee_Options 2.0 (CurseForge 2, 2.1 out) and
/// Homebrew (no provider knows it). CurseForge also offers Cee (3) in search and the popular list.
/// WowUp Hub offers Cee too (7, same repo `cee-dev/Cee`, release `v70` = Cee 3.0-hub) and Dee (8),
/// in search and its featured list. GitHub has `Questie/Questie` (stable v12.9.0).
pub struct World {
    pub tmp: tempfile::TempDir,
    pub server: FakeServer,
    /// `.../World of Warcraft` (holds `.build.info`).
    pub wow_root: PathBuf,
    /// `.../_forever_`.
    pub client: PathBuf,
    pub search_roots: Vec<PathBuf>,
    pub wowup_dirs: Vec<PathBuf>,
}

impl World {
    pub fn new() -> World {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = tmp.path().join("testprefix");
        let wow_root = prefix.join("drive_c/Program Files (x86)/World of Warcraft");
        let client = wow_root.join("_forever_");
        write(&wow_root.join(".build.info"), BUILD_INFO);
        let w = World {
            server: FakeServer::start(),
            wow_root,
            client,
            search_roots: vec![prefix],
            wowup_dirs: vec![],
            tmp,
        };
        w.folder("Ace", "1.0");
        w.folder("Bee", "2.0");
        w.folder("Bee_Options", "2.0");
        w.folder("Homebrew", "0.1");
        w.serve();
        w
    }

    pub fn addons_dir(&self) -> PathBuf {
        self.client.join("Interface/AddOns")
    }

    fn folder(&self, name: &str, version: &str) {
        let dir = self.addons_dir().join(name);
        write(&dir.join(format!("{name}.toc")), toc(name, version));
        write(&dir.join("Core.lua"), format!("-- {name} {version}\n"));
    }

    /// A file below the client folder, e.g. `Interface/AddOns/Ace/Ace.toc`.
    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.client.join(rel)).unwrap_or_default()
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.client.join(rel).exists()
    }

    fn fp(&self, folder: &str) -> u32 {
        folder_fingerprint(&self.addons_dir().join(folder)).unwrap()
    }

    fn serve(&self) {
        let s = &self.server;
        let base = s.url();
        let ace = cf_mod_with_files(
            1,
            "Ace",
            vec![
                cf_file(11, "1.1", 1, &["Ace"], 0),
                cf_file(10, "1.0", 1, &["Ace"], 0),
            ],
        );
        let bee_files = |id, v| cf_file(id, v, 1, &["Bee", "Bee_Options"], 0);
        let bee = cf_mod_with_files(2, "Bee", vec![bee_files(21, "2.1"), bee_files(20, "2.0")]);
        let mut cee = cf_mod_with_files(3, "Cee", vec![cf_file(30, "3.0", 1, &["Cee"], 0)]);
        cee["links"]["sourceUrl"] = json!("https://github.com/cee-dev/Cee");
        s.json(
            "POST",
            "/v1/mods",
            &local(cf_envelope(json!([ace, bee, cee.clone()])), base),
        );
        s.json(
            "GET",
            "/v1/mods/search",
            &local(cf_envelope(json!([cee.clone()])), base),
        );
        s.json(
            "POST",
            "/v1/mods/featured",
            &local(
                cf_envelope(json!({ "popular": [cee], "recentlyUpdated": [] })),
                base,
            ),
        );

        // The installed files, with module fingerprints of the folders on disk.
        let mut ace10 = cf_file(10, "1.0", 1, &["Ace"], 0);
        ace10["modules"] = json!([{ "name": "Ace", "fingerprint": self.fp("Ace") }]);
        let mut bee20 = bee_files(20, "2.0");
        bee20["modules"] = json!([
            { "name": "Bee", "fingerprint": self.fp("Bee") },
            { "name": "Bee_Options", "fingerprint": self.fp("Bee_Options") }
        ]);
        let matches = json!({ "exactMatches": [{ "id": 1, "file": ace10 }, { "id": 2, "file": bee20 }], "partialMatches": [] });
        s.json(
            "POST",
            "/v1/fingerprints/1",
            &local(cf_envelope(matches), base),
        );

        let zip = |folder: &str, version: &str, extra: &str| {
            let toc = toc(folder, version);
            make_zip(&[
                (&format!("{folder}/{folder}.toc"), toc.as_bytes()),
                (&format!("{folder}/{extra}"), version.as_bytes()),
            ])
        };
        s.bytes("GET", "/files/0/11/1.1.zip", zip("Ace", "1.1", "New.lua"));
        s.bytes(
            "GET",
            "/files/0/21/2.1.zip",
            make_zip(&[
                ("Bee/Bee.toc", toc("Bee", "2.1").as_bytes()),
                (
                    "Bee_Options/Bee_Options.toc",
                    toc("Bee_Options", "2.1").as_bytes(),
                ),
            ]),
        );
        s.bytes("GET", "/files/0/30/3.0.zip", zip("Cee", "3.0", "Core.lua"));

        // WowUp Hub: Cee (same repo as CurseForge's Cee) and Dee, nothing matched on disk.
        let hub_entry = |id, repo, folder: &str, asset| {
            let url = format!("{base}/hubdl/{folder}-v{asset}.zip");
            let mut a = hub_addon(id, repo, &[folder], asset, &url);
            // Avatars from the fake server too: nothing leaves 127.0.0.1.
            a["owner_image_url"] = json!(format!("{base}/media/hub/{id}.png"));
            a
        };
        let hub = json!([
            hub_entry(7, "cee-dev/Cee", "Cee", 70),
            hub_entry(8, "dee-dev/Dee", "Dee", 80),
        ]);
        s.json(
            "GET",
            "/hub/addons/search/forever",
            &json!({ "addons": hub }),
        );
        s.json(
            "GET",
            "/hub/addons/featured/forever",
            &json!({ "addons": hub, "recent": [] }),
        );
        s.json(
            "POST",
            "/hub/addons/fingerprint",
            &json!({ "exactMatches": [] }),
        );
        s.json(
            "POST",
            "/hub/addons/batch/forever",
            &json!({ "addons": [] }),
        );
        s.bytes(
            "GET",
            "/hubdl/Cee-v70.zip",
            zip("Cee", "3.0-hub", "Core.lua"),
        );
        s.respond(
            "GET",
            "/repos/cee-dev/Cee/readme",
            200,
            "text/plain",
            "# Cee\n\nCee, straight from GitHub.\n",
        );
        s.respond("GET", "/media/*", 200, "image/svg+xml", THUMB_SVG);
        let cat = |id: u32, name: &str, slug: &str, parent: u32| json!({ "id": id, "gameId": 1, "name": name, "slug": slug, "classId": 1, "parentCategoryId": parent });
        s.json(
            "GET",
            "/v1/categories",
            &cf_envelope(json!([
                { "id": 1, "gameId": 1, "name": "Addons", "slug": "addons", "isClass": true },
                cat(1009, "Bags & Inventory", "bags-inventory", 1),
                cat(1020, "Class", "class", 1),
                cat(1028, "Warrior", "warrior", 1020),
                cat(1036, "Death Knight", "death-knight", 1020),
            ])),
        );
        s.json(
            "GET",
            "/v1/mods/3/description",
            &cf_envelope(json!(
                "<p>Cee keeps your <b>bags</b> tidy.</p><ul><li>Sorts</li><li>Stacks</li></ul>"
            )),
        );
        s.json(
            "GET",
            "/repos/Questie/Questie",
            &fixture("github_repo.json"),
        );
        s.json(
            "GET",
            "/repos/Questie/Questie/releases",
            &fixture_at("github_releases.json", base),
        );
        s.respond(
            "GET",
            "/repos/Questie/Questie/readme",
            200,
            "text/plain",
            "# Questie\n\nThe quest helper for **Forever**.\n",
        );
        s.json(
            "GET",
            "/dl/82/release.json",
            &fixture("github_release.json"),
        );
        s.bytes(
            "GET",
            "/dl/80/Questie-v12.9.0.zip",
            zip("Questie", "12.9.0", "Core.lua"),
        );
    }

    /// WowUp (CurseForge edition) config that knows Ace for this client: installed 1.0, 1.1 out.
    pub fn with_wowup(mut self) -> World {
        let dir = self.tmp.path().join("WowUpCf");
        let prefs = json!({ "wow_installations": [{ "id": "fev", "location": format!("{}/Wow.exe", self.client.display()) }] });
        let ace = json!({
            "id": "wowup-ace", "name": "Ace", "author": "Author", "providerName": "Curse", "externalId": "1",
            "installedVersion": "1.0", "installedExternalReleaseId": "10", "installedFolderList": ["Ace"],
            "installationId": "fev", "channelType": 0, "isIgnored": false, "latestVersion": "1.1",
            "externalLatestReleaseId": "11", "externalChannel": "Stable",
            "downloadUrl": format!("{}/files/0/11/1.1.zip", self.server.url())
        });
        write(&dir.join("preferences.json"), prefs.to_string());
        write(
            &dir.join("addons.json"),
            json!({ "wowup-ace": ace }).to_string(),
        );
        self.wowup_dirs = vec![dir];
        self
    }

    pub fn prefs_path(&self) -> PathBuf {
        self.tmp.path().join("config/prefs.json")
    }

    /// Stores a CurseForge key as if the user had saved it earlier.
    pub fn with_key(self, key: &str) -> World {
        Prefs::at(self.prefs_path())
            .set("curseforge_key", key)
            .unwrap();
        self
    }

    pub fn config(&self) -> Config {
        let url = self.server.url().to_string();
        Config {
            data_dir: self.tmp.path().join("data"),
            cache_dir: self.tmp.path().join("cache"),
            prefs: Prefs::at(self.prefs_path()),
            search_roots: self.search_roots.clone(),
            wowup_dirs: self.wowup_dirs.clone(),
            providers: ProviderConfig {
                curseforge_base_url: url.clone(),
                github_base_url: url.clone(),
                hub_base_url: format!("{url}/hub"),
                wowinterface_base_url: url,
                ..Default::default()
            },
        }
    }

    /// Starts the app (a new process, as far as the app can tell) and lets it boot.
    pub fn app(&self) -> App {
        self.app_with(self.config())
    }

    pub fn app_with(&self, config: Config) -> App {
        let state = State::new(Arc::new(CoreBackend::new(config)));
        let mut rt = runtime(state, (1280.0, 800.0));
        rt.frame(0.0);
        App { rt }
    }
}

/// The running UI, driven like a user would.
pub struct App {
    pub rt: Runtime<State>,
}

const TIMEOUT: Duration = Duration::from_secs(15);

impl App {
    pub fn state(&self) -> &State {
        &self.rt.state
    }

    pub fn frame(&mut self) {
        self.rt.frame(1.0 / 60.0);
    }

    pub fn tap(&mut self, key: &str) {
        let (x, y) = self
            .rt
            .find(key)
            .unwrap_or_else(|| panic!("nothing keyed {key} on screen"));
        self.rt.pointer_down(x, y);
        self.rt.pointer_up(x, y);
        self.frame();
    }

    pub fn type_text(&mut self, t: &str) {
        self.rt.key(KeyEvent {
            key: Key::Text(t.into()),
            mods: Mods::default(),
        });
        self.frame();
    }

    pub fn press(&mut self, key: Key) {
        self.rt.key(KeyEvent {
            key,
            mods: Mods::default(),
        });
        self.frame();
    }

    pub fn has(&self, key: &str) -> bool {
        self.rt.find(key).is_some()
    }

    /// Visible text anywhere on screen contains `needle`.
    pub fn shows(&self, needle: &str) -> bool {
        self.rt
            .elements()
            .iter()
            .any(|e| !e.exiting && e.text.as_deref().is_some_and(|t| t.contains(needle)))
    }

    /// Visible text inside the element keyed `key` contains `needle` (by layout target, so a row
    /// that is still animating in counts).
    pub fn shows_in(&self, key: &str, needle: &str) -> bool {
        let Some(rect) = self
            .rt
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key) && !e.exiting)
            .map(|e| e.target)
        else {
            return false;
        };
        let inside = |r: [f32; 4]| {
            r[0] >= rect[0]
                && r[1] >= rect[1]
                && r[0] + r[2] <= rect[0] + rect[2] + 0.5
                && r[1] + r[3] <= rect[1] + rect[3] + 0.5
        };
        self.rt.elements().iter().any(|e| {
            !e.exiting && inside(e.target) && e.text.as_deref().is_some_and(|t| t.contains(needle))
        })
    }

    /// Steps frames (and real time, for the workers) until `done` holds.
    pub fn wait_until(&mut self, what: &str, done: impl Fn(&App) -> bool) {
        let start = Instant::now();
        while !done(self) {
            assert!(start.elapsed() < TIMEOUT, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(5));
            self.frame();
        }
        self.frame();
    }

    pub fn wait_for(&mut self, what: &str, done: impl Fn(&State) -> bool) {
        self.wait_until(what, |app| done(app.state()));
    }

    pub fn wait_for_text(&mut self, needle: &str) {
        self.wait_until(&format!("text {needle:?}"), |app| app.shows(needle));
    }

    /// No background job and no addon busy.
    pub fn wait_idle(&mut self) {
        self.wait_for("the app to settle", |s| {
            s.job == Job::Idle
                && s.link != addon_app::state::Fetch::Loading
                && s.installing.is_empty()
                && !s.addons.iter().any(|a| a.op.as_ref().is_some_and(Op::busy))
        });
    }

    /// Local id of the addon named `name`.
    pub fn id(&self, name: &str) -> String {
        self.state()
            .addons
            .iter()
            .find(|a| a.addon.name == name)
            .unwrap_or_else(|| panic!("no addon {name}"))
            .addon
            .id
            .clone()
    }

    pub fn status(&self, name: &str) -> Status {
        self.state()
            .addons
            .iter()
            .find(|a| a.addon.name == name)
            .unwrap_or_else(|| panic!("no addon {name}"))
            .status()
    }

    /// Key of the addon's row, or of a part of it (`row("Ace", "update")`).
    pub fn row(&self, name: &str, part: &str) -> String {
        let id = self.id(name);
        if part.is_empty() {
            format!("addon/{id}")
        } else {
            format!("addon/{id}/{part}")
        }
    }

    pub fn names(&self) -> Vec<String> {
        self.state()
            .addons
            .iter()
            .map(|a| a.addon.name.clone())
            .collect()
    }
}
