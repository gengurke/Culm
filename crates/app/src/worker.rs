//! Background work: backend calls run on Rune's task pool and come back as [`Msg`]s over a
//! channel; the waker runs the window's `poll`, which lands them with `State::apply`.

use crate::backend::{Backend, Backup, Found, Settings, Warned};
use crate::state::{MaintCard, State};
use addon_core::self_update::Release;
use addon_core::{Addon, Category, Installation};
use rune::Waker;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

/// A worker result. Errors are already formatted for display.
#[derive(Debug)]
pub enum Msg {
    Booted {
        settings: Settings,
        installations: Vec<Installation>,
    },
    /// Results tagged `inst` (an installation id) are dropped if the user switched installations meanwhile.
    Scanned {
        inst: String,
        result: Result<Warned<Vec<Addon>>, String>,
    },
    /// `only`: the one addon checked (F3.3); None for a full check.
    Checked {
        inst: String,
        only: Option<String>,
        result: Result<Warned<Vec<Addon>>, String>,
    },
    /// Download progress 0..=1 of an addon id (update) or a result's install key (install).
    Progress {
        id: String,
        done: f32,
    },
    /// A look in Downloads for the `looked` addons' zips; `found` as (addon id, path) (F3.5).
    Downloads {
        inst: String,
        looked: Vec<String>,
        found: Vec<(String, PathBuf)>,
    },
    Updated {
        id: String,
        result: Result<Addon, String>,
    },
    Installed {
        inst: String,
        /// The result's `state::install_key`.
        key: String,
        result: Result<Addon, String>,
    },
    Removed {
        id: String,
        result: Result<(), String>,
    },
    Featured(Result<Found, String>),
    Categories(Result<Vec<Category>, String>),
    /// The long description of the remote addon `(provider, external_id)` (None: it has none).
    Description {
        addon: (String, String),
        result: Result<Option<String>, String>,
    },
    /// Thumbnail bytes for a URL (None: failed, the initials tile stays).
    Thumbnail {
        url: String,
        bytes: Option<Vec<u8>>,
    },
    /// An original class icon's bytes for a class slug (F4.6).
    ClassIcon {
        slug: String,
        bytes: Vec<u8>,
    },
    GameRunning {
        inst: String,
        running: bool,
    },
    Maintained {
        inst: String,
        card: MaintCard,
        result: Result<String, String>,
    },
    Backups {
        inst: String,
        list: Vec<Backup>,
    },
    InstallationAdded(Result<Installation, String>),
    Imported {
        inst: String,
        result: Result<usize, String>,
    },
    Failed {
        title: String,
        error: String,
    },
    /// The app's newest release, if newer (F8.4); `asked`: the user pressed "Check for updates".
    AppRelease {
        asked: bool,
        result: Result<Option<Release>, String>,
    },
    /// Download progress 0..=1 of the app update.
    AppProgress(f32),
    /// The app's file was replaced by `version`.
    AppInstalled {
        version: String,
        result: Result<(), String>,
    },
}

/// How a worker reports back: each message wakes the window.
#[derive(Clone)]
pub struct Post {
    tx: Sender<Msg>,
    wake: Waker,
}

impl Post {
    pub fn send(&self, msg: Msg) {
        if self.tx.send(msg).is_ok() {
            (self.wake)();
        }
    }
}

/// Runs `job` with the backend off the UI thread.
pub fn spawn(s: &State, name: &str, job: impl FnOnce(&dyn Backend, &Post) + Send + 'static) {
    let backend = s.backend.clone();
    let post = Post {
        tx: s.msgs.0.clone(),
        wake: s.waker.clone(),
    };
    let fallback = post.clone();
    if let Err(e) = rune::task::spawn(name, move || job(&*backend, &post)) {
        fallback.send(Msg::Failed {
            title: "Could not start background work".into(),
            error: e.to_string(),
        });
    }
}
