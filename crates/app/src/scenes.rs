//! Named scenes for `--snap --scene <name>` and the tree dumps (`scripts/tree-diff.sh`): mock
//! states of every screen, each also as `<name>@<theme>`.

use crate::backend;
use crate::state::{AddonUi, AppUpdate, Detail, Fetch, MaintCard, MaintPhase, Menu, Tone};
use crate::{Job, Op, Screen, State, brand, mock, theme};
use addon_core::Channel;

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
pub fn all() -> &'static [Scene] {
    let mut all = SCENES.to_vec();
    for id in theme::ThemeId::ALL {
        for (name, f) in SCENES {
            all.push((String::leak(format!("{name}@{}", id.id())), *f));
        }
    }
    all.leak()
}

/// Get Addons with the category list loaded, over the scanned mock installation.
fn get_addons(s: &mut State) {
    loaded(s);
    s.screen = Screen::GetAddons;
    s.categories = Fetch::Ready(mock::categories());
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
    // Every toast tone at once.
    ("toasts", |s| {
        loaded(s);
        s.toast(Tone::Info, "3 updates available", "");
        s.toast(
            Tone::Danger,
            "Could not update WeakAuras",
            "download https://edge.forgecdn.net/files/9645/914/WeakAuras-5.19.12-forever.zip: HTTP 503 Service Unavailable",
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
    // An update its author hands out only through CurseForge's own app or website (F3.5).
    ("my-addons-app-only", |s| {
        loaded(s);
        s.selected = Some("chattynator".into());
    }),
    // Its download is open in the browser; Culm waits for the zip in Downloads (F3.5).
    ("my-addons-awaiting-download", |s| {
        loaded(s);
        set_op(
            s,
            "chattynator",
            Op::AwaitingDownload {
                until: f64::MAX,
                opened: true,
            },
        );
        s.selected = Some("chattynator".into());
    }),
    ("my-addons-scanning", |s| {
        loaded(s);
        s.addons.clear();
        s.job = Job::Scanning;
    }),
    ("get-addons", |s| {
        get_addons(s);
        s.installing
            .insert("CurseForge:deadlybossmods".into(), Op::Downloading(0.6));
        s.featured = Fetch::Ready(mock::featured(true));
    }),
    ("get-addons-search", |s| {
        get_addons(s);
        s.query.replace_text("quest");
        s.results = Fetch::Ready(mock::found(&s.search_filter(), true));
    }),
    // A result on CurseForge and WowUp Hub, the Hub picked to install from.
    ("get-addons-detail", |s| {
        get_addons(s);
        s.query.replace_text("boss");
        let found = mock::found(&s.search_filter(), true);
        let pick = found.entries.iter().find(|e| e.sources.len() > 1).cloned();
        s.results = Fetch::Ready(found);
        s.detail = pick.map(|entry| {
            let description = Fetch::Ready(Some(mock::description(&entry.sources[1])));
            Detail {
                entry,
                source: 1,
                description,
            }
        });
    }),
    // Hekili's author allows downloads only through CurseForge: Install opens the browser (F4.3),
    // and the selected result waits for the zip in Downloads.
    ("get-addons-by-hand", |s| {
        get_addons(s);
        s.query.replace_text("e");
        let found = mock::found(&s.search_filter(), true);
        let pick = found
            .entries
            .iter()
            .find(|e| e.preferred().name == "Hekili")
            .cloned();
        s.results = Fetch::Ready(found);
        s.detail = pick.map(|entry| {
            let description = Fetch::Ready(Some(mock::description(&entry.sources[0])));
            Detail {
                entry,
                source: 0,
                description,
            }
        });
    }),
    ("get-addons-awaiting-download", |s| {
        get_addons(s);
        s.query.replace_text("hek");
        let found = mock::found(&s.search_filter(), true);
        let pick = found
            .entries
            .iter()
            .find(|e| e.preferred().name == "Hekili")
            .cloned();
        s.results = Fetch::Ready(found);
        s.installing.insert(
            "CurseForge:hekili".into(),
            Op::AwaitingDownload {
                until: f64::MAX,
                opened: true,
            },
        );
        s.detail = pick.map(|entry| {
            let description = Fetch::Ready(Some(mock::description(&entry.sources[0])));
            Detail {
                entry,
                source: 0,
                description,
            }
        });
    }),
    // WowUp Hub is down: CurseForge's results and a quiet note.
    ("get-addons-partial", |s| {
        get_addons(s);
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
        get_addons(s);
        s.category = Some(1028); // Warrior
        s.sort = addon_core::Sort::LastUpdated;
        s.results = Fetch::Ready(mock::found(&s.search_filter(), true));
    }),
    ("get-addons-menu", |s| {
        get_addons(s);
        s.category = Some(1009); // Bags & Inventory
        s.results = Fetch::Ready(mock::found(&s.search_filter(), true));
        s.menu = Some(Menu::Category);
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
    ("app-update", |s| {
        loaded(s);
        s.screen = Screen::Settings;
        s.app_update = AppUpdate::Available(addon_core::self_update::Release {
            version: "0.2.0".into(),
            page: format!("https://github.com/{}/releases/tag/v0.2.0", brand::REPO),
            asset: None,
            sums: None,
        });
    }),
    ("swatches", |s| {
        loaded(s);
        s.screen = Screen::Swatches;
    }),
    ("onboarding", |s| {
        s.settings = s.backend.load_settings();
        s.job = Job::Idle;
        scene_theme(s);
    }),
];
