mod common;
use addon_core::providers::Provider;
use addon_core::{Channel, IgnoreReason, fingerprint, scan, store};
use addon_testkit::{toc, write};
use common::*;

#[test]
fn f2_1_scan_picks_forever_toc() {
    let w = fake_wow();
    let q = w.addons().join("Questie");
    write(&q.join("Questie.toc"), fixture_toc("Questie.toc"));
    write(
        &q.join("Questie_Forever.toc"),
        fixture_toc("Questie_Forever.toc"),
    );
    write(
        &q.join("Questie_Mists.toc"),
        "## Interface: 50504\n## Title: Questie Mists\n",
    );
    let d = w.addons().join("Details");
    write(
        &d.join("Details_Camelot.toc"),
        fixture_toc("Details_Camelot.toc"),
    );
    write(
        &d.join("Details.toc"),
        "## Interface: 120105\n## Title: Details retail\n",
    );
    w.add_folder(
        "Plain",
        "Plain.toc",
        &toc("|cff00ff00Plain|r Addon", "1.2"),
        &[("Core.lua", "x")],
    );
    std::fs::create_dir_all(w.addons().join("NoToc/media")).unwrap();

    let (folders, _) = scan::scan_folders(&w.inst).unwrap();
    let names: Vec<&str> = folders.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["Details", "Plain", "Questie"]);

    let toc = |n: &str| {
        folders
            .iter()
            .find(|f| f.name == n)
            .unwrap()
            .toc
            .clone()
            .unwrap()
    };
    let q = toc("Questie");
    assert_eq!(q.file_name, "Questie_Forever.toc");
    assert_eq!(q.title.as_deref(), Some("Questie v12.9.0"));
    assert_eq!(q.version.as_deref(), Some("12.9.0"));
    assert_eq!(q.forever_game_version().as_deref(), Some("1.60.1"));
    let d = toc("Details");
    assert_eq!(d.file_name, "Details_Camelot.toc");
    assert_eq!(d.curse_project_id.as_deref(), Some("61284"));
    assert_eq!(d.wago_id.as_deref(), Some("qv63A7Gb"));
    assert_eq!(toc("Plain").title.as_deref(), Some("Plain Addon"));
    let ids = folders[0].scanned().toc_ids;
    assert!(ids.contains(&("CurseForge".into(), "61284".into())));
}

/// Golden fixture: CurseForge fingerprint 3501589137, md5
/// df5900e9e7c37399f8a51f67495b9317, 5 files.
#[test]
fn f2_3_fingerprint_matches_golden_value() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("AddOns/FixtureAddon");
    write(
        &dir.join("FixtureAddon.toc"),
        [
            "## Interface: 110000",
            "# a comment",
            "Core.lua",
            "Modules\\Panel.xml",
            "",
            "Missing.lua",
        ]
        .join("\r\n"),
    );
    write(&dir.join("Core.lua"), "-- core\nlocal a = 1\n");
    write(
        &dir.join("Modules/Panel.xml"),
        r#"<Ui><!-- skip --><Script file="Panel.lua"/></Ui>"#,
    );
    write(&dir.join("Modules/Panel.lua"), "-- panel\n");
    write(&dir.join("Bindings.xml"), "<Bindings/>");
    write(&dir.join("README.md"), "ignored");
    assert_eq!(fingerprint::fingerprint_files(&dir).unwrap().len(), 5);
    assert_eq!(
        fingerprint::folder_fingerprints(&dir).unwrap().0,
        3501589137
    );
    // The Hub's md5 fingerprint of the same fixture.
    assert_eq!(
        fingerprint::folder_fingerprints(&dir).unwrap().1,
        "df5900e9e7c37399f8a51f67495b9317"
    );
}

