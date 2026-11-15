//! Grimoire: the classic in-game book windows (quest log, spellbook, character pane). Pages of
//! aged parchment with dark ink, held in a dark stone frame with gold rims, a red-brown title
//! banner with gold serif lettering, item-slot tiles, red lacquered buttons. One light from the
//! top left: raised parts catch it on their top-left edge and cast their shadow down-right,
//! fields and wells sink, pressed buttons darken. Textures are generated (texture.rs).

use super::{Medal, BASE_TYPE, BandDeco, Border, Corners, Overlay, Pal, Skin, Surface, Tex, Theme, Type, h};
use crate::icons;
use rune::tok::{Family, TextStyle};
use rune::{Color, Shadow};
use std::sync::LazyLock;

/// Only Noto Serif ships with the system; the others are used when the user has them.
const SERIF: Family = Family::Named("Cinzel, Marcellus, Cormorant SC, Noto Serif, serif");
const BOOK: Family = Family::Named("EB Garamond, Cormorant Garamond, Noto Serif, Liberation Serif, serif");

/// Light on the top-left edge, inside the box.
fn lit(a: f32) -> Shadow {
    Shadow { x: 1.0, y: 1.0, color: Color::WHITE.alpha(a), inset: true, ..Shadow::default() }
}

/// Shade on the bottom-right edge, inside the box.
fn shade(a: f32) -> Shadow {
    Shadow { x: -1.0, y: -1.0, color: Color::BLACK.alpha(a), inset: true, ..Shadow::default() }
}

/// A sunk edge: dark along the top left, inside the box.
fn sunk(d: f32, a: f32) -> Shadow {
    Shadow { x: d, y: d, blur: d, color: Color::BLACK.alpha(a), inset: true, ..Shadow::default() }
}

/// The shadow a raised part casts down and to the right.
fn cast(d: f32, blur: f32, a: f32) -> Shadow {
    Shadow { x: d * 0.6, y: d, blur, color: h("#1a0d04").alpha(a), ..Shadow::default() }
}

fn ring(color: Color) -> Shadow {
    Shadow { spread: 1.0, color, ..Shadow::default() }
}

