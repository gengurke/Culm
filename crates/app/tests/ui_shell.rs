//! UI-only behaviour over the mock backend (filtering, navigation, failure display, game-running
//! lock). Features end to end over the real core live in `e2e_*.rs`.

mod common;
use addon_app::state::{MaintCard, Status, Tone};
use addon_app::theme::{BAND_H, SIDEBAR_W};
use addon_app::{Op, Screen, loaded, runtime};
use common::{App, mock_state};
use rune::{ChromeInsets, TitlebarAction};
use std::sync::Arc;
use std::time::Duration;

const SIZE: (f32, f32) = (1280.0, 800.0);

#[test]
fn navigation_switches_screens() {
    let mut app = App::mock(loaded);
    for (key, screen, title) in [
        ("nav/get-addons", Screen::GetAddons, "Get Addons"),
        ("nav/maintenance", Screen::Maintenance, "Maintenance"),
        ("nav/settings", Screen::Settings, "Settings"),
        ("nav/my-addons", Screen::MyAddons, "My Addons"),
    ] {
        app.tap(key);
        assert_eq!(app.state().screen, screen);
        let heading = app
            .rt
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("page-title") && !e.exiting)
            .unwrap();
        assert_eq!(heading.text.as_deref(), Some(title));
    }
}

#[test]
fn f2_1_filter_narrows_the_list() {
    let mut app = App::mock(loaded);
    assert!(app.has("addon/ace3"));
    app.tap("my-addons/filter");
    app.type_text("details");
    assert_eq!(app.state().filter.text, "details");
    assert!(app.has("addon/detailsdamagemeter"));
    assert!(!app.has("addon/ace3"));
    app.type_text("zzz");
    assert!(app.shows("No addons match"));
}

/// Names of the rows in display order.
fn order(app: &App) -> Vec<String> {
    app.state()
        .visible_addons()
        .iter()
        .map(|a| a.addon.name.clone())
        .collect()
}

