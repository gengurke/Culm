//! Canned World of Warcraft: Forever data behind [`Backend`], for the headless scenes (`--snap`,
//! `--script`), the theme tests and UI tests that need no disk or network. Names and versions come
//! from a real Forever install.

use crate::backend::{
    Backend, Backup, Found, Maintenance, Settings, Warned, curseforge_only, sort_entries,
};
use crate::ui::slug;
use addon_core::discover::group;
use addon_core::{
    Addon, Category, Channel, Flavor, IgnoreReason, Installation, RemoteAddon, RemoteFile,
    SearchFilter, Sort,
};
use anyhow::{Result, anyhow, bail};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

/// One catalog row: name, author, main folder, extra folders, installed, latest, downloads, summary.
type Row = (
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static str,
    &'static str,
    u64,
    &'static str,
);

const CATALOG: &[Row] = &[
    (
        "Questie",
        "Aero, Muehe, BreakBB",
        "Questie",
        &["QuestieDB"],
        "12.9.0",
        "12.9.2",
        48_211_903,
        "The ultimate quest helper: shows quest givers, objectives and turn-ins on the map.",
    ),
    (
        "Details! Damage Meter",
        "Terciob",
        "Details",
        &[
            "Details_DataStorage",
            "Details_EncounterDetails",
            "Details_RaidCheck",
            "Details_Streamer",
            "Details_TinyThreat",
            "Details_Compare2",
            "Details_Vanguard",
        ],
        "20261109.15410",
        "20261113.15433",
        312_884_120,
        "Damage, heal and threat meter with plugins and a deep breakdown window.",
    ),
    (
        "Auctionator",
        "plusmouse, Borjamacare",
        "Auctionator",
        &[],
        "346",
        "346",
        96_402_118,
        "Faster auction house: shopping lists, price history and one-click posting.",
    ),
    (
        "Baganator",
        "plusmouse",
        "Baganator",
        &[],
        "856",
        "861",
        8_420_551,
        "Single or category bag view with item search across characters.",
    ),
    (
        "Syndicator",
        "plusmouse",
        "Syndicator",
        &[],
        "214",
        "214",
        7_993_004,
        "Tracks items and currencies across characters for Baganator and friends.",
    ),
    (
        "Chattynator",
        "plusmouse",
        "Chattynator",
        &[],
        "142",
        "145",
        1_204_388,
        "A modern chat frame replacement with tabs, filters and copyable text.",
    ),
    (
        "Moonlit Guides",
        "Moonlit",
        "Moonlit",
        &[
            "Moonlit_Guides",
            "Moonlit_Nav",
            "Moonlit_QuestDB",
            "Moonlit_ItemDB",
            "Moonlit_Codex",
        ],
        "0.3.7",
        "0.3.7",
        412_870,
        "Free leveling guides with a step arrow, from your race's start to level 60.",
    ),
    (
        "Pawn",
        "Vger",
        "Pawn",
        &[],
        "2.14.3",
        "2.14.3",
        61_002_441,
        "Finds upgrades: scores items with stat weights and marks better gear.",
    ),
    (
        "GatherLite",
        "Wuild",
        "GatherLite",
        &[],
        "8.4.10",
        "8.5.0",
        5_300_112,
        "Lightweight gathering nodes tracker for herbs, ores and treasure.",
    ),
    (
        "Spoken",
        "Spoken Team",
        "Spoken",
        &[
            "Spoken_Quests",
            "Spoken_Places",
            "Spoken_Bosses",
            "Spoken_Readables",
        ],
        "3.9.0",
        "3.9.1",
        288_904,
        "Voiced quest text, places and bosses: listen to Azeroth's stories.",
    ),
    (
        "Enhance QoL",
        "R41z0r",
        "EnhanceQoL",
        &[
            "EnhanceQoLBags",
            "EnhanceQoLTooltip",
            "EnhanceQoLVendor",
            "EnhanceQoLMover",
            "EnhanceQoLUnitFrames",
        ],
        "6.12.0",
        "6.12.0",
        3_870_221,
        "Dozens of quality-of-life tweaks in one modular addon.",
    ),
    (
        "DialogueUI",
        "Raenore, Peterodox",
        "DialogueUI",
        &[],
        "1.0.7c",
        "1.0.7c",
        2_011_870,
        "Replaces quest and gossip frames with a cinematic dialogue view.",
    ),
    (
        "What's Training?",
        "Sveng",
        "WhatsTraining",
        &[],
        "11.4.7",
        "11.4.7",
        4_118_300,
        "Shows which spells you can learn at your trainer and what they cost.",
    ),
    (
        "DialogKey",
        "Numy",
        "DialogKey_Numy",
        &[],
        "1.12.0",
        "1.12.0",
        902_115,
        "Use number keys to pick quests and gossip options.",
    ),
    (
        "Bartender4",
        "Nevcairiel",
        "Bartender4",
        &[],
        "4.15.1",
        "4.15.2",
        101_558_903,
        "Full action bar replacement with paging, fading and per-bar layouts.",
    ),
    (
        "WeakAuras",
        "The WeakAuras Team",
        "WeakAuras",
        &["WeakAurasOptions"],
        "5.19.6",
        "5.19.6",
        230_001_557,
        "Powerful framework to display highly customizable graphics on screen.",
    ),
    (
        "Leatrix Plus",
        "Leatrix",
        "Leatrix_Plus",
        &[],
        "1.60.14",
        "1.60.16",
        38_440_210,
        "Interface and automation options in a single compact panel.",
    ),
    (
        "OmniCC",
        "tullamods",
        "OmniCC",
        &[],
        "11.2.3",
        "11.2.3",
        64_220_008,
        "Cooldown count text on every action button and item.",
    ),
    (
        "Plater Nameplates",
        "Terciob",
        "Plater",
        &[],
        "607",
        "612",
        87_105_442,
        "Nameplate addon with scripting, auras and threat colors.",
    ),
    (
        "TomTom",
        "Cladhaire",
        "TomTom",
        &[],
        "4.0.6",
        "4.0.6",
        22_870_445,
        "Waypoints and a crazy arrow to point you there.",
    ),
    (
        "AtlasLoot",
        "Hoizen",
        "AtlasLoot",
        &["AtlasLoot_Classic"],
        "8.08.01",
        "8.08.01",
        51_904_334,
        "Browse loot tables for dungeons, raids, factions and crafting.",
    ),
    (
        "Bagnon",
        "Jaliborc",
        "Bagnon",
        &[],
        "10.2.7",
        "10.2.7",
        77_420_103,
        "Merges all your bags into one window, with offline viewing.",
    ),
    (
        "Recount",
        "Elsia",
        "Recount",
        &[],
        "r1.0.8",
        "r1.0.8",
        14_220_998,
        "Graphs damage, healing and resources per character.",
    ),
    (
        "Prat 3.0",
        "Sylvanaar",
        "Prat-3.0",
        &[],
        "3.9.62",
        "3.9.64",
        9_104_337,
        "Chat customization: timestamps, colors, channel names and more.",
    ),
    (
        "Clique",
        "Cladhaire",
        "Clique",
        &[],
        "4.1.4",
        "4.1.4",
        11_340_667,
        "Click-casting on unit frames with modifiers.",
    ),
    (
        "Decursive",
        "Archarodim",
        "Decursive",
        &[],
        "2.7.32",
        "2.7.32",
        29_877_014,
        "Cleansing made easy: shows and removes debuffs in your group.",
    ),
    (
        "BugSack",
        "funkehdude",
        "BugSack",
        &["!BugGrabber"],
        "11.1.0",
        "11.1.0",
        18_090_445,
        "Collects Lua errors silently and shows them on demand.",
    ),
    (
        "Shadowed Unit Frames",
        "Shadowed",
        "ShadowedUnitFrames",
        &[],
        "4.4.13",
        "4.4.13",
        8_401_334,
        "Configurable unit frames with a light footprint.",
    ),
    (
        "HandyNotes",
        "Xinhuan",
        "HandyNotes",
        &[],
        "1.6.7",
        "1.6.8",
        32_044_910,
        "Map notes framework with plugins for rares and treasures.",
    ),
    (
        "Altoholic",
        "Teelo",
        "Altoholic",
        &["DataStore", "DataStore_Characters"],
        "4.1.008",
        "4.1.008",
        26_771_220,
        "Everything about your alts: gold, items, professions and quests.",
    ),
    (
        "Masque",
        "StormFX",
        "Masque",
        &[],
        "11.1.4",
        "11.1.4",
        19_500_330,
        "Skin engine for buttons from many addons.",
    ),
    (
        "SexyMap",
        "funkehdude",
        "SexyMap",
        &[],
        "11.0.12",
        "11.0.12",
        9_880_442,
        "Minimap skins, button handling and clock tweaks.",
    ),
    (
        "Titan Panel",
        "Titan Development Team",
        "Titan",
        &["TitanClassic"],
        "8.2.9",
        "8.3.0",
        33_102_998,
        "Info bars along the top or bottom: gold, bags, location, latency.",
    ),
    (
        "Postal",
        "Xinhuan",
        "Postal",
        &[],
        "3.5.7",
        "3.5.7",
        21_880_105,
        "Mailbox upgrades: open all, quick attach, sender history.",
    ),
    (
        "Nova World Buffs",
        "Novaspark",
        "NovaWorldBuffs",
        &[],
        "2.92",
        "2.94",
        12_330_220,
        "Tracks world buff timers and layer info.",
    ),
    (
        "Mapster",
        "Nevcairiel",
        "Mapster",
        &[],
        "1.13.2",
        "1.13.2",
        15_220_119,
        "World map tweaks: coordinates, fog clear and scaling.",
    ),
    (
        "ClassicCastbars",
        "Wardz",
        "ClassicCastbars",
        &[],
        "2.6.3",
        "2.6.3",
        14_480_210,
        "Shows enemy cast bars on target and nameplates.",
    ),
    (
        "Grid2",
        "Michael",
        "Grid2",
        &["Grid2Options"],
        "3.2.7",
        "3.2.7",
        25_301_740,
        "Compact party and raid frames with indicators.",
    ),
    (
        "MoveAnything",
        "Vladinator",
        "MoveAnything",
        &[],
        "11.b5",
        "11.b5",
        6_334_521,
        "Move, scale and hide most frames of the default UI.",
    ),
    (
        "Ace3",
        "Ace3 Team",
        "Ace3",
        &[],
        "r1355",
        "r1355",
        99_408_321,
        "Library collection many addons build on.",
    ),
    (
        "LibSharedMedia-3.0",
        "Elkano",
        "LibSharedMedia-3.0",
        &[],
        "11.0.5",
        "11.0.5",
        40_552_014,
        "Shared fonts, textures and sounds for addons.",
    ),
    (
        "Narcissus",
        "Peterodox",
        "Narcissus",
        &[],
        "1.7.4",
        "1.7.5",
        4_210_998,
        "A beautiful character pane and photo mode.",
    ),
    (
        "Immersion",
        "Meorawr",
        "Immersion",
        &[],
        "1.4.38",
        "1.4.38",
        6_120_337,
        "Interactive quest and gossip frames in a cinematic style.",
    ),
    (
        "Rarity",
        "Allara",
        "Rarity",
        &[],
        "11.0.3",
        "11.0.3",
        9_002_110,
        "Tracks rare mounts, pets and toys with attempt counts.",
    ),
    (
        "Zygor Guide Viewer",
        "Zygor",
        "ZygorGuidesViewerClassic",
        &[],
        "8.1.4",
        "8.1.4",
        3_220_014,
        "Step-by-step guide viewer with waypoints.",
    ),
    (
        "Quartz",
        "Nevcairiel",
        "Quartz",
        &[],
        "3.7.12",
        "3.7.12",
        17_665_401,
        "Modular cast bar with latency and swing timers.",
    ),
    (
        "ThreatClassic2",
        "dfherr",
        "ThreatClassic2",
        &[],
        "2.62",
        "2.63",
        7_440_210,
        "Threat meter for groups and raids.",
    ),
    (
        "BigWigs",
        "BigWigs Team",
        "BigWigs",
        &["BigWigs_Core", "BigWigs_Plugins"],
        "v389",
        "v391",
        120_440_018,
        "Boss mods: timers, warnings and sounds for encounters.",
    ),
    (
        "Auto Turn In",
        "Lethe",
        "AutoTurnIn",
        &[],
        "11.0.1",
        "11.0.1",
        5_660_009,
        "Accepts and turns in quests automatically.",
    ),
    (
        "ForeverLootSparkles",
        "Mirelle",
        "ForeverLootSparkles",
        &[],
        "0.4.2",
        "0.4.2",
        0,
        "",
    ),
    (
        "ForeverQuestTint",
        "local",
        "ForeverQuestTint",
        &[],
        "dev",
        "dev",
        0,
        "",
    ),
];

