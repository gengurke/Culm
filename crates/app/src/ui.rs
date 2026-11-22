//! Small components painted from the current theme's skin (`theme::t()`). Built on Rune elements
//! directly because rune-kit's look (rounded, neutral) is not ours.

use crate::icons;
use crate::state::State;
use crate::theme::{self, StyleExt, Surface, SurfaceExt, Tone, pal, t};
use addon_core::Channel;
use rune::{
    Color, Cursor, El, HelpLook, Icon, ScrollSpec, Shadow, TextEdit, TextStyle, Tx, col, el, icon,
    row, text,
};
use rune_kit::{Kit, PaletteSeed, ScrollBar, ScrollBarStyle, TextInput};

/// Opacity of a control that can't run now (a locked card is a different state: 0.6).
pub const DISABLED: f32 = 0.45;

/// The looks every control shares, whatever the theme.
pub trait Themed {
    /// The keyboard focus ring: a thin `accent_focus` line just outside the box.
    fn focus_look(self) -> El;
    /// A state (focused field, open dropdown) drawn as a 1 pt `color` ring just inside the box's
    /// border, so the theme's border and bevel stay.
    fn ring(self, color: Color) -> El;
    /// A chosen card (the selected addon, the theme in use): a 2 pt `accent` ring inside its frame.
    fn selected(self) -> El;
}

impl Themed for El {
    fn focus_look(self) -> El {
        self.focus_ring(1.0, 2.0, pal().accent_focus)
    }
    fn ring(self, color: Color) -> El {
        self.box_shadow(Shadow {
            spread: 1.0,
            color,
            inset: true,
            ..Shadow::default()
        })
    }
    fn selected(self) -> El {
        self.box_shadow(Shadow {
            spread: 2.0,
            color: pal().accent,
            inset: true,
            ..Shadow::default()
        })
    }
}

/// The wash while `sk` is pressed: its own, or a stronger hover.
fn press_of(sk: &Surface) -> Color {
    if sk.press.a > 0.0 {
        sk.press
    } else {
        sk.hover.alpha(sk.hover.a * 1.6)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

/// A 32 pt button (callers make it smaller with `.h(..)`). Disabled buttons drop the handler and dim
/// to 45%.
pub fn button(
    key: impl Into<String>,
    label: &str,
    kind: Kind,
    glyph: Option<&'static Icon>,
    enabled: bool,
    f: impl Fn(&mut State, &mut Tx) + 'static,
) -> El {
    let th = t();
    let sk = match kind {
        Kind::Primary => &th.skin.primary,
        Kind::Secondary => &th.skin.secondary,
        Kind::Ghost => &th.skin.ghost,
        Kind::Danger => &th.skin.danger,
    };
    let (ink, hover, press) = (sk.ink, sk.hover, press_of(sk));
    let font = if kind == Kind::Primary {
        th.ty.label.weight_of(600.0)
    } else {
        th.ty.label
    };
    // The hover wash goes on below with the press state, and only when enabled.
    let mut b = row()
        .key(key)
        .role("button")
        .label(label)
        .focusable(enabled)
        .h(theme::CONTROL_H)
        .px(14.0)
        .gap(8.0)
        .center()
        .skin(&Surface {
            hover: Color::TRANSPARENT,
            press: Color::TRANSPARENT,
            ..sk.clone()
        })
        .focus_look();
    if let Some(g) = glyph {
        b = b.child(icon(theme::glyph(g), 14.0).color(ink));
    }
    b = b.child(text(label).font(font).color(ink));
    if enabled {
        b.hover(hover)
            .press(press)
            .cursor(Cursor::Pointer)
            .on_press_tx(f)
    } else {
        b.disabled(true).opacity(DISABLED)
    }
}

/// A square 28 pt glyph-only button.
pub fn icon_button(
    key: impl Into<String>,
    label: &str,
    glyph: &'static Icon,
    f: impl Fn(&mut State, &mut Tx) + 'static,
) -> El {
    let sk = &t().skin.icon_button;
    el().key(key)
        .role("button")
        .label(label)
        .help(label)
        .focusable(true)
        .size(28.0, 28.0)
        .center()
        .skin(sk)
        .press(press_of(sk))
        .cursor(Cursor::Pointer)
        .on_press_tx(f)
        .child(icon(theme::glyph(glyph), 16.0).color(sk.ink))
}

/// What a status tag means; its colour and glyph come from the theme.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mark {
    /// Up to date, installed, connected.
    Good,
    /// An update waits.
    Update,
    /// Something runs, or is the one in use.
    Busy,
    Bad,
    /// Nothing known yet.
    Muted,
    /// Deliberately left alone (ignored, git checkout).
    Off,
}

impl Mark {
    /// Its colours from the palette's status tones (the quiet marks: muted text on the chip ground).
    pub fn tone(self) -> Tone {
        let p = pal();
        let quiet = |ink: Color| Tone {
            ink,
            soft: p.chip,
            line: p.border,
        };
        match self {
            Mark::Good => p.success,
            Mark::Update => p.warning,
            Mark::Busy => p.busy,
            Mark::Bad => p.danger,
            Mark::Muted => quiet(p.text_muted),
            Mark::Off => quiet(p.text_dim),
        }
    }

    fn glyph(self) -> &'static Icon {
        match self {
            Mark::Good => &icons::CHECK,
            Mark::Update => &icons::ARROW_UP,
            Mark::Busy => &icons::CLOCK,
            Mark::Bad => &icons::ALERT,
            Mark::Muted => &icons::HELP,
            Mark::Off => &icons::EYE_OFF,
        }
    }

    /// Needs the user's eye: the label is coloured, not just the glyph.
    fn loud(self) -> bool {
        matches!(self, Mark::Update | Mark::Bad | Mark::Busy)
    }
}