#[test]
fn f2_3_fingerprint_follows_flavor_tocs_case_insensitively_and_skips_media() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Foo");
    write(
        &dir.join("Foo_Camelot.toc"),
        "## Interface: 16001\nlibs\\LIBS.XML\nmain.lua\n",
    );
    write(&dir.join("Foo-deDE.toc"), "de.lua\n"); // not a root
    write(
        &dir.join("Libs/libs.xml"),
        "<Ui>\n<Include file=\"Lib\\lib.xml\"/>\n</Ui>",
    );
    write(
        &dir.join("Libs/Lib/lib.xml"),
        "<Ui><Script file='lib.lua'/></Ui>",
    );
    write(&dir.join("Libs/Lib/lib.lua"), "x");
    write(&dir.join("Main.lua"), "y");
    write(&dir.join("de.lua"), "z");
    write(&dir.join("sound.ogg"), "media");
    let files: Vec<String> = fingerprint::fingerprint_files(&dir)
        .unwrap()
        .iter()
        .map(|p| p.strip_prefix(&dir).unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        files,
        [
            "Foo_Camelot.toc",
            "Libs/Lib/lib.lua",
            "Libs/Lib/lib.xml",
            "Libs/libs.xml",
            "Main.lua"
        ]
    );
}

fn grouped_fixture() -> (FakeWow, Vec<Box<dyn Provider>>) {
    let w = fake_wow();
    w.add_folder(
        "Details",
        "Details.toc",
        &toc("Details!", "1"),
        &[("Core.lua", "a")],
    );
    w.add_folder(
        "Details_TinyThreat",
        "Details_TinyThreat.toc",
        "## Title: Tiny\n## RequiredDeps: Details\n",
        &[],
    );
    w.add_folder("Moonlit", "Moonlit.toc", &toc("Moonlit", "2"), &[]);
    w.add_folder(
        "Moonlit_Codex",
        "Moonlit_Codex.toc",
        "## Dependencies: Moonlit\n",
        &[],
    );
    w.add_folder(
        "Moonlit_Guides",
        "Moonlit_Guides.toc",
        "## Dependencies: Moonlit_Codex, Moonlit\n",
        &[],
    );
    w.add_folder(
        "Lonely",
        "Lonely.toc",
        "## Title: 123\n## Dependencies: Moonlit\n",
        &[],
    );
    w.add_folder("DevAddon", "DevAddon.toc", &toc("Dev", "dev"), &[]);
    std::fs::create_dir_all(w.addons().join("DevAddon/.git")).unwrap();
    // A folder with only a retail TOC, listed in the provider file of Details.
    w.add_folder(
        "Details_Retail",
        "Details_Retail_Mainline.toc",
        "## Title: R\n",
        &[],
    );
    let file = remote_file(
        "1001",
        "v1",
        Channel::Beta,
        &["Details", "Details_TinyThreat", "Details_Retail"],
    );
    let details = remote_addon("61284", "Details! Damage Meter", vec![file.clone()]);
    let providers: Vec<Box<dyn Provider>> = vec![Box::new(FakeProvider {
        matches: vec![
            ("Details".into(), details.clone(), Some(file.clone())),
            ("Details_TinyThreat".into(), details, Some(file)),
        ],
        fail: false,
    })];
    (w, providers)
}

#[test]
fn f2_2_groups_by_provider_and_dependencies() {
    let (w, providers) = grouped_fixture();
    let res = scan::scan_installation(&w.inst, &providers).unwrap();
    assert!(res.errors.is_empty());
    let by_name = |n: &str| {
        res.addons
            .iter()
            .find(|a| a.name == n)
            .unwrap_or_else(|| panic!("{n}: {:#?}", res.addons))
    };

    let d = by_name("Details! Damage Meter");
    assert_eq!(
        d.folders,
        ["Details", "Details_Retail", "Details_TinyThreat"]
    );
    assert_eq!(d.provider.as_deref(), Some("Fake"));
    assert_eq!(d.installed_file_id.as_deref(), Some("1001"));
    assert_eq!(d.channel, Channel::Beta);
    assert_eq!(d.game_version.as_deref(), Some("1.60.1"));
    assert_eq!(d.ignored, None);

    let p = by_name("Moonlit");
    assert_eq!(p.folders, ["Moonlit", "Moonlit_Codex", "Moonlit_Guides"]);
    assert_eq!(p.ignored, Some(IgnoreReason::Unknown));
    assert_eq!(p.installed_version.as_deref(), Some("2"));
    // Depends on Moonlit but is not prefixed by it: stays alone, named after its folder.
    assert_eq!(by_name("Lonely").folders, ["Lonely"]);
    assert_eq!(by_name("Dev").ignored, Some(IgnoreReason::GitRepo));
    assert_eq!(res.addons.len(), 4);
}

