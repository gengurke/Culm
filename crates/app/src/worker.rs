//! Background work: backend calls run on Rune's task pool and come back as [`Msg`]s over a
//! channel; the waker runs the window's `poll`, which lands them with `State::apply`.

use crate::backend::{Backend, Backup, Settings, Warned};
use crate::state::{MaintCard, State};
use addon_core::{Addon, Installation, RemoteAddon};
use rune::Waker;
use std::sync::mpsc::Sender;

/// A worker result. Errors are already formatted for display.
#[derive(Debug)]
pub enum Msg {
    Booted { settings: Settings, installations: Vec<Installation> },
    /// Results tagged `inst` (an installation id) are dropped if the user switched installations meanwhile.
    Scanned { inst: String, result: Result<Warned<Vec<Addon>>, String> },
    Checked { inst: String, result: Result<Warned<Vec<Addon>>, String> },
    /// Download progress 0..=1 of an addon id (update) or external id (install).
    Progress { id: String, done: f32 },
    Updated { id: String, result: Result<Addon, String> },
    /// `external_id` is empty for install-from-URL.
    Installed { inst: String, external_id: String, result: Result<Addon, String> },
    Removed { id: String, result: Result<(), String> },
    Featured(Result<Vec<RemoteAddon>, String>),
    /// Thumbnail bytes for a URL (None: failed, the initials tile stays).
    Thumbnail { url: String, bytes: Option<Vec<u8>> },
    GameRunning { inst: String, running: bool },
    Maintained { inst: String, card: MaintCard, result: Result<String, String> },
    Backups { inst: String, list: Vec<Backup> },
    InstallationAdded(Result<Installation, String>),
    Imported { inst: String, result: Result<usize, String> },
    Failed { title: String, error: String },
}

/// How a worker reports back: each message wakes the window.
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
    let post = Post { tx: s.msgs.0.clone(), wake: s.waker.clone() };
    let fallback = Post { tx: s.msgs.0.clone(), wake: s.waker.clone() };
    if let Err(e) = rune::task::spawn(name, move || job(&*backend, &post)) {
        fallback.send(Msg::Failed { title: "Could not start background work".into(), error: e.to_string() });
    }
}
