//! Get Addons: one field for search text or an install link (debounced), category and sort
//! filters, the popular list, and a detail panel for the clicked result. Results come from every
//! source, one card per addon (F4.8); the detail panel picks the source to install from.

use super::{band, band_note};
use crate::backend::{Found, curseforge_only, source_label};
use crate::icons;
use crate::state::{
    Detail, Fetch, Input, Intent, Menu, Op, Screen, State, Status, dispatch, install_key, single,
};
use crate::theme::{self, Surface, SurfaceExt, pal, t};
use crate::ui::{self, Kind};
use addon_core::discover::Entry;
use addon_core::providers::{curseforge, github};
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

/// One toggle chip per Forever class (medallion + name); the active one uses the theme's "on"
/// segment skin and a gilded ring. Picking one filters by it, picking it again clears it. None if
/// no class is known.
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
        .gap(4.0)
        .center_y()
        .focus_group()
        .child(
            text("CLASS")
                .font(th.ty.eyebrow)
                .color(p.text_muted)
                .pr(8.0),
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
            .h(30.0)
            .pl(3.0)
            .pr(11.0)
            .gap(7.0)
            .center_y()
            .skin(&Surface {
                // A pill around the medallion where the theme rounds chips at all.
                radius: if sk.radius > 0.0 { 15.0 } else { 0.0 },
                ..sk.clone()
            })
            .focus_ring(1.0, 2.0, p.accent_focus);
        if let Some(m) = ui::class_medal(&c.slug, 24.0, on, s.class_icon(&c.slug)) {
            chip = chip.child(m);
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

fn sort_label(sort: Sort) -> Option<&'static str> {
    SORTS.iter().find(|(v, _)| *v == sort).map(|(_, l)| *l)
}

pub fn page(s: &State) -> El {
    let p = pal();
    let ready = s.curseforge_ready();
    let input = s.input();
    let placeholder = if ready {
        "Search, or paste an addon link"
    } else {
        "Search WowUp Hub, or paste an addon link"
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
            ui::tag(&format!("{} link", source_label(source)), ui::Mark::Busy)
                .key("get-addons/link-source"),
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
        let sort = sort_label(s.sort).unwrap_or("");
        // Resets class, category and sort in one go; only there while one is set. The search text
        // stays: it is what the user is looking for, not a filter on it.
        if on && s.filters_set() {
            toolbar = toolbar.child(
                ui::button(
                    "get-addons/clear-filters",
                    "Clear filters",
                    Kind::Ghost,
                    Some(&icons::CLOSE),
                    true,
                    |s, tx| dispatch(s, Intent::ClearFilters, tx),
                )
                .help("Back to all categories, every class and Popularity"),
            );
        }
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
            content = content.child(section(
                format!("INSTALL FROM {}", source_label(source).to_uppercase()),
                None,
            ));
            content = content.child(match &s.link {
                Fetch::Idle | Fetch::Loading => loading("Looking up the link…"),
                Fetch::Failed(e) => error(e),
                Fetch::Ready(r) => grid(s, &[single(r.clone())]),
            });
        }
        _ => {
            if !ready {
                content = content.child(no_key_hint());
            }
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
            // The order shows in the heading once it isn't the default ("WARRIOR | LAST UPDATED").
            let order = sort_label(filter.sort)
                .filter(|_| filter.sort != Sort::default())
                .map(str::to_uppercase);
            content = content.child(section(title, order));
            let cf_only = ready && !filter.is_default() && curseforge_only(&filter);
            if let Fetch::Ready(found) = fetch {
                for (provider, e) in &found.failed {
                    content = content.child(note(
                        "get-addons/source-error",
                        &icons::ALERT,
                        &format!(
                            "{} didn't answer, showing the other sources ({e}).",
                            source_label(provider)
                        ),
                    ));
                }
            }
            let searching = match (ready, cf_only) {
                (false, _) => "Searching WowUp Hub…",
                (true, true) => "Searching CurseForge…",
                (true, false) => "Searching CurseForge and WowUp Hub…",
            };
            content = content.child(match fetch {
                Fetch::Idle | Fetch::Loading => loading(searching),
                Fetch::Failed(e) => error(e),
                Fetch::Ready(Found { entries, .. }) if entries.is_empty() => {
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
                Fetch::Ready(found) => grid(s, &found.entries),
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
            || vec![band_note("Find new addons for World of Warcraft: Forever")],
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

fn section(title: String, order: Option<String>) -> El {
    let eyebrow = |s: String| text(s).font(t().ty.eyebrow).color(pal().text_muted);
    let mut parts = vec![eyebrow(title)];
    parts.extend(order.map(eyebrow));
    ui::meta_line(parts).key("get-addons/section")
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
                .wrap_anywhere()
                .grow(1.0)
                .min_w(0.0),
        )
}

/// Without a key: WowUp Hub still answers, a key adds CurseForge (F7.1).
fn no_key_hint() -> El {
    let p = pal();
    row()
        .key("get-addons/no-key")
        .w_full()
        .max_w(760.0)
        .gap(14.0)
        .px(16.0)
        .py(12.0)
        .center_y()
        .skin(&t().skin.panel)
        .child(ui::badge(&icons::KEY, p.accent))
    .child(
        col()
            .grow(1.0)
            .min_w(0.0)
            .gap(2.0)
            .child(
                text("Add a CurseForge key to search CurseForge too")
                    .font(t().ty.label)
                    .color(p.heading),
            )
            .child(
                text("Results below come from WowUp Hub. Categories and CurseForge updates use your own free key from console.curseforge.com.")
                    .font(t().ty.small)
                    .color(p.text_dim)
                    .wrap(),
            ),
    )
    .child(ui::button(
        "get-addons/open-settings",
        "Open Settings",
        Kind::Secondary,
        None,
        true,
        |s, tx| dispatch(s, Intent::Navigate(Screen::Settings), tx),
    ))
}

/// A quiet one-line note over the results.
fn note(key: &str, glyph: &'static Icon, line: &str) -> El {
    let p = pal();
    row()
        .key(key)
        .w_full()
        .gap(8.0)
        .center_y()
        .child(icon(glyph, 12.0).color(p.text_muted))
        .child(
            text(line.to_string())
                .font(t().ty.small)
                .color(p.text_muted)
                .wrap_anywhere()
                .grow(1.0)
                .min_w(0.0),
        )
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
        .help(match menu {
            Menu::Category => "Show only addons of one category",
            Menu::Sort => "Change the order of the results",
        })
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
                        .wrap_anywhere()
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

fn grid(s: &State, list: &[Entry]) -> El {
    // The detail panel takes a column's width.
    let columns = if s.detail.is_some() { 2 } else { 3 };
    let mut g = col().key("get-addons/results").w_full().gap(16.0);
    for chunk in list.chunks(columns) {
        let mut r = row().w_full().gap(16.0).items_stretch();
        for entry in chunk {
            r = r.child(result_card(s, entry).grow(1.0).min_w(0.0));
        }
        for _ in chunk.len()..columns {
            r = r.child(el().grow(1.0));
        }
        g = g.child(r);
    }
    g
}

/// A result's key part: the id for CurseForge and GitHub (`remote/61284`, `remote/owner/repo`),
/// else prefixed by the provider, since ids are only unique per provider (`remote/wowuphub-12`).
fn card_id(r: &RemoteAddon) -> String {
    match r.provider.as_str() {
        curseforge::NAME | github::NAME => r.external_id.clone(),
        other => format!("{}-{}", ui::slug(other), r.external_id),
    }
}

/// "CurseForge | also on WowUp Hub": the preferred source, then the others.
fn source_line(entry: &Entry) -> El {
    let others: Vec<&str> = entry.sources[1..]
        .iter()
        .map(|r| source_label(&r.provider))
        .collect();
    let also = if others.is_empty() {
        String::new()
    } else {
        format!("also on {}", others.join(", "))
    };
    ui::meta(&[source_label(&entry.preferred().provider), &also])
}

/// Install progress, or the install button (`primary`: the detail panel's wide one).
fn install_action(s: &State, remote: &RemoteAddon, key: &str, label: &str, primary: bool) -> El {
    let p = pal();
    match s.installing.get(&install_key(remote)) {
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
            // No URL: the author allows downloads only through the source's own app.
            let allowed = remote.files.iter().any(|f| !f.download_url.is_empty());
            let b = ui::button(
                format!("{key}/install"),
                label,
                if primary {
                    Kind::Primary
                } else {
                    Kind::Secondary
                },
                Some(&icons::DOWNLOAD),
                allowed,
                move |s, tx| dispatch(s, Intent::Install(Box::new(r.clone())), tx),
            )
            .help(if allowed {
                format!(
                    "Download {} from {} into your AddOns folder",
                    remote.name,
                    source_label(&remote.provider)
                )
            } else {
                format!(
                    "The author only allows downloads through {}'s own app or website",
                    source_label(&remote.provider)
                )
            });
            if primary { b.w_full() } else { b.h(28.0) }
        }
    }
}

/// One addon: the preferred source's facts, the other sources as a line, and install (from the
/// preferred source).
fn result_card(s: &State, entry: &Entry) -> El {
    let p = pal();
    let remote = entry.preferred();
    let key = format!("remote/{}", card_id(remote));
    let action = if s.installed_entry(entry).is_some() {
        ui::tag("Installed", ui::Mark::Good).key(format!("{key}/installed"))
    } else {
        install_action(s, remote, &key, "Install", false)
    };
    let selected = s.detail.as_ref().is_some_and(|d| d.entry.ids == entry.ids);
    let thumb = ui::addon_tile(
        &remote.name,
        40.0,
        s.thumb(remote.thumbnail_url.as_deref()),
        format!("{key}/thumb"),
    );
    let e = entry.clone();
    let mut card = col()
        .key(key.clone())
        .role("button")
        .label(remote.name.clone())
        .skin(&t().skin.panel)
        .padding(16.0)
        .gap(12.0)
        .hover(p.hover)
        .cursor(Cursor::Pointer)
        .on_press_tx(move |s: &mut State, tx: &mut Tx| {
            dispatch(s, Intent::ShowRemote(e.clone()), tx)
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
            .key(format!("{key}/sources"))
            .w_full()
            .gap(6.0)
            .center_y()
            .child(icon(&icons::LAYERS, 12.0).color(p.text_muted))
            .child(source_line(entry).grow(1.0)),
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
/// picked source's long description. Several sources: a picker chooses the one to install from.
fn detail(s: &State, d: &Detail) -> El {
    let p = pal();
    let remote = d.addon();
    let several = d.entry.sources.len() > 1;
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
    let installed = s.installed_entry(&d.entry);
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
                    .help("Download and install the newest version")
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
                .help("Open this addon's details in My Addons")
                .w_full(),
            )
        }
        None if several => install_action(
            s,
            remote,
            &key("addon"),
            &format!("Install from {}", source_label(&remote.provider)),
            true,
        ),
        None => install_action(s, remote, &key("addon"), "Install", true),
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
    if !several {
        facts = facts.child(fact(
            "Source",
            source_label(&remote.provider).to_string(),
            false,
        ));
    }

    let website = remote.url.clone();
    let website = ui::button(
        key("website"),
        &format!("Open on {}", source_label(&remote.provider)),
        Kind::Secondary,
        Some(&icons::EXTERNAL),
        website.is_some(),
        move |s, tx| {
            if let Some(url) = &website {
                dispatch(s, Intent::OpenUrl(url.clone()), tx)
            }
        },
    )
    .help("Open the addon's page in your browser")
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
    let mut body = col().w_full().gap(18.0).padding(24.0).child(
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
    );
    if several {
        body = body.child(source_picker(s, d));
    }
    body = body.child(action);
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

/// The sources that have the addon, one radio row each (name, latest version, downloads); the
/// picked one is what Install uses and what the facts describe.
fn source_picker(s: &State, d: &Detail) -> El {
    let (th, p) = (t(), pal());
    let mut list = col()
        .key("remote-detail/sources")
        .role("radiogroup")
        .label("Install from")
        .w_full()
        .gap(6.0)
        .focus_group()
        .child(text("INSTALL FROM").font(th.ty.eyebrow).color(p.text_muted));
    for (i, r) in d.entry.sources.iter().enumerate() {
        let on = i == d.source;
        let sk = if on {
            &th.skin.segment_on
        } else {
            &th.skin.chip
        };
        let ink = if on { sk.ink } else { p.text_dim };
        let version = r
            .latest_for(s.settings.default_channel)
            .or_else(|| r.latest_for(Channel::Alpha))
            .map_or_else(String::new, |f| f.version.clone());
        list = list.child(
            row()
                .key(format!("remote-detail/source/{}", ui::slug(&r.provider)))
                .role("radio")
                .label(source_label(&r.provider))
                .help(format!(
                    "Install and update from {}",
                    source_label(&r.provider)
                ))
                .checked(on)
                .focusable(true)
                .w_full()
                .h(34.0)
                .pl(10.0)
                .pr(12.0)
                .gap(8.0)
                .center_y()
                .skin(sk)
                .focus_ring(1.0, 2.0, p.accent_focus)
                .hover(p.hover)
                .cursor(Cursor::Pointer)
                .on_press_tx(move |s: &mut State, tx: &mut Tx| {
                    dispatch(s, Intent::PickSource(i), tx)
                })
                .child(icon(&icons::CHECK, 13.0).color(if on { ink } else { ink.alpha(0.0) }))
                .child(
                    text(source_label(&r.provider).to_string())
                        .font(th.ty.label)
                        .color(ink)
                        .truncate()
                        .grow(1.0)
                        .min_w(0.0),
                )
                .child(
                    text(version)
                        .font(th.ty.mono)
                        .color(ink)
                        .truncate()
                        .max_w(120.0),
                )
                .child(
                    text(ui::count(r.download_count))
                        .font(th.ty.small)
                        .color(if on { ink } else { p.text_muted }),
                ),
        );
    }
    list
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
