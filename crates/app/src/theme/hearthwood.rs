//! Hearthwood: the feel of a classic fantasy MMO's in-game UI. Carved wood and stone frames,
//! dark leather panels with gold studs, gold serif headings in quest yellow, red lacquered
//! buttons with a bevel, item-quality status colours. Textures are generated (texture.rs).

use super::{BASE_TYPE, BandDeco, Border, Corners, Medal, Overlay, Pal, Skin, Surface, Tex, Theme, Tone, Type, drop, face, h, inner_ring, inset};
use super::glyphs;
use rune::tok::{Family, TextStyle};
use rune::Color;
use std::sync::LazyLock;

/// The heading face, bundled like every font (`theme::FONTS`).
const SERIF: Family = Family::Named("Cinzel");

pub static THEME: LazyLock<Theme> = LazyLock::new(|| {
    let gold = h("#c9a04a");
    let parchment = h("#f4e7cc");
    let panel_alt = h("#3b2a1a");
    let well = h("#0b0704").alpha(0.75);
    let selected = h("#ffc95a").alpha(0.11);
    let p = Pal {
        bg: h("#1b140e"),
        panel: h("#2a1d13"),
        text: parchment,
        heading: h("#ffd100"),
        text_dim: h("#d9c6a0"),
        text_muted: h("#ad9876"),
        border: gold.alpha(0.30),
        border_strong: gold.alpha(0.55),
        divider: gold.alpha(0.14),
        hover: h("#ffdc9a").alpha(0.07),
        chip: parchment.alpha(0.08),
        track: parchment.alpha(0.12),
        accent: h("#e8b84a"),
        accent_hi: h("#ffd96b"),
        accent_focus: h("#ffd100").alpha(0.80),
        success: Tone { ink: h("#5bd65b"), soft: h("#5bd65b").alpha(0.15), line: h("#5bd65b").alpha(0.4) },
        warning: Tone { ink: h("#ffa135"), soft: h("#ffa135").alpha(0.16), line: h("#ffa135").alpha(0.4) },
        danger: Tone { ink: h("#ff5a48"), soft: h("#ff5a48").alpha(0.10), line: h("#ff5a48").alpha(0.50) },
        busy: Tone { ink: h("#e8b84a"), soft: h("#e8b84a").alpha(0.18), line: h("#e8b84a").alpha(0.4) },
    };
    let ty = Type {
        display: TextStyle::new(24.0, 700.0).family(SERIF).tracking(0.2).lh(30.0),
        hero: TextStyle::new(21.0, 700.0).family(SERIF).tracking(0.4).lh(26.0),
        title: TextStyle::new(19.0, 700.0).family(SERIF).tracking(0.2).lh(24.0),
        heading: TextStyle::new(15.5, 700.0).family(SERIF).tracking(0.3).lh(20.0),
        eyebrow: TextStyle::new(10.5, 700.0).family(SERIF).tracking(1.5).lh(14.0),
        ..BASE_TYPE
    };
    let under = Some(Border::sides([0.0, 0.0, 1.0, 0.0], p.divider));
    let bevel = Some(Border::bevel(1.0, h("#b08a44"), h("#4a3418")));
    let studs = Some(Corners { glyph: &glyphs::STUD, size: 9.0, inset: 3.0, color: h("#d9b25a") });
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
    let panel = Surface { corners: studs, ..leather.clone() };
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
        panel: panel.clone(),
        detail: Surface { border: Some(Border::sides([0.0, 0.0, 0.0, 2.0], h("#8a6a2e"))), ..wood },
        menu: Surface { overlay: None, shadows: vec![inset(1.0, h("#ffcf80").alpha(0.1)), drop(6.0, 18.0, 0.65)], ..leather },
        list_header: Surface { fill: Color::BLACK.alpha(0.22), border: Some(Border::sides([0.0, 0.0, 1.0, 0.0], gold.alpha(0.25))), ..Default::default() },
        // A warm-lit row in a thin gold ring.
        row_selected: Surface {
            fill: selected,
            vertical: Some((gold.alpha(0.16), gold.alpha(0.07))),
            border: under,
            shadows: vec![inner_ring(gold.alpha(0.45))],
            hover: p.hover,
            ..Default::default()
        },
        // A gilt plaque on the wood, inset from the sidebar's edges.
        nav_active: Surface {
            ink: p.heading,
            border: Some(Border::margin([3.0, 10.0, 3.0, 10.0])),
            radius: 2.0,
            shadows: vec![
                face(h("#3a2814").alpha(0.92)),
                inset(2.0, h("#ffcf80").alpha(0.14)),
                inset(-2.0, Color::BLACK.alpha(0.35)),
                inner_ring(gold.alpha(0.6)),
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
            fill: panel_alt,
            ink: h("#f0dcae"),
            vertical: Some((h("#4c3622"), h("#2c1f13"))),
            border: bevel,
            radius: 3.0,
            shadows: vec![drop(1.0, 2.0, 0.6)],
            hover: h("#ffdc9a").alpha(0.08),
            ..Default::default()
        },
        danger: Surface {
            fill: h("#3a100b"),
            ink: h("#ff7a66"),
            vertical: Some((h("#4a140d"), h("#2c0b07"))),
            border: Some(Border::bevel(1.0, h("#d0503a"), h("#5a1a10"))),
            radius: 3.0,
            hover: h("#ff5a40").alpha(0.14),
            ..Default::default()
        },
        input: Surface {
            fill: well,
            border: Some(Border::bevel(1.0, h("#120a05"), h("#8a6a35"))),
            radius: 2.0,
            shadows: vec![inset(2.0, Color::BLACK.alpha(0.5))],
            ..Default::default()
        },
        tile: Surface { fill: panel_alt, ink: h("#e8d3a8"), vertical: Some((h("#4a3422"), h("#2a1d12"))), border: bevel, radius: 2.0, ..Default::default() },
        segments: Surface { fill: Color::BLACK.alpha(0.25), border: Some(Border::bevel(1.0, h("#1a1009"), h("#6a5028"))), radius: 3.0, ..Default::default() },
        toggle_on: Surface {
            fill: h("#5a3e22"),
            ink: p.heading,
            vertical: Some((h("#6a4a28"), h("#4a3219"))),
            border: Some(Border::all(1.0, gold.alpha(0.75))),
            radius: 2.0,
            ..Default::default()
        },
        toast: Surface { shadows: vec![drop(2.0, 8.0, 0.55), drop(6.0, 18.0, 0.5)], ..panel },
        kbd: Surface { ink: p.text_muted, border: Some(Border::all(1.0, gold.alpha(0.35))), radius: 2.0, ..Default::default() },
        medal: Medal { ring: [h("#d8c39a"), h("#86683c"), h("#36220f")], ring_on: [h("#fff0b8"), h("#e8b84a"), h("#7a5010")], outline: h("#0d0804") },
        emblem: &glyphs::CREST,
        ..Skin::base(&p, 3.0)
    };
    Theme {
        id: "hearthwood",
        name: "Hearthwood",
        blurb: "Carved wood, stone and gold",
        pal: p,
        ty,
        skin,
        frame: None,
    }
});
