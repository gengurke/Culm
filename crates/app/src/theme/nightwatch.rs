//! Nightwatch: a dark, misty Duskwood night lit by torches. Cold blue-violet stone and iron,
//! bevelled panels that stand off the ground with deep shadows, dead trees and headstones in
//! silhouette along the header, mist drifting over the window ground. The pointer carries a
//! flickering torch and iron sconces burn on the walls (`light.rs`); away from them everything
//! stays legible in moonlight.

use super::light::{Glow, Lamp, Lights, Mist};
use super::{BASE_TYPE, BandDeco, Border, Overlay, Pal, Skin, Surface, Tex, Theme, Type, h, ink, inset};
use super::SIDEBAR_W;
use rune::tok::{Family, TextStyle};
use rune::{Color, Cycle, Icon, Shadow};
use std::sync::LazyLock;

const SERIF: Family = Family::Named("Cormorant Garamond, Cinzel, Marcellus, Noto Serif, serif");

const TORCH: &str = "#ff9a3c";

/// The brand mark: a hooded lantern with its flame.
static LANTERN: Icon = Icon {
    name: "ra-nw-lantern",
    view: 128.0,
    body: r#"<path d="M54 22 C54 8 74 8 74 22" fill="none" stroke="currentColor" stroke-width="6" stroke-linecap="round"/><path d="M46 22 H82 L90 36 H38 Z" fill="currentColor"/><rect x="43" y="40" width="42" height="56" rx="7" fill="none" stroke="currentColor" stroke-width="7"/><path d="M64 50 C74 62 74 75 64 86 C54 75 54 62 64 50Z" fill="currentColor"/><path d="M38 100 H90 L84 114 H44Z" fill="currentColor"/>"#,
};

/// A wrought-iron wall sconce holding a torch (the flame is [`FLAME`]).
static SCONCE: Icon = Icon {
    name: "ra-nw-sconce",
    view: 64.0,
    body: r##"<defs><linearGradient id="iron" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#5d5866"/><stop offset="1" stop-color="#16141b"/></linearGradient><linearGradient id="wood" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#6b4a2c"/><stop offset="1" stop-color="#2a1b10"/></linearGradient></defs><rect x="27" y="40" width="10" height="22" rx="2" fill="url(#iron)" stroke="#0a090c" stroke-width="1"/><circle cx="32" cy="45" r="1.4" fill="#8a8494"/><circle cx="32" cy="57" r="1.4" fill="#8a8494"/><path d="M32 46 C24 44 22 36 26 31" fill="none" stroke="#2b2830" stroke-width="3" stroke-linecap="round"/><rect x="29.5" y="22" width="5" height="14" rx="1.5" fill="url(#wood)"/><path d="M24 26 H40 L37 33 H27 Z" fill="url(#iron)" stroke="#0a090c" stroke-width="0.8"/><path d="M24.5 26 H39.5" stroke="#b08a5a" stroke-width="1" stroke-opacity="0.8"/>"##,
};

/// The sconce's flame, drawn over [`SCONCE`] and flickered by the lamp.
static FLAME: Icon = Icon {
    name: "ra-nw-flame",
    view: 64.0,
    body: r##"<defs><radialGradient id="f" cx="0.5" cy="0.72" r="0.6"><stop offset="0" stop-color="#fffbe6"/><stop offset="0.35" stop-color="#ffd270"/><stop offset="0.7" stop-color="#ff8a24"/><stop offset="1" stop-color="#d0400c" stop-opacity="0"/></radialGradient></defs><path d="M32 4 C36 11 42 15 41 21 C40 26 36 28 32 28 C27 28 23 25 23 20 C23 15 28 13 29 8 C30 12 31 13 32 14 C33 10 33 7 32 4Z" fill="url(#f)"/>"##,
};

