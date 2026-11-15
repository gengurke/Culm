//! Get Addons: one field for search text or an install link (debounced), category and sort
//! filters, the popular list, and a detail panel for the clicked result.

use super::{band, subtitle};
use crate::icons;
use crate::state::{
    Detail, Fetch, Input, Intent, Menu, Op, Screen, State, Status, dispatch, remote_key,
};
use crate::theme::{self, SurfaceExt, pal, t};
use crate::ui::{self, Kind};
use addon_core::{Category, Channel, RemoteAddon, Sort};
use rune::{Align, Cursor, El, Icon, ScrollSpec, Side, Tx, col, el, icon, row, text};

/// CurseForge's "Class" category: its children are the game's classes.
const CLASS: u32 = 1020;
/// The classes World of Warcraft: Forever has (the 1.x ones), by CurseForge category id:
/// Warrior, Mage, Rogue, Druid, Hunter, Shaman, Priest, Warlock, Paladin.
const FOREVER_CLASSES: [u32; 9] = [1028, 1021, 1027, 1023, 1024, 1025, 1026, 1029, 1022];

fn is_class(c: &Category) -> bool {
    c.id == CLASS || c.parent == Some(CLASS)
}

/// One toggle chip per Forever class (icon + name); the active one uses the theme's "on" segment
/// skin. Picking one filters by it, picking it again clears it. None if no class is known.
fn class_chips(s: &State, list: &[Category], enabled: bool) -> Option<El> {
    let (th, p) = (t(), pal());
    let mut chips = row()
        .key("get-addons/classes")
        .role("radiogroup")
        .label("Class")
        .w_full()
        .flex_wrap()
        .px(theme::PAD)
        .pb(12.0)
        .gap(6.0)
        .center_y()
        .focus_group()
        .child(
            text("CLASS")
                .font(th.ty.eyebrow)
                .color(p.text_muted)
                .pr(6.0),
        );
    let mut any = false;
    for c in list
        .iter()
        .filter(|c| FOREVER_CLASSES.contains(&c.id) && c.parent == Some(CLASS))
    {
        any = true;
        let (id, on) = (c.id, s.category == Some(c.id));
        let sk = if on {
            &th.skin.segment_on
        } else {
            &th.skin.chip
        };
        let ink = if on { sk.ink } else { p.text_dim };
        let mut chip = row()
            .key(format!("class/{id}"))
            .role("radio")
            .label(c.name.clone())
            .checked(on)
            .focusable(enabled)
            .h(26.0)
            .pl(8.0)
            .pr(10.0)
            .gap(6.0)
            .center_y()
            .skin(sk)
            .focus_ring(1.0, 2.0, p.accent_focus);
        if let Some(g) = icons::class_icon(&c.slug) {
            chip = chip.child(icon(g, 14.0).color(ink));
        }
        chip = chip.child(text(c.name.clone()).font(th.ty.small).color(ink));
        chips = chips.child(if enabled {
            chip.hover(p.hover).cursor(Cursor::Pointer).on_press_tx(
                move |s: &mut State, tx: &mut Tx| {
                    let pick = (s.category != Some(id)).then_some(id);
                    dispatch(s, Intent::SetCategory(pick), tx)
                },
            )
        } else {
            chip.opacity(0.45)
        });
    }
    any.then_some(chips)
}

const SORTS: [(Sort, &str); 4] = [
    (Sort::Popularity, "Popularity"),
    (Sort::LastUpdated, "Last updated"),
    (Sort::Name, "Name"),
    (Sort::TotalDownloads, "Total downloads"),
];

