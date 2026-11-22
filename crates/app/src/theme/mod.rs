//! Switchable themes. A [`Theme`] is data: a palette ([`Pal`]), a type ladder ([`Type`]) and a
//! [`Skin`] of named [`Surface`]s. Views read the current one through [`t()`] (or [`pal()`]) and
//! paint boxes with [`SurfaceExt::skin`], never with raw colours. Layout metrics are global.
//!
//! Adding a theme: write `theme/<name>.rs` with a `pub static THEME: LazyLock<Theme>` and add one
//! `Variant => module` line to the `themes!` list below. `docs/THEMES.md` explains every slot.

mod deco;
mod glyphs;
mod texture;

use crate::state::{Screen, State};
pub use deco::band_deco;
use rune::color::hex;
use rune::tok::{Family, TextStyle};
use rune::{ChromeInsets, Color, El, Icon, Image, Shadow, el, icon};
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

/// One look: what a `theme/<name>.rs` file defines (see `docs/THEMES.md`).
pub struct Theme {
    /// The stable id kept in prefs and scene names (`settings@<id>`); never rename a shipped one.
    pub id: &'static str,
    /// The name on the picker card.
    pub name: &'static str,
    /// One line for the picker.
    pub blurb: &'static str,
    /// The colours views read.
    pub pal: Pal,
    /// The text style per role.
    pub ty: Type,
    /// The named surfaces components paint, plus the emblem and glyph swaps.
    pub skin: Skin,
    /// A second look for the window frame (sidebar and header band), for themes whose pages
    /// are light but whose frame is dark. Views draw the frame inside [`framed`].
    pub frame: Option<&'static LazyLock<Theme>>,
}

impl Theme {
    /// Textures its skin and its frame's use (generated once when the theme is first shown).
    pub fn textures(&self) -> Vec<Tex> {
        let mut out = Vec::new();
        let frame = self.frame.map(|f| &f.skin);
        for s in std::iter::once(&self.skin).chain(frame) {
            for (tex, _) in s.surfaces().iter().filter_map(|s| s.texture) {
                if !out.contains(&tex) {
                    out.push(tex);
                }
            }
        }
        out
    }

