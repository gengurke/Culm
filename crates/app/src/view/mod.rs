//! The window: sidebar, starry header band, the current screen and toasts.

mod get_addons;
mod maintenance;
mod my_addons;
mod settings;

use crate::brand;
use crate::icons;
use crate::state::{Intent, Job, Screen, State, Tone, dispatch};
use crate::theme::{self, StyleExt, SurfaceExt, pal, t};
use crate::theme::{BandDeco, Surface};
use crate::ui::{self, Kind};
use rune::{Cursor, Cx, El, Icon, Tx, col, el, icon, row, text};
use rune_kit::{Toast, ToastRegion};

pub fn view(s: &State, _cx: &mut Cx<State>) -> El {
    theme::begin(s);
    let page = match (s.screen, s.installation()) {
        (Screen::Settings, _) => settings::page(s),
        (_, None) if s.job == Job::Booting || s.job == Job::Scanning => loading_page(s),
        (_, None) => onboarding_page(),
        (Screen::MyAddons, Some(_)) => my_addons::page(s),
        (Screen::GetAddons, Some(_)) => get_addons::page(s),
        (Screen::Maintenance, Some(_)) => maintenance::page(s),
    };
    row()
        .key("app")
        .w_full()
        .h_full()
        .skin(&t().skin.window)
        .child(theme::framed(|| sidebar(s)))
        .child(col().key("main").grow(1.0).min_w(0.0).h_full().child(page))
        .child(toasts(s))
}

/// The band every page starts with: title, a subtitle and actions on the right.
pub(crate) fn band(title: &str, subtitle: El, actions: Vec<El>) -> El {
    theme::framed(|| band_in_frame(title, subtitle, actions))
}

fn band_in_frame(title: &str, subtitle: El, actions: Vec<El>) -> El {
    let (th, p) = (t(), pal());
    let mut b = row()
        .key("band")
        .w_full()
        .h(theme::BAND_H)
        .px(theme::PAD)
        .pb(22.0)
        .gap(16.0)
        .items_end()
        .skin(&th.skin.band);
    if let Some(deco) = band_deco(th.skin.band_deco) {
        b = b.child(deco);
    }
    b.child(
        col()
            .grow(1.0)
            .min_w(0.0)
            .gap(6.0)
            .child(
                text(title)
                    .key("page-title")
                    .role("heading")
                    .font(th.ty.hero)
                    .color(p.heading),
            )
            .child(subtitle),
    )
    .child(row().gap(8.0).center_y().children(actions))
}

pub(crate) fn subtitle(s: impl Into<String>) -> El {
    theme::framed(|| {
        text(s.into())
            .font(t().ty.note)
            .color(pal().text_dim)
            .truncate()
    })
}

/// The band's decoration layer: a fixed scatter (a tiny LCG keeps it the same every frame).
fn band_deco(deco: BandDeco) -> Option<El> {
    let p = pal();
    let mut layer = el()
        .key("band/stars")
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .clip_rounded(0.0)
        .a11y_hidden();
    let mut seed: u32 = 0x9e37_79b9;
    let mut next = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    match deco {
        BandDeco::None => return None,
        BandDeco::Stars => {
            for _ in 0..46 {
                let (x, y, size, a) = (
                    next() * 1600.0,
                    next() * theme::BAND_H,
                    if next() > 0.85 { 2.0 } else { 1.0 },
                    0.18 + next() * 0.55,
                );
                layer = layer.child(
                    el().absolute(x, y)
                        .size(size, size)
                        .radius(size / 2.0)
                        .fill(p.text.alpha(a)),
                );
            }
        }
        BandDeco::Embers => {
            // Sparks thin out and fade as they rise from the bottom edge.
            for _ in 0..40 {
                let (x, rise, size) = (
                    next() * 1600.0,
                    next() * next(),
                    if next() > 0.8 { 3.0 } else { 2.0 },
                );
                let a = 0.85 - rise * 0.75;
                let spark = p.warning.mix(p.accent_hi, next());
                layer = layer.child(
                    el().absolute(x, theme::BAND_H - 6.0 - rise * theme::BAND_H * 0.9)
                        .size(size, size)
                        .radius(size / 2.0)
                        .fill(spark.alpha(a))
                        .shadow(0.0, 6.0, spark.alpha(a * 0.8)),
                );
            }
        }
    }
    Some(layer)
}

