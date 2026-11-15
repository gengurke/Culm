//! Switchable themes. A [`Theme`] is data: a palette ([`Pal`]), a type ladder ([`Type`]) and a
//! [`Skin`] of named [`Surface`]s. Views read the current one through [`t()`] (or [`pal()`]) and
//! paint boxes with [`SurfaceExt::skin`], never with raw colours. Layout metrics are global.
//!
//! Adding a theme: write `theme/<name>.rs` with a `pub static THEME: LazyLock<Theme>` and add one
//! `Variant => module` line to the `themes!` list below.

mod texture;

use crate::state::{Screen, State};
use rune::color::hex;
use rune::tok::{Family, TextStyle};
use rune::{Color, El, Icon, Image, Shadow, el, icon};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Once, OnceLock};
pub use texture::Tex;

macro_rules! themes {
    ($($variant:ident => $module:ident),+ $(,)?) => {
        $(mod $module;)+

        /// Which theme is on; stored in the settings by [`ThemeId::id`].
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum ThemeId { $($variant),+ }

        impl ThemeId {
            /// Every theme, in picker order. The first is the default.
            pub const ALL: &[ThemeId] = &[$(ThemeId::$variant),+];

            pub fn theme(self) -> &'static Theme {
                match self { $(ThemeId::$variant => &$module::THEME),+ }
            }
        }
    };
}

themes! {
    Moonlit => moonlit,
    Hearthwood => hearthwood,
    Emberglow => emberglow,
    Ledger => ledger,
    Grimoire => grimoire,
}

impl Default for ThemeId {
    fn default() -> ThemeId {
        ThemeId::ALL[0]
    }
}

impl ThemeId {
    /// The stable id kept in prefs and used in keys (`settings/theme/<id>`).
    pub fn id(self) -> &'static str {
        self.theme().id
    }

    /// The theme stored as `id`; an unknown or retired id falls back to the default.
    pub fn from_id(id: &str) -> ThemeId {
        ThemeId::ALL
            .iter()
            .copied()
            .find(|t| t.id() == id)
            .unwrap_or_default()
    }
}

pub struct Theme {
    pub id: &'static str,
    pub name: &'static str,
    /// One line for the picker.
    pub blurb: &'static str,
    pub pal: Pal,
    pub ty: Type,
    pub skin: Skin,
    /// Textures its skin uses (generated once when the theme is first shown), its frame's too.
    pub textures: &'static [Tex],
    /// A second look for the window frame (sidebar and header band), for themes whose pages
    /// are light but whose frame is dark. Views draw the frame inside [`framed`].
    pub frame: Option<&'static LazyLock<Theme>>,
}

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
    /// Page titles and section headings.
    pub heading: Color,
    pub text_dim: Color,
    pub text_muted: Color,
    pub text_off: Color,
    pub on_accent: Color,
    // ink
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

/// Text style per role. Tracking is in points: em × size.
#[derive(Clone, Copy)]
pub struct Type {
    pub display: TextStyle,
    pub hero: TextStyle,
    pub title: TextStyle,
    pub heading: TextStyle,
    pub body: TextStyle,
    pub strong: TextStyle,
    pub label: TextStyle,
    pub note: TextStyle,
    pub small: TextStyle,
    pub number: TextStyle,
    pub caption: TextStyle,
    pub eyebrow: TextStyle,
    pub mono: TextStyle,
    pub wordmark: TextStyle,
}

