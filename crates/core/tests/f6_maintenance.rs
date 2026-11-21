mod common;
use addon_core::maintenance::{self, Cleanup};
use addon_core::wow;
use common::*;

#[test]
fn f6_1_clear_cache_removes_cache_dir() {
    let w = fake_wow();
    let root = &w.inst.root;
    write(&root.join("Cache/WDB/enUS/creaturecache.wdb"), "12345");
    write(&root.join("Cache/ADB/enUS/DBCache.bin"), "123");
    write(&root.join("WTF/Config.wtf"), "keep");

    assert_eq!(
        maintenance::clear_cache(&w.inst, true).unwrap(),
        Cleanup { files: 1, bytes: 5 }
    );
    assert!(!root.join("Cache/WDB").exists());
    assert!(root.join("Cache/ADB/enUS/DBCache.bin").exists());

    assert_eq!(
        maintenance::clear_cache(&w.inst, false).unwrap(),
        Cleanup { files: 1, bytes: 3 }
    );
    assert!(!root.join("Cache").exists());
    assert!(root.join("WTF/Config.wtf").exists());
    assert_eq!(
        maintenance::clear_cache(&w.inst, false).unwrap(),
        Cleanup::default()
    );
}

/// The fake game: this test binary, copied as `Wow.exe`, runs only this (ignored) test.
#[test]
#[ignore = "helper process for f6_1_refuses_while_game_running"]
fn fake_game() {
    std::thread::sleep(std::time::Duration::from_secs(30));
}

fn start_fake_game(exe: &std::path::Path, cwd: &std::path::Path) -> std::process::Child {
    std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
    std::fs::copy(std::env::current_exe().unwrap(), exe).unwrap();
    std::process::Command::new(exe)
        .args(["--exact", "fake_game", "--ignored"])
        .current_dir(cwd)
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap()
}

#[test]
fn f6_1_refuses_while_game_running() {
    let w = fake_wow();
    write(&w.inst.root.join("Cache/WDB/x.wdb"), "x");
    assert!(!wow::is_game_running(&w.inst));
    // A Forever client running from another folder (say the user's real game) doesn't lock this one.
    let other = w.tmp.path().join("other/World of Warcraft/_forever_");
    let mut elsewhere = start_fake_game(&other.join("Wow.exe"), &other);
    std::thread::sleep(std::time::Duration::from_millis(300));
    let other_locks = wow::is_game_running(&w.inst);
    elsewhere.kill().unwrap();
    elsewhere.wait().unwrap();
    assert!(!other_locks, "another client's game must not lock this one");
    // The game in this client folder. Under Wine (Linux) its program lies outside, in the prefix,
    // and only its working directory is the client folder.
    let exe = if cfg!(target_os = "linux") {
        w.tmp.path().join("bin/Wow.exe")
    } else {
        w.inst.root.join("Wow.exe")
    };
    let mut child = start_fake_game(&exe, &w.inst.root);
    let mut running = false;
    for _ in 0..100 {
        running = wow::is_game_running(&w.inst);
        if running {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let results = [
        maintenance::clear_cache(&w.inst, false).map(|_| ()),
        maintenance::clean_backup_files(&w.inst).map(|_| ()),
        maintenance::backup_wtf(&w.inst, &w.tmp.path().join("b")).map(|_| ()),
    ];
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(running);
    for r in results {
        assert!(r.unwrap_err().to_string().contains("running"));
    }
    assert!(w.inst.root.join("Cache/WDB/x.wdb").exists());
}

#[test]
fn f6_2_clean_backup_files() {
    let w = fake_wow();
    let wtf = w.inst.root.join("WTF");
    write(&wtf.join("Account/ACC/SavedVariables/Details.lua"), "keep");
    write(
        &wtf.join("Account/ACC/SavedVariables/Details.lua.bak"),
        "1234",
    );
    write(
        &wtf.join("Account/ACC/Realm/Char/SavedVariables/Questie.lua.bak"),
        "12",
    );
    write(&wtf.join("config-cache.old"), "1");
    write(
        &wtf.join("Account/ACC/bindings-cache.old"),
        "deep .old files stay",
    );
    assert_eq!(
        maintenance::clean_backup_files(&w.inst).unwrap(),
        Cleanup { files: 3, bytes: 7 }
    );
    assert!(wtf.join("Account/ACC/SavedVariables/Details.lua").exists());
    assert!(wtf.join("Account/ACC/bindings-cache.old").exists());
    assert!(!wtf.join("config-cache.old").exists());
}

#[test]
fn f6_3_backup_wtf_zips_and_lists() {
    let w = fake_wow();
    let wtf = w.inst.root.join("WTF");
    write(&wtf.join("Config.wtf"), "SET a 1");
    write(
        &wtf.join("Account/ACC/SavedVariables/Details.lua"),
        "Details = {}",
    );
    let backups = w.tmp.path().join("data/wtf_backups").join(&w.inst.id);
    assert!(maintenance::list_wtf_backups(&backups).unwrap().is_empty());

    let first = maintenance::backup_wtf(&w.inst, &backups).unwrap();
    let second = maintenance::backup_wtf(&w.inst, &backups).unwrap();
    assert_ne!(first, second);
    let list = maintenance::list_wtf_backups(&backups).unwrap();
    assert_eq!(list.len(), 2);
    assert!(list.iter().all(|b| b.size > 0));

    let mut zip = zip::ZipArchive::new(std::fs::File::open(&first).unwrap()).unwrap();
    let mut names: Vec<String> = zip.file_names().map(|n| n.unwrap().to_string()).collect();
    names.sort();
    assert_eq!(
        names,
        [
            "WTF/Account/ACC/SavedVariables/Details.lua",
            "WTF/Config.wtf"
        ]
    );
    let mut s = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("WTF/Config.wtf").unwrap(), &mut s).unwrap();
    assert_eq!(s, "SET a 1");
}

#[test]
fn f6_1_only_touches_a_forever_client_folder() {
    let w = fake_wow();
    // A broken installations.json pointing at the WoW root (or anywhere else) must not clear it.
    let mut wrong = w.inst.clone();
    wrong.root = w.wow_root.clone();
    write(&w.wow_root.join("Cache/keep.bin"), "x");
    let err = maintenance::clear_cache(&wrong, false).unwrap_err();
    assert!(
        err.to_string().contains("not a WoW Forever client"),
        "{err:#}"
    );
    assert!(maintenance::clean_backup_files(&wrong).is_err());
    assert!(w.wow_root.join("Cache/keep.bin").exists());

    // A Cache link is not followed into the folder it points at.
    let elsewhere = w.tmp.path().join("elsewhere");
    write(&elsewhere.join("precious.txt"), "keep");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&elsewhere, w.inst.root.join("Cache")).unwrap();
    // Needs admin rights or Developer Mode on Windows (CI runners have them).
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&elsewhere, w.inst.root.join("Cache")).unwrap();
    assert!(maintenance::clear_cache(&w.inst, false).is_err());
    assert!(elsewhere.join("precious.txt").exists());
}
