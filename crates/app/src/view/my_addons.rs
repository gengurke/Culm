//! My Addons: toolbar, the installed list (virtual rows) and the selected addon's detail panel.

use super::{band, band_note};
use crate::backend::SortBy;
use crate::icons;
use crate::state::{AddonUi, Intent, Job, Op, Screen, State, Status, dispatch};
use crate::theme::{self, StyleExt, SurfaceExt, pal, t};
use crate::ui::{self, Kind, Mark};
use addon_core::{Channel, IgnoreReason};
use rune::{Cursor, El, GrowthSpec, ScrollSpec, Tx, col, el, icon, rich, row, text};
use std::borrow::Cow;

const VERSION_W: f32 = 180.0;
const GAME_W: f32 = 72.0;
const STATUS_W: f32 = 140.0;
const ACTION_W: f32 = 92.0;

pub fn page(s: &State) -> El {
    let n = s.update_count();
    let installed = s.addons.len();
    let unchecked = s.not_checked_count();
    // Without a key CurseForge addons only carry the imported last known release: don't claim more.
    let cf_unchecked = !s.curseforge_ready()
        && s.addons.iter().any(|a| {
            a.addon.ignored.is_none() && a.addon.provider.as_deref() == Some("CurseForge")
        });
    let state = match (n, unchecked) {
        (1.., _) => format!("{n} update{} available", if n == 1 { "" } else { "s" }),
        _ if cf_unchecked => "CurseForge updates not checked".to_string(),
        (0, 1..) => format!("{unchecked} not checked"),
        (0, 0) => "Everything up to date".to_string(),
    };
    let reading = installed == 0 && s.job != Job::Idle;
    let meta = || match s.installation() {
        Some(i) if reading => vec![band_note(format!("Reading {}…", s.inst_name(i)))],
        Some(_) => {
            let count = format!("{installed} addon{}", if installed == 1 { "" } else { "s" });
            let state = band_note(state);
            vec![
                band_note(count),
                if n > 0 {
                    state
                        .font(t().ty.note.weight_of(600.0))
                        .color(pal().warning)
                } else {
                    state
                },
            ]
        }
        None => vec![],
    };
    let update_all = ui::button(
        "my-addons/update-all",
        &if n > 0 {
            format!("Update all ({n})")
        } else {
            "Update all".into()
        },
        Kind::Primary,
        Some(&icons::DOWNLOAD),
        n > 0,
        |s, tx| dispatch(s, Intent::UpdateAll, tx),
    );
    let body = if s.addons.is_empty() && s.job != Job::Idle {
        busy(s)
    } else {
        let mut main = row()
            .key("my-addons/body")
            .grow(1.0)
            .min_h(0.0)
            .w_full()
            .child(list(s));
        if let Some(a) = s.selected.as_deref().and_then(|id| s.addon(id)) {
            main = main.child(detail(s, a));
        }
        main
    };
    col()
        .w_full()
        .h_full()
        .child(band("My Addons", meta, vec![update_all]))
        .child(toolbar(s))
        .child(body)
}

fn toolbar(s: &State) -> El {
    let p = pal();
    let idle = s.job == Job::Idle;
    let status = match s.job {
        Job::Scanning => Some("Scanning AddOns folder…"),
        Job::CheckingUpdates => Some("Checking for updates…"),
        _ => None,
    };
    let mut bar = row()
        .key("my-addons/toolbar")
        .w_full()
        .px(theme::PAD)
        .py(14.0)
        .gap(8.0)
        .center_y()
        .skin(&t().skin.toolbar)
        .child(
            ui::field(
                "my-addons/filter",
                &s.filter,
                "Filter installed addons",
                Some(&icons::SEARCH),
                |s: &mut State| &mut s.filter,
            )
            .trailing(ui::kbd("/"))
            .build()
            .w(300.0),
        )
        .child(el().grow(1.0));
    // Without a key, CurseForge addons keep the imported latest versions: say why.
    if !s.curseforge_ready()
        && s.addons
            .iter()
            .any(|a| a.addon.provider.as_deref() == Some("CurseForge"))
    {
        bar = bar.child(
            ui::button(
                "my-addons/no-key",
                "Add a CurseForge key for update checks",
                Kind::Ghost,
                Some(&icons::KEY),
                true,
                |s, tx| dispatch(s, Intent::Navigate(Screen::Settings), tx),
            )
            .help(
                "CurseForge addons are only checked for updates with your own API key (Settings).",
            ),
        );
    }
    if let Some(label) = status {
        bar = bar.child(
            row()
                .key("my-addons/job")
                .gap(8.0)
                .center_y()
                .pr(8.0)
                .child(ui::spinner(14.0, p.accent))
                .child(text(label).font(t().ty.small).color(p.text_dim)),
        );
    }
    bar.child(ui::button(
        "my-addons/check",
        "Check for updates",
        Kind::Secondary,
        Some(&icons::REFRESH),
        idle,
        |s, tx| dispatch(s, Intent::CheckUpdates, tx),
    ))
    .child(ui::button(
        "my-addons/rescan",
        "Rescan",
        Kind::Ghost,
        Some(&icons::FOLDER),
        idle,
        |s, tx| dispatch(s, Intent::Rescan, tx),
    ))
}

