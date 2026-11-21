mod common;
use addon_core::{Channel, install, store};
use common::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// Serves zips by URL and records requests.
struct Server {
    zips: HashMap<String, Vec<u8>>,
    hits: RefCell<Vec<String>>,
}

impl Server {
    fn new(entries: Vec<(&addon_core::RemoteFile, Vec<u8>)>) -> Self {
        Server {
            zips: entries
                .into_iter()
                .map(|(f, z)| (f.download_url.clone(), z))
                .collect(),
            hits: RefCell::new(vec![]),
        }
    }
    fn get(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        self.hits.borrow_mut().push(url.to_string());
        self.zips
            .get(url)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("404 {url}"))
    }
}

fn read(w: &FakeWow, rel: &str) -> String {
    std::fs::read_to_string(w.addons().join(rel)).unwrap()
}

#[test]
fn f4_3_install_creates_record_and_replaces_existing_folders() {
    let w = fake_wow();
    w.add_folder(
        "Foo",
        "Foo.toc",
        &simple_toc("Old Foo", "0"),
        &[("Old.lua", "old")],
    );
    let stable = remote_file("10", "1.0", Channel::Stable, &[]);
    let beta = remote_file("11", "1.1-beta", Channel::Beta, &[]);
    let remote = remote_addon("42", "Foo", vec![stable.clone(), beta.clone()]);
    let zip = zip_bytes(&[
        ("Foo/Foo.toc", &simple_toc("Foo", "1.0")),
        ("Foo/Core.lua", "new"),
        ("Foo_Options/Foo_Options.toc", "## Interface: 16001\n"),
        ("README.md", "loose file"),
        ("__MACOSX/Foo/._Core.lua", "junk"),
    ]);
    let server = Server::new(vec![(&stable, zip)]);
    let addon = install::install_remote(&w.inst, &remote, Channel::Stable, &|u| {
        server.get(u).map(install::in_memory)
    })
    .unwrap();

    assert_eq!(addon.folders, ["Foo", "Foo_Options"]);
    assert_eq!(addon.installed_file_id.as_deref(), Some("10"));
    assert_eq!(addon.installed_version.as_deref(), Some("1.0"));
    assert_eq!(addon.game_version.as_deref(), Some("1.60.1"));
    assert_eq!(addon.provider.as_deref(), Some("Fake"));
    assert!(addon.installed_at.is_some());
    assert!(!addon.update_available());
    assert_eq!(w.folder_names(), ["Foo", "Foo_Options"]);
    assert!(
        !w.addons().join("Foo/Old.lua").exists(),
        "old folder content replaced"
    );
    assert_eq!(read(&w, "Foo/Core.lua"), "new");
    assert_eq!(
        *server.hits.borrow(),
        std::slice::from_ref(&stable.download_url)
    );
    // No staging/backup leftovers next to AddOns.
    let leftovers: Vec<_> = std::fs::read_dir(w.addons().parent().unwrap())
        .unwrap()
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert_eq!(leftovers, ["AddOns"]);

    // The installed addon replaces the Unknown record of its folder.
    let mut list = vec![addon_core::Addon {
        id: "u".into(),
        folders: vec!["Foo".into()],
        ..addon.clone()
    }];
    store::upsert_addon(&mut list, addon.clone());
    assert_eq!(list, vec![addon]);
}

