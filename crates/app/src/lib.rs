//! Rune UI of the addon manager: one `State`, a pure `view`, workers over a [`backend::Backend`].

pub mod backend;
mod brand;
pub mod core_backend;
mod debug_info;
pub mod icons;
pub mod mock;
mod scenes;
pub mod state;
pub mod theme;
pub mod ui;
mod view;
pub mod worker;

pub use scenes::loaded;
pub use state::{Intent, Job, Op, Screen, State, dispatch, poll, setup};
pub use view::view;

use mock::MockBackend;
use rune::text::TextSystem;
use rune::{AppSpec, Env, Key, KeyEvent, Runtime, Tx};
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

/// Logs errors and crashes to `<logs>/culm.log` (F7.7); headless runs (`--snap`, `--script`) log nothing.
pub fn start_log() {
    if rune::tooling::headless_run().is_some() {
        return;
    }
    let Some(dirs) = rune::dirs::AppDirs::new(brand::APP_ID) else {
        return;
    };
    let file = dirs
        .logs()
        .join(format!("{}.log", brand::APP_NAME.to_lowercase()));
    addon_core::log::init(&file, dirs.home());
    addon_core::log::info(&format!(
        "{} {} started on {} {}",
        brand::APP_NAME,
        brand::version_label(),
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
}

/// The window, with named scenes for `--snap --scene <name>`.
pub fn spec() -> AppSpec<State> {
    AppSpec {
        min_size: Some((960.0, 600.0)),
        scenes: scenes::all(),
        fonts: theme::FONTS,
        families: theme::FAMILIES,
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(on_key),
        ..AppSpec::new(brand::APP_NAME, (1280.0, 800.0), init, view)
    }
}

/// The app: the window from `spec`, its header band drawn as the titlebar (`view::band`). Where Rune
/// replaces the native titlebar (Windows, macOS) the window's controls sit over the band; elsewhere
/// the native frame stays.
pub fn app() -> rune::App<State> {
    rune::App::new(spec()).titlebar(rune::Titlebar::new(theme::BAND_H))
}

/// A headless runtime over `state` with the app's fonts and poll wired, for tests.
pub fn runtime(state: State, size: (f32, f32)) -> Runtime<State> {
    let text = TextSystem::new(theme::FONTS, theme::FAMILIES);
    let mut rt = Runtime::with_text(state, view, size, Env::new(true, 1.0), text);
    rt.poll = Some(poll);
    rt.on_key = Some(on_key);
    rt
}

/// Esc closes an open menu (a dropdown or the installation menu), then the Get Addons detail panel; `/` focuses the filter or
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

/// Wayland app id / X11 class, from `brand`.
pub fn spec_app_id() -> &'static str {
    brand::APP_ID
}
