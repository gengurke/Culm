//! End-to-end feature specs for installations, My Addons, updates, removal and maintenance:
//! the real UI and core over a temp Forever tree and a fake CurseForge (see `common`).

mod common;
use addon_app::state::{MaintCard, MaintPhase, Op, Status};
use addon_core::IgnoreReason;
use addon_testkit::{toc, write};
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
fn f1_2_installation_added_from_settings_becomes_current() {
    let world = World::new();
    let other = world.second_installation();
    let mut app = world.app();
    app.wait_idle();
    assert_eq!(app.names(), ["Ace", "Bee", "Homebrew"]);
    app.tap("nav/settings");
    app.reveal("settings/add-installation");
    app.tap("settings/add-installation");
    assert!(app.rt.file_dialog().is_some(), "folder picker requested");
    app.rt.resolve_file_dialog(Some(other)).unwrap();
    app.frame();
    app.wait_for("the new installation to be scanned", |s| {
        s.installations.len() == 2 && !s.addons.is_empty()
    });
    app.wait_idle();
    assert!(app.shows("Installation added"));
    assert_eq!(app.names(), ["Zed"]);

    // Settings lists both; Use switches back to the first.
    let first = app.state().installations[0].id.clone();
    let use_first = format!("settings/installation/{first}/use");
    app.reveal(&use_first);
    app.tap(&use_first);
    app.wait_for("the first installation's scan", |s| !s.addons.is_empty());
    app.wait_idle();
    assert_eq!(app.state().current.as_deref(), Some(first.as_str()));
    assert_eq!(app.names(), ["Ace", "Bee", "Homebrew"]);
}

#[test]
fn f1_3_sidebar_picker_switches_installations_and_keeps_the_choice() {
    let mut world = World::new();
    world.second_installation();
    world
        .search_roots
        .push(world.tmp.path().join("otherprefix"));
    let names_of = |app: &App, id: &str| {
        let inst = app.state().installations.iter().find(|i| i.id == id);
        if inst.unwrap().root.to_string_lossy().contains("otherprefix") {
            vec!["Zed"]
        } else {
            vec!["Ace", "Bee", "Homebrew"]
        }
    };
    let mut app = world.app();
    app.wait_idle();
    assert_eq!(app.state().installations.len(), 2);
    let current = app.state().current.clone().unwrap();
    assert_eq!(app.names(), names_of(&app, &current));
    let other = app
        .state()
        .installations
        .iter()
        .find(|i| i.id != current)
        .unwrap()
        .id
        .clone();

    app.tap("installation/selector");
    run(&mut app, 0.5); // the menu slides open
    assert!(app.shows("WoW Forever (testprefix)") && app.shows("WoW Forever (otherprefix)"));
    app.tap(&format!("installation/{other}"));
    app.wait_for("the other installation's scan", |s| !s.addons.is_empty());
    app.wait_idle();
    assert_eq!(app.state().current.as_deref(), Some(other.as_str()));
    assert_eq!(app.names(), names_of(&app, &other));

    let mut again = world.app();
    again.wait_idle();
    assert_eq!(
        again.state().current.as_deref(),
        Some(other.as_str()),
        "the choice is kept"
    );
    assert_eq!(again.names(), names_of(&again, &other));
}

#[test]
fn f2_4_import_from_wowup_in_settings_takes_over_its_links() {
    let world = World::new().with_wowup();
    // WowUp knew nothing yet at the first scan.
    let records = world.wowup_dirs[0].join("addons.json");
    let known = std::fs::read_to_string(&records).unwrap();
    write(&records, "{}");
    let mut app = world.app();
    app.wait_idle();
    assert_eq!(app.status("Ace"), Status::Unknown);

    write(&records, known);
    app.tap("nav/settings");
    app.reveal("settings/import-wowup");
    app.tap("settings/import-wowup");
    app.wait_for_text("Imported from WowUp");
    app.wait_idle();
    let ace = &app.state().addons[0].addon;
    assert_eq!(
        (ace.provider.as_deref(), ace.external_id.as_deref()),
        (Some("CurseForge"), Some("1"))
    );
    assert_eq!(app.status("Ace"), Status::UpdateAvailable);
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
fn f2_5_changes_made_just_before_quitting_are_stored() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    let (id, inst) = (app.id("Ace"), app.state().current.clone().unwrap());
    let data = world.config().data_dir;
    let stored = || {
        let list = addon_core::store::load_addons(&data, &inst).unwrap();
        list.into_iter().find(|a| a.id == id).unwrap()
    };
    // A long store edit (a remove, a scan merge) holds the writes back while the user toggles
    // Ignore three times and quits.
    let saves = app.state().saves.clone();
    let edit = saves.hold();
    app.tap(&app.row("Ace", ""));
    for _ in 0..3 {
        app.tap("detail/ignore");
    }
    assert_eq!(
        stored().ignored,
        None,
        "nothing written while the edit runs"
    );
    drop(edit);
    // What quitting runs: it writes the waiting saves in order before the app ends.
    app.state().flush_saves();
    assert_eq!(
        stored().ignored,
        Some(IgnoreReason::User),
        "the last toggle wins"
    );

    let mut again = world.app();
    again.wait_idle();
    assert_eq!(again.status("Ace"), Status::Ignored);
}