/// The parchment pages.
pub static THEME: LazyLock<Theme> = LazyLock::new(|| {
    let ink = h("#2a1a0c");
    let rim = h("#5a3e1e");
    let p = Pal {
        sidebar: h("#1d1a17"),
        footer: h("#1d1a17"),
        bg: h("#e4d0a2"),
        panel: h("#eee0bc"),
        panel_alt: h("#dcc594"),
        sky: h("#d6bf8c"),
        well: h("#f6edd4"),
        overlay: h("#120b05").alpha(0.7),
        text: ink,
        heading: h("#6a1808"),
        text_dim: h("#46301a"),
        text_muted: h("#6a5234"),
        text_off: h("#97805c"),
        on_accent: h("#fff3d6"),
        border: rim.alpha(0.38),
        border_strong: rim.alpha(0.65),
        divider: rim.alpha(0.2),
        hover: h("#6a4210").alpha(0.08),
        selected: h("#c08a24").alpha(0.24),
        chip: rim.alpha(0.11),
        track: rim.alpha(0.18),
        accent: h("#80500c"),
        accent_hi: h("#a06a14"),
        accent_warm: h("#5a3606"),
        accent_soft: h("#a06a14").alpha(0.16),
        accent_focus: h("#9a2a10").alpha(0.85),
        success: h("#1d6a1d"),
        success_soft: h("#2a8a2a").alpha(0.15),
        warning: h("#9a4400"),
        warning_soft: h("#d06a10").alpha(0.17),
        danger: h("#a01e0e"),
        danger_soft: h("#c03020").alpha(0.12),
        danger_line: h("#a01e0e").alpha(0.5),
        info: h("#1c4c94"),
        info_soft: h("#2a5cb0").alpha(0.14),
        haze: h("#c08a24").alpha(0.12),
    };
    let ty = Type {
        display: TextStyle::new(24.0, 700.0).family(SERIF).tracking(0.3).lh(30.0),
        hero: TextStyle::new(27.0, 700.0).family(SERIF).tracking(0.5).lh(32.0),
        title: TextStyle::new(19.0, 700.0).family(SERIF).tracking(0.3).lh(24.0),
        heading: TextStyle::new(15.5, 700.0).family(SERIF).tracking(0.3).lh(20.0),
        strong: TextStyle::new(13.5, 650.0).lh(18.0),
        note: TextStyle::new(13.5, 400.0).family(BOOK).lh(19.0),
        eyebrow: TextStyle::new(10.5, 700.0).family(SERIF).tracking(1.5).lh(14.0),
        wordmark: TextStyle::new(15.0, 700.0).family(SERIF).tracking(2.4).lh(20.0),
        ..BASE_TYPE
    };
    let under = Some(Border::sides([0.0, 0.0, 1.0, 0.0], p.divider));
    // Gold-rimmed: light gold catching the light on top, dark bronze below.
    let gilt = Border::bevel(1.0, h("#e8c870"), h("#6a4a18"));
    let studs = Some(Corners { glyph: &icons::STUD, size: 8.0, inset: 4.0, color: h("#8a6428") });
    // A raised page: lighter vellum, a brown rule, lit top left, shadow down right.
    let page = Surface {
        fill: p.panel,
        vertical: Some((h("#f3e7c6"), h("#e8d7ae"))),
        border: Some(Border::bevel(1.0, h("#9a7a48"), h("#4a3218"))),
        radius: 2.0,
        shadows: vec![lit(0.7), shade(0.12), cast(3.0, 7.0, 0.35)],
        ..Default::default()
    };
    // Red lacquer with a gold rim: the classic action button.
    let lacquer = Surface {
        fill: h("#8a1c0c"),
        ink: h("#ffd84a"),
        vertical: Some((h("#b2301a"), h("#6a1206"))),
        border: Some(gilt),
        radius: 3.0,
        shadows: vec![lit(0.35), shade(0.35), cast(2.0, 3.0, 0.45)],
        hover: h("#ffb060").alpha(0.16),
        press: Color::BLACK.alpha(0.28),
        ..Default::default()
    };
    // An item slot: a dark socket with a bronze rim, sunk into the page.
    let slot = Surface {
        fill: h("#241c14"),
        ink: h("#ecc860"),
        vertical: Some((h("#1a140e"), h("#33281c"))),
        border: Some(Border::bevel(1.0, h("#4a3418"), h("#c8a050"))),
        radius: 3.0,
        shadows: vec![sunk(2.0, 0.6), ring(h("#1a0d04").alpha(0.35))],
        ..Default::default()
    };
    let skin = Skin {
        window: Surface { fill: p.bg, ink: p.text, texture: Some((Tex::Parchment, 0.45)), ..Default::default() },
        // Frame parts are drawn by FRAME (sidebar, band); these are its fallbacks.
        sidebar: Surface { fill: p.sidebar, ..Default::default() },
        band: Surface { fill: p.sky, ..Default::default() },
        band_deco: BandDeco::None,
        toolbar: Surface { border: Some(Border::sides([0.0, 0.0, 1.0, 0.0], rim.alpha(0.3))), ..Default::default() },
        panel: Surface { corners: studs, ..page.clone() },
        detail: Surface {
            fill: h("#e9d8ae"),
            texture: Some((Tex::Parchment, 0.5)),
            border: Some(Border::sides([0.0, 0.0, 0.0, 3.0], h("#3a3128"))),
            shadows: vec![Shadow { x: 1.5, color: h("#c8a050"), inset: true, ..Shadow::default() }],
            ..Default::default()
        },
        menu: Surface { shadows: vec![lit(0.7), shade(0.12), cast(4.0, 12.0, 0.45)], ..page.clone() },
        list_header: Surface { fill: rim.alpha(0.1), border: Some(Border::sides([0.0, 0.0, 1.0, 0.0], rim.alpha(0.35))), ..Default::default() },
        row: Surface { border: under, hover: p.hover, ..Default::default() },
        // The quest log's highlight: a warm wash over the whole line in a bronze rule.
        row_selected: Surface {
            fill: p.selected,
            border: Some(Border::sides([0.0, 0.0, 1.0, 0.0], h("#a06a14").alpha(0.45))),
            shadows: vec![Shadow { spread: 1.0, color: h("#a06a14").alpha(0.55), inset: true, ..Shadow::default() }],
            hover: p.hover,
            ..Default::default()
        },
        nav: Surface { hover: p.hover, ..Default::default() },
        nav_active: Surface { fill: p.selected, ..Default::default() },
        primary: lacquer.clone(),
        secondary: Surface {
            fill: h("#3a2c1e"),
            ink: h("#f4e2b8"),
            vertical: Some((h("#4c3a28"), h("#2a2016"))),
            border: Some(Border::bevel(1.0, h("#a8844a"), h("#3a2814"))),
            radius: 3.0,
            shadows: vec![lit(0.18), shade(0.3), cast(2.0, 3.0, 0.4)],
            hover: h("#ffdc9a").alpha(0.1),
            press: Color::BLACK.alpha(0.3),
            ..Default::default()
        },
        ghost: Surface { ink: p.text_dim, border: Some(Border::all(1.0, Color::TRANSPARENT)), radius: 3.0, hover: p.hover, press: rim.alpha(0.16), ..Default::default() },
        danger: Surface {
            fill: h("#f2dccc"),
            ink: h("#8a1606"),
            vertical: Some((h("#f6e4d4"), h("#e6c8b4"))),
            border: Some(Border::bevel(1.0, h("#c05a40"), h("#6a1a0c"))),
            radius: 3.0,
            shadows: vec![lit(0.6), shade(0.15), cast(2.0, 3.0, 0.3)],
            hover: h("#c03020").alpha(0.1),
            press: h("#6a1206").alpha(0.2),
            ..Default::default()
        },
        icon_button: Surface { ink: p.text_muted, radius: 3.0, hover: p.hover, ..Default::default() },
        // A field cut into the page: dark on its top-left lip, lit on the bottom right.
        input: Surface {
            fill: p.well,
            border: Some(Border::bevel(1.0, h("#5a3e1e"), h("#b89a66"))),
            radius: 2.0,
            shadows: vec![sunk(2.0, 0.28)],
            ..Default::default()
        },
        chip: Surface { radius: 3.0, ..Default::default() },
        tile: slot,
        segments: Surface { fill: rim.alpha(0.12), border: Some(Border::bevel(1.0, h("#6a4a24"), h("#c8aa70"))), radius: 3.0, shadows: vec![sunk(2.0, 0.22)], ..Default::default() },
        // The spellbook's open tab: red-brown with gold lettering, standing out of the page.
        segment_on: Surface { radius: 2.0, ..lacquer.clone() },
        toast: Surface { shadows: vec![lit(0.7), cast(4.0, 14.0, 0.5)], ..page },
        kbd: Surface { ink: p.text_muted, border: Some(Border::bevel(1.0, h("#a8885a"), h("#5a3e1e"))), radius: 2.0, ..Default::default() },
        medal: Medal { ring: [h("#c4c0b8"), h("#68625a"), h("#24201c")], ring_on: [h("#ffe38a"), h("#c8961e"), h("#5a3606")], outline: h("#2a1a0a") },
        emblem: &icons::TOME_EMBLEM,
        glyphs: GLYPHS,
    };
    Theme {
        id: "grimoire",
        name: "Grimoire",
        blurb: "Parchment pages in a gilded stone frame",
        pal: p,
        ty,
        skin,
        textures: &[Tex::Parchment, Tex::Stone, Tex::Leather],
        frame: Some(&FRAME),
    }
});