pub fn page(s: &State) -> El {
    let p = pal();
    let ready = s.curseforge_ready();
    let input = s.input();
    let placeholder = if ready {
        "Search, or paste a CurseForge / GitHub link"
    } else {
        "Paste a GitHub link (search needs a CurseForge key)"
    };
    let mut field = ui::field(
        "get-addons/search",
        &s.query,
        placeholder,
        Some(if matches!(input, Input::Link(..)) {
            &icons::LINK
        } else {
            &icons::SEARCH
        }),
        |s: &mut State| &mut s.query,
    )
    .on_change(|s, tx| dispatch(s, Intent::Search, tx))
    .on_submit(|s, tx| dispatch(s, Intent::Submit, tx));
    if let Input::Link(_, source) = input {
        field = field.trailing(
            ui::tag(&format!("{source} link"), ui::Mark::Busy).key("get-addons/link-source"),
        );
    }
    let mut toolbar = row()
        .key("get-addons/toolbar")
        .w_full()
        .px(theme::PAD)
        .py(14.0)
        .gap(8.0)
        .center_y()
        .child(field.build().grow(10.0).min_w(220.0).max_w(560.0))
        .child(el().grow(1.0).min_w(8.0));
    if ready {
        // A link installs one addon: the filters don't apply to it.
        let on = !matches!(input, Input::Link(..));
        // A class is chosen with its chip, not in the dropdown.
        let label = match s.category.and_then(|id| s.category_by_id(id)) {
            Some(c) if !is_class(c) => c.name.clone(),
            _ => "All categories".to_string(),
        };
        let glyph = &icons::FILTER;
        let sort = SORTS
            .iter()
            .find(|(v, _)| *v == s.sort)
            .map_or("", |(_, l)| l);
        toolbar = toolbar
            .child(
                dropdown(s, Menu::Category, "get-addons/category", &label, glyph, on).max_w(230.0),
            )
            .child(dropdown(
                s,
                Menu::Sort,
                "get-addons/sort",
                sort,
                &icons::UPDOWN,
                on,
            ));
    }

    let classes = match &s.categories {
        Fetch::Ready(list) if ready => class_chips(s, list, !matches!(input, Input::Link(..))),
        _ => None,
    };
    let toolbar = match classes {
        Some(chips) => col()
            .w_full()
            .skin(&t().skin.toolbar)
            .child(toolbar.pb(10.0))
            .child(chips),
        None => col().w_full().skin(&t().skin.toolbar).child(toolbar),
    };

    let mut content = col().w_full().gap(16.0).px(theme::PAD).py(24.0);
    match input {
        Input::Link(_, source) => {
            content = content.child(section(format!("INSTALL FROM {}", source.to_uppercase())));
            content = content.child(match &s.link {
                Fetch::Idle | Fetch::Loading => loading("Looking up the link…"),
                Fetch::Failed(e) => error(e),
                Fetch::Ready(r) => grid(s, std::slice::from_ref(r)),
            });
        }
        _ if !ready => content = content.child(no_key_card()),
        _ => {
            let filter = s.search_filter();
            let category = filter
                .category
                .and_then(|id| s.category_by_id(id))
                .map(|c| c.name.to_uppercase());
            let (title, fetch) = match (filter.text.as_str(), category) {
                _ if filter.is_default() => ("POPULAR FOR FOREVER".to_string(), &s.featured),
                ("", Some(c)) => (c, &s.results),
                ("", None) => ("ALL ADDONS".to_string(), &s.results),
                (q, Some(c)) => (
                    format!("RESULTS FOR “{}” IN {c}", q.to_uppercase()),
                    &s.results,
                ),
                (q, None) => (format!("RESULTS FOR “{}”", q.to_uppercase()), &s.results),
            };
            content = content.child(section(title));
            content = content.child(match fetch {
                Fetch::Idle | Fetch::Loading => loading("Searching CurseForge…"),
                Fetch::Failed(e) => error(e),
                Fetch::Ready(list) if list.is_empty() => {
                    let (line, hint) = if filter.text.is_empty() {
                        ("No Forever addons in this category".to_string(), "Pick another category above.")
                    } else {
                        (
                            format!("No addons match “{}”", filter.text),
                            "Try a shorter name, another category, or paste the addon's link above.",
                        )
                    };
                    col()
                        .key("get-addons/none")
                        .py(32.0)
                        .gap(6.0)
                        .child(text(line).font(t().ty.body).color(p.text_dim))
                        .child(text(hint).font(t().ty.small).color(p.text_muted))
                }
                Fetch::Ready(list) => grid(s, list),
            });
        }
    }
    let mut body = row()
        .key("get-addons/body")
        .grow(1.0)
        .min_h(0.0)
        .w_full()
        .child(
            col()
                .key("get-addons/scroll")
                .grow(1.0)
                .min_w(0.0)
                .h_full()
                .min_h(0.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(content),
        );
    if let Some(d) = &s.detail {
        body = body.child(detail(s, d));
    }
    let mut page = col()
        .w_full()
        .h_full()
        .child(band(
            "Get Addons",
            subtitle("Find new addons for World of Warcraft: Forever"),
            vec![],
        ))
        .child(toolbar)
        .child(body);
    if let Some(menu) = s.menu.filter(|_| ready) {
        // A click anywhere else closes the menu.
        page = page
            .child(
                el().key("get-addons/menu-backdrop")
                    .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                    .on_press(|s: &mut State| s.menu = None),
            )
            .child(menu_panel(s, menu));
    }
    page
}

fn section(title: String) -> El {
    text(title)
        .key("get-addons/section")
        .font(t().ty.eyebrow)
        .color(pal().text_muted)
}

fn loading(label: &str) -> El {
    let p = pal();
    row()
        .key("get-addons/loading")
        .py(24.0)
        .gap(10.0)
        .center_y()
        .child(ui::spinner(14.0, p.accent))
        .child(text(label).font(t().ty.body).color(p.text_dim))
}

fn error(e: &str) -> El {
    let p = pal();
    row()
        .key("get-addons/error")
        .gap(8.0)
        .child(icon(&icons::ALERT, 14.0).color(p.danger))
        .child(
            text(e.to_string())
                .font(t().ty.body)
                .color(p.danger)
                .wrap()
                .grow(1.0)
                .min_w(0.0),
        )
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
                    text("Search, categories and CurseForge updates use your own free key from console.curseforge.com. Add it in Settings. GitHub links work without one: paste one above.")
                        .font(t().ty.note)
                        .color(p.text_dim)
                        .wrap(),
                ),
        )
        .child(ui::button("get-addons/open-settings", "Open Settings", Kind::Primary, None, true, |s, tx| dispatch(s, Intent::Navigate(Screen::Settings), tx)))
}