// Two periods that never line up, sampled at 20 fps: a torch's uneven gutter, not a pulse.
const GUTTER: &[(f32, f32)] = &[
    (0.0, 0.92), (0.09, 1.0), (0.17, 0.83), (0.24, 0.96), (0.33, 0.88), (0.41, 0.99),
    (0.5, 0.78), (0.57, 0.93), (0.66, 0.86), (0.74, 1.0), (0.83, 0.82), (0.91, 0.95), (1.0, 0.92),
];
const LICK: &[(f32, f32)] = &[(0.0, 0.85), (0.13, 1.0), (0.29, 0.7), (0.45, 0.95), (0.6, 0.76), (0.78, 1.0), (0.9, 0.8), (1.0, 0.85)];

fn flicker(phase: f32) -> (Cycle, Cycle) {
    (
        Cycle { period: 2.3, phase, stops: GUTTER, still: 0.93, fps: 20.0 },
        Cycle { period: 0.97, phase: phase * 1.7, stops: LICK, still: 0.9, fps: 20.0 },
    )
}

fn drop(y: f32, blur: f32, a: f32) -> Shadow {
    Shadow { y, blur, color: Color::BLACK.alpha(a), ..Shadow::default() }
}

pub static LIGHTS: LazyLock<Lights> = LazyLock::new(|| {
    let torch = h(TORCH);
    let flame = h("#ffd08a");
    let lamp = |x: f32, y: f32, from_right: bool, phase: f32| Lamp {
        x,
        y,
        from_right,
        glow: Glow { color: torch, radius: 300.0, strength: 0.28, core: Some((flame.alpha(0.3), 0.2)), flicker: Some(flicker(phase)) },
        fixture: Some((&SCONCE, &FLAME, 54.0)),
    };
    Lights {
        torch: Some(Glow { color: torch, radius: 380.0, strength: 0.26, core: Some((flame.alpha(0.2), 0.28)), flicker: Some(flicker(0.0)) }),
        lamps: vec![lamp(SIDEBAR_W + 560.0, 30.0, false, 0.31), lamp(110.0, 30.0, true, 0.67), lamp(SIDEBAR_W - 36.0, 420.0, false, 0.13)],
        mist: Some(Mist { tex: Tex::Mist, alpha: 0.2, scale: 2.0, cycle: Cycle { period: 36.0, phase: 0.0, stops: &[(0.0, 0.0), (0.333, 1.0), (0.667, 0.0), (1.0, 0.0)], still: 0.5, fps: 6.0 } }),
        vignette: Some(h("#020306").alpha(0.5)),
    }
});

