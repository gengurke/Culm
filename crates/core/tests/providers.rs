//! Provider feature tests (CurseForge, GitHub, WowInterface) against a local fake server.

use addon_core::providers::{
    self, CurseForge, GitHub, Provider, ProviderConfig, ScannedFolder, WowInterface,
};
use addon_core::{Channel, DependencyKind, Flavor, SearchFilter, Sort, http};
use addon_testkit::{
    FakeServer, cf_envelope, cf_file, cf_mod, cf_mod_with_files, fixture, fixture_at,
};
use serde_json::json;

const F: Flavor = Flavor::Forever;

fn curseforge(s: &FakeServer) -> CurseForge {
    CurseForge {
        api_key: Some("test-key".into()),
        base_url: s.url().into(),
    }
}

fn folder(name: &str, fingerprint: u32) -> ScannedFolder {
    ScannedFolder {
        name: name.into(),
        fingerprint,
        hub_fingerprint: String::new(),
        toc_ids: vec![],
    }
}

#[test]
fn f4_1_search_curseforge_filters_forever_files() {
    let s = FakeServer::start();
    s.json("GET", "/v1/mods/search", &fixture("cf_search.json"));

    let results = curseforge(&s).search(F, "damage meter").unwrap();

    let req = &s.requests_to("/v1/mods/search")[0];
    assert_eq!(req.header("x-api-key"), Some("test-key"));
    for (k, v) in [
        ("gameId", "1"),
        ("gameVersionTypeId", "88568"),
        ("searchFilter", "damage meter"),
        ("sortField", "2"),
        ("sortOrder", "desc"),
        ("pageSize", "50"),
    ] {
        assert_eq!(req.query_param(k).as_deref(), Some(v), "query param {k}");
    }
    // The retail-only mod is dropped.
    assert_eq!(results.len(), 1);
    let a = &results[0];
    assert_eq!(
        (a.provider.as_str(), a.external_id.as_str()),
        ("CurseForge", "61284")
    );
    assert_eq!(a.name, "Details! Damage Meter");
    assert_eq!(a.author, "Terciob", "_forgeuser names are dropped");
    assert_eq!(a.download_count, 512_345_678);
    assert_eq!(
        a.url.as_deref(),
        Some("https://www.curseforge.com/wow/addons/details")
    );
    assert!(a.thumbnail_url.as_deref().unwrap().ends_with("details.png"));
    // Retail file 5200003 and alternative (nolib) file 5200004 are skipped; one per channel, newest first.
    let ids: Vec<(&str, Channel)> = a.files.iter().map(|f| (f.id.as_str(), f.channel)).collect();
    assert_eq!(
        ids,
        [
            ("5200005", Channel::Beta),
            ("5200002", Channel::Alpha),
            ("5200001", Channel::Stable)
        ]
    );
    let stable = a.latest_for(Channel::Stable).unwrap();
    assert_eq!(stable.version, "Details.Forever.20260920");
    assert_eq!(stable.folders, ["Details", "Details_DataStorage"]);
    assert_eq!(stable.game_versions, ["1.60.1"]);
    assert_eq!(stable.released_at.as_deref(), Some("2026-09-20T10:00:00Z"));
    let kinds: Vec<DependencyKind> = stable.dependencies.iter().map(|d| d.kind).collect();
    assert_eq!(
        kinds,
        [
            DependencyKind::Embedded,
            DependencyKind::Optional,
            DependencyKind::Required,
            DependencyKind::Other
        ]
    );
    assert_eq!(stable.dependencies[0].external_id, "1001");
    assert_eq!(a.categories, ["Combat"]);
    assert_eq!(a.updated_at.as_deref(), Some("2026-10-12T10:00:00Z"));
    // Null downloadUrl: the author allows downloads only in CurseForge's app, so no URL is made up.
    assert_eq!(a.files[0].download_url, "");
}

