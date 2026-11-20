mod common;
use addon_core::{store, wow};
use common::*;

#[test]
fn f1_1_detects_forever_install_from_build_info() {
    let w = fake_wow();
    // A retail-only install (no wowf row) and a prefix without WoW must not be detected.
    let retail = w
        .tmp
        .path()
        .join("other/drive_c/Program Files (x86)/World of Warcraft");
    write(
        &retail.join(".build.info"),
        "Branch!STRING:0|Version!STRING:0|Product!STRING:0\neu|11.2.7.1|wow\n",
    );
    std::fs::create_dir_all(retail.join("_forever_")).unwrap();
    std::fs::create_dir_all(w.tmp.path().join("empty/drive_c")).unwrap();

    let roots = vec![
        w.tmp.path().join("battlenet"),
        w.tmp.path().join("other"),
        w.tmp.path().join("empty"),
        w.tmp.path().join("battlenet"),
    ];
    let found = wow::detect_installations(&roots);
    assert_eq!(found.len(), 1, "{found:?}");
    let inst = &found[0];
    assert_eq!(inst.version.as_deref(), Some("1.60.1.70938"));
    assert!(inst.root.ends_with("World of Warcraft/_forever_"));
    assert_eq!(inst.label, "WoW Forever");
    assert_eq!(wow::prefix_name(&inst.root).as_deref(), Some("battlenet"));
    assert_eq!(inst, &w.inst, "detection and manual add agree on id");
}

/// A Forever client (with its `.build.info`) in `wow_root`.
fn forever_at(wow_root: &std::path::Path) {
    write(&wow_root.join(".build.info"), BUILD_INFO);
    std::fs::create_dir_all(wow_root.join("_forever_")).unwrap();
}

fn found_roots(roots: &[std::path::PathBuf]) -> Vec<std::path::PathBuf> {
    wow::detect_installations(roots)
        .into_iter()
        .map(|i| i.root.parent().unwrap().to_path_buf())
        .collect()
}

#[test]
fn f1_1_detects_windows_installs_on_every_drive_and_in_the_profile() {
    let tmp = tempfile::tempdir().unwrap();
    let (c, d, home) = (
        tmp.path().join("C"),
        tmp.path().join("D"),
        tmp.path().join("C/Users/me"),
    );
    let installs = [
        c.join("Program Files (x86)/World of Warcraft"), // Battle.net's default
        c.join("Program Files/World of Warcraft"),
        d.join("World of Warcraft"),
        d.join("Games/World of Warcraft"),
        home.join("Games/World of Warcraft"),
    ];
    installs.iter().for_each(|r| forever_at(r));
    let roots = wow::search_roots("windows", &home, &[c, d]);
    let found = found_roots(&roots);
    for want in &installs {
        let rel = want.strip_prefix(tmp.path()).unwrap();
        assert!(
            found.iter().any(|f| f.ends_with(rel)),
            "{rel:?} not in {found:?}"
        );
    }
    assert_eq!(found.len(), installs.len(), "{found:?}");
}

#[test]
fn f1_1_detects_macos_installs_in_applications() {
    let tmp = tempfile::tempdir().unwrap();
    let (disk, home) = (tmp.path().join("disk"), tmp.path().join("disk/Users/me"));
    let installs = [
        disk.join("Applications/World of Warcraft"), // the installer's default
        home.join("Applications/World of Warcraft"),
    ];
    installs.iter().for_each(|r| forever_at(r));
    let found = found_roots(&wow::search_roots("macos", &home, &[disk]));
    assert_eq!(found.len(), 2, "{found:?}");
}

#[test]
fn f1_1_detects_linux_installs_in_wine_prefixes() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let installs = [
        home.join(".wine/drive_c/Program Files (x86)/World of Warcraft"),
        home.join("Games/battlenet/drive_c/Program Files (x86)/World of Warcraft"),
        home.join(".local/share/Steam/steamapps/compatdata/123/pfx/drive_c/Program Files (x86)/World of Warcraft"),
    ];
    installs.iter().for_each(|r| forever_at(r));
    let found = found_roots(&wow::search_roots("linux", home, &[]));
    assert_eq!(found.len(), 3, "{found:?}");
}

#[test]
fn f1_2_normalizes_manual_paths() {
    let w = fake_wow();
    let client = w.wow_root.join("_forever_");
    write(&client.join("Wow.exe"), "MZ");
    for p in [
        w.wow_root.clone(),
        client.clone(),
        client.join("Interface/AddOns"),
        client.join("Interface"),
        client.join("Wow.exe"),
    ] {
        let inst = wow::installation_from_path(&p).unwrap();
        assert_eq!(inst.id, w.inst.id, "{}", p.display());
        assert_eq!(inst.version.as_deref(), Some("1.60.1.70938"));
    }
    let err = wow::installation_from_path(w.tmp.path()).unwrap_err();
    assert!(err.to_string().contains("_forever_"), "{err}");
}

#[test]
fn f1_3_installations_persist() {
    let w = fake_wow();
    let data = tempfile::tempdir().unwrap();
    assert!(store::load_installations(data.path()).unwrap().is_empty());
    store::save_installations(data.path(), std::slice::from_ref(&w.inst)).unwrap();
    assert_eq!(
        store::load_installations(data.path()).unwrap(),
        vec![w.inst.clone()]
    );
}