/// A status tag: a glyph and a word, no box; only marks that need attention colour their word.
pub fn tag(label: &str, mark: Mark) -> El {
    let th = t();
    let p = &th.pal;
    let fg = mark.tone().ink;
    let quiet = if mark == Mark::Good { fg } else { p.text_muted };
    row()
        .h(20.0)
        .gap(7.0)
        .center_y()
        .child(icon(mark.glyph(), 14.0).color(if mark.loud() { fg } else { quiet }))
        .child(
            text(label)
                .font(if mark.loud() {
                    th.ty.small.weight_of(600.0)
                } else {
                    th.ty.small
                })
                .color(if mark.loud() { fg } else { p.text_muted }),
        )
}

/// The slim progress line: gold fill over a hairline track.
pub fn progress_line(done: f32, height: f32) -> El {
    let p = pal();
    row()
        .w_full()
        .h(height)
        .fill(p.track)
        .child(el().h_full().w_frac(done.clamp(0.0, 1.0)).fill(p.accent))
}

/// A text field in a `well`: the input skin with an `accent_focus` ring inside it on focus, optional leading glyph and
/// trailing accessory. `edit` must be the field `access` returns.
pub fn field(
    key: &str,
    edit: &TextEdit,
    placeholder: &str,
    glyph: Option<&'static Icon>,
    access: impl for<'a> Fn(&'a mut State) -> &'a mut TextEdit + 'static,
) -> FieldBuilder {
    FieldBuilder {
        key: key.into(),
        input: TextInput::new(edit)
            .plain()
            .width(None)
            .placeholder(placeholder)
            .label(placeholder)
            .on_edit(access),
        glyph,
        focused: edit.focused,
        trailing: None,
        mono: false,
    }
}

pub struct FieldBuilder {
    key: String,
    input: TextInput,
    glyph: Option<&'static Icon>,
    focused: bool,
    trailing: Option<El>,
    mono: bool,
}

impl FieldBuilder {
    pub fn on_change(mut self, f: impl Fn(&mut State, &mut Tx) + 'static) -> Self {
        self.input = self.input.on_change(f);
        self
    }
    pub fn on_submit(mut self, f: impl Fn(&mut State, &mut Tx) + 'static) -> Self {
        self.input = self.input.on_submit(f);
        self
    }
    pub fn secure(mut self, revealed: bool) -> Self {
        self.input = self.input.secure().revealed(revealed);
        self
    }
    pub fn trailing(mut self, e: El) -> Self {
        self.trailing = Some(e);
        self
    }
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }
    pub fn disabled(mut self, d: bool) -> Self {
        self.input = self.input.disabled(d);
        self
    }
    pub fn build(self) -> El {
        let (th, p) = (t(), pal());
        let k = kit();
        let style = if self.mono {
            th.ty.mono.size_of(13.0)
        } else {
            th.ty.body.weight_of(400.0)
        };
        let mut input = self.input.text_style(style).height(theme::CONTROL_H - 2.0);
        if let Some(g) = self.glyph {
            input = input.leading_icon(g).leading_icon_size(14.0);
        }
        let mut f = row()
            .key(self.key)
            .h(theme::CONTROL_H + 4.0)
            .pr(6.0)
            .gap(6.0)
            .center_y()
            .skin(&th.skin.input);
        if self.focused {
            f = f.ring(p.accent_focus);
        }
        f = f.child(input.build(&k).grow(1.0).min_w(0.0));
        if let Some(t) = self.trailing {
            f = f.child(t);
        }
        f
    }
}