#[test]
fn f3_2_only_curseforge_hosts_get_the_key() {
    use providers::curseforge::is_curseforge_host as cf;
    assert!(cf("https://edge.forgecdn.net/files/1/2/a.zip"));
    assert!(cf("https://mediafilez.forgecdn.net/files/1/2/a.zip"));
    assert!(cf("https://www.curseforge.com/wow/addons/x"));
    assert!(!cf("https://forgecdn.net.evil.example/a.zip"));
    assert!(!cf("https://evilforgecdn.net/a.zip"));
    assert!(!cf("https://edge.forgecdn.net@evil.example/a.zip"));
    assert!(!cf("https://github.com/a/b/releases/download/v1/a.zip"));
}

#[test]
fn f4_2_popular_lists_featured() {
    let s = FakeServer::start();
    s.json("POST", "/v1/mods/featured", &fixture("cf_featured.json"));

    let results = curseforge(&s).featured(F).unwrap();

    let req = &s.requests_to("/v1/mods/featured")[0];
    assert_eq!(req.header("x-api-key"), Some("test-key"));
    assert_eq!(
        req.json(),
        json!({ "gameId": 1, "gameVersionTypeId": 88568, "excludedModIds": [] })
    );
    // popular + recentlyUpdated, deduped, mods without a Forever file dropped ("featured" is not used).
    let ids: Vec<&str> = results.iter().map(|a| a.external_id.as_str()).collect();
    assert_eq!(ids, ["61284", "334372", "1592"]);
}

#[test]
fn f3_1_get_many_reports_latest_file_per_channel() {
    let s = FakeServer::start();
    let a = cf_mod_with_files(
        1,
        "Alpha Addon",
        vec![
            cf_file(13, "1.3-alpha", 3, &["A"], 100),
            cf_file(10, "1.0", 1, &["A"], 100),
            cf_file(12, "1.2-beta", 2, &["A"], 100),
            cf_file(11, "1.1", 1, &["A"], 100),
        ],
    );
    let b = cf_mod(2, "Bee", &["Bee", "Bee_Options"], 20, "Bee-2.0.zip");
    s.json("POST", "/v1/mods", &cf_envelope(json!([a, b])));

    let addons = curseforge(&s)
        .get_many(F, &["1".into(), "2".into(), "junk".into()])
        .unwrap();

    let req = &s.requests_to("/v1/mods")[0];
    assert_eq!(req.header("x-api-key"), Some("test-key"));
    assert_eq!(req.json(), json!({ "modIds": [1, 2] }));
    let latest = |ch| addons[0].latest_for(ch).unwrap().id.clone();
    assert_eq!(latest(Channel::Stable), "11");
    assert_eq!(latest(Channel::Beta), "12");
    assert_eq!(latest(Channel::Alpha), "13");
    assert_eq!(
        addons[1].latest_for(Channel::Stable).unwrap().folders,
        ["Bee", "Bee_Options"]
    );
    // A zip name uploaded as the display name shows as a version.
    assert_eq!(
        addons[1].latest_for(Channel::Stable).unwrap().version,
        "Bee-2.0"
    );
}

#[test]
fn f2_3_fingerprint_match_maps_modules_to_folders() {
    let s = FakeServer::start();
    s.json(
        "POST",
        "/v1/fingerprints/1",
        &fixture("cf_fingerprints.json"),
    );
    let details = fixture("cf_search.json")["data"][0].clone();
    s.json("POST", "/v1/mods", &cf_envelope(json!([details])));

    let folders = [
        folder("Details", 3001111111),
        folder("Details_DataStorage", 3002222222),
        folder("MyLocalHack", 999),
    ];
    let matches = curseforge(&s).match_folders(F, &folders).unwrap();

    let req = &s.requests_to("/v1/fingerprints/1")[0];
    assert_eq!(req.header("x-api-key"), Some("test-key"));
    assert_eq!(
        req.json(),
        json!({ "fingerprints": [3001111111u32, 3002222222u32, 999] })
    );
    assert_eq!(
        s.requests_to("/v1/mods")[0].json(),
        json!({ "modIds": [61284] })
    );
    let got: Vec<&str> = matches.iter().map(|m| m.folder.as_str()).collect();
    assert_eq!(got, ["Details", "Details_DataStorage"]);
    for m in &matches {
        assert_eq!(m.addon.external_id, "61284");
        let installed = m.installed_file.as_ref().unwrap();
        assert_eq!(installed.id, "5200001");
        assert_eq!(installed.folders, ["Details", "Details_DataStorage"]);
    }
}