#[test]
fn f2_6_column_headers_sort_the_list() {
    use addon_app::backend::{AddonOrder, SortBy};
    let mut app = App::mock(loaded);
    let by_name = order(&app);
    assert!(
        by_name
            .windows(2)
            .all(|w| w[0].to_lowercase() <= w[1].to_lowercase())
    );

    // Addon again: Z to A, the chevron flips.
    app.tap("addons/sort/addon");
    assert_eq!(
        app.state().settings.addon_order,
        AddonOrder {
            by: SortBy::Name,
            desc: true
        }
    );
    let mut reversed = by_name.clone();
    reversed.reverse();
    assert_eq!(order(&app), reversed);
    assert!(
        app.rt
            .find(&format!(
                "addon/{}",
                app.state().visible_addons()[0].addon.id
            ))
            .is_some()
    );

    // Status: what needs the user first (an error, then updates), then the rest.
    app.rt.update(|s, _| {
        s.addons
            .iter_mut()
            .find(|a| a.addon.id == "spoken")
            .unwrap()
            .op = Some(Op::Failed("503".into()))
    });
    app.tap("addons/sort/status");
    let statuses: Vec<Status> = app
        .rt
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
    assert!(app.has("addon/spoken"), "the first row is on screen");

    // The filter narrows the ordered list; the order holds.
    app.tap("my-addons/filter");
    app.type_text("b");
    let filtered: Vec<Status> = app
        .rt
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
    assert!(app.state().visible_addons().iter().all(|a| {
        let q = "b";
        a.addon.name.to_lowercase().contains(q) || a.addon.author.to_lowercase().contains(q)
    }));

    // Keyboard: Space on the focused header flips it (Return goes to buttons only on the web); the choice is saved with the settings.
    app.rt.update(|_, tx| tx.focus("addons/sort/status", true));
    app.frame();
    app.type_text(" ");
    assert_eq!(
        app.state().settings.addon_order,
        AddonOrder {
            by: SortBy::Status,
            desc: true
        }
    );
    app.wait_for("the order saved", |s| {
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
    let mut app = App::mock(loaded);
    app.type_text("/");
    app.type_text("pawn");
    assert_eq!(app.state().filter.text, "pawn");
    assert!(app.has("addon/pawn"));
}

#[test]
fn f3_2_failed_update_shows_error() {
    let mut app = App::mock(loaded);
    // Spoken is below the fold of the virtual list: select it as the row press would.
    app.rt.update(|s, _| s.selected = Some("spoken".into()));
    app.frame();
    app.tap("detail/update");
    app.wait_for("update to fail", |s| {
        matches!(s.addon("spoken").unwrap().op, Some(Op::Failed(_)))
    });
    assert_eq!(app.state().addon("spoken").unwrap().status(), Status::Error);
    assert!(app.shows("GitHub answered 503"));
}

#[test]
fn f5_3_open_website() {
    let mut app = App::mock(loaded);
    app.tap("addon/ace3");
    app.tap("detail/website");
    assert_eq!(
        app.rt.take_url_requests(),
        vec!["https://www.curseforge.com/wow/addons/ace3".to_string()]
    );
}

#[test]
fn f5_3_open_folder_shows_the_addon_folder() {
    use addon_app::mock::{MockBackend, installation};
    let backend = Arc::new(MockBackend::with_delay(Duration::from_millis(30)));
    let mut s = addon_app::State::new(backend.clone());
    loaded(&mut s);
    let mut app = App {
        rt: runtime(s, SIZE),
    };
    app.frame();
    app.tap("addon/ace3");
    app.tap("detail/folder");
    app.wait_until("the folder to open", |_| {
        !backend.opened_folders().is_empty()
    });
    let folder = &app.state().addon("ace3").unwrap().addon.folders[0];
    assert_eq!(
        backend.opened_folders(),
        [installation().addons_dir().join(folder)]
    );
}

#[test]
fn f6_1_maintenance_locked_while_game_runs() {
    let mut app = App::mock(|s| {
        loaded(s);
        s.screen = Screen::Maintenance;
        s.game_running = true;
    });
    app.frame();
    assert!(app.has("maintenance/game-running"));
    app.tap("maint/cache/run");
    assert_eq!(
        app.state().maint.get(&MaintCard::Cache),
        None,
        "disabled button does nothing"
    );
}

#[test]
fn late_results_never_land_on_the_wrong_installation_or_restart_updates() {
    use addon_app::worker::Msg;
    let mut s = mock_state();
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

#[test]
fn f1_3_installation_names_show_the_prefix_only_for_twins() {
    let mut app = App::mock(loaded);
    assert!(app.shows("WoW Forever") && !app.shows("battlenet"));
    let mut twin = addon_app::mock::installation();
    twin.id = "forever-lutris".into();
    twin.root = "~/Games/lutris/drive_c/Program Files (x86)/World of Warcraft/_forever_".into();
    app.rt.update(|s, _| s.installations.push(twin));
    app.tap("installation/selector");
    assert!(app.shows("WoW Forever (battlenet)") && app.shows("WoW Forever (lutris)"));
}

#[test]
fn f1_3_escape_closes_the_installation_menu() {
    use addon_app::state::Menu;
    let mut app = App::mock(loaded);
    app.tap("installation/selector");
    assert_eq!(app.state().menu, Some(Menu::Installation));
    app.press(rune::Key::Escape);
    assert_eq!(app.state().menu, None);

    // One popup at a time: a Get Addons dropdown replaces it.
    app.tap("installation/selector");
    app.tap("nav/get-addons");
    assert_eq!(app.state().menu, None, "changing screens closes it");
    app.tap("installation/selector");
    app.tap("get-addons/sort");
    assert_eq!(app.state().menu, Some(Menu::Sort));
}

/// Meaningful actions carry a short help text (a tooltip after a rest of the pointer), and it
/// shows when the pointer rests on the button.
#[test]
fn ui_buttons_explain_themselves() {
    let mut app = App::mock(|s| {
        loaded(s);
        s.selected = Some("questie".into());
    });
    app.frame();
    let help = |app: &App, key: &str| {
        app.rt
            .elements()
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
        let h = help(&app, key);
        assert!(h.contains(needle), "{key}: help {h:?} lacks {needle:?}");
    }
    // A glyph-only button's help names it.
    assert_eq!(help(&app, "detail/close"), "Close details");

    let (x, y) = app.rt.find("my-addons/rescan").expect("rescan button");
    app.rt.pointer_move(x, y);
    app.frame();
    app.rt.frame(1.0);
    let (tip, _) = app
        .rt
        .help_tooltip()
        .expect("a tooltip after resting on Rescan");
    assert!(tip.starts_with("Read the AddOns folder again"), "{tip}");
}

#[test]
fn f7_5_about_shows_the_version_and_release_stage() {
    let mut app = App::mock(loaded);
    app.tap("nav/settings");
    let label = app
        .rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("settings/version") && !e.exiting)
        .and_then(|e| e.text)
        .unwrap();
    assert_eq!(label, format!("v{} · Beta", env!("CARGO_PKG_VERSION")));
}

#[test]
fn f7_5_about_links_the_privacy_note() {
    let mut app = App::mock(loaded);
    app.tap("nav/settings");
    assert!(app.shows("No telemetry."));
    let tip = app
        .rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("settings/privacy"))
        .and_then(|e| e.help)
        .unwrap_or_default();
    assert!(tip.starts_with("Open the privacy note"), "{tip}");
}

#[test]
fn f7_7_copy_debug_info_leaves_out_the_key_and_the_user_name() {
    use std::path::{MAIN_SEPARATOR as SEP, Path};
    let home = rune::dirs::home().expect("a home folder");
    let user = home.file_name().unwrap().to_str().unwrap().to_string();
    let key = "cf-secret-key-123";
    let mut s = mock_state();
    loaded(&mut s);
    s.settings.curseforge_key = Some(key.into());
    s.installations[0].root = home
        .join("Games")
        .join("World of Warcraft")
        .join("_forever_");
    // A game on a second drive, mounted under the user name (KDE, GNOME).
    let mut drive = s.installations[0].clone();
    drive.id = "drive".into();
    drive.root = Path::new(&SEP.to_string())
        .join("run")
        .join("media")
        .join(&user)
        .join("Games");
    s.installations.push(drive);
    let home = home.display().to_string();
    // The error log is process-wide and keeps the last 10 entries: no other test here shows that many errors.
    s.toast(
        Tone::Danger,
        "Could not update Questie",
        format!("{home}/Games/x.zip: rejected key {key}"),
    );
    // Tall enough that About is on screen without scrolling.
    let mut app = App {
        rt: runtime(s, (1280.0, 3000.0)),
    };
    let clipboard = rune::clipboard::memory();
    app.rt.use_clipboard(&clipboard);
    app.rt.frame(0.0);
    app.tap("nav/settings");
    let tip = |app: &App, key: &str| {
        app.rt
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .and_then(|e| e.help)
            .unwrap_or_default()
    };
    assert!(tip(&app, "settings/copy-debug-info").starts_with("Copy Culm's version"));
    assert!(tip(&app, "settings/report-bug").starts_with("Open the bug report form"));
    app.tap("settings/copy-debug-info");
    let text = clipboard
        .text()
        .expect("debug info on the clipboard")
        .to_string();
    assert!(
        text.starts_with(&format!("Culm v{}", env!("CARGO_PKG_VERSION"))),
        "{text}"
    );
    for part in [
        "System: ".to_string(),
        format!(
            "WoW Forever (Forever 1.60.1.70938): ~{SEP}Games{SEP}World of Warcraft{SEP}_forever_"
        ),
        format!("{SEP}run{SEP}media{SEP}<user>{SEP}Games"),
        "error: Could not update Questie: ~/Games/x.zip: rejected key (hidden)".to_string(),
    ] {
        assert!(text.contains(&part), "no {part:?} in {text}");
    }
    assert!(!text.contains(key), "the key leaked: {text}");
    assert!(!text.contains(&home), "the home folder leaked: {text}");
    assert!(
        !text.contains(&format!("{SEP}{user}")),
        "the user name leaked: {text}"
    );
    assert!(app.shows("Debug info copied"));
}

#[test]
fn f2_1_an_empty_installation_says_so_instead_of_a_filter_miss() {
    let mut app = App::mock(|s| {
        loaded(s);
        s.addons.clear();
    });
    app.frame();
    assert!(app.shows("No addons installed yet"));
    assert!(!app.shows("No addons match"));
}

#[test]
fn ui_every_page_scrolls_with_a_thin_scroll_bar() {
    let mut app = App::mock(loaded);
    let pages = [
        (Screen::MyAddons, "my-addons/scroll"),
        (Screen::GetAddons, "get-addons/scroll"),
        (Screen::Maintenance, "maintenance/scroll"),
        (Screen::Settings, "settings/scroll"),
    ];
    for (screen, viewport) in pages {
        app.rt.update(|s, _| s.screen = screen);
        app.frame();
        let els = app.rt.elements();
        let area = format!("{viewport}-scroll-area");
        let at = els
            .iter()
            .position(|e| e.key.as_deref() == Some(area.as_str()) && !e.exiting)
            .unwrap_or_else(|| panic!("{screen:?}: no scroll bar around {viewport}"));
        assert!(
            els.iter()
                .any(|e| e.parent == Some(at) && e.key.as_deref() == Some("scroll-bar")),
            "{screen:?}: no scroll bar in {area}"
        );
    }
}

#[test]
fn ui_the_header_band_is_the_titlebar() {
    let mut app = App::mock(loaded);
    let target = |app: &App, key: &str| {
        app.rt
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap_or_else(|| panic!("no element keyed {key}"))
            .target
    };
    let press = |app: &mut App, (x, y): (f32, f32)| {
        app.rt.pointer_move(x, y);
        app.rt.pointer_down(x, y);
        let action = app.rt.take_titlebar_action();
        app.rt.pointer_up(x, y);
        app.frame();
        action
    };
    let band = target(&app, "band");
    let empty = (band[0] + band[2] * 0.6, band[1] + band[3] / 2.0);
    // Its empty parts and the sidebar's brand block move the window; a double press maximizes.
    assert_eq!(press(&mut app, empty), Some(TitlebarAction::Drag));
    assert_eq!(press(&mut app, empty), Some(TitlebarAction::DoubleClick));
    let brand = target(&app, "brand");
    assert_eq!(
        press(&mut app, (brand[0] + 4.0, brand[1] + brand[3] / 2.0)),
        Some(TitlebarAction::Drag)
    );
    // Buttons on it keep their presses.
    let update_all = app.rt.find("my-addons/update-all").expect("Update all");
    assert_eq!(press(&mut app, update_all), None);

    // Its own content keeps clear of the window's controls (macOS traffic lights at the leading
    // edge, Windows caption buttons at the trailing edge).
    let wordmark = target(&app, "brand/wordmark");
    let mut insets = ChromeInsets::default();
    (insets.leading, insets.trailing, insets.height) = (76.0, 138.0, BAND_H);
    app.rt.set_chrome_insets(insets);
    app.frame();
    let els = app.rt.elements();
    let brand = els
        .iter()
        .position(|e| e.key.as_deref() == Some("brand"))
        .unwrap();
    assert!(
        els.iter()
            .filter(|e| e.parent == Some(brand))
            .all(|e| e.target[0] >= 76.0),
        "the brand mark clears the traffic lights"
    );
    let moved = target(&app, "brand/wordmark");
    assert_eq!(moved[2], wordmark[2], "the wordmark is not cut");
    assert!(
        moved[0] + moved[2] <= SIDEBAR_W,
        "the wordmark stays in the sidebar"
    );
    let actions = target(&app, "my-addons/update-all");
    assert!(
        actions[0] + actions[2] <= SIZE.0 - 138.0,
        "Update all clears the caption buttons"
    );
}

#[test]
fn f7_6_about_links_the_third_party_licences() {
    let mut app = App::mock(loaded);
    app.tap("nav/settings");
    let tip = app
        .rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("settings/licences"))
        .and_then(|e| e.help)
        .unwrap_or_default();
    assert!(tip.starts_with("Open the licences"), "{tip}");
}