/// Tooltips (`El::help`) in the theme's floating look (its `menu` surface: dropdowns, the
/// installation menu), after a short rest of the pointer.
pub fn help_look() -> HelpLook {
    let (th, p) = (t(), pal());
    let sk = &th.skin.menu;
    // A tooltip has one flat fill: the middle of the menu's gradient where it has one.
    let fill = match sk.vertical {
        Some((top, bottom)) => top.mix(bottom, 0.5),
        None => sk.fill,
    };
    let edge = sk.border.map_or(p.border_strong, |b| b.top);
    HelpLook::system(true)
        .fill(fill.alpha(1.0))
        .ink(p.text)
        .ring(1.0, edge)
        .radius(sk.radius.min(6.0))
        .padding(9.0, 5.0)
        .font(th.ty.small.weight_of(400.0))
        .max_width(Some(280.0))
        .keycaps(p.chip, p.text_muted, th.ty.caption, false)
        .delay(0.6)
        .skip_delay(0.4)
        .fade(0.12, 0.08, 0.96)
        .shadows(sk.shadows.iter().filter(|s| !s.inset).copied())
}

/// rune-kit, tinted for the few kit parts we use (text fields).
pub fn kit() -> Kit {
    let p = pal();
    let mut k = Kit::from_palette(PaletteSeed {
        bg: p.bg,
        fg: p.text,
        accent: p.accent,
    });
    k.c.text_faint = p.text_muted;
    k.c.icon_muted = p.text_muted;
    k.c.text_accent = p.accent;
    k.c.border_focus = p.accent_focus;
    let r = t().skin.input.radius;
    (k.radius.xs, k.radius.sm, k.radius.md, k.radius.lg) = (r, r, r, r);
    k
}

/// `viewport` scrolling down, with a thin overlay scroll bar: hidden at rest, shown while the
/// pointer is over the area or it scrolls, a 3 pt pill in `border_strong` that widens to 6 pt in
/// the theme's `accent` under the pointer or while dragged. Every scroll area goes through here.
#[track_caller]
pub fn scroll(viewport: El) -> El {
    let p = pal();
    let look = ScrollBarStyle::scroll_view(&kit())
        .thumb(p.border_strong)
        .thumb_hover(p.accent)
        .width(3.0)
        .hover_width(6.0);
    viewport
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .scroll_bar_styled(look)
}

/// A glyph in a small round badge: the mark's soft wash in a ring of its line.
pub fn badge(glyph: &'static Icon, mark: Mark) -> El {
    let tone = mark.tone();
    el().size(28.0, 28.0)
        .radius(14.0)
        .center()
        .fill(tone.soft)
        .ring(tone.line)
        .child(icon(glyph, 14.0).color(tone.ink))
}

/// Paints `e` as a notice (a warning, an error, a confirmation of a deletion): the mark's soft wash
/// edged in its line, with the panel's corners and 12 pt padding. The caller lays out the content.
pub fn notice(e: El, mark: Mark) -> El {
    let tone = mark.tone();
    e.w_full()
        .padding(12.0)
        .radius(t().skin.panel.radius)
        .fill(tone.soft)
        .border(1.0, tone.line)
}

/// A key hint box (`/`).
pub fn kbd(label: &str) -> El {
    let sk = &t().skin.kbd;
    el().size(20.0, 20.0)
        .center()
        .skin(sk)
        .child(text(label).font(t().ty.caption).color(sk.ink))
}

/// Each release channel, its name and what it means: the channel pickers' options and help.
pub const CHANNELS: [(Channel, &str, &str); 3] = [
    (Channel::Stable, "Stable", "Finished releases only"),
    (
        Channel::Beta,
        "Beta",
        "Releases and betas, which may still have bugs",
    ),
    (Channel::Alpha, "Alpha", "Every build, even untested alphas"),
];

/// A channel's name ("Stable").
pub fn channel_name(c: Channel) -> &'static str {
    CHANNELS
        .iter()
        .find(|(ch, ..)| *ch == c)
        .map_or("", |o| o.1)
}

/// `1 addon`, `3 addons`.
pub fn plural(n: usize, noun: &str) -> String {
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}