/// A filter trigger: glyph, current value and a chevron; opens `menu`.
fn dropdown(
    s: &State,
    menu: Menu,
    key: &str,
    label: &str,
    glyph: &'static Icon,
    enabled: bool,
) -> El {
    let (th, p) = (t(), pal());
    let sk = &th.skin.secondary;
    let open = s.menu == Some(menu);
    let mut b = row()
        .key(key)
        .role("button")
        .label(label)
        .expanded(open)
        .focusable(enabled)
        .h(theme::CONTROL_H)
        .min_w(0.0)
        .pl(12.0)
        .pr(10.0)
        .gap(8.0)
        .center_y()
        .skin(sk)
        .focus_ring(1.0, 2.0, p.accent_focus)
        .child(icon(glyph, 14.0).color(sk.ink))
        .child(
            text(label.to_string())
                .font(th.ty.label)
                .color(sk.ink)
                .truncate()
                .min_w(0.0),
        )
        .child(icon(&icons::CHEVRON_DOWN, 14.0).color(p.text_muted));
    if open {
        b = b.border(1.0, p.accent_focus);
    }
    if enabled {
        b.cursor(Cursor::Pointer).on_press(move |s: &mut State| {
            s.menu = if s.menu == Some(menu) {
                None
            } else {
                Some(menu)
            };
        })
    } else {
        b.disabled(true).opacity(0.45)
    }
}

fn menu_item(
    key: String,
    label: &str,
    on: bool,
    glyph: Option<&'static Icon>,
    indent: bool,
    f: impl Fn(&mut State, &mut Tx) + 'static,
) -> El {
    let p = pal();
    let mut r = row()
        .key(key)
        .role("menuitemradio")
        .label(label)
        .checked(on)
        .w_full()
        .h(32.0)
        .pl(if indent { 34.0 } else { 14.0 })
        .pr(14.0)
        .gap(10.0)
        .center_y()
        .hover(p.hover)
        .cursor(Cursor::Pointer)
        .on_press_tx(f)
        .child(icon(&icons::CHECK, 14.0).color(if on { p.accent } else { p.accent.alpha(0.0) }));
    if let Some(g) = glyph {
        r = r.child(icon(g, 15.0).color(if on { p.accent } else { p.text_dim }));
    }
    r.child(
        text(label.to_string())
            .font(t().ty.small)
            .color(if on { p.text } else { p.text_dim })
            .truncate()
            .grow(1.0)
            .min_w(0.0),
    )
}

