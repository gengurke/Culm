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
use rune::text::TextSystem;
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
    let config =
        core_backend::Config::from_env().expect("no home folder to keep the app's data in");
    State::new(Arc::new(core_backend::CoreBackend::new(config)))
}

/// The window, with named scenes for `--snap --scene <name>`.
pub fn spec() -> AppSpec<State> {
    AppSpec {
        min_size: Some((960.0, 600.0)),
        scenes: scenes(),
        fonts: theme::FONTS,
        families: theme::FAMILIES,
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(on_key),
        ..AppSpec::new(brand::APP_NAME, (1280.0, 800.0), init, view)
    }
}

/// A headless runtime over `state` with the app's fonts and poll wired, for tests.
pub fn runtime(state: State, size: (f32, f32)) -> Runtime<State> {
    let text = TextSystem::new(theme::FONTS, theme::FAMILIES);
    let mut rt = Runtime::with_text(state, view, size, Env::new(true, 1.0), text);
    rt.poll = Some(poll);
    rt.on_key = Some(on_key);
    rt
}

/// Esc closes an open dropdown, then the Get Addons detail panel; `/` focuses the filter or
/// search field when nothing else has focus.
fn on_key(s: &mut State, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if ev.key == Key::Escape {
        return s.menu.take().is_some()
            || (s.screen == Screen::GetAddons && s.detail.take().is_some());
    }
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
    s.api_key
        .replace_text(s.settings.curseforge_key.clone().unwrap_or_default());
    s.installations = vec![mock::installation()];
    s.current = Some(mock::installation().id);
    s.addons = mock::with_latest(&mock::installed_addons())
        .into_iter()
        .map(|addon| AddonUi { addon, op: None })
        .collect();
    s.addons.sort_by_key(|a| a.addon.name.to_lowercase());
    s.job = Job::Idle;
    scene_theme(s);
}

/// `--scene <name>@<theme>` draws a scene in that theme.
// shortcut: scene fns are plain fn pointers, so the theme is read back from the command line;
// in the playground (no `--scene`) every variant shows the stored theme.
fn scene_theme(s: &mut State) {
    if rune::tooling::headless_run().is_none() {
        return;
    }
    let args: Vec<String> = std::env::args().collect();
    let scene = args.iter().skip_while(|a| *a != "--scene").nth(1);
    if let Some((_, id)) = scene.and_then(|n| n.split_once('@')) {
        s.settings.theme = theme::ThemeId::from_id(id);
    }
}

