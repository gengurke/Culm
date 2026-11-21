//! End-to-end feature specs for installations, My Addons, updates, removal and maintenance:
//! the real UI and core over a temp Forever tree and a fake CurseForge (see `common`).

mod common;
use addon_app::state::{MaintCard, MaintPhase, Op, Status};
use addon_core::IgnoreReason;
use common::*;
use rune::ImageCache;
use std::sync::Arc;

#[test]
fn f1_1_detected_installation_is_selected_and_scanned() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    assert!(
        app.shows("WoW Forever") && !app.shows("testprefix)"),
        "a single installation is named without its Wine prefix"
    );
    assert!(app.shows("1.60.1.70938"));
    assert!(app.shows("Ace"));
}

#[test]
fn f1_2_installation_added_from_folder_picker_is_remembered() {
    let mut world = World::new().with_key("test-key");
    world.search_roots.clear(); // nothing to auto-detect
    let mut app = world.app();
    app.wait_until("onboarding", |a| a.has("onboarding/add"));
    app.tap("onboarding/add");
    assert!(app.rt.file_dialog().is_some(), "folder picker requested");
    app.rt
        .resolve_file_dialog(Some(world.wow_root.clone()))
        .unwrap();
    app.frame();
    app.wait_for("the new installation to be scanned", |s| {
        s.installation().is_some() && !s.addons.is_empty()
    });
    app.wait_idle();
    assert!(app.shows("Installation added"));

    let mut again = world.app();
    again.wait_idle();
    assert!(
        again.state().installation().is_some(),
        "stored installation found without auto-detect"
    );
    assert!(again.shows("Ace"));
}

#[test]
fn f2_1_f2_3_scan_shows_matched_and_unknown_addons() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    assert_eq!(app.names(), ["Ace", "Bee", "Homebrew"]);
    let ace = app.row("Ace", "");
    assert!(
        app.shows_in(&ace, "CurseForge"),
        "fingerprint match shows the provider"
    );
    assert!(app.shows_in(&app.row("Homebrew", ""), "Unknown"));
    let bee = &app.state().addons[1].addon;
    assert_eq!(
        bee.folders,
        ["Bee", "Bee_Options"],
        "one entry for both folders"
    );
    assert_eq!(app.status("Homebrew"), Status::Unknown);
}

#[test]
fn f2_1_thumbnails_load_over_initials() {
    let world = World::new().with_key("test-key");
    let mut state = addon_app::State::new(Arc::new(addon_app::core_backend::CoreBackend::new(
        world.config(),
    )));
    state.images = Some(ImageCache::new(Arc::new(|| {})));
    let mut app = App {
        rt: addon_app::runtime(state, (1280.0, 800.0)),
    };
    app.rt.frame(0.0);
    app.wait_idle();
    let thumb = app.row("Ace", "thumb");
    app.wait_until("the thumbnail", |a| a.has(&thumb));
    assert!(
        !app.has(&app.row("Homebrew", "thumb")),
        "unknown addons keep their initials"
    );
    assert!(
        world
            .tmp
            .path()
            .join("cache/images")
            .read_dir()
            .unwrap()
            .next()
            .is_some(),
        "cached on disk"
    );
}

#[test]
fn f2_4_first_scan_imports_wowup_records_without_a_key() {
    let world = World::new().with_wowup();
    let mut app = world.app();
    app.wait_idle();
    assert_eq!(app.names(), ["Ace", "Bee", "Homebrew"]);
    let ace = &app.state().addons[0].addon;
    assert_eq!(
        (ace.provider.as_deref(), ace.external_id.as_deref()),
        (Some("CurseForge"), Some("1"))
    );
    assert_eq!(
        app.status("Ace"),
        Status::UpdateAvailable,
        "WowUp's last known release"
    );
    assert_eq!(
        app.status("Bee"),
        Status::Unknown,
        "not in WowUp, no key to match it"
    );
    assert!(
        app.has("my-addons/no-key"),
        "says why CurseForge isn't checked"
    );
    // Only the keyless WowUp Hub is asked (its fingerprints match nothing here).
    assert!(
        world
            .server
            .requests()
            .iter()
            .all(|r| r.path.starts_with("/hub/")),
        "no CurseForge without a key"
    );
}