/// The open dropdown, under its trigger.
fn menu_panel(s: &State, menu: Menu) -> El {
    let mut items: Vec<El> = Vec::new();
    let (anchor, width) = match menu {
        Menu::Sort => {
            for (sort, label) in SORTS {
                items.push(menu_item(
                    format!("sort/{}", ui::slug(label)),
                    label,
                    s.sort == sort,
                    None,
                    false,
                    move |s, tx| dispatch(s, Intent::SetSort(sort), tx),
                ));
            }
            ("get-addons/sort", 200.0)
        }
        Menu::Category => {
            items.push(menu_item(
                "category/all".into(),
                "All categories",
                s.category.is_none(),
                None,
                false,
                |s, tx| dispatch(s, Intent::SetCategory(None), tx),
            ));
            let list: &[Category] = match &s.categories {
                Fetch::Ready(list) => list,
                _ => &[],
            };
            let item = |c: &Category| {
                let id = c.id;
                menu_item(
                    format!("category/{id}"),
                    &c.name,
                    s.category == Some(id),
                    None,
                    c.parent.is_some(),
                    move |s, tx| dispatch(s, Intent::SetCategory(Some(id)), tx),
                )
            };
            for top in list.iter().filter(|c| c.parent.is_none() && !is_class(c)) {
                items.push(item(top));
                items.extend(list.iter().filter(|c| c.parent == Some(top.id)).map(item));
            }
            if let Fetch::Loading | Fetch::Idle = s.categories {
                items.push(
                    row()
                        .h(32.0)
                        .px(14.0)
                        .gap(10.0)
                        .center_y()
                        .child(ui::spinner(12.0, pal().accent))
                        .child(
                            text("Loading categories…")
                                .font(t().ty.small)
                                .color(pal().text_muted),
                        ),
                );
            }
            if let Fetch::Failed(e) = &s.categories {
                items.push(
                    text(format!("Categories unavailable: {e}"))
                        .font(t().ty.small)
                        .color(pal().danger)
                        .wrap()
                        .px(14.0)
                        .py(6.0),
                );
            }
            ("get-addons/category", 260.0)
        }
    };
    col()
        .key("get-addons/menu")
        .role("menu")
        .w(width)
        .min_h(0.0)
        .py(6.0)
        .skin(&t().skin.menu)
        .child(
            col()
                .w_full()
                .shrink()
                .min_h(0.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(col().w_full().children(items)),
        )
        .anchored_fit(anchor, Side::Bottom, Align::Start, 4.0)
}

fn grid(s: &State, list: &[RemoteAddon]) -> El {
    // The detail panel takes a column's width.
    let columns = if s.detail.is_some() { 2 } else { 3 };
    let mut g = col().key("get-addons/results").w_full().gap(16.0);
    for chunk in list.chunks(columns) {
        let mut r = row().w_full().gap(16.0).items_stretch();
        for remote in chunk {
            r = r.child(result_card(s, remote).grow(1.0).min_w(0.0));
        }
        for _ in chunk.len()..columns {
            r = r.child(el().grow(1.0));
        }
        g = g.child(r);
    }
    g
}

/// Install progress, or the install button (`primary`: the detail panel's wide one).
fn install_action(s: &State, remote: &RemoteAddon, key: &str, primary: bool) -> El {
    let p = pal();
    match s.installing.get(&remote.external_id) {
        Some(Op::Downloading(done)) => col()
            .key(format!("{key}/progress"))
            .w(if primary { 200.0 } else { 120.0 })
            .gap(6.0)
            .child(
                text(format!("Installing… {:.0}%", done * 100.0))
                    .font(t().ty.small)
                    .color(p.text_dim),
            )
            .child(ui::progress_line(*done, 2.0)),
        Some(_) => row()
            .key(format!("{key}/progress"))
            .gap(8.0)
            .center_y()
            .child(ui::spinner(12.0, p.accent))
            .child(text("Installing…").font(t().ty.small).color(p.text_dim)),
        None => {
            let r = remote.clone();
            let b = ui::button(
                format!("{key}/install"),
                "Install",
                if primary {
                    Kind::Primary
                } else {
                    Kind::Secondary
                },
                Some(&icons::DOWNLOAD),
                true,
                move |s, tx| dispatch(s, Intent::Install(r.clone()), tx),
            );
            if primary { b.w_full() } else { b.h(28.0) }
        }
    }
}

fn result_card(s: &State, remote: &RemoteAddon) -> El {
    let p = pal();
    let key = format!("remote/{}", remote.external_id);
    let action = if s.is_installed(remote) {
        ui::tag("Installed", ui::Mark::Good).key(format!("{key}/installed"))
    } else {
        install_action(s, remote, &key, false)
    };
    let selected = s
        .detail
        .as_ref()
        .is_some_and(|d| remote_key(&d.addon) == remote_key(remote));
    let thumb = ui::addon_tile(
        &remote.name,
        40.0,
        s.thumb(remote.thumbnail_url.as_deref()),
        format!("{key}/thumb"),
    );
    let r = remote.clone();
    let mut card = col()
        .key(key)
        .role("button")
        .label(remote.name.clone())
        .skin(&t().skin.panel)
        .padding(16.0)
        .gap(12.0)
        .hover(p.hover)
        .cursor(Cursor::Pointer)
        .on_press_tx(move |s: &mut State, tx: &mut Tx| {
            dispatch(s, Intent::ShowRemote(r.clone()), tx)
        });
    if selected {
        card = card.border(1.0, p.accent);
    }
    card.child(
        row().w_full().gap(12.0).child(thumb).child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(3.0)
                .child(
                    text(remote.name.clone())
                        .font(t().ty.heading)
                        .color(p.text)
                        .truncate(),
                )
                .child(
                    text(remote.author.clone())
                        .font(t().ty.small)
                        .color(p.text_muted)
                        .truncate(),
                ),
        ),
    )
    .child(
        text(remote.summary.clone())
            .font(t().ty.note)
            .color(p.text_dim)
            .wrap()
            .max_lines(2)
            .h(38.0),
    )
    .child(
        row()
            .w_full()
            .h(28.0)
            .gap(10.0)
            .center_y()
            .child(icon(&icons::DOWNLOAD, 12.0).color(p.text_muted))
            .child(
                text(ui::count(remote.download_count))
                    .font(t().ty.number)
                    .color(p.text_dim),
            )
            .child(
                text(remote.provider.clone())
                    .font(t().ty.small)
                    .color(p.text_muted),
            )
            .child(el().grow(1.0))
            .child(action),
    )
}

