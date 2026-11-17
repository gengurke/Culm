//! End-to-end feature specs for Get Addons and Settings: the real UI and core over a temp Forever
//! tree and a fake CurseForge / WowUp Hub / GitHub (see `common`).

mod common;
use addon_app::Screen;
use addon_app::state::Fetch;
use addon_core::Channel;
use common::*;
use rune::Key;

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
        |s| matches!(&s.results, Fetch::Ready(r) if !r.entries.is_empty()),
    );
    assert!(app.shows("RESULTS FOR “CEE”"));
    // CurseForge's Cee and the Hub's Cee are one card (same repo); Dee is the Hub's alone.
    assert!(app.shows_in("remote/3/sources", "CurseForge"));
    assert!(app.shows_in("remote/3/sources", "also on WowUp Hub"));
    assert!(app.has("remote/wowuphub-8") && !app.has("remote/wowuphub-7"));
    assert_eq!(
        world.server.requests_to("/hub/addons/search/forever")[0]
            .query_param("query")
            .as_deref(),
        Some("cee")
    );
    let search = &world.server.requests_to("/v1/mods/search")[0];
    assert_eq!(search.query_param("searchFilter").as_deref(), Some("cee"));
    assert_eq!(search.header("x-api-key"), Some("test-key"));

    app.tap("remote/3/install");
    assert!(
        app.state().detail.is_none(),
        "the card's button doesn't open the detail"
    );
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
    app.tap("get-addons/search");
    app.type_text("https://github.com/Questie/Questie");
    app.wait_for("the link to resolve", |s| matches!(s.link, Fetch::Ready(_)));
    assert!(app.shows("INSTALL FROM GITHUB") && app.has("get-addons/link-source"));
    // Its detail shows the README as the description (F4.5).
    app.tap("remote/Questie/Questie");
    app.wait_for_text("The quest helper for Forever.");
    app.press(Key::Escape);
    app.tap("remote/Questie/Questie/install");
    app.wait_idle();
    assert!(app.shows("Installed Questie"));
    assert!(app.has("remote/Questie/Questie/installed"));
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
fn f4_1_one_field_routes_links_to_install_and_text_to_search() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    app.tap("nav/get-addons");
    app.tap("get-addons/search");
    app.type_text("Questie/Questie");
    app.wait_for("the link to resolve", |s| matches!(s.link, Fetch::Ready(_)));
    assert!(app.shows("INSTALL FROM GITHUB"));
    assert!(app.has("remote/Questie/Questie/install"));
    assert!(
        world.server.requests_to("/v1/mods/search").is_empty(),
        "a link is not searched"
    );
    assert_eq!(world.server.requests_to("/repos/Questie/Questie").len(), 1);

    // Enter installs the resolved link.
    app.press(Key::Enter);
    app.wait_idle();
    assert!(app.shows("Installed Questie"));

    app.rt.state.query.replace_text("");
    app.type_text("cee");
    app.wait_for(
        "search results",
        |s| matches!(&s.results, Fetch::Ready(r) if !r.entries.is_empty()),
    );
    assert_eq!(app.state().link, Fetch::Idle);
    let search = world.server.requests_to("/v1/mods/search");
    assert_eq!(search.len(), 1);
    assert_eq!(
        search[0].query_param("searchFilter").as_deref(),
        Some("cee")
    );
}

#[test]
fn f4_5_clicking_a_result_opens_its_detail_and_esc_closes_it() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    app.tap("nav/get-addons");
    app.wait_for("the popular list", |s| {
        matches!(s.featured, Fetch::Ready(_))
    });
    app.tap("remote/3");
    assert!(app.has("remote-detail"));
    app.wait_for_text("Cee keeps your bags tidy.");
    assert!(app.has("remote-detail/description"));
    assert!(app.shows("Sorts") && app.shows("Stacks"));
    assert!(app.shows("Open on CurseForge") && app.shows("3.0"));
    let desc = &world.server.requests_to("/v1/mods/3/description")[0];
    assert_eq!(desc.header("x-api-key"), Some("test-key"));

    // Install from the panel; the card and the panel both say so.
    app.tap("remote-detail/addon/install");
    app.wait_idle();
    assert!(app.has("remote/3/installed"));
    assert!(app.has("remote-detail/show"));

    app.press(Key::Escape);
    assert!(app.state().detail.is_none() && !app.has("remote-detail"));
    // Clicking the open result again closes it too.
    app.tap("remote/3");
    app.tap("remote/3");
    assert!(app.state().detail.is_none());
}

