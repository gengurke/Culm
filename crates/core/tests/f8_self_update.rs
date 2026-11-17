//! F8.4: the app finds its newest release and swaps its own file for it, checked against SHA256SUMS.

use addon_core::self_update::{self, Release};
use addon_testkit::{FakeServer, serve_app_release};
use std::fs;

const REPO: &str = "owner/App";
const ASSET: &str = "App-0.2.0-x86_64.AppImage";

fn release(server: &FakeServer) -> Release {
    self_update::latest(server.url(), REPO, |n| n.ends_with("-x86_64.AppImage")).unwrap()
}

#[test]
fn f8_4_latest_release_picks_the_fitting_asset() {
    let server = FakeServer::start();
    serve_app_release(&server, REPO, "0.2.0", ASSET, b"new app", None);
    let r = release(&server);
    assert_eq!(r.version, "0.2.0");
    assert_eq!(r.asset.as_ref().unwrap().0, ASSET);
    assert!(r.sums.is_some());
    assert!(self_update::is_newer(&r.version, "0.1.0"));
}

#[test]
fn f8_4_update_replaces_the_file_and_keeps_it_executable() {
    let server = FakeServer::start();
    serve_app_release(&server, REPO, "0.2.0", ASSET, b"new app", None);
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("App.AppImage");
    fs::write(&app, b"old app").unwrap();

    let mut seen = 0;
    self_update::replace(&app, &release(&server), &mut |done, _| seen = done).unwrap();

    assert_eq!(fs::read(&app).unwrap(), b"new app");
    assert_eq!(seen, 7);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&app).unwrap().permissions().mode() & 0o111,
            0o111
        );
    }
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        1,
        "no leftover download"
    );
}

#[test]
fn f8_4_damaged_download_leaves_the_app_alone() {
    let server = FakeServer::start();
    let wrong = format!("{}  {ASSET}\n", "0".repeat(64));
    serve_app_release(&server, REPO, "0.2.0", ASSET, b"new app", Some(&wrong));
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("App.AppImage");
    fs::write(&app, b"old app").unwrap();

    let err = self_update::replace(&app, &release(&server), &mut |_, _| {}).unwrap_err();

    assert!(err.to_string().contains("damaged"), "{err}");
    assert_eq!(fs::read(&app).unwrap(), b"old app");
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        1,
        "the partial download is removed"
    );
}

#[test]
fn f8_4_release_without_checksums_is_refused() {
    let server = FakeServer::start();
    serve_app_release(
        &server,
        REPO,
        "0.2.0",
        ASSET,
        b"new app",
        Some("nothing useful\n"),
    );
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("App.AppImage");
    fs::write(&app, b"old app").unwrap();

    assert!(self_update::replace(&app, &release(&server), &mut |_, _| {}).is_err());
    assert_eq!(fs::read(&app).unwrap(), b"old app");
}