/// Extra search-only catalog entries (not installed).
const REMOTE_ONLY: &[Row] = &[
    (
        "Deadly Boss Mods",
        "DBM Team",
        "DBM-Core",
        &["DBM-StatusBarTimers"],
        "",
        "11.0.42",
        412_998_005,
        "Raid and dungeon warnings, timers and alerts for every boss.",
    ),
    (
        "ElvUI",
        "Tukui Team",
        "ElvUI",
        &["ElvUI_Options"],
        "",
        "13.84",
        188_420_118,
        "A complete user interface replacement with unit frames, bars and skins.",
    ),
    (
        "Bagnon Classic",
        "Jaliborc",
        "BagnonClassic",
        &[],
        "",
        "10.2.7",
        3_120_009,
        "Bagnon tuned for classic-era clients.",
    ),
    (
        "Leatrix Maps",
        "Leatrix",
        "Leatrix_Maps",
        &[],
        "",
        "1.60.08",
        14_551_220,
        "Map enhancements: unexplored areas, zone levels and coordinates.",
    ),
    (
        "Questie Lite",
        "Muehe",
        "QuestieLite",
        &[],
        "",
        "0.4.1",
        220_118,
        "A lighter Questie with fewer map icons.",
    ),
    (
        "Hekili",
        "Hekili",
        "Hekili",
        &[],
        "",
        "2.4.1",
        30_220_441,
        "Priority helper that suggests your next ability.",
    ),
    (
        "Healbot Continued",
        "Strife",
        "HealBot",
        &[],
        "",
        "11.0.2",
        61_004_117,
        "Click healing with clean raid frames.",
    ),
    (
        "Leatrix Sounds",
        "Leatrix",
        "Leatrix_Sounds",
        &[],
        "",
        "1.60.02",
        1_210_555,
        "Browse and play every game sound and music track.",
    ),
];