#[test]
fn f4_6_category_and_sort_filter_the_search() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    app.tap("nav/get-addons");
    app.wait_for("the categories", |s| {
        matches!(s.categories, Fetch::Ready(_))
    });
    assert_eq!(world.server.requests_to("/v1/categories").len(), 1);

    // Classes are chips beside the dropdown, which doesn't list them; Forever's classes only.
    app.tap("get-addons/category");
    assert!(app.has("category/1009") && !app.has("category/1028") && !app.has("category/1020"));
    app.press(Key::Escape);
    assert!(
        app.has("class/1028") && !app.has("class/1036"),
        "no Death Knight in Forever"
    );
    app.tap("class/1028");
    app.wait_for(
        "filtered results",
        |s| matches!(&s.results, Fetch::Ready(r) if !r.entries.is_empty()),
    );
    assert!(app.shows("WARRIOR") && app.has("remote/3"));
    // The Hub has no categories: CurseForge's results alone, without a note about it.
    assert!(!app.has("get-addons/curseforge-only"));
    assert!(!app.has("remote/wowuphub-8"));
    assert!(
        world
            .server
            .requests_to("/hub/addons/search/forever")
            .is_empty()
    );
    let req = world.server.requests_to("/v1/mods/search").pop().unwrap();
    assert_eq!(req.query_param("categoryId").as_deref(), Some("1028"));
    assert_eq!(req.query_param("searchFilter"), None);
    assert_eq!(req.query_param("sortField").as_deref(), Some("2"));

    app.tap("get-addons/sort");
    app.tap("sort/lastupdated");
    app.wait_for("re-sorted results", |s| {
        matches!(&s.results, Fetch::Ready(_))
    });
    let req = world.server.requests_to("/v1/mods/search").pop().unwrap();
    assert_eq!(req.query_param("sortField").as_deref(), Some("3"));
    assert_eq!(req.query_param("sortOrder").as_deref(), Some("desc"));
    assert_eq!(req.query_param("categoryId").as_deref(), Some("1028"));
    assert!(app.shows_in("get-addons/section", "WARRIOR"));
    assert!(app.shows_in("get-addons/section", "LAST UPDATED"));

    // A category from the dropdown replaces the class.
    app.tap("get-addons/category");
    app.tap("category/1009");
    assert!(app.state().menu.is_none());
    app.wait_for("bag results", |s| matches!(&s.results, Fetch::Ready(_)));
    let req = world.server.requests_to("/v1/mods/search").pop().unwrap();
    assert_eq!(req.query_param("categoryId").as_deref(), Some("1009"));
    assert!(app.shows("BAGS & INVENTORY"));

    // Clicking the active class chip again clears it; default order and no category: popular.
    app.tap("class/1028");
    app.tap("class/1028");
    assert_eq!(app.state().category, None);
    app.tap("get-addons/sort");
    app.tap("sort/popularity");
    assert!(app.shows("POPULAR FOR FOREVER"));
}

#[test]
fn f4_6_clear_filters_resets_class_category_and_sort() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    app.tap("nav/get-addons");
    app.wait_for("the categories", |s| {
        matches!(s.categories, Fetch::Ready(_))
    });
    // Nothing to clear yet.
    assert!(!app.has("get-addons/clear-filters"));

    app.tap("get-addons/search");
    app.type_text("cee");
    app.tap("class/1028");
    app.tap("get-addons/sort");
    app.tap("sort/name");
    assert!(app.has("get-addons/clear-filters"));
    app.wait_for("filtered results", |s| {
        matches!(&s.results, Fetch::Ready(_))
    });

    let hub = world.server.requests_to("/hub/addons/search/forever").len();
    app.tap("get-addons/clear-filters");
    assert_eq!(app.state().category, None);
    assert_eq!(app.state().sort, addon_core::Sort::default());
    // The text is what the user looks for, not a filter: it stays, and searches every source again.
    assert_eq!(app.state().query.text, "cee");
    assert!(!app.has("get-addons/clear-filters"));
    app.wait_for(
        "unfiltered results",
        |s| matches!(&s.results, Fetch::Ready(r) if !r.entries.is_empty()),
    );
    let req = world.server.requests_to("/v1/mods/search").pop().unwrap();
    assert_eq!(req.query_param("categoryId"), None);
    assert_eq!(req.query_param("searchFilter").as_deref(), Some("cee"));
    assert!(world.server.requests_to("/hub/addons/search/forever").len() > hub);
    assert!(app.shows("RESULTS FOR “CEE”"));
}

#[test]
fn f4_8_a_result_on_two_sources_installs_from_the_picked_one() {
    let world = World::new().with_key("test-key");
    let mut app = world.app();
    app.wait_idle();
    app.tap("nav/get-addons");
    app.wait_for("the popular list", |s| {
        matches!(s.featured, Fetch::Ready(_))
    });
    // The landing list merges CurseForge's and the Hub's too.
    assert!(app.shows_in("remote/3/sources", "also on WowUp Hub"));
    assert!(app.has("remote/wowuphub-8"));

    app.tap("remote/3");
    assert!(app.has("remote-detail/sources"));
    assert!(app.has("remote-detail/source/curseforge") && app.has("remote-detail/source/wowuphub"));
    // CurseForge is preferred: picked first, with its description.
    assert!(app.shows("Install from CurseForge"));
    app.wait_for_text("Cee keeps your bags tidy.");

    app.tap("remote-detail/source/wowuphub");
    assert_eq!(app.state().detail.as_ref().unwrap().source, 1);
    assert!(app.shows("Install from WowUp Hub") && app.shows("Open on WowUp Hub"));
    assert!(app.shows("v70"));
    // A Hub addon's long description is its repo's README.
    app.wait_for_text("Cee, straight from GitHub.");

    app.tap("remote-detail/addon/install");
    app.wait_idle();
    assert!(app.shows("Installed Cee"));
    assert!(app.has("remote/3/installed") && app.has("remote-detail/show"));
    assert!(
        world
            .read("Interface/AddOns/Cee/Cee.toc")
            .contains("## Version: 3.0-hub")
    );
    assert_eq!(world.server.requests_to("/hubdl/Cee-v70.zip").len(), 1);
    assert!(world.server.requests_to("/files/0/30/3.0.zip").is_empty());
    let cee = &app
        .state()
        .addons
        .iter()
        .find(|a| a.addon.name == "Cee")
        .unwrap()
        .addon;
    assert_eq!(
        (cee.provider.as_deref(), cee.external_id.as_deref()),
        (Some("WowUpHub"), Some("7"))
    );
}