#[test]
fn f2_5_a_late_toggle_only_writes_the_users_choices() {
    // A toggle's save that waited behind an update must not put the old version and folders back.
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    let (inst, ace) = (
        app.state().installation().unwrap().clone(),
        app.state().addons[0].addon.clone(),
    );
    assert_eq!(ace.name, "Ace");
    let stale = addon_core::Addon {
        installed_version: Some("0.1".into()),
        folders: vec!["Gone".into()],
        ignored: Some(IgnoreReason::User),
        channel: addon_core::Channel::Beta,
        ..ace.clone()
    };
    app.state().backend.save_addon(&inst, &stale).unwrap();
    let list = addon_core::store::load_addons(&world.config().data_dir, &inst.id).unwrap();
    let saved = list.iter().find(|a| a.id == ace.id).unwrap();
    assert_eq!(
        (saved.ignored, saved.channel),
        (Some(IgnoreReason::User), addon_core::Channel::Beta)
    );
    assert_eq!(
        (&saved.installed_version, &saved.folders),
        (&ace.installed_version, &ace.folders)
    );
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
    // A worker writes the ignore: reopen only once it is stored, or the new app's own backend could
    // scan the list before the write lands (#133).
    let (id, inst) = (app.id("Ace"), app.state().current.clone().unwrap());
    let data = world.config().data_dir;
    app.wait_until("the ignore stored", |_| {
        addon_core::store::load_addons(&data, &inst)
            .is_ok_and(|list| list.iter().any(|a| a.id == id && a.ignored.is_some()))
    });

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
fn f5_2_reinstall_from_the_detail_panel_restores_modified_files() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    app.tap(&app.row("Ace", "update"));
    app.wait_idle();
    assert_eq!(app.status("Ace"), Status::UpToDate);
    write(&world.addons_dir().join("Ace/New.lua"), "edited by hand");
    std::fs::remove_file(world.addons_dir().join("Ace/Ace.toc")).unwrap();

    app.tap(&app.row("Ace", ""));
    app.tap("detail/reinstall");
    app.wait_idle();
    assert_eq!(world.read("Interface/AddOns/Ace/New.lua"), "1.1");
    assert!(
        world
            .read("Interface/AddOns/Ace/Ace.toc")
            .contains("## Version: 1.1")
    );
    assert_eq!(app.status("Ace"), Status::UpToDate);
}

#[test]
fn f6_1_clear_cache_deletes_the_cache_at_once() {
    let world = World::new().with_key("test-key");
    write(
        &world.client.join("Cache/WDB/enUS/creaturecache.wdb"),
        vec![0u8; 2048],
    );
    let mut app = world.app();
    app.wait_idle();
    app.tap("nav/maintenance");
    app.tap("maint/cache/run");
    assert!(!app.has("maint/cache/confirm"), "nothing to confirm");
    app.wait_for("cache cleared", |s| {
        matches!(s.maint.get(&MaintCard::Cache), Some(MaintPhase::Done(_)))
    });
    assert!(app.shows("Freed 2 KB from Cache"));
    assert!(!world.exists("Cache"));
}