/// Every scene, plus a `<scene>@<theme>` variant per theme.
fn scenes() -> &'static [Scene] {
    let mut all = SCENES.to_vec();
    for id in theme::ThemeId::ALL {
        for (name, f) in SCENES {
            all.push((String::leak(format!("{name}@{}", id.id())), *f));
        }
    }
    all.leak()
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
        set_op(
            s,
            "spoken",
            Op::Failed("Download failed: GitHub answered 503".into()),
        );
        s.toast(
            Tone::Success,
            "Updated Details! Damage Meter",
            "20261113.15433",
        );
    }),
    // Every toast tone at once (other looks: ADDONS_TOAST_STYLE).
    ("toasts", |s| {
        loaded(s);
        s.toast(Tone::Info, "3 updates available", "");
        s.toast(
            Tone::Danger,
            "Could not update WeakAuras",
            "Download failed: GitHub answered 503 Service Unavailable",
        );
        s.toast(
            Tone::Success,
            "Updated Details! Damage Meter",
            "20261113.15433",
        );
        s.toast(
            Tone::Info,
            "CurseForge key removed",
            "CurseForge search is off",
        );
    }),
    // Every status on one screen.
    ("my-addons-badges", |s| {
        loaded(s);
        let keep = [
            "ace3",
            "baganator",
            "bagnon",
            "bartender4",
            "foreverlootsparkles",
            "foreverquesttint",
            "questie",
            "recount",
            "spoken",
            "details",
        ];
        s.addons.retain(|a| keep.contains(&a.addon.id.as_str()));
        if let Some(a) = s.addons.iter_mut().find(|a| a.addon.id == "bagnon") {
            a.addon.latest = None;
        }
        set_op(s, "spoken", Op::Failed("GitHub answered 503".into()));
        s.selected = Some("bartender4".into());
    }),
    // Ordered by status: what needs the user (errors, updates) first.
    ("my-addons-sorted", |s| {
        loaded(s);
        s.settings.addon_order = backend::AddonOrder {
            by: backend::SortBy::Status,
            desc: false,
        };
        set_op(s, "spoken", Op::Failed("GitHub answered 503".into()));
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
        s.installing
            .insert("CurseForge:deadlybossmods".into(), Op::Downloading(0.6));
        s.featured = Fetch::Ready(mock::featured(true));
        s.categories = Fetch::Ready(mock::categories());
    }),
    ("get-addons-search", |s| {
        loaded(s);
        s.screen = Screen::GetAddons;
        s.categories = Fetch::Ready(mock::categories());
        s.query.replace_text("quest");
        s.results = Fetch::Ready(mock::found(&s.search_filter(), true));
    }),
    // A result on CurseForge and WowUp Hub, the Hub picked to install from.
    ("get-addons-detail", |s| {
        loaded(s);
        s.screen = Screen::GetAddons;
        s.categories = Fetch::Ready(mock::categories());
        s.query.replace_text("boss");
        let found = mock::found(&s.search_filter(), true);
        let pick = found.entries.iter().find(|e| e.sources.len() > 1).cloned();
        s.results = Fetch::Ready(found);
        s.detail = pick.map(|entry| {
            let description = Fetch::Ready(Some(mock::description(&entry.sources[1])));
            state::Detail {
                entry,
                source: 1,
                description,
            }
        });
    }),
    // WowUp Hub is down: CurseForge's results and a quiet note.
    ("get-addons-partial", |s| {
        loaded(s);
        s.screen = Screen::GetAddons;
        s.categories = Fetch::Ready(mock::categories());
        s.query.replace_text("quest");
        let mut found = mock::found(&s.search_filter(), true);
        found
            .entries
            .retain(|e| e.preferred().provider == "CurseForge");
        for e in &mut found.entries {
            e.sources.truncate(1);
        }
        found.failed = vec![(
            "WowUpHub".into(),
            "hub.wowup.io answered 503 Service Unavailable".into(),
        )];
        s.results = Fetch::Ready(found);
    }),
    ("get-addons-category", |s| {
        loaded(s);
        s.screen = Screen::GetAddons;
        s.categories = Fetch::Ready(mock::categories());
        s.category = Some(1028); // Warrior
        s.sort = addon_core::Sort::LastUpdated;
        s.results = Fetch::Ready(mock::found(&s.search_filter(), true));
    }),
    ("get-addons-menu", |s| {
        loaded(s);
        s.screen = Screen::GetAddons;
        s.categories = Fetch::Ready(mock::categories());
        s.category = Some(1009); // Bags & Inventory
        s.results = Fetch::Ready(mock::found(&s.search_filter(), true));
        s.menu = Some(state::Menu::Category);
    }),
    ("get-addons-link", |s| {
        loaded(s);
        s.screen = Screen::GetAddons;
        s.query.replace_text("https://github.com/Questie/Questie");
        s.link = Fetch::Ready(
            mock::catalog()
                .into_iter()
                .find(|r| r.name == "Questie Lite")
                .unwrap_or_else(|| mock::catalog().remove(0)),
        );
    }),
    // No key: WowUp Hub answers alone.
    ("get-addons-no-key", |s| {
        loaded(s);
        s.screen = Screen::GetAddons;
        s.settings.curseforge_key = None;
        s.api_key.replace_text("");
        s.featured = Fetch::Ready(mock::featured(false));
    }),
    ("maintenance", |s| {
        loaded(s);
        s.screen = Screen::Maintenance;
        s.backups = vec![
            backend::Backup {
                name: "WTF-2026-11-12_2114.zip".into(),
                created_at: "2026-11-12T21:14:00Z".into(),
                size_bytes: 18_404_311,
            },
            backend::Backup {
                name: "WTF-2026-11-02_0930.zip".into(),
                created_at: "2026-11-02T09:30:00Z".into(),
                size_bytes: 17_990_120,
            },
        ];
        s.maint.insert(
            MaintCard::Cache,
            MaintPhase::Done("Freed 412 MB from Cache".into()),
        );
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
        scene_theme(s);
    }),
];

/// Wayland app id / X11 class, from `brand`.
pub fn spec_app_id() -> &'static str {
    brand::APP_ID
}