const GLYPHS: &[(&rune::Icon, &rune::Icon)] = &[
    (&icons::PACKAGE, &icons::TOME),
    (&icons::COMPASS, &icons::SCROLL),
    (&icons::WRENCH, &icons::ANVIL),
    (&icons::SLIDERS, &icons::COG),
    (&icons::DRIVE, &icons::CHEST),
    (&icons::DOWNLOAD, &icons::POUCH),
];

/// The frame round the pages: dark stone, gold rims and the red-brown title banner.
static FRAME: LazyLock<Theme> = LazyLock::new(|| {
    let page = &*THEME;
    let gold = h("#d8b050");
    let parchment = h("#f2e4c4");
    let p = Pal {
        sidebar: h("#1d1a17"),
        footer: h("#1a1714"),
        bg: h("#1d1a17"),
        panel: h("#2a241e"),
        panel_alt: h("#3a3128"),
        sky: h("#4a1a0e"),
        well: h("#0a0806").alpha(0.7),
        overlay: page.pal.overlay,
        text: parchment,
        heading: h("#ffd100"),
        text_dim: h("#dccaa6"),
        text_muted: h("#ab987c"),
        text_off: h("#6e604c"),
        on_accent: h("#1d0f04"),
        border: gold.alpha(0.3),
        border_strong: gold.alpha(0.55),
        divider: gold.alpha(0.16),
        hover: h("#ffdc9a").alpha(0.07),
        selected: h("#ffc95a").alpha(0.12),
        chip: parchment.alpha(0.08),
        track: parchment.alpha(0.12),
        accent: h("#f0c050"),
        accent_hi: h("#ffd96b"),
        accent_warm: h("#fff0c8"),
        accent_soft: h("#f0c050").alpha(0.18),
        accent_focus: h("#ffd100").alpha(0.8),
        success: h("#5bd65b"),
        success_soft: h("#5bd65b").alpha(0.15),
        warning: h("#ffa135"),
        warning_soft: h("#ffa135").alpha(0.17),
        danger: h("#ff5a48"),
        danger_soft: h("#ff5a48").alpha(0.12),
        danger_line: h("#ff5a48").alpha(0.5),
        info: h("#66b0ff"),
        info_soft: h("#66b0ff").alpha(0.15),
        haze: h("#ff9a40").alpha(0.1),
    };
    let ps = &page.skin;
    let skin = Skin {
        window: Surface { fill: p.bg, ink: p.text, ..Default::default() },
        // Dark stone with a gilded inner edge where it meets the pages.
        sidebar: Surface {
            fill: p.sidebar,
            vertical: Some((h("#25211c"), h("#171411"))),
            texture: Some((Tex::Stone, 0.4)),
            border: Some(Border::sides([0.0, 3.0, 0.0, 0.0], h("#0c0a08"))),
            shadows: vec![Shadow { x: -1.5, color: gold, inset: true, ..Shadow::default() }],
            ..Default::default()
        },
        // The title banner: red-brown leather on a gold rule.
        band: Surface {
            fill: h("#5a1c0e"),
            vertical: Some((h("#6e2412"), h("#3e1006"))),
            overlay: Some(Overlay::Radial { center: [0.18, 0.0], radius: [520.0, 140.0], stops: vec![(0.0, h("#ffb060").alpha(0.16)), (1.0, h("#ffb060").alpha(0.0))] }),
            texture: Some((Tex::Leather, 0.45)),
            border: Some(Border::sides([0.0, 0.0, 3.0, 0.0], gold)),
            shadows: vec![
                Shadow { y: -2.0, color: h("#0c0a08"), inset: true, ..Shadow::default() },
                cast(4.0, 8.0, 0.55),
            ],
            ..Default::default()
        },
        band_deco: BandDeco::None,
        nav: Surface { hover: p.hover, ..Default::default() },
        // The open tab: a lit stone plaque with a gold rim, sitting proud of the frame.
        nav_active: Surface {
            fill: h("#3e3428"),
            vertical: Some((h("#4c4032"), h("#2e261c"))),
            border: Some(Border { sides: [3.0, 10.0, 3.0, 10.0], top: Color::TRANSPARENT, bottom: Color::TRANSPARENT }),
            radius: 4.0,
            shadows: vec![Shadow { spread: 1.0, color: gold.alpha(0.75), inset: true, ..Shadow::default() }],
            hover: p.hover,
            ..Default::default()
        },
        menu: Surface {
            fill: p.panel,
            vertical: Some((h("#302920"), h("#221d18"))),
            border: Some(Border::bevel(1.0, h("#c8a050"), h("#4a3418"))),
            radius: 2.0,
            shadows: vec![lit(0.08), cast(-4.0, 12.0, 0.5)],
            ..Default::default()
        },
        primary: ps.primary.clone(),
        secondary: ps.secondary.clone(),
        chip: Surface { radius: 3.0, ..Default::default() },
        kbd: Surface { ink: p.text_muted, border: Some(Border::all(1.0, gold.alpha(0.35))), radius: 2.0, ..Default::default() },
        ..ps.clone()
    };
    Theme {
        id: "grimoire",
        name: "Grimoire",
        blurb: page.blurb,
        pal: p,
        ty: page.ty,
        skin,
        textures: &[],
        frame: None,
    }
});
