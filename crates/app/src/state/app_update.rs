//! The app's own update (F8.4).

use super::{State, Tone, err};
use crate::brand::{self, version_label};
use crate::worker::{self, Msg};
use addon_core::self_update::Release;
use rune::Tx;

/// The app's own update (F8.4).
#[derive(Debug, Clone, PartialEq)]
pub enum AppUpdate {
    /// Not asked yet (or the quiet check at start failed: offline is not news).
    Unknown,
    Checking,
    UpToDate,
    Available(Release),
    Downloading {
        release: Release,
        done: f32,
    },
    /// The new version is in place; it runs after a restart.
    Ready(String),
    Failed {
        release: Release,
        error: String,
    },
}

impl AppUpdate {
    /// The release on offer, while one is.
    pub fn release(&self) -> Option<&Release> {
        match self {
            AppUpdate::Available(r)
            | AppUpdate::Downloading { release: r, .. }
            | AppUpdate::Failed { release: r, .. } => Some(r),
            _ => None,
        }
    }
}

impl State {
    /// Asks for the app's newest release; `asked`: the user pressed the button (say "up to date").
    pub(super) fn check_app_update(&mut self, asked: bool) {
        self.app_update = AppUpdate::Checking;
        worker::spawn(self, "app-update", move |b, post| {
            post.send(Msg::AppRelease {
                asked,
                result: b.app_release().map_err(err),
            })
        });
    }

    pub(super) fn app_release(&mut self, asked: bool, result: Result<Option<Release>, String>) {
        match result {
            Ok(Some(release)) => {
                let how = if self.backend.self_updating() {
                    "Update from the sidebar; it takes a restart"
                } else {
                    "Download it from the sidebar"
                };
                self.toast(
                    Tone::Info,
                    format!("{} {} is available", brand::APP_NAME, release.version),
                    how,
                );
                self.app_update = AppUpdate::Available(release);
            }
            Ok(None) => {
                self.app_update = AppUpdate::UpToDate;
                if asked {
                    self.toast(
                        Tone::Success,
                        format!("{} is up to date", brand::APP_NAME),
                        version_label(),
                    );
                }
            }
            Err(e) => {
                self.app_update = AppUpdate::Unknown;
                if asked {
                    self.toast(Tone::Danger, "Could not check for updates", e);
                }
            }
        }
    }

    pub(super) fn app_installed(&mut self, version: String, result: Result<(), String>) {
        let release = self.app_update.release().cloned();
        match (result, release) {
            (Ok(()), _) => {
                self.toast(
                    Tone::Success,
                    format!("{} {version} is installed", brand::APP_NAME),
                    "Restart to use it",
                );
                self.app_update = AppUpdate::Ready(version);
            }
            (Err(error), Some(release)) => {
                self.toast(Tone::Danger, "Update failed", error.clone());
                self.app_update = AppUpdate::Failed { release, error };
            }
            (Err(error), None) => self.toast(Tone::Danger, "Update failed", error),
        }
    }

    pub(super) fn install_app_update(&mut self, tx: &mut Tx) {
        let Some(release) = self.app_update.release().cloned() else {
            return;
        };
        if matches!(self.app_update, AppUpdate::Downloading { .. }) {
            return;
        }
        if !self.backend.self_updating() {
            let _ = tx.open_url(&release.page);
            return;
        }
        self.app_update = AppUpdate::Downloading {
            release: release.clone(),
            done: 0.0,
        };
        worker::spawn(self, "app-update", move |b, post| {
            let result = b
                .install_app_release(&release, &|p| post.send(Msg::AppProgress(p)))
                .map_err(err);
            post.send(Msg::AppInstalled {
                version: release.version,
                result,
            });
        });
    }
}
