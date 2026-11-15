//! Goblin Ledger: a calm, neutral in-game menu, like an auction house or a settings window.
//! Near-black ground, dark charcoal-brown panels with a 1 pt inner bevel, rounded tiles, ochre
//! gold accents, serif headings in off-white and one deep red action button. Flat, no textures:
//! the practical daily driver.

use super::{Medal, BASE_TYPE, BandDeco, Border, Pal, Skin, Surface, Theme, Type, h, ink, inset};
use crate::icons;
use rune::tok::{Family, TextStyle};
use rune::{Color, Shadow};
use std::sync::LazyLock;

/// Only Noto Serif ships with the system; the others are used when the user has them.
const SERIF: Family = Family::Named("Marcellus, Cinzel, Cormorant SC, Noto Serif, serif");

fn drop(y: f32, blur: f32, a: f32) -> Shadow {
    Shadow { y, blur, color: Color::BLACK.alpha(a), ..Shadow::default() }
}

pub static THEME: LazyLock<Theme> = LazyLock::new(|| {
    let gold = h("#ccaa44");
    let tile = h("#282322");
    let p = Pal {
        sidebar: h("#0c0605"),
        footer: h("#0a0504"),
        bg: h("#060100"),
        panel: h("#1e1918"),
        panel_alt: tile,
        sky: h("#161110"),
        well: h("#282322"),
        overlay: h("#030000").alpha(0.78),
        text: h("#eeeeee"),
        heading: h("#f2eee6"),
        text_dim: h("#cccccc"),
        text_muted: h("#9a9390"),
        text_off: h("#6a6361"),
        on_accent: h("#1a1205"),
        border: ink(0.09),
        border_strong: ink(0.18),
        divider: ink(0.06),
        hover: ink(0.05),
        selected: gold.alpha(0.10),
        chip: ink(0.07),
        track: ink(0.10),
        accent: gold,
        accent_hi: h("#e6c860"),
        accent_warm: h("#f6e7b4"),
        accent_soft: gold.alpha(0.16),
        accent_focus: h("#66c6e0").alpha(0.85),
        success: h("#62d26f"),
        success_soft: h("#62d26f").alpha(0.13),
        warning: h("#f0b44a"),
        warning_soft: h("#f0b44a").alpha(0.14),
        danger: h("#f0645a"),
        danger_soft: h("#f0645a").alpha(0.10),
        danger_line: h("#f0645a").alpha(0.45),
        info: h("#66c6e0"),
        info_soft: h("#66c6e0").alpha(0.13),
        haze: gold.alpha(0.05),
    };
    let ty = Type {
        display: TextStyle::new(23.0, 500.0).family(SERIF).tracking(0.2).lh(28.0),
        hero: TextStyle::new(21.0, 500.0).family(SERIF).tracking(0.3).lh(26.0),
        title: TextStyle::new(18.5, 500.0).family(SERIF).tracking(0.2).lh(24.0),
        heading: TextStyle::new(15.5, 500.0).family(SERIF).tracking(0.2).lh(20.0),
        eyebrow: TextStyle::new(10.5, 600.0).family(SERIF).tracking(1.4).lh(14.0),
        wordmark: TextStyle::new(15.0, 600.0).family(SERIF).tracking(2.4).lh(20.0),
        ..BASE_TYPE
    };
    // The panel frame: a thin dark line outside a 1 pt lighter top edge (the in-game bevel).
    let frame = Some(Border::bevel(1.0, h("#3a3331"), h("#141010")));
    let under = Some(Border::sides([0.0, 0.0, 1.0, 0.0], p.divider));
    let lip = inset(1.0, ink(0.04));
    let panel = Surface { fill: p.panel, border: frame, radius: 8.0, shadows: vec![lip, drop(2.0, 6.0, 0.5)], ..Default::default() };
    // Sidebar rows look like separate tiles: a border in the sidebar colour leaves a gap round each.
    let gap = |c: Color| Some(Border { sides: [3.0, 6.0, 3.0, 6.0], top: c, bottom: c });
    let skin = Skin {
        window: Surface { fill: p.bg, ink: p.text, ..Default::default() },
        sidebar: Surface { fill: p.sidebar, border: Some(Border::sides([0.0, 1.0, 0.0, 0.0], ink(0.05))), ..Default::default() },
        band: Surface { fill: p.sky, vertical: Some((h("#1a1413"), p.bg)), border: Some(Border::sides([0.0, 0.0, 1.0, 0.0], gold.alpha(0.22))), ..Default::default() },
        band_deco: BandDeco::None,
        toolbar: Surface { border: under, ..Default::default() },
        panel: panel.clone(),
        detail: Surface { fill: h("#120d0c"), border: Some(Border::sides([0.0, 0.0, 0.0, 1.0], ink(0.07))), ..Default::default() },
        menu: Surface { fill: h("#24201f"), border: frame, radius: 8.0, shadows: vec![lip, drop(6.0, 18.0, 0.6)], ..Default::default() },
        list_header: Surface { fill: Color::WHITE.alpha(0.025), border: under, ..Default::default() },
        row: Surface { border: under, hover: p.hover, ..Default::default() },
        row_selected: Surface { fill: p.selected, border: Some(Border::sides([0.0, 0.0, 1.0, 0.0], gold.alpha(0.25))), hover: p.hover, ..Default::default() },
        nav: Surface { fill: tile, border: gap(p.sidebar), radius: 14.0, hover: p.hover, ..Default::default() },
        nav_active: Surface { fill: h("#3a3230"), border: gap(p.sidebar), radius: 14.0, hover: p.hover, ..Default::default() },
        primary: Surface {
            fill: h("#6e0808"),
            ink: p.heading,
            vertical: Some((h("#7e0c0a"), h("#4c0505"))),
            border: Some(Border::bevel(1.0, h("#c0403a"), h("#7a1a16"))),
            radius: 6.0,
            shadows: vec![inset(1.0, h("#ff8070").alpha(0.22)), drop(1.0, 3.0, 0.6)],
            hover: h("#ff6050").alpha(0.12),
            ..Default::default()
        },
        secondary: Surface {
            fill: tile,
            ink: p.text,
            border: Some(Border::bevel(1.0, h("#4a4240"), h("#1c1716"))),
            radius: 6.0,
            shadows: vec![lip],
            hover: p.hover,
            ..Default::default()
        },
        ghost: Surface { ink: p.text_dim, border: Some(Border::all(1.0, Color::TRANSPARENT)), radius: 6.0, hover: p.hover, ..Default::default() },
        danger: Surface { fill: p.danger_soft, ink: p.danger, border: Some(Border::all(1.0, p.danger_line)), radius: 6.0, hover: p.danger.alpha(0.12), ..Default::default() },
        icon_button: Surface { ink: p.text_muted, radius: 6.0, hover: p.hover, ..Default::default() },
        input: Surface { fill: p.well, border: Some(Border::bevel(1.0, h("#141010"), h("#3a3331"))), radius: 8.0, ..Default::default() },
        chip: Surface { radius: 6.0, ..Default::default() },
        tile: Surface { fill: tile, ink: gold, border: frame, radius: 6.0, ..Default::default() },
        segments: Surface { fill: h("#120d0c"), border: Some(Border::all(1.0, ink(0.08))), radius: 8.0, ..Default::default() },
        segment_on: Surface { fill: h("#3a3230"), ink: gold, border: Some(Border::all(1.0, gold.alpha(0.45))), radius: 7.0, ..Default::default() },
        toast: Surface { shadows: vec![drop(4.0, 16.0, 0.6)], ..panel },
        kbd: Surface { ink: p.text_muted, border: Some(Border::all(1.0, ink(0.16))), radius: 4.0, ..Default::default() },
        medal: Medal { ring: [h("#dadada"), h("#7a7a7a"), h("#2a2a2a")], ring_on: [h("#fff0b8"), h("#ccaa44"), h("#6a5010")], outline: h("#000000") },
        emblem: &icons::EMBLEM,
        glyphs: &[],
    };
    Theme {
        id: "ledger",
        name: "Goblin Ledger",
        blurb: "Charcoal menus, ochre gold",
        pal: p,
        ty,
        skin,
        textures: &[],
        frame: None,
    }
});
