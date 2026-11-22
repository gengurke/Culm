//! F3.5, F4.3: an addon the user fetched by hand is recognised by its size and SHA-1, then installed.

mod common;
use addon_core::{Channel, FileCheck, install, manual};
use addon_testkit::{make_zip, toc, write};
use common::*;
use sha1::{Digest, Sha1};

fn check_of(bytes: &[u8]) -> FileCheck {
    FileCheck {
        size: bytes.len() as u64,
        sha1: Sha1::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    }
}

#[test]
fn f3_5_finds_only_the_exact_zip_in_downloads() {
    let dir = tempfile::tempdir().unwrap();
    let right = make_zip(&[("Bar/Bar.toc", toc("Bar", "2").as_bytes())]);
    let check = check_of(&right);
    let mut same_size = right.clone();
    *same_size.last_mut().unwrap() ^= 1;
    write(&dir.path().join("Bar-2.0.zip"), &same_size);
    write(&dir.path().join("Bar-2.0(1).zip.part"), &right);
    write(&dir.path().join("Other.zip"), b"not even a zip");
    assert_eq!(
        manual::find_in(dir.path(), &check),
        Vec::<std::path::PathBuf>::new(),
        "a changed or unfinished file never counts"
    );

    write(&dir.path().join("Bar-2.0(1).zip"), &right);
    write(&dir.path().join("Bar-2.0(2).zip"), &right);
    assert_eq!(
        manual::find_in(dir.path(), &check),
        [
            dir.path().join("Bar-2.0(1).zip"),
            dir.path().join("Bar-2.0(2).zip")
        ],
        "every identical copy, whatever the browser named it"
    );
    assert!(manual::find_in(&dir.path().join("missing"), &check).is_empty());
}

#[test]
fn f3_5_installs_a_zip_fetched_by_hand_only_when_it_is_the_file() {
    let w = fake_wow();
    let v1 = remote_file("1", "1.0", Channel::Stable, &[]);
    let zip1 = make_zip(&[("Bar/Bar.toc", toc("Bar", "1").as_bytes())]);
    let remote = remote_addon("7", "Bar", vec![v1.clone()]);
    let mut addon = install::install_remote(&w.inst, &remote, Channel::Stable, &|_| {
        Ok(install::in_memory(zip1.clone()))
    })
    .unwrap();

    let zip2 = make_zip(&[
        ("Bar/Bar.toc", toc("Bar", "2").as_bytes()),
        ("Bar_New/y.lua", b"new module"),
    ]);
    let mut v2 = remote_file("22", "2.0", Channel::Stable, &[]);
    v2.download_url.clear();
    addon.latest = Some(v2.clone());
    let downloads = tempfile::tempdir().unwrap();
    let path = downloads.path().join("Bar-2.0.zip");
    write(&path, &zip2);

    let err = manual::install(&w.inst, &addon, &path).unwrap_err();
    assert!(
        format!("{err:#}").contains("check for updates first"),
        "{err:#}"
    );

    addon.latest.as_mut().unwrap().check = Some(check_of(b"something else"));
    let err = manual::install(&w.inst, &addon, &path).unwrap_err();
    assert!(
        format!("{err:#}").contains("this file is not Bar 2.0"),
        "{err:#}"
    );
    assert_eq!(w.folder_names(), ["Bar"], "nothing changed");

    addon.latest.as_mut().unwrap().check = Some(check_of(&zip2));
    let updated = manual::install(&w.inst, &addon, &path).unwrap();
    assert_eq!(updated.installed_file_id.as_deref(), Some("22"));
    assert_eq!(w.folder_names(), ["Bar", "Bar_New"]);
}

#[test]
fn f4_3_installs_a_new_addon_from_a_zip_fetched_by_hand() {
    let w = fake_wow();
    let zip = make_zip(&[("Bar/Bar.toc", toc("Bar", "2").as_bytes())]);
    let mut beta = remote_file("30", "3.0-beta", Channel::Beta, &[]);
    beta.released_at = Some("2030-01-01T00:00:00Z".into());
    let mut v2 = remote_file("22", "2.0", Channel::Stable, &[]);
    v2.download_url.clear();
    v2.check = Some(check_of(&zip));
    let remote = remote_addon("7", "Bar", vec![v2, beta]);
    let downloads = tempfile::tempdir().unwrap();
    let path = downloads.path().join("Bar-2.0.zip");
    write(&path, &zip);

    let record = manual::new_record(&remote, Channel::Stable).unwrap();
    assert_eq!(
        record.latest.as_ref().map(|f| f.id.as_str()),
        Some("22"),
        "the file to fetch is the one a normal install takes"
    );
    assert!(record.folders.is_empty() && record.installed_file_id.is_none());

    let installed = manual::install(&w.inst, &record, &path).unwrap();
    assert_eq!(installed.installed_file_id.as_deref(), Some("22"));
    assert_eq!(installed.external_id.as_deref(), Some("7"));
    assert_eq!(installed.folders, ["Bar"]);
    assert_eq!(w.folder_names(), ["Bar"]);
}