#[test]
fn f4_4_curseforge_url_resolves_slug() {
    let s = FakeServer::start();
    s.json("GET", "/v1/mods/search", &fixture("cf_search.json"));
    let cf = curseforge(&s);

    let addon = cf
        .from_url(F, "https://www.curseforge.com/wow/addons/details")
        .unwrap()
        .unwrap();
    assert_eq!(addon.external_id, "61284");
    let req = &s.requests_to("/v1/mods/search")[0];
    assert_eq!(req.query_param("slug").as_deref(), Some("details"));
    assert_eq!(req.query_param("gameId").as_deref(), Some("1"));

    let err = cf
        .from_url(F, "https://www.curseforge.com/wow/addons/retail-only")
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("no World of Warcraft: Forever release"),
        "{err}"
    );
    assert!(cf.from_url(F, "https://github.com/a/b").unwrap().is_none());
}

#[test]
fn f4_4_link_source_tells_links_from_search_text() {
    for (input, source) in [
        (
            "https://www.curseforge.com/wow/addons/details",
            Some("CurseForge"),
        ),
        (
            "curseforge.com/wow/addons/dbm-core/files",
            Some("CurseForge"),
        ),
        ("https://github.com/Questie/Questie", Some("GitHub")),
        ("  Questie/Questie ", Some("GitHub")),
        ("https://wowup.io/addons/1234", Some("WowUpHub")),
        ("wowup.io/addons/1234/questie", Some("WowUpHub")),
        ("https://wowup.io/addons/questie", None),
        ("details", None),
        ("damage meter", None),
        ("bag / bank", None),
        ("https://example.com/a/b", None),
    ] {
        assert_eq!(providers::link_source(input), source, "{input:?}");
    }
}

#[test]
fn f4_5_description_is_plain_text() {
    let s = FakeServer::start();
    s.json(
        "GET",
        "/v1/mods/61284/description",
        &cf_envelope(json!(
            "<h2>&nbsp;</h2><p><img src=\"x.png\"><em>Most powerful</em> meter &amp; more.</p>\
             <script>alert(1)</script><ul><li>Raids</li><li>Dungeons</li></ul>"
        )),
    );
    let text = curseforge(&s).description("61284").unwrap();
    assert_eq!(text, "Most powerful meter & more.\n\n• Raids\n• Dungeons");
    assert_eq!(
        s.requests_to("/v1/mods/61284/description")[0].header("x-api-key"),
        Some("test-key")
    );
    assert!(
        curseforge(&s).description("../x").is_err(),
        "ids are numbers"
    );
}

#[test]
fn f4_6_categories_are_the_addon_tree() {
    let s = FakeServer::start();
    let cat = |id: u32, name: &str, parent: u32| {
        json!({ "id": id, "gameId": 1, "name": name, "slug": name.to_lowercase().replace(' ', "-"),
                "classId": 1, "parentCategoryId": parent, "isClass": null })
    };
    s.json(
        "GET",
        "/v1/categories",
        &cf_envelope(json!([
            { "id": 1, "gameId": 1, "name": "Addons", "slug": "addons", "isClass": true },
            cat(1009, "Bags & Inventory", 1),
            cat(1020, "Class", 1),
            cat(1028, "Warrior", 1020),
            { "id": 6, "gameId": 1, "name": "Other class root", "slug": "x", "classId": 99, "parentCategoryId": 99 },
        ])),
    );
    let cats = curseforge(&s).categories().unwrap();
    let got: Vec<(u32, &str, Option<u32>)> = cats
        .iter()
        .map(|c| (c.id, c.name.as_str(), c.parent))
        .collect();
    assert_eq!(
        got,
        [
            (1009, "Bags & Inventory", None),
            (1020, "Class", None),
            (1028, "Warrior", Some(1020))
        ]
    );
    assert_eq!(cats[2].slug, "warrior");
    let req = &s.requests_to("/v1/categories")[0];
    assert_eq!(req.query_param("gameId").as_deref(), Some("1"));
}

