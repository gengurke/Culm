//! WowUp Hub provider (F4.7) and multi-source search with strict dedupe (F4.8), against a fake server.

use addon_core::providers::{CurseForge, Hub, Provider, ScannedFolder};
use addon_core::updates::check_updates;
use addon_core::{Channel, Flavor, discover, http, install};
use addon_testkit::{FakeServer, cf_envelope, cf_mod, fixture, fixture_at, hub_addon, make_zip};
use serde_json::{Value, json};

mod common;

const F: Flavor = Flavor::Forever;

fn hub(s: &FakeServer) -> Hub {
    Hub {
        base_url: format!("{}/hub", s.url()),
    }
}

fn curseforge(s: &FakeServer) -> CurseForge {
    CurseForge {
        api_key: Some("test-key".into()),
        base_url: s.url().into(),
    }
}

fn download(u: &str) -> anyhow::Result<Box<dyn install::Zip>> {
    http::download(u, &mut |_, _| {}).map(install::in_memory)
}

#[test]
fn f4_7_hub_search_keeps_forever_releases() {
    let s = FakeServer::start();
    s.json(
        "GET",
        "/hub/addons/search/forever",
        &fixture("hub_search.json"),
    );
    let mut retail = hub_addon(9, "me/RetailOnly", &["RetailOnly"], 1, "https://x/r.zip");
    retail["releases"][0]["game_versions"][0]["game_type"] = json!("retail");
    let mut list = fixture("hub_search.json");
    list["addons"].as_array_mut().unwrap().push(retail);
    s.json("GET", "/hub/addons/search/forever", &list);

    let results = hub(&s).search(F, "quest").unwrap();

    let req = &s.requests_to("/hub/addons/search/forever")[0];
    assert_eq!(req.query_param("query").as_deref(), Some("quest"));
    assert!(
        req.header("authorization").is_none(),
        "the Hub needs no key"
    );
    assert!(req.header("user-agent").unwrap().contains("addon manager"));
    let names: Vec<&str> = results.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        names,
        ["Questie", "XenQuest"],
        "the retail-only addon is dropped"
    );
    let q = &results[0];
    assert_eq!(
        (
            q.provider.as_str(),
            q.external_id.as_str(),
            q.author.as_str()
        ),
        ("WowUpHub", "47671", "Questie")
    );
    assert_eq!(q.url.as_deref(), Some("https://wowup.io/addons/47671"));
    assert_eq!(
        q.source_url.as_deref(),
        Some("https://github.com/Questie/Questie")
    );
    assert!(q.summary.starts_with("Questie"), "{}", q.summary);
    assert!(!q.summary.contains('<'));
    let stable = q.latest_for(Channel::Stable).unwrap();
    assert_eq!(
        (stable.id.as_str(), stable.version.as_str()),
        ("745727013", "bundle/v12.9.0+v1.1.1"),
        "file id is the GitHub asset id, version the tag"
    );
    assert_eq!(stable.folders, ["Questie", "QuestieDB", "QuestieTrace"]);
    assert_eq!(stable.game_versions[0], "1.60.1");
    let x = &results[1];
    assert_eq!(x.latest_for(Channel::Beta).unwrap().version, "v1.0.4");
    assert_eq!(
        x.files
            .iter()
            .find(|f| f.channel == Channel::Beta)
            .unwrap()
            .version,
        "v0.2.14-beta"
    );
}

#[test]
fn f4_7_hub_featured_dedupes_popular_and_recent() {
    let s = FakeServer::start();
    s.json(
        "GET",
        "/hub/addons/featured/forever",
        &fixture("hub_featured.json"),
    );

    let results = hub(&s).featured(F).unwrap();

    let req = &s.requests_to("/hub/addons/featured/forever")[0];
    assert_eq!(req.query_param("count").as_deref(), Some("60"));
    let ids: Vec<&str> = results.iter().map(|a| a.external_id.as_str()).collect();
    assert_eq!(ids, ["1158475", "47671", "3", "2797979"]);
}