    /// The look of the window frame (sidebar, header band): [`Theme::frame`] or the theme itself.
    pub fn frame_look(&'static self) -> &'static Theme {
        self.frame.map_or(self, |f| &**f)
    }
}

/// The colours views read directly (everything else a theme paints goes through its [`Skin`]).
/// Theme-only tones (sidebar wood, band sky, field wells, …) are local `let`s in the theme file.
pub struct Pal {
    /// The window ground; the contrast test's base and the kit's (dropdown, slider) background.
    pub bg: Color,
    /// Card ground: the contrast test checks text, accent and status ink on it, so make it
    /// `skin.panel`'s (average) fill.
    pub panel: Color,
    /// Body text and the window caption.
    pub text: Color,
    /// Page titles and section headings.
    pub heading: Color,
    /// Secondary text: descriptions, inactive tabs, quiet badges.
    pub text_dim: Color,
    /// Hints, counts, timestamps, keyboard caps and icon buttons.
    pub text_muted: Color,
    /// The edge of quiet badges (`Mark::Muted`/`Off`) and the stock key-cap outline.
    pub border: Color,
    /// Scroll thumbs and the edge of a tooltip without its own border.
    pub border_strong: Color,
    /// Hairlines between list rows and sections.
    pub divider: Color,
    /// Hover wash on clickable rows, chips and cards.
    pub hover: Color,
    /// Quiet badge ground and keyboard caps.
    pub chip: Color,
    /// Progress bar track.
    pub track: Color,
    /// Links, active controls, progress, info toasts, the chosen card's ring and the scroll thumb
    /// under the pointer.
    pub accent: Color,
    /// The lit side of accent: band embers.
    pub accent_hi: Color,
    /// Keyboard focus rings, the focused field's and the open dropdown's ring.
    pub accent_focus: Color,
    /// Up to date, installed, done.
    pub success: Tone,
    /// Update available, out of date, game running; the sidebar's update count.
    pub warning: Tone,
    /// Errors, failures, and confirmations of what deletes something.
    pub danger: Tone,
    /// Something runs or is the one in use.
    pub busy: Tone,
}

/// A status colour: `ink` for its text and glyphs, `soft` for the ground of its badges and notices,
/// `line` for their edge (read through `ui::Mark`).
#[derive(Clone, Copy)]
pub struct Tone {
    /// Text and glyphs in this status; must read at 3:1 on `bg` and `panel`.
    pub ink: Color,
    /// The wash behind a badge or notice (usually `ink` at 0.10–0.17).
    pub soft: Color,
    /// The 1 pt edge of a badge or notice (usually `ink` at 0.4–0.5).
    pub line: Color,
}

/// Text style per role. Tracking is in points: em × size.
#[derive(Clone, Copy)]
pub struct Type {
    /// The first-run welcome's title.
    pub display: TextStyle,
    /// The page title in the header band (and on the picker's miniature band).
    pub hero: TextStyle,
    /// The addon's name in the detail panel.
    pub title: TextStyle,
    /// Section headings, card and toast titles.
    pub heading: TextStyle,
    /// Running text: descriptions, field text, menu entries.
    pub body: TextStyle,
    /// The addon's name in a My Addons row.
    pub strong: TextStyle,
    /// Control labels: buttons, segments, dropdowns.
    pub label: TextStyle,
    /// Explanations and hints: the band's meta line, notes under sections, empty states.
    pub note: TextStyle,
    /// Secondary lines: row details, sidebar sub-lines, tooltips (at weight 400).
    pub small: TextStyle,
    /// Counts and status numbers, with tabular figures: the sidebar's update count, status cells.
    pub number: TextStyle,
    /// The smallest text: key caps and initials tiles.
    pub caption: TextStyle,
    /// Spaced capitals over a group: "CLASS", the sidebar's group names, detail sections.
    pub eyebrow: TextStyle,
    /// Versions, paths and ids. Keep `Family::Mono` (JetBrains Mono).
    pub mono: TextStyle,
}

/// The Moonlit ladder; other themes start from it (`Type { hero: .., ..BASE_TYPE }`).
pub const BASE_TYPE: Type = Type {
    display: TextStyle::new(22.0, 600.0).tracking(-0.5).lh(28.0),
    hero: TextStyle::new(20.0, 600.0).tracking(-0.4).lh(26.0),
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
};

/// Every face the app draws text in (`AppSpec::fonts`), bundled so text looks the same on every
/// OS: Inter, JetBrains Mono and each family a theme names with `Family::Named` (one name, no
/// fallbacks). OFL fonts from google/fonts, licences beside them in `assets/fonts/<family>/`.
pub const FONTS: &[&[u8]] = &[
    include_bytes!("../../assets/fonts/inter/Inter[opsz,wght].ttf"),
    include_bytes!("../../assets/fonts/jetbrainsmono/JetBrainsMono[wght].ttf"),
    include_bytes!("../../assets/fonts/cinzel/Cinzel[wght].ttf"),
    include_bytes!("../../assets/fonts/marcellus/Marcellus-Regular.ttf"),
    include_bytes!("../../assets/fonts/ebgaramond/EBGaramond[wght].ttf"),
];

/// `AppSpec::families`: the sans and mono slots, both in [`FONTS`].
pub const FAMILIES: (&str, &str) = ("Inter", "JetBrains Mono");

// Layout metrics, the same in every theme.
pub const SIDEBAR_W: f32 = 240.0;
pub const BAND_H: f32 = 64.0;
pub const PAD: f32 = 32.0;
pub const ROW_H: f32 = 56.0;
pub const CONTROL_H: f32 = 32.0;

/// The named surfaces views and components paint with.
#[derive(Clone)]
pub struct Skin {
    /// The window ground behind every page (`ink`: the default text colour).
    pub window: Surface,
    /// The left column: brand, navigation, installation picker.
    pub sidebar: Surface,
    /// The page header band.
    pub band: Surface,
    /// What draws over the band behind its title (`theme/deco.rs`).
    pub band_deco: BandDeco,
    /// The strip under the band (filter, search).
    pub toolbar: Surface,
    /// Cards: addon cards, theme picker cards, maintenance and settings cards. Notices take its
    /// radius; Moonlit-style hairline cards need only fill and border.
    pub panel: Surface,
    /// The addon detail panel at the right.
    pub detail: Surface,
    /// Floating popups: the installation menu, Get Addons' dropdown list. Tooltips take its fill,
    /// border colour, radius (up to 6) and outer shadows. Give it a full border and a drop shadow.
    pub menu: Surface,
    /// The column header row of My Addons.
    pub list_header: Surface,
    /// A My Addons row (border: its separator; hover).
    pub row: Surface,
    /// The My Addons row whose detail is open.
    pub row_selected: Surface,
    /// Sidebar rows (the label in `ink`; the glyph takes `accent` when active, else `text_muted`).
    pub nav: Surface,
    /// The sidebar row of the open page (often an inset plaque: `Border::margin` + `face`).
    pub nav_active: Surface,
    /// The main action (Install, Update all); `ink` must read at 4.5:1 on `fill`.
    pub primary: Surface,
    /// Other buttons and the dropdown button; `ink` must read at 4.5:1 on `fill`.
    pub secondary: Surface,
    /// Quiet buttons with no frame until hovered.
    pub ghost: Surface,
    /// Buttons that delete something (Remove, Clean).
    pub danger: Surface,
    /// Glyph-only buttons (close the detail, show the key, toast close); the glyph takes `ink`.
    pub icon_button: Surface,
    /// Text fields (search, API key); its radius also rounds the kit's dropdowns and sliders.
    pub input: Surface,
    /// An unselected toggle (Get Addons' class chips, the source rows); also the shape of the
    /// sidebar's update count. Its `ink` is unused: unselected text is `Pal::text_dim`.
    pub toggle: Surface,
    /// Square wells: initials tiles, glyph tiles, toast item slots; the glyph takes `ink`.
    pub tile: Surface,
    /// The track of a segmented control; unselected segments are bare (`hover`, `text_muted`).
    pub segments: Surface,
    /// A selected toggle: the chosen segment, class chip or source row.
    pub toggle_on: Surface,
    /// Toasts (framed alerts); their corner glyph, if any, also crowns the top edge as a gem.
    pub toast: Surface,
    /// Key hint boxes (`/`); the key in `ink`.
    pub kbd: Surface,
    /// Round class medallions (Get Addons' class chips).
    pub medal: Medal,
    /// The emblem on the first-run welcome (`ui::emblem`); the sidebar's brand mark stays stock.
    pub emblem: &'static Icon,
    /// Glyphs this theme draws differently: `(default, own)`, looked up by [`glyph`].
    pub glyphs: &'static [(&'static Icon, &'static Icon)],
}

impl Skin {
    /// Every surface, so nothing that reads them all (textures) misses a slot added later.
    fn surfaces(&self) -> [&Surface; 24] {
        let Skin {
            window,
            sidebar,
            band,
            band_deco: _,
            toolbar,
            panel,
            detail,
            menu,
            list_header,
            row,
            row_selected,
            nav,
            nav_active,
            primary,
            secondary,
            ghost,
            danger,
            icon_button,
            input,
            toggle,
            tile,
            segments,
            toggle_on,
            toast,
            kbd,
            medal: _,
            emblem: _,
            glyphs: _,
        } = self;
        [
            window,
            sidebar,
            band,
            toolbar,
            panel,
            detail,
            menu,
            list_header,
            row,
            row_selected,
            nav,
            nav_active,
            primary,
            secondary,
            ghost,
            danger,
            icon_button,
            input,
            toggle,
            tile,
            segments,
            toggle_on,
            toast,
            kbd,
        ]
    }