#[test]
fn f4_6_search_sends_category_and_sort() {
    let s = FakeServer::start();
    s.json("GET", "/v1/mods/search", &fixture("cf_search.json"));
    let cf = curseforge(&s);
    let results = cf
        .search_filtered(&SearchFilter {
            text: String::new(),
            category: Some(1009),
            sort: Sort::Name,
        })
        .unwrap();
    assert_eq!(results.len(), 1);
    let req = &s.requests_to("/v1/mods/search")[0];
    for (k, v) in [
        ("categoryId", Some("1009")),
        ("sortField", Some("4")),
        ("sortOrder", Some("asc")),
        ("gameVersionTypeId", Some("88568")),
        ("searchFilter", None),
    ] {
        assert_eq!(req.query_param(k).as_deref(), v, "query param {k}");
    }
    for (sort, field) in [
        (Sort::Popularity, "2"),
        (Sort::LastUpdated, "3"),
        (Sort::TotalDownloads, "6"),
    ] {
        cf.search_filtered(&SearchFilter {
            text: "bag".into(),
            category: None,
            sort,
        })
        .unwrap();
        let req = s.requests_to("/v1/mods/search").pop().unwrap();
        assert_eq!(req.query_param("sortField").as_deref(), Some(field));
        assert_eq!(req.query_param("sortOrder").as_deref(), Some("desc"));
        assert_eq!(req.query_param("searchFilter").as_deref(), Some("bag"));
        assert_eq!(req.query_param("categoryId"), None);
    }
}

#[test]
fn f4_5_github_readme_is_plain_text() {
    let s = FakeServer::start();
    s.respond(
        "GET",
        "/repos/Questie/Questie/readme",
        200,
        "text/plain",
        "# Questie\n[![CI](https://x/ci.svg)](https://x)\n\nThe **best** quest helper.\n\n- Shows quests\n",
    );
    let gh = GitHub {
        token: None,
        api_base: s.url().into(),
    };
    assert_eq!(
        gh.readme("Questie/Questie").unwrap(),
        "Questie\n\nThe best quest helper.\n\n• Shows quests"
    );
    let req = &s.requests_to("/repos/Questie/Questie/readme")[0];
    assert_eq!(req.header("accept"), Some("application/vnd.github.raw"));
    assert!(gh.readme("../../etc").is_err(), "only owner/repo ids");
}

fn github_server() -> FakeServer {
    let s = FakeServer::start();
    s.json(
        "GET",
        "/repos/Questie/Questie",
        &fixture("github_repo.json"),
    );
    s.json(
        "GET",
        "/repos/Questie/Questie/releases",
        &fixture_at("github_releases.json", s.url()),
    );
    s.json(
        "GET",
        "/dl/82/release.json",
        &fixture("github_release.json"),
    );
    s
}

