//! Maintenance: clear cache, clean leftovers, back up WTF. Each card confirms first and shows a
//! result line; all are locked while the game runs (F6).

use super::{band, band_note};
use crate::icons;
use crate::state::{Intent, MaintCard, MaintPhase, State, dispatch};
use crate::theme::{self, pal, t};
use crate::ui::{self, Kind};
use rune::{El, Icon, ScrollSpec, col, icon, row, text};

pub fn page(s: &State) -> El {
    let p = pal();
    let mut content = col()
        .key("maintenance")
        .w_full()
        .max_w(820.0)
        .gap(16.0)
        .px(theme::PAD)
        .py(24.0);
    if s.game_running {
        content = content.child(
            row()
                .key("maintenance/game-running")
                .w_full()
                .gap(12.0)
                .padding(12.0)
                .center_y()
                .radius(t().skin.panel.radius)
                .fill(p.warning_soft)
                .border(1.0, p.warning.alpha(0.4))
                .child(ui::badge(&icons::ALERT, p.warning))
            .child(
                text(
                    "World of Warcraft is running. Close the game to clean or back up its folders.",
                )
                .font(t().ty.body)
                .color(p.text),
            ),
        );
    }
    let scope = ui::segmented(
        "maint/cache/scope",
        &[
            (false, "Whole cache", "Delete the whole Cache folder"),
            (
                true,
                "Only WDB",
                "Delete only Cache/WDB, the cached item, quest and creature data",
            ),
        ],
        s.cache_wdb_only,
        !s.game_running,
        |s: &mut State, v, _| s.cache_wdb_only = v,
    );
    content = content
        .child(card(
            s,
            MaintCard::Cache,
            &icons::BROOM,
            "Clear cache",
            "Deletes the client's Cache folder. Fixes stale item and quest data; the game rebuilds it on the next login.",
            "Clear cache",
            Some(scope),
        ))
        .child(card(
            s,
            MaintCard::Leftovers,
            &icons::TRASH,
            "Clean leftover files",
            "Removes .lua.bak and .old files the game leaves in WTF after crashes and patches.",
            "Clean up",
            None,
        ))
        .child(card(
            s,
            MaintCard::Wtf,
            &icons::ARCHIVE,
            "Back up settings (WTF)",
            "Zips your WTF folder — key bindings, macros and all addon SavedVariables — into the app's data folder.",
            "Back up now",
            Some(backups(s)),
        ));
    col()
        .w_full()
        .h_full()
        .child(band(
            "Maintenance",
            || {
                vec![band_note(
                    "Keep the client folder tidy. Nothing here touches your addons.",
                )]
            },
            vec![],
        ))
        .child(
            col()
                .key("maintenance/scroll")
                .grow(1.0)
                .min_h(0.0)
                .w_full()
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(content),
        )
}

