//! Small Moonlit components: square, hairline-edged, one gold accent. Built on Rune elements
//! directly because rune-kit's look (rounded, neutral) is not this design.

use crate::icons;
use crate::state::State;
use crate::theme::{self, StyleExt, pal};
use rune::{Color, Cursor, El, Icon, Shadow, TextEdit, Tx, col, el, icon, row, text};
use rune_kit::{Kit, PaletteSeed, TextInput};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

/// A 32 pt button (`small`: 28). Disabled buttons drop the handler and dim to 45%.
pub fn button(
    key: impl Into<String>,
    label: &str,
    kind: Kind,
    glyph: Option<&'static Icon>,
    enabled: bool,
    f: impl Fn(&mut State, &mut Tx) + 'static,
) -> El {
    let p = pal();
    let (fill, ink, edge, hover) = match kind {
        Kind::Primary => (p.accent, p.on_accent, p.accent, p.accent_warm.alpha(0.28)),
        Kind::Secondary => (p.panel_alt, p.text, p.border, p.hover),
        Kind::Ghost => (Color::TRANSPARENT, p.text_dim, Color::TRANSPARENT, p.hover),
        Kind::Danger => (p.danger_soft, p.danger, p.danger_line, p.danger.alpha(0.12)),
    };
    let font = if kind == Kind::Primary { theme::LABEL.weight_of(600.0) } else { theme::LABEL };
    let mut b = row()
        .key(key)
        .role("button")
        .label(label)
        .focusable(enabled)
        .h(theme::CONTROL_H)
        .px(14.0)
        .gap(8.0)
        .center()
        .fill(fill)
        .border(1.0, edge)
        .focus_ring(1.0, 2.0, p.accent_focus);
    if kind == Kind::Primary {
        b = b.box_shadow(Shadow { y: 1.0, color: p.accent_warm.alpha(0.55), inset: true, ..Shadow::default() });
    }
    if let Some(g) = glyph {
        b = b.child(icon(g, 14.0).color(ink));
    }
    b = b.child(text(label).font(font).color(ink));
    if enabled {
        b.hover(hover).press(hover.alpha(hover.a * 1.6)).cursor(Cursor::Pointer).on_press_tx(f)
    } else {
        b.disabled(true).opacity(0.45)
    }
}

/// A square 28 pt glyph-only button.
pub fn icon_button(key: impl Into<String>, label: &str, glyph: &'static Icon, f: impl Fn(&mut State, &mut Tx) + 'static) -> El {
    let p = pal();
    el().key(key)
        .role("button")
        .label(label)
        .help(label)
        .focusable(true)
        .size(28.0, 28.0)
        .center()
        .hover(p.hover)
        .press(p.selected)
        .cursor(Cursor::Pointer)
        .on_press_tx(f)
        .child(icon(glyph, 16.0).color(p.text_muted))
}

/// A pill chip (status badge): `caption` text on a soft fill, optional leading dot.
pub fn badge(label: &str, fg: Color, bg: Color, dot: bool) -> El {
    let mut b = row().h(20.0).px(9.0).gap(6.0).center().radius(10.0).fill(bg);
    if dot {
        b = b.child(el().size(6.0, 6.0).radius(3.0).fill(fg).shadow(0.0, 6.0, fg.alpha(0.6)));
    }
    b.child(text(label).font(theme::CAPTION).color(fg))
}

/// The slim progress line: gold fill over a hairline track.
pub fn progress_line(done: f32, height: f32) -> El {
    let p = pal();
    row().w_full().h(height).fill(p.track).child(el().h_full().w_frac(done.clamp(0.0, 1.0)).fill(p.accent))
}

/// A text field in a `well`: hairline edge that turns gold on focus, optional leading glyph and
/// trailing accessory. `edit` must be the field `access` returns.
pub fn field(
    key: &str,
    edit: &TextEdit,
    placeholder: &str,
    glyph: Option<&'static Icon>,
    access: impl for<'a> Fn(&'a mut State) -> &'a mut TextEdit + 'static,
) -> FieldBuilder {
    FieldBuilder { key: key.into(), input: TextInput::new(edit).plain().width(None).placeholder(placeholder).label(placeholder).on_edit(access), glyph, focused: edit.focused, trailing: None, mono: false }
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
        let p = pal();
        let k = kit();
        let style = if self.mono { theme::MONO.size_of(13.0) } else { theme::BODY.weight_of(400.0) };
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
            .fill(p.well)
            .border(1.0, if self.focused { p.accent_focus } else { p.border })
            .child(input.build(&k).grow(1.0).min_w(0.0));
        if let Some(t) = self.trailing {
            f = f.child(t);
        }
        f
    }
}