#[test]
fn f4_4_github_url_picks_forever_asset() {
    let s = github_server();
    let gh = GitHub {
        token: Some("gh-token".into()),
        api_base: s.url().into(),
    };

    let addon = gh
        .from_url(F, "https://github.com/Questie/Questie")
        .unwrap()
        .unwrap();

    assert_eq!(
        (addon.provider.as_str(), addon.external_id.as_str()),
        ("GitHub", "Questie/Questie")
    );
    assert_eq!(addon.author, "Questie");
    // Draft skipped; prerelease → Beta via `-forever.zip` (not the nolib one); stable via release.json (nolib=false).
    let beta = addon
        .files
        .iter()
        .find(|f| f.channel == Channel::Beta)
        .unwrap();
    assert_eq!(
        (beta.id.as_str(), beta.version.as_str()),
        ("91", "v12.10.0-beta")
    );
    let stable = addon.latest_for(Channel::Stable).unwrap();
    assert_eq!(
        (stable.id.as_str(), stable.version.as_str()),
        ("80", "v12.9.0")
    );
    assert_eq!(
        stable.download_url,
        format!("{}/dl/80/Questie-v12.9.0.zip", s.url())
    );
    assert_eq!(addon.latest_for(Channel::Beta).unwrap().id, "91");

    let api = &s.requests_to("/repos/Questie/Questie")[0];
    assert_eq!(api.header("authorization"), Some("Bearer gh-token"));
    assert_eq!(
        s.requests_to("/dl/82/release.json")[0].header("authorization"),
        None
    );
    // Bare owner/repo works too; other URLs are not GitHub's.
    assert!(gh.from_url(F, "Questie/Questie").unwrap().is_some());
    assert!(
        gh.from_url(F, "https://www.curseforge.com/wow/addons/details")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        gh.get_many(F, &["Questie/Questie".into()]).unwrap()[0]
            .files
            .len(),
        2
    );
}

#[test]
fn f4_4_github_plain_zip_fallback_skips_other_flavors() {
    let s = FakeServer::start();
    s.json(
        "GET",
        "/repos/me/Plain",
        &json!({
            "full_name": "me/Plain", "name": "Plain", "html_url": "https://github.com/me/Plain",
            "description": null, "owner": { "login": "me", "avatar_url": null }
        }),
    );
    let asset = |id: u64, name: &str| {
        json!({
            "id": id, "name": name, "content_type": "application/zip", "download_count": 1,
            "browser_download_url": format!("{}/dl/{id}/{name}", s.url())
        })
    };
    s.json("GET", "/repos/me/Plain/releases", &json!([
        { "tag_name": "v2", "draft": false, "prerelease": false, "published_at": "2026-11-02T00:00:00Z",
          "assets": [asset(21, "Plain-v2-mainline.zip"), asset(22, "Plain-v2-classic.zip")] },
        { "tag_name": "v1", "draft": false, "prerelease": false, "published_at": "2026-11-01T00:00:00Z",
          "assets": [asset(11, "Plain-v1-classic.zip"), asset(12, "Plain-v1.zip")] }
    ]));
    let gh = GitHub {
        token: None,
        api_base: s.url().into(),
    };

    let addon = gh.from_url(F, "github.com/me/Plain").unwrap().unwrap();
    assert_eq!(addon.files.len(), 1);
    assert_eq!(
        (addon.files[0].id.as_str(), addon.files[0].version.as_str()),
        ("12", "v1")
    );
    assert_eq!(
        s.requests_to("/repos/me/Plain")[0].header("authorization"),
        None
    );
}

#[test]
fn f2_3_wowinterface_matches_toc_ids() {
    let s = FakeServer::start();
    s.json(
        "GET",
        "/filedetails/5108.json",
        &fixture_at("wowi_filedetails.json", s.url()),
    );
    let wowi = WowInterface {
        base_url: s.url().into(),
    };
    let folders = [
        ScannedFolder {
            name: "Clique".into(),
            fingerprint: 1,
            hub_fingerprint: String::new(),
            toc_ids: vec![("WowInterface".into(), "5108".into())],
        },
        folder("Other", 2),
    ];

    let matches = wowi.match_folders(F, &folders).unwrap();

    assert_eq!(matches.len(), 1);
    let a = &matches[0].addon;
    assert_eq!(
        (
            matches[0].folder.as_str(),
            a.provider.as_str(),
            a.name.as_str()
        ),
        ("Clique", "WowInterface", "Clique")
    );
    let file = a.latest_for(Channel::Stable).unwrap();
    assert_eq!(file.version, "v5.1.5-release");
    assert_eq!(
        file.download_url,
        format!("{}/wowi/getfile.php?id=5108", s.url())
    );
    let by_url = wowi
        .from_url(
            F,
            "https://www.wowinterface.com/downloads/info5108-Clique.html",
        )
        .unwrap();
    assert_eq!(by_url.unwrap().external_id, "5108");
}

