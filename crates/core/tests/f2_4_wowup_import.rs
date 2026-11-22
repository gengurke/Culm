mod common;
use addon_core::{Channel, DependencyKind, IgnoreReason, scan, store, wowup_import};
use addon_testkit::{toc, write};
use common::*;

#[test]
fn f2_4_imports_wowup_records() {
    let w = fake_wow();
    w.add_folder("Details", "Details.toc", &toc("Details!", "1"), &[]);
    w.add_folder(
        "Details_TinyThreat",
        "Details_TinyThreat.toc",
        "## RequiredDeps: Details\n",
        &[],
    );
    w.add_folder("Questie", "Questie.toc", &toc("Questie", "1"), &[]);
    w.add_folder("Local", "Local.toc", &toc("Local", "1"), &[]);

    let wowup = w.tmp.path().join("WowUpCf");
    let client = w.inst.root.display().to_string();
    let retail = w.wow_root.join("_retail_").display().to_string();
    let prefs = serde_json::json!({
        "wow_installations": [
            {"id": "ret", "clientType": 0, "location": format!("{retail}/Wow.exe")},
            {"id": "fev", "clientType": 10, "location": format!("{client}/Wow.exe")}
        ]
    });
    let rec = |id: &str, name: &str, folders: &[&str], inst: &str, provider: &str| {
        serde_json::json!({
            "id": id, "name": name, "author": "A", "providerName": provider, "externalId": format!("ext-{id}"),
            "installedVersion": "1.0", "installedExternalReleaseId": "100", "installedFolderList": folders,
            "clientType": 10, "installationId": inst, "channelType": 1, "isIgnored": false,
            "externalUrl": "https://www.curseforge.com/wow/addons/x", "thumbnailUrl": "https://t/x.png",
            "summary": "S", "gameVersion": ["12.0.0", "1.60.1"], "latestVersion": "2.0",
            "externalLatestReleaseId": "200", "externalChannel": "Beta", "autoUpdateEnabled": true,
            "dependencies": [{"externalAddonId": "14320", "type": 2}]
        })
    };
    let addons = serde_json::json!({
        "u1": rec("u1", "Details! Damage Meter", &["Details", "Details_TinyThreat"], "fev", "Curse"),
        "u2": rec("u2", "Questie", &["Questie"], "fev", "Curse"),
        "u3": rec("u3", "Gone", &["Gone"], "fev", "Curse"),
        "u4": rec("u4", "Retail", &["Details"], "ret", "Curse"),
        "u5": rec("u5", "Unknown", &["Local"], "fev", "Unknown")
    });
    write(&wowup.join("preferences.json"), prefs.to_string());
    write(&wowup.join("addons.json"), addons.to_string());

    let imported = wowup_import::import_wowup(&wowup, &w.inst).unwrap();
    let names: Vec<&str> = imported.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Details! Damage Meter", "Questie"]);
    let d = &imported[0];
    assert_eq!(d.id, "u1");
    assert_eq!(d.provider.as_deref(), Some("CurseForge"));
    assert_eq!(d.external_id.as_deref(), Some("ext-u1"));
    assert_eq!(d.folders, ["Details", "Details_TinyThreat"]);
    assert_eq!(d.installed_file_id.as_deref(), Some("100"));
    assert_eq!(d.game_version.as_deref(), Some("1.60.1"));
    assert_eq!(d.channel, Channel::Beta);
    assert!(d.auto_update);
    assert_eq!(d.ignored, None::<IgnoreReason>);
    assert_eq!(
        d.latest.as_ref().map(|l| (l.id.as_str(), l.channel)),
        Some(("200", Channel::Beta))
    );
    assert!(d.update_available());
    assert_eq!(d.dependencies[0].kind, DependencyKind::Required);

    // Imported links survive a rescan without any provider (no API key yet).
    let fresh = scan::scan_installation(&w.inst, &[]).unwrap().addons;
    let merged = store::merge_rescan(&imported, fresh, &[]);
    let names: Vec<(&str, bool)> = merged
        .iter()
        .map(|a| (a.name.as_str(), a.provider.is_some()))
        .collect();
    assert_eq!(
        names,
        [
            ("Details! Damage Meter", true),
            ("Local", false),
            ("Questie", true)
        ]
    );
}

