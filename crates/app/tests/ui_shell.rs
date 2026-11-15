//! The UI shell driven headlessly against the mock backend: tap by key, wait for workers,
//! assert on visible text and state.

use addon_app::mock::MockBackend;
use addon_app::state::{Fetch, MaintCard, MaintPhase, Status};
use addon_app::{Job, Op, Screen, State, loaded, runtime};
use rune::{Key, KeyEvent, Mods, Runtime};
use std::sync::Arc;
use std::time::{Duration, Instant};

const SIZE: (f32, f32) = (1280.0, 800.0);

fn fresh() -> State {
    State::new(Arc::new(MockBackend::with_delay(Duration::from_millis(30))))
}

fn app(prepare: fn(&mut State)) -> Runtime<State> {
    let mut s = fresh();
    prepare(&mut s);
    let mut rt = runtime(s, SIZE);
    rt.frame(0.0);
    rt
}

fn tap(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element keyed {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
}

/// Steps frames (and real time, for the worker threads) until `done` holds.
fn wait(rt: &mut Runtime<State>, what: &str, done: impl Fn(&State) -> bool) {
    let start = Instant::now();
    while !done(&rt.state) {
        assert!(start.elapsed() < Duration::from_secs(10), "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(5));
        rt.frame(1.0 / 60.0);
    }
    rt.frame(1.0 / 60.0);
}

fn shows(rt: &Runtime<State>, needle: &str) -> bool {
    rt.elements().iter().any(|e| e.text.as_deref().is_some_and(|t| t.contains(needle)) && !e.exiting)
}

fn type_text(rt: &mut Runtime<State>, t: &str) {
    rt.key(KeyEvent { key: Key::Text(t.into()), mods: Mods::default() });
    rt.frame(1.0 / 60.0);
}

#[test]
fn f2_1_boot_scans_and_checks_updates() {
    let mut rt = app(|_| {});
    wait(&mut rt, "boot, scan and update check", |s| s.job == Job::Idle && !s.addons.is_empty());
    assert!(rt.state.update_count() > 0);
    assert!(shows(&rt, "Ace3"));
    assert!(rt.find("addon/ace3").is_some());
    assert!(shows(&rt, "Update all ("));
}

#[test]
fn f1_2_no_installation_shows_onboarding() {
    let mut rt = app(|s| s.job = Job::Idle);
    assert!(rt.find("onboarding/add").is_some());
    tap(&mut rt, "onboarding/add");
    assert!(rt.file_dialog().is_some(), "folder picker requested");
    let _ = rt.resolve_file_dialog(Some("/games/World of Warcraft".into()));
    rt.frame(1.0 / 60.0);
    wait(&mut rt, "installation added and scanned", |s| s.installation().is_some() && s.job == Job::Idle && !s.addons.is_empty());
}

#[test]
fn navigation_switches_screens() {
    let mut rt = app(loaded);
    for (key, screen, title) in [
        ("nav/get-addons", Screen::GetAddons, "Get Addons"),
        ("nav/maintenance", Screen::Maintenance, "Maintenance"),
        ("nav/settings", Screen::Settings, "Settings"),
        ("nav/my-addons", Screen::MyAddons, "My Addons"),
    ] {
        tap(&mut rt, key);
        assert_eq!(rt.state.screen, screen);
        let heading = rt.elements().into_iter().find(|e| e.key.as_deref() == Some("page-title") && !e.exiting).unwrap();
        assert_eq!(heading.text.as_deref(), Some(title));
    }
}

#[test]
fn f2_1_filter_narrows_the_list() {
    let mut rt = app(loaded);
    assert!(rt.find("addon/ace3").is_some());
    tap(&mut rt, "my-addons/filter");
    type_text(&mut rt, "details");
    assert_eq!(rt.state.filter.text, "details");
    assert!(rt.find("addon/detailsdamagemeter").is_some());
    assert!(rt.find("addon/ace3").is_none());
    type_text(&mut rt, "zzz");
    assert!(shows(&rt, "No addons match"));
}

#[test]
fn slash_focuses_the_filter() {
    let mut rt = app(loaded);
    type_text(&mut rt, "/");
    type_text(&mut rt, "pawn");
    assert_eq!(rt.state.filter.text, "pawn");
    assert!(rt.find("addon/pawn").is_some());
}

#[test]
fn f3_2_update_button_updates_addon() {
    let mut rt = app(loaded);
    assert_eq!(rt.state.addon("baganator").unwrap().status(), Status::UpdateAvailable);
    tap(&mut rt, "addon/baganator/update");
    assert!(matches!(rt.state.addon("baganator").unwrap().op, Some(Op::Downloading(_) | Op::Installing)));
    wait(&mut rt, "update to finish", |s| s.addon("baganator").unwrap().op.is_none());
    let a = rt.state.addon("baganator").unwrap();
    assert_eq!(a.status(), Status::UpToDate);
    assert_eq!(a.addon.installed_version.as_deref(), Some("861"));
    assert!(shows(&rt, "Updated Baganator"));
}

#[test]
fn f3_2_failed_update_shows_error() {
    let mut rt = app(loaded);
    // Spoken is below the fold of the virtual list: select it as the row press would.
    rt.update(|s, _| s.selected = Some("spoken".into()));
    rt.frame(1.0 / 60.0);
    tap(&mut rt, "detail/update");
    wait(&mut rt, "update to fail", |s| matches!(s.addon("spoken").unwrap().op, Some(Op::Failed(_))));
    assert_eq!(rt.state.addon("spoken").unwrap().status(), Status::Error);
    assert!(shows(&rt, "GitHub answered 503"));
}

#[test]
fn f3_2_update_all_updates_everything_available() {
    let mut rt = app(loaded);
    let n = rt.state.update_count();
    assert!(n > 5);
    tap(&mut rt, "my-addons/update-all");
    wait(&mut rt, "all updates to settle", |s| s.addons.iter().all(|a| !a.op.as_ref().is_some_and(Op::busy)));
    assert_eq!(rt.state.update_count(), 0, "only the failing addon is left, as Error");
    assert_eq!(rt.state.addons.iter().filter(|a| a.status() == Status::Error).count(), 1);
}

#[test]
fn f3_4_ignore_from_detail_panel() {
    let mut rt = app(loaded);
    tap(&mut rt, "addon/baganator");
    assert!(rt.find("detail/baganator").is_some());
    tap(&mut rt, "detail/ignore");
    assert_eq!(rt.state.addon("baganator").unwrap().status(), Status::Ignored);
    assert!(rt.find("addon/baganator/update").is_none());
    tap(&mut rt, "detail/ignore");
    assert_eq!(rt.state.addon("baganator").unwrap().status(), Status::UpdateAvailable);
}

#[test]
fn f5_1_remove_needs_confirm() {
    let mut rt = app(loaded);
    tap(&mut rt, "addon/ace3");
    tap(&mut rt, "detail/remove");
    assert!(rt.state.addon("ace3").is_some(), "first press only asks");
    tap(&mut rt, "detail/remove-confirm");
    wait(&mut rt, "remove", |s| s.addon("ace3").is_none());
    assert!(rt.find("addon/ace3").is_none());
    assert!(shows(&rt, "Removed Ace3"));
}

#[test]
fn f5_3_open_website() {
    let mut rt = app(loaded);
    tap(&mut rt, "addon/ace3");
    tap(&mut rt, "detail/website");
    assert_eq!(rt.take_url_requests(), vec!["https://www.curseforge.com/wow/addons/ace3".to_string()]);
}

#[test]
fn f4_2_popular_list_loads() {
    let mut rt = app(loaded);
    tap(&mut rt, "nav/get-addons");
    wait(&mut rt, "popular list", |s| matches!(s.featured, Fetch::Ready(_)));
    assert!(shows(&rt, "POPULAR FOR FOREVER"));
    assert!(rt.find("remote/deadlybossmods/install").is_some());
}

#[test]
fn f4_1_search_shows_results_and_f4_3_install_marks_installed() {
    let mut rt = app(loaded);
    tap(&mut rt, "nav/get-addons");
    tap(&mut rt, "get-addons/search");
    type_text(&mut rt, "elvui");
    assert_eq!(rt.state.query.text, "elvui");
    wait(&mut rt, "search results", |s| matches!(&s.results, Fetch::Ready(r) if !r.is_empty()));
    assert!(shows(&rt, "Tukui Team"));
    tap(&mut rt, "remote/elvui/install");
    assert!(rt.state.installing.contains_key("elvui"));
    wait(&mut rt, "install", |s| s.addons.iter().any(|a| a.addon.id == "elvui"));
    assert!(rt.find("remote/elvui/installed").is_some());
    assert!(shows(&rt, "Installed ElvUI"));
}

#[test]
fn f7_1_without_key_get_addons_points_to_settings() {
    let mut rt = app(|s| {
        loaded(s);
        s.settings.curseforge_key = None;
        s.api_key.replace_text("");
    });
    tap(&mut rt, "nav/get-addons");
    assert!(rt.find("get-addons/no-key").is_some());
    tap(&mut rt, "get-addons/open-settings");
    assert_eq!(rt.state.screen, Screen::Settings);
    tap(&mut rt, "settings/api-key");
    type_text(&mut rt, "my-key");
    tap(&mut rt, "settings/api-key/save");
    assert!(rt.state.curseforge_ready());
    tap(&mut rt, "nav/get-addons");
    assert!(rt.find("get-addons/no-key").is_none());
}

#[test]
fn f6_1_clear_cache_confirm_flow() {
    let mut rt = app(loaded);
    tap(&mut rt, "nav/maintenance");
    tap(&mut rt, "maint/cache/scope/onlywdb");
    assert!(rt.state.cache_wdb_only);
    tap(&mut rt, "maint/cache/run");
    assert_eq!(rt.state.maint.get(&MaintCard::Cache), Some(&MaintPhase::Confirm));
    tap(&mut rt, "maint/cache/cancel");
    assert_eq!(rt.state.maint.get(&MaintCard::Cache), None);
    tap(&mut rt, "maint/cache/run");
    tap(&mut rt, "maint/cache/confirm");
    wait(&mut rt, "cache cleared", |s| matches!(s.maint.get(&MaintCard::Cache), Some(MaintPhase::Done(_))));
    assert!(shows(&rt, "Freed 38 MB from Cache/WDB"));
}

#[test]
fn f6_3_wtf_backup_lists_new_backup() {
    let mut rt = app(loaded);
    tap(&mut rt, "nav/maintenance");
    wait(&mut rt, "backups listed", |s| s.backups.len() == 2);
    tap(&mut rt, "maint/wtf/run");
    tap(&mut rt, "maint/wtf/confirm");
    wait(&mut rt, "backup", |s| s.backups.len() == 3);
    assert!(shows(&rt, "WTF-2026-11-15_1402.zip"));
}

#[test]
fn f6_1_maintenance_locked_while_game_runs() {
    let mut rt = app(|s| {
        loaded(s);
        s.screen = Screen::Maintenance;
        s.game_running = true;
    });
    rt.frame(1.0 / 60.0);
    assert!(rt.find("maintenance/game-running").is_some());
    tap(&mut rt, "maint/cache/run");
    assert_eq!(rt.state.maint.get(&MaintCard::Cache), None, "disabled button does nothing");
}