/// WowUp Hub entries: id, repo name, owner, `owner/repo`, latest tag, downloads, summary. Questie
/// Details and Deadly Boss Mods are on CurseForge too (same repo), the others only on the Hub.
const HUB: &[(u64, &str, &str, &str, &str, u64, &str)] = &[
    (
        1201,
        "Questie",
        "Questie",
        "Questie/Questie",
        "v12.9.2",
        1_204_551,
        "The ultimate quest helper, built from the GitHub releases.",
    ),
    (
        1388,
        "Details-Damage-Meter",
        "Tercioo",
        "Tercioo/Details-Damage-Meter",
        "20261113.15433",
        640_118,
        "Damage, healing and threat meter with plugins.",
    ),
    (
        1544,
        "DeadlyBossMods",
        "DeadlyBossMods",
        "DeadlyBossMods/DeadlyBossMods",
        "11.0.43",
        2_118_004,
        "Boss timers and warnings, packaged from the GitHub releases.",
    ),
    (
        2210,
        "pfQuest",
        "shagu",
        "shagu/pfQuest",
        "7.0.1",
        902_114,
        "Quest helper with a database of every NPC, object and item.",
    ),
    (
        2214,
        "ShaguTweaks",
        "shagu",
        "shagu/ShaguTweaks",
        "1.4.0",
        351_090,
        "Small quality-of-life tweaks for the default interface.",
    ),
    (
        2302,
        "QuestTint",
        "moonlit",
        "moonlit/QuestTint",
        "0.9.3",
        12_408,
        "Colors quest titles in the log by difficulty.",
    ),
];

