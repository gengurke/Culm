//! F3.1 at core level: one batch per provider, failures isolated, ignored addons left alone.

use addon_core::providers::{self, ProviderConfig, WowInterface};
use addon_core::updates::check_updates;
use addon_core::{Addon, Channel, Flavor, IgnoreReason};
use addon_testkit::{
    FakeServer, cf_envelope, cf_file, cf_mod, cf_mod_with_files, fixture, fixture_at,
};
use serde_json::json;

fn addon(provider: &str, external_id: &str, installed_file: &str) -> Addon {
    Addon {
        id: format!("{provider}-{external_id}"),
        name: String::new(),
        author: String::new(),
        summary: String::new(),
        provider: Some(provider.into()),
        external_id: Some(external_id.into()),
        url: None,
        thumbnail_url: None,
        folders: vec![external_id.replace('/', "_")],
        installed_version: None,
        installed_file_id: Some(installed_file.into()),
        installed_at: None,
        game_version: None,
        channel: Channel::Stable,
        latest: None,
        ignored: None,
        auto_update: false,
        dependencies: vec![],
    }
}

fn config(s: &FakeServer, key: Option<&str>) -> ProviderConfig {
    ProviderConfig {
        curseforge_api_key: key.map(Into::into),
        curseforge_base_url: s.url().into(),
        github_token: None,
        github_base_url: s.url().into(),
        wowinterface_base_url: format!("{}/wowi", s.url()),
    }
}

#[test]
fn f3_1_check_updates_batches_per_provider_and_isolates_failures() {
    let s = FakeServer::start();
    let beta = cf_mod_with_files(
        2,
        "Bee",
        vec![
            cf_file(21, "2.1", 1, &["Bee"], 200),
            cf_file(22, "2.2-beta", 2, &["Bee"], 200),
        ],
    );
    s.json(
        "POST",
        "/v1/mods",
        &cf_envelope(json!([cf_mod(1, "Ace", &["Ace"], 11, "1.1"), beta])),
    );
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
    // gone/Repo: 404 (unrouted). WowInterface answers 500.
    s.respond("GET", "/wowi/*", 500, "text/plain", "down");

    let mut addons = vec![
        addon("CurseForge", "1", "10"),
        addon("CurseForge", "2", "21"),
        addon("CurseForge", "3", "30"), // not returned by the API: stays without `latest`
        addon("GitHub", "Questie/Questie", "80"),
        addon("GitHub", "gone/Repo", "1"),
        addon("WowInterface", "123", "1"),
    ];
    addons[1].channel = Channel::Beta;
    let mut ignored = addon("CurseForge", "4", "40");
    ignored.ignored = Some(IgnoreReason::User);
    addons.push(ignored);

    // WowInterface is not in `providers::all` (no flavor info); add it to check failure isolation.
    let mut list = providers::all(&config(&s, Some("k")));
    list.push(Box::new(WowInterface { base_url: format!("{}/wowi", s.url()) }));
    let errors = check_updates(Flavor::Forever, &mut addons, &list);

    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].starts_with("WowInterface: "), "{errors:?}");
    // One CurseForge batch with every managed id, ignored ones left out.
    let cf = s.requests_to("/v1/mods");
    assert_eq!(cf.len(), 1);
    assert_eq!(cf[0].json(), json!({ "modIds": [1, 2, 3] }));

    let latest = |i: usize| addons[i].latest.as_ref().map(|f| f.id.as_str());
    assert_eq!(latest(0), Some("11"));
    assert!(addons[0].update_available());
    assert_eq!(
        (addons[0].name.as_str(), addons[0].summary.as_str()),
        ("Ace", "Ace summary")
    );
    assert!(
        addons[0].url.is_some() && addons[0].thumbnail_url.is_some(),
        "empty fields filled"
    );
    assert_eq!(latest(1), Some("22"), "Beta channel takes the beta file");
    assert_eq!(latest(2), None);
    assert_eq!(
        latest(3),
        Some("80"),
        "GitHub checked although another repo failed"
    );
    assert!(!addons[3].update_available());
    assert_eq!(latest(4), None);
    assert_eq!(latest(6), None, "ignored addons are not checked");
}

#[test]
fn f3_1_check_updates_without_key_skips_curseforge_quietly() {
    let s = FakeServer::start();
    let mut addons = vec![addon("CurseForge", "1", "10")];
    let errors = check_updates(
        Flavor::Forever,
        &mut addons,
        &providers::all(&config(&s, None)),
    );
    assert!(errors.is_empty(), "{errors:?}");
    assert!(s.requests().is_empty());
    assert_eq!(addons[0].latest, None);
}
