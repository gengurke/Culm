//! Settings: CurseForge key, default channel, installations, about (F7, F1).

use super::{band, subtitle};
use crate::brand;
use crate::icons;
use crate::state::{Intent, State, dispatch};
use crate::theme::{self, StyleExt, SurfaceExt, ThemeId, pal, t};
use crate::ui::{self, Kind};
use addon_core::Channel;
use rune::{Cursor, El, ScrollSpec, Tx, col, el, icon, row, text};

pub fn page(s: &State) -> El {
    let p = pal();
    let saved = s.settings.curseforge_key.clone().unwrap_or_default();
    let dirty = s.api_key.text.trim() != saved;
    let reveal = ui::icon_button(
        "settings/api-key/reveal",
        if s.api_key_revealed {
            "Hide key"
        } else {
            "Show key"
        },
        if s.api_key_revealed {
            &icons::EYE_OFF
        } else {
            &icons::EYE
        },
        |s, _| s.api_key_revealed = !s.api_key_revealed,
    );
    let key_field = ui::field(
        "settings/api-key",
        &s.api_key,
        "Paste your CurseForge API key",
        Some(&icons::KEY),
        |s: &mut State| &mut s.api_key,
    )
    .secure(s.api_key_revealed)
    .mono()
    .trailing(reveal)
    .on_submit(|s, tx| dispatch(s, Intent::SaveApiKey, tx))
    .build()
    .w(340.0);
    let key_status = if s.curseforge_ready() {
        ui::tag("Connected", ui::Mark::Good)
    } else {
        ui::tag("Not set", ui::Mark::Muted)
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
        installs = installs.child(
            text("No World of Warcraft: Forever installation yet.")
                .font(t().ty.note)
                .color(p.text_muted)
                .py(12.0),
        );
    }
    for inst in &s.installations {
        let on = s.current.as_deref() == Some(&inst.id);
        let id = inst.id.clone();
        let trailing = if on {
            ui::tag("In use", ui::Mark::Busy)
        } else {
            ui::button(
                format!("settings/installation/{}/use", inst.id),
                "Use",
                Kind::Secondary,
                None,
                true,
                move |s, tx| dispatch(s, Intent::SelectInstallation(id.clone()), tx),
            )
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
                .child(ui::glyph_tile(&icons::DRIVE, 36.0))
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(3.0)
                        .child(
                            row()
                                .gap(10.0)
                                .center_y()
                                .child(text(inst.label.clone()).font(t().ty.body).color(p.text))
                                .child(
                                    text(inst.version.clone().unwrap_or_default())
                                        .font(t().ty.mono.size_of(11.0))
                                        .color(p.text_muted),
                                ),
                        )
                        .child(
                            text(inst.root.display().to_string())
                                .font(t().ty.mono.size_of(11.5))
                                .color(p.text_muted)
                                .truncate(),
                        ),
                )
                .child(trailing),
        );
    }
    installs = installs.child(
        row()
            .pt(14.0)
            .gap(8.0)
            .child(ui::button(
                "settings/add-installation",
                "Add installation…",
                Kind::Secondary,
                Some(&icons::PLUS),
                true,
                |s, tx| dispatch(s, Intent::AddInstallation, tx),
            ))
            .child(ui::button(
                "settings/import-wowup",
                "Import from WowUp",
                Kind::Ghost,
                Some(&icons::DOWNLOAD),
                s.installation().is_some(),
                |s, tx| dispatch(s, Intent::ImportWowUp, tx),
            )),
    );

    let about = section("About").child(setting_row(
        brand::APP_NAME,
        "An addon manager for World of Warcraft: Forever. Free software under the GNU GPL v3 or later.",
        text(format!("v{}", env!("CARGO_PKG_VERSION")))
            .key("settings/version")
            .font(t().ty.mono)
            .color(p.text_dim),
    ))
    .child(
        text(brand::DISCLAIMER)
            .key("settings/disclaimer")
            .font(t().ty.small)
            .color(p.text_muted)
            .pt(10.0),
    );

    let mut themes = row()
        .key("settings/theme")
        .role("radiogroup")
        .w_full()
        .pt(14.0)
        .gap(12.0)
        .flex_wrap()
        .focus_group();
    for id in ThemeId::ALL {
        themes = themes.child(theme_card(s, *id));
    }
    let appearance = section("Appearance").child(themes);

    let content = col()
        .key("settings")
        .w_full()
        .max_w(880.0)
        .px(theme::PAD)
        .pb(40.0)
        .child(curseforge)
        .child(defaults)
        .child(appearance)
        .child(installs)
        .child(about);
    col()
        .w_full()
        .h_full()
        .child(band(
            "Settings",
            subtitle("Providers, defaults and your game installations"),
            vec![],
        ))
        .child(
            col()
                .key("settings/scroll")
                .grow(1.0)
                .min_h(0.0)
                .w_full()
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(content),
        )
}