#[test]
fn f4_7_hub_install_and_update() {
    let s = FakeServer::start();
    let w = common::fake_wow();
    let v1 = make_zip(&[
        ("Foo/Foo.toc", b"## Interface: 16001\n## Version: 1\n"),
        ("Foo/a.lua", b"1"),
    ]);
    let v2 = make_zip(&[
        ("Foo/Foo.toc", b"## Interface: 16001\n## Version: 2\n"),
        ("Foo/a.lua", b"2"),
    ]);
    s.bytes("GET", "/dl/1/Foo.zip", v1);
    s.bytes("GET", "/dl/2/Foo.zip", v2);
    let one = hub_addon(
        5,
        "me/Foo",
        &["Foo"],
        1,
        &format!("{}/dl/1/Foo.zip", s.url()),
    );
    s.json(
        "GET",
        "/hub/addons/search/forever",
        &json!({ "addons": [one], "count": 1 }),
    );
    let h = hub(&s);

    let remote = h.search(F, "foo").unwrap().remove(0);
    let mut addon = install::install_remote(&w.inst, &remote, Channel::Stable, &download).unwrap();
    assert_eq!(addon.provider.as_deref(), Some("WowUpHub"));
    assert_eq!(addon.folders, ["Foo"]);
    assert_eq!(addon.installed_file_id.as_deref(), Some("1"));
    assert_eq!(std::fs::read(w.addons().join("Foo/a.lua")).unwrap(), b"1");

    let two = hub_addon(
        5,
        "me/Foo",
        &["Foo"],
        2,
        &format!("{}/dl/2/Foo.zip", s.url()),
    );
    s.json(
        "POST",
        "/hub/addons/batch/forever",
        &json!({ "addons": [two], "count": 1 }),
    );
    let providers: Vec<Box<dyn Provider>> = vec![Box::new(hub(&s))];
    let mut list = vec![addon.clone()];
    assert!(check_updates(F, &mut list, &providers).is_empty());
    assert_eq!(
        s.requests_to("/hub/addons/batch/forever")[0].json(),
        json!({ "addonIds": [5] })
    );
    assert!(list[0].update_available());
    addon = install::update_addon(&w.inst, &list[0], &download).unwrap();
    assert_eq!(addon.installed_file_id.as_deref(), Some("2"));
    assert_eq!(std::fs::read(w.addons().join("Foo/a.lua")).unwrap(), b"2");
}

#[test]
fn f4_7_hub_fingerprint_matches_folders() {
    let s = FakeServer::start();
    s.json(
        "POST",
        "/hub/addons/fingerprint",
        &fixture("hub_fingerprint.json"),
    );
    s.json(
        "POST",
        "/hub/addons/batch/forever",
        &fixture_at("hub_batch.json", s.url()),
    );
    let folder = |name: &str, md5: &str| ScannedFolder {
        name: name.into(),
        fingerprint: 1,
        hub_fingerprint: md5.into(),
        toc_ids: vec![],
    };
    let folders = [
        folder("XenQuest", "a204bf9bcf6ab09267db847f53385a2b"),
        folder("Mine", "ffffffffffffffffffffffffffffffff"),
        folder("NoHash", ""),
    ];

    let matches = hub(&s).match_folders(F, &folders).unwrap();

    assert_eq!(
        s.requests_to("/hub/addons/fingerprint")[0].json(),
        json!({ "fingerprints": ["a204bf9bcf6ab09267db847f53385a2b", "ffffffffffffffffffffffffffffffff"] })
    );
    assert_eq!(
        s.requests_to("/hub/addons/batch/forever")[0].json(),
        json!({ "addonIds": [2797963] })
    );
    assert_eq!(matches.len(), 1);
    let m = &matches[0];
    assert_eq!(
        (m.folder.as_str(), m.addon.name.as_str()),
        ("XenQuest", "XenQuest")
    );
    assert_eq!(m.installed_file.as_ref().unwrap().id, "713233517");
    assert_eq!(
        m.addon.latest_for(Channel::Stable).unwrap().version,
        "v1.0.4"
    );
}

#[test]
fn f4_7_hub_url_resolves_id() {
    let s = FakeServer::start();
    let addon = hub_addon(7, "me/Bar", &["Bar"], 3, "https://example.invalid/b.zip");
    s.json("GET", "/hub/addons/7", &json!({ "addon": addon }));
    let h = hub(&s);
    let got = h.from_url(F, "https://wowup.io/addons/7").unwrap().unwrap();
    assert_eq!(got.external_id, "7");
    assert!(
        h.from_url(F, "https://github.com/me/Bar")
            .unwrap()
            .is_none()
    );
    // Unsafe download URLs from the API are not offered.
    let mut bad = hub_addon(8, "me/Bad", &["Bad"], 4, "file:///etc/passwd");
    bad["id"] = json!(8);
    s.json("GET", "/hub/addons/8", &json!({ "addon": bad }));
    assert!(h.from_url(F, "https://wowup.io/addons/8").is_err());
}

// --- F4.8 multi-source search ---

fn cf_with_source(id: u64, name: &str, source: Option<&str>) -> Value {
    let mut m = cf_mod(id, name, &[name], id * 10, "1.0");
    if let Some(src) = source {
        m["links"]["sourceUrl"] = json!(src);
    }
    m
}

/// CurseForge and Hub on one fake server; `providers` lists Hub first to show CurseForge still wins.
fn sources(cf: Vec<Value>, hub_addons: Vec<Value>) -> (FakeServer, Vec<Box<dyn Provider>>) {
    let s = FakeServer::start();
    s.json("GET", "/v1/mods/search", &cf_envelope(json!(cf)));
    s.json(
        "GET",
        "/hub/addons/search/forever",
        &json!({ "addons": hub_addons, "count": 0 }),
    );
    let providers: Vec<Box<dyn Provider>> = vec![Box::new(hub(&s)), Box::new(curseforge(&s))];
    (s, providers)
}

