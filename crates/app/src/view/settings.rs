//! Settings: CurseForge key, default channel, installations, about (F7, F1).

use super::{band, subtitle};
use crate::brand;
use crate::icons;
use crate::state::{Intent, State, dispatch};
use crate::theme::{self, StyleExt, pal};
use crate::ui::{self, Kind};
use addon_core::Channel;
use rune::{El, ScrollSpec, Tx, col, el, icon, row, text};

pub fn page(s: &State) -> El {
    let p = pal();
    let saved = s.settings.curseforge_key.clone().unwrap_or_default();
    let dirty = s.api_key.text.trim() != saved;
    let reveal = ui::icon_button("settings/api-key/reveal", if s.api_key_revealed { "Hide key" } else { "Show key" }, if s.api_key_revealed { &icons::EYE_OFF } else { &icons::EYE }, |s, _| {
        s.api_key_revealed = !s.api_key_revealed
    });
    let key_field = ui::field("settings/api-key", &s.api_key, "Paste your CurseForge API key", Some(&icons::KEY), |s: &mut State| &mut s.api_key)
        .secure(s.api_key_revealed)
        .mono()
        .trailing(reveal)
        .on_submit(|s, tx| dispatch(s, Intent::SaveApiKey, tx))
        .build()
        .w(340.0);
    let key_status = if s.curseforge_ready() {
        ui::badge("Connected", p.success, p.success_soft, true)
    } else {
        ui::badge("Not set", p.text_muted, p.chip, false)
    };
    let curseforge = section("CurseForge")
        .child(setting_row(
            "API key",
            "CurseForge requires a personal key for search and updates. Create one for free at console.curseforge.com; it is stored only on this computer.",
            row()
                .gap(8.0)
                .center_y()
                .child(key_field)
                .child(ui::button("settings/api-key/save", "Save", Kind::Primary, None, dirty, |s, tx| dispatch(s, Intent::SaveApiKey, tx))),
        ))
        .child(setting_row("Status", "Whether CurseForge features are available.", key_status));

    let defaults = section("Updates").child(setting_row(
        "Default channel",
        "Release channel for newly installed addons. Beta also accepts stable files, Alpha accepts everything.",
        ui::segmented(
            "settings/channel",
            &[(Channel::Stable, "Stable"), (Channel::Beta, "Beta"), (Channel::Alpha, "Alpha")],
            s.settings.default_channel,
            true,
            |s: &mut State, c, tx: &mut Tx| dispatch(s, Intent::SetDefaultChannel(c), tx),
        ),
    ));

    let mut installs = section("Installations");
    if s.installations.is_empty() {
        installs = installs.child(text("No World of Warcraft: Forever installation yet.").font(theme::NOTE).color(p.text_muted).py(12.0));
    }
    for inst in &s.installations {
        let on = s.current.as_deref() == Some(&inst.id);
        let id = inst.id.clone();
        let trailing = if on {
            ui::badge("In use", p.accent, p.accent_soft, false)
        } else {
            ui::button(format!("settings/installation/{}/use", inst.id), "Use", Kind::Secondary, None, true, move |s, tx| {
                dispatch(s, Intent::SelectInstallation(id.clone()), tx)
            })
            .h(28.0)
        };
        installs = installs.child(
            row()
                .key(format!("settings/installation/{}", inst.id))
                .w_full()
                .py(14.0)
                .gap(14.0)
                .center_y()
                .border_sides([0.0, 0.0, 1.0, 0.0], p.divider)
                .child(el().size(36.0, 36.0).center().fill(p.panel_alt).border(1.0, p.border).child(icon(&icons::DRIVE, 18.0).color(p.text_dim)))
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(3.0)
                        .child(
                            row()
                                .gap(10.0)
                                .center_y()
                                .child(text(inst.label.clone()).font(theme::BODY).color(p.text))
                                .child(text(inst.version.clone().unwrap_or_default()).font(theme::MONO.size_of(11.0)).color(p.text_muted)),
                        )
                        .child(text(inst.root.display().to_string()).font(theme::MONO.size_of(11.5)).color(p.text_muted).truncate()),
                )
                .child(trailing),
        );
    }
    installs = installs.child(
        row()
            .pt(14.0)
            .gap(8.0)
            .child(ui::button("settings/add-installation", "Add installation…", Kind::Secondary, Some(&icons::PLUS), true, |s, tx| dispatch(s, Intent::AddInstallation, tx)))
            .child(ui::button("settings/import-wowup", "Import from WowUp", Kind::Ghost, Some(&icons::DOWNLOAD), s.installation().is_some(), |s, tx| {
                dispatch(s, Intent::ImportWowUp, tx)
            })),
    );

    let about = section("About").child(setting_row(
        brand::APP_NAME,
        "An addon manager for World of Warcraft: Forever. Free software under the MIT licence.",
        text(format!("v{}", env!("CARGO_PKG_VERSION"))).key("settings/version").font(theme::MONO).color(p.text_dim),
    ));

    let content = col().key("settings").w_full().max_w(880.0).px(theme::PAD).pb(40.0).child(curseforge).child(defaults).child(installs).child(about);
    col()
        .w_full()
        .h_full()
        .child(band("Settings", subtitle("Providers, defaults and your game installations"), vec![]))
        .child(col().key("settings/scroll").grow(1.0).min_h(0.0).w_full().scroll(ScrollSpec { stick_bottom: false }).child(content))
}

fn section(title: &str) -> El {
    let p = pal();
    col().w_full().pt(28.0).child(text(title.to_string()).font(theme::HEADING).color(p.text).pb(4.0))
}

fn setting_row(label: &str, description: &str, control: El) -> El {
    let p = pal();
    row()
        .w_full()
        .py(14.0)
        .gap(24.0)
        .center_y()
        .border_sides([0.0, 0.0, 1.0, 0.0], p.divider)
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(3.0)
                .child(text(label.to_string()).font(theme::BODY).color(p.text))
                .child(text(description.to_string()).font(theme::NOTE).color(p.text_muted).wrap()),
        )
        .child(control)
}