#[test]
fn f7_1_curseforge_unavailable_without_key() {
    let s = FakeServer::start();
    for key in [None, Some(String::new())] {
        let cf = CurseForge {
            api_key: key,
            base_url: s.url().into(),
        };
        assert!(!cf.available());
        let err = cf.search(F, "x").unwrap_err();
        assert!(err.to_string().contains("API key"), "{err}");
    }
    assert!(s.requests().is_empty(), "no request without a key");

    let all = providers::all(&ProviderConfig::default());
    let names: Vec<(&str, bool)> = all.iter().map(|p| (p.name(), p.available())).collect();
    assert_eq!(
        names,
        [("CurseForge", false), ("WowUpHub", true), ("GitHub", true)]
    );
    let cfg = ProviderConfig {
        curseforge_api_key: Some("k".into()),
        ..Default::default()
    };
    assert!(providers::all(&cfg)[0].available());
}

#[test]
fn f3_2_download_reports_progress_and_http_errors() {
    let s = FakeServer::start();
    let zip = addon_testkit::make_zip(&[("A/A.toc", b"## Title: A")]);
    s.bytes("GET", "/files/a.zip", zip.clone());
    s.respond(
        "GET",
        "/v1/mods/search",
        403,
        "application/json",
        r#"{"error":"bad key"}"#,
    );

    let mut calls = vec![];
    let bytes = http::download(&format!("{}/files/a.zip", s.url()), &mut |done, total| {
        calls.push((done, total))
    })
    .unwrap();
    assert_eq!(bytes, zip);
    assert_eq!(
        calls.last(),
        Some(&(zip.len() as u64, Some(zip.len() as u64)))
    );
    assert!(
        s.requests()[0]
            .header("user-agent")
            .unwrap()
            .starts_with("addon-manager/")
    );

    let err = curseforge(&s).search(F, "x").unwrap_err().to_string();
    assert!(err.contains("403") && err.contains("bad key"), "{err}");
    assert!(
        http::download(&format!("{}/missing", s.url()), &mut |_, _| {})
            .unwrap_err()
            .to_string()
            .contains("404")
    );
}

#[test]
fn f3_5_curseforge_file_carries_its_size_and_sha1() {
    let s = FakeServer::start();
    let mut file = cf_file(11, "1.1", 1, &["A"], 100);
    file["downloadUrl"] = serde_json::Value::Null;
    file["fileLength"] = json!(830726);
    file["hashes"] = json!([
        { "value": "0E3F7B43BD22A290B66B9E291A2FE75A", "algo": 2 },
        { "value": "4923D6A3F467C5A53DC1B915005440ED15849FF3", "algo": 1 }
    ]);
    s.json(
        "POST",
        "/v1/mods",
        &cf_envelope(json!([cf_mod_with_files(1, "Alpha Addon", vec![file])])),
    );

    let addons = curseforge(&s).get_many(F, &["1".into()]).unwrap();
    let latest = addons[0].latest_for(Channel::Stable).unwrap();
    assert_eq!(
        latest.download_url, "",
        "no link: the author allows only CurseForge"
    );
    assert_eq!(
        latest.check,
        Some(addon_core::FileCheck {
            size: 830726,
            sha1: "4923d6a3f467c5a53dc1b915005440ed15849ff3".into()
        })
    );
    assert!(
        !serde_json::to_string(latest).unwrap().contains("4923d6a3"),
        "the hash is never written to disk (#69)"
    );
}