/// The GitHub repo a catalog addon names as its source (CurseForge `links.sourceUrl`).
fn source_repo(name: &str) -> Option<&'static str> {
    match name {
        "Questie" => Some("Questie/Questie"),
        "Details! Damage Meter" => Some("Tercioo/Details-Damage-Meter"),
        "Deadly Boss Mods" => Some("DeadlyBossMods/DeadlyBossMods"),
        _ => None,
    }
}

/// A slice of CurseForge's WoW category tree (ids as on CurseForge).
const CATEGORIES: &[(u32, &str, &str, Option<u32>)] = &[
    (1018, "Action Bars", "action-bars", None),
    (1002, "Auction & Economy", "auction-economy", None),
    (1009, "Bags & Inventory", "bags-inventory", None),
    (1014, "Boss Encounters", "boss-encounters", None),
    (1001, "Chat & Communication", "chat-communication", None),
    (1020, "Class", "class", None),
    (1023, "Druid", "druid", Some(1020)),
    (1024, "Hunter", "hunter", Some(1020)),
    (1021, "Mage", "mage", Some(1020)),
    (1022, "Paladin", "paladin", Some(1020)),
    (1026, "Priest", "priest", Some(1020)),
    (1027, "Rogue", "rogue", Some(1020)),
    (1025, "Shaman", "shaman", Some(1020)),
    (1029, "Warlock", "warlock", Some(1020)),
    (1028, "Warrior", "warrior", Some(1020)),
    (1019, "Combat", "combat", None),
    (1035, "Damage Dealer", "damage-dealer", Some(1019)),
    (1032, "Healer", "healer", Some(1019)),
    (1033, "Tank", "tank", Some(1019)),
    (1010, "Libraries", "libraries", None),
    (1011, "Map & Minimap", "map-minimap", None),
    (1017, "Miscellaneous", "miscellaneous", None),
    (1015, "Professions", "professions", None),
    (1013, "Quests & Leveling", "quests-leveling", None),
    (1055, "Tooltip", "tooltip", None),
    (1016, "Unit Frames", "unit-frames", None),
    (1037, "Raid Frames", "raid-frames", Some(1016)),
];

pub fn categories() -> Vec<Category> {
    let mut list: Vec<Category> = CATEGORIES
        .iter()
        .map(|&(id, name, slug, parent)| Category {
            id,
            name: name.into(),
            slug: slug.into(),
            parent,
        })
        .collect();
    list.sort_by_key(|c| c.name.to_lowercase());
    list
}

