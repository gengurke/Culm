//! End-to-end feature specs for Get Addons and Settings: the real UI and core over a temp Forever
//! tree and a fake CurseForge / GitHub (see `common`).

mod common;
use addon_app::Screen;
use addon_app::state::Fetch;
use addon_core::Channel;
use common::*;

#[test]
fn f4_1_search_and_f4_3_install_from_a_result() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    app.tap("nav/get-addons");
    app.wait_for("the popular list (F4.2)", |s| {
        matches!(s.featured, Fetch::Ready(_))
    });
    assert!(app.shows("POPULAR FOR FOREVER") && app.shows("Cee"));

    app.tap("get-addons/search");
    app.type_text("cee");
    app.wait_for(
        "search results",
        |s| matches!(&s.results, Fetch::Ready(r) if !r.is_empty()),
    );
    assert!(app.shows("RESULTS FOR “CEE”"));
    let search = &world.server.requests_to("/v1/mods/search")[0];
    assert_eq!(search.query_param("searchFilter").as_deref(), Some("cee"));
    assert_eq!(search.header("x-api-key"), Some("test-key"));

    app.tap("remote/3/install");
    app.wait_idle();
    assert!(app.has("remote/3/installed"));
    assert!(app.shows("Installed Cee"));
    assert!(
        world
            .read("Interface/AddOns/Cee/Cee.toc")
            .contains("## Version: 3.0")
    );
    app.tap("nav/my-addons");
    assert!(app.has(&app.row("Cee", "")));
    assert!(app.shows_in(&app.row("Cee", ""), "Up to date"));
}

#[test]
fn f4_4_install_from_github_url_without_a_key() {
    let world = World::new();
    let mut app = world.app();
    app.wait_idle();
    app.tap("nav/get-addons");
    app.tap("get-addons/url");
    app.type_text("https://github.com/Questie/Questie");
    app.tap("get-addons/url-install");
    app.wait_idle();
    assert!(app.shows("Installed Questie"));
    assert!(world.exists("Interface/AddOns/Questie/Questie.toc"));
    app.tap("nav/my-addons");
    let questie = &app
        .state()
        .addons
        .iter()
        .find(|a| a.addon.name == "Questie")
        .unwrap()
        .addon;
    assert_eq!(
        (
            questie.provider.as_deref(),
            questie.installed_version.as_deref()
        ),
        (Some("GitHub"), Some("v12.9.0"))
    );
}

#[test]
fn f7_1_without_a_key_search_is_off_and_points_to_settings() {
    let world = World::new();
    let mut app = world.app();
    app.wait_idle();
    assert_eq!(
        app.names(),
        ["Ace", "Bee", "Homebrew"],
        "My Addons works without a key"
    );
    app.tap("nav/get-addons");
    assert!(app.has("get-addons/no-key"));
    app.tap("get-addons/search");
    app.type_text("cee");
    assert_eq!(app.state().query.text, "", "search field is disabled");
    assert!(
        world.server.requests().is_empty(),
        "nothing asked CurseForge"
    );
    app.tap("get-addons/open-settings");
    assert_eq!(app.state().screen, Screen::Settings);
    assert!(app.shows("Not set"));
}

#[test]
fn f7_1_f7_2_saved_settings_survive_a_restart() {
    let world = World::new();
    let mut app = world.app();
    app.wait_idle();
    app.tap("nav/settings");
    app.tap("settings/api-key");
    app.type_text("my-key");
    app.tap("settings/api-key/save");
    app.tap("settings/channel/beta");
    app.wait_until("settings written", |_| {
        std::fs::read_to_string(world.prefs_path())
            .is_ok_and(|p| p.contains("my-key") && p.contains("Beta"))
    });
    app.wait_idle();
    assert_eq!(
        app.status("Ace"),
        addon_app::state::Status::UpdateAvailable,
        "saving the key checks CurseForge"
    );

    let mut again = world.app();
    again.wait_idle();
    assert_eq!(
        again.state().settings.curseforge_key.as_deref(),
        Some("my-key")
    );
    assert_eq!(again.state().settings.default_channel, Channel::Beta);
    again.tap("nav/settings");
    assert!(again.shows("Connected"));
    again.tap("nav/get-addons");
    again.wait_for("the popular list", |s| {
        matches!(s.featured, Fetch::Ready(_))
    });
    let featured = world.server.requests_to("/v1/mods/featured");
    assert_eq!(featured.last().unwrap().header("x-api-key"), Some("my-key"));
}
