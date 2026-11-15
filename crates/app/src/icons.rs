//! Line glyphs (Lucide shapes, ISC licence) at a 24 view, ~1.5 px at 16 pt, plus the star emblem.

use rune::Icon;

macro_rules! line_icon {
    ($id:ident, $name:literal, $body:literal) => {
        pub static $id: Icon = Icon {
            name: concat!("ra-", $name),
            view: 24.0,
            body: concat!(
                r#"<g fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">"#,
                $body,
                "</g>"
            ),
        };
    };
}

line_icon!(
    PACKAGE,
    "package",
    r#"<path d="M16.5 9.4 7.55 4.24"/><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><path d="M3.3 7 12 12l8.7-5"/><path d="M12 22V12"/>"#
);
line_icon!(
    COMPASS,
    "compass",
    r#"<circle cx="12" cy="12" r="10"/><path d="M16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88z"/>"#
);
line_icon!(
    WRENCH,
    "wrench",
    r#"<path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>"#
);
line_icon!(
    SLIDERS,
    "sliders",
    r#"<path d="M21 4h-7M10 4H3M21 12h-9M8 12H3M21 20h-5M12 20H3M14 2v4M8 10v4M16 18v4"/>"#
);
line_icon!(
    REFRESH,
    "refresh",
    r#"<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/>"#
);
line_icon!(
    SEARCH,
    "search",
    r#"<circle cx="11" cy="11" r="7.5"/><path d="m20.5 20.5-4.2-4.2"/>"#
);
line_icon!(
    DOWNLOAD,
    "download",
    r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 10 5 5 5-5"/><path d="M12 15V3"/>"#
);
line_icon!(
    LAYERS,
    "layers",
    r#"<path d="m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z"/><path d="m22 17.65-9.17 4.16a2 2 0 0 1-1.66 0L2 17.65"/><path d="m22 12.65-9.17 4.16a2 2 0 0 1-1.66 0L2 12.65"/>"#
);
line_icon!(CHECK, "check", r#"<path d="M20 6 9 17l-5-5"/>"#);
line_icon!(
    FOLDER,
    "folder",
    r#"<path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>"#
);
line_icon!(
    EXTERNAL,
    "external",
    r#"<path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>"#
);
line_icon!(
    TRASH,
    "trash",
    r#"<path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>"#
);
line_icon!(
    CLOSE,
    "close",
    r#"<path d="M18 6 6 18"/><path d="m6 6 12 12"/>"#
);
line_icon!(
    ALERT,
    "alert",
    r#"<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/>"#
);
line_icon!(
    ARROW_UP,
    "arrow-up",
    r#"<path d="m5 12 7-7 7 7"/><path d="M12 19V5"/>"#
);
line_icon!(
    HELP,
    "help",
    r#"<circle cx="12" cy="12" r="9.5"/><path d="M9.1 9a3 3 0 0 1 5.8 1c0 2-3 3-3 3"/><path d="M12 17h.01"/>"#
);
line_icon!(PLUS, "plus", r#"<path d="M5 12h14"/><path d="M12 5v14"/>"#);
line_icon!(
    LINK,
    "link",
    r#"<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>"#
);
line_icon!(
    ARCHIVE,
    "archive",
    r#"<rect width="20" height="5" x="2" y="3" rx="1"/><path d="M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8"/><path d="M10 12h4"/>"#
);
line_icon!(
    UPDOWN,
    "updown",
    r#"<path d="m7 15 5 5 5-5"/><path d="m7 9 5-5 5 5"/>"#
);
line_icon!(
    EYE_OFF,
    "eye-off",
    r#"<path d="M9.88 9.88a3 3 0 1 0 4.24 4.24"/><path d="M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 10 7 10 7a13.16 13.16 0 0 1-1.67 2.68"/><path d="M6.61 6.61A13.53 13.53 0 0 0 2 12s3 7 10 7a9.74 9.74 0 0 0 5.39-1.61"/><path d="M2 2l20 20"/>"#
);
line_icon!(
    EYE,
    "eye",
    r#"<path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/>"#
);
line_icon!(
    ROTATE,
    "rotate",
    r#"<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/>"#
);
line_icon!(
    KEY,
    "key",
    r#"<circle cx="7.5" cy="15.5" r="5.5"/><path d="m21 2-9.6 9.6"/><path d="m15.5 7.5 3 3L22 7l-3-3"/>"#
);
line_icon!(
    DRIVE,
    "drive",
    r#"<path d="M22 12H2"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/><path d="M6 16h.01M10 16h.01"/>"#
);
line_icon!(
    BROOM,
    "broom",
    r#"<path d="m13 11 9-9"/><path d="M14.6 12.6c.8.8.9 2.1.2 3L10 22l-8-8 6.4-4.8c.9-.7 2.2-.6 3 .2Z"/><path d="m6.8 10.4 6.8 6.8"/><path d="m5 17 1.4-1.4"/>"#
);
line_icon!(
    CLOCK,
    "clock",
    r#"<circle cx="12" cy="12" r="9.5"/><path d="M12 7v5l3 2"/>"#
);
line_icon!(
    LOADER,
    "loader",
    r#"<path d="M21 12a9 9 0 1 1-6.22-8.56"/>"#
);

/// The brand mark: a bamboo culm with two leaves under a crescent moon and a four-point star.
pub static EMBLEM: Icon = Icon {
    name: "ra-emblem",
    view: 128.0,
    body: r#"<path d="M109.6 39.6 A21 21 0 1 1 87.5 11.1 A18 18 0 1 0 109.6 39.6Z" fill="currentColor"/><g transform="translate(54 16) rotate(-6)" fill="currentColor"><rect x="-7.5" y="0" width="15" height="28" rx="3.5"/><rect x="-7.5" y="32" width="15" height="32" rx="3.5"/><rect x="-7.5" y="68" width="15" height="36" rx="3.5"/><rect x="-10" y="28.3" width="20" height="3.4" rx="1.7"/><rect x="-10" y="64.3" width="20" height="3.4" rx="1.7"/><path d="M8 66 C20 70 32 78 40 90 C28 88 16 80 8 70Z"/><path d="M-8 30 C-20 22 -32 20 -42 24 C-32 32 -20 34 -8 34Z"/></g><path d="M104 61.0 L106.275 71.725 L117.0 74 L106.275 76.275 L104 87.0 L101.725 76.275 L91.0 74 L101.725 71.725Z" fill="currentColor"/>"#,
};

/// A small filled four-point star (empty states, sparkles).
pub static STAR: Icon = Icon {
    name: "ra-star",
    view: 128.0,
    body: r#"<path d="M64 4 L71.5 56.5 L124 64 L71.5 71.5 L64 124 L56.5 71.5 L4 64 L56.5 56.5Z" fill="currentColor"/>"#,
};

/// Hearthwood's crest: a shield bearing a four-point star.
pub static CREST: Icon = Icon {
    name: "ra-crest",
    view: 128.0,
    body: r#"<path d="M64 8 L114 24 C114 70 98 102 64 120 C30 102 14 70 14 24 Z" fill="currentColor" fill-opacity="0.16" stroke="currentColor" stroke-width="7" stroke-linejoin="round"/><path d="M64 30 L73 58 L100 64 L73 70 L64 98 L55 70 L28 64 L55 58Z" fill="currentColor"/>"#,
};

/// A bevelled gold stud for frame corners.
pub static STUD: Icon = Icon {
    name: "ra-stud",
    view: 16.0,
    body: r##"<path d="M8 1 L15 8 L8 15 L1 8Z" fill="currentColor"/><path d="M8 1 L15 8 L8 15 L1 8Z" fill="none" stroke="#1a0d04" stroke-opacity="0.85" stroke-width="1.2" stroke-linejoin="round"/><path d="M8 2.6 L8 8 L2.6 8Z" fill="#fff8e0" fill-opacity="0.5"/><path d="M8 13.4 L8 8 L13.4 8Z" fill="#000" fill-opacity="0.35"/>"##,
};

line_icon!(CHEVRON_DOWN, "chevron-down", r#"<path d="m6 9 6 6 6-6"/>"#);
line_icon!(CHEVRON_UP, "chevron-up", r#"<path d="m6 15 6-6 6 6"/>"#);
line_icon!(
    FILTER,
    "filter",
    r#"<path d="M3 6h18"/><path d="M7 12h10"/><path d="M10 18h4"/>"#
);

// Class emblems for CurseForge's class categories: our own filled silhouettes (no game art), drawn
// to read at 12-14 pt inside a class medallion (`ui::class_medal`).
macro_rules! emblem {
    ($id:ident, $name:literal, $body:literal) => {
        pub static $id: Icon = Icon {
            name: concat!("ra-", $name),
            view: 24.0,
            body: concat!(
                r#"<g fill="currentColor" stroke-linecap="round" stroke-linejoin="round">"#,
                $body,
                "</g>"
            ),
        };
    };
}

// A sword, point up, centred on x = 12: blade, guard, grip, pommel.
emblem!(
    CLASS_WARRIOR,
    "class-warrior",
    r#"<g transform="rotate(-40 12 12.2)"><path d="M12 .4 13.9 3.2V14.6H10.1V3.2Z"/><path d="M6.6 14.6h10.8v2.3H6.6z"/><path d="M10.9 16.9h2.2v3.8h-2.2z"/><circle cx="12" cy="22" r="1.8"/></g><g transform="rotate(40 12 12.2)"><path d="M12 .4 13.9 3.2V14.6H10.1V3.2Z"/><path d="M6.6 14.6h10.8v2.3H6.6z"/><path d="M10.9 16.9h2.2v3.8h-2.2z"/><circle cx="12" cy="22" r="1.8"/></g>"#
);
emblem!(
    CLASS_HUNTER,
    "class-hunter",
    r#"<path d="M5.2 3.2a15.8 15.8 0 0 1 15.6 15.6" fill="none" stroke="currentColor" stroke-width="2.6"/><path d="M5.2 3.2 20.8 18.8" fill="none" stroke="currentColor" stroke-width="0.9"/><path d="M3.6 20.4 14.6 9.4" fill="none" stroke="currentColor" stroke-width="1.9"/><path d="M19.4 4.6 12.4 7 17 11.6Z"/><path d="M6.4 17.6 3.4 16.4 2.6 18.6 4.4 19.6 5.4 21.4 7.6 20.6Z"/>"#
);
emblem!(
    CLASS_ROGUE,
    "class-rogue",
    r#"<g transform="rotate(42 12 12)"><path d="M12 .2c2 2.4 3.1 5 3.1 8.4L14 14h-4L8.9 8.6c0-3.4 1.1-6 3.1-8.4z"/><path d="M6.2 14h11.6a1.1 1.1 0 0 1 0 2.2H6.2a1.1 1.1 0 0 1 0-2.2z"/><path d="M10.9 16.2h2.2v4.2h-2.2z"/><circle cx="12" cy="21.8" r="1.9"/></g>"#
);
emblem!(
    CLASS_PRIEST,
    "class-priest",
    r#"<path d="M10.8 1.8h2.4v18.4h-2.4z"/><path d="M4.6 7.8h14.8v2.4H4.6z"/><circle cx="12" cy="9" r="4.8" fill="none" stroke="currentColor" stroke-width="2.1"/><path d="M8.6 20.2h6.8l.8 2.2H7.8z"/>"#
);
emblem!(
    CLASS_MAGE,
    "class-mage",
    r#"<path transform="translate(12 12.6) scale(1.2) translate(-12 -13.8)" fill-rule="evenodd" d="M12 22.2c-4 0-6.6-2.7-6.6-6.4 0-4.1 2.8-6.1 4.1-10.4 2 1.7 3.3 3.9 3.5 6.1 1-.9 1.5-2.1 1.6-3.3 2.2 2.2 4 4.7 4 7.6 0 3.7-2.6 6.4-6.6 6.4zM12 20.2c-1.4 0-2.4-1-2.4-2.4 0-1.5 1.3-2.4 2.4-3.9 1.1 1.5 2.4 2.4 2.4 3.9s-1 2.4-2.4 2.4z"/>"#
);
emblem!(
    CLASS_WARLOCK,
    "class-warlock",
    r#"<path fill-rule="evenodd" d="M12 2.2c-4.6 0-7.8 3.1-7.8 7.5 0 2.6 1.2 4.5 3.1 5.6v4.4c0 .8.6 1.4 1.3 1.4h6.8c.7 0 1.3-.6 1.3-1.4v-4.4c1.9-1.1 3.1-3 3.1-5.6 0-4.4-3.2-7.5-7.8-7.5zM6.8 10.6a2.2 2.2 0 1 0 4.4 0a2.2 2.2 0 1 0-4.4 0zM12.8 10.6a2.2 2.2 0 1 0 4.4 0a2.2 2.2 0 1 0-4.4 0zM12 13.6l1.1 2.2h-2.2zM10.1 18h1v3.1h-1zM12.9 18h1v3.1h-1z"/>"#
);
emblem!(
    CLASS_DRUID,
    "class-druid",
    r#"<path d="M12 11.6c-3.2 0-5.9 3.2-5.9 6 0 1.9 1.4 2.8 3 2.8 1.2 0 1.9-.6 2.9-.6s1.7.6 2.9.6c1.6 0 3-.9 3-2.8 0-2.8-2.7-6-5.9-6z"/><ellipse cx="5.4" cy="10" rx="1.9" ry="2.4" transform="rotate(-20 5.4 10)"/><ellipse cx="9.3" cy="5.4" rx="1.9" ry="2.5" transform="rotate(-8 9.3 5.4)"/><ellipse cx="14.7" cy="5.4" rx="1.9" ry="2.5" transform="rotate(8 14.7 5.4)"/><ellipse cx="18.6" cy="10" rx="1.9" ry="2.4" transform="rotate(20 18.6 10)"/>"#
);
emblem!(
    CLASS_PALADIN,
    "class-paladin",
    r#"<path fill-rule="evenodd" d="M4.6 3h14.8c.8 0 1.4.6 1.4 1.4v5c0 .8-.6 1.4-1.4 1.4H4.6c-.8 0-1.4-.6-1.4-1.4v-5C3.2 3.6 3.8 3 4.6 3zM12 4.6l1.7 2.3L12 9.2l-1.7-2.3z"/><path d="M10.8 10.8h2.4v9h-2.4z"/><path d="M9.2 19.6h5.6l-.9 2.8h-3.8z"/>"#
);
emblem!(
    CLASS_SHAMAN,
    "class-shaman",
    r#"<path d="M14.2 1.6 5 13.4h6.2L9.4 22.4 19 10.2h-6.2l3.3-8.6z"/>"#
);

/// The class colour for a CurseForge class category slug, as the game's UI uses it (plain data).
pub fn class_color(slug: &str) -> Option<&'static str> {
    Some(match slug {
        "warrior" => "#C69B6D",
        "paladin" => "#F48CBA",
        "hunter" => "#AAD372",
        "rogue" => "#FFF468",
        "priest" => "#FFFFFF",
        "shaman" => "#0070DD",
        "mage" => "#3FC7EB",
        "warlock" => "#8788EE",
        "druid" => "#FF7C0A",
        _ => return None,
    })
}

/// The glyph for a CurseForge class category slug ("warrior"), for the classes Forever has.
pub fn class_icon(slug: &str) -> Option<&'static Icon> {
    Some(match slug {
        "warrior" => &CLASS_WARRIOR,
        "hunter" => &CLASS_HUNTER,
        "rogue" => &CLASS_ROGUE,
        "priest" => &CLASS_PRIEST,
        "mage" => &CLASS_MAGE,
        "warlock" => &CLASS_WARLOCK,
        "druid" => &CLASS_DRUID,
        "paladin" => &CLASS_PALADIN,
        "shaman" => &CLASS_SHAMAN,
        _ => return None,
    })
}

// Grimoire's own glyphs: a tome, a scroll, an anvil, a cog and a chest (line drawings in the
// same style; scroll, anvil and cog after Lucide's, ISC licence).
line_icon!(
    TOME,
    "tome",
    r#"<path d="M5 19.5v-15A1.5 1.5 0 0 1 6.5 3H19v15H6.5A1.5 1.5 0 0 0 5 19.5a1.5 1.5 0 0 0 1.5 1.5H19v-3"/><path d="M12 6.5l1.2 2.3 2.3 1.2-2.3 1.2-1.2 2.3-1.2-2.3-2.3-1.2 2.3-1.2z"/>"#
);
line_icon!(
    SCROLL,
    "scroll",
    r#"<path d="M15 12h-5"/><path d="M15 8h-5"/><path d="M19 17V5a2 2 0 0 0-2-2H4"/><path d="M8 21h12a2 2 0 0 0 2-2v-1a1 1 0 0 0-1-1H11a1 1 0 0 0-1 1v1a2 2 0 1 1-4 0V5a2 2 0 1 0-4 0v2a1 1 0 0 0 1 1h3"/>"#
);
line_icon!(
    ANVIL,
    "anvil",
    r#"<path d="M7 10H6a4 4 0 0 1-4-4 1 1 0 0 1 1-1h4"/><path d="M7 5a1 1 0 0 1 1-1h13a1 1 0 0 1 1 1 7 7 0 0 1-7 7H8a1 1 0 0 1-1-1z"/><path d="M9 12v5"/><path d="M15 12v5"/><path d="M5 20a3 3 0 0 1 3-3h8a3 3 0 0 1 3 3 1 1 0 0 1-1 1H6a1 1 0 0 1-1-1"/>"#
);
line_icon!(
    COG,
    "cog",
    r#"<path d="M12 20a8 8 0 1 0 0-16 8 8 0 0 0 0 16Z"/><path d="M12 14a2 2 0 1 0 0-4 2 2 0 0 0 0 4Z"/><path d="M12 2v2M12 22v-2M17 20.66l-1-1.73M11 10.27 7 3.34M20.66 17l-1.73-1M3.34 7l1.73 1M14 12h8M2 12h2M20.66 7l-1.73 1M3.34 17l1.73-1M17 3.34l-1 1.73M11 13.73l-4 6.93"/>"#
);
line_icon!(
    CHEST,
    "chest",
    r#"<path d="M3 10a6 6 0 0 1 6-6h6a6 6 0 0 1 6 6v10H3z"/><path d="M3 11h7M14 11h7"/><rect x="10" y="9" width="4" height="5" rx="1"/><path d="M7 4.6V20M17 4.6V20"/>"#
);
line_icon!(
    POUCH,
    "pouch",
    r#"<path d="M9 3h6l-1.5 3h-3z"/><path d="M8.5 6h7"/><path d="M8.5 6C5.5 9 3.5 12 3.5 15a6 6 0 0 0 6 6h5a6 6 0 0 0 6-6c0-3-2-6-5-9"/><path d="M12 11v6M9.5 14.5 12 17l2.5-2.5"/>"#
);

/// Grimoire's emblem: an open tome under a four-point star.
pub static TOME_EMBLEM: Icon = Icon {
    name: "ra-tome-emblem",
    view: 128.0,
    body: r#"<path d="M64 50C50 40 30 38 12 42V110C30 106 50 108 64 118C78 108 98 106 116 110V42C98 38 78 40 64 50Z" fill="currentColor" fill-opacity="0.16" stroke="currentColor" stroke-width="7" stroke-linejoin="round"/><path d="M64 52V114" fill="none" stroke="currentColor" stroke-width="6"/><path d="M64 2L69.5 21.5L89 27L69.5 32.5L64 52L58.5 32.5L39 27L58.5 21.5Z" fill="currentColor"/>"#,
};