/// A spinner beside a short status line ("Installing…"), spinning in `spin`.
pub fn busy(label: impl Into<String>, spin: Color) -> El {
    row()
        .gap(8.0)
        .center_y()
        .child(spinner(12.0, spin))
        .child(text(label.into()).font(t().ty.small).color(pal().text_dim))
}

/// Nothing to list: what is missing and a hint what to do. Callers place it (`py`, `center`).
pub fn empty(key: &str, title: impl Into<String>, hint: &str) -> El {
    let p = pal();
    col()
        .key(key)
        .gap(6.0)
        .child(text(title.into()).font(t().ty.body).color(p.text_dim))
        .child(
            text(hint.to_string())
                .font(t().ty.small)
                .color(p.text_muted),
        )
}

/// An error: the alert glyph and the wrapping message in the danger ink, set in `font`.
pub fn error_line(e: impl Into<String>, font: TextStyle) -> El {
    let p = pal();
    row()
        .gap(8.0)
        .child(icon(&icons::ALERT, 14.0).color(p.danger.ink))
        .child(
            text(e.into())
                .font(font)
                .color(p.danger.ink)
                .wrap_anywhere()
                .grow(1.0)
                .min_w(0.0),
        )
}

/// The panel at the right of My Addons and Get Addons: the detail skin, 340 pt wide, `body` scrolling.
#[track_caller]
pub fn detail_panel(key: impl Into<String>, body: El) -> El {
    col()
        .key(key)
        .w(340.0)
        .h_full()
        .min_h(0.0)
        .skin(&t().skin.detail)
        .child(scroll(col().grow(1.0).min_h(0.0).w_full().child(body)))
}

/// A detail panel's head: the addon's tile, name, author and an optional badge, and a close button.
pub fn detail_header(
    tile: El,
    name: &str,
    author: &str,
    badge: Option<El>,
    close_key: String,
    close: impl Fn(&mut State, &mut Tx) + 'static,
) -> El {
    let p = pal();
    let mut head = col().grow(1.0).min_w(0.0).gap(4.0).child(
        text(name.to_string())
            .font(t().ty.title)
            .color(p.heading)
            .wrap(),
    );
    head = head.child(
        text(author.to_string())
            .font(t().ty.small)
            .color(p.text_muted)
            .truncate(),
    );
    if let Some(b) = badge {
        head = head.child(row().pt(6.0).child(b));
    }
    row()
        .w_full()
        .gap(14.0)
        .items_start()
        .child(tile)
        .child(head)
        .child(icon_button(
            close_key,
            "Close details",
            &icons::CLOSE,
            close,
        ))
}

/// One line of a detail panel's facts: a muted label and its value (`mono` for versions and folders).
pub fn fact(label: &str, value: String, mono: bool) -> El {
    let p = pal();
    row()
        .w_full()
        .py(7.0)
        .gap(12.0)
        .border_sides([0.0, 0.0, 1.0, 0.0], p.divider)
        .child(
            text(label.to_string())
                .font(t().ty.small)
                .color(p.text_muted)
                .w(96.0),
        )
        .child(
            text(value)
                .font(if mono { t().ty.mono } else { t().ty.small })
                .color(p.text_dim)
                .grow(1.0)
                .min_w(0.0)
                .wrap(),
        )
}

/// A segmented control: options (value, label, help; "" for none) in a hairline box, the
/// active one raised.
pub fn segmented<T: Copy + PartialEq + 'static>(
    key: &str,
    options: &[(T, &str, &str)],
    current: T,
    enabled: bool,
    f: impl Fn(&mut State, T, &mut Tx) + Clone + 'static,
) -> El {
    let (th, p) = (t(), pal());
    let mut seg = row()
        .key(key)
        .role("radiogroup")
        .h(theme::CONTROL_H)
        .skin(&th.skin.segments)
        .padding(2.0)
        .gap(2.0)
        .focus_group();
    for &(value, label, help) in options {
        let on = value == current;
        let mut item = row()
            .key(format!("{key}/{}", slug(label)))
            .role("radio")
            .label(label)
            .help(help)
            .checked(on)
            .focusable(enabled)
            .h_full()
            .px(12.0)
            .center();
        if on {
            item = item.skin(&th.skin.toggle_on);
        }
        item = item.child(text(label).font(th.ty.label).color(if on {
            th.skin.toggle_on.ink
        } else {
            p.text_muted
        }));
        if enabled {
            let f = f.clone();
            item = item
                .hover(p.hover)
                .cursor(Cursor::Pointer)
                .on_press_tx(move |s: &mut State, tx: &mut Tx| f(s, value, tx));
        }
        seg = seg.child(item);
    }
    if enabled { seg } else { seg.opacity(DISABLED) }
}