#[test]
fn f3_2_update_swaps_folders_and_rolls_back_on_bad_zip() {
    let w = fake_wow();
    let v1 = remote_file("1", "1.0", Channel::Stable, &[]);
    let v2 = remote_file("22", "2.0", Channel::Stable, &[]);
    let remote = remote_addon("7", "Bar", vec![v1.clone()]);
    let zip1 = zip_bytes(&[
        ("Bar/Bar.toc", &simple_toc("Bar", "1")),
        ("Bar_Old/x.lua", "old module"),
    ]);
    let zip2 = zip_bytes(&[
        ("Bar/Bar.toc", &simple_toc("Bar", "2")),
        ("Bar_New/y.lua", "new module"),
    ]);
    let server = Server::new(vec![(&v1, zip1), (&v2, zip2)]);
    let mut addon = install::install_remote(&w.inst, &remote, Channel::Stable, &|u| {
        server.get(u).map(install::in_memory)
    })
    .unwrap();
    assert_eq!(w.folder_names(), ["Bar", "Bar_Old"]);

    addon.latest = Some(v2.clone());
    assert!(addon.update_available());
    let updated =
        install::update_addon(&w.inst, &addon, &|u| server.get(u).map(install::in_memory)).unwrap();
    assert_eq!(updated.folders, ["Bar", "Bar_New"]);
    assert_eq!(updated.installed_file_id.as_deref(), Some("22"));
    assert!(!updated.update_available());
    assert_eq!(
        w.folder_names(),
        ["Bar", "Bar_New"],
        "folders dropped by the new version are removed"
    );
    assert!(read(&w, "Bar/Bar.toc").contains("## Version: 2"));

    // Bad zips never touch the installed folders.
    for bad in [
        zip_bytes(&[("Bar/Bar.toc", "x"), ("../evil.lua", "pwned")]),
        zip_bytes(&[("/abs/Bar.toc", "x")]),
        zip_bytes(&[("README.md", "no folders")]),
        // A wrapper folder without a TOC would replace Bar with a folder WoW never loads.
        zip_bytes(&[("Bar-v3/Bar/Bar.toc", "x"), ("Bar-v3/README.md", "x")]),
        b"not a zip".to_vec(),
    ] {
        let v3 = remote_file("333", "3.0", Channel::Stable, &[]);
        let server = Server::new(vec![(&v3, bad)]);
        let err = install::install_file(&w.inst, &updated, &v3, &|u| {
            server.get(u).map(install::in_memory)
        })
        .unwrap_err();
        assert!(format!("{err:#}").contains("installing Bar"), "{err:#}");
        assert_eq!(w.folder_names(), ["Bar", "Bar_New"]);
        assert!(read(&w, "Bar/Bar.toc").contains("## Version: 2"));
        assert!(!w.tmp.path().join("battlenet/drive_c/Program Files (x86)/World of Warcraft/_forever_/Interface/evil.lua").exists());
    }
    let leftovers = std::fs::read_dir(w.addons().parent().unwrap())
        .unwrap()
        .count();
    assert_eq!(leftovers, 1);

    // A failed download leaves everything alone too.
    let v4 = remote_file("4444", "4.0", Channel::Stable, &[]);
    let err = install::install_file(&w.inst, &updated, &v4, &|u| {
        server.get(u).map(install::in_memory)
    })
    .unwrap_err();
    assert!(format!("{err:#}").contains("404"));
}

#[test]
fn f3_2_swap_failure_restores_backups() {
    let w = fake_wow();
    w.add_folder(
        "A_First",
        "A_First.toc",
        "## Title: A\n",
        &[("a.lua", "keep me")],
    );
    w.add_folder("Z_Locked", "Z_Locked.toc", "## Title: Z\n", &[]);
    // This backup step fails: moving a directory needs write permission on it (Unix), and none
    // with a file open inside can move (Windows).
    let locked = w.addons().join("Z_Locked");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
    }
    #[cfg(windows)]
    let open_inside = std::fs::File::open(locked.join("Z_Locked.toc")).unwrap();
    let addon = addon_core::Addon {
        folders: vec!["A_First".into(), "Z_Locked".into()],
        ..install::install_remote(
            &w.inst,
            &remote_addon(
                "1",
                "Seed",
                vec![remote_file("s", "s", Channel::Stable, &[])],
            ),
            Channel::Stable,
            &|_| {
                Ok(install::in_memory(zip_bytes(&[(
                    "Seed/Seed.toc",
                    "## Title: Seed\n",
                )])))
            },
        )
        .unwrap()
    };
    let file = remote_file("n", "2", Channel::Stable, &[]);
    let zip = zip_bytes(&[("A_First/A_First.toc", "## Title: new A\n")]);
    let res = install::install_file(&w.inst, &addon, &file, &|_| {
        Ok(install::in_memory(zip.clone()))
    });
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    #[cfg(windows)]
    drop(open_inside);
    let err = res.unwrap_err();
    assert!(format!("{err:#}").contains("backing up"), "{err:#}");
    assert_eq!(read(&w, "A_First/a.lua"), "keep me");
    assert_eq!(w.folder_names(), ["A_First", "Seed", "Z_Locked"]);
    assert_eq!(
        std::fs::read_dir(w.addons().parent().unwrap())
            .unwrap()
            .count(),
        1,
        "no leftovers"
    );
}

