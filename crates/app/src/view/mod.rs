//! The window: sidebar, starry header band, the current screen and toasts.

mod get_addons;
mod maintenance;
mod my_addons;
mod settings;
mod toasts;

use crate::brand;
use crate::icons;
use crate::state::{Intent, Job, Screen, State, dispatch};
use crate::theme::{self, StyleExt, SurfaceExt, pal, t};
use crate::theme::{BandDeco, Surface};
use crate::ui::{self, Kind};
use rune::{Cursor, Cx, El, Icon, Tx, col, el, icon, row, text};

pub fn view(s: &State, cx: &mut Cx<State>) -> El {
    theme::begin(s);
    cx.help_look(ui::help_look());
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
        .child(toasts::region(s))
}

/// The slim band every page starts with: the title, muted meta pieces beside it (built by `meta`
/// inside the band's frame, so they take its palette) set apart by hairlines, and actions at the right.
pub(crate) fn band(title: &str, meta: impl FnOnce() -> Vec<El>, actions: Vec<El>) -> El {
    theme::framed(|| band_in_frame(title, meta(), actions))
}

fn band_in_frame(title: &str, meta: Vec<El>, actions: Vec<El>) -> El {
    let (th, p) = (t(), pal());
    let mut b = row()
        .key("band")
        .w_full()
        .h(theme::BAND_H)
        .px(theme::PAD)
        .gap(14.0)
        .center_y()
        .skin(&th.skin.band);
    if let Some(deco) = band_deco(th.skin.band_deco) {
        b = b.child(deco);
    }
    b = b.child(
        text(title)
            .key("page-title")
            .role("heading")
            .font(th.ty.hero)
            .color(p.heading),
    );
    if !meta.is_empty() {
        b = b
            .child(ui::hairline(18.0))
            .child(ui::meta_line(meta).key("page-meta").shrink());
    }
    b.child(el().grow(1.0))
        .child(row().gap(8.0).center_y().children(actions))
}

/// A muted piece of the band's meta line.
pub(crate) fn band_note(s: impl Into<String>) -> El {
    text(s.into()).font(t().ty.note).color(pal().text_dim)
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
            for _ in 0..28 {
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
            for _ in 0..26 {
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
        .h(theme::BAND_H)
        .px(20.0)
        .gap(10.0)
        .center_y()
        .child(el().size(40.0, 40.0).center().child(ui::brand_mark(32.0)))
        .child(wordmark());
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

/// Which wordmark the sidebar header draws (`ADDONS_WORDMARK`, default 0).
// shortcut: a temporary chooser for the wordmark comparison sheet; keep the winner, drop the rest
// and the env var once one is picked. Every family but Inter (embedded) needs bundling first.
static WORDMARK: std::sync::LazyLock<usize> = std::sync::LazyLock::new(|| {
    std::env::var("ADDONS_WORDMARK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
});

/// The app name (and tagline) beside the brand mark.
fn wordmark() -> El {
    use rune::{Family, TextStyle};
    let (th, p) = (t(), pal());
    let name = brand::APP_NAME;
    let tag = |s: &str, style: TextStyle| text(s.to_string()).font(style).color(p.text_muted);
    let (mark, tagline) = match *WORDMARK {
        // 1: Inter SemiBold, title case, no tagline.
        1 => (
            text(name).font(TextStyle::new(19.0, 600.0).tracking(-0.4).lh(24.0)),
            None,
        ),
        // 2: Inter, title case over a quiet sentence-case tagline.
        2 => (
            text(name).font(TextStyle::new(17.0, 650.0).tracking(-0.3).lh(21.0)),
            Some(tag(
                "Addons for WoW Forever",
                TextStyle::new(11.5, 400.0).lh(15.0),
            )),
        ),
        // 3: Noto Serif SemiBold, title case, no tagline.
        3 => (
            text(name).font(
                TextStyle::new(20.0, 600.0)
                    .family(Family::Named("Noto Serif, serif"))
                    .lh(24.0),
            ),
            None,
        ),
        // 4: STIX Two Text, title case, with a small-caps-style tracked tagline.
        4 => (
            text(name).font(
                TextStyle::new(21.0, 600.0)
                    .family(Family::Named("STIX Two Text, serif"))
                    .lh(24.0),
            ),
            Some(tag(
                "ADDON MANAGER",
                TextStyle::new(9.5, 500.0).tracking(1.4).lh(12.0),
            )),
        ),
        // 5: Comfortaa Bold, lowercase, rounded, no tagline.
        5 => (
            text(name.to_lowercase()).font(
                TextStyle::new(20.0, 700.0)
                    .family(Family::Named("Comfortaa"))
                    .lh(24.0),
            ),
            None,
        ),
        // 6: Inter Bold caps, widely tracked and small: a label, not a logo.
        6 => (
            text(name.to_uppercase()).font(TextStyle::new(13.0, 700.0).tracking(3.2).lh(16.0)),
            None,
        ),
        // 0: the theme's serif caps over the tracked tagline.
        _ => (
            text(name.to_uppercase()).font(th.ty.wordmark),
            Some(tag(brand::TAGLINE, th.ty.eyebrow)),
        ),
    };
    let mut c = col()
        .key("brand/wordmark")
        .gap(2.0)
        .min_w(0.0)
        .child(mark.color(p.heading).truncate());
    if let Some(t) = tagline {
        c = c.child(t.truncate());
    }
    c
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
    r = r
        .child(icon(theme::glyph(glyph), 16.0).color(if on { p.accent } else { p.text_muted }))
        .child(
            text(label)
                .font(if on {
                    th.ty.body.weight_of(600.0)
                } else {
                    th.ty.body
                })
                .color(if on { p.heading } else { p.text_dim })
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
                        text(s.inst_name(inst))
                            .font(t().ty.small)
                            .color(p.text_dim)
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
    // The name alone (it fits the sidebar); what tells it apart from a same-named installation
    // joins the version line.
    let (label, version, tell) = match s.installation() {
        Some(i) => (
            i.label.clone(),
            i.version
                .clone()
                .unwrap_or_else(|| "unknown version".into()),
            s.inst_tell(i),
        ),
        None => ("No installation".into(), "Add one to start".into(), None),
    };
    let mut second = vec![
        text(version)
            .font(t().ty.mono.size_of(11.0))
            .color(p.text_muted),
    ];
    if let Some(tell) = tell {
        second.push(text(tell).font(t().ty.small).color(p.text_muted));
    }
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
                .child(text(label).font(t().ty.small).color(p.text))
                .child(ui::meta_line(second)),
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
            || vec![band_note("Looking for World of Warcraft: Forever…")],
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
            || vec![band_note("Let's get your addons in order")],
            vec![],
        ))
        .child(centered(card))
}
