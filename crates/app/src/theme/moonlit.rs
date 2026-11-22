//! Moonlit: a calm navy night sky with one warm gold star. Square
//! hairline boxes, Inter, a starry header band. The default theme.

use super::{BASE_TYPE, BandDeco, Border, Medal, Overlay, Pal, Skin, Surface, Theme, Tone, drop, face, h, ink, inner_ring, inset};
use rune::oklcha;
use std::sync::LazyLock;

pub static THEME: LazyLock<Theme> = LazyLock::new(|| {
    let sidebar = h("#121624");
    let panel_alt = h("#232a3f");
    let sky = h("#283254");
    let well = h("#000005").alpha(0.35);
    let on_accent = h("#0a0d14");
    let accent_warm = h("#fff6de");
    let haze = oklcha(0.5, 0.1, 270.0, 0.25);
    let p = Pal {
        bg: h("#181d2d"),
        panel: h("#1c2234"),
        text: h("#f7f8fd"),
        heading: h("#f7f8fd"),
        text_dim: h("#c3cadd"),
        text_muted: h("#9ba2b9"),
        border: ink(0.10),
        border_strong: ink(0.20),
        divider: ink(0.08),
        hover: ink(0.07),
        chip: ink(0.07),
        track: ink(0.10),
        accent: h("#e6ba66"),
        accent_hi: h("#f5d898"),
        accent_focus: h("#e6ba66").alpha(0.80),
        success: Tone { ink: h("#6bd999"), soft: h("#6bd999").alpha(0.14), line: h("#6bd999").alpha(0.4) },
        warning: Tone { ink: h("#fabf4d"), soft: h("#fabf4d").alpha(0.14), line: h("#fabf4d").alpha(0.4) },
        danger: Tone { ink: h("#f57373"), soft: h("#f57373").alpha(0.10), line: h("#f57373").alpha(0.45) },
        busy: Tone { ink: h("#e6ba66"), soft: h("#e6ba66").alpha(0.16), line: h("#e6ba66").alpha(0.4) },
    };
    let hairline = Some(Border::all(1.0, p.border));
    let panel = Surface { fill: p.panel, border: hairline, ..Default::default() };
    let skin = Skin {
        sidebar: Surface { fill: sidebar, border: Some(Border::sides([0.0, 1.0, 0.0, 0.0], p.divider)), ..Default::default() },
        band: Surface {
            vertical: Some((sky, p.bg)),
            overlay: Some(Overlay::Radial { center: [0.5, 0.0], radius: [700.0, 140.0], stops: vec![(0.0, haze), (1.0, haze.alpha(0.0))] }),
            ..Default::default()
        },
        band_deco: BandDeco::Stars,
        panel: panel.clone(),
        detail: Surface { fill: sidebar, border: Some(Border::sides([0.0, 0.0, 0.0, 1.0], p.divider)), ..Default::default() },
        // Floating (dropdowns, the installation menu): a hairline box lifted off the page.
        menu: Surface {
            fill: panel_alt,
            border: Some(Border::all(1.0, p.border_strong)),
            shadows: vec![drop(6.0, 18.0, 0.45)],
            ..Default::default()
        },
        // A tinted row in a faint gold ring.
        row_selected: Surface { fill: p.accent.alpha(0.07), border: Some(Border::sides([0.0, 0.0, 1.0, 0.0], p.divider)), shadows: vec![inner_ring(p.accent.alpha(0.32))], hover: p.hover, ..Default::default() },
        // A gold-tinted plaque inset from the sidebar's edges.
        nav_active: Surface {
            ink: p.heading,
            border: Some(Border::margin([2.0, 10.0, 2.0, 10.0])),
            radius: 2.0,
            shadows: vec![face(p.accent.alpha(0.11)), inner_ring(p.accent.alpha(0.24))],
            hover: p.hover,
            ..Default::default()
        },
        primary: Surface {
            fill: p.accent,
            ink: on_accent,
            border: Some(Border::all(1.0, p.accent)),
            shadows: vec![inset(1.0, accent_warm.alpha(0.55))],
            hover: accent_warm.alpha(0.28),
            ..Default::default()
        },
        secondary: Surface { fill: panel_alt, ink: p.text, border: hairline, hover: p.hover, ..Default::default() },
        input: Surface { fill: well, border: hairline, ..Default::default() },
        toggle: Surface { radius: 10.0, ..Default::default() },
        tile: Surface { fill: panel_alt, ink: p.text_dim, border: hairline, ..Default::default() },
        segments: Surface { border: hairline, ..Default::default() },
        toggle_on: Surface { fill: panel_alt, ink: p.text, border: Some(Border::all(1.0, ink(0.22))), ..Default::default() },
        toast: Surface { shadows: vec![drop(6.0, 18.0, 0.5)], ..panel },
        medal: Medal { ring: [h("#d4dcf0"), h("#6c7a9e"), h("#262e48")], ring_on: [h("#fff1c4"), h("#e6ba66"), h("#7a5418")], outline: h("#05080f") },
        ..Skin::base(&p, 0.0)
    };
    Theme {
        id: "moonlit",
        name: "Moonlit",
        blurb: "Navy night sky, one gold star",
        pal: p,
        ty: BASE_TYPE,
        skin,
        frame: None,
    }
});