pub fn slug(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// A card: the skin's panel, 20 pt padding.
pub fn card(key: impl Into<String>) -> El {
    col().key(key).skin(&t().skin.panel).padding(20.0).gap(12.0)
}

/// A square well holding a glyph (installations, maintenance cards).
pub fn glyph_tile(glyph: &'static Icon, size: f32) -> El {
    let sk = &t().skin.tile;
    el().size(size, size)
        .center()
        .skin(sk)
        .child(icon(theme::glyph(glyph), size / 2.0).color(sk.ink))
}

/// Thumbnail fallback: the addon's initials on a raised square.
pub fn initials_tile(name: &str, size: f32) -> El {
    let th = t();
    let letters: String = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .take(2)
        .filter_map(|w| w.chars().next())
        .collect::<String>()
        .to_uppercase();
    el().size(size, size).center().skin(&th.skin.tile).child(
        text(letters)
            .font(th.ty.caption.size_of(size * 0.36))
            .color(th.skin.tile.ink),
    )
}

/// The addon's thumbnail over its initials tile: the initials show while the image decodes or
/// if it fails (a pending or failed image paints nothing). `key` names the image layer.
pub fn addon_tile(name: &str, size: f32, thumb: Option<&rune::Image>, key: String) -> El {
    let tile = initials_tile(name, size);
    match thumb {
        Some(img) => tile.child(
            el().key(key)
                .role("img")
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .pixels_fit(img.pixels(), rune::SurfaceFit::Fill),
        ),
        None => tile,
    }
}

/// The theme's emblem with its soft glow, for empty states.
pub fn emblem(size: f32) -> El {
    glowing(t().skin.emblem, size)
}

/// The brand mark with the same glow, for the sidebar header.
pub fn brand_mark(size: f32) -> El {
    glowing(&icons::EMBLEM, size)
}

fn glowing(glyph: &'static Icon, size: f32) -> El {
    let p = pal();
    el().size(size * 2.0, size * 2.0)
        .center()
        .radial_gradient(
            [0.5, 0.5],
            size,
            [(0.0, p.accent.alpha(0.35)), (1.0, p.accent.alpha(0.0))],
        )
        .child(icon(glyph, size).color(p.accent))
}

/// How far a class icon is enlarged behind its disc: just enough to crop the 2 px dark frame
/// baked into Blizzard's 56 px JPGs, so the whole picture shows inside the ring.
const ICON_ZOOM: f32 = 56.0 / 52.0;

/// A class medallion (classicplus.io-style): a bevelled metal ring in the theme's
/// [`Medal`](theme::Medal) tones, gilded when `on`, around a sunk disc showing the class's original
/// icon (`icon`, downloaded at runtime: it is Blizzard's art, never bundled) cut round, over our own
/// fallback: the disc in the class colour (darkened, lit from the centre like a faint glow) bearing
/// the class emblem in light ink, which shows while the icon is missing or decoding. None for a
/// slug that isn't one of Forever's classes.
pub fn class_medal(slug: &str, size: f32, on: bool, icon: Option<&rune::Image>) -> Option<El> {
    let glyph = icons::class_icon(slug)?;
    let class = theme::h(icons::class_color(slug)?);
    let m = t().skin.medal;
    let [hi, mid, lo] = if on { m.ring_on } else { m.ring };
    let ring = (size * 0.09).max(1.5);
    let d = size - 2.0 * ring;
    let (deep, core, glow) = (
        class.mix(Color::BLACK, 0.93),
        class.mix(Color::BLACK, 0.7),
        // Light classes (Priest, Rogue) are held down so the light emblem still reads.
        class.mix(
            Color::BLACK,
            (1.0 - 0.22 / class.luminance().max(0.01)).max(0.25),
        ),
    );
    // A soft specular sheen on the disc's upper left, under the emblem.
    let sheen = el()
        .pin(Some(d * 0.14), Some(d * 0.07), None, None)
        .size(d * 0.56, d * 0.34)
        .radius(d * 0.2)
        .radial_gradient(
            [0.5, 0.5],
            [d * 0.28, d * 0.17],
            [
                (0.0, Color::WHITE.alpha(0.30)),
                (1.0, Color::WHITE.alpha(0.0)),
            ],
        )
        .a11y_hidden();
    let mut disc = el()
        .pin(Some(ring), Some(ring), None, None)
        .size(d, d)
        .radius(d / 2.0)
        .center()
        .fill(deep)
        .radial_gradient(
            [0.5, 0.48],
            d * 0.6,
            [(0.0, glow), (0.45, core), (1.0, deep)],
        )
        .clip_rounded(d / 2.0)
        .child(sheen)
        .child(self::icon(glyph, d * 0.8).color(Color::WHITE.mix(class, 0.12)));
    if let Some(img) = icon {
        // Painted once decoded, over the glyph (AGENTS.md: `.pixels` over a fallback). Only the
        // baked frame falls outside the disc; the picture itself stays whole inside the ring.
        let z = d * ICON_ZOOM;
        disc = disc.child(
            el().key(format!("class-icon/{slug}"))
                .pin(Some((d - z) / 2.0), Some((d - z) / 2.0), None, None)
                .size(z, z)
                .pixels(img.pixels()),
        );
    }
    // The disc sits sunk in the ring: a dark lip on the ring's inner edge, around the disc rather
    // than over it, so nothing covers the picture.
    let lip = el()
        .pin(Some(ring - 0.75), Some(ring - 0.75), None, None)
        .size(d + 1.5, d + 1.5)
        .radius(d / 2.0 + 0.75)
        .border_gradient(0.75, Color::BLACK.alpha(0.75), Color::BLACK.alpha(0.35));
    let mut medal = el()
        .size(size, size)
        .radius(size / 2.0)
        .fill(mid)
        // Light from the top left: highlight, body, shade.
        .linear_gradient(
            std::f32::consts::FRAC_PI_4 * 3.0,
            [(0.0, hi), (0.45, mid), (1.0, lo)],
        )
        .border(0.75, m.outline)
        .box_shadow(Shadow {
            y: 1.0,
            blur: 2.0,
            color: Color::BLACK.alpha(0.45),
            ..Shadow::default()
        })
        .a11y_hidden();
    if on {
        medal = medal.box_shadow(Shadow {
            blur: 6.0,
            color: m.ring_on[1].alpha(0.55),
            ..Shadow::default()
        });
    }
    Some(medal.child(disc).child(lip))
}

/// A muted line of meta text pieces, set apart by thin vertical hairlines ([`meta_line`]).
/// Empty pieces are left out.
pub fn meta(parts: &[&str]) -> El {
    let p = pal();
    meta_line(
        parts
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| text(s.to_string()).font(t().ty.small).color(p.text_muted))
            .collect(),
    )
}

