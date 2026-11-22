//! Emberglow: green-black carved stone lit by glowing gold. A near-black olive ground of cracked,
//! mossy flagstone, parchment text, warm gold serif headings, gold gradient buttons and tabs that
//! glow amber from below. Textures are generated (texture.rs).

use super::{BASE_TYPE, BandDeco, Border, Corners, Medal, Overlay, Pal, Skin, Surface, Tex, Theme, Tone, Type, drop, h, inner_ring, inset, outer_ring};
use super::glyphs;
use rune::tok::{Family, TextStyle};
use rune::{Color, Shadow};
use std::f32::consts::PI;
use std::sync::LazyLock;

/// The heading face, bundled like every font (`theme::FONTS`).
const SERIF: Family = Family::Named("Cinzel");

fn glow(blur: f32, color: Color) -> Shadow {
    Shadow { blur, color, ..Shadow::default() }
}

pub static THEME: LazyLock<Theme> = LazyLock::new(|| {
    let gold = h("#e0ae4a");
    let bronze = h("#9a7a3e");
    let amber = h("#ffa032");
    let parchment = h("#ece3cc");
    let sidebar = h("#0a0f08");
    let panel_alt = h("#1b2114");
    let on_accent = h("#1e1406");
    let selected = amber.alpha(0.10);
    let p = Pal {
        bg: h("#070b05"),
        panel: h("#11170d"),
        text: parchment,
        heading: h("#ffcf66"),
        text_dim: h("#d2c9b0"),
        text_muted: h("#a59d86"),
        border: bronze.alpha(0.34),
        border_strong: bronze.alpha(0.6),
        divider: bronze.alpha(0.16),
        hover: h("#ffd98a").alpha(0.06),
        chip: parchment.alpha(0.08),
        track: parchment.alpha(0.12),
        accent: h("#f0b84f"),
        accent_hi: h("#ffd978"),
        accent_focus: gold.alpha(0.85),
        success: Tone { ink: h("#8fd86a"), soft: h("#8fd86a").alpha(0.14), line: h("#8fd86a").alpha(0.4) },
        warning: Tone { ink: h("#ffa94d"), soft: h("#ffa94d").alpha(0.15), line: h("#ffa94d").alpha(0.4) },
        danger: Tone { ink: h("#ff6b52"), soft: h("#ff6b52").alpha(0.10), line: h("#ff6b52").alpha(0.50) },
        busy: Tone { ink: h("#f0b84f"), soft: h("#f0b84f").alpha(0.17), line: h("#f0b84f").alpha(0.4) },
    };
    let ty = Type {
        display: TextStyle::new(24.0, 700.0).family(SERIF).tracking(0.6).lh(30.0),
        hero: TextStyle::new(21.0, 700.0).family(SERIF).tracking(0.8).lh(26.0),
        title: TextStyle::new(19.0, 700.0).family(SERIF).tracking(0.5).lh(24.0),
        heading: TextStyle::new(15.5, 700.0).family(SERIF).tracking(0.5).lh(20.0),
        eyebrow: TextStyle::new(10.5, 700.0).family(SERIF).tracking(1.8).lh(14.0),
        // Buttons, tabs and segments in small caps, like engraved plaques.
        label: TextStyle::new(13.5, 600.0).family(SERIF).features("'smcp' 1").tracking(0.4).lh(16.0),
        ..BASE_TYPE
    };
    let under = Some(Border::sides([0.0, 0.0, 1.0, 0.0], p.divider));
    // Carved stone: a lit top lip, a dark bottom, a black outer ring.
    let carved = Some(Border::bevel(2.0, h("#6e6640"), h("#23251a")));
    let studs = Some(Corners { glyph: &glyphs::STUD, size: 7.0, inset: 4.0, color: bronze.alpha(0.85) });
    // Cards stay untextured (many on screen); the frame does the work: a carved olive-bronze
    // rim, a fine gold line inside it, a black outer ring and a soft drop.
    let stone = Surface {
        fill: p.panel,
        vertical: Some((h("#182012"), h("#0d1209"))),
        overlay: Some(Overlay::Radial { center: [0.5, 0.0], radius: [420.0, 160.0], stops: vec![(0.0, h("#c8b07a").alpha(0.06)), (1.0, h("#c8b07a").alpha(0.0))] }),
        border: carved,
        radius: 3.0,
        shadows: vec![
            Shadow { spread: 3.0, color: h("#0c1008"), inset: true, ..Shadow::default() },
            Shadow { spread: 4.0, color: gold.alpha(0.28), inset: true, ..Shadow::default() },
            outer_ring(Color::BLACK.alpha(0.75)),
            drop(4.0, 12.0, 0.6),
        ],
        ..Default::default()
    };
    let slab = Surface { fill: sidebar, vertical: Some((h("#0d130a"), h("#080c06"))), texture: Some((Tex::Moss, 0.42)), ..Default::default() };
    let press = Color::BLACK.alpha(0.22);
    let panel = Surface { corners: studs, ..stone.clone() };
    let skin = Skin {
        window: Surface { fill: p.bg, ink: p.text, texture: Some((Tex::Moss, 0.42)), ..Default::default() },
        sidebar: Surface { border: Some(Border::sides([0.0, 1.0, 0.0, 0.0], bronze.alpha(0.45))), ..slab.clone() },
        band: Surface {
            fill: h("#0e140a"),
            vertical: Some((h("#151c0f"), h("#090d06"))),
            overlay: Some(Overlay::Radial { center: [0.5, 1.0], radius: [620.0, 120.0], stops: vec![(0.0, amber.alpha(0.13)), (1.0, amber.alpha(0.0))] }),
            texture: Some((Tex::Moss, 0.42)),
            border: Some(Border::sides([0.0, 0.0, 2.0, 0.0], gold.alpha(0.7))),
            shadows: vec![glow(14.0, amber.alpha(0.22)), drop(3.0, 10.0, 0.7)],
            ..Default::default()
        },
        band_deco: BandDeco::Embers,
        toolbar: Surface { fill: Color::BLACK.alpha(0.25), border: Some(Border::sides([0.0, 0.0, 1.0, 0.0], bronze.alpha(0.25))), ..Default::default() },
        panel: panel.clone(),
        detail: Surface { border: Some(Border::sides([0.0, 0.0, 0.0, 1.0], bronze.alpha(0.45))), ..slab },
        menu: Surface {
            overlay: None,
            border: Some(Border::bevel(1.0, h("#a08048"), h("#3a3018"))),
            shadows: vec![inset(1.0, h("#ffe0a0").alpha(0.08)), outer_ring(Color::BLACK.alpha(0.75)), drop(6.0, 18.0, 0.7)],
            ..stone
        },
        list_header: Surface { fill: Color::BLACK.alpha(0.3), border: Some(Border::sides([0.0, 0.0, 1.0, 0.0], bronze.alpha(0.3))), ..Default::default() },
        // Lit amber from below, like the open tab, in a thin gold ring.
        row_selected: Surface {
            fill: selected,
            overlay: Some(Overlay::Radial { center: [0.5, 1.3], radius: [700.0, 50.0], stops: vec![(0.0, amber.alpha(0.2)), (1.0, amber.alpha(0.0))] }),
            border: under,
            shadows: vec![inner_ring(gold.alpha(0.45))],
            hover: p.hover,
            ..Default::default()
        },
        nav_active: Surface {
            ink: p.heading,
            fill: h("#0d1209").alpha(0.9),
            overlay: Some(Overlay::Radial { center: [0.5, 1.15], radius: [150.0, 34.0], stops: vec![(0.0, amber.alpha(0.42)), (1.0, amber.alpha(0.0))] }),
            border: Some(Border::margin([3.0, 10.0, 3.0, 10.0])),
            radius: 5.0,
            shadows: vec![
                inner_ring(gold),
                Shadow { spread: 2.0, color: Color::BLACK.alpha(0.6), inset: true, ..Shadow::default() },
            ],
            hover: p.hover,
            ..Default::default()
        },
        primary: Surface {
            fill: h("#e2a43e"),
            ink: on_accent,
            overlay: Some(Overlay::Linear { angle: PI, stops: vec![(0.0, h("#ffd978")), (0.55, h("#e2a43e")), (1.0, h("#b9791f"))] }),
            radius: 5.0,
            shadows: vec![inset(1.0, Color::WHITE.alpha(0.5)), outer_ring(Color::BLACK.alpha(0.6)), drop(2.0, 6.0, 0.6)],
            hover: h("#fff4d0").alpha(0.2),
            press: h("#5a3008").alpha(0.25),
            ..Default::default()
        },
        secondary: Surface {
            fill: h("#141a0f"),
            ink: h("#f0dcae"),
            vertical: Some((h("#1c2315"), h("#0e130a"))),
            border: Some(Border::bevel(1.0, h("#a08048"), h("#5a4626"))),
            radius: 5.0,
            shadows: vec![inset(1.0, h("#ffe0a0").alpha(0.08)), drop(1.0, 3.0, 0.6)],
            hover: h("#ffd98a").alpha(0.08),
            press,
            ..Default::default()
        },
        danger: Surface {
            fill: h("#2a0d08"),
            ink: h("#ff8a72"),
            vertical: Some((h("#3a120b"), h("#220905"))),
            border: Some(Border::bevel(1.0, h("#c8503a"), h("#5a1a10"))),
            radius: 5.0,
            hover: h("#ff5a40").alpha(0.14),
            ..Default::default()
        },
        input: Surface {
            fill: h("#0a0e07"),
            vertical: Some((h("#05070a"), h("#11170d"))),
            border: Some(Border::bevel(1.0, h("#8a6c38"), h("#c89a48"))),
            radius: 5.0,
            shadows: vec![inset(2.0, Color::BLACK.alpha(0.6)), glow(10.0, amber.alpha(0.12))],
            ..Default::default()
        },
        toggle: Surface { radius: 4.0, ..Default::default() },
        tile: Surface {
            fill: panel_alt,
            ink: h("#ecd6a4"),
            vertical: Some((h("#20281a"), h("#11160c"))),
            border: Some(Border::bevel(2.0, h("#e0b860"), h("#6a4a1e"))),
            radius: 3.0,
            shadows: vec![inner_ring(Color::BLACK.alpha(0.7)), outer_ring(Color::BLACK.alpha(0.6))],
            ..Default::default()
        },
        segments: Surface {
            fill: Color::BLACK.alpha(0.35),
            border: Some(Border::bevel(1.0, h("#0a0d07"), h("#4a4024"))),
            radius: 5.0,
            shadows: vec![inset(2.0, Color::BLACK.alpha(0.5))],
            ..Default::default()
        },
        toggle_on: Surface {
            fill: h("#0d1009").alpha(0.85),
            ink: p.heading,
            overlay: Some(Overlay::Radial { center: [0.5, 1.0], radius: [70.0, 30.0], stops: vec![(0.0, amber.alpha(0.3)), (1.0, amber.alpha(0.0))] }),
            border: Some(Border::all(1.0, gold)),
            radius: 5.0,
            shadows: vec![inset(2.0, Color::BLACK.alpha(0.55)), glow(12.0, h("#ff9632").alpha(0.4))],
            ..Default::default()
        },
        toast: Surface { shadows: [panel.shadows.clone(), vec![drop(6.0, 18.0, 0.5)]].concat(), ..panel },
        kbd: Surface { ink: p.text_muted, border: Some(Border::all(1.0, bronze.alpha(0.45))), radius: 3.0, ..Default::default() },
        medal: Medal { ring: [h("#d9c08a"), h("#7d6a44"), h("#2b2414")], ring_on: [h("#fff0b8"), h("#f0b84f"), h("#7a4e10")], outline: h("#020301") },
        emblem: &glyphs::CREST,
        ..Skin::base(&p, 5.0)
    };
    Theme {
        id: "emberglow",
        name: "Emberglow",
        blurb: "Black stone, glowing gold",
        pal: p,
        ty,
        skin,
        frame: None,
    }
});