#[test]
fn f2_4_overlapping_wowup_records_get_one_owner_per_folder() {
    let w = fake_wow();
    for f in ["Questie", "QuestieTrace"] {
        w.add_folder(f, &format!("{f}.toc"), &toc(f, "1"), &[]);
    }
    let wowup = w.tmp.path().join("WowUpCf");
    let client = w.inst.root.display().to_string();
    let prefs = serde_json::json!({
        "wow_installations": [{"id": "fev", "location": format!("{client}/Wow.exe")}]
    });
    let rec = |id: &str, name: &str, folders: &[&str]| {
        serde_json::json!({
            "id": id, "name": name, "providerName": "Curse", "externalId": id,
            "installedFolderList": folders, "installationId": "fev"
        })
    };
    // WowUp lists QuestieTrace under Questie and as its own addon.
    let addons = serde_json::json!({
        "q": rec("q", "Questie", &["Questie", "QuestieTrace"]),
        "t": rec("t", "QuestieTrace", &["QuestieTrace"]),
    });
    write(&wowup.join("preferences.json"), prefs.to_string());
    write(&wowup.join("addons.json"), addons.to_string());

    let imported = wowup_import::import_wowup(&wowup, &w.inst).unwrap();
    let folders: Vec<(&str, Vec<String>)> = imported
        .iter()
        .map(|a| (a.name.as_str(), a.folders.clone()))
        .collect();
    assert_eq!(
        folders,
        [
            ("Questie", vec!["Questie".to_string()]),
            ("QuestieTrace", vec!["QuestieTrace".to_string()])
        ]
    );
    // Stays that way through a rescan merge, and removing Questie won't delete QuestieTrace.
    let fresh = scan::scan_installation(&w.inst, &[]).unwrap().addons;
    let merged = store::merge_rescan(&imported, fresh, &[]);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].folders, ["Questie"]);
}

#[test]
fn f2_4_wowup_link_is_not_taken_over_by_another_providers_toc_match() {
    let w = fake_wow();
    for f in ["EnhanceQoL", "EnhanceQoLBags"] {
        w.add_folder(f, &format!("{f}.toc"), &toc(f, "13.21.3"), &[]);
    }
    let wowup = w.tmp.path().join("WowUpCf");
    let client = w.inst.root.display().to_string();
    let prefs = serde_json::json!({
        "wow_installations": [{"id": "fev", "location": format!("{client}/Wow.exe")}]
    });
    let addons = serde_json::json!({
        "e": {
            "id": "e", "name": "Enhance QoL", "providerName": "Curse", "externalId": "1076354",
            "installedVersion": "13.21.3-forever", "installedFolderList": ["EnhanceQoL", "EnhanceQoLBags"],
            "installationId": "fev"
        }
    });
    write(&wowup.join("preferences.json"), prefs.to_string());
    write(&wowup.join("addons.json"), addons.to_string());
    let imported = wowup_import::import_wowup(&wowup, &w.inst).unwrap();

    // Without a CurseForge key another provider matches the main folder by its TOC id
    // (on the real install: WowInterface, whose file is not the Forever build).
    let other = remote_addon(
        "26888",
        "Enhance QoL",
        vec![remote_file("26888-1", "13.21.3", Channel::Stable, &[])],
    );
    let providers: Vec<Box<dyn addon_core::providers::Provider>> = vec![Box::new(FakeProvider {
        matches: vec![("EnhanceQoL".into(), other, None)],
        fail: false,
    })];
    let fresh = scan::scan_installation(&w.inst, &providers).unwrap().addons;
    let merged = store::merge_rescan(&imported, fresh, &[]);
    assert_eq!(merged.len(), 1, "{merged:#?}");
    let e = &merged[0];
    assert_eq!(
        (e.provider.as_deref(), e.external_id.as_deref()),
        (Some("CurseForge"), Some("1076354"))
    );
    assert_eq!(e.folders, ["EnhanceQoL", "EnhanceQoLBags"]);
    assert_eq!(e.installed_version.as_deref(), Some("13.21.3-forever"));
}

#[test]
fn f2_4_zip_file_name_versions_show_the_toc_version() {
    let w = fake_wow();
    w.add_folder("Tint", "Tint.toc", &toc("Tint", "0.7.3"), &[]);
    w.add_folder("NoVer", "NoVer.toc", "## Title: NoVer\n", &[]);
    let wowup = w.tmp.path().join("WowUpCf");
    let client = w.inst.root.display().to_string();
    let prefs = serde_json::json!({
        "wow_installations": [{"id": "fev", "location": format!("{client}/Wow.exe")}]
    });
    let rec = |id: &str, folder: &str, version: &str, latest: (&str, &str)| {
        serde_json::json!({
            "id": id, "name": folder, "providerName": "Curse", "externalId": id,
            "installedFolderList": [folder], "installationId": "fev",
            "installedVersion": version, "installedExternalReleaseId": "9",
            "latestVersion": latest.1, "externalLatestReleaseId": latest.0
        })
    };
    let addons = serde_json::json!({
        "t": rec("t", "Tint", "ForeverQuestTint-0.7.3.zip", ("10", "ForeverQuestTint-0.7.4.zip")),
        "n": rec("n", "NoVer", "NoVer-1.2.ZIP", ("9", "NoVer-1.2.ZIP")),
    });
    write(&wowup.join("preferences.json"), prefs.to_string());
    write(&wowup.join("addons.json"), addons.to_string());
    let imported = wowup_import::import_wowup(&wowup, &w.inst).unwrap();
    let v = |i: usize| {
        let a = &imported[i];
        (
            a.installed_version.clone().unwrap(),
            a.latest.as_ref().map(|l| l.version.clone()),
            a.update_available(),
        )
    };
    // WowUp's "latest = installed" is dropped: we haven't checked it ourselves.
    assert_eq!(v(0), ("NoVer-1.2".into(), None, false));
    assert_eq!(
        v(1),
        ("0.7.3".into(), Some("ForeverQuestTint-0.7.4".into()), true)
    );
}