/// Category names of a catalog addon (by name, roughly as on CurseForge).
fn categories_of(name: &str) -> Vec<String> {
    let n = name.to_lowercase();
    let pick: &[&str] = match () {
        _ if n.contains("bag") || n.contains("syndicator") => &["Bags & Inventory"],
        _ if n.contains("questie") || n.contains("guide") || n.contains("turn in") => {
            &["Quests & Leveling", "Map & Minimap"]
        }
        _ if n.contains("details") || n.contains("recount") || n.contains("threat") => {
            &["Combat", "Damage Dealer"]
        }
        _ if n.contains("boss") || n.contains("bigwigs") || n.contains("weakauras") => {
            &["Boss Encounters", "Combat"]
        }
        _ if n.contains("heal") || n.contains("clique") || n.contains("decursive") => {
            &["Healer", "Priest", "Druid", "Shaman", "Paladin"]
        }
        _ if n.contains("hekili") => &["Combat", "Warrior", "Rogue", "Hunter", "Mage", "Warlock"],
        _ if n.contains("map") || n.contains("tomtom") || n.contains("handynotes") => {
            &["Map & Minimap"]
        }
        _ if n.contains("auction") || n.contains("postal") || n.contains("pawn") => {
            &["Auction & Economy"]
        }
        _ if n.contains("chat") || n.contains("prat") || n.contains("spoken") => {
            &["Chat & Communication"]
        }
        _ if n.contains("unit")
            || n.contains("grid")
            || n.contains("plater")
            || n.contains("elvui") =>
        {
            &["Unit Frames", "Raid Frames"]
        }
        _ if n.contains("bartender") || n.contains("omnicc") || n.contains("masque") => {
            &["Action Bars"]
        }
        _ if n.contains("ace3") || n.contains("lib") => &["Libraries"],
        _ if n.contains("gather") || n.contains("training") => &["Professions", "Miscellaneous"],
        _ => &["Miscellaneous"],
    };
    pick.iter().map(|s| s.to_string()).collect()
}

pub struct MockBackend {
    /// Simulated latency of one backend call (download steps take a tenth of it each).
    delay: Duration,
    settings: Mutex<Settings>,
    installed: Mutex<Vec<Addon>>,
    backups: Mutex<Vec<Backup>>,
    /// Folders `open_folder` was asked to show, for tests (F5.3).
    opened: Mutex<Vec<PathBuf>>,
}

impl MockBackend {
    /// Feels like the real thing: calls take a few hundred milliseconds.
    pub fn new() -> MockBackend {
        MockBackend::with_delay(Duration::from_millis(450))
    }

    pub fn with_delay(delay: Duration) -> MockBackend {
        MockBackend {
            delay,
            settings: Mutex::new(Settings {
                curseforge_key: Some("mock-key".into()),
                ..Settings::default()
            }),
            installed: Mutex::new(installed_addons()),
            backups: Mutex::new(vec![
                Backup {
                    name: "WTF-2026-11-12_2114.zip".into(),
                    created_at: "2026-11-12T21:14:00Z".into(),
                    size_bytes: 18_404_311,
                },
                Backup {
                    name: "WTF-2026-11-02_0930.zip".into(),
                    created_at: "2026-11-02T09:30:00Z".into(),
                    size_bytes: 17_990_120,
                },
            ]),
            opened: Mutex::default(),
        }
    }

    /// The folders shown in the file manager so far (F5.3).
    pub fn opened_folders(&self) -> Vec<PathBuf> {
        self.opened.lock().unwrap().clone()
    }