fn sidebar(s: &State) -> El {
    let p = pal();
    let updates = s.update_count();
    let brand_block = row()
        .key("brand")
        .h(72.0)
        .px(20.0)
        .gap(10.0)
        .center_y()
        .child(el().size(28.0, 28.0).center().child(ui::emblem(19.0)))
        .child(
            text(brand::APP_NAME.to_uppercase())
                .font(t().ty.wordmark)
                .color(p.heading)
                .truncate(),
        );
    let nav = col()
        .key("nav")
        .role("list")
        .focus_group()
        .pt(8.0)
        .child(
            text("LIBRARY")
                .font(t().ty.eyebrow)
                .color(p.text_muted)
                .px(24.0)
                .pb(8.0),
        )
        .child(nav_row(
            s,
            Screen::MyAddons,
            "nav/my-addons",
            "My Addons",
            &icons::PACKAGE,
            updates,
        ))
        .child(nav_row(
            s,
            Screen::GetAddons,
            "nav/get-addons",
            "Get Addons",
            &icons::COMPASS,
            0,
        ))
        .child(
            text("CLIENT")
                .font(t().ty.eyebrow)
                .color(p.text_muted)
                .px(24.0)
                .pt(20.0)
                .pb(8.0),
        )
        .child(nav_row(
            s,
            Screen::Maintenance,
            "nav/maintenance",
            "Maintenance",
            &icons::WRENCH,
            0,
        ))
        .child(nav_row(
            s,
            Screen::Settings,
            "nav/settings",
            "Settings",
            &icons::SLIDERS,
            0,
        ));
    col()
        .key("sidebar")
        .w(theme::SIDEBAR_W)
        .h_full()
        .skin(&t().skin.sidebar)
        .child(brand_block)
        .child(nav)
        .child(el().grow(1.0))
        .child(installation_picker(s))
}

fn nav_row(
    s: &State,
    screen: Screen,
    key: &str,
    label: &str,
    glyph: &'static Icon,
    count: usize,
) -> El {
    let (th, p) = (t(), pal());
    let on = s.screen == screen;
    let mut r = row()
        .key(key)
        .role("link")
        .label(label)
        .checked(on)
        .focusable(true)
        .h(36.0)
        .pl(24.0)
        .pr(16.0)
        .gap(12.0)
        .center_y()
        .skin(if on {
            &th.skin.nav_active
        } else {
            &th.skin.nav
        })
        .cursor(Cursor::Pointer)
        .on_press_tx(move |s: &mut State, tx: &mut Tx| dispatch(s, Intent::Navigate(screen), tx));
    if on {
        r = r.child(el().absolute(0.0, 0.0).w(2.0).h(36.0).fill(p.accent));
    }
    r = r
        .child(icon(theme::glyph(glyph), 16.0).color(if on { p.accent } else { p.text_muted }))
        .child(
            text(label)
                .font(th.ty.body)
                .color(if on { p.text } else { p.text_dim })
                .grow(1.0),
        );
    if count > 0 {
        r = r.child(
            row()
                .key(format!("{key}/count"))
                .h(18.0)
                .px(7.0)
                .center()
                .skin(&Surface {
                    fill: p.warning_soft,
                    ..th.skin.chip.clone()
                })
                .child(
                    text(count.to_string())
                        .font(t().ty.number.lh(14.0))
                        .color(p.warning),
                ),
        );
    }
    r
}

