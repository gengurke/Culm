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
    let (x, y) = rt
        .find(key)
        .unwrap_or_else(|| panic!("no element keyed {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
}

/// Steps frames (and real time, for the worker threads) until `done` holds.
fn wait(rt: &mut Runtime<State>, what: &str, done: impl Fn(&State) -> bool) {
    let start = Instant::now();
    while !done(&rt.state) {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "timed out waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(5));
        rt.frame(1.0 / 60.0);
    }
    rt.frame(1.0 / 60.0);
}

fn shows(rt: &Runtime<State>, needle: &str) -> bool {
    rt.elements()
        .iter()
        .any(|e| e.text.as_deref().is_some_and(|t| t.contains(needle)) && !e.exiting)
}

fn type_text(rt: &mut Runtime<State>, t: &str) {
    rt.key(KeyEvent {
        key: Key::Text(t.into()),
        mods: Mods::default(),
    });
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
        let heading = rt
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("page-title") && !e.exiting)
            .unwrap();
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

/// Names of the rows in display order.
fn order(rt: &Runtime<State>) -> Vec<String> {
    rt.state
        .visible_addons()
        .iter()
        .map(|a| a.addon.name.clone())
        .collect()
}

#[test]
fn f2_6_column_headers_sort_the_list() {
    use addon_app::backend::{AddonOrder, SortBy};
    let mut rt = app(loaded);
    let by_name = order(&rt);
    assert!(
        by_name
            .windows(2)
            .all(|w| w[0].to_lowercase() <= w[1].to_lowercase())
    );

    // Addon again: Z to A, the chevron flips.
    tap(&mut rt, "addons/sort/addon");
    assert_eq!(
        rt.state.settings.addon_order,
        AddonOrder {
            by: SortBy::Name,
            desc: true
        }
    );
    let mut reversed = by_name.clone();
    reversed.reverse();
    assert_eq!(order(&rt), reversed);
    assert!(
        rt.find(&format!("addon/{}", rt.state.visible_addons()[0].addon.id))
            .is_some()
    );

    // Status: what needs the user first (an error, then updates), then the rest.
    rt.update(|s, _| {
        s.addons
            .iter_mut()
            .find(|a| a.addon.id == "spoken")
            .unwrap()
            .op = Some(Op::Failed("503".into()))
    });
    tap(&mut rt, "addons/sort/status");
    let statuses: Vec<Status> = rt
        .state
        .visible_addons()
        .iter()
        .map(|a| a.status())
        .collect();
    assert_eq!(statuses[0], Status::Error);
    assert_eq!(statuses[1], Status::UpdateAvailable);
    let last_update = statuses
        .iter()
        .rposition(|s| *s == Status::UpdateAvailable)
        .unwrap();
    assert!(
        statuses[..=last_update]
            .iter()
            .all(|s| matches!(s, Status::Error | Status::UpdateAvailable))
    );
    assert!(
        rt.find("addon/spoken").is_some(),
        "the first row is on screen"
    );

    // The filter narrows the ordered list; the order holds.
    tap(&mut rt, "my-addons/filter");
    type_text(&mut rt, "b");
    let filtered: Vec<Status> = rt
        .state
        .visible_addons()
        .iter()
        .map(|a| a.status())
        .collect();
    assert!(
        filtered
            .windows(2)
            .all(|w| !(w[0] == Status::UpToDate && w[1] == Status::UpdateAvailable))
    );
    assert!(rt.state.visible_addons().iter().all(|a| {
        let q = "b";
        a.addon.name.to_lowercase().contains(q) || a.addon.author.to_lowercase().contains(q)
    }));

    // Keyboard: Space on the focused header flips it (Return goes to buttons only on the web); the choice is saved with the settings.
    rt.update(|_, tx| tx.focus("addons/sort/status", true));
    rt.frame(1.0 / 60.0);
    type_text(&mut rt, " ");
    assert_eq!(
        rt.state.settings.addon_order,
        AddonOrder {
            by: SortBy::Status,
            desc: true
        }
    );
    wait(&mut rt, "the order saved", |s| {
        s.backend.load_settings().addon_order
            == AddonOrder {
                by: SortBy::Status,
                desc: true,
            }
    });
    assert_eq!(AddonOrder::from_id("status-desc").id(), "status-desc");
    assert_eq!(AddonOrder::from_id("nonsense"), AddonOrder::default());
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
    wait(&mut rt, "update to fail", |s| {
        matches!(s.addon("spoken").unwrap().op, Some(Op::Failed(_)))
    });
    assert_eq!(rt.state.addon("spoken").unwrap().status(), Status::Error);
    assert!(shows(&rt, "GitHub answered 503"));
}

#[test]
fn f5_3_open_website() {
    let mut rt = app(loaded);
    tap(&mut rt, "addon/ace3");
    tap(&mut rt, "detail/website");
    assert_eq!(
        rt.take_url_requests(),
        vec!["https://www.curseforge.com/wow/addons/ace3".to_string()]
    );
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
    assert_eq!(
        rt.state.maint.get(&MaintCard::Cache),
        None,
        "disabled button does nothing"
    );
}

#[test]
fn late_results_never_land_on_the_wrong_installation_or_restart_updates() {
    use addon_app::worker::Msg;
    let mut s = fresh();
    loaded(&mut s);
    let current = s.current.clone().unwrap();
    let before: Vec<String> = s.addons.iter().map(|a| a.addon.name.clone()).collect();

    // The scan of an installation the user switched away from answers late: dropped.
    s.apply(Msg::Scanned {
        inst: "other-installation".into(),
        result: Ok((vec![], vec![])),
    });
    assert_eq!(
        s.addons
            .iter()
            .map(|a| a.addon.name.clone())
            .collect::<Vec<_>>(),
        before
    );

    // A rescan of the current installation keeps showing a running update, so it can't start twice.
    let id = s.addons[0].addon.id.clone();
    s.addons[0].op = Some(Op::Downloading(0.5));
    let list = s.addons.iter().map(|a| a.addon.clone()).collect();
    s.apply(Msg::Scanned {
        inst: current,
        result: Ok((list, vec![])),
    });
    assert_eq!(s.addon(&id).unwrap().op, Some(Op::Downloading(0.5)));
}

/// Meaningful actions carry a short help text (a tooltip after a rest of the pointer), and it
/// shows when the pointer rests on the button.
#[test]
fn ui_buttons_explain_themselves() {
    let mut rt = app(|s| {
        loaded(s);
        s.selected = Some("questie".into());
    });
    rt.frame(1.0 / 60.0);
    let help = |rt: &Runtime<State>, key: &str| {
        rt.elements()
            .iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap_or_else(|| panic!("no element keyed {key}"))
            .help
            .clone()
            .unwrap_or_default()
    };
    for (key, needle) in [
        ("my-addons/rescan", "Read the AddOns folder again"),
        ("my-addons/check", "newer versions"),
        ("my-addons/update-all", "every waiting update"),
        ("addons/sort/status", "Sort by status"),
        ("detail/reinstall", "replace the addon's folders"),
        ("detail/ignore", "Skip this addon"),
        ("detail/channel/beta", "betas"),
        ("detail/website", "browser"),
    ] {
        let h = help(&rt, key);
        assert!(h.contains(needle), "{key}: help {h:?} lacks {needle:?}");
    }
    // A glyph-only button's help names it.
    assert_eq!(help(&rt, "detail/close"), "Close details");

    let (x, y) = rt.find("my-addons/rescan").expect("rescan button");
    rt.pointer_move(x, y);
    rt.frame(1.0 / 60.0);
    rt.frame(1.0);
    let (tip, _) = rt
        .help_tooltip()
        .expect("a tooltip after resting on Rescan");
    assert!(tip.starts_with("Read the AddOns folder again"), "{tip}");
}
