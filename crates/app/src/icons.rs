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

line_icon!(PACKAGE, "package", r#"<path d="M16.5 9.4 7.55 4.24"/><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><path d="M3.3 7 12 12l8.7-5"/><path d="M12 22V12"/>"#);
line_icon!(COMPASS, "compass", r#"<circle cx="12" cy="12" r="10"/><path d="M16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88z"/>"#);
line_icon!(WRENCH, "wrench", r#"<path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>"#);
line_icon!(SLIDERS, "sliders", r#"<path d="M21 4h-7M10 4H3M21 12h-9M8 12H3M21 20h-5M12 20H3M14 2v4M8 10v4M16 18v4"/>"#);
line_icon!(REFRESH, "refresh", r#"<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/>"#);
line_icon!(SEARCH, "search", r#"<circle cx="11" cy="11" r="7.5"/><path d="m20.5 20.5-4.2-4.2"/>"#);
line_icon!(DOWNLOAD, "download", r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 10 5 5 5-5"/><path d="M12 15V3"/>"#);
line_icon!(CHECK, "check", r#"<path d="M20 6 9 17l-5-5"/>"#);
line_icon!(FOLDER, "folder", r#"<path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>"#);
line_icon!(EXTERNAL, "external", r#"<path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>"#);
line_icon!(TRASH, "trash", r#"<path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>"#);
line_icon!(CLOSE, "close", r#"<path d="M18 6 6 18"/><path d="m6 6 12 12"/>"#);
line_icon!(ALERT, "alert", r#"<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/>"#);
line_icon!(PLUS, "plus", r#"<path d="M5 12h14"/><path d="M12 5v14"/>"#);
line_icon!(LINK, "link", r#"<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>"#);
line_icon!(ARCHIVE, "archive", r#"<rect width="20" height="5" x="2" y="3" rx="1"/><path d="M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8"/><path d="M10 12h4"/>"#);
line_icon!(UPDOWN, "updown", r#"<path d="m7 15 5 5 5-5"/><path d="m7 9 5-5 5 5"/>"#);
line_icon!(EYE_OFF, "eye-off", r#"<path d="M9.88 9.88a3 3 0 1 0 4.24 4.24"/><path d="M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 10 7 10 7a13.16 13.16 0 0 1-1.67 2.68"/><path d="M6.61 6.61A13.53 13.53 0 0 0 2 12s3 7 10 7a9.74 9.74 0 0 0 5.39-1.61"/><path d="M2 2l20 20"/>"#);
line_icon!(EYE, "eye", r#"<path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/>"#);
line_icon!(ROTATE, "rotate", r#"<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/>"#);
line_icon!(KEY, "key", r#"<circle cx="7.5" cy="15.5" r="5.5"/><path d="m21 2-9.6 9.6"/><path d="m15.5 7.5 3 3L22 7l-3-3"/>"#);
line_icon!(DRIVE, "drive", r#"<path d="M22 12H2"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/><path d="M6 16h.01M10 16h.01"/>"#);
line_icon!(BROOM, "broom", r#"<path d="m13 11 9-9"/><path d="M14.6 12.6c.8.8.9 2.1.2 3L10 22l-8-8 6.4-4.8c.9-.7 2.2-.6 3 .2Z"/><path d="m6.8 10.4 6.8 6.8"/><path d="m5 17 1.4-1.4"/>"#);
line_icon!(CLOCK, "clock", r#"<circle cx="12" cy="12" r="9.5"/><path d="M12 7v5l3 2"/>"#);
line_icon!(LOADER, "loader", r#"<path d="M21 12a9 9 0 1 1-6.22-8.56"/>"#);

/// The brand mark: a four-point star inside a thin ring.
pub static EMBLEM: Icon = Icon {
    name: "ra-emblem",
    view: 128.0,
    body: r#"<circle cx="64" cy="64" r="34" fill="none" stroke="currentColor" stroke-width="3.5" stroke-opacity="0.75"/><path d="M64 4 L71.5 56.5 L94 64 L71.5 71.5 L64 124 L56.5 71.5 L34 64 L56.5 56.5Z" fill="currentColor"/>"#,
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