/// `2026-11-14T00:40:32Z` → `Nov 14, 2026` (anything else as given).
fn date(rfc3339: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mut parts = rfc3339.get(..10).unwrap_or(rfc3339).split('-');
    let (Some(y), Some(m), Some(d)) = (parts.next(), parts.next(), parts.next()) else {
        return rfc3339.to_string();
    };
    match (m.parse::<usize>(), d.parse::<u32>()) {
        (Ok(m @ 1..=12), Ok(d)) => format!("{} {d}, {y}", MONTHS[m - 1]),
        _ => rfc3339.to_string(),
    }
}

fn channel_name(c: Channel) -> &'static str {
    match c {
        Channel::Stable => "Stable",
        Channel::Beta => "Beta",
        Channel::Alpha => "Alpha",
    }
}

/// The clicked result: what My Addons' detail panel shows for an installed addon, plus the
/// provider's long description.
fn detail(s: &State, d: &Detail) -> El {
    let p = pal();
    let remote = &d.addon;
    let key = |part: &str| format!("remote-detail/{part}");
    let fact = |label: &str, value: String, mono: bool| {
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
    };
    let installed = s.installed(remote);
    let state = match installed {
        Some(a) if a.status() == Status::UpdateAvailable => {
            Some(ui::tag("Update available", ui::Mark::Update))
        }
        Some(_) => Some(ui::tag("Installed", ui::Mark::Good)),
        None => None,
    };
    let action = match installed {
        Some(a) => {
            let id = a.addon.id.clone();
            let update = a.status() == Status::UpdateAvailable;
            let show = {
                let id = id.clone();
                move |s: &mut State, tx: &mut Tx| {
                    s.selected = Some(id.clone());
                    s.confirm_remove = None;
                    dispatch(s, Intent::Navigate(Screen::MyAddons), tx);
                }
            };
            let mut c = col().w_full().gap(8.0);
            if update {
                c = c.child(
                    ui::button(
                        key("update"),
                        "Update now",
                        Kind::Primary,
                        Some(&icons::DOWNLOAD),
                        true,
                        move |s, tx| dispatch(s, Intent::Update(id.clone()), tx),
                    )
                    .w_full(),
                );
            }
            c.child(
                ui::button(
                    key("show"),
                    "Show in My Addons",
                    Kind::Secondary,
                    Some(&icons::PACKAGE),
                    true,
                    show,
                )
                .w_full(),
            )
        }
        None => install_action(s, remote, &key("addon"), true),
    };
    let file = remote
        .latest_for(s.settings.default_channel)
        .or_else(|| remote.latest_for(Channel::Alpha));
    let dash = || "—".to_string();
    let mut facts = col()
        .w_full()
        .child(fact("Downloads", ui::count(remote.download_count), false))
        .child(fact(
            "Updated",
            remote.updated_at.as_deref().map_or_else(dash, date),
            false,
        ))
        .child(fact(
            "Latest file",
            file.map_or_else(dash, |f| f.version.clone()),
            true,
        ));
    if let Some(f) = file {
        facts = facts
            .child(fact(
                "Released",
                f.released_at.as_deref().map_or_else(dash, date),
                false,
            ))
            .child(fact("Channel", channel_name(f.channel).into(), false));
        if !f.game_versions.is_empty() {
            facts = facts.child(fact("Game version", f.game_versions.join(", "), true));
        }
    }
    if !remote.categories.is_empty() {
        facts = facts.child(fact("Categories", remote.categories.join(", "), false));
    }
    facts = facts.child(fact("Source", remote.provider.clone(), false));

    let website = remote.url.clone();
    let website = ui::button(
        key("website"),
        &format!("Open on {}", remote.provider),
        Kind::Secondary,
        Some(&icons::EXTERNAL),
        website.is_some(),
        move |s, tx| {
            if let Some(url) = &website {
                dispatch(s, Intent::OpenUrl(url.clone()), tx)
            }
        },
    )
    .w_full();

    let about = match &d.description {
        Fetch::Idle | Fetch::Loading => row()
            .key(key("description-loading"))
            .gap(8.0)
            .center_y()
            .child(ui::spinner(12.0, p.accent))
            .child(
                text("Loading description…")
                    .font(t().ty.small)
                    .color(p.text_muted),
            ),
        Fetch::Ready(Some(body)) => description(body).key(key("description")),
        Fetch::Ready(None) => text("No description beyond the summary.")
            .font(t().ty.small)
            .color(p.text_muted),
        Fetch::Failed(e) => text(format!("Could not load the description: {e}"))
            .key(key("description-error"))
            .font(t().ty.small)
            .color(p.text_muted)
            .wrap(),
    };

    let mut head = col().grow(1.0).min_w(0.0).gap(4.0).child(
        text(remote.name.clone())
            .font(t().ty.title)
            .color(p.heading)
            .wrap(),
    );
    head = head.child(
        text(remote.author.clone())
            .font(t().ty.small)
            .color(p.text_muted)
            .truncate(),
    );
    if let Some(b) = state {
        head = head.child(row().pt(6.0).child(b));
    }
    let mut body = col()
        .w_full()
        .gap(18.0)
        .padding(24.0)
        .child(
            row()
                .w_full()
                .gap(14.0)
                .items_start()
                .child(ui::addon_tile(
                    &remote.name,
                    48.0,
                    s.thumb(remote.thumbnail_url.as_deref()),
                    key("thumb"),
                ))
                .child(head)
                .child(ui::icon_button(
                    key("close"),
                    "Close details",
                    &icons::CLOSE,
                    |s, tx| dispatch(s, Intent::CloseRemote, tx),
                )),
        )
        .child(action);
    if !remote.summary.is_empty() {
        body = body.child(
            text(remote.summary.clone())
                .font(t().ty.note)
                .color(p.text_dim)
                .wrap(),
        );
    }
    body = body.child(facts).child(website).child(
        col()
            .w_full()
            .gap(10.0)
            .child(text("ABOUT").font(t().ty.eyebrow).color(p.text_muted))
            .child(about),
    );
    col()
        .key("remote-detail")
        .w(340.0)
        .h_full()
        .min_h(0.0)
        .skin(&t().skin.detail)
        .child(
            col()
                .grow(1.0)
                .min_h(0.0)
                .w_full()
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(body),
        )
}

/// Plain-text description: paragraphs, and `• ` lines as a list with a hanging indent.
fn description(body: &str) -> El {
    let p = pal();
    let line = |l: &str| match l.strip_prefix("• ") {
        Some(item) => row()
            .w_full()
            .gap(8.0)
            .child(text("•").font(t().ty.note).color(p.text_muted))
            .child(
                text(item.to_string())
                    .font(t().ty.note)
                    .color(p.text_dim)
                    .wrap()
                    .grow(1.0)
                    .min_w(0.0),
            ),
        None => text(l.to_string())
            .font(t().ty.note)
            .color(p.text_dim)
            .wrap()
            .w_full(),
    };
    let mut c = col().w_full().gap(12.0);
    for para in body.split("\n\n") {
        c = c.child(col().w_full().gap(4.0).children(para.lines().map(line)));
    }
    c
}