#[test]
fn f6_2_clean_backups_asks_then_deletes_the_leftovers() {
    let world = World::new().with_key("test-key");
    let saved = world.client.join("WTF/Account/ACC/SavedVariables");
    write(&saved.join("Ace.lua"), "keep");
    write(&saved.join("Ace.lua.bak"), "1234");
    write(&world.client.join("WTF/config-cache.old"), "1");
    let mut app = world.app();
    app.wait_idle();
    app.tap("nav/maintenance");
    app.wait_until("the maintenance page", |a| a.has("maint/leftovers/run"));
    app.tap("maint/leftovers/run");
    assert_eq!(
        app.state().maint.get(&MaintCard::Leftovers),
        Some(&MaintPhase::Confirm)
    );
    assert!(
        world.exists("WTF/config-cache.old"),
        "first press only asks"
    );
    app.tap("maint/leftovers/confirm");
    app.wait_for("leftovers removed", |s| {
        matches!(
            s.maint.get(&MaintCard::Leftovers),
            Some(MaintPhase::Done(_))
        )
    });
    assert!(app.shows("Removed 2 .bak and .old files"));
    assert!(!world.exists("WTF/config-cache.old"));
    assert!(!world.exists("WTF/Account/ACC/SavedVariables/Ace.lua.bak"));
    assert!(world.exists("WTF/Account/ACC/SavedVariables/Ace.lua"));
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
    assert!(!app.has("maint/wtf/confirm"), "nothing to confirm");
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

/// Culm died mid-update: `folder` sits in the swap's backup next to AddOns.
fn interrupted(world: &World, folder: &str, tag: &str) -> std::path::PathBuf {
    let backup = world.client.join(format!("Interface/.addons-backup-{tag}"));
    std::fs::create_dir_all(&backup).unwrap();
    std::fs::rename(world.addons_dir().join(folder), backup.join(folder)).unwrap();
    backup
}

/// The next start, without a key: links can't come back from CurseForge.
fn restart_without_key(world: &World) -> addon_app::core_backend::CoreBackend {
    rune::prefs::Prefs::at(world.prefs_path())
        .remove("curseforge_key")
        .unwrap();
    addon_app::core_backend::CoreBackend::new(world.config())
}

#[test]
fn f3_2_scan_puts_back_folders_of_an_interrupted_install() {
    use addon_app::backend::Backend;
    let world = World::new().with_key("test-key");
    let backend = addon_app::core_backend::CoreBackend::new(world.config());
    let inst = backend.installations().unwrap().remove(0);
    let (before, _) = backend.scan(&inst).unwrap();
    let ace = before.iter().find(|a| a.name == "Ace").unwrap().clone();
    assert!(ace.provider.is_some());
    let backup = interrupted(&world, "Ace", "1");

    let (after, warnings) = restart_without_key(&world).scan(&inst).unwrap();
    assert!(world.exists("Interface/AddOns/Ace/Ace.toc"), "put back");
    assert!(!backup.exists(), "backup cleaned up");
    let a = after.iter().find(|a| a.id == ace.id).expect("record kept");
    assert_eq!(a.provider, ace.provider, "still linked: {warnings:?}");
}

#[cfg(unix)]
#[test]
fn f3_2_scan_keeps_the_link_of_folders_it_cannot_put_back() {
    use addon_app::backend::Backend;
    use std::os::unix::fs::PermissionsExt;
    let world = World::new().with_key("test-key");
    let backend = addon_app::core_backend::CoreBackend::new(world.config());
    let inst = backend.installations().unwrap().remove(0);
    let (before, _) = backend.scan(&inst).unwrap();
    let bee = before.iter().find(|a| a.name == "Bee").unwrap().clone();
    assert!(bee.provider.is_some());
    let backup = interrupted(&world, "Bee", "2");
    let backend = restart_without_key(&world);

    let mode = |m| std::fs::set_permissions(world.addons_dir(), std::fs::Permissions::from_mode(m));
    mode(0o555).unwrap();
    if std::fs::File::create(world.addons_dir().join("probe")).is_ok() {
        mode(0o755).unwrap();
        eprintln!("skipped: running as root, AddOns stays writable");
        return;
    }
    let res = backend.scan(&inst);
    mode(0o755).unwrap();
    let (after, warnings) = res.unwrap();
    assert!(
        warnings.iter().any(|w| w.contains("putting back")),
        "{warnings:?}"
    );
    assert!(
        backup.join("Bee/Bee.toc").is_file(),
        "kept for the next scan"
    );
    let b = after.iter().find(|a| a.id == bee.id).expect("record kept");
    assert_eq!(b.provider, bee.provider);
    assert_eq!(b.folders, bee.folders);

    let (after, _) = backend.scan(&inst).unwrap();
    assert!(
        world.exists("Interface/AddOns/Bee/Bee.toc"),
        "back once it can be"
    );
    assert_eq!(
        after.iter().find(|a| a.id == bee.id).unwrap().provider,
        bee.provider
    );
}