    /// The slots most themes share, drawn from the palette, with `radius` on the small controls:
    /// a plain window, underlined toolbar, list header and rows, hover-only nav, ghost and icon
    /// buttons, a soft danger button, toggles and hairline key caps. Every other slot is
    /// blank (a transparent box), and the emblem is the stock one: themes set those and override
    /// what they make their own (`Skin { panel: .., ..Skin::base(&p, 3.0) }`).
    pub fn base(p: &Pal, radius: f32) -> Skin {
        let under = Some(Border::sides([0.0, 0.0, 1.0, 0.0], p.divider));
        let blank = Surface::default;
        Skin {
            window: Surface {
                fill: p.bg,
                ink: p.text,
                ..blank()
            },
            sidebar: blank(),
            band: blank(),
            band_deco: BandDeco::None,
            toolbar: Surface {
                border: under,
                ..blank()
            },
            panel: blank(),
            detail: blank(),
            menu: blank(),
            list_header: Surface {
                border: under,
                ..blank()
            },
            row: Surface {
                border: under,
                hover: p.hover,
                ..blank()
            },
            row_selected: blank(),
            nav: Surface {
                ink: p.text_dim,
                hover: p.hover,
                ..blank()
            },
            nav_active: Surface {
                ink: p.heading,
                ..blank()
            },
            primary: blank(),
            secondary: blank(),
            ghost: Surface {
                ink: p.text_dim,
                border: Some(Border::all(1.0, Color::TRANSPARENT)),
                radius,
                hover: p.hover,
                ..blank()
            },
            danger: Surface {
                fill: p.danger.soft,
                ink: p.danger.ink,
                border: Some(Border::all(1.0, p.danger.line)),
                radius,
                hover: p.danger.ink.alpha(0.12),
                ..blank()
            },
            icon_button: Surface {
                ink: p.text_muted,
                radius,
                hover: p.hover,
                ..blank()
            },
            input: blank(),
            toggle: Surface { radius, ..blank() },
            tile: blank(),
            segments: blank(),
            toggle_on: blank(),
            toast: blank(),
            kbd: Surface {
                ink: p.text_muted,
                border: Some(Border::all(1.0, p.border)),
                radius,
                ..blank()
            },
            medal: Medal::default(),
            emblem: &crate::icons::EMBLEM,
            glyphs: &[],
        }
    }
}

/// A round metal medallion's ring: `[highlight, body, shade]` along the light from the top left,
/// gilded (`ring_on`) on the active chip, and the dark outline that sets it off the ground.
#[derive(Clone, Copy, Default)]
pub struct Medal {
    /// The ring's highlight, body and shade on an unselected chip.
    pub ring: [Color; 3],
    /// The same on the selected chip (its middle tone also glows round it).
    pub ring_on: [Color; 3],
    /// The thin dark line round the ring.
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
    /// The ground; also the fallback while a texture loads and what the contrast test reads.
    pub fill: Color,
    /// Text and glyph colour on it, where the component reads it (buttons, nav, tiles, kbd).
    pub ink: Color,
    /// Top to bottom, over the fill.
    pub vertical: Option<(Color, Color)>,
    /// A second gradient over `vertical`: a glow, a sheen.
    pub overlay: Option<Overlay>,
    /// A texture over fill and gradient (inside a uniform border) at this opacity. Big surfaces
    /// only (window, sidebar, band, detail): every textured box costs GPU time.
    pub texture: Option<(Tex, f32)>,
    /// Hairline, bevel (`Border::bevel`), single sides or a clear margin (`Border::margin`).
    pub border: Option<Border>,
    /// Corner radius in points.
    pub radius: f32,
    /// Painted in order: inset ones are highlights and inner rings (`inset`, `inner_ring`,
    /// `face`), the others outer rings and drops (`outer_ring`, `drop`).
    pub shadows: Vec<Shadow>,
    /// Hover wash (transparent: none).
    pub hover: Color,
    /// Wash while pressed (transparent: a stronger hover).
    pub press: Color,
    /// A glyph pinned into each corner (studs, rivets).
    pub corners: Option<Corners>,
}

/// A second gradient over the fill (Rune keeps one linear or radial per box).
#[derive(Clone)]
pub enum Overlay {
    /// Along `angle` in radians (CSS: 0 bottom to top, `PI / 2` left to right, `PI` top to bottom),
    /// stops at 0–1.
    Linear {
        angle: f32,
        stops: Vec<(f32, Color)>,
    },
    /// An ellipse at `center` (fractions of the box) with `radius` in points, stops at 0–1.
    Radial {
        center: [f32; 2],
        radius: [f32; 2],
        stops: Vec<(f32, Color)>,
    },
}

/// Border widths `[top, right, bottom, left]`; with uniform widths, `top` ≠ `bottom` draws a bevel.
#[derive(Clone, Copy)]
pub struct Border {
    /// Widths `[top, right, bottom, left]`.
    pub sides: [f32; 4],
    /// The colour (a bevel's lit edge).
    pub top: Color,
    /// A bevel's shaded edge; the same as `top` for a plain border.
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
    /// A clear border: a margin round a plaque whose face a `face` shadow paints (a fill
    /// would run under the border).
    pub fn margin(sides: [f32; 4]) -> Border {
        Border::sides(sides, Color::TRANSPARENT)
    }
}

/// A glyph pinned into each corner (rivets, studs).
#[derive(Clone, Copy)]
pub struct Corners {
    /// The ornament (`theme/glyphs.rs`).
    pub glyph: &'static Icon,
    /// Its size in points.
    pub size: f32,
    /// Its distance from the two edges.
    pub inset: f32,
    /// The glyph's colour.
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
    static CHROME: Cell<ChromeInsets> = Cell::new(ChromeInsets::default());
    static TEXTURES: RefCell<HashMap<Tex, Image>> = RefCell::new(HashMap::new());
}

/// The theme the view on this thread is drawing with (its frame look inside [`framed`]).
pub fn t() -> &'static Theme {
    let th = CURRENT.with(Cell::get).theme();
    if FRAMED.with(Cell::get) {
        th.frame_look()
    } else {
        th
    }
}

