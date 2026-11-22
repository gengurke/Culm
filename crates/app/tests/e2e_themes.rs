//! F7.3 Themes: the picker in Settings, persistence, every theme drawing every screen in bundled fonts.

mod common;

use addon_app::mock::MockBackend;
use addon_app::theme::{FAMILIES, FONTS, ThemeId, Type};
use addon_app::{Screen, State, loaded, runtime};
use common::World;
use rune::ImageCache;
use rune::prefs::Prefs;
use rune::tok::{Family, TextStyle};
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
fn f7_3_every_theme_shows_its_swatches() {
    for &theme in ThemeId::ALL {
        let mut state = State::new(Arc::new(MockBackend::new()));
        loaded(&mut state);
        state.settings.theme = theme;
        state.screen = Screen::Swatches;
        let mut rt = runtime(state, (1280.0, 800.0));
        rt.frame(0.0);
        let th = theme.theme();
        let mut keys: Vec<String> = th
            .skin
            .surfaces()
            .iter()
            .map(|(name, _)| format!("swatches/skin/{name}"))
            .collect();
        keys.extend(
            [
                "page-title",
                "swatches/page/accent",
                "swatches/type/mono",
                "swatches/status",
            ]
            .map(String::from),
        );
        if !std::ptr::eq(th, th.frame_look()) {
            keys.push("swatches/frame/accent".into());
        }
        for k in keys {
            assert!(rt.find(&k).is_some(), "{}: nothing keyed {k}", theme.id());
        }
    }
}