#[test]
fn f3_1_check_shows_update_available() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    assert_eq!(app.status("Ace"), Status::UpdateAvailable);
    assert!(app.shows_in(&app.row("Ace", ""), "Update available"));
    assert!(app.shows_in(&app.row("Ace", ""), "1.0 → 1.1"));
    assert!(app.shows("Update all (2)"));

    let checks = world.server.requests_to("/v1/mods").len();
    app.tap("my-addons/check");
    app.wait_idle();
    assert!(
        world.server.requests_to("/v1/mods").len() > checks,
        "asked CurseForge again"
    );
    assert_eq!(app.status("Ace"), Status::UpdateAvailable);
}

#[test]
fn f3_2_update_replaces_folder_contents() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    app.tap(&app.row("Ace", "update"));
    app.wait_idle();
    assert_eq!(app.status("Ace"), Status::UpToDate);
    assert!(app.shows_in(&app.row("Ace", ""), "Up to date"));
    assert!(app.shows("Updated Ace"));
    assert!(
        world
            .read("Interface/AddOns/Ace/Ace.toc")
            .contains("## Version: 1.1")
    );
    assert!(world.exists("Interface/AddOns/Ace/New.lua"));
    assert!(
        !world.exists("Interface/AddOns/Ace/Core.lua"),
        "files of the old version are gone"
    );

    let mut again = world.app();
    again.wait_idle();
    assert_eq!(
        again.status("Ace"),
        Status::UpToDate,
        "the new version is remembered"
    );
}

#[test]
fn f3_2_update_all_updates_every_outdated_addon() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    app.tap("my-addons/update-all");
    app.wait_idle();
    assert_eq!(app.state().update_count(), 0);
    assert_eq!(
        (app.status("Ace"), app.status("Bee")),
        (Status::UpToDate, Status::UpToDate)
    );
    assert!(
        world
            .read("Interface/AddOns/Bee_Options/Bee_Options.toc")
            .contains("2.1")
    );
    assert!(app.shows("Update all"));
}

#[test]
fn f3_3_channel_change_rechecks_only_that_addon() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    assert!(app.shows_in(&app.row("Ace", ""), "1.0 → 1.1"));
    let before = world.server.requests_to("/v1/mods").len();
    app.tap(&app.row("Ace", ""));
    app.tap("detail/channel/beta");
    assert_eq!(app.state().job, addon_app::Job::Idle, "no full check");
    assert_eq!(
        app.status("Ace"),
        Status::Working,
        "Ace shows it is being checked"
    );
    app.wait_idle();
    let asked = world.server.requests_to("/v1/mods");
    assert_eq!(asked.len(), before + 1, "one request for the change");
    assert_eq!(
        asked[before].json()["modIds"],
        serde_json::json!([1]),
        "only Ace is asked"
    );
    assert!(app.shows_in(&app.row("Ace", ""), "1.0 → 1.2-beta"));
    assert!(!app.shows("Update check failed"));
}

#[test]
fn f3_4_ignored_addon_is_not_offered_for_update() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    app.tap(&app.row("Ace", ""));
    app.tap("detail/ignore");
    assert_eq!(app.status("Ace"), Status::Ignored);
    assert!(!app.has(&app.row("Ace", "update")));
    assert!(app.shows("Update all (1)"));

    let mut again = world.app();
    again.wait_idle();
    assert_eq!(
        again.state().addons[0].addon.ignored,
        Some(IgnoreReason::User),
        "ignore is stored"
    );
    assert_eq!(again.status("Ace"), Status::Ignored);
    again.tap(&again.row("Ace", ""));
    again.tap("detail/ignore");
    again.wait_idle();
    assert_eq!(again.status("Ace"), Status::UpdateAvailable);
}

#[test]
fn f5_1_remove_asks_then_deletes_the_folders() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    app.tap(&app.row("Bee", ""));
    app.tap("detail/remove");
    assert!(app.shows("Delete Bee and its 2 folders from AddOns?"));
    assert!(
        world.exists("Interface/AddOns/Bee"),
        "first press only asks"
    );
    app.tap("detail/remove-confirm");
    app.wait_idle();
    assert_eq!(app.names(), ["Ace", "Homebrew"]);
    assert!(app.shows("Removed Bee"));
    assert!(!world.exists("Interface/AddOns/Bee") && !world.exists("Interface/AddOns/Bee_Options"));

    let mut again = world.app();
    again.wait_idle();
    assert_eq!(again.names(), ["Ace", "Homebrew"]);
}

