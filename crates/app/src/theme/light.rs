//! Lights and mist a theme can lay over its skin: a torch that follows the pointer, lamps fixed
//! to the window and slow mist on the window ground. Off unless the theme lists [`Lights`] in
//! the `themes!` list (`Variant => module + LIGHTS`): other themes build none of this.
//!
//! Everything here is plain elements. A glow is a radial gradient painted over the window
//! (source-over: warm ink on a dark ground reads as light, faint enough that text keeps its
//! contrast). shortcut: no shadows or normals; `rune-light` through `App::compositor` would
//! give real ones, but it takes over drawing every frame of every theme. The
//! torch moves with `live_offset` (no view rebuild per pointer move) and flicker is a
//! renderer-owned opacity [`Cycle`] at a low fixed cadence, so nothing runs in `poll`, the
//! renderer pauses it while the window is covered, reduced motion holds it still and headless
//! snapshots are deterministic (no pointer: no torch).

use super::{Tex, texture};
use rune::tok::motion;
use rune::{Color, Cycle, El, Icon, Live, LoopSpec, el, icon};
use std::cell::RefCell;

pub struct Lights {
    /// Follows the pointer while it is over the window.
    pub torch: Option<Glow>,
    /// Fixed to the window, like sconces on a wall.
    pub lamps: Vec<Lamp>,
    /// Mist layers on the window ground (seen wherever surfaces leave it bare).
    pub mist: Option<Mist>,
    /// Darkens the window's edges and corners, over everything but the lights.
    pub vignette: Option<Color>,
}

/// A soft round light: `color` at `strength` alpha in its middle, falling off to nothing at
/// `radius` points; an optional brighter `core` of a fraction of the radius.
#[derive(Clone, Copy)]
pub struct Glow {
    pub color: Color,
    pub radius: f32,
    pub strength: f32,
    pub core: Option<(Color, f32)>,
    /// Opacity over time (flicker); the core flickers with `flicker.1`.
    pub flicker: Option<(Cycle, Cycle)>,
}

/// Where a lamp hangs: points from the window's top-left, or top-right with `from_right`.
#[derive(Clone, Copy)]
pub struct Lamp {
    pub x: f32,
    pub y: f32,
    pub from_right: bool,
    pub glow: Glow,
    /// Drawn centred on the light, `size` points: the fixture, then its flame (which flickers).
    pub fixture: Option<(&'static Icon, &'static Icon, f32)>,
}

/// Mist: `layers` copies of a texture, offset and cross-faded by `cycle` (each a third of a
/// period later), shown at `alpha` and `scale` × its pixel size.
#[derive(Clone, Copy)]
pub struct Mist {
    pub tex: Tex,
    pub alpha: f32,
    pub scale: f32,
    pub cycle: Cycle,
}

/// Where the pointer is and whether it is over the window (0 or 1).
type Pointer = (Live<[f32; 2]>, Live<f32>);

thread_local! {
    // Per thread like the current theme; shared by the view and the handlers it builds.
    static POINTER: RefCell<Option<Pointer>> = const { RefCell::new(None) };
}

fn pointer() -> Pointer {
    POINTER.with(|p| {
        p.borrow_mut()
            .get_or_insert_with(|| (Live::new([0.0, 0.0]), Live::new(0.0)))
            .clone()
    })
}

/// The mist, for the window ground: add it to the root before the content.
pub fn ground(l: &Lights) -> Option<El> {
    let m = l.mist?;
    let img = texture(m.tex)?;
    let (w, h) = m.tex.size();
    let (w, h) = (w as f32 * m.scale, h as f32 * m.scale);
    let mut layer = el()
        .key("lights/mist")
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .clip_rounded(0.0)
        .a11y_hidden();
    for (i, (dx, dy)) in [(0.0, 0.0), (0.37, 0.61), (0.71, 0.23)]
        .into_iter()
        .enumerate()
    {
        let cycle = Cycle {
            phase: m.cycle.phase + i as f32 / 3.0,
            ..m.cycle
        };
        // shortcut: tiles cover 2560 × 1600 points; a bigger window shows bare ground past that.
        // The fade goes on each tile: on the sheet it would composite a window-sized layer.
        let mut sheet = el()
            .pin(Some(0.0), Some(0.0), None, None)
            .size(2560.0, 1600.0);
        for y in -1..(1600.0 / h).ceil() as i32 {
            for x in -1..(2560.0 / w).ceil() as i32 {
                sheet = sheet.child(
                    el().absolute((x as f32 + dx) * w, (y as f32 + dy) * h)
                        .size(w, h)
                        .pixels(img.pixels())
                        .looping(LoopSpec::cycle(cycle)),
                );
            }
        }
        layer = layer.child(sheet);
    }
    Some(layer.dim(m.alpha))
}

