//! Get Addons: CurseForge search (debounced), the popular list, install from URL.

use super::{band, subtitle};
use crate::icons;
use crate::state::{Fetch, Intent, Op, Screen, State, dispatch};
use crate::theme::{self, SurfaceExt, pal, t};
use crate::ui::{self, Kind};
use addon_core::RemoteAddon;
use rune::{El, ScrollSpec, col, el, icon, row, text};

const COLUMNS: usize = 3;

pub fn page(s: &State) -> El {
    let p = pal();
    let ready = s.curseforge_ready();
    let search = ui::field("get-addons/search", &s.query, "Search CurseForge for Forever addons", Some(&icons::SEARCH), |s: &mut State| &mut s.query)
        .on_change(|s, tx| dispatch(s, Intent::Search, tx))
        .on_submit(|s, tx| dispatch(s, Intent::Search, tx))
        .disabled(!ready)
        .build()
        .grow(1.0)
        .max_w(560.0);
    let url = ui::field("get-addons/url", &s.url, "Paste a CurseForge or GitHub URL", Some(&icons::LINK), |s: &mut State| &mut s.url)
        .on_submit(|s, tx| dispatch(s, Intent::InstallFromUrl, tx))
        .build()
        .w(300.0);
    let install_url = ui::button(
        "get-addons/url-install",
        if s.url_busy { "Installing…" } else { "Install" },
        Kind::Secondary,
        Some(&icons::DOWNLOAD),
        !s.url_busy && !s.url.text.trim().is_empty(),
        |s, tx| dispatch(s, Intent::InstallFromUrl, tx),
    );
    let toolbar = row()
        .key("get-addons/toolbar")
        .w_full()
        .px(theme::PAD)
        .py(14.0)
        .gap(8.0)
        .center_y()
        .skin(&t().skin.toolbar)
        .child(search)
        .child(el().grow(1.0).min_w(16.0))
        .child(url)
        .child(install_url);

    let mut content = col().w_full().gap(16.0).px(theme::PAD).py(24.0);
    if !ready {
        content = content.child(no_key_card());
    } else {
        let q = s.query.text.trim();
        let (title, fetch) = if q.is_empty() { ("POPULAR FOR FOREVER".to_string(), &s.featured) } else { (format!("RESULTS FOR “{}”", q.to_uppercase()), &s.results) };
        content = content.child(text(title).key("get-addons/section").font(t().ty.eyebrow).color(p.text_muted));
        content = content.child(match fetch {
            Fetch::Idle | Fetch::Loading => row().key("get-addons/loading").py(24.0).gap(10.0).center_y().child(ui::spinner(14.0, p.accent)).child(text("Searching CurseForge…").font(t().ty.body).color(p.text_dim)),
            Fetch::Failed(e) => row().key("get-addons/error").gap(8.0).child(icon(&icons::ALERT, 14.0).color(p.danger)).child(text(e.clone()).font(t().ty.body).color(p.danger)),
            Fetch::Ready(list) if list.is_empty() => col()
                .key("get-addons/none")
                .py(32.0)
                .gap(6.0)
                .child(text(format!("No addons match “{q}”")).font(t().ty.body).color(p.text_dim))
                .child(text("Try a shorter name, or paste the addon's URL above.").font(t().ty.small).color(p.text_muted)),
            Fetch::Ready(list) => grid(s, list),
        });
    }
    col()
        .w_full()
        .h_full()
        .child(band("Get Addons", subtitle("Find new addons for World of Warcraft: Forever"), vec![]))
        .child(toolbar)
        .child(col().key("get-addons/scroll").grow(1.0).min_h(0.0).w_full().scroll(ScrollSpec { stick_bottom: false }).child(content))
}

fn no_key_card() -> El {
    let p = pal();
    // The accent bar replaces the panel's border; an inset hairline keeps the other edges.
    ui::edge(row().key("get-addons/no-key").w_full().max_w(760.0).gap(16.0).padding(20.0).skin(&t().skin.panel), p.border)
        .border_sides([0.0, 0.0, 0.0, 2.0], p.accent)
        .child(el().size(36.0, 36.0).center().fill(p.accent_soft).child(icon(&icons::KEY, 18.0).color(p.accent)))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(6.0)
                .child(text("CurseForge needs an API key").font(t().ty.heading).color(p.heading))
                .child(
                    text("Search and CurseForge updates use your own free key from console.curseforge.com. Add it in Settings. GitHub addons and install-from-URL work without one.")
                        .font(t().ty.note)
                        .color(p.text_dim)
                        .wrap(),
                ),
        )
        .child(ui::button("get-addons/open-settings", "Open Settings", Kind::Primary, None, true, |s, tx| dispatch(s, Intent::Navigate(Screen::Settings), tx)))
}

fn grid(s: &State, list: &[RemoteAddon]) -> El {
    let mut g = col().key("get-addons/results").w_full().gap(16.0);
    for chunk in list.chunks(COLUMNS) {
        let mut r = row().w_full().gap(16.0).items_stretch();
        for remote in chunk {
            r = r.child(result_card(s, remote).grow(1.0).min_w(0.0));
        }
        for _ in chunk.len()..COLUMNS {
            r = r.child(el().grow(1.0));
        }
        g = g.child(r);
    }
    g
}

fn result_card(s: &State, remote: &RemoteAddon) -> El {
    let p = pal();
    let key = format!("remote/{}", remote.external_id);
    let action = if s.is_installed(remote) {
        ui::badge("Installed", p.success, p.success_soft, true).key(format!("{key}/installed"))
    } else {
        match s.installing.get(&remote.external_id) {
            Some(Op::Downloading(done)) => col()
                .key(format!("{key}/progress"))
                .w(120.0)
                .gap(6.0)
                .child(text(format!("Installing… {:.0}%", done * 100.0)).font(t().ty.small).color(p.text_dim))
                .child(ui::progress_line(*done, 2.0)),
            Some(_) => row().key(format!("{key}/progress")).gap(8.0).center_y().child(ui::spinner(12.0, p.accent)).child(text("Installing…").font(t().ty.small).color(p.text_dim)),
            None => {
                let r = remote.clone();
                ui::button(format!("{key}/install"), "Install", Kind::Secondary, Some(&icons::DOWNLOAD), true, move |s, tx| dispatch(s, Intent::Install(r.clone()), tx)).h(28.0)
            }
        }
    };
    let thumb = ui::addon_tile(&remote.name, 40.0, s.thumb(remote.thumbnail_url.as_deref()), format!("{key}/thumb"));
    col()
        .key(key)
        .skin(&t().skin.panel)
        .padding(16.0)
        .gap(12.0)
        .hover(p.hover)
        .child(
            row()
                .w_full()
                .gap(12.0)
                .child(thumb)
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(3.0)
                        .child(text(remote.name.clone()).font(t().ty.heading).color(p.text).truncate())
                        .child(text(remote.author.clone()).font(t().ty.small).color(p.text_muted).truncate()),
                ),
        )
        .child(text(remote.summary.clone()).font(t().ty.note).color(p.text_dim).wrap().max_lines(2).h(38.0))
        .child(
            row()
                .w_full()
                .h(28.0)
                .gap(10.0)
                .center_y()
                .child(icon(&icons::DOWNLOAD, 12.0).color(p.text_muted))
                .child(text(ui::count(remote.download_count)).font(t().ty.number).color(p.text_dim))
                .child(text(remote.provider.clone()).font(t().ty.small).color(p.text_muted))
                .child(el().grow(1.0))
                .child(action),
        )
}
