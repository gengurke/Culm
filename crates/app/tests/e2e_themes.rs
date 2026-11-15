//! F7.3 Themes: the picker in Settings, persistence, and every theme drawing every screen.

mod common;

use addon_app::mock::MockBackend;
use addon_app::theme::ThemeId;
use addon_app::{Screen, State, loaded, runtime};
use common::World;
use rune::ImageCache;
use rune::prefs::Prefs;
use std::sync::Arc;

/// The page title's colour: each theme draws headings in its own.
fn title_color(app: &common::App) -> rune::Color {
    app.rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("page-title") && !e.exiting)
        .and_then(|e| e.text_color)
        .expect("a page title")
}

#[test]
fn f7_3_theme_choice_applies_and_survives_restart() {
    let world = World::new();
    let mut app = world.app();
    app.wait_idle();
    assert_eq!(app.state().settings.theme, ThemeId::Moonlit);
    app.tap("nav/settings");
    assert_eq!(title_color(&app), ThemeId::Moonlit.theme().pal.heading);

    app.tap("settings/theme/hearthwood");
    assert_eq!(app.state().settings.theme, ThemeId::Hearthwood);
    // Colours spring to the new theme over a few frames.
    app.wait_until("the title in Hearthwood gold", |app| {
        title_color(app) == ThemeId::Hearthwood.theme().pal.heading
    });
    app.wait_until("theme written", |_| {
        std::fs::read_to_string(world.prefs_path()).is_ok_and(|p| p.contains("hearthwood"))
    });

    let mut again = world.app();
    again.wait_idle();
    assert_eq!(again.state().settings.theme, ThemeId::Hearthwood);
    again.tap("nav/settings");
    assert_eq!(title_color(&again), ThemeId::Hearthwood.theme().pal.heading);
}

#[test]
fn f7_3_unknown_stored_theme_falls_back_to_moonlit() {
    let world = World::new();
    Prefs::at(world.prefs_path())
        .set("theme", "retired-theme")
        .unwrap();
    let mut app = world.app();
    app.wait_idle();
    assert_eq!(app.state().settings.theme, ThemeId::Moonlit);
}

#[test]
fn f7_3_every_theme_renders_every_screen() {
    let screens = [
        (Screen::MyAddons, "my-addons/list"),
        (Screen::GetAddons, "get-addons/toolbar"),
        (Screen::Maintenance, "maintenance"),
        (Screen::Settings, "settings/theme"),
    ];
    for &theme in ThemeId::ALL {
        let mut state = State::new(Arc::new(MockBackend::new()));
        loaded(&mut state);
        state.settings.theme = theme;
        // With a cache the textures are generated and painted too.
        state.images = Some(ImageCache::new(Arc::new(|| {})));
        let mut rt = runtime(state, (1280.0, 800.0));
        rt.frame(0.0);
        for (screen, key) in screens {
            rt.update(|s, _| s.screen = screen);
            rt.frame(1.0 / 60.0);
            for k in ["sidebar", "band", "page-title", key] {
                assert!(
                    rt.find(k).is_some(),
                    "{} {screen:?}: nothing keyed {k}",
                    theme.id()
                );
            }
        }
        rt.update(|s, _| {
            s.screen = Screen::MyAddons;
            s.selected = Some("questie".into());
        });
        rt.frame(1.0 / 60.0);
        assert!(
            rt.find("detail/questie").is_some(),
            "{}: the detail panel",
            theme.id()
        );

        let mut fresh = State::new(Arc::new(MockBackend::new()));
        fresh.settings.theme = theme;
        fresh.job = addon_app::Job::Idle;
        let mut rt = runtime(fresh, (1280.0, 800.0));
        rt.frame(0.0);
        assert!(
            rt.find("onboarding/add").is_some(),
            "{}: onboarding",
            theme.id()
        );
    }
}

#[test]
fn f7_3_nightwatch_torch_follows_the_pointer_and_other_themes_have_no_lights() {
    for &theme in ThemeId::ALL {
        let mut state = State::new(Arc::new(MockBackend::new()));
        loaded(&mut state);
        state.settings.theme = theme;
        let mut rt = runtime(state, (1280.0, 800.0));
        rt.frame(0.0);
        assert_eq!(
            rt.find("lights").is_some(),
            theme.lights().is_some(),
            "{}: a lights layer only with lights",
            theme.id()
        );
    }
    let mut state = State::new(Arc::new(MockBackend::new()));
    loaded(&mut state);
    state.settings.theme = ThemeId::Nightwatch;
    let mut rt = runtime(state, (1280.0, 800.0));
    rt.frame(0.0);
    let torch = |rt: &rune::Runtime<State>| {
        let e = rt
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("lights/torch"))
            .expect("the torch");
        // `rect` is [x, y, width, height].
        (
            e.rect[0] + e.rect[2] / 2.0,
            e.rect[1] + e.rect[3] / 2.0,
            e.opacity,
        )
    };
    assert_eq!(torch(&rt).2, 0.0, "no torch before the pointer comes in");
    rt.pointer_move(500.0, 300.0);
    for _ in 0..60 {
        rt.frame(1.0 / 60.0);
    }
    let commits = rt.stats.commits;
    for i in 1..=20 {
        rt.pointer_move(500.0 + i as f32 * 10.0, 300.0 + i as f32 * 7.0);
        rt.frame(1.0 / 60.0);
    }
    // Debug builds still re-run the view for a live-only event a quarter second after the last
    // commit, to check it; release builds don't.
    assert!(
        rt.stats.commits - commits <= 2,
        "moving the torch rebuilt the view {} times",
        rt.stats.commits - commits
    );
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    let (x, y, lit) = torch(&rt);
    assert!(lit > 0.9, "the torch is lit: {lit}");
    assert!(
        (x - 700.0).abs() < 1.0 && (y - 440.0).abs() < 1.0,
        "the torch is centred on the pointer, at {x},{y}"
    );
}
