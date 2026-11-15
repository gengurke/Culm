//! Hearthwood: the feel of a classic fantasy MMO's in-game UI. Carved wood and stone frames,
//! dark leather panels with gold studs, gold serif headings in quest yellow, red lacquered
//! buttons with a bevel, item-quality status colours. Textures are generated (texture.rs).

use super::{Medal, BASE_TYPE, BandDeco, Border, Corners, Overlay, Pal, Skin, Surface, Tex, Theme, Type, h, inset};
use crate::icons;
use rune::tok::{Family, TextStyle};
use rune::{Color, Shadow};
use std::sync::LazyLock;

/// Only Noto Serif ships with the system; the others are used when the user has them.
const SERIF: Family = Family::Named("Cinzel, Marcellus, Cormorant SC, Noto Serif, serif");

fn drop(y: f32, blur: f32, a: f32) -> Shadow {
    Shadow { y, blur, color: Color::BLACK.alpha(a), ..Shadow::default() }
}

/// A 1 pt line just inside the box's edge.
fn ring(color: Color) -> Shadow {
    Shadow { spread: 1.0, color, inset: true, ..Shadow::default() }
}

pub static THEME: LazyLock<Theme> = LazyLock::new(|| {
    let gold = h("#c9a04a");
    let parchment = h("#f4e7cc");
    let p = Pal {
        sidebar: h("#24170d"),
        footer: h("#1e140c"),
        bg: h("#1b140e"),
        panel: h("#2a1d13"),
        panel_alt: h("#3b2a1a"),
        sky: h("#3a2c1f"),
        well: h("#0b0704").alpha(0.75),
        overlay: h("#0b0704").alpha(0.75),
        text: parchment,
        heading: h("#ffd100"),
        text_dim: h("#d9c6a0"),
        text_muted: h("#ad9876"),
        text_off: h("#6f5f48"),
        on_accent: h("#1d0f04"),
        border: gold.alpha(0.30),
        border_strong: gold.alpha(0.55),
        divider: gold.alpha(0.14),
        hover: h("#ffdc9a").alpha(0.07),
        selected: h("#ffc95a").alpha(0.11),
        chip: parchment.alpha(0.08),
        track: parchment.alpha(0.12),
        accent: h("#e8b84a"),
        accent_hi: h("#ffd96b"),
        accent_warm: h("#fff0c8"),
        accent_soft: h("#e8b84a").alpha(0.18),
        accent_focus: h("#ffd100").alpha(0.80),
        success: h("#5bd65b"),
        success_soft: h("#5bd65b").alpha(0.15),
        warning: h("#ffa135"),
        warning_soft: h("#ffa135").alpha(0.16),
        danger: h("#ff5a48"),
        danger_soft: h("#ff5a48").alpha(0.10),
        danger_line: h("#ff5a48").alpha(0.50),
        info: h("#55aaff"),
        info_soft: h("#55aaff").alpha(0.15),
        haze: h("#ff9a40").alpha(0.10),
    };
    let ty = Type {
        display: TextStyle::new(24.0, 700.0).family(SERIF).tracking(0.2).lh(30.0),
        hero: TextStyle::new(27.0, 700.0).family(SERIF).tracking(0.4).lh(32.0),
        title: TextStyle::new(19.0, 700.0).family(SERIF).tracking(0.2).lh(24.0),
        heading: TextStyle::new(15.5, 700.0).family(SERIF).tracking(0.3).lh(20.0),
        eyebrow: TextStyle::new(10.5, 700.0).family(SERIF).tracking(1.5).lh(14.0),
        wordmark: TextStyle::new(15.0, 700.0).family(SERIF).tracking(2.4).lh(20.0),
        ..BASE_TYPE
    };
    let under = Some(Border::sides([0.0, 0.0, 1.0, 0.0], p.divider));
    let bevel = Some(Border::bevel(1.0, h("#b08a44"), h("#4a3418")));
    let studs = Some(Corners { glyph: &icons::STUD, size: 9.0, inset: 3.0, color: h("#d9b25a") });
    // Cards stay untextured (many on screen): a leather-brown gradient under a warm top glow.
    let leather = Surface {
        fill: p.panel,
        vertical: Some((h("#35261a"), h("#21160d"))),
        overlay: Some(Overlay::Radial { center: [0.3, 0.0], radius: [380.0, 150.0], stops: vec![(0.0, h("#ffcf80").alpha(0.07)), (1.0, h("#ffcf80").alpha(0.0))] }),
        border: Some(Border::bevel(1.0, h("#a8833e"), h("#4a3418"))),
        radius: 2.0,
        shadows: vec![drop(2.0, 8.0, 0.55)],
        ..Default::default()
    };
    let wood = Surface { fill: h("#2a1a0e"), vertical: Some((h("#311f11"), h("#1f130a"))), texture: Some((Tex::Wood, 0.5)), ..Default::default() };
    let skin = Skin {
        window: Surface { fill: p.bg, ink: p.text, texture: Some((Tex::Leather, 0.25)), ..Default::default() },
        sidebar: Surface { border: Some(Border::sides([0.0, 2.0, 0.0, 0.0], h("#8a6a2e"))), ..wood.clone() },
        band: Surface {
            fill: h("#3a3129"),
            vertical: Some((h("#4d4238"), h("#26201a"))),
            texture: Some((Tex::Stone, 0.55)),
            border: Some(Border::sides([0.0, 0.0, 2.0, 0.0], gold)),
            shadows: vec![drop(3.0, 10.0, 0.6)],
            ..Default::default()
        },
        band_deco: BandDeco::Embers,
        toolbar: Surface { fill: Color::BLACK.alpha(0.2), border: Some(Border::sides([0.0, 0.0, 1.0, 0.0], gold.alpha(0.2))), ..Default::default() },
        panel: Surface { corners: studs, ..leather.clone() },
        detail: Surface { border: Some(Border::sides([0.0, 0.0, 0.0, 2.0], h("#8a6a2e"))), ..wood },
        menu: Surface { overlay: None, shadows: vec![inset(1.0, h("#ffcf80").alpha(0.1)), drop(6.0, 18.0, 0.65)], ..leather.clone() },
        list_header: Surface { fill: Color::BLACK.alpha(0.22), border: Some(Border::sides([0.0, 0.0, 1.0, 0.0], gold.alpha(0.25))), ..Default::default() },
        row: Surface { border: under, hover: p.hover, ..Default::default() },
        // A warm-lit row in a thin gold ring.
        row_selected: Surface {
            fill: p.selected,
            vertical: Some((gold.alpha(0.16), gold.alpha(0.07))),
            border: under,
            shadows: vec![ring(gold.alpha(0.45))],
            hover: p.hover,
            ..Default::default()
        },
        nav: Surface { hover: p.hover, ..Default::default() },
        // A gilt plaque on the wood, inset from the sidebar's edges: the clear border is its
        // margin, and a wide inset shadow paints its face (a fill would run under the border).
        nav_active: Surface {
            border: Some(Border { sides: [3.0, 10.0, 3.0, 10.0], top: Color::TRANSPARENT, bottom: Color::TRANSPARENT }),
            radius: 2.0,
            shadows: vec![
                Shadow { spread: 40.0, color: h("#3a2814").alpha(0.92), inset: true, ..Shadow::default() },
                inset(2.0, h("#ffcf80").alpha(0.14)),
                inset(-2.0, Color::BLACK.alpha(0.35)),
                ring(gold.alpha(0.6)),
            ],
            hover: p.hover,
            ..Default::default()
        },
        primary: Surface {
            fill: h("#8b1f10"),
            ink: p.heading,
            vertical: Some((h("#b0331b"), h("#66150a"))),
            border: Some(Border::bevel(1.0, h("#f0c860"), h("#6a4a14"))),
            radius: 3.0,
            shadows: vec![inset(1.0, h("#ffb080").alpha(0.35)), drop(1.0, 3.0, 0.7)],
            hover: h("#ffb060").alpha(0.16),
            ..Default::default()
        },
        secondary: Surface {
            fill: p.panel_alt,
            ink: h("#f0dcae"),
            vertical: Some((h("#4c3622"), h("#2c1f13"))),
            border: bevel,
            radius: 3.0,
            shadows: vec![drop(1.0, 2.0, 0.6)],
            hover: h("#ffdc9a").alpha(0.08),
            ..Default::default()
        },
        ghost: Surface { ink: p.text_dim, border: Some(Border::all(1.0, Color::TRANSPARENT)), radius: 3.0, hover: p.hover, ..Default::default() },
        danger: Surface {
            fill: h("#3a100b"),
            ink: h("#ff7a66"),
            vertical: Some((h("#4a140d"), h("#2c0b07"))),
            border: Some(Border::bevel(1.0, h("#d0503a"), h("#5a1a10"))),
            radius: 3.0,
            hover: h("#ff5a40").alpha(0.14),
            ..Default::default()
        },
        icon_button: Surface { ink: p.text_muted, radius: 3.0, hover: p.hover, ..Default::default() },
        input: Surface {
            fill: p.well,
            border: Some(Border::bevel(1.0, h("#120a05"), h("#8a6a35"))),
            radius: 2.0,
            shadows: vec![inset(2.0, Color::BLACK.alpha(0.5))],
            ..Default::default()
        },
        chip: Surface { radius: 3.0, ..Default::default() },
        tile: Surface { fill: p.panel_alt, ink: h("#e8d3a8"), vertical: Some((h("#4a3422"), h("#2a1d12"))), border: bevel, radius: 2.0, ..Default::default() },
        segments: Surface { fill: Color::BLACK.alpha(0.25), border: Some(Border::bevel(1.0, h("#1a1009"), h("#6a5028"))), radius: 3.0, ..Default::default() },
        segment_on: Surface {
            fill: h("#5a3e22"),
            ink: p.heading,
            vertical: Some((h("#6a4a28"), h("#4a3219"))),
            border: Some(Border::all(1.0, gold.alpha(0.75))),
            radius: 2.0,
            ..Default::default()
        },
        // No texture on toasts: the kit builds their content before the skin is applied.
        toast: Surface { texture: None, shadows: vec![drop(4.0, 16.0, 0.6)], ..leather },
        kbd: Surface { ink: p.text_muted, border: Some(Border::all(1.0, gold.alpha(0.35))), radius: 2.0, ..Default::default() },
        medal: Medal { ring: [h("#d8c39a"), h("#86683c"), h("#36220f")], ring_on: [h("#fff0b8"), h("#e8b84a"), h("#7a5010")], outline: h("#0d0804") },
        emblem: &icons::CREST,
        glyphs: &[],
    };
    Theme {
        id: "hearthwood",
        name: "Hearthwood",
        blurb: "Carved wood, stone and gold",
        pal: p,
        ty,
        skin,
        textures: &[Tex::Wood, Tex::Stone, Tex::Leather],
        frame: None,
    }
});
