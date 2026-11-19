//! F8.4 end to end: the app finds its own newer release, replaces its AppImage and restarts into it.
//! The "AppImage" is a temp file; the new one is a shell script that leaves a mark when it starts.

mod common;
use addon_app::state::AppUpdate;
use addon_testkit::serve_app_release;
use common::*;

const REPO: &str = "gengurke/Culm";

fn asset(version: &str) -> String {
    format!("Culm-{version}-{}.AppImage", std::env::consts::ARCH)
}

#[cfg(unix)]
#[test]
fn f8_4_update_from_the_sidebar_replaces_the_appimage_and_restarts_it() {
    let world = World::new();
    let started = world.tmp.path().join("new-version-started");
    // The new version records the descriptors it inherited (Linux has `/proc`), then that it started.
    let script = format!(
        "#!/bin/sh\n{{ ls /proc/self/fd 2>/dev/null; echo started; }} > '{0}.part'\nmv '{0}.part' '{0}'\n",
        started.display()
    );
    serve_app_release(
        &world.server,
        REPO,
        "99.0.0",
        &asset("99.0.0"),
        script.as_bytes(),
        None,
    );
    let appimage = world.tmp.path().join("Culm.AppImage");
    std::fs::write(&appimage, b"old version").unwrap();
    let mut config = world.config();
    config.appimage = Some(appimage.clone());

    let mut app = world.app_with(config);
    app.wait_for("the check at start", |s| {
        matches!(s.app_update, AppUpdate::Available(_))
    });
    assert!(app.shows("Culm 99.0.0 is available"));
    assert!(app.shows_in("app-update/sidebar", "Update to 99.0.0"));

    app.tap("app-update/sidebar");
    app.wait_for("the download", |s| {
        matches!(s.app_update, AppUpdate::Ready(_))
    });
    assert_eq!(std::fs::read_to_string(&appimage).unwrap(), script);
    assert!(app.shows_in("app-update/sidebar", "Restart for 99.0.0"));

    // Like the AppImage runtime's mount keep-alive: a descriptor without close-on-exec.
    #[cfg(target_os = "linux")]
    let keepalive = unsafe { libc::dup2(0, 1000) };
    app.tap("app-update/sidebar");
    let start = std::time::Instant::now();
    while std::fs::read_to_string(&started).map_or(true, |s| s.is_empty()) {
        assert!(
            start.elapsed().as_secs() < 10,
            "the new version never started"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    #[cfg(target_os = "linux")]
    {
        assert_eq!(keepalive, 1000);
        let fds = std::fs::read_to_string(&started).unwrap();
        assert!(
            !fds.lines().any(|l| l == "1000"),
            "the new version inherited the old one's descriptors: {fds}"
        );
        unsafe { libc::close(1000) };
    }
}

#[test]
fn f8_4_damaged_update_keeps_the_running_version() {
    let world = World::new();
    let wrong = format!("{}  {}\n", "0".repeat(64), asset("99.0.0"));
    serve_app_release(
        &world.server,
        REPO,
        "99.0.0",
        &asset("99.0.0"),
        b"new",
        Some(&wrong),
    );
    let appimage = world.tmp.path().join("Culm.AppImage");
    std::fs::write(&appimage, b"old version").unwrap();
    let mut config = world.config();
    config.appimage = Some(appimage.clone());

    let mut app = world.app_with(config);
    app.wait_for("the check at start", |s| {
        matches!(s.app_update, AppUpdate::Available(_))
    });
    app.tap("app-update/sidebar");
    app.wait_for("the failed download", |s| {
        matches!(s.app_update, AppUpdate::Failed { .. })
    });
    assert_eq!(std::fs::read(&appimage).unwrap(), b"old version");
    assert!(app.shows("Update failed"));
    assert!(
        app.shows_in("app-update/sidebar", "Update to 99.0.0"),
        "it can be tried again"
    );
}

#[test]
fn f8_4_a_copy_that_cant_update_itself_asks_only_when_told_and_links_the_release() {
    let world = World::new();
    serve_app_release(
        &world.server,
        REPO,
        "99.0.0",
        &asset("99.0.0"),
        b"new",
        None,
    );

    let mut app = world.app();
    app.wait_idle();
    assert!(
        world
            .server
            .requests_to(&format!("/repos/{REPO}/releases/latest"))
            .is_empty(),
        "no check at start outside an AppImage"
    );
    assert!(!app.has("app-update/sidebar"));

    app.tap("nav/settings");
    app.reveal("settings/check-app-update");
    app.tap("settings/check-app-update");
    app.wait_for("the check", |s| {
        matches!(s.app_update, AppUpdate::Available(_))
    });
    assert!(app.shows_in("app-update/sidebar", "Get 99.0.0"));
    assert!(app.shows_in("settings/app-update", "Get 99.0.0"));
}

#[test]
fn f8_4_check_says_when_the_app_is_up_to_date() {
    let world = World::new();
    let current = env!("CARGO_PKG_VERSION");
    serve_app_release(&world.server, REPO, current, &asset(current), b"same", None);

    let mut app = world.app();
    app.wait_idle();
    app.tap("nav/settings");
    app.reveal("settings/check-app-update");
    app.tap("settings/check-app-update");
    app.wait_for("the check", |s| s.app_update == AppUpdate::UpToDate);
    assert!(app.shows("Culm is up to date"));
    assert!(app.shows("You have the newest version."));
    assert!(!app.has("app-update/sidebar"));
}
