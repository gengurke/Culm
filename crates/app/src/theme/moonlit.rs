//! Moonlit: a calm navy night sky with one warm gold star. Square
//! hairline boxes, Inter, a starry header band. The default theme.

use super::{BASE_TYPE, BandDeco, Border, Overlay, Pal, Skin, Surface, Theme, h, ink, inset};
use crate::icons;
use rune::{Color, Shadow, oklcha};
use std::sync::LazyLock;

pub static THEME: LazyLock<Theme> = LazyLock::new(|| {
    let p = Pal {
        sidebar: h("#121624"),
        footer: h("#141927"),
        bg: h("#181d2d"),
        panel: h("#1c2234"),
        panel_alt: h("#232a3f"),
        sky: h("#283254"),
        well: h("#000005").alpha(0.35),
        overlay: h("#05080f").alpha(0.72),
        text: h("#f7f8fd"),
        heading: h("#f7f8fd"),
        text_dim: h("#c3cadd"),
        text_muted: h("#9ba2b9"),
        text_off: h("#6e7590"),
        on_accent: h("#0a0d14"),
        border: ink(0.10),
        border_strong: ink(0.20),
        divider: ink(0.08),
        hover: ink(0.07),
        selected: ink(0.12),
        chip: ink(0.07),
        track: ink(0.10),
        accent: h("#e6ba66"),
        accent_hi: h("#f5d898"),
        accent_warm: h("#fff6de"),
        accent_soft: h("#e6ba66").alpha(0.16),
        accent_focus: h("#e6ba66").alpha(0.80),
        success: h("#6bd999"),
        success_soft: h("#6bd999").alpha(0.14),
        warning: h("#fabf4d"),
        warning_soft: h("#fabf4d").alpha(0.14),
        danger: h("#f57373"),
        danger_soft: h("#f57373").alpha(0.10),
        danger_line: h("#f57373").alpha(0.45),
        info: h("#94c9ff"),
        info_soft: h("#94c9ff").alpha(0.14),
        haze: oklcha(0.5, 0.1, 270.0, 0.25),
    };
    let hairline = Some(Border::all(1.0, p.border));
    let under = Some(Border::sides([0.0, 0.0, 1.0, 0.0], p.divider));
    let skin = Skin {
        window: Surface { fill: p.bg, ink: p.text, ..Default::default() },
        sidebar: Surface { fill: p.sidebar, border: Some(Border::sides([0.0, 1.0, 0.0, 0.0], p.divider)), ..Default::default() },
        band: Surface {
            vertical: Some((p.sky, p.bg)),
            overlay: Some(Overlay::Radial { center: [0.5, 0.0], radius: [700.0, 140.0], stops: vec![(0.0, p.haze), (1.0, p.haze.alpha(0.0))] }),
            ..Default::default()
        },
        band_deco: BandDeco::Stars,
        toolbar: Surface { border: under, ..Default::default() },
        panel: Surface { fill: p.panel, border: hairline, ..Default::default() },
        detail: Surface { fill: p.sidebar, border: Some(Border::sides([0.0, 0.0, 0.0, 1.0], p.divider)), ..Default::default() },
        menu: Surface { fill: p.panel, border: under, ..Default::default() },
        list_header: Surface { border: under, ..Default::default() },
        row: Surface { border: under, hover: p.hover, ..Default::default() },
        row_selected: Surface { fill: p.selected, border: under, hover: p.hover, ..Default::default() },
        nav: Surface { hover: p.hover, ..Default::default() },
        nav_active: Surface {
            overlay: Some(Overlay::Linear { angle: std::f32::consts::FRAC_PI_2, stops: vec![(0.0, p.accent.alpha(0.12)), (0.85, p.accent.alpha(0.0))] }),
            hover: p.hover,
            ..Default::default()
        },
        primary: Surface {
            fill: p.accent,
            ink: p.on_accent,
            border: Some(Border::all(1.0, p.accent)),
            shadows: vec![inset(1.0, p.accent_warm.alpha(0.55))],
            hover: p.accent_warm.alpha(0.28),
            ..Default::default()
        },
        secondary: Surface { fill: p.panel_alt, ink: p.text, border: hairline, hover: p.hover, ..Default::default() },
        ghost: Surface { ink: p.text_dim, border: Some(Border::all(1.0, Color::TRANSPARENT)), hover: p.hover, ..Default::default() },
        danger: Surface { fill: p.danger_soft, ink: p.danger, border: Some(Border::all(1.0, p.danger_line)), hover: p.danger.alpha(0.12), ..Default::default() },
        icon_button: Surface { ink: p.text_muted, hover: p.hover, ..Default::default() },
        input: Surface { fill: p.well, border: hairline, ..Default::default() },
        chip: Surface { radius: 10.0, ..Default::default() },
        tile: Surface { fill: p.panel_alt, ink: p.text_dim, border: hairline, ..Default::default() },
        segments: Surface { border: hairline, ..Default::default() },
        segment_on: Surface { fill: p.panel_alt, ink: p.text, border: Some(Border::all(1.0, ink(0.22))), ..Default::default() },
        toast: Surface { fill: p.panel, shadows: vec![Shadow { spread: 1.0, color: p.border, ..Default::default() }], ..Default::default() },
        kbd: Surface { ink: p.text_muted, border: hairline, ..Default::default() },
        emblem: &icons::EMBLEM,
    };
    Theme {
        id: "moonlit",
        name: "Moonlit",
        blurb: "Navy night sky, one gold star",
        pal: p,
        ty: BASE_TYPE,
        skin,
        textures: &[],
    }
});