/// Every role's style in a type ladder.
fn styles(ty: &Type) -> [(&'static str, TextStyle); 13] {
    [
        ("display", ty.display),
        ("hero", ty.hero),
        ("title", ty.title),
        ("heading", ty.heading),
        ("body", ty.body),
        ("strong", ty.strong),
        ("label", ty.label),
        ("note", ty.note),
        ("small", ty.small),
        ("number", ty.number),
        ("caption", ty.caption),
        ("eyebrow", ty.eyebrow),
        ("mono", ty.mono),
    ]
}

#[test]
fn f7_3_every_theme_font_is_bundled() {
    let bundled: Vec<String> = FONTS
        .iter()
        .map(|data| {
            let font = swash::FontRef::from_index(data, 0).expect("a bundled font parses");
            let name = font
                .localized_strings()
                .find_by_id(swash::StringId::Family, None)
                .expect("a family name");
            name.to_string()
        })
        .collect();
    for slot in [FAMILIES.0, FAMILIES.1] {
        assert!(
            bundled.iter().any(|b| b == slot),
            "slot family {slot} isn't bundled: {bundled:?}"
        );
    }

    // Shaped in the app's own text system, a named family draws differently from the fallback,
    // so the name reaches a real face (and no system font: the name is one of the bundled).
    let mut rt = runtime(State::new(Arc::new(MockBackend::new())), (800.0, 600.0));
    let sample = "Culm Addon Manager 1.2.3";
    for &id in ThemeId::ALL {
        let th = id.theme();
        for th in std::iter::once(th).chain(th.frame.map(|f| &**f)) {
            for (role, style) in styles(&th.ty) {
                let Family::Named(name) = style.family else {
                    assert!(
                        matches!(style.family, Family::Sans | Family::Mono),
                        "{} {role}: {:?} is a system family",
                        th.id,
                        style.family
                    );
                    continue;
                };
                assert!(
                    bundled.iter().any(|b| b == name),
                    "{} {role}: {name:?} isn't one bundled family ({bundled:?})",
                    th.id
                );
                let missing = style.family(Family::Named("No Such Family"));
                assert_ne!(
                    rt.text.measure(sample, style, 1.0),
                    rt.text.measure(sample, missing, 1.0),
                    "{} {role}: {name} draws as the fallback",
                    th.id
                );
            }
        }
    }
}

/// Spellings that decide a look outside `theme/`: raw colours, theme checks and the frame theme.
const LOOKS: [&str; 7] = [
    "h(\"",
    "hex(",
    "oklch",
    "ThemeId::",
    ".frame",
    "Color::BLACK",
    "Color::WHITE",
];

/// Where `ui.rs` and `view/` may use one of `LOOKS`.
enum Allow {
    /// Every line of the function whose signature line holds this (up to its closing `}` in column 0).
    Fn(&'static str),
    /// The one line holding this fragment.
    Line(&'static str),
}

/// The exceptions, as (file, where, why). Each must still match (a `Line` exactly one line), so the
/// list can't go stale or silently cover new lines.
const ALLOWED: &[(&str, Allow, &str)] = &[
    (
        "ui.rs",
        Allow::Fn("pub fn class_medal("),
        "class medallion: the disc, sheen, emblem ink, lip and drop are shaded from the class colour, \
         not the theme; only the ring comes from `Skin::medal`",
    ),
    (
        "view/settings.rs",
        Allow::Line("for id in ThemeId::ALL"),
        "the theme picker lists every theme",
    ),
    (
        "view/toasts.rs",
        Allow::Line("tone.mix(Color::WHITE, 0.45)"),
        "lifts a dark status tone off a dark ground (a contrast rule)",
    ),
];

fn ident(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// `line` without its `//` comment; a `//` inside a string literal (a URL) stays.
fn code_of(line: &str) -> &str {
    let b = line.as_bytes();
    let (mut i, mut in_str) = (0, false);
    while i < b.len() {
        match b[i] {
            b'\\' if in_str => i += 1,
            b'"' => in_str = !in_str,
            // A quote as a char literal ('"') opens no string.
            b'\'' if b.get(i + 1) == Some(&b'"') && b.get(i + 2) == Some(&b'\'') => i += 2,
            b'/' if !in_str && b.get(i + 1) == Some(&b'/') => return &line[..i],
            _ => {}
        }
        i += 1;
    }
    line
}

/// The `LOOKS` spellings in `line`'s code, each as a whole name: `h("` but not `push("`; `.frame`
/// as the field, not the `frame_look()` accessor or `framed(..)` that views reach the frame by.
fn looks_in(line: &str) -> Vec<&'static str> {
    let code = code_of(line);
    LOOKS
        .into_iter()
        .filter(|pat| {
            code.match_indices(pat).any(|(i, _)| {
                let before = code[..i].chars().next_back();
                let after = code[i + pat.len()..].chars().next();
                if *pat == ".frame" {
                    !after.is_some_and(ident)
                } else {
                    !before.is_some_and(ident)
                }
            })
        })
        .collect()
}

#[test]
fn f7_3_views_leave_looks_to_the_theme() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = vec!["ui.rs".to_string()];
    for entry in std::fs::read_dir(src.join("view")).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if name.ends_with(".rs") {
            files.push(format!("view/{name}"));
        }
    }
    let mut hits = vec![0; ALLOWED.len()];
    let mut found = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(src.join(file)).unwrap();
        let mut in_fn = None;
        for (n, line) in text.lines().enumerate() {
            if in_fn.is_none() {
                in_fn = ALLOWED.iter().position(
                    |(f, a, _)| matches!(a, Allow::Fn(sig) if f == file && line.contains(sig)),
                );
            }
            if !looks_in(line).is_empty() {
                let allowed = in_fn.or_else(|| {
                    ALLOWED.iter().position(
                        |(f, a, _)| matches!(a, Allow::Line(frag) if f == file && line.contains(frag)),
                    )
                });
                match allowed {
                    Some(i) => hits[i] += 1,
                    None => found.push(format!("{file}:{}: `{}`", n + 1, line.trim())),
                }
            }
            if line == "}" {
                in_fn = None;
            }
        }
    }
    assert!(
        found.is_empty(),
        "looks belong in the theme (Pal, Skin, theme/ helpers; see docs/THEMES.md), not in views:\n{}",
        found.join("\n")
    );
    for ((file, allow, _), n) in ALLOWED.iter().zip(hits) {
        match allow {
            Allow::Fn(sig) => assert!(
                n > 0,
                "{file}: `{sig}` no longer needs its allow-list entry"
            ),
            Allow::Line(frag) => assert_eq!(n, 1, "{file}: `{frag}` should allow exactly one line"),
        }
    }
}

#[test]
fn f7_3_look_lint_sees_what_it_should() {
    assert_eq!(looks_in(r##"    .fill(h("#fff"))"##), ["h(\""]);
    assert_eq!(looks_in(r##"    .fill(theme::h("#fff"))"##), ["h(\""]);
    assert!(looks_in(r#"    if url.starts_with("https://") {"#).is_empty());
    assert!(looks_in(r#"    path.push("a");"#).is_empty());
    assert_eq!(looks_in("    if th.frame.is_some() {"), [".frame"]);
    assert!(looks_in("    let fr = th.frame_look();").is_empty());
    assert!(looks_in("    // Color::BLACK in a comment").is_empty());
    assert_eq!(
        looks_in("    .color(Color::WHITE) // white"),
        ["Color::WHITE"]
    );
    assert_eq!(
        looks_in(r#"    let u = "https://x"; let c = Color::BLACK;"#),
        ["Color::BLACK"]
    );
    assert_eq!(
        looks_in(r#"    let q = '"'; let c = Color::BLACK; // "x""#),
        ["Color::BLACK"]
    );
    // An escaped quote doesn't end the string, so its `//` is no comment.
    assert_eq!(
        looks_in(r#"    let s = "a \" // Color::BLACK"; // x"#),
        ["Color::BLACK"]
    );
}