#[test]
fn f2_3_unmatched_are_unknown_and_provider_errors_are_reported() {
    let w = fake_wow();
    w.add_folder("A", "A.toc", &toc("A", "1"), &[]);
    let providers: Vec<Box<dyn Provider>> = vec![Box::new(FakeProvider {
        matches: vec![],
        fail: true,
    })];
    let res = scan::scan_installation(&w.inst, &providers).unwrap();
    assert_eq!(res.errors, ["Fake: offline"]);
    assert_eq!(res.addons[0].ignored, Some(IgnoreReason::Unknown));
    assert_eq!(res.addons[0].provider, None);
}

#[test]
fn f2_5_store_roundtrip_and_rescan_keeps_user_choices() {
    let (w, providers) = grouped_fixture();
    let data = tempfile::tempdir().unwrap();
    let first = scan::scan_installation(&w.inst, &providers).unwrap().addons;
    let mut saved = store::merge_rescan(&[], first, &[]);
    let d = saved
        .iter_mut()
        .find(|a| a.external_id.as_deref() == Some("61284"))
        .unwrap();
    d.channel = Channel::Alpha;
    d.auto_update = true;
    d.ignored = Some(IgnoreReason::User);
    let details_id = d.id.clone();
    let moonlit_id = saved
        .iter()
        .find(|a| a.name == "Moonlit")
        .unwrap()
        .id
        .clone();
    store::save_addons(data.path(), &w.inst.id, &saved).unwrap();
    assert!(
        data.path()
            .join("addons")
            .join(format!("{}.json", w.inst.id))
            .is_file()
    );

    // Rescan online: user choices and ids survive.
    let previous = store::load_addons(data.path(), &w.inst.id).unwrap();
    assert_eq!(previous, saved);
    let fresh = scan::scan_installation(&w.inst, &providers).unwrap().addons;
    let merged = store::merge_rescan(&previous, fresh, &[]);
    let d = merged.iter().find(|a| a.id == details_id).unwrap();
    assert_eq!(
        (d.channel, d.auto_update, d.ignored),
        (Channel::Alpha, true, Some(IgnoreReason::User))
    );
    assert_eq!(
        merged.iter().find(|a| a.name == "Moonlit").unwrap().id,
        moonlit_id
    );

    // Rescan offline: the provider link is kept instead of degrading to Unknown.
    let offline: Vec<Box<dyn Provider>> = vec![Box::new(FakeProvider {
        matches: vec![],
        fail: true,
    })];
    let fresh = scan::scan_installation(&w.inst, &offline).unwrap().addons;
    let merged = store::merge_rescan(&previous, fresh, &[]);
    assert_eq!(merged.len(), 4, "{merged:#?}");
    let d = merged.iter().find(|a| a.id == details_id).unwrap();
    assert_eq!(d.provider.as_deref(), Some("Fake"));

    // Folders deleted by hand: the record goes away.
    std::fs::remove_dir_all(w.addons().join("Details_TinyThreat")).unwrap();
    let fresh = scan::scan_installation(&w.inst, &offline).unwrap().addons;
    let merged = store::merge_rescan(&previous, fresh, &[]);
    assert!(merged.iter().all(|a| a.id != details_id));
    assert!(
        merged
            .iter()
            .any(|a| a.folders == ["Details"] && a.provider.is_none())
    );
}

