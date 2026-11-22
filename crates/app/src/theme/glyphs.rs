//! Glyphs only themes use: emblems, corner ornaments and Grimoire's swaps for stock glyphs
//! ([`super::Skin::glyphs`]). Stock glyphs stay in `icons.rs`.

use crate::icons::line_icon;
use rune::Icon;

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
