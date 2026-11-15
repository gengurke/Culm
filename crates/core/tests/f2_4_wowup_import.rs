mod common;
use addon_core::{Channel, DependencyKind, IgnoreReason, scan, store, wowup_import};
use common::*;

#[test]
fn f2_4_imports_wowup_records() {
    let w = fake_wow();
    w.add_folder("Details", "Details.toc", &simple_toc("Details!", "1"), &[]);
    w.add_folder(
        "Details_TinyThreat",
        "Details_TinyThreat.toc",
        "## RequiredDeps: Details\n",
        &[],
    );
    w.add_folder("Questie", "Questie.toc", &simple_toc("Questie", "1"), &[]);
    w.add_folder("Local", "Local.toc", &simple_toc("Local", "1"), &[]);

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
    let merged = store::merge_rescan(&imported, fresh);
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
        w.add_folder(f, &format!("{f}.toc"), &simple_toc(f, "1"), &[]);
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
    let merged = store::merge_rescan(&imported, fresh);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].folders, ["Questie"]);
}