/// rune-kit, tinted for the few kit parts we use (text fields).
pub fn kit() -> Kit {
    let p = pal();
    let mut k = Kit::from_palette(PaletteSeed { bg: p.bg, fg: p.text, accent: p.accent });
    k.c.text_faint = p.text_muted;
    k.c.icon_muted = p.text_muted;
    k.c.text_accent = p.accent;
    k.c.border_focus = p.accent_focus;
    k.radius.xs = 0.0;
    k.radius.sm = 0.0;
    k.radius.md = 0.0;
    k.radius.lg = 0.0;
    k
}

/// A 1 pt inset hairline that leaves `border` free for a coloured side bar.
pub fn edge(e: El, c: Color) -> El {
    e.box_shadow(Shadow { spread: 1.0, color: c, inset: true, ..Shadow::default() })
}

/// A key hint box (`/`).
pub fn kbd(label: &str) -> El {
    let p = pal();
    el().size(20.0, 20.0).center().border(1.0, p.border).child(text(label).font(theme::CAPTION).color(p.text_muted))
}

/// A segmented control: options in a hairline box, the active one raised.
pub fn segmented<T: Copy + PartialEq + 'static>(
    key: &str,
    options: &[(T, &str)],
    current: T,
    enabled: bool,
    f: impl Fn(&mut State, T, &mut Tx) + Clone + 'static,
) -> El {
    let p = pal();
    let mut seg = row().key(key).role("radiogroup").h(theme::CONTROL_H).border(1.0, p.border).padding(2.0).gap(2.0).focus_group();
    for &(value, label) in options {
        let on = value == current;
        let mut item = row()
            .key(format!("{key}/{}", slug(label)))
            .role("radio")
            .label(label)
            .checked(on)
            .focusable(enabled)
            .h_full()
            .px(12.0)
            .center()
            .child(text(label).font(theme::LABEL).color(if on { p.text } else { p.text_muted }));
        if on {
            item = item.fill(p.panel_alt).border(1.0, Color::WHITE.alpha(0.22));
        }
        if enabled {
            let f = f.clone();
            item = item.hover(p.hover).cursor(Cursor::Pointer).on_press_tx(move |s: &mut State, tx: &mut Tx| f(s, value, tx));
        }
        seg = seg.child(item);
    }
    if enabled { seg } else { seg.opacity(0.45) }
}

pub fn slug(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase()
}

/// A card: panel ground, hairline edge, 20 pt padding.
pub fn card(key: impl Into<String>) -> El {
    let p = pal();
    col().key(key).fill(p.panel).border(1.0, p.border).padding(20.0).gap(12.0)
}

/// Thumbnail fallback: the addon's initials on a raised square.
pub fn initials_tile(name: &str, size: f32) -> El {
    let p = pal();
    let letters: String = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .take(2)
        .filter_map(|w| w.chars().next())
        .collect::<String>()
        .to_uppercase();
    el().size(size, size)
        .center()
        .fill(p.panel_alt)
        .border(1.0, p.border)
        .child(text(letters).font(theme::CAPTION.size_of(size * 0.36)).color(p.text_dim))
}

/// The gold star with its soft glow, for empty states and the brand block.
pub fn emblem(size: f32) -> El {
    let p = pal();
    el().size(size * 2.0, size * 2.0)
        .center()
        .radial_gradient([0.5, 0.5], size, [(0.0, p.accent.alpha(0.35)), (1.0, p.accent.alpha(0.0))])
        .child(icon(&icons::EMBLEM, size).color(p.accent))
}

/// A muted line of meta text pieces joined by middle dots.
pub fn meta(parts: &[&str]) -> El {
    let p = pal();
    text(parts.iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join("  ·  ")).font(theme::SMALL).color(p.text_muted).truncate()
}

/// A rotating loader glyph.
pub fn spinner(size: f32, color: Color) -> El {
    icon(&icons::LOADER, size).color(color).looping(rune::LoopSpec::rotation(0.9))
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