/// The Moonlit ladder; other themes start from it (`Type { hero: .., ..BASE_TYPE }`).
pub const BASE_TYPE: Type = Type {
    display: TextStyle::new(22.0, 600.0).tracking(-0.5).lh(28.0),
    hero: TextStyle::new(26.0, 600.0).tracking(-0.65).lh(30.0),
    title: TextStyle::new(18.0, 600.0).tracking(-0.27).lh(24.0),
    heading: TextStyle::new(15.0, 600.0).tracking(-0.15).lh(20.0),
    body: TextStyle::new(13.0, 500.0).lh(18.0),
    strong: TextStyle::new(13.5, 600.0).lh(18.0),
    label: TextStyle::new(13.0, 500.0).lh(16.0),
    note: TextStyle::new(12.5, 400.0).lh(19.0),
    small: TextStyle::new(12.0, 500.0).lh(16.0),
    number: TextStyle::new(12.0, 600.0).features("'tnum' 1").lh(16.0),
    caption: TextStyle::new(11.0, 600.0).tracking(0.22).lh(14.0),
    eyebrow: TextStyle::new(10.5, 600.0).tracking(1.26).lh(14.0),
    mono: TextStyle::new(12.0, 400.0).family(Family::Mono).lh(16.0),
    wordmark: TextStyle::new(15.0, 500.0)
        .family(Family::Named("Cinzel, Cormorant SC, Noto Serif, serif"))
        .tracking(2.7)
        .lh(20.0),
};

/// `AppSpec::families`: Inter (embedded by Rune) and a mono for versions and paths.
pub const FAMILIES: (&str, &str) = ("Inter", "JetBrains Mono, ui-monospace");

// Layout metrics, the same in every theme.
pub const SIDEBAR_W: f32 = 240.0;
pub const BAND_H: f32 = 112.0;
pub const PAD: f32 = 32.0;
pub const ROW_H: f32 = 56.0;
pub const CONTROL_H: f32 = 32.0;

/// The named surfaces views and components paint with.
#[derive(Clone)]
pub struct Skin {
    /// The window ground behind every page.
    pub window: Surface,
    pub sidebar: Surface,
    /// The page header band and what decorates it.
    pub band: Surface,
    pub band_deco: BandDeco,
    /// The strip under the band (filter, search).
    pub toolbar: Surface,
    /// Cards and notices.
    pub panel: Surface,
    /// The addon detail panel at the right.
    pub detail: Surface,
    /// The installation menu above the picker.
    pub menu: Surface,
    pub list_header: Surface,
    pub row: Surface,
    pub row_selected: Surface,
    pub nav: Surface,
    pub nav_active: Surface,
    pub primary: Surface,
    pub secondary: Surface,
    pub ghost: Surface,
    pub danger: Surface,
    pub icon_button: Surface,
    pub input: Surface,
    /// Status badges: their shape (the fill and ink come from the status).
    pub chip: Surface,
    /// Square wells: initials tiles, icon boxes.
    pub tile: Surface,
    pub segments: Surface,
    pub segment_on: Surface,
    /// Painted after the kit built the toast: a texture or corners would cover its content.
    pub toast: Surface,
    pub kbd: Surface,
    /// Round class medallions (Get Addons' class chips).
    pub medal: Medal,
    /// The brand mark beside the wordmark and on empty states.
    pub emblem: &'static Icon,
    /// Glyphs this theme draws differently: `(default, own)`, looked up by [`glyph`].
    pub glyphs: &'static [(&'static Icon, &'static Icon)],
}

/// A round metal medallion's ring: `[highlight, body, shade]` along the light from the top left,
/// gilded (`ring_on`) on the active chip, and the dark outline that sets it off the ground.
#[derive(Clone, Copy)]
pub struct Medal {
    pub ring: [Color; 3],
    pub ring_on: [Color; 3],
    pub outline: Color,
}

/// What draws over the band behind its title.
#[derive(Clone, Copy, PartialEq)]
pub enum BandDeco {
    None,
    /// A fixed, sparse sky of white dots.
    Stars,
    /// Warm sparks drifting up from the bottom edge.
    Embers,
}

/// A box's look as data, painted by [`SurfaceExt::skin`]: fill, optional gradient and texture,
/// border, radius, shadows (inset ones are highlights), hover wash and corner ornaments.
#[derive(Clone, Default)]
pub struct Surface {
    pub fill: Color,
    /// Text and glyph colour on it.
    pub ink: Color,
    /// Top to bottom, over the fill.
    pub vertical: Option<(Color, Color)>,
    pub overlay: Option<Overlay>,
    /// A texture over fill and gradient (inside a uniform border) at this opacity.
    pub texture: Option<(Tex, f32)>,
    pub border: Option<Border>,
    pub radius: f32,
    pub shadows: Vec<Shadow>,
    /// Hover wash (transparent: none).
    pub hover: Color,
    /// Wash while pressed (transparent: a stronger hover).
    pub press: Color,
    pub corners: Option<Corners>,
}