fn installation_picker(s: &State) -> El {
    let p = pal();
    let mut wrap = col()
        .key("installation")
        .border_sides([1.0, 0.0, 0.0, 0.0], p.divider);
    if s.inst_menu {
        let mut menu = col()
            .key("installation/menu")
            .role("menu")
            .py(6.0)
            .skin(&t().skin.menu);
        for inst in &s.installations {
            let id = inst.id.clone();
            let on = s.current.as_deref() == Some(&inst.id);
            menu = menu.child(
                row()
                    .key(format!("installation/{}", inst.id))
                    .role("menuitem")
                    .h(36.0)
                    .px(20.0)
                    .gap(10.0)
                    .center_y()
                    .hover(p.hover)
                    .cursor(Cursor::Pointer)
                    .on_press_tx(move |s: &mut State, tx: &mut Tx| {
                        dispatch(s, Intent::SelectInstallation(id.clone()), tx)
                    })
                    .child(icon(&icons::CHECK, 14.0).color(if on {
                        p.accent
                    } else {
                        p.accent.alpha(0.0)
                    }))
                    .child(
                        text(inst.label.clone())
                            .font(t().ty.small)
                            .color(p.text_dim)
                            .truncate()
                            .grow(1.0),
                    ),
            );
        }
        menu = menu.child(
            row()
                .key("installation/add")
                .role("menuitem")
                .h(36.0)
                .px(20.0)
                .gap(10.0)
                .center_y()
                .hover(p.hover)
                .cursor(Cursor::Pointer)
                .on_press_tx(|s: &mut State, tx: &mut Tx| dispatch(s, Intent::AddInstallation, tx))
                .child(icon(&icons::PLUS, 14.0).color(p.text_muted))
                .child(
                    text("Add installation…")
                        .font(t().ty.small)
                        .color(p.text_dim),
                ),
        );
        wrap = wrap.child(menu);
    }
    let (label, version) = match s.installation() {
        Some(i) => (
            i.label.clone(),
            i.version
                .clone()
                .unwrap_or_else(|| "unknown version".into()),
        ),
        None => ("No installation".into(), "Add one to start".into()),
    };
    let selector = row()
        .key("installation/selector")
        .role("button")
        .label("Installation")
        .focusable(true)
        .expanded(s.inst_menu)
        .h(64.0)
        .px(20.0)
        .gap(12.0)
        .center_y()
        .hover(p.hover)
        .cursor(Cursor::Pointer)
        .on_press(|s: &mut State| s.inst_menu = !s.inst_menu)
        .child(ui::glyph_tile(&icons::DRIVE, 32.0))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(2.0)
                .child(text(label).font(t().ty.small).color(p.text).truncate())
                .child(
                    text(version)
                        .font(t().ty.mono.size_of(11.0))
                        .color(p.text_muted)
                        .truncate(),
                ),
        )
        .child(icon(&icons::UPDOWN, 14.0).color(p.text_muted));
    wrap.child(selector)
}

fn centered(content: El) -> El {
    col()
        .key("content")
        .grow(1.0)
        .min_h(0.0)
        .w_full()
        .center()
        .padding(theme::PAD)
        .child(content)
}

fn loading_page(_s: &State) -> El {
    let p = pal();
    col()
        .w_full()
        .h_full()
        .child(band(
            "Welcome",
            subtitle("Looking for World of Warcraft: Forever…"),
            vec![],
        ))
        .child(centered(
            row()
                .key("loading")
                .gap(12.0)
                .center_y()
                .child(ui::spinner(16.0, p.accent))
                .child(
                    text("Looking for installations…")
                        .font(t().ty.body)
                        .color(p.text_dim),
                ),
        ))
}

fn onboarding_page() -> El {
    let p = pal();
    let card = col()
        .key("onboarding")
        .max_w(460.0)
        .gap(14.0)
        .items_center()
        .child(ui::emblem(32.0))
        .child(text("Find your World of Warcraft: Forever").font(t().ty.display).color(p.heading).align(rune::Align::Center))
        .child(
            text("No Forever client was found in the usual places. Point us at your World of Warcraft folder: the game folder, its _forever_ folder or the one holding Wow.exe all work.")
                .font(t().ty.note)
                .color(p.text_dim)
                .wrap()
                .align(rune::Align::Center),
        )
        .child(row().pt(10.0).gap(10.0).child(ui::button(
            "onboarding/add",
            "Add installation",
            Kind::Primary,
            Some(&icons::PLUS),
            true,
            |s, tx| dispatch(s, Intent::AddInstallation, tx),
        )));
    col()
        .w_full()
        .h_full()
        .child(band(
            "Welcome",
            subtitle("Let's get your addons in order"),
            vec![],
        ))
        .child(centered(card))
}

fn toasts(s: &State) -> El {
    let (sk, p) = (&t().skin.toast, pal());
    let k = ui::kit();
    let mut region = ToastRegion::new();
    for t in &s.toasts {
        let (tone, glyph) = match t.tone {
            Tone::Success => (p.success, &icons::CHECK),
            Tone::Info => (p.accent, &icons::DOWNLOAD),
            Tone::Danger => (p.danger, &icons::ALERT),
        };
        let id = t.id;
        let mut toast = Toast::new(format!("toast/{id}"), t.title.clone())
            .lead(icon(glyph, 16.0).color(tone))
            .on_close(move |s: &mut State| s.dismiss_toast(id));
        if !t.body.is_empty() {
            toast = toast.description(t.body.clone());
        }
        region = region.toast(
            // The tone bar replaces the skin's border; the kit hard-codes its own radius (AGENTS.md).
            toast
                .build(&k)
                .w(340.0)
                .skin(sk)
                .border_sides([0.0, 0.0, 0.0, 2.0], tone),
        );
    }
    region.build(&k).pin(None, None, Some(24.0), Some(24.0))
}