fn card(
    s: &State,
    which: MaintCard,
    glyph: &'static Icon,
    title: &str,
    body: &str,
    action: &str,
    extra: Option<El>,
) -> El {
    let p = pal();
    let key = match which {
        MaintCard::Cache => "maint/cache",
        MaintCard::Leftovers => "maint/leftovers",
        MaintCard::Wtf => "maint/wtf",
    };
    let phase = s.maint.get(&which).cloned().unwrap_or(MaintPhase::Idle);
    let enabled = !s.game_running && !matches!(phase, MaintPhase::Running | MaintPhase::Confirm);
    let head = row()
        .w_full()
        .gap(16.0)
        .items_start()
        .child(ui::glyph_tile(glyph, 36.0))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(4.0)
                .child(
                    text(title.to_string())
                        .font(t().ty.heading)
                        .color(p.heading),
                )
                .child(
                    text(body.to_string())
                        .font(t().ty.note)
                        .color(p.text_dim)
                        .wrap(),
                ),
        )
        .child(
            ui::button(
                format!("{key}/run"),
                action,
                Kind::Secondary,
                None,
                enabled,
                move |s, tx| dispatch(s, Intent::Maintain(which), tx),
            )
            .help(match which {
                MaintCard::Cache => "Delete the game's cache; you confirm first",
                MaintCard::Leftovers => {
                    "Delete the leftover .bak and .old files; you confirm first"
                }
                MaintCard::Wtf => {
                    "Zip your WTF folder into the app's data folder; you confirm first"
                }
            }),
        );
    let mut c = ui::card(key).w_full().child(head);
    if let Some(e) = extra {
        c = c.child(row().pl(52.0).child(e));
    }
    let foot = match &phase {
        MaintPhase::Idle => None,
        MaintPhase::Confirm => Some(
            row()
                .key(format!("{key}/confirm-row"))
                .w_full()
                .gap(10.0)
                .padding(12.0)
                .center_y()
                .fill(p.danger_soft)
                .border(1.0, p.danger_line)
                .child(
                    text(confirm_text(s, which))
                        .font(t().ty.note)
                        .color(p.text)
                        .grow(1.0)
                        .min_w(0.0)
                        .wrap(),
                )
                .child(ui::button(
                    format!("{key}/confirm"),
                    "Confirm",
                    Kind::Danger,
                    None,
                    true,
                    move |s, tx| dispatch(s, Intent::ConfirmMaintain(which), tx),
                ))
                .child(ui::button(
                    format!("{key}/cancel"),
                    "Cancel",
                    Kind::Ghost,
                    None,
                    true,
                    move |s, _| {
                        s.maint.remove(&which);
                    },
                )),
        ),
        MaintPhase::Running => Some(
            row()
                .gap(8.0)
                .center_y()
                .child(ui::spinner(14.0, p.accent))
                .child(text("Working…").font(t().ty.small).color(p.text_dim)),
        ),
        MaintPhase::Done(line) => Some(
            row()
                .key(format!("{key}/result"))
                .gap(8.0)
                .center_y()
                .child(icon(&icons::CHECK, 14.0).color(p.success))
                .child(text(line.clone()).font(t().ty.small).color(p.success)),
        ),
        MaintPhase::Failed(e) => Some(
            row()
                .key(format!("{key}/result"))
                .gap(8.0)
                .center_y()
                .child(icon(&icons::ALERT, 14.0).color(p.danger))
                .child(
                    text(e.clone())
                        .font(t().ty.small)
                        .color(p.danger)
                        .wrap_anywhere()
                        .grow(1.0)
                        .min_w(0.0),
                ),
        ),
    };
    if let Some(f) = foot {
        c = c.child(row().w_full().pl(52.0).child(f));
    }
    if s.game_running { c.opacity(0.6) } else { c }
}

fn confirm_text(s: &State, which: MaintCard) -> String {
    match which {
        MaintCard::Cache if s.cache_wdb_only => {
            "Delete Cache/WDB? The game re-downloads item and quest data.".into()
        }
        MaintCard::Cache => "Delete the whole Cache folder?".into(),
        MaintCard::Leftovers => "Delete all .bak and .old files under WTF?".into(),
        MaintCard::Wtf => "Create a zip of the WTF folder now?".into(),
    }
}

fn backups(s: &State) -> El {
    let p = pal();
    let mut list = col().key("maint/wtf/backups").w_full().min_w(420.0);
    if s.backups.is_empty() {
        return list.child(
            text("No backups yet.")
                .font(t().ty.small)
                .color(p.text_muted),
        );
    }
    for b in &s.backups {
        list = list.child(
            row()
                .w_full()
                .py(6.0)
                .gap(12.0)
                .center_y()
                .border_sides([0.0, 0.0, 1.0, 0.0], p.divider)
                .child(icon(&icons::ARCHIVE, 13.0).color(p.text_muted))
                .child(
                    text(b.name.clone())
                        .font(t().ty.mono)
                        .color(p.text_dim)
                        .grow(1.0)
                        .min_w(0.0)
                        .truncate(),
                )
                .child(
                    text(
                        b.created_at
                            .replace('T', " ")
                            .trim_end_matches('Z')
                            .trim_end_matches(":00")
                            .to_string(),
                    )
                    .font(t().ty.small)
                    .color(p.text_muted),
                )
                .child(
                    text(ui::bytes(b.size_bytes))
                        .font(t().ty.number)
                        .color(p.text_muted)
                        .w(64.0)
                        .align(rune::Align::End),
                ),
        );
    }
    list
}
