//! F7.7: the log file. Its own test binary: the logger and the panic hook are process-wide.

use addon_core::log;

#[test]
fn f7_7_log_rotates_keeps_recent_errors_hides_home_and_records_crashes() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home").join("someone");
    let file = tmp.path().join("logs").join("culm.log");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    // A full log from an earlier run, ending in a crash.
    let mut earlier = "2026-11-20T10:00:00Z info: started\n".repeat(8000);
    earlier.push_str("2026-11-20T10:00:01Z crash: boom at src/x.rs:1 (thread main)\n  0: frame\n");
    assert!(earlier.len() as u64 > log::MAX_BYTES);
    std::fs::write(&file, &earlier).unwrap();

    log::init(&file, Some(&home));
    assert_eq!(
        std::fs::read_to_string(file.with_extension("old.log")).unwrap(),
        earlier,
        "the full log moved aside"
    );
    assert_eq!(
        log::recent_errors(),
        ["2026-11-20T10:00:01Z crash: boom at src/x.rs:1 (thread main)\n  0: frame"],
        "an earlier run's crash is still offered"
    );

    log::error(&format!(
        "Install failed: {}/AddOns is read-only",
        home.display()
    ));
    let crashed = std::thread::Builder::new()
        .name("worker".into())
        .spawn(|| panic!("worker gave up"))
        .unwrap()
        .join();
    assert!(crashed.is_err());

    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        text.contains(" error: Install failed: ~/AddOns is read-only"),
        "{text}"
    );
    assert!(!text.contains(&home.display().to_string()), "{text}");
    assert!(text.contains(" crash: worker gave up at "), "{text}");
    assert!(
        text.contains("log.rs:"),
        "the crash names its place: {text}"
    );
    assert!(
        text.contains("(thread worker)\n  "),
        "a backtrace follows: {text}"
    );
    let recent = log::recent_errors();
    assert_eq!(recent.len(), 3);
    assert!(recent[2].contains("crash: worker gave up"));
}