/// Pieces on one line with a hairline between each two. When space runs short the first piece
/// truncates (an author's long name) and the rest keep their width.
pub fn meta_line(parts: Vec<El>) -> El {
    let mut line = row().min_w(0.0).gap(9.0).center_y();
    for (i, part) in parts.into_iter().enumerate() {
        line = if i == 0 {
            line.child(part.truncate().shrink())
        } else {
            line.child(hairline(12.0)).child(part)
        };
    }
    line
}

/// A thin vertical divider between pieces of one line.
pub fn hairline(h: f32) -> El {
    el().w(1.0)
        .h(h)
        .fill(pal().text_muted.alpha(0.4))
        .a11y_hidden()
}

/// A rotating loader glyph.
pub fn spinner(size: f32, color: Color) -> El {
    icon(&icons::LOADER, size)
        .color(color)
        .looping(rune::LoopSpec::rotation(0.9))
}

/// `1.2 MB`-style sizes.
pub fn bytes(n: u64) -> String {
    match n {
        0..1_000 => format!("{n} B"),
        1_000..1_000_000 => format!("{:.0} KB", n as f64 / 1e3),
        1_000_000..1_000_000_000 => format!("{:.1} MB", n as f64 / 1e6),
        _ => format!("{:.1} GB", n as f64 / 1e9),
    }
}

/// `48.2M`-style download counts.
pub fn count(n: u64) -> String {
    match n {
        0..1_000 => n.to_string(),
        1_000..1_000_000 => format!("{:.1}K", n as f64 / 1e3),
        _ => format!("{:.1}M", n as f64 / 1e6),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn formats() {
        assert_eq!(super::count(48_211_903), "48.2M");
        assert_eq!(super::bytes(18_404_311), "18.4 MB");
        assert_eq!(super::slug("What's Training?"), "whatstraining");
    }
}
