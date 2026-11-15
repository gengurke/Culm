//! End-to-end feature specs for installations, My Addons, updates, removal and maintenance:
//! the real UI and core over a temp Forever tree and a fake CurseForge (see `common`).

mod common;
use addon_app::state::{MaintCard, MaintPhase, Status};
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
        app.shows("WoW Forever (testprefix)"),
        "installation picker names the Wine prefix"
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
            .join("cache/thumbnails")
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
    assert!(!app.shows("Up to date") && !app.shows("everything up to date"));
    assert!(app.shows("CurseForge updates not checked — add a CurseForge key"));
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