fn busy(s: &State) -> El {
    let p = pal();
    let label = if s.job == Job::CheckingUpdates {
        "Checking for updates…"
    } else {
        "Scanning your AddOns folder…"
    };
    col()
        .key("my-addons/busy")
        .grow(1.0)
        .w_full()
        .center()
        .gap(12.0)
        .child(ui::spinner(20.0, p.accent))
        .child(text(label).font(t().ty.body).color(p.text_dim))
}

fn compact(s: &State) -> bool {
    s.selected.is_some()
}

/// A column header. Sortable ones (`by`) are buttons: a press orders by the column or flips the
/// order, and the active one shows a chevron (up: ascending). Version is not sortable: versions are
/// each author's own strings, and whether one is outdated is what Status sorts by.
fn column(s: &State, label: &str, by: Option<SortBy>) -> El {
    let (th, p) = (t(), pal());
    let caption = text(label.to_uppercase()).font(th.ty.eyebrow);
    let Some(by) = by else {
        return row().center_y().child(caption.color(p.text_muted));
    };
    let order = s.settings.addon_order;
    let on = order.by == by;
    let chevron = if on && order.desc {
        &icons::CHEVRON_DOWN
    } else {
        &icons::CHEVRON_UP
    };
    let dir = match (on, order.desc) {
        (true, false) => ", ascending",
        (true, true) => ", descending",
        (false, _) => "",
    };
    row()
        .key(format!("addons/sort/{}", ui::slug(label)))
        .role("button")
        .label(format!("Sort by {label}{dir}"))
        .focusable(true)
        .h(24.0)
        .gap(4.0)
        .center_y()
        .cursor(Cursor::Pointer)
        .focus_ring(1.0, 2.0, p.accent_focus)
        .on_press_tx(move |s: &mut State, tx: &mut Tx| dispatch(s, Intent::SortAddons(by), tx))
        .child(caption.color(if on { p.text } else { p.text_muted }))
        .child(icon(chevron, 12.0).color(if on { p.accent } else { p.accent.alpha(0.0) }))
}

fn header(s: &State) -> El {
    let mut r = row()
        .key("my-addons/header")
        .w_full()
        .h(36.0)
        .px(theme::PAD - 8.0)
        .gap(12.0)
        .center_y()
        .skin(&t().skin.list_header)
        .child(el().w(36.0))
        .child(
            row()
                .grow(1.0)
                .child(column(s, "Addon", Some(SortBy::Name))),
        )
        .child(row().w(VERSION_W).child(column(s, "Version", None)));
    if !compact(s) {
        r = r.child(row().w(GAME_W).child(column(s, "Game", Some(SortBy::Game))));
    }
    r.child(
        row()
            .w(STATUS_W)
            .child(column(s, "Status", Some(SortBy::Status))),
    )
    .child(el().w(ACTION_W))
}

fn list(s: &State) -> El {
    let p = pal();
    let visible = s.visible_addons();
    let rows: El = if visible.is_empty() {
        col()
            .key("my-addons/empty")
            .w_full()
            .py(48.0)
            .center()
            .gap(6.0)
            .child(
                text(format!("No addons match “{}”", s.filter.text.trim()))
                    .font(t().ty.body)
                    .color(p.text_dim),
            )
            .child(
                text("Clear the filter to see everything.")
                    .font(t().ty.small)
                    .color(p.text_muted),
            )
    } else {
        col()
            .key("my-addons/rows")
            .w_full()
            .growth(GrowthSpec::default())
            .virtual_rows(
                visible.len(),
                theme::ROW_H,
                |s: &State, i| Cow::Owned(format!("addon/{}", s.visible_addons()[i].addon.id)),
                |s: &State, i| addon_row(s, s.visible_addons()[i]),
            )
    };
    col()
        .key("my-addons/list")
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .min_h(0.0)
        .child(header(s))
        .child(
            col()
                .key("my-addons/scroll")
                .grow(1.0)
                .min_h(0.0)
                .w_full()
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(rows),
        )
}

