mod common;
use addon_core::providers::Provider;
use addon_core::{Channel, IgnoreReason, fingerprint, scan, store};
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
        &simple_toc("|cff00ff00Plain|r Addon", "1.2"),
        &[("Core.lua", "x")],
    );
    std::fs::create_dir_all(w.addons().join("NoToc/media")).unwrap();

    let folders = scan::scan_folders(&w.inst).unwrap();
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
    assert!(
        d.saved_variables
            .contains(&"_detalhes_database".to_string())
    );
    assert!(d.files.contains(&"core/parser_nocleu.xml".to_string()));
    assert_eq!(toc("Plain").title.as_deref(), Some("Plain Addon"));
    let ids = folders[0].scanned().toc_ids;
    assert!(ids.contains(&("CurseForge".into(), "61284".into())));
}

/// Golden fixture: CurseForge fingerprint 3501589137, 5 files.
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
    assert_eq!(fingerprint::folder_fingerprint(&dir).unwrap(), 3501589137);
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
        &simple_toc("Details!", "1"),
        &[("Core.lua", "a")],
    );
    w.add_folder(
        "Details_TinyThreat",
        "Details_TinyThreat.toc",
        "## Title: Tiny\n## RequiredDeps: Details\n",
        &[],
    );
    w.add_folder(
        "Moonlit",
        "Moonlit.toc",
        &simple_toc("Moonlit", "2"),
        &[],
    );
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
    w.add_folder("DevAddon", "DevAddon.toc", &simple_toc("Dev", "dev"), &[]);
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
    w.add_folder("A", "A.toc", &simple_toc("A", "1"), &[]);
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
    let mut saved = store::merge_rescan(&[], first);
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
    let merged = store::merge_rescan(&previous, fresh);
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
    let merged = store::merge_rescan(&previous, fresh);
    assert_eq!(merged.len(), 4, "{merged:#?}");
    let d = merged.iter().find(|a| a.id == details_id).unwrap();
    assert_eq!(d.provider.as_deref(), Some("Fake"));

    // Folders deleted by hand: the record goes away.
    std::fs::remove_dir_all(w.addons().join("Details_TinyThreat")).unwrap();
    let fresh = scan::scan_installation(&w.inst, &offline).unwrap().addons;
    let merged = store::merge_rescan(&previous, fresh);
    assert!(merged.iter().all(|a| a.id != details_id));
    assert!(
        merged
            .iter()
            .any(|a| a.folders == ["Details"] && a.provider.is_none())
    );
}