/// A second gradient over the fill (Rune keeps one linear or radial per box).
#[derive(Clone)]
pub enum Overlay {
    Linear {
        angle: f32,
        stops: Vec<(f32, Color)>,
    },
    Radial {
        center: [f32; 2],
        radius: [f32; 2],
        stops: Vec<(f32, Color)>,
    },
}

/// Border widths `[top, right, bottom, left]`; with uniform widths, `top` ≠ `bottom` draws a bevel.
#[derive(Clone, Copy)]
pub struct Border {
    pub sides: [f32; 4],
    pub top: Color,
    pub bottom: Color,
}

impl Border {
    pub fn all(w: f32, c: Color) -> Border {
        Border {
            sides: [w; 4],
            top: c,
            bottom: c,
        }
    }
    pub fn bevel(w: f32, top: Color, bottom: Color) -> Border {
        Border {
            sides: [w; 4],
            top,
            bottom,
        }
    }
    pub fn sides(sides: [f32; 4], c: Color) -> Border {
        Border {
            sides,
            top: c,
            bottom: c,
        }
    }
}

/// A glyph pinned into each corner (rivets, studs).
#[derive(Clone, Copy)]
pub struct Corners {
    pub glyph: &'static Icon,
    pub size: f32,
    pub inset: f32,
    pub color: Color,
}

/// The largest box a texture covers, in points.
const COVER: (f32, f32) = (2048.0, 1536.0);

pub trait SurfaceExt {
    /// Paints `s` on this box. Call it before adding children: texture and ornaments are
    /// pinned children that must sit under the content.
    fn skin(self, s: &Surface) -> El;
}

impl SurfaceExt for El {
    fn skin(self, s: &Surface) -> El {
        let mut e = self.fill(s.fill).radius(s.radius);
        if let Some((top, bottom)) = s.vertical {
            e = e.gradient(top, bottom);
        }
        match &s.overlay {
            Some(Overlay::Linear { angle, stops }) => e = e.linear_gradient(*angle, stops),
            Some(Overlay::Radial {
                center,
                radius,
                stops,
            }) => e = e.radial_gradient(*center, *radius, stops),
            None => {}
        }
        let mut inner = 0.0;
        if let Some(b) = s.border {
            let uniform = b.sides.iter().all(|&w| w == b.sides[0]);
            e = match (uniform, b.top == b.bottom) {
                (true, true) => e.border(b.sides[0], b.top),
                (true, false) => e.border_gradient(b.sides[0], b.top, b.bottom),
                (false, _) => e.border_sides(b.sides, b.top),
            };
            if uniform {
                inner = b.sides[0];
            }
        }
        for sh in &s.shadows {
            e = e.box_shadow(*sh);
        }
        if s.hover.a > 0.0 {
            e = e.hover(s.hover);
        }
        if s.press.a > 0.0 {
            e = e.press(s.press);
        }
        if let Some((tex, a)) = s.texture
            && let Some(img) = texture(tex)
        {
            // Tiles over the fill, which stays the fallback: a pending image paints nothing (AGENTS.md).
            // shortcut: covers boxes up to COVER points; past that the plain fill shows.
            let at = Some(inner);
            let r = (s.radius - inner).max(0.0);
            let (w, h) = tex.size();
            let (w, h) = (w as f32, h as f32);
            let mut layer = el()
                .pin(at, at, at, at)
                .radius(r)
                .clip_rounded(r)
                .opacity(a)
                .a11y_hidden();
            for y in 0..(COVER.1 / h).ceil() as usize {
                for x in 0..(COVER.0 / w).ceil() as usize {
                    layer = layer.child(
                        el().absolute(x as f32 * w, y as f32 * h)
                            .size(w, h)
                            .pixels(img.pixels()),
                    );
                }
            }
            e = e.child(layer);
        }
        if let Some(c) = s.corners {
            let (i, far) = (Some(c.inset), None);
            for (l, t, r, b) in [
                (i, i, far, far),
                (far, i, i, far),
                (far, far, i, i),
                (i, far, far, i),
            ] {
                e = e.child(
                    el().pin(l, t, r, b)
                        .size(c.size, c.size)
                        .a11y_hidden()
                        .child(icon(c.glyph, c.size).color(c.color)),
                );
            }
        }
        e
    }
}