fn addon_row(s: &State, a: &AddonUi) -> El {
    let (th, p) = (t(), pal());
    let addon = &a.addon;
    let id = addon.id.clone();
    let selected = s.selected.as_deref() == Some(&addon.id);
    let provider = addon.provider.as_deref().unwrap_or("Unmatched");
    let mut r = row()
        .key(format!("addon/{id}"))
        .role("row")
        .label(addon.name.clone())
        .w_full()
        .h(theme::ROW_H)
        .px(theme::PAD - 8.0)
        .gap(12.0)
        .center_y()
        .skin(if selected {
            &th.skin.row_selected
        } else {
            &th.skin.row
        })
        .cursor(Cursor::Pointer)
        .on_press({
            let id = id.clone();
            move |s: &mut State| {
                s.confirm_remove = None;
                s.selected = if s.selected.as_deref() == Some(&id) {
                    None
                } else {
                    Some(id.clone())
                };
            }
        });
    if selected {
        r = r.child(
            el().absolute(0.0, 0.0)
                .w(2.0)
                .h(theme::ROW_H)
                .fill(p.accent),
        );
    }
    r = r
        .child(ui::addon_tile(
            &addon.name,
            36.0,
            s.thumb(addon.thumbnail_url.as_deref()),
            format!("addon/{id}/thumb"),
        ))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(3.0)
                .child(
                    text(addon.name.clone())
                        .font(t().ty.strong)
                        .color(p.text)
                        .truncate(),
                )
                .child(ui::meta(&[&addon.author, provider])),
        )
        .child(version_cell(a).w(VERSION_W));
    if !compact(s) {
        r = r.child(
            text(addon.game_version.clone().unwrap_or_else(|| "—".into()))
                .font(t().ty.mono)
                .color(p.text_muted)
                .w(GAME_W),
        );
    }
    r.child(status_cell(a).w(STATUS_W))
        .child(row().w(ACTION_W).justify_end().child(action_cell(a)))
}

fn version_cell(a: &AddonUi) -> El {
    let p = pal();
    let installed = a
        .addon
        .installed_version
        .clone()
        .unwrap_or_else(|| "—".into());
    match (&a.addon.latest, a.status()) {
        (Some(latest), Status::UpdateAvailable | Status::Working | Status::Error)
            if latest.version != installed =>
        {
            let s = format!("{installed} → {}", latest.version);
            let start = installed.len() + " → ".len();
            rich(
                s.clone(),
                vec![
                    rune::Span::new(0, installed.len()).color(p.text_dim),
                    rune::Span::new(start, s.len()).color(p.accent),
                ],
            )
            .font(t().ty.mono)
            .color(p.text_muted)
            .truncate()
        }
        _ => text(installed)
            .font(t().ty.mono)
            .color(p.text_dim)
            .truncate(),
    }
}

/// What the status says and how it is marked.
fn status_mark(a: &AddonUi) -> (&'static str, Mark) {
    match a.status() {
        Status::UpToDate => ("Up to date", Mark::Good),
        Status::NotChecked => ("Not checked", Mark::Muted),
        Status::UpdateAvailable => ("Update available", Mark::Update),
        Status::Ignored if a.addon.ignored == Some(IgnoreReason::GitRepo) => {
            ("Git checkout", Mark::Off)
        }
        Status::Ignored => ("Ignored", Mark::Off),
        Status::Unknown => ("Unknown", Mark::Muted),
        Status::Error => ("Error", Mark::Bad),
        Status::Working => ("Working", Mark::Busy),
    }
}

pub(crate) fn status_badge(a: &AddonUi) -> El {
    let (label, mark) = status_mark(a);
    let b = ui::tag(label, mark);
    if a.status() == Status::NotChecked {
        b.help("No update check has answered for this addon yet (CurseForge needs your API key).")
    } else {
        b
    }
}

