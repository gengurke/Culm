//! Toasts: the stack of notices at the bottom right.
//!
//! shortcut: `ADDONS_TOAST_STYLE=0..5` picks one of several looks while the user compares them
//! (`--scene toasts@<theme>`); delete the losers and the variable once one is chosen.

use crate::icons;
use crate::state::{State, Tone};
use crate::theme::{Border, Overlay, StyleExt, Surface, SurfaceExt, pal, t};
use crate::ui;
use rune::{Color, Cursor, Edge, El, Icon, Shadow, Transition, col, el, icon, row, text};
use rune_kit::{Toast, ToastRegion};
use std::sync::LazyLock;

/// Which look toasts use (see the module note).
static STYLE: LazyLock<u8> = LazyLock::new(|| {
    std::env::var("ADDONS_TOAST_STYLE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
});

const W: f32 = 340.0;

pub fn region(s: &State) -> El {
    if *STYLE == 0 {
        return kit_region(s);
    }
    let mut stack = col()
        .key("toast-region")
        .role("region")
        .label("Notifications")
        .pin(None, None, Some(24.0), Some(24.0))
        .gap(10.0)
        .items_end();
    for n in &s.toasts {
        let look = Look::of(n.tone);
        let body = match *STYLE {
            1 => badge(n.id, &n.title, &n.body, look),
            2 => framed(n.id, &n.title, &n.body, look),
            3 => pill(n.id, &n.title, &n.body, look),
            4 => tinted(n.id, &n.title, &n.body, look),
            _ => seal(n.id, &n.title, &n.body, look),
        };
        stack = stack.child(
            body.role("status")
                .label(n.title.clone())
                .description(n.body.clone())
                .transition(Transition::move_edge(Edge::Bottom))
                .motion(rune::Spring::new(0.28, 0.0)),
        );
    }
    stack
}

#[derive(Clone, Copy)]
struct Look {
    tone: Color,
    glyph: &'static Icon,
}

impl Look {
    fn of(tone: Tone) -> Look {
        let p = pal();
        let (tone, glyph) = match tone {
            Tone::Success => (p.success, &icons::CHECK),
            Tone::Info => (p.accent, &icons::DOWNLOAD),
            Tone::Danger => (p.danger, &icons::ALERT),
        };
        Look { tone, glyph }
    }
}

fn close(id: u64) -> El {
    let p = pal();
    el().key(format!("toast/{id}/close"))
        .role("button")
        .label("Dismiss")
        .focusable(true)
        .size(22.0, 22.0)
        .center()
        .radius(t().skin.icon_button.radius)
        .hover(p.hover)
        .cursor(Cursor::Pointer)
        .on_press(move |s: &mut State| s.dismiss_toast(id))
        .child(icon(&icons::CLOSE, 14.0).color(p.text_muted))
}

/// Title over a wrapping body.
fn words(title: &str, body: &str, title_color: Color, heading: bool) -> El {
    let (th, p) = (t(), pal());
    let font = if heading {
        th.ty.heading.size_of(14.5)
    } else {
        th.ty.strong
    };
    let mut c = col()
        .grow(1.0)
        .min_w(0.0)
        .gap(3.0)
        .child(text(title).font(font).color(title_color).wrap());
    if !body.is_empty() {
        c = c.child(text(body).font(th.ty.small).color(p.text_dim).wrap());
    }
    c
}

/// 1: the status glyph in a small round badge, title and body; the theme's toast box, no stripe.
fn badge(id: u64, title: &str, body: &str, l: Look) -> El {
    let p = pal();
    let mark = el()
        .size(28.0, 28.0)
        .radius(14.0)
        .center()
        .fill(l.tone.alpha(0.16))
        .box_shadow(ring(l.tone.alpha(0.4)))
        .child(icon(l.glyph, 14.0).color(l.tone));
    row()
        .key(format!("toast/{id}"))
        .w(W)
        .padding(12.0)
        .gap(12.0)
        .items_start()
        .skin(&t().skin.toast)
        .child(mark)
        .child(words(title, body, p.text, false).pt(4.0))
        .child(close(id))
}

/// 2: a game alert frame: the theme's panel (studs, bevel) with an ornate top edge (a rule that
/// glows in the tone, a diamond at its middle) and the glyph in an item slot.
fn framed(id: u64, title: &str, body: &str, l: Look) -> El {
    let (th, p) = (t(), pal());
    // The gem's light spilling down over the top of the frame.
    let glow = el()
        .pin(Some(0.0), Some(0.0), Some(0.0), None)
        .h(30.0)
        .radial_gradient(
            [0.5, 0.0],
            [150.0, 30.0],
            [(0.0, l.tone.alpha(0.28)), (1.0, l.tone.alpha(0.0))],
        )
        .a11y_hidden();
    let gem = el()
        .pin(None, Some(-5.0), None, None)
        .w_full()
        .center()
        .a11y_hidden()
        .child(
            icon(&icons::STUD, 11.0)
                .color(l.tone)
                .shadow(0.0, 6.0, l.tone.alpha(0.7)),
        );
    let slot = el()
        .size(40.0, 40.0)
        .center()
        .skin(&th.skin.tile)
        .child(icon(l.glyph, 20.0).color(lit(l.tone, th.skin.tile.fill)));
    row()
        .key(format!("toast/{id}"))
        .w(W + 40.0)
        .padding(14.0)
        .gap(14.0)
        .items_start()
        .skin(&Surface {
            shadows: th
                .skin
                .panel
                .shadows
                .iter()
                .copied()
                .chain([Shadow {
                    y: 6.0,
                    blur: 18.0,
                    color: Color::BLACK.alpha(0.5),
                    ..Shadow::default()
                }])
                .collect(),
            ..th.skin.panel.clone()
        })
        .child(glow)
        .child(gem)
        .child(slot)
        .child(words(title, body, p.heading, true).pt(1.0))
        .child(close(id))
}

/// 3: a compact capsule: glyph, title and a muted body on one line.
fn pill(id: u64, title: &str, body: &str, l: Look) -> El {
    let (th, p) = (t(), pal());
    let mut r = row()
        .key(format!("toast/{id}"))
        .h(40.0)
        .pl(14.0)
        .pr(8.0)
        .gap(10.0)
        .center_y()
        .skin(&Surface {
            radius: 20.0,
            corners: None,
            texture: None,
            ..th.skin.menu.clone()
        })
        .child(icon(l.glyph, 15.0).color(l.tone))
        .child(text(title).font(th.ty.label.weight_of(600.0)).color(p.text));
    if !body.is_empty() {
        r = r.child(
            // shortcut: cut by characters; Rune sizes the capsule by the text's full width
            // even when the text itself is capped and truncated.
            text(if body.chars().count() > 26 {
                format!("{}…", body.chars().take(24).collect::<String>().trim_end())
            } else {
                body.to_string()
            })
            .font(th.ty.small)
            .color(p.text_muted),
        );
    }
    r.child(close(id))
}

/// 4: the whole surface tinted with the tone, edged in it.
fn tinted(id: u64, title: &str, body: &str, l: Look) -> El {
    let (th, p) = (t(), pal());
    let base = p.panel.alpha(1.0);
    let sk = Surface {
        fill: base,
        vertical: None,
        overlay: Some(Overlay::Linear {
            angle: std::f32::consts::PI,
            stops: vec![(0.0, l.tone.alpha(0.13)), (1.0, l.tone.alpha(0.06))],
        }),
        texture: None,
        corners: None,
        border: Some(Border::all(1.0, l.tone.alpha(0.5))),
        ..th.skin.toast.clone()
    };
    row()
        .key(format!("toast/{id}"))
        .w(W)
        .padding(12.0)
        .gap(12.0)
        .items_start()
        .skin(&sk)
        .child(
            el().size(16.0, 20.0)
                .center()
                .child(icon(l.glyph, 16.0).color(l.tone)),
        )
        .child(words(title, body, p.text, false))
        .child(close(id))
}

/// 5: an achievement seal: a round gilded medallion bearing the glyph overlaps the toast's left
/// edge.
fn seal(id: u64, title: &str, body: &str, l: Look) -> El {
    let (th, p) = (t(), pal());
    const D: f32 = 46.0;
    let socket = th.skin.tile.fill.alpha(1.0);
    let medal = el()
        .absolute(0.0, 6.0)
        .size(D, D)
        .radius(D / 2.0)
        .center()
        .fill(socket)
        .radial_gradient(
            [0.5, 0.4],
            D * 0.6,
            [(0.0, socket.mix(l.tone, 0.3)), (1.0, socket)],
        )
        .border_gradient(2.0, p.accent_hi, p.accent.mix(Color::BLACK, 0.55))
        .box_shadow(Shadow {
            y: 2.0,
            blur: 6.0,
            color: Color::BLACK.alpha(0.55),
            ..Shadow::default()
        })
        .child(icon(l.glyph, 20.0).color(lit(l.tone, socket)))
        .a11y_hidden();
    let card = row()
        .w(W)
        .min_h(D + 12.0)
        .pl(D / 2.0 + 14.0)
        .pr(10.0)
        .py(10.0)
        .gap(10.0)
        .items_start()
        .skin(&th.skin.toast)
        .child(words(title, body, p.heading, true).pt(2.0))
        .child(close(id));
    el().key(format!("toast/{id}"))
        .pl(D / 2.0)
        .child(card)
        .child(medal)
}

/// The tone, lightened when it would sink into a dark `ground` (a light theme's dark tones).
fn lit(tone: Color, ground: Color) -> Color {
    if ground.luminance() < 0.2 && tone.luminance() < 0.2 {
        tone.mix(Color::WHITE, 0.45)
    } else {
        tone
    }
}

fn ring(c: Color) -> Shadow {
    Shadow {
        spread: 1.0,
        color: c,
        inset: true,
        ..Shadow::default()
    }
}

/// 0: rune-kit's toast with a tone bar on its left edge (the look before the comparison).
fn kit_region(s: &State) -> El {
    let sk = &t().skin.toast;
    let k = ui::kit();
    let mut region = ToastRegion::new();
    for n in &s.toasts {
        let l = Look::of(n.tone);
        let id = n.id;
        let mut toast = Toast::new(format!("toast/{id}"), n.title.clone())
            .lead(icon(l.glyph, 16.0).color(l.tone))
            .on_close(move |s: &mut State| s.dismiss_toast(id));
        if !n.body.is_empty() {
            toast = toast.description(n.body.clone());
        }
        region = region.toast(
            // The tone bar replaces the skin's border; the kit hard-codes its own radius (AGENTS.md).
            toast
                .build(&k)
                .w(W)
                .skin(sk)
                .border_sides([0.0, 0.0, 0.0, 2.0], l.tone),
        );
    }
    region.build(&k).pin(None, None, Some(24.0), Some(24.0))
}