thread_local! {
    // Per thread, not per process: tests run many windows on parallel threads.
    static CURRENT: Cell<ThemeId> = Cell::new(ThemeId::default());
    static FRAMED: Cell<bool> = const { Cell::new(false) };
    static TEXTURES: RefCell<HashMap<Tex, Image>> = RefCell::new(HashMap::new());
}

/// The theme the view on this thread is drawing with (its frame look inside [`framed`]).
pub fn t() -> &'static Theme {
    let th = CURRENT.with(Cell::get).theme();
    match th.frame {
        Some(frame) if FRAMED.with(Cell::get) => frame,
        _ => th,
    }
}

pub fn pal() -> &'static Pal {
    &t().pal
}

/// Builds the window frame (sidebar, header band): inside, [`t`] and [`pal`] give the theme's
/// frame look when it has one.
pub fn framed<R>(f: impl FnOnce() -> R) -> R {
    let was = FRAMED.with(|c| c.replace(true));
    let out = f();
    FRAMED.with(|c| c.set(was));
    out
}

/// The current theme's version of a stock glyph (`&icons::PACKAGE`), or the glyph itself.
pub fn glyph(g: &'static Icon) -> &'static Icon {
    t().skin
        .glyphs
        .iter()
        .find(|(stock, _)| std::ptr::eq(*stock, g))
        .map_or(g, |(_, own)| own)
}

/// WCAG contrast ratio of `fg` over `bg` (both composited over `under` when translucent).
pub fn contrast(fg: Color, bg: Color, under: Color) -> f32 {
    let over = |c: Color, u: Color| u.mix(c.alpha(1.0), c.a);
    let bg = over(bg, under);
    let (a, b) = (over(fg, bg).luminance(), bg.luminance());
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Starts a view: makes the stored theme current and makes sure the textures it shows exist
/// (every theme's on Settings, for the picker). Without an image cache surfaces keep their fill.
pub fn begin(s: &State) {
    CURRENT.with(|c| c.set(s.settings.theme));
    let Some(cache) = s.images.as_ref() else {
        return;
    };
    let shown: &[ThemeId] = if s.screen == Screen::Settings {
        ThemeId::ALL
    } else {
        std::slice::from_ref(&s.settings.theme)
    };
    for &tex in shown.iter().flat_map(|id| id.theme().textures) {
        TEXTURES.with(|m| {
            let mut m = m.borrow_mut();
            if m.contains_key(&tex) {
                return;
            }
            // After `prewarm` the bytes come from its threads (until then the fill shows);
            // without it (tests, headless runs) they are made here, so snapshots have them.
            let slot = &RGBA[tex as usize];
            let bytes = match slot.get() {
                Some(b) => b,
                None if PREWARM.is_completed() => return,
                None => slot.get_or_init(|| {
                    let (w, h, bytes) = tex.rgba();
                    (w, h, bytes.into())
                }),
            };
            m.insert(tex, cache.rgba(bytes.0, bytes.1, bytes.2.clone()));
        });
    }
}

/// Width, height and RGBA8 bytes of a generated texture.
type Rgba = (u32, u32, Arc<[u8]>);

/// Texture bytes per [`Tex`] (by discriminant), made once per process.
static RGBA: [OnceLock<Rgba>; Tex::ALL.len()] = [const { OnceLock::new() }; Tex::ALL.len()];
static PREWARM: Once = Once::new();
/// Set when a prewarm thread finished; [`textures_landed`] takes it.
static LANDED: AtomicBool = AtomicBool::new(false);

/// Generates every texture on background threads (Wood ~40 ms in release, Settings needs all
/// of them), so neither the first frame nor the first visit to Settings waits on the UI thread.
/// Each one wakes the window; `poll` asks [`textures_landed`] whether to draw again.
pub fn prewarm(waker: &rune::Waker) {
    PREWARM.call_once(|| {
        for tex in Tex::ALL {
            let waker = waker.clone();
            std::thread::spawn(move || {
                let (w, h, bytes) = tex.rgba();
                if RGBA[tex as usize].set((w, h, bytes.into())).is_ok() {
                    LANDED.store(true, Ordering::Release);
                    waker();
                }
            });
        }
    });
}

/// Whether a prewarmed texture arrived since the last call (the view should run again).
pub fn textures_landed() -> bool {
    LANDED.swap(false, Ordering::AcqRel)
}

fn texture(tex: Tex) -> Option<Image> {
    TEXTURES.with(|m| m.borrow().get(&tex).cloned())
}

// Helpers for the theme files.

pub(crate) fn h(s: &str) -> Color {
    hex(s).expect("valid token hex")
}

pub(crate) fn ink(a: f32) -> Color {
    Color::WHITE.alpha(a)
}

/// A 1 pt highlight (`y > 0`: along the top) or shade (`y < 0`: along the bottom) inside the box.
pub(crate) fn inset(y: f32, color: Color) -> Shadow {
    Shadow {
        y,
        color,
        inset: true,
        ..Shadow::default()
    }
}

/// Small tweaks of a ladder style at the call site.
pub trait StyleExt {
    fn size_of(self, size: f32) -> Self;
    fn weight_of(self, weight: f32) -> Self;
}

impl StyleExt for TextStyle {
    fn size_of(self, size: f32) -> Self {
        TextStyle {
            size,
            line_height: (size * 1.3).round(),
            ..self
        }
    }
    fn weight_of(self, weight: f32) -> Self {
        TextStyle { weight, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::{ThemeId, contrast};

    /// Text reads at 4.5:1 on the window and on cards; dimmed text (muted, status) at 3:1.
    #[test]
    fn f7_3_every_theme_keeps_text_readable() {
        let mut bad = Vec::new();
        for id in ThemeId::ALL {
            let th = id.theme();
            for (part, th) in std::iter::once(("page", th)).chain(th.frame.map(|f| ("frame", &**f)))
            {
                let p = &th.pal;
                for (ground, bg) in [("bg", p.bg), ("panel", p.panel)] {
                    let checks = [
                        ("text", p.text, 4.5),
                        ("heading", p.heading, 4.5),
                        ("text_dim", p.text_dim, 4.5),
                        ("text_muted", p.text_muted, 3.0),
                        ("accent", p.accent, 3.0),
                        ("success", p.success, 3.0),
                        ("warning", p.warning, 3.0),
                        ("danger", p.danger, 3.0),
                        ("info", p.info, 3.0),
                    ];
                    for (name, fg, min) in checks {
                        let c = contrast(fg, bg, p.bg);
                        if c < min {
                            bad.push(format!(
                                "{} {part}: {name} on {ground} {c:.2} < {min}",
                                th.id
                            ));
                        }
                    }
                }
                for (name, sk) in [
                    ("primary", &th.skin.primary),
                    ("secondary", &th.skin.secondary),
                ] {
                    let c = contrast(sk.ink, sk.fill, p.bg);
                    if c < 4.5 {
                        bad.push(format!("{} {part}: {name} ink {c:.2} < 4.5", th.id));
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    #[test]
    fn f7_3_unknown_theme_ids_fall_back_to_moonlit() {
        assert_eq!(ThemeId::from_id("hearthwood"), ThemeId::Hearthwood);
        assert_eq!(ThemeId::from_id("no-such-theme"), ThemeId::Moonlit);
        for id in ThemeId::ALL {
            assert_eq!(ThemeId::from_id(id.id()), *id);
        }
    }
}