fn status_cell(a: &AddonUi) -> El {
    let p = pal();
    match &a.op {
        Some(Op::Downloading(done)) => col()
            .key(format!("addon/{}/progress", a.addon.id))
            .gap(6.0)
            .child(
                row()
                    .justify_between()
                    .child(text("Updating").font(t().ty.small).color(p.text_dim))
                    .child(
                        text(format!("{:.0}%", done * 100.0))
                            .font(t().ty.number)
                            .color(p.text_dim),
                    ),
            )
            .child(ui::progress_line(*done, 2.0)),
        Some(Op::Installing) => row()
            .gap(8.0)
            .center_y()
            .child(ui::spinner(12.0, p.accent))
            .child(text("Installing…").font(t().ty.small).color(p.text_dim)),
        Some(Op::Removing) => row()
            .gap(8.0)
            .center_y()
            .child(ui::spinner(12.0, p.danger))
            .child(text("Removing…").font(t().ty.small).color(p.text_dim)),
        Some(Op::Failed(e)) => row().help(e.clone()).child(status_badge(a)),
        None => row().child(status_badge(a)),
    }
}

fn action_cell(a: &AddonUi) -> El {
    let id = a.addon.id.clone();
    match a.status() {
        Status::UpdateAvailable => ui::button(
            format!("addon/{id}/update"),
            "Update",
            Kind::Secondary,
            None,
            true,
            move |s, tx| dispatch(s, Intent::Update(id.clone()), tx),
        )
        .h(28.0),
        Status::Error => ui::button(
            format!("addon/{id}/retry"),
            "Retry",
            Kind::Ghost,
            Some(&icons::REFRESH),
            true,
            move |s, tx| dispatch(s, Intent::Update(id.clone()), tx),
        )
        .h(28.0),
        _ => el(),
    }
}