#[test]
fn f4_8_a_source_that_fails_is_a_note_and_the_rest_still_works() {
    let world = World::new().with_key("test-key");
    world.server.respond(
        "GET",
        "/hub/addons/search/forever",
        503,
        "text/plain",
        "down",
    );
    let mut app = world.app();
    app.wait_idle();
    app.tap("nav/get-addons");
    app.tap("get-addons/search");
    app.type_text("cee");
    app.wait_for(
        "search results",
        |s| matches!(&s.results, Fetch::Ready(r) if !r.entries.is_empty()),
    );
    assert!(app.has("get-addons/source-error"));
    assert!(app.shows("WowUp Hub didn't answer"));
    assert!(app.has("remote/3/install") && !app.has("get-addons/error"));
    assert!(app.shows_in("remote/3/sources", "CurseForge"));
    assert!(!app.shows("also on"));
}

#[test]
fn f7_1_without_a_key_the_hub_answers_and_settings_add_curseforge() {
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
    assert!(!app.has("get-addons/category"), "no filters without a key");
    app.wait_for("the Hub's popular list", |s| {
        matches!(s.featured, Fetch::Ready(_))
    });
    assert!(app.has("remote/wowuphub-7") && app.has("remote/wowuphub-8"));
    assert!(app.shows_in("remote/wowuphub-7/sources", "WowUp Hub"));

    app.tap("get-addons/search");
    app.type_text("dee");
    app.wait_for(
        "Hub results",
        |s| matches!(&s.results, Fetch::Ready(r) if !r.entries.is_empty()),
    );
    assert!(app.has("remote/wowuphub-8") && app.has("get-addons/no-key"));
    assert!(
        world
            .server
            .requests()
            .iter()
            .all(|r| !r.path.starts_with("/v1/")),
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

/// The app with an image cache (tests don't run `setup`), so class icons can load.
fn app_with_images(world: &World) -> App {
    let mut state = addon_app::State::new(std::sync::Arc::new(
        addon_app::core_backend::CoreBackend::new(world.config()),
    ));
    state.images = Some(rune::ImageCache::new(std::sync::Arc::new(|| {})));
    let mut app = App {
        rt: addon_app::runtime(state, (1280.0, 800.0)),
    };
    app.rt.frame(0.0);
    app
}

fn icon_requests(world: &World) -> usize {
    let all = world.server.requests();
    all.iter().filter(|r| r.path.starts_with("/icons/")).count()
}

#[test]
fn f4_6_class_chips_show_the_original_icons() {
    let world = World::new().with_key("test-key");
    // The first source is gone; the second serves the icons.
    world.server.respond(
        "GET",
        "/icons/b/*",
        200,
        "image/jpeg",
        addon_testkit::CLASS_ICON,
    );
    let mut app = app_with_images(&world);
    app.wait_idle();
    assert_eq!(icon_requests(&world), 0, "not fetched before Get Addons");

    app.tap("nav/get-addons");
    app.wait_until("the warrior icon", |a| a.has("class-icon/warrior"));
    assert!(app.has("class/1028"));
    assert_eq!(
        world
            .server
            .requests_to("/icons/b/classicon_warrior.jpg")
            .len(),
        1
    );
    let cached = world.tmp.path().join("data/class-icons/warrior.jpg");
    assert!(cached.exists(), "kept in the data dir");
    let asked = icon_requests(&world);
    drop(app);

    // The next start reads them from the cache.
    let mut app = app_with_images(&world);
    app.wait_idle();
    app.tap("nav/get-addons");
    app.wait_until("the cached icon", |a| a.has("class-icon/warrior"));
    assert_eq!(icon_requests(&world), asked, "no network");
}

#[test]
fn f4_6_class_chips_keep_their_glyphs_without_icons() {
    let world = World::new().with_key("test-key");
    let mut app = app_with_images(&world);
    app.wait_idle();
    app.tap("nav/get-addons");
    app.wait_until("both sources tried", |_| icon_requests(&world) == 2);
    // Leaving and coming back doesn't try again in this run.
    app.tap("nav/my-addons");
    app.tap("nav/get-addons");
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        app.frame();
    }
    assert!(app.has("class/1028") && !app.has("class-icon/warrior"));
    assert_eq!(icon_requests(&world), 2, "one attempt per source per run");
}