/// What the window's own controls take of the titlebar this run (`Cx::chrome_insets`): the header
/// band and the sidebar's brand block keep clear of them.
pub fn chrome() -> ChromeInsets {
    CHROME.with(Cell::get)
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

/// Starts a view: makes the stored theme and the window's `chrome` insets current and makes sure the textures it shows exist
/// (every theme's on Settings, for the picker). Without an image cache surfaces keep their fill.
pub fn begin(s: &State, chrome: ChromeInsets) {
    CURRENT.with(|c| c.set(s.settings.theme));
    CHROME.with(|c| c.set(chrome));
    let Some(cache) = s.images.as_ref() else {
        return;
    };
    let shown: &[ThemeId] = if s.screen == Screen::Settings {
        ThemeId::ALL
    } else {
        std::slice::from_ref(&s.settings.theme)
    };
    for tex in shown.iter().flat_map(|id| id.theme().textures()) {
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

/// A black drop shadow outside the box, `y` points down.
pub(crate) fn drop(y: f32, blur: f32, a: f32) -> Shadow {
    Shadow {
        y,
        blur,
        color: Color::BLACK.alpha(a),
        ..Shadow::default()
    }
}

/// A 1 pt line just inside the box's edge.
pub(crate) fn inner_ring(color: Color) -> Shadow {
    Shadow {
        spread: 1.0,
        color,
        inset: true,
        ..Shadow::default()
    }
}

/// A 1 pt line just outside the box's edge.
pub(crate) fn outer_ring(color: Color) -> Shadow {
    Shadow {
        spread: 1.0,
        color,
        ..Shadow::default()
    }
}

/// Paints the whole inside of a box's border (its padding box), unlike a fill: the face of a
/// plaque inset by a [`Border::margin`].
pub(crate) fn face(color: Color) -> Shadow {
    Shadow {
        spread: 40.0,
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
                        ("success", p.success.ink, 3.0),
                        ("warning", p.warning.ink, 3.0),
                        ("danger", p.danger.ink, 3.0),
                        ("busy", p.busy.ink, 3.0),
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