    fn wait(&self) {
        std::thread::sleep(self.delay);
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

pub fn installation() -> Installation {
    Installation {
        id: "forever-faugus".into(),
        label: "WoW Forever".into(),
        flavor: Flavor::Forever,
        root: PathBuf::from(
            "~/Faugus/battlenet/drive_c/Program Files (x86)/World of Warcraft/_forever_",
        ),
        version: Some("1.60.1.70938".into()),
    }
}

fn file(id: &str, version: &str, channel: Channel) -> RemoteFile {
    RemoteFile {
        id: id.into(),
        version: version.into(),
        channel,
        download_url: format!("https://example.invalid/{id}.zip"),
        released_at: Some("2026-11-13T10:00:00Z".into()),
        game_versions: vec!["1.60.1".into()],
        folders: vec![],
        dependencies: vec![],
        check: None,
    }
}

fn remote(row: &Row) -> RemoteAddon {
    let (name, author, main, extra, _, latest, downloads, summary) = *row;
    let id = slug(name);
    let mut f = file(&format!("{id}-{latest}"), latest, Channel::Stable);
    f.folders = std::iter::once(main)
        .chain(extra.iter().copied())
        .map(String::from)
        .collect();
    if name == "Chattynator" || name == "Hekili" {
        // Its author allows downloads only through CurseForge's own app or website (F3.5, F4.3).
        f.download_url.clear();
        f.check = Some(addon_core::FileCheck {
            size: 1,
            sha1: "0".repeat(40),
        });
    }
    RemoteAddon {
        provider: "CurseForge".into(),
        external_id: id.clone(),
        name: name.into(),
        author: author.into(),
        summary: summary.into(),
        url: Some(format!("https://www.curseforge.com/wow/addons/{id}")),
        thumbnail_url: None,
        download_count: downloads,
        files: vec![f],
        source_url: source_repo(name).map(|r| format!("https://github.com/{r}")),
        categories: categories_of(name),
        updated_at: Some(format!("2026-11-{:02}T10:00:00Z", 1 + downloads % 14)),
    }
}

fn to_addon(r: &RemoteAddon, f: &RemoteFile) -> Addon {
    Addon {
        id: r.external_id.clone(),
        name: r.name.clone(),
        author: r.author.clone(),
        summary: r.summary.clone(),
        provider: Some(r.provider.clone()),
        external_id: Some(r.external_id.clone()),
        url: r.url.clone(),
        thumbnail_url: None,
        folders: f.folders.clone(),
        installed_version: Some(f.version.clone()),
        installed_file_id: Some(f.id.clone()),
        installed_at: Some("2026-11-15T12:00:00Z".into()),
        game_version: Some("1.60.1".into()),
        game_versions: vec!["1.60.1".into()],
        channel: f.channel,
        latest: None,
        ignored: None,
        auto_update: false,
        dependencies: vec![],
    }
}

/// The installed list as a fresh scan sees it (no `latest` yet).
pub fn installed_addons() -> Vec<Addon> {
    CATALOG
        .iter()
        .map(|row| {
            let r = remote(row);
            let installed = row.4;
            let mut a = to_addon(
                &r,
                &file(
                    &format!("{}-{installed}", r.external_id),
                    installed,
                    Channel::Stable,
                ),
            );
            a.folders = r.files[0].folders.clone();
            match row.0 {
                "ForeverLootSparkles" => {
                    (a.provider, a.external_id, a.url, a.author) =
                        (None, None, None, "Unknown".into());
                    a.ignored = Some(IgnoreReason::Unknown);
                }
                "ForeverQuestTint" => {
                    (a.provider, a.external_id, a.url) = (None, None, None);
                    a.ignored = Some(IgnoreReason::GitRepo);
                }
                "Recount" | "MoveAnything" => a.ignored = Some(IgnoreReason::User),
                "Spoken" => a.provider = Some("GitHub".into()),
                "Moonlit Guides" => {
                    a.provider = Some("GitHub".into());
                    a.channel = Channel::Beta;
                }
                _ => {}
            }
            if row.0 == "MoveAnything" {
                a.game_version = Some("1.15.4".into());
                a.game_versions = vec!["1.15.4".into()];
            }
            a
        })
        .collect()
}

/// `addons` after an update check: managed addons get the catalog's latest file.
pub fn with_latest(addons: &[Addon]) -> Vec<Addon> {
    addons
        .iter()
        .map(|a| {
            let mut a = a.clone();
            if a.external_id.is_some() {
                a.latest = CATALOG
                    .iter()
                    .find(|r| slug(r.0) == a.id)
                    .map(|row| remote(row).files[0].clone());
            }
            a
        })
        .collect()
}

pub fn catalog() -> Vec<RemoteAddon> {
    CATALOG
        .iter()
        .chain(REMOTE_ONLY)
        .filter(|r| r.6 > 0)
        .map(remote)
        .collect()
}

/// The WowUp Hub's entries.
pub fn hub_catalog() -> Vec<RemoteAddon> {
    HUB.iter()
        .map(|&(id, name, owner, repo, tag, downloads, summary)| {
            let mut f = file(&format!("hub-{id}-{tag}"), tag, Channel::Stable);
            f.folders = vec![name.replace('-', "_")];
            RemoteAddon {
                provider: "WowUpHub".into(),
                external_id: id.to_string(),
                name: name.into(),
                author: owner.into(),
                summary: summary.into(),
                url: Some(format!("https://wowup.io/addons/{id}")),
                thumbnail_url: None,
                download_count: downloads,
                files: vec![f],
                source_url: Some(format!("https://github.com/{repo}")),
                categories: vec![],
                updated_at: Some(format!("2026-11-{:02}T08:00:00Z", 1 + downloads % 14)),
            }
        })
        .collect()
}

/// What the real backend finds for `filter`: CurseForge (with a key) and the Hub, grouped per
/// addon; a category or browsing without text is CurseForge's alone.
pub fn found(filter: &SearchFilter, curseforge: bool) -> Found {
    let cf = if curseforge { search(filter) } else { vec![] };
    if curseforge_only(filter) {
        return Found {
            entries: group(vec![cf]),
            failed: vec![],
        };
    }
    let q = filter.text.trim().to_lowercase();
    let hub = hub_catalog()
        .into_iter()
        .filter(|r| r.name.to_lowercase().contains(&q) || r.summary.to_lowercase().contains(&q))
        .collect();
    let mut entries = group(vec![cf, hub]);
    sort_entries(&mut entries, filter.sort);
    Found {
        entries,
        failed: vec![],
    }
}

/// The landing list: CurseForge's most downloaded (with a key) and the Hub's, grouped.
pub fn featured(curseforge: bool) -> Found {
    let mut cf = if curseforge { catalog() } else { vec![] };
    cf.sort_by_key(|r| std::cmp::Reverse(r.download_count));
    cf.truncate(12);
    let mut hub = hub_catalog();
    hub.sort_by_key(|r| std::cmp::Reverse(r.download_count));
    Found {
        entries: group(vec![cf, hub]),
        failed: vec![],
    }
}

/// The catalog filtered and ordered like CurseForge would (a parent category includes its children).
pub fn search(filter: &SearchFilter) -> Vec<RemoteAddon> {
    let q = filter.text.trim().to_lowercase();
    let cats = categories();
    let wanted: Vec<&str> = cats
        .iter()
        .filter(|c| {
            Some(c.id) == filter.category || (c.parent.is_some() && c.parent == filter.category)
        })
        .map(|c| c.name.as_str())
        .collect();
    let mut list: Vec<RemoteAddon> = catalog()
        .into_iter()
        .filter(|r| r.name.to_lowercase().contains(&q) || r.summary.to_lowercase().contains(&q))
        .filter(|r| {
            filter.category.is_none() || r.categories.iter().any(|c| wanted.contains(&c.as_str()))
        })
        .collect();
    match filter.sort {
        Sort::Popularity | Sort::TotalDownloads => {
            list.sort_by_key(|r| std::cmp::Reverse(r.download_count))
        }
        Sort::LastUpdated => list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
        Sort::Name => list.sort_by_key(|r| r.name.to_lowercase()),
    }
    list
}

/// A canned long description, as the detail panel shows it.
pub fn description(remote: &RemoteAddon) -> String {
    format!(
        "{}\n\n{} is maintained by {} and works with World of Warcraft: Forever.\n\nFeatures\n• Works out of the box, no setup needed\n• Options in the interface menu and through a slash command\n• Light on memory and CPU, even in raids\n\nReport problems on the project page; include the version and what you were doing.",
        remote.summary, remote.name, remote.author
    )
}

impl Backend for MockBackend {
    fn load_settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    fn save_settings(&self, settings: &Settings) -> Result<()> {
        *self.settings.lock().unwrap() = settings.clone();
        Ok(())
    }

    fn installations(&self) -> Result<Vec<Installation>> {
        self.wait();
        Ok(vec![installation()])
    }

    fn add_installation(&self, path: &Path) -> Result<Installation> {
        self.wait();
        if !path.to_string_lossy().contains("World of Warcraft") {
            bail!("No World of Warcraft: Forever client in {}", path.display());
        }
        let mut inst = installation();
        inst.id = format!("forever-{}", slug(&path.to_string_lossy()));

        inst.root = path.join("_forever_");
        Ok(inst)
    }

    fn scan(&self, _inst: &Installation) -> Result<Warned<Vec<Addon>>> {
        self.wait();
        Ok((self.installed.lock().unwrap().clone(), vec![]))
    }

    fn check_updates(&self, _inst: &Installation, addons: &[Addon]) -> Result<Warned<Vec<Addon>>> {
        self.wait();
        Ok((with_latest(addons), vec![]))
    }

    fn update(&self, _inst: &Installation, addon: &Addon, progress: &dyn Fn(f32)) -> Result<Addon> {
        let latest = addon
            .latest
            .clone()
            .ok_or_else(|| anyhow!("{} has no known release", addon.name))?;
        for i in 1..=10 {
            std::thread::sleep(self.delay / 10);
            progress(i as f32 / 10.0);
        }
        if addon.name == "Spoken" {
            bail!("Download failed: GitHub answered 503");
        }
        let mut a = addon.clone();
        a.installed_version = Some(latest.version.clone());
        a.installed_file_id = Some(latest.id.clone());
        let mut installed = self.installed.lock().unwrap();
        if let Some(slot) = installed.iter_mut().find(|x| x.id == a.id) {
            *slot = a.clone();
        }
        Ok(a)
    }

    // Snapshots never watch a Downloads folder.
    fn find_download(&self, _file: &RemoteFile) -> Option<PathBuf> {
        None
    }

    fn downloads_dir(&self) -> Option<PathBuf> {
        None
    }

    fn install_downloaded(
        &self,
        inst: &Installation,
        addon: &Addon,
        _path: &Path,
        _remove: bool,
    ) -> Result<Addon> {
        let a = self.update(inst, addon, &|_| {})?;
        let mut installed = self.installed.lock().unwrap();
        if !installed.iter().any(|x| x.id == a.id) {
            installed.push(a.clone());
        }
        Ok(a)
    }

    fn install(
        &self,
        _inst: &Installation,
        remote: &RemoteAddon,
        channel: Channel,
        progress: &dyn Fn(f32),
    ) -> Result<Addon> {
        let f = remote
            .latest_for(channel)
            .ok_or_else(|| anyhow!("{} has no file for this channel", remote.name))?;
        for i in 1..=10 {
            std::thread::sleep(self.delay / 10);
            progress(i as f32 / 10.0);
        }
        let mut a = to_addon(remote, f);
        a.latest = Some(f.clone());
        a.channel = channel;
        self.installed.lock().unwrap().push(a.clone());
        Ok(a)
    }

    fn remove(&self, _inst: &Installation, addon: &Addon) -> Result<()> {
        self.wait();
        self.installed.lock().unwrap().retain(|a| a.id != addon.id);
        Ok(())
    }

    fn save_addon(&self, _inst: &Installation, addon: &Addon) -> Result<()> {
        if let Some(slot) = self
            .installed
            .lock()
            .unwrap()
            .iter_mut()
            .find(|x| x.id == addon.id)
        {
            *slot = addon.clone();
        }
        Ok(())
    }

    fn search(&self, filter: &SearchFilter) -> Result<Found> {
        self.wait();
        Ok(found(filter, self.load_settings().curseforge_key.is_some()))
    }

    fn featured(&self) -> Result<Found> {
        self.wait();
        Ok(featured(self.load_settings().curseforge_key.is_some()))
    }

    fn categories(&self) -> Result<Vec<Category>> {
        self.wait();
        Ok(categories())
    }

    fn description(&self, remote: &RemoteAddon) -> Result<Option<String>> {
        self.wait();
        Ok(Some(description(remote)))
    }

    fn resolve_url(&self, url: &str) -> Result<RemoteAddon> {
        self.wait();
        let tail = url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or_default();
        catalog()
            .into_iter()
            .chain(hub_catalog())
            .find(|r| r.external_id == slug(tail) || r.external_id == tail)
            .ok_or_else(|| anyhow!("Not a known CurseForge or GitHub addon URL"))
    }

    fn game_running(&self, _inst: &Installation) -> bool {
        false
    }

    fn maintenance(&self, _inst: &Installation, job: Maintenance) -> Result<String> {
        self.wait();
        Ok(match job {
            Maintenance::ClearCache { wdb_only: false } => "Freed 412 MB from Cache".into(),
            Maintenance::ClearCache { wdb_only: true } => "Freed 38 MB from Cache/WDB".into(),
            Maintenance::CleanBackups => "Removed 23 .bak and .old files (4.1 MB)".into(),
            Maintenance::BackupWtf => {
                let b = Backup {
                    name: "WTF-2026-11-15_1402.zip".into(),
                    created_at: "2026-11-15T14:02:00Z".into(),
                    size_bytes: 18_620_004,
                };
                let line = format!("Saved {}", b.name);
                self.backups.lock().unwrap().insert(0, b);
                line
            }
        })
    }

    fn backups(&self, _inst: &Installation) -> Result<Vec<Backup>> {
        Ok(self.backups.lock().unwrap().clone())
    }

    fn import_wowup(&self, _inst: &Installation) -> Result<usize> {
        self.wait();
        Ok(self
            .installed
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.provider.is_some())
            .count())
    }

    fn open_folder(&self, path: &Path) -> Result<()> {
        self.opened.lock().unwrap().push(path.to_path_buf());
        Ok(())
    }

    fn thumbnail(&self, _url: &str) -> Result<Vec<u8>> {
        bail!("no thumbnails in the mock")
    }

    /// None: the mock draws the own class glyphs.
    fn class_icons(&self, _each: &mut dyn FnMut(&str, Option<Vec<u8>>)) {}

    fn app_release(&self) -> Result<Option<addon_core::self_update::Release>> {
        std::thread::sleep(self.delay);
        Ok(None)
    }

    fn self_updating(&self) -> bool {
        false
    }

    fn checks_app_updates(&self) -> bool {
        false
    }

    fn install_app_release(
        &self,
        _release: &addon_core::self_update::Release,
        _progress: &dyn Fn(f32),
    ) -> Result<()> {
        bail!("the mock can't update itself")
    }

    fn launch_app(&self) -> Result<()> {
        bail!("the mock can't restart")
    }
}