#[test]
fn f5_1_remove_deletes_folders() {
    let w = fake_wow();
    w.add_folder("Keep", "Keep.toc", "", &[]);
    w.add_folder("Gone", "Gone.toc", "", &[("sub/x.lua", "x")]);
    let mut addon = install::install_remote(
        &w.inst,
        &remote_addon(
            "1",
            "Gone",
            vec![remote_file("1", "1", Channel::Stable, &[])],
        ),
        Channel::Stable,
        &|_| {
            Ok(install::in_memory(zip_bytes(&[
                ("Gone/Gone.toc", "x"),
                ("Gone_Data/d.lua", "d"),
            ])))
        },
    )
    .unwrap();
    addon.folders.push("AlreadyMissing".into());
    install::remove_addon(&w.inst, &addon).unwrap();
    assert_eq!(w.folder_names(), ["Keep"]);
    addon.folders = vec!["../Keep".into()];
    assert!(install::remove_addon(&w.inst, &addon).is_err());
}

#[test]
fn f5_2_reinstall_restores_modified_files() {
    let w = fake_wow();
    let file = remote_file("5", "5.0", Channel::Stable, &[]);
    let zip = zip_bytes(&[
        ("R/R.toc", &simple_toc("R", "5")),
        ("R/Core.lua", "original"),
    ]);
    let dl = |_: &str| Ok(install::in_memory(zip.clone()));
    let addon = install::install_remote(
        &w.inst,
        &remote_addon("9", "R", vec![file.clone()]),
        Channel::Stable,
        &dl,
    )
    .unwrap();
    write(&w.addons().join("R/Core.lua"), "broken");
    write(&w.addons().join("R/stray.lua"), "stray");
    let again = install::install_file(&w.inst, &addon, &file, &dl).unwrap();
    assert_eq!(again.id, addon.id);
    assert_eq!(read(&w, "R/Core.lua"), "original");
    assert!(!w.addons().join("R/stray.lua").exists());
}

#[test]
fn f4_3_install_takes_only_shared_folders_from_other_records() {
    let base = addon_core::Addon {
        id: "a".into(),
        name: "A".into(),
        author: String::new(),
        summary: String::new(),
        provider: Some("Fake".into()),
        external_id: Some("1".into()),
        url: None,
        thumbnail_url: None,
        folders: vec!["A".into(), "LibShared".into()],
        installed_version: None,
        installed_file_id: None,
        installed_at: None,
        game_version: None,
        game_versions: vec![],
        channel: Channel::Beta,
        latest: None,
        ignored: None,
        auto_update: false,
        dependencies: vec![],
    };
    let unknown = addon_core::Addon {
        id: "u".into(),
        name: "B".into(),
        provider: None,
        folders: vec!["B".into()],
        ..base.clone()
    };
    let b = addon_core::Addon {
        id: "b".into(),
        name: "B".into(),
        external_id: Some("2".into()),
        folders: vec!["B".into(), "LibShared".into()],
        ..base.clone()
    };
    let mut list = vec![base.clone(), unknown];
    store::upsert_addon(&mut list, b.clone());
    // A keeps its own folder and settings; the Unknown record of B's folder is gone.
    assert_eq!(list.len(), 2, "{list:#?}");
    assert_eq!(list[0].folders, ["A"]);
    assert_eq!(list[0].channel, Channel::Beta);
    assert_eq!(list[1], b);

    // Removing never touches a folder another record also lists.
    let mut overlapping = list.clone();
    overlapping[0].folders.push("LibShared".into());
    assert_eq!(store::owned_folders(&overlapping, &b), ["B"]);
    assert_eq!(store::owned_folders(&list, &b), ["B", "LibShared"]);
}

#[test]
fn f4_3_no_download_url_means_the_author_said_no() {
    // CurseForge sends no URL when the author turned off downloads outside its own app.
    let w = fake_wow();
    let mut file = remote_file("10", "1.0", Channel::Stable, &[]);
    file.download_url.clear();
    let remote = remote_addon("42", "Foo", vec![file]);
    let err = install::install_remote(&w.inst, &remote, Channel::Stable, &|_| {
        panic!("nothing is downloaded")
    })
    .unwrap_err();
    assert!(
        format!("{err:#}").contains("only allows downloads"),
        "{err:#}"
    );
}