#[test]
fn f2_7_out_of_date_when_the_newest_interface_number_is_older_than_the_client() {
    let w = fake_wow();
    let toc = |interface: &str| format!("## Interface: {interface}\n## Title: T\n");
    w.add_folder("Fits", "Fits.toc", &toc("16001"), &[]);
    w.add_folder("Listed", "Listed.toc", &toc("11509, 16001"), &[]);
    w.add_folder("Newer", "Newer.toc", &toc("16100"), &[]);
    w.add_folder("Older", "Older.toc", &toc("16000"), &[]);
    w.add_folder("Classic", "Classic.toc", &toc("11509, 11508"), &[]);
    w.add_folder("NoNumber", "NoNumber.toc", "## Title: T\n", &[]);
    let res = scan::scan_installation(&w.inst, &[]).unwrap();
    let client = w.inst.version.as_deref();
    assert_eq!(client, Some("1.60.1.70938"), "read from .build.info");
    let old = |folder: &str| {
        let a = res.addons.iter().find(|a| a.folders == [folder]).unwrap();
        a.out_of_date(client)
    };
    assert!(!old("Fits") && !old("Listed"));
    assert!(!old("Newer"), "a newer number than the client's fits");
    assert!(old("Older") && old("Classic"));
    assert!(!old("NoNumber"), "no interface number: not judged");
    let classic = res
        .addons
        .iter()
        .find(|a| a.folders == ["Classic"])
        .unwrap();
    assert_eq!(classic.game_versions, ["1.15.9", "1.15.8"]);
    assert!(
        !classic.out_of_date(None),
        "unknown client version: not judged"
    );
    assert!(!classic.out_of_date(Some("1.15.9.1234")));
}

#[cfg(unix)]
#[test]
fn f2_1_scan_skips_an_unreadable_folder_and_keeps_its_record() {
    use std::os::unix::fs::PermissionsExt;
    let (w, providers) = grouped_fixture();
    let first = scan::scan_installation(&w.inst, &providers).unwrap().addons;
    let mut previous = store::merge_rescan(&[], first, &[]);
    let lonely = previous
        .iter_mut()
        .find(|a| a.folders == ["Lonely"])
        .unwrap();
    lonely.ignored = Some(IgnoreReason::User);
    let lonely = lonely.clone();
    let details = previous
        .iter()
        .find(|a| a.external_id.as_deref() == Some("61284"))
        .unwrap()
        .clone();

    let locked = ["Details_TinyThreat", "Lonely"].map(|f| w.addons().join(f));
    let set = |mode| {
        for p in &locked {
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)).unwrap();
        }
    };
    set(0o000);
    if std::fs::read_dir(&locked[0]).is_ok() {
        set(0o755);
        eprintln!("skipped: running as root, folders stay readable");
        return;
    }
    let offline: Vec<Box<dyn Provider>> = vec![Box::new(FakeProvider {
        matches: vec![],
        fail: true,
    })];
    let res = scan::scan_installation(&w.inst, &offline);
    set(0o755);
    let res = res.unwrap();
    assert_eq!(res.unreadable, ["Details_TinyThreat", "Lonely"]);
    assert!(
        res.errors
            .iter()
            .any(|e| e.contains("Details_TinyThreat") && e.contains("ermission denied")),
        "{:?}",
        res.errors
    );
    assert!(
        res.addons
            .iter()
            .any(|a| a.folders == ["Moonlit", "Moonlit_Codex", "Moonlit_Guides"])
    );

    // A folder the scan couldn't read is not gone: its records keep their link and choices.
    let merged = store::merge_rescan(&previous, res.addons, &res.unreadable);
    let d = merged.iter().find(|a| a.id == details.id).unwrap();
    assert_eq!(d.provider.as_deref(), Some("Fake"));
    assert_eq!(d.folders, details.folders);
    assert_eq!(merged.iter().find(|a| a.id == lonely.id), Some(&lonely));
}