fn hub_repo(id: u64, repo: &str) -> Value {
    let name = repo.split('/').nth(1).unwrap();
    hub_addon(id, repo, &[name], id, "https://example.invalid/x.zip")
}

fn shape(d: &discover::Discovery) -> Vec<Vec<String>> {
    d.entries
        .iter()
        .map(|e| {
            e.sources
                .iter()
                .map(|a| format!("{}:{}", a.provider, a.external_id))
                .collect()
        })
        .collect()
}

#[test]
fn f4_8_merges_curseforge_and_hub_on_exact_github_repo() {
    let (_s, providers) = sources(
        vec![cf_with_source(
            100,
            "Questie",
            Some("https://github.com/Questie/Questie.git"),
        )],
        vec![hub_repo(47671, "questie/questie"), hub_repo(2, "me/Other")],
    );

    let d = discover::search(F, &providers, "questie");

    assert!(d.errors.is_empty(), "{:?}", d.errors);
    assert_eq!(
        shape(&d),
        [vec!["CurseForge:100", "WowUpHub:47671"], vec!["WowUpHub:2"]]
    );
    let e = &d.entries[0];
    assert_eq!(e.preferred().provider, "CurseForge");
    assert_eq!(e.ids, ["curse:100", "github:questie/questie", "hub:47671"]);
}

#[test]
fn f4_8_same_name_with_different_repos_is_not_merged() {
    let (_s, providers) = sources(
        vec![
            cf_with_source(100, "Questie", Some("https://github.com/Questie/Questie")),
            // Not exactly a repo URL: never an id.
            cf_with_source(
                200,
                "Bagnon",
                Some("https://github.com/Jaliborc/Bagnon/tree/main"),
            ),
            cf_with_source(300, "Details", None),
        ],
        vec![
            hub_repo(1, "SomeFork/Questie"),
            hub_repo(2, "Jaliborc/Bagnon"),
            hub_repo(3, "Tercioo/Details"),
        ],
    );

    let d = discover::search(F, &providers, "x");

    assert_eq!(d.entries.len(), 6, "{:?}", shape(&d));
    assert!(d.entries.iter().all(|e| e.sources.len() == 1));
}

#[test]
fn f4_8_conflicting_ids_are_not_merged() {
    // Two CurseForge projects name the same repo: the Hub entry can't tell which one it is.
    let (_s, providers) = sources(
        vec![
            cf_with_source(1, "Questie", Some("https://github.com/Questie/Questie")),
            cf_with_source(2, "QuestieDB", Some("https://github.com/Questie/Questie")),
        ],
        vec![hub_repo(47671, "Questie/Questie")],
    );

    let d = discover::search(F, &providers, "questie");

    assert_eq!(d.entries.len(), 3, "{:?}", shape(&d));
    assert!(d.entries.iter().all(|e| e.sources.len() == 1));
}

#[test]
fn f4_8_failing_provider_returns_partial_results() {
    let (s, providers) = sources(vec![], vec![hub_repo(1, "me/Foo")]);
    s.respond("GET", "/v1/mods/search", 500, "text/plain", "boom");

    let d = discover::search(F, &providers, "foo");

    assert_eq!(shape(&d), [vec!["WowUpHub:1"]]);
    assert_eq!(d.errors.len(), 1);
    assert_eq!(d.errors[0].0, "CurseForge");
    assert!(d.errors[0].1.contains("500"), "{}", d.errors[0].1);

    // Without a key CurseForge is skipped quietly; featured lists group the same way.
    s.json(
        "GET",
        "/hub/addons/featured/forever",
        &json!({ "addons": [hub_repo(1, "me/Foo")], "recent": [hub_repo(1, "me/Foo")] }),
    );
    let keyless: Vec<Box<dyn Provider>> = vec![
        Box::new(CurseForge {
            api_key: None,
            base_url: s.url().into(),
        }),
        Box::new(hub(&s)),
    ];
    let d = discover::featured(F, &keyless);
    assert!(d.errors.is_empty());
    assert_eq!(shape(&d), [vec!["WowUpHub:1"]]);
}

#[test]
fn f3_2_big_downloads_stream_to_a_file() {
    // Addon zips (audio packs pass 512 MB) go to a file; only small downloads are held in memory.
    let s = FakeServer::start();
    let size = (64 << 20) + 4096;
    s.bytes("GET", "/big.zip", vec![7u8; size]);
    let url = format!("{}/big.zip", s.url());
    let err = http::download(&url, &mut |_, _| {}).unwrap_err();
    assert!(format!("{err:#}").contains("too large"), "{err:#}");

    let mut file = tempfile::tempfile().unwrap();
    let mut last = (0, None::<u64>);
    let n = http::download_to(&url, &mut file, &mut |done, total| last = (done, total)).unwrap();
    assert_eq!(n, size as u64);
    assert_eq!(last.0, size as u64, "progress reaches the end");
    assert_eq!(file.metadata().unwrap().len(), size as u64);
}