#[test]
fn f2_7_install_and_update_record_the_game_versions() {
    let w = fake_wow();
    let client = w.inst.version.as_deref();
    let toc = |interface: &str| format!("## Interface: {interface}\n## Title: G\n");
    let v1 = remote_file("1", "1.0", Channel::Stable, &[]);
    let v2 = remote_file("22", "2.0", Channel::Stable, &[]);
    let server = Server::new(vec![
        (&v1, zip_bytes(&[("G/G.toc", &toc("16000"))])),
        (&v2, zip_bytes(&[("G/G.toc", &toc("11509, 16001"))])),
    ]);
    let dl = |u: &str| server.get(u).map(install::in_memory);
    let mut addon = install::install_remote(
        &w.inst,
        &remote_addon("3", "G", vec![v1]),
        Channel::Stable,
        &dl,
    )
    .unwrap();
    assert_eq!(addon.game_versions, ["1.60.0"]);
    assert!(addon.out_of_date(client), "flagged without a rescan");

    addon.latest = Some(v2);
    let updated = install::update_addon(&w.inst, &addon, &dl).unwrap();
    assert_eq!(updated.game_versions, ["1.15.9", "1.60.1"]);
    assert_eq!(updated.game_version.as_deref(), Some("1.60.1"));
    assert!(
        !updated.out_of_date(client),
        "the fixed build is no longer out of date"
    );
}

#[test]
fn f4_3_install_never_replaces_a_git_checkout() {
    let w = fake_wow();
    w.add_folder(
        "Foo",
        "Foo.toc",
        &simple_toc("My Foo", "dev"),
        &[
            (".git/HEAD", "ref: refs/heads/main\n"),
            ("Wip.lua", "unsaved work"),
        ],
    );
    let file = remote_file("1", "1.0", Channel::Stable, &[]);
    let zip = zip_bytes(&[
        ("Foo/Foo.toc", &simple_toc("Foo", "1.0")),
        ("Foo_Extra/Foo_Extra.toc", "## Title: Extra\n"),
    ]);
    let err = install::install_remote(
        &w.inst,
        &remote_addon("5", "Foo", vec![file]),
        Channel::Stable,
        &|_| Ok(install::in_memory(zip.clone())),
    )
    .unwrap_err();
    assert!(
        format!("{err:#}").contains("Foo is a git checkout"),
        "{err:#}"
    );
    assert_eq!(w.folder_names(), ["Foo"], "nothing installed");
    assert_eq!(read(&w, "Foo/Wip.lua"), "unsaved work");
    assert!(w.addons().join("Foo/.git/HEAD").is_file());
    let leftovers = std::fs::read_dir(w.addons().parent().unwrap())
        .unwrap()
        .count();
    assert_eq!(leftovers, 1, "no staging or backup left");
}

#[test]
fn f3_2_recover_puts_back_folders_of_an_interrupted_install() {
    let w = fake_wow();
    let interface = w.addons().parent().unwrap().to_path_buf();
    // Cut off mid-swap: Foo and Foo_Lib were backed up, the new Foo_Lib was already moved in.
    w.add_folder("Foo_Lib", "Foo_Lib.toc", "## Title: new lib\n", &[]);
    let backup = interface.join(".addons-backup-20260101-abcdef12");
    write(&backup.join("Foo/Foo.toc"), "## Title: old Foo\n");
    write(&backup.join("Foo_Lib/Foo_Lib.toc"), "## Title: old lib\n");
    write(
        &interface.join(".addons-staging-20260101-abcdef12/Foo/Foo.toc"),
        "## Title: new Foo\n",
    );
    // A backup kept after a failed restore, for a folder the user has since deleted.
    write(
        &interface.join(".addons-backup-20260102-12345678/Bar/Bar.toc"),
        "## Title: Bar\n",
    );

    install::recover(&w.inst).unwrap();
    assert_eq!(w.folder_names(), ["Bar", "Foo", "Foo_Lib"]);
    assert_eq!(
        read(&w, "Foo/Foo.toc"),
        "## Title: old Foo\n",
        "missing folder put back"
    );
    assert_eq!(
        read(&w, "Foo_Lib/Foo_Lib.toc"),
        "## Title: new lib\n",
        "present folder kept"
    );
    let left: Vec<_> = std::fs::read_dir(&interface).unwrap().collect();
    assert_eq!(left.len(), 1, "backup and staging dirs deleted");

    install::recover(&w.inst).unwrap();
    assert_eq!(
        w.folder_names(),
        ["Bar", "Foo", "Foo_Lib"],
        "nothing to do the second time"
    );
}
