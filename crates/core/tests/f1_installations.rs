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
