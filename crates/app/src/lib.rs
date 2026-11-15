//! Rune UI of the addon manager: one `State`, a pure `view`, workers over a [`backend::Backend`].

pub mod backend;
mod brand;
pub mod core_backend;
pub mod icons;
pub mod mock;
pub mod state;
pub mod theme;
pub mod ui;
mod view;
pub mod worker;

pub use state::{Intent, Job, Op, Screen, State, dispatch, poll, setup};
pub use view::view;

use addon_core::Channel;
use mock::MockBackend;
use rune::{AppSpec, Env, Key, KeyEvent, Runtime, Tx};
use state::{AddonUi, Fetch, MaintCard, MaintPhase, Tone};
use std::sync::Arc;

/// The real backend, except in headless binary runs (`--snap`, `--script`), which show the
/// deterministic mock data unless `ADDONS_BACKEND=core` asks for the real one.
pub fn init() -> State {
    let headless = rune::tooling::headless_run().is_some();
    if headless && std::env::var("ADDONS_BACKEND").as_deref() != Ok("core") {
        return State::new(Arc::new(MockBackend::new()));
    }
    let config = core_backend::Config::from_env().expect("no home folder to keep the app's data in");
    State::new(Arc::new(core_backend::CoreBackend::new(config)))
}

/// The window, with named scenes for `--snap --scene <name>`.
pub fn spec() -> AppSpec<State> {
    AppSpec {
        min_size: Some((960.0, 600.0)),
        scenes: SCENES,
        families: theme::FAMILIES,
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(on_key),
        ..AppSpec::new(brand::APP_NAME, (1280.0, 800.0), init, view)
    }
}

/// A headless runtime over `state` with the app's poll wired, for tests.
pub fn runtime(state: State, size: (f32, f32)) -> Runtime<State> {
    let mut rt = Runtime::new(state, view, size, Env::new(true, 1.0));
    rt.poll = Some(poll);
    rt.on_key = Some(on_key);
    rt
}

/// `/` focuses the filter or search field when nothing else has focus.
fn on_key(s: &mut State, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if tx.focused().is_some() || !matches!(&ev.key, Key::Text(t) if t == "/") {
        return false;
    }
    match s.screen {
        // Setting `focused` once asks the runtime to focus the field (rune README, "Text fields").
        Screen::MyAddons => s.filter.focused = true,
        Screen::GetAddons => s.query.focused = true,
        _ => return false,
    }
    true
}

/// State as it is once the mock installation is scanned and checked (no workers involved).
pub fn loaded(s: &mut State) {
    s.settings = s.backend.load_settings();
    s.api_key.replace_text(s.settings.curseforge_key.clone().unwrap_or_default());
    s.installations = vec![mock::installation()];
    s.current = Some(mock::installation().id);
    s.addons = mock::with_latest(&mock::installed_addons()).into_iter().map(|addon| AddonUi { addon, op: None }).collect();
    s.addons.sort_by_key(|a| a.addon.name.to_lowercase());
    s.job = Job::Idle;
}

fn set_op(s: &mut State, id: &str, op: Op) {
    if let Some(a) = s.addons.iter_mut().find(|a| a.addon.id == id) {
        a.op = Some(op);
    }
}

type Scene = (&'static str, fn(&mut State));

const SCENES: &[Scene] = &[
    ("my-addons", loaded),
    ("my-addons-updating", |s| {
        loaded(s);
        set_op(s, "baganator", Op::Downloading(0.42));
        set_op(s, "chattynator", Op::Downloading(0.88));
        set_op(s, "bartender4", Op::Installing);
        set_op(s, "spoken", Op::Failed("Download failed: GitHub answered 503".into()));
        s.toast(Tone::Success, "Updated Details! Damage Meter", "20261113.15433");
    }),
    ("my-addons-detail", |s| {
        loaded(s);
        s.selected = Some("questie".into());
    }),
    ("my-addons-scanning", |s| {
        loaded(s);
        s.addons.clear();
        s.job = Job::Scanning;
    }),
    ("get-addons", |s| {
        loaded(s);
        s.screen = Screen::GetAddons;
        let mut featured = mock::catalog();
        featured.sort_by_key(|r| std::cmp::Reverse(r.download_count));
        featured.truncate(12);
        s.installing.insert("deadlybossmods".into(), Op::Downloading(0.6));
        s.featured = Fetch::Ready(featured);
    }),
    ("get-addons-search", |s| {
        loaded(s);
        s.screen = Screen::GetAddons;
        s.query.replace_text("bag");
        s.results = Fetch::Ready(mock::catalog().into_iter().filter(|r| r.name.to_lowercase().contains("bag")).collect());
    }),
    ("get-addons-no-key", |s| {
        loaded(s);
        s.screen = Screen::GetAddons;
        s.settings.curseforge_key = None;
        s.api_key.replace_text("");
    }),
    ("maintenance", |s| {
        loaded(s);
        s.screen = Screen::Maintenance;
        s.backups = vec![
            backend::Backup { name: "WTF-2026-11-12_2114.zip".into(), created_at: "2026-11-12T21:14:00Z".into(), size_bytes: 18_404_311 },
            backend::Backup { name: "WTF-2026-11-02_0930.zip".into(), created_at: "2026-11-02T09:30:00Z".into(), size_bytes: 17_990_120 },
        ];
        s.maint.insert(MaintCard::Cache, MaintPhase::Done("Freed 412 MB from Cache".into()));
        s.maint.insert(MaintCard::Leftovers, MaintPhase::Confirm);
    }),
    ("maintenance-game-running", |s| {
        loaded(s);
        s.screen = Screen::Maintenance;
        s.game_running = true;
    }),
    ("settings", |s| {
        loaded(s);
        s.screen = Screen::Settings;
        s.settings.default_channel = Channel::Stable;
    }),
    ("onboarding", |s| {
        s.settings = s.backend.load_settings();
        s.job = Job::Idle;
    }),
];

/// Wayland app id / X11 class, from `brand`.
pub fn spec_app_id() -> &'static str {
    brand::APP_ID
}