/// The lights, over everything but toasts: the vignette, the lamps and the torch.
pub fn layer(l: &Lights) -> El {
    let mut layer = el()
        .key("lights")
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .clip_rounded(0.0)
        .a11y_hidden();
    if let Some(c) = l.vignette {
        layer = layer.child(
            el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .radial_gradient(
                    [0.55, 0.42],
                    [1100.0, 760.0],
                    [
                        (0.0, c.alpha(0.0)),
                        (0.55, c.alpha(0.0)),
                        (0.8, c.alpha(c.a * 0.55)),
                        (1.0, c),
                    ],
                ),
        );
    }
    for lamp in &l.lamps {
        let r = lamp.glow.radius;
        let (x, y) = (lamp.x - r, lamp.y - r);
        let at = el().size(2.0 * r, 2.0 * r);
        let at = if lamp.from_right {
            at.pin(None, Some(y), Some(x), None)
        } else {
            at.pin(Some(x), Some(y), None, None)
        };
        let mut a = at.child(glow(&lamp.glow));
        if let Some((fixture, flame, size)) = lamp.fixture {
            let place = |e: El| e.absolute(r - size / 2.0, r - size / 2.0);
            a = a.child(place(icon(fixture, size)));
            let flame = place(icon(flame, size));
            a = a.child(match lamp.glow.flicker {
                Some((_, core)) => flame.looping(LoopSpec::cycle(core)),
                None => flame,
            });
        }
        layer = layer.child(a);
    }
    if let Some(g) = &l.torch {
        let (at, present) = pointer();
        let r = g.radius;
        layer = layer.child(
            el().key("lights/torch")
                .absolute(-r, -r)
                .size(2.0 * r, 2.0 * r)
                .live_offset(&at)
                .live_opacity(present.smooth(motion::CALM))
                .child(glow(g)),
        );
    }
    layer
}

/// Lets the torch follow the pointer over `root` (live values only: the view doesn't re-run).
pub fn track(root: El, l: &Lights) -> El {
    if l.torch.is_none() {
        return root;
    }
    let (at, present) = pointer();
    let shown = present.clone();
    root.on_pointer_move_any(move |p, tx| {
        at.set([p.window_x, p.window_y]);
        present.set(1.0);
        tx.live_only();
    })
    .on_hover_change_any(move |inside, tx| {
        if !inside {
            shown.set(0.0);
        }
        tx.live_only();
    })
}

/// A falloff close to a windowed inverse square, as gradient stops.
fn glow(g: &Glow) -> El {
    let falloff = |c: Color, a: f32| -> Vec<(f32, Color)> {
        [0.0, 0.08, 0.2, 0.35, 0.5, 0.65, 0.8, 1.0]
            .into_iter()
            .map(|t: f32| {
                let k = (1.0 - t * t).powi(2) / (1.0 + 9.0 * t * t);
                (t, c.alpha(a * k))
            })
            .collect()
    };
    let flick = |e: El, c: Option<Cycle>| match c {
        Some(c) => e.looping(LoopSpec::cycle(c)),
        None => e,
    };
    let fill = |e: El| e.pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0));
    let mut out = el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0));
    out = out.child(flick(
        fill(el()).radial_gradient([0.5, 0.5], g.radius, falloff(g.color, g.strength)),
        g.flicker.map(|f| f.0),
    ));
    if let Some((c, frac)) = g.core {
        let rc = g.radius * frac;
        let core = el()
            .absolute(g.radius - rc, g.radius - rc)
            .size(2.0 * rc, 2.0 * rc)
            .radial_gradient([0.5, 0.5], rc, falloff(c, c.a));
        out = out.child(flick(core, g.flicker.map(|f| f.1)));
    }
    out
}