#[test]
fn f6_1_clear_cache_asks_then_deletes_the_cache() {
    let world = World::new().with_key("test-key");
    write(
        &world.client.join("Cache/WDB/enUS/creaturecache.wdb"),
        vec![0u8; 2048],
    );
    let mut app = world.app();
    app.wait_idle();
    app.tap("nav/maintenance");
    app.tap("maint/cache/run");
    assert_eq!(
        app.state().maint.get(&MaintCard::Cache),
        Some(&MaintPhase::Confirm)
    );
    assert!(world.exists("Cache"), "first press only asks");
    app.tap("maint/cache/confirm");
    app.wait_for("cache cleared", |s| {
        matches!(s.maint.get(&MaintCard::Cache), Some(MaintPhase::Done(_)))
    });
    assert!(app.shows("Freed 2 KB from Cache"));
    assert!(!world.exists("Cache"));
}

#[test]
fn f6_3_wtf_backup_is_zipped_into_the_app_data() {
    let world = World::new().with_key("test-key");
    write(
        &world.client.join("WTF/Config.wtf"),
        "SET locale \"enUS\"\n",
    );
    let mut app = world.app();
    app.wait_idle();
    app.tap("nav/maintenance");
    app.wait_until("the maintenance page", |a| a.has("maint/wtf/run"));
    assert!(app.shows("No backups yet."));
    app.tap("maint/wtf/run");
    app.tap("maint/wtf/confirm");
    app.wait_for("the backup", |s| s.backups.len() == 1);
    let name = app.state().backups[0].name.clone();
    assert!(app.shows(&format!("Saved {name}")));
    let id = &app.state().installation().unwrap().id;
    assert!(
        world
            .tmp
            .path()
            .join("data/wtf-backups")
            .join(id)
            .join(&name)
            .is_file()
    );
}

#[test]
fn f3_1_never_checked_addons_say_not_checked() {
    let world = World::new().with_wowup();
    // WowUp never saw a newer release for Ace: no `latest` until a check answers.
    let records = world.wowup_dirs[0].join("addons.json");
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&records).unwrap()).unwrap();
    for k in ["latestVersion", "externalLatestReleaseId"] {
        v["wowup-ace"].as_object_mut().unwrap().remove(k);
    }
    write(&records, v.to_string());
    let mut app = world.app();
    app.wait_idle();
    assert_eq!(app.status("Ace"), Status::NotChecked);
    assert!(app.shows_in(&app.row("Ace", ""), "Not checked"));
    assert!(!app.shows("Up to date") && !app.shows("Everything up to date"));
    assert!(app.shows("CurseForge updates not checked"));
}

#[test]
fn f5_1_remove_keeps_folders_another_addon_lists() {
    use addon_app::backend::Backend;
    let world = World::new().with_key("test-key");
    let backend = addon_app::core_backend::CoreBackend::new(world.config());
    let inst = backend.installations().unwrap().remove(0);
    let (addons, _) = backend.scan(&inst).unwrap();
    // A stale UI copy claims Homebrew too (it belongs to its own record).
    let mut ace = addons.iter().find(|a| a.name == "Ace").unwrap().clone();
    ace.folders.push("Homebrew".into());
    backend.remove(&inst, &ace).unwrap();
    assert!(!world.exists("Interface/AddOns/Ace"));
    assert!(
        world.exists("Interface/AddOns/Homebrew"),
        "only the stored record's own folders go"
    );
    let (after, _) = backend.scan(&inst).unwrap();
    assert!(after.iter().any(|a| a.name == "Homebrew") && !after.iter().any(|a| a.name == "Ace"));
}

