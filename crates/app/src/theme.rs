//! Moonlit design tokens as Rune values. Views read colours from
//! [`pal()`] and type from the consts here, never raw hex.

use rune::color::hex;
use rune::tok::{Family, TextStyle};
use rune::{Color, oklcha};
use std::sync::LazyLock;

pub struct Pal {
    // grounds, darkest to lightest
    pub sidebar: Color,
    pub footer: Color,
    pub bg: Color,
    pub panel: Color,
    pub panel_alt: Color,
    pub sky: Color,
    pub well: Color,
    pub overlay: Color,
    // text ladder
    pub text: Color,
    pub text_dim: Color,
    pub text_muted: Color,
    pub text_off: Color,
    pub on_accent: Color,
    // white ink
    pub border: Color,
    pub border_strong: Color,
    pub divider: Color,
    pub hover: Color,
    pub selected: Color,
    pub chip: Color,
    pub track: Color,
    // accent
    pub accent: Color,
    pub accent_hi: Color,
    pub accent_warm: Color,
    pub accent_soft: Color,
    pub accent_focus: Color,
    // status
    pub success: Color,
    pub success_soft: Color,
    pub warning: Color,
    pub warning_soft: Color,
    pub danger: Color,
    pub danger_soft: Color,
    pub danger_line: Color,
    pub info: Color,
    pub info_soft: Color,
    pub haze: Color,
}

fn h(s: &str) -> Color {
    hex(s).expect("valid token hex")
}

fn ink(a: f32) -> Color {
    Color::WHITE.alpha(a)
}

static PAL: LazyLock<Pal> = LazyLock::new(|| Pal {
    sidebar: h("#121624"),
    footer: h("#141927"),
    bg: h("#181d2d"),
    panel: h("#1c2234"),
    panel_alt: h("#232a3f"),
    sky: h("#283254"),
    well: h("#000005").alpha(0.35),
    overlay: h("#05080f").alpha(0.72),
    text: h("#f7f8fd"),
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
});

pub fn pal() -> &'static Pal {
    &PAL
}

// Type ladder. Tracking is in points: em × size.
pub const DISPLAY: TextStyle = TextStyle::new(22.0, 600.0).tracking(-0.5).lh(28.0);
pub const HERO: TextStyle = TextStyle::new(26.0, 600.0).tracking(-0.65).lh(30.0);
pub const TITLE: TextStyle = TextStyle::new(18.0, 600.0).tracking(-0.27).lh(24.0);
pub const HEADING: TextStyle = TextStyle::new(15.0, 600.0).tracking(-0.15).lh(20.0);
pub const BODY: TextStyle = TextStyle::new(13.0, 500.0).lh(18.0);
pub const STRONG: TextStyle = TextStyle::new(13.5, 600.0).lh(18.0);
pub const LABEL: TextStyle = TextStyle::new(13.0, 500.0).lh(16.0);
pub const NOTE: TextStyle = TextStyle::new(12.5, 400.0).lh(19.0);
pub const SMALL: TextStyle = TextStyle::new(12.0, 500.0).lh(16.0);
pub const NUMBER: TextStyle = TextStyle::new(12.0, 600.0).features("'tnum' 1").lh(16.0);
pub const CAPTION: TextStyle = TextStyle::new(11.0, 600.0).tracking(0.22).lh(14.0);
pub const EYEBROW: TextStyle = TextStyle::new(10.5, 600.0).tracking(1.26).lh(14.0);
pub const MONO: TextStyle = TextStyle::new(12.0, 400.0).family(Family::Mono).lh(16.0);
pub const WORDMARK: TextStyle =
    TextStyle::new(15.0, 500.0).family(Family::Named("Cinzel, Cormorant SC, Noto Serif, serif")).tracking(2.7).lh(20.0);

/// `AppSpec::families`: Inter (embedded by Rune) and a mono for versions and paths.
pub const FAMILIES: (&str, &str) = ("Inter", "JetBrains Mono, ui-monospace");

// Layout metrics.
pub const SIDEBAR_W: f32 = 240.0;
pub const BAND_H: f32 = 112.0;
pub const PAD: f32 = 32.0;
pub const ROW_H: f32 = 56.0;
pub const CONTROL_H: f32 = 32.0;

/// Small tweaks of a ladder style at the call site.
pub trait StyleExt {
    fn size_of(self, size: f32) -> Self;
    fn weight_of(self, weight: f32) -> Self;
}

impl StyleExt for TextStyle {
    fn size_of(self, size: f32) -> Self {
        TextStyle { size, line_height: (size * 1.3).round(), ..self }
    }
    fn weight_of(self, weight: f32) -> Self {
        TextStyle { weight, ..self }
    }
}