pub static THEME: LazyLock<Theme> = LazyLock::new(|| {
    let moon = h("#a9bde0");
    let steel = h("#33405a");
    let p = Pal {
        sidebar: h("#0d121b"),
        footer: h("#0c1119"),
        bg: h("#0a0e15"),
        panel: h("#131a26"),
        panel_alt: h("#1b2333"),
        sky: h("#1a2236"),
        well: h("#020308").alpha(0.55),
        overlay: h("#020308").alpha(0.78),
        text: h("#dfe6f1"),
        heading: h("#f2dcae"),
        text_dim: h("#adb8ca"),
        text_muted: h("#8895aa"),
        text_off: h("#5d6880"),
        on_accent: h("#1a0d03"),
        border: moon.alpha(0.14),
        border_strong: moon.alpha(0.26),
        divider: moon.alpha(0.08),
        hover: h("#ffb766").alpha(0.07),
        selected: h("#ffb766").alpha(0.10),
        chip: moon.alpha(0.08),
        track: moon.alpha(0.12),
        accent: h("#f2a541"),
        accent_hi: h("#ffc978"),
        accent_warm: h("#fff0d0"),
        accent_soft: h("#f2a541").alpha(0.16),
        accent_focus: h("#ffb24d").alpha(0.85),
        success: h("#7fd6a0"),
        success_soft: h("#7fd6a0").alpha(0.13),
        warning: h("#f5b851"),
        warning_soft: h("#f5b851").alpha(0.14),
        danger: h("#f07a6e"),
        danger_soft: h("#f07a6e").alpha(0.11),
        danger_line: h("#f07a6e").alpha(0.45),
        info: h("#8fbaf0"),
        info_soft: h("#8fbaf0").alpha(0.14),
        haze: moon.alpha(0.16),
    };
    let ty = Type {
        display: TextStyle::new(25.0, 600.0).family(SERIF).tracking(0.2).lh(30.0),
        hero: TextStyle::new(30.0, 600.0).family(SERIF).tracking(0.3).lh(34.0),
        title: TextStyle::new(21.0, 600.0).family(SERIF).tracking(0.2).lh(26.0),
        heading: TextStyle::new(17.0, 600.0).family(SERIF).tracking(0.3).lh(22.0),
        eyebrow: TextStyle::new(10.5, 600.0).tracking(1.6).lh(14.0),
        wordmark: TextStyle::new(16.0, 600.0).family(SERIF).tracking(2.6).lh(20.0),
        ..BASE_TYPE
    };
    let under = Some(Border::sides([0.0, 0.0, 1.0, 0.0], p.divider));
    // Lit from above: a cool rim along the top edge, the bottom edge in shade.
    let bevel = Some(Border::bevel(1.0, h("#3a4762"), h("#06080d")));
    let slab = Surface {
        fill: p.panel,
        vertical: Some((h("#18202f"), h("#0f141e"))),
        border: bevel,
        radius: 6.0,
        shadows: vec![inset(1.0, moon.alpha(0.07)), drop(10.0, 26.0, 0.6), drop(2.0, 4.0, 0.5)],
        ..Default::default()
    };
    let iron = Surface {
        fill: p.panel_alt,
        ink: h("#d9e1ee"),
        vertical: Some((h("#252f43"), h("#151b28"))),
        border: Some(Border::bevel(1.0, h("#4a5874"), h("#05070b"))),
        radius: 5.0,
        shadows: vec![inset(1.0, moon.alpha(0.10)), drop(2.0, 5.0, 0.55)],
        hover: h("#ffb766").alpha(0.08),
        ..Default::default()
    };
    let skin = Skin {
        window: Surface { fill: p.bg, ink: p.text, vertical: Some((h("#0c111a"), h("#080b11"))), ..Default::default() },
        sidebar: Surface {
            fill: p.sidebar.alpha(0.86),
            vertical: Some((h("#111827").alpha(0.9), h("#090d14").alpha(0.86))),
            border: Some(Border::sides([0.0, 1.0, 0.0, 0.0], h("#1f283a"))),
            shadows: vec![Shadow { x: 6.0, blur: 22.0, color: Color::BLACK.alpha(0.55), ..Shadow::default() }],
            ..Default::default()
        },
        band: Surface {
            fill: p.sky,
            vertical: Some((h("#1d2640"), h("#0c111b"))),
            overlay: Some(Overlay::Radial { center: [0.82, 0.05], radius: [520.0, 150.0], stops: vec![(0.0, moon.alpha(0.20)), (0.5, moon.alpha(0.06)), (1.0, moon.alpha(0.0))] }),
            border: Some(Border::sides([0.0, 0.0, 1.0, 0.0], h("#26314a"))),
            shadows: vec![drop(8.0, 22.0, 0.65)],
            ..Default::default()
        },
        band_deco: BandDeco::Treeline,
        toolbar: Surface { fill: Color::BLACK.alpha(0.18), border: under, ..Default::default() },
        panel: slab.clone(),
        detail: Surface {
            fill: p.sidebar,
            vertical: Some((h("#121927"), h("#0a0e16"))),
            border: Some(Border::sides([0.0, 0.0, 0.0, 1.0], h("#26314a"))),
            shadows: vec![Shadow { x: -8.0, blur: 24.0, color: Color::BLACK.alpha(0.55), ..Shadow::default() }],
            ..Default::default()
        },
        menu: Surface { radius: 0.0, shadows: vec![drop(-6.0, 18.0, 0.6)], ..slab.clone() },
        list_header: Surface { fill: Color::BLACK.alpha(0.22), border: Some(Border::sides([0.0, 0.0, 1.0, 0.0], p.border)), ..Default::default() },
        row: Surface { border: under, hover: p.hover, ..Default::default() },
        row_selected: Surface {
            fill: p.selected,
            overlay: Some(Overlay::Linear { angle: std::f32::consts::FRAC_PI_2, stops: vec![(0.0, p.accent.alpha(0.16)), (0.7, p.accent.alpha(0.0))] }),
            border: under,
            hover: p.hover,
            ..Default::default()
        },
        nav: Surface { hover: p.hover, ..Default::default() },
        nav_active: Surface {
            overlay: Some(Overlay::Linear { angle: std::f32::consts::FRAC_PI_2, stops: vec![(0.0, p.accent.alpha(0.20)), (0.9, p.accent.alpha(0.0))] }),
            hover: p.hover,
            ..Default::default()
        },
        primary: Surface {
            fill: h("#d9832c"),
            ink: p.on_accent,
            vertical: Some((h("#f6ae55"), h("#c0661c"))),
            border: Some(Border::bevel(1.0, h("#ffdca6"), h("#6a3510"))),
            radius: 5.0,
            shadows: vec![inset(1.0, h("#fff0d0").alpha(0.45)), Shadow { y: 2.0, blur: 14.0, color: h(TORCH).alpha(0.35), ..Shadow::default() }, drop(2.0, 4.0, 0.6)],
            hover: h("#fff0d0").alpha(0.2),
            ..Default::default()
        },
        secondary: iron.clone(),
        ghost: Surface { ink: p.text_dim, border: Some(Border::all(1.0, Color::TRANSPARENT)), radius: 5.0, hover: p.hover, ..Default::default() },
        danger: Surface {
            fill: h("#2a0f10"),
            ink: h("#ff8f82"),
            vertical: Some((h("#3a1416"), h("#1f0a0b"))),
            border: Some(Border::bevel(1.0, h("#9c3c38"), h("#140506"))),
            radius: 5.0,
            shadows: vec![drop(2.0, 5.0, 0.55)],
            hover: h("#ff6a5a").alpha(0.12),
            ..Default::default()
        },
        icon_button: Surface { ink: p.text_muted, radius: 5.0, hover: p.hover, ..Default::default() },
        input: Surface {
            fill: p.well,
            // Sunk into the slab: shade along the top, the rim catches light along the bottom.
            border: Some(Border::bevel(1.0, h("#020306"), h("#33405a"))),
            radius: 5.0,
            shadows: vec![inset(3.0, Color::BLACK.alpha(0.55))],
            ..Default::default()
        },
        chip: Surface { radius: 4.0, border: Some(Border::all(1.0, ink(0.06))), ..Default::default() },
        tile: Surface { radius: 5.0, ink: h("#c9d3e3"), shadows: vec![inset(1.0, moon.alpha(0.1)), drop(2.0, 6.0, 0.6)], ..iron.clone() },
        segments: Surface { fill: Color::BLACK.alpha(0.3), border: Some(Border::bevel(1.0, h("#020306"), steel)), radius: 5.0, shadows: vec![inset(2.0, Color::BLACK.alpha(0.45))], ..Default::default() },
        segment_on: Surface { radius: 4.0, ink: p.heading, shadows: vec![drop(1.0, 3.0, 0.6)], ..iron.clone() },
        toast: Surface { shadows: vec![drop(10.0, 30.0, 0.7)], ..slab },
        kbd: Surface { ink: p.text_muted, border: Some(Border::bevel(1.0, steel, h("#05070b"))), radius: 3.0, ..Default::default() },
        emblem: &LANTERN,
    };
    Theme {
        id: "nightwatch",
        name: "Nightwatch",
        blurb: "Duskwood fog, torchlight on iron",
        pal: p,
        ty,
        skin,
        textures: &[Tex::Mist],
    }
});
