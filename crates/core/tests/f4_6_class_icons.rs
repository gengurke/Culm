//! F4.6 class icons: downloaded once from the first working source, then read from disk.

use addon_core::class_icons::{CLASSES, load};
use addon_testkit::{CLASS_ICON, FakeServer};
use std::path::Path;

/// Every class's result of one `load` (as the app makes once per run).
fn run(dir: &Path, sources: &[String]) -> Vec<(String, Option<Vec<u8>>)> {
    let mut got = Vec::new();
    load(dir, sources, &mut |slug, bytes| {
        got.push((slug.into(), bytes))
    });
    got
}

fn sources(s: &FakeServer) -> Vec<String> {
    ["a", "b", "c"]
        .map(|n| format!("{}/{n}/", s.url()))
        .to_vec()
}

#[test]
fn f4_6_class_icons_come_from_the_first_working_source() {
    let s = FakeServer::start();
    let dir = tempfile::tempdir().unwrap();
    // a: gone (404); b: answers, but not with an image; c: the icons.
    s.respond("GET", "/b/*", 200, "image/jpeg", "<html>moved</html>");
    s.respond("GET", "/c/*", 200, "image/jpeg", CLASS_ICON);
    let got = run(dir.path(), &sources(&s));
    assert_eq!(got.len(), CLASSES.len());
    assert!(got.iter().all(|(_, b)| b.as_deref() == Some(CLASS_ICON)));
    assert_eq!(s.requests_to("/c/classicon_warrior.jpg").len(), 1);
    // A source that failed once isn't asked again in this run.
    assert_eq!(s.requests_to("/a/classicon_warrior.jpg").len(), 1);
    assert_eq!(s.requests_to("/a/classicon_mage.jpg").len(), 0);
    let file = dir.path().join("class-icons/paladin.jpg");
    assert_eq!(std::fs::read(file).unwrap(), CLASS_ICON, "cached on disk");
}

#[test]
fn f4_6_cached_class_icons_need_no_request() {
    let s = FakeServer::start();
    let dir = tempfile::tempdir().unwrap();
    s.respond("GET", "/a/*", 200, "image/jpeg", CLASS_ICON);
    run(dir.path(), &sources(&s));
    let asked = s.requests().len();
    assert_eq!(asked, CLASSES.len());

    // The next start: from disk, no network.
    let got = run(dir.path(), &sources(&s));
    assert!(got.iter().all(|(_, b)| b.as_deref() == Some(CLASS_ICON)));
    assert_eq!(s.requests().len(), asked);
}

#[test]
fn f4_6_corrupt_cached_class_icon_is_fetched_again() {
    let s = FakeServer::start();
    let dir = tempfile::tempdir().unwrap();
    s.respond("GET", "/a/*", 200, "image/jpeg", CLASS_ICON);
    run(dir.path(), &sources(&s));
    let file = dir.path().join("class-icons/mage.jpg");
    std::fs::write(&file, &CLASS_ICON[..CLASS_ICON.len() / 2]).unwrap();

    let got = run(dir.path(), &sources(&s));
    assert!(got.iter().all(|(_, b)| b.as_deref() == Some(CLASS_ICON)));
    assert_eq!(s.requests_to("/a/classicon_mage.jpg").len(), 2);
    assert_eq!(s.requests_to("/a/classicon_rogue.jpg").len(), 1);
    assert_eq!(std::fs::read(file).unwrap(), CLASS_ICON, "repaired");
}

#[test]
fn f4_6_class_icons_unavailable_cost_one_attempt_per_source() {
    let s = FakeServer::start();
    let dir = tempfile::tempdir().unwrap();
    // Every source gone (or offline): no icons, so the app keeps its own glyphs.
    let got = run(dir.path(), &sources(&s));
    assert_eq!(got.len(), CLASSES.len());
    assert!(got.iter().all(|(_, b)| b.is_none()));
    assert_eq!(s.requests().len(), 3, "one try per source, no loop");
    assert!(!dir.path().join("class-icons/warrior.jpg").exists());
}

#[test]
fn f4_6_oversized_class_icon_is_refused() {
    let s = FakeServer::start();
    let dir = tempfile::tempdir().unwrap();
    // A valid image, but no class icon is 512 px wide.
    let mut big = Vec::new();
    image::DynamicImage::new_rgb8(512, 512)
        .write_to(&mut std::io::Cursor::new(&mut big), image::ImageFormat::Png)
        .unwrap();
    s.respond("GET", "/a/*", 200, "image/png", big);
    s.respond("GET", "/b/*", 200, "image/jpeg", CLASS_ICON);
    let got = run(dir.path(), &sources(&s));
    assert!(got.iter().all(|(_, b)| b.as_deref() == Some(CLASS_ICON)));
}