/// A world where Ace 1.1 is out but its author allows downloads only through CurseForge's own app
/// or website (no downloadUrl, F3.5), with the zip the browser would fetch.
fn app_only_world() -> (World, Vec<u8>) {
    use sha1::{Digest, Sha1};
    let world = World::new().with_key("test-key");
    let zip = addon_testkit::make_zip(&[
        ("Ace/Ace.toc", toc("Ace", "1.1").as_bytes()),
        ("Ace/New.lua", b"1.1"),
    ]);
    let mut ace11 = addon_testkit::cf_file(11, "1.1", 1, &["Ace"], 0);
    ace11["downloadUrl"] = serde_json::Value::Null;
    ace11["fileLength"] = serde_json::json!(zip.len());
    let sha1: String = Sha1::digest(&zip)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    ace11["hashes"] = serde_json::json!([{ "value": sha1, "algo": 1 }]);
    let ace = addon_testkit::cf_mod_with_files(
        1,
        "Ace",
        vec![ace11, addon_testkit::cf_file(10, "1.0", 1, &["Ace"], 0)],
    );
    let bee_files = |id, v| addon_testkit::cf_file(id, v, 1, &["Bee", "Bee_Options"], 0);
    let bee = addon_testkit::cf_mod_with_files(
        2,
        "Bee",
        vec![bee_files(21, "2.1"), bee_files(20, "2.0")],
    );
    // Bee's update downloads from the fake server, like the default world's.
    let mods = addon_testkit::cf_envelope(serde_json::json!([ace, bee]))
        .to_string()
        .replace("https://edge.forgecdn.net", world.server.url());
    world
        .server
        .json("POST", "/v1/mods", &serde_json::from_str(&mods).unwrap());
    std::fs::create_dir_all(world.downloads()).unwrap();
    (world, zip)
}

fn awaiting(app: &App, name: &str) -> bool {
    app.state()
        .addons
        .iter()
        .any(|a| a.addon.name == name && matches!(a.op, Some(Op::AwaitingDownload { .. })))
}

/// Lets the app run `secs` seconds of its clock (it looks in Downloads once a second).
fn run(app: &mut App, secs: f64) {
    for _ in 0..(secs * 60.0) as usize {
        app.frame();
    }
}

#[test]
fn f3_5_update_only_on_curseforge_installs_the_browser_download() {
    let (world, zip) = app_only_world();
    let mut app = world.app();
    app.wait_idle();
    assert_eq!(app.status("Ace"), Status::AppOnly);
    assert!(app.shows_in(&app.row("Ace", ""), "Update available"));
    assert!(app.shows_in(&app.row("Ace", "open-source"), "Update"));
    assert!(app.shows_in(&app.row("Ace", ""), "1.0 → 1.1"));
    assert!(app.shows("Update all (1)"), "Update all leaves Ace out");

    app.tap(&app.row("Ace", "open-source"));
    assert!(awaiting(&app, "Ace"));
    app.wait_until("Culm to look in Downloads first", |app| {
        app.state()
            .addons
            .iter()
            .any(|a| matches!(a.op, Some(Op::AwaitingDownload { opened: true, .. })))
    });
    assert_eq!(
        app.rt.take_url_requests(),
        ["https://www.curseforge.com/wow/addons/ace/download/11"],
        "not there yet: opens the new file's download page"
    );
    assert!(app.shows_in(&app.row("Ace", ""), "Waiting for download"));

    // Unfinished, or not exactly the file: ignored.
    let downloads = world.downloads();
    let mut wrong = zip.clone();
    *wrong.last_mut().unwrap() ^= 1;
    std::fs::write(downloads.join("Ace-1.1.zip"), &wrong).unwrap();
    std::fs::write(downloads.join("Ace-1.1(1).zip.part"), &zip).unwrap();
    run(&mut app, 3.0);
    app.wait_until("the looks to finish", |app| awaiting(app, "Ace"));
    assert!(
        world
            .read("Interface/AddOns/Ace/Ace.toc")
            .contains("## Version: 1.0")
    );

    // The browser renames the finished download: Culm installs it and deletes the zip.
    std::fs::rename(
        downloads.join("Ace-1.1(1).zip.part"),
        downloads.join("Ace-1.1(1).zip"),
    )
    .unwrap();
    app.wait_until("Ace to update", |app| app.status("Ace") == Status::UpToDate);
    assert!(app.shows("Updated Ace"));
    assert!(
        world
            .read("Interface/AddOns/Ace/Ace.toc")
            .contains("## Version: 1.1")
    );
    assert!(world.exists("Interface/AddOns/Ace/New.lua"));
    assert!(
        !downloads.join("Ace-1.1(1).zip").exists(),
        "the zip is gone"
    );
    assert!(downloads.join("Ace-1.1.zip").exists(), "other files stay");
}

