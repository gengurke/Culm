//! UI-only behaviour over the mock backend (filtering, navigation, failure display, game-running
//! lock). Features end to end over the real core live in `e2e_*.rs`.

use addon_app::mock::MockBackend;
use addon_app::state::{MaintCard, Status};
use addon_app::{Op, Screen, State, loaded, runtime};
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
fn f5_3_open_website() {
    let mut rt = app(loaded);
    tap(&mut rt, "addon/ace3");
    tap(&mut rt, "detail/website");
    assert_eq!(rt.take_url_requests(), vec!["https://www.curseforge.com/wow/addons/ace3".to_string()]);
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