/// A picker card: a miniature window painted in that theme's own skin, its name and a line.
fn theme_card(s: &State, id: ThemeId) -> El {
    let (th, p) = (id.theme(), pal());
    let fr = th.frame.map_or(th, |f| &**f);
    let on = s.settings.theme == id;
    let sk = &th.skin;
    let preview = row()
        .w_full()
        .h(80.0)
        .clip_rounded(0.0)
        .skin(&sk.window)
        .child(
            col()
                .w(36.0)
                .h_full()
                .pt(10.0)
                .items_center()
                .skin(&fr.skin.sidebar)
                .child(icon(sk.emblem, 18.0).color(fr.pal.accent)),
        )
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .h_full()
                .child(
                    row()
                        .w_full()
                        .h(38.0)
                        .px(10.0)
                        .center_y()
                        .skin(&fr.skin.band)
                        .child(
                            text(th.name)
                                .font(th.ty.heading.size_of(13.5))
                                .color(fr.pal.heading)
                                .truncate(),
                        ),
                )
                .child(
                    row()
                        .w_full()
                        .grow(1.0)
                        .px(10.0)
                        .gap(6.0)
                        .center_y()
                        .child(el().w(30.0).h(14.0).skin(&sk.primary))
                        .child(el().w(22.0).h(14.0).skin(&sk.secondary))
                        .child(el().w(8.0).h(8.0).radius(4.0).fill(th.pal.success)),
                ),
        );
    let card = col()
        .key(format!("settings/theme/{}", th.id))
        .role("radio")
        .label(th.name)
        .checked(on)
        .focusable(true)
        .w(140.0)
        .grow(1.0)
        .padding(8.0)
        .gap(10.0)
        .border(
            if on { 2.0 } else { 1.0 },
            if on { p.accent } else { p.border },
        )
        .hover(p.hover)
        .cursor(Cursor::Pointer)
        .on_press_tx(move |s: &mut State, tx: &mut Tx| dispatch(s, Intent::SetTheme(id), tx))
        .child(preview);
    card.child(
        row()
            .w_full()
            .gap(8.0)
            .px(4.0)
            .center_y()
            .child(
                col()
                    .grow(1.0)
                    .min_w(0.0)
                    .gap(2.0)
                    .child(text(th.name).font(t().ty.body).color(p.text))
                    .child(
                        text(th.blurb)
                            .font(t().ty.small)
                            .color(p.text_muted)
                            .truncate(),
                    ),
            )
            .child(icon(&icons::CHECK, 16.0).color(if on {
                p.accent
            } else {
                p.accent.alpha(0.0)
            })),
    )
}

fn section(title: &str) -> El {
    let p = pal();
    col().w_full().pt(28.0).child(
        text(title.to_string())
            .font(t().ty.heading)
            .color(p.heading)
            .pb(4.0),
    )
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
                .child(text(label.to_string()).font(t().ty.body).color(p.text))
                .child(
                    text(description.to_string())
                        .font(t().ty.note)
                        .color(p.text_muted)
                        .wrap(),
                ),
        )
        .child(control)
}