#[test]
fn f3_5_a_zip_already_in_downloads_installs_without_the_browser() {
    let (world, zip) = app_only_world();
    let downloads = world.downloads();
    std::fs::write(downloads.join("Ace-1.1.zip"), &zip).unwrap();
    std::fs::write(downloads.join("Ace-1.1(1).zip"), &zip).unwrap();
    std::fs::write(downloads.join("Notes.zip"), b"something else").unwrap();
    let mut app = world.app();
    app.wait_idle();
    app.tap(&app.row("Ace", "open-source"));
    app.wait_until("Ace to update", |app| app.status("Ace") == Status::UpToDate);
    assert!(
        app.rt.take_url_requests().is_empty(),
        "no browser tab: the file was already there"
    );
    assert!(
        world
            .read("Interface/AddOns/Ace/Ace.toc")
            .contains("## Version: 1.1")
    );
    assert!(!downloads.join("Ace-1.1.zip").exists());
    assert!(
        !downloads.join("Ace-1.1(1).zip").exists(),
        "identical copies go too"
    );
    assert!(downloads.join("Notes.zip").exists(), "other files stay");
}

#[test]
fn f3_5_waiting_can_be_cancelled_and_ends_by_itself() {
    let (world, _) = app_only_world();
    let mut app = world.app();
    app.wait_idle();
    app.tap(&app.row("Ace", "open-source"));
    assert!(awaiting(&app, "Ace"));
    app.tap(&app.row("Ace", "stop-waiting"));
    assert_eq!(app.status("Ace"), Status::AppOnly);

    app.tap(&app.row("Ace", "open-source"));
    assert!(awaiting(&app, "Ace"));
    app.rt.frame(601.0);
    app.frame();
    assert_eq!(
        app.status("Ace"),
        Status::AppOnly,
        "gave up after ten minutes"
    );
}

#[test]
fn f3_5_install_from_file_checks_the_zip_first() {
    let (world, zip) = app_only_world();
    let mut app = world.app();
    app.wait_idle();
    app.tap(&app.row("Ace", ""));
    app.wait_until("the detail panel", |app| {
        app.has("detail/install-from-file")
    });
    let elsewhere = world.downloads().join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let wrong = elsewhere.join("Ace-1.0.zip");
    std::fs::write(&wrong, b"not the update").unwrap();

    app.tap("detail/install-from-file");
    assert!(
        app.rt.file_dialog().is_some(),
        "a file dialog asks for the zip"
    );
    app.rt.resolve_file_dialog(Some(wrong.clone())).unwrap();
    app.frame();
    app.wait_idle();
    assert!(app.shows("this file is not Ace 1.1"));
    assert!(
        world
            .read("Interface/AddOns/Ace/Ace.toc")
            .contains("## Version: 1.0")
    );

    let right = elsewhere.join("Ace-1.1.zip");
    std::fs::write(&right, &zip).unwrap();
    app.tap("detail/install-from-file");
    app.rt.resolve_file_dialog(Some(right.clone())).unwrap();
    app.frame();
    app.wait_until("Ace to update", |app| app.status("Ace") == Status::UpToDate);
    assert!(
        world
            .read("Interface/AddOns/Ace/Ace.toc")
            .contains("## Version: 1.1")
    );
    assert!(right.exists(), "a file the user picked stays where it is");
}

#[test]
fn f2_7_addons_for_another_game_version_are_marked_out_of_date() {
    let world = World::new().with_key("test-key");
    let dir = world.addons_dir().join("Oldie");
    write(
        &dir.join("Oldie.toc"),
        "## Interface: 11509, 11508\n## Title: Oldie\n## Version: 3\n",
    );
    write(&dir.join("Core.lua"), "-- old\n");
    let mut app = world.app();
    app.wait_idle();
    let mark = app.row("Oldie", "out-of-date");
    assert!(app.shows_in(&mark, "Out of date"));
    assert!(
        !app.has(&app.row("Ace", "out-of-date")),
        "Ace lists the client's 16001"
    );
    let help = app
        .rt
        .elements()
        .iter()
        .find(|e| e.key.as_deref() == Some(mark.as_str()))
        .and_then(|e| e.help.clone())
        .unwrap_or_default();
    assert!(
        help.contains("1.15.9, 1.15.8") && help.contains("your game is 1.60.1"),
        "{help}"
    );
    app.tap(&app.row("Oldie", ""));
    assert!(app.shows_in("detail/out-of-date", "(out of date)"));
}
