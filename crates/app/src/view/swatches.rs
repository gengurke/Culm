//! Swatches: every colour, text role, surface and glyph of the current theme on one screen, for
//! theme authors (`--snap out.png --scene swatches@<theme>`; docs/THEMES.md). Not in the navigation.

use super::band;
use crate::icons;
use crate::state::State;
use crate::theme::{self, Pal, Surface, SurfaceExt, pal, t};
use crate::ui::{self, Mark};
use rune::{Color, El, col, el, icon, row, text};

pub fn page(_s: &State) -> El {
    let th = t();
    // A frame theme (light pages in a dark frame) also shows the frame's palette and surfaces, on
    // its sidebar: the frame paints the sidebar, band and nav.
    let framed = !std::ptr::eq(th, th.frame_look());
    let on_frame = |inner: fn() -> El| {
        theme::framed(|| {
            el().w_full()
                .padding(8.0)
                .skin(&t().skin.sidebar)
                .child(inner())
        })
    };
    let mut colours = col()
        .gap(18.0)
        .w(250.0)
        .child(section("Palette").child(palette("page")));
    if framed {
        colours = colours.child(section("Frame palette").child(on_frame(|| palette("frame"))));
    }
    let words = col()
        .gap(18.0)
        .w(210.0)
        .child(section("Type").child(type_roles()))
        .child(section("Status").child(status()));
    let mut boxes = col()
        .gap(18.0)
        .grow(1.0)
        .min_w(0.0)
        .child(section("Surfaces").child(surfaces("skin", 112.0, 36.0)));
    if framed {
        boxes = boxes.child(
            section("Frame surfaces").child(on_frame(|| surfaces("frame-skin", 90.0, 28.0))),
        );
    }
    let boxes = boxes.child(section("Glyphs").child(glyphs()));
    col()
        .key("swatches")
        .w_full()
        .h_full()
        .child(band("Swatches", Vec::new, Vec::new()))
        .child(ui::scroll(
            row()
                .w_full()
                .padding(theme::PAD)
                .gap(24.0)
                .child(colours)
                .child(words)
                .child(boxes),
        ))
}

fn section(title: &str) -> El {
    col().gap(8.0).child(
        text(title.to_uppercase())
            .font(t().ty.eyebrow)
            .color(pal().text_muted),
    )
}

/// One chip per `Pal` colour, its name under it (status tones as their ink).
fn palette(part: &str) -> El {
    let p = pal();
    let mut grid = row()
        .key(format!("swatches/{part}"))
        .w_full()
        .gap(4.0)
        .flex_wrap();
    for (name, c) in p.colours() {
        grid = grid.child(
            row()
                .key(format!("swatches/{part}/{name}"))
                .w(115.0)
                .gap(6.0)
                .center_y()
                .child(el().size(16.0, 16.0).fill(c).border(1.0, p.divider))
                .child(text(name).font(t().ty.small).color(p.text_dim).truncate()),
        );
    }
    grid
}

/// Each `Type` role set in itself.
fn type_roles() -> El {
    let (ty, p) = (&t().ty, pal());
    let mut list = col().key("swatches/type").gap(4.0);
    for (name, style) in ty.roles() {
        list = list.child(
            text(name)
                .key(format!("swatches/type/{name}"))
                .font(style)
                .color(p.text),
        );
    }
    list
}

/// Every `Skin` surface as a `w` × `h` box with its name in its ink, keyed `swatches/<part>/<slot>`.
fn surfaces(part: &str, w: f32, h: f32) -> El {
    let (th, p) = (t(), pal());
    let font = if h < 32.0 { th.ty.caption } else { th.ty.small };
    let mut grid = row()
        .key(format!("swatches/{part}"))
        .w_full()
        .gap(6.0)
        .flex_wrap();
    for (name, sk) in th.skin.surfaces() {
        grid = grid.child(
            el().key(format!("swatches/{part}/{name}"))
                .w(w)
                .h(h)
                .skin(sk)
                .center()
                .child(text(name).font(font).color(ink(sk, p))),
        );
    }
    grid
}

/// The surface's own ink, or where it leaves ink unset, whichever of `text` and `bg` reads on it.
fn ink(sk: &Surface, p: &Pal) -> Color {
    let on = |c| theme::contrast(c, sk.fill, p.bg);
    match sk.ink {
        ink if ink.a > 0.0 => ink,
        _ if on(p.text) >= on(p.bg) => p.text,
        _ => p.bg,
    }
}

/// A badge per mark, and a warning and a danger notice.
fn status() -> El {
    let marks = [
        ("good", Mark::Good),
        ("update", Mark::Update),
        ("busy", Mark::Busy),
        ("bad", Mark::Bad),
        ("muted", Mark::Muted),
        ("off", Mark::Off),
    ];
    let mut badges = row()
        .key("swatches/status")
        .w_full()
        .gap(8.0)
        .center_y()
        .flex_wrap();
    for (name, mark) in marks {
        badges = badges
            .child(ui::tag(name, mark))
            .child(ui::badge(&icons::ALERT, mark));
    }
    let note = |mark, words: &str| {
        ui::notice(row().gap(8.0).center_y(), mark)
            .child(text(words.to_string()).font(t().ty.body).color(pal().text))
    };
    col()
        .gap(8.0)
        .child(badges)
        .child(note(Mark::Update, "A warning notice"))
        .child(note(Mark::Bad, "A danger notice"))
}

/// The emblem, the class medallion's rings (plain and gilded) and the theme's glyph swaps.
fn glyphs() -> El {
    let p = pal();
    let mut r = row()
        .key("swatches/glyphs")
        .w_full()
        .gap(14.0)
        .flex_wrap()
        .center_y()
        .child(ui::emblem(32.0));
    for on in [false, true] {
        if let Some(m) = ui::class_medal("druid", 32.0, on, None) {
            r = r.child(m);
        }
    }
    for (stock, own) in t().skin.glyphs {
        r = r.child(
            row()
                .gap(4.0)
                .center_y()
                .child(icon(stock, 18.0).color(p.text_muted))
                .child(text("→").font(t().ty.small).color(p.text_muted))
                .child(icon(own, 18.0).color(p.text)),
        );
    }
    r
}
