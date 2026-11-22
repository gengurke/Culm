//! Toasts: the stack of notices at the bottom right, each a framed alert in the theme's `toast`
//! surface. `--scene toasts@<theme>` shows every tone at once.

use crate::icons;
use crate::state::{State, Tone};
use crate::theme::{StyleExt, SurfaceExt, pal, t};
use rune::{Color, Cursor, Edge, El, Icon, Transition, col, el, icon, row, text};

const W: f32 = 380.0;

pub fn region(s: &State) -> El {
    let mut stack = col()
        .key("toast-region")
        .role("region")
        .label("Notifications")
        .pin(None, None, Some(24.0), Some(24.0))
        .gap(10.0)
        .items_end();
    for n in &s.toasts {
        stack = stack.child(
            framed(n.id, &n.title, &n.body, Look::of(n.tone))
                .role("status")
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
            Tone::Success => (p.success.ink, &icons::CHECK),
            Tone::Info => (p.accent, &icons::DOWNLOAD),
            Tone::Danger => (p.danger.ink, &icons::ALERT),
        };
        Look { tone, glyph }
    }
}

fn close(id: u64) -> El {
    let sk = &t().skin.icon_button;
    el().key(format!("toast/{id}/close"))
        .role("button")
        .label("Dismiss")
        .focusable(true)
        .size(22.0, 22.0)
        .center()
        .skin(sk)
        .cursor(Cursor::Pointer)
        .on_press(move |s: &mut State| s.dismiss_toast(id))
        .child(icon(&icons::CLOSE, 14.0).color(sk.ink))
}

/// A heading over a wrapping body.
fn words(title: &str, body: &str) -> El {
    let (th, p) = (t(), pal());
    let mut c = col().grow(1.0).min_w(0.0).gap(3.0).child(
        text(title)
            .font(th.ty.heading.size_of(14.5))
            .color(p.heading)
            .wrap_anywhere(),
    );
    if !body.is_empty() {
        c = c.child(
            text(body)
                .font(th.ty.small)
                .color(p.text_dim)
                .wrap_anywhere(),
        );
    }
    c
}

/// A game alert frame: the theme's toast surface (studs, bevel) with an ornate top edge (a rule
/// that glows in the tone, the surface's corner glyph as a gem at its middle) and the glyph in an
/// item slot.
fn framed(id: u64, title: &str, body: &str, l: Look) -> El {
    let th = t();
    let sk = &th.skin.toast;
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
    let slot = el()
        .size(40.0, 40.0)
        .center()
        .skin(&th.skin.tile)
        .child(icon(l.glyph, 20.0).color(lit(l.tone, th.skin.tile.fill)));
    let mut r = row()
        .key(format!("toast/{id}"))
        .w(W)
        .padding(14.0)
        .gap(14.0)
        .items_start()
        .skin(sk)
        .child(glow);
    if let Some(c) = sk.corners {
        r = r.child(
            el().pin(None, Some(-5.0), None, None)
                .w_full()
                .center()
                .a11y_hidden()
                .child(
                    icon(c.glyph, 11.0)
                        .color(l.tone)
                        .shadow(0.0, 6.0, l.tone.alpha(0.7)),
                ),
        );
    }
    r.child(slot)
        .child(words(title, body).pt(1.0))
        .child(close(id))
}

/// The tone, lightened when it would sink into a dark `ground` (a light theme's dark tones).
fn lit(tone: Color, ground: Color) -> Color {
    if ground.luminance() < 0.2 && tone.luminance() < 0.2 {
        tone.mix(Color::WHITE, 0.45)
    } else {
        tone
    }
}