fn detail(s: &State, a: &AddonUi) -> El {
    let p = pal();
    let addon = &a.addon;
    let id = addon.id.clone();
    let key = |part: &str| format!("detail/{part}");
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
                    .truncate(),
            )
    };
    let managed = addon.external_id.is_some();
    let ignored = addon.ignored.is_some();
    let busy = a.op.as_ref().is_some_and(Op::busy);

    let mut actions = col().gap(8.0).w_full();
    if a.status() == Status::UpdateAvailable {
        let id = id.clone();
        actions = actions.child(
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
    let row2 = |a: El, b: El| {
        row()
            .w_full()
            .gap(8.0)
            .child(a.grow(1.0))
            .child(b.grow(1.0))
    };
    actions = actions
        .child(row2(
            ui::button(
                key("reinstall"),
                "Reinstall",
                Kind::Secondary,
                Some(&icons::ROTATE),
                managed && !busy && addon.latest.is_some(),
                {
                    let id = id.clone();
                    move |s, tx| dispatch(s, Intent::Reinstall(id.clone()), tx)
                },
            ),
            ui::button(
                key("ignore"),
                if ignored { "Unignore" } else { "Ignore" },
                Kind::Secondary,
                Some(if ignored {
                    &icons::EYE
                } else {
                    &icons::EYE_OFF
                }),
                managed && !busy,
                {
                    let id = id.clone();
                    move |s, tx| dispatch(s, Intent::SetIgnored(id.clone(), !ignored), tx)
                },
            ),
        ))
        .child(row2(
            ui::button(
                key("website"),
                "Website",
                Kind::Secondary,
                Some(&icons::EXTERNAL),
                addon.url.is_some(),
                {
                    let id = id.clone();
                    move |s, tx| dispatch(s, Intent::OpenWebsite(id.clone()), tx)
                },
            ),
            ui::button(
                key("folder"),
                "Folder",
                Kind::Secondary,
                Some(&icons::FOLDER),
                true,
                {
                    let id = id.clone();
                    move |s, tx| dispatch(s, Intent::OpenFolder(id.clone()), tx)
                },
            ),
        ));

    let remove = if s.confirm_remove.as_deref() == Some(&addon.id) {
        let folders = addon.folders.len();
        col()
            .key(key("confirm"))
            .w_full()
            .gap(10.0)
            .padding(12.0)
            .fill(p.danger_soft)
            .border(1.0, p.danger_line)
            .child(
                text(format!(
                    "Delete {} and its {folders} folder{} from AddOns?",
                    addon.name,
                    if folders == 1 { "" } else { "s" }
                ))
                .font(t().ty.note)
                .color(p.text)
                .wrap(),
            )
            .child(
                row()
                    .gap(8.0)
                    .child(ui::button(
                        key("remove-confirm"),
                        "Remove",
                        Kind::Danger,
                        Some(&icons::TRASH),
                        true,
                        {
                            let id = id.clone();
                            move |s, tx| dispatch(s, Intent::ConfirmRemove(id.clone()), tx)
                        },
                    ))
                    .child(ui::button(
                        key("remove-cancel"),
                        "Cancel",
                        Kind::Ghost,
                        None,
                        true,
                        |s, _| s.confirm_remove = None,
                    )),
            )
    } else {
        ui::button(
            key("remove"),
            "Remove…",
            Kind::Danger,
            Some(&icons::TRASH),
            !busy,
            {
                let id = id.clone();
                move |s, tx| dispatch(s, Intent::Remove(id.clone()), tx)
            },
        )
        .w_full()
    };

    let channel = ui::segmented(
        "detail/channel",
        &[
            (Channel::Stable, "Stable"),
            (Channel::Beta, "Beta"),
            (Channel::Alpha, "Alpha"),
        ],
        addon.channel,
        managed && !busy,
        {
            let id = id.clone();
            move |s: &mut State, c, tx: &mut Tx| dispatch(s, Intent::SetChannel(id.clone(), c), tx)
        },
    );

    let latest = addon
        .latest
        .as_ref()
        .map(|f| f.version.clone())
        .unwrap_or_else(|| "—".into());
    let body = col()
        .w_full()
        .gap(18.0)
        .padding(24.0)
        .child(
            row()
                .w_full()
                .gap(14.0)
                .items_start()
                .child(ui::addon_tile(
                    &addon.name,
                    48.0,
                    s.thumb(addon.thumbnail_url.as_deref()),
                    format!("detail/{id}/thumb"),
                ))
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(4.0)
                        .child(
                            text(addon.name.clone())
                                .font(t().ty.title)
                                .color(p.heading)
                                .wrap(),
                        )
                        .child(
                            text(addon.author.clone())
                                .font(t().ty.small)
                                .color(p.text_muted)
                                .truncate(),
                        )
                        .child(row().pt(6.0).child(status_badge(a))),
                )
                .child(ui::icon_button(
                    key("close"),
                    "Close details",
                    &icons::CLOSE,
                    |s, _| {
                        s.selected = None;
                        s.confirm_remove = None;
                    },
                )),
        )
        .child(if addon.summary.is_empty() {
            text("No description: this folder isn't matched to a provider.")
                .font(t().ty.note)
                .color(p.text_muted)
                .wrap()
        } else {
            text(addon.summary.clone())
                .font(t().ty.note)
                .color(p.text_dim)
                .wrap()
        })
        .child(
            col()
                .w_full()
                .child(fact(
                    "Installed",
                    addon
                        .installed_version
                        .clone()
                        .unwrap_or_else(|| "—".into()),
                    true,
                ))
                .child(fact("Latest", latest, true))
                .child(fact(
                    "Game version",
                    addon.game_version.clone().unwrap_or_else(|| "—".into()),
                    true,
                ))
                .child(fact(
                    "Provider",
                    addon
                        .provider
                        .clone()
                        .unwrap_or_else(|| "Not matched".into()),
                    false,
                ))
                .child(fact("Folders", addon.folders.join(", "), true)),
        )
        .child(
            col()
                .gap(8.0)
                .child(
                    text("RELEASE CHANNEL")
                        .font(t().ty.eyebrow)
                        .color(p.text_muted),
                )
                .child(channel),
        )
        .child(actions)
        .child(remove);
    let body = match &a.op {
        Some(Op::Failed(e)) => body.child(
            row()
                .w_full()
                .gap(8.0)
                .padding(10.0)
                .fill(p.danger.alpha(0.08))
                .child(rune::icon(&icons::ALERT, 14.0).color(p.danger))
                .child(
                    text(e.clone())
                        .font(t().ty.small.size_of(12.0))
                        .color(p.danger)
                        .wrap()
                        .grow(1.0)
                        .min_w(0.0),
                ),
        ),
        _ => body,
    };
    col()
        .key(format!("detail/{id}"))
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
