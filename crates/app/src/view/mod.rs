//! The window: sidebar, starry header band, the current screen and toasts.

mod get_addons;
mod maintenance;
mod my_addons;
mod settings;
mod toasts;

use crate::brand;
use crate::icons;
use crate::state::{AppUpdate, Intent, Job, Screen, State, dispatch};
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
        .children(app_update(s, "app-update/sidebar").map(|b| col().px(16.0).pb(14.0).child(b)))
        .child(installation_picker(s))
}

/// The app's own update while one is on offer (F8.4): the sidebar shows it on every screen,
/// Settings → About next to "Check for updates".
pub(crate) fn app_update(s: &State, key: &str) -> Option<El> {
    let p = pal();
    let self_updating = s.backend.self_updating();
    Some(match &s.app_update {
        AppUpdate::Available(r) | AppUpdate::Failed { release: r, .. } if self_updating => ui::button(
            key,
            &format!("Update to {}", r.version),
            Kind::Primary,
            Some(&icons::DOWNLOAD),
            true,
            |s, tx| dispatch(s, Intent::InstallAppUpdate, tx),
        )
        .help("Download the new version over this one (checked before it's used); a restart starts it"),
        AppUpdate::Available(r) | AppUpdate::Failed { release: r, .. } => ui::button(
            key,
            &format!("Update to {}", r.version),
            Kind::Primary,
            Some(&icons::EXTERNAL),
            true,
            |s, tx| dispatch(s, Intent::InstallAppUpdate, tx),
        )
        .help("Open the release page to download it: this copy can't replace itself"),
        AppUpdate::Downloading { release, done } => text(format!(
            "Downloading {}… {}%",
            release.version,
            (done * 100.0).round() as u32
        ))
        .key(key)
        .font(t().ty.note)
        .color(p.text_dim),
        AppUpdate::Ready(version) => ui::button(
            key,
            &format!("Restart for {version}"),
            Kind::Primary,
            Some(&icons::ROTATE),
            true,
            |s, tx| dispatch(s, Intent::RestartApp, tx),
        )
        .help("Close and start the new version"),
        _ => return None,
    })
}

/// The app name over the tagline beside the brand mark, in Inter (bundled, so it looks the same
/// everywhere) whatever the theme.
fn wordmark() -> El {
    use rune::TextStyle;
    let p = pal();
    col()
        .key("brand/wordmark")
        .gap(2.0)
        .min_w(0.0)
        .child(
            text(brand::APP_NAME)
                .font(TextStyle::new(17.0, 650.0).tracking(-0.3).lh(21.0))
                .color(p.heading)
                .truncate(),
        )
        .child(
            text(brand::TAGLINE)
                .font(TextStyle::new(11.5, 400.0).lh(15.0))
                .color(p.text_muted)
                .truncate(),
        )
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

/// What a wait for a download fetched by hand means (F3.5, F4.3).
pub(crate) const AWAITING_HELP: &str = "Save the file in your browser; Culm installs it as soon as it is in your Downloads folder. Saved somewhere else? Use \"Install from file\" in the details.";

/// Why an install or update opens the browser (F3.5, F4.3).
pub(crate) fn by_hand_help(source: &str) -> String {
    format!(
        "The author only allows downloads through {source}'s own app or website. This starts the download in your browser; Culm installs the file once it is in your Downloads folder."
    )
}
