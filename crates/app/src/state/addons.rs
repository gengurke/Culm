//! My Addons: the installed list, its statuses and order, and scan, update check, update, remove
//! and the downloads fetched by hand (F2, F3, F5).

use super::{Job, State, Tone, err};
use crate::backend::{AddonOrder, Backend, SortBy, Warned};
use crate::worker::{self, Msg};
use addon_core::{Addon, IgnoreReason, Installation};
use rune::Tx;
use std::path::PathBuf;

/// A running or failed operation on one addon (or one search result being installed).
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// The update is fetched by hand (F3.5): Culm installs it once it is in Downloads. Looks there
    /// first and opens the download in the browser (`opened`) only if it isn't. Gives up at
    /// `until` (window clock).
    AwaitingDownload {
        until: f64,
        opened: bool,
    },
    /// Only this addon is being checked for updates (its channel changed, F3.3).
    Checking,
    Downloading(f32),
    Installing,
    Removing,
    Failed(String),
}

impl Op {
    pub fn busy(&self) -> bool {
        !matches!(self, Op::Failed(_))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddonUi {
    pub addon: Addon,
    pub op: Option<Op>,
}

/// What a row's status badge says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    UpToDate,
    /// Managed, but no update check produced a latest release (e.g. CurseForge without a key).
    NotChecked,
    UpdateAvailable,
    /// A newer release exists, but its author only allows downloads through the source's own app or
    /// website (no download URL), so Culm can only point there.
    AppOnly,
    Working,
    Ignored,
    Unknown,
    Error,
}

/// Where `addon.latest` is fetched by hand (F3.5, F4.3): on CurseForge the file's download page
/// (the browser starts the download itself), else the addon's page.
pub fn manual_url(addon: &Addon) -> Option<String> {
    let url = addon.url.clone()?;
    match &addon.latest {
        Some(f) if addon.provider.as_deref() == Some(addon_core::providers::curseforge::NAME) => {
            Some(format!("{}/download/{}", url.trim_end_matches('/'), f.id))
        }
        _ => Some(url),
    }
}

/// How long Culm looks for an addon fetched by hand, and how often (F3.5).
pub(super) const AWAIT_DOWNLOAD_SECS: f64 = 600.0;
const DOWNLOADS_CHECK_SECS: f64 = 1.0;

impl AddonUi {
    /// Where the update is fetched by hand: [`manual_url`].
    pub fn manual_url(&self) -> Option<String> {
        manual_url(&self.addon)
    }

    /// Whether the update waiting has to be fetched by hand: its author allows downloads only
    /// through the source's own app or website, so it has no download link (F3.5).
    pub fn by_hand(&self) -> bool {
        self.addon.update_available()
            && self
                .addon
                .latest
                .as_ref()
                .is_some_and(|f| f.download_url.is_empty())
    }

    /// Whether a zip fetched by hand can be recognised: the source published its size and SHA-1.
    pub fn can_install_by_hand(&self) -> bool {
        self.addon
            .latest
            .as_ref()
            .is_some_and(|f| f.check.is_some())
    }

    pub fn status(&self) -> Status {
        match (&self.op, self.addon.ignored) {
            (Some(Op::Failed(_)), _) => Status::Error,
            (Some(_), _) => Status::Working,
            (None, Some(IgnoreReason::Unknown)) => Status::Unknown,
            (None, Some(_)) => Status::Ignored,
            (None, None) if self.by_hand() => Status::AppOnly,
            (None, None) if self.addon.update_available() => Status::UpdateAvailable,
            (None, None) if self.addon.latest.is_none() => Status::NotChecked,
            (None, None) => Status::UpToDate,
        }
    }
}

/// Where a status sorts in ascending order: what needs the user first.
fn attention(s: Status) -> u8 {
    match s {
        Status::Error => 0,
        Status::UpdateAvailable => 1,
        Status::AppOnly => 2,
        Status::Working => 3,
        Status::NotChecked => 4,
        Status::UpToDate => 5,
        Status::Unknown => 6,
        Status::Ignored => 7,
    }
}

/// `1.15.4` before `1.60.1`: dotted numbers compare part by part, anything else as text.
fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parts = |v: &str| -> Vec<Result<u64, String>> {
        v.split('.')
            .map(|p| p.parse::<u64>().map_err(|_| p.to_lowercase()))
            .collect()
    };
    parts(a).cmp(&parts(b))
}

/// Two rows in `order`. Rows without a game version stay last either way.
fn compare(order: AddonOrder, a: &AddonUi, b: &AddonUi) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let by = match order.by {
        SortBy::Name => a
            .addon
            .name
            .to_lowercase()
            .cmp(&b.addon.name.to_lowercase()),
        SortBy::Status => attention(a.status()).cmp(&attention(b.status())),
        SortBy::Game => match (&a.addon.game_version, &b.addon.game_version) {
            (Some(x), Some(y)) => version_cmp(x, y),
            (None, None) => Ordering::Equal,
            (None, Some(_)) => return Ordering::Greater,
            (Some(_), None) => return Ordering::Less,
        },
    };
    if order.desc { by.reverse() } else { by }
}

impl State {
    pub fn addon(&self, id: &str) -> Option<&AddonUi> {
        self.addons.iter().find(|a| a.addon.id == id)
    }

    pub(super) fn addon_mut(&mut self, id: &str) -> Option<&mut AddonUi> {
        self.addons.iter_mut().find(|a| a.addon.id == id)
    }

    pub fn update_count(&self) -> usize {
        self.addons
            .iter()
            .filter(|a| a.status() == Status::UpdateAvailable)
            .count()
    }

    pub fn not_checked_count(&self) -> usize {
        self.addons
            .iter()
            .filter(|a| a.status() == Status::NotChecked)
            .count()
    }

    /// Whether CurseForge addons are shown that no update check covers: without a key they only
    /// carry the imported last known release.
    pub fn curseforge_unchecked(&self) -> bool {
        !self.curseforge_ready()
            && self.addons.iter().any(|a| {
                a.addon.ignored.is_none()
                    && a.addon.provider.as_deref() == Some(addon_core::providers::curseforge::NAME)
            })
    }

    /// Addons matching the filter text, in the picked order (F2.6).
    pub fn visible_addons(&self) -> Vec<&AddonUi> {
        let q = self.filter.text.trim().to_lowercase();
        let mut list: Vec<&AddonUi> = self
            .addons
            .iter()
            .filter(|a| {
                q.is_empty()
                    || a.addon.name.to_lowercase().contains(&q)
                    || a.addon.author.to_lowercase().contains(&q)
            })
            .collect();
        // `addons` is kept in name order, so a stable sort breaks ties by name.
        let order = self.settings.addon_order;
        list.sort_by(|a, b| compare(order, a, b));
        list
    }

    /// Replaces the list (a scan result). Addons with an update or remove still running keep
    /// showing it, so they can't be started twice.
    pub(super) fn set_addons(&mut self, addons: Vec<Addon>) {
        let busy = |id: &str| self.addon(id).and_then(|a| a.op.clone()).filter(Op::busy);
        let mut list: Vec<AddonUi> = addons
            .into_iter()
            .map(|addon| AddonUi {
                op: busy(&addon.id),
                addon,
            })
            .collect();
        list.sort_by_key(|a| a.addon.name.to_lowercase());
        self.addons = list;
        self.load_addon_thumbs();
        if self
            .selected
            .as_ref()
            .is_some_and(|id| self.addon(id).is_none())
        {
            self.selected = None;
        }
    }

    /// Fetches the installed addons' thumbnails not requested yet.
    pub(super) fn load_addon_thumbs(&mut self) {
        self.load_thumbs(
            self.addons
                .iter()
                .filter_map(|a| a.addon.thumbnail_url.clone())
                .collect(),
        );
    }

    /// Marks addon `id` with `op` and hands it out with its installation; None while it is busy.
    pub(super) fn begin_op(&mut self, id: &str, op: Op) -> Option<(Installation, Addon)> {
        let inst = self.installation().cloned()?;
        let a = self.addon_mut(id)?;
        if a.op.as_ref().is_some_and(Op::busy) {
            return None;
        }
        a.op = Some(op);
        Some((inst, a.addon.clone()))
    }

    /// Clears the per-addon check mark of `id` (F3.3).
    pub(super) fn end_check_of(&mut self, id: &str) {
        if let Some(a) = self.addon_mut(id).filter(|a| a.op == Some(Op::Checking)) {
            a.op = None;
        }
    }

    /// Lands an update check: of every addon, or `only` that one.
    pub(super) fn checked(
        &mut self,
        only: Option<String>,
        result: Result<Warned<Vec<Addon>>, String>,
    ) {
        match &only {
            Some(id) => self.end_check_of(id),
            None => self.job = Job::Idle,
        }
        let (addons, warnings) = match result {
            Ok(found) => found,
            Err(e) => return self.toast(Tone::Danger, "Update check failed", e),
        };
        for fresh in addons {
            if let Some(a) = self.addon_mut(&fresh.id) {
                let a = &mut a.addon;
                // A check started before a channel change answers for the old channel.
                if a.channel == fresh.channel {
                    a.latest = fresh.latest;
                }
                (a.name, a.author, a.summary) = (fresh.name, fresh.author, fresh.summary);
                (a.url, a.thumbnail_url) = (fresh.url, fresh.thumbnail_url);
            }
        }
        self.load_addon_thumbs();
        self.warn(warnings);
        if only.is_none()
            && let n @ 1.. = self.update_count()
        {
            self.toast(
                Tone::Info,
                format!("{} available", crate::ui::plural(n, "update")),
                "",
            );
        }
    }

    /// Lands a look in Downloads (F3.5, F4.3).
    pub(super) fn downloads_looked(
        &mut self,
        inst: &str,
        looked: Vec<String>,
        found: Vec<(String, PathBuf)>,
    ) {
        self.downloads_due = Some(self.clock + DOWNLOADS_CHECK_SECS);
        if !self.is_current(inst) {
            return;
        }
        // Not in Downloads yet: now the browser fetches it. `id` is an addon's id, or a
        // search result's install key.
        for id in looked {
            if found.iter().any(|(f, _)| *f == id) {
                continue;
            }
            let url = self
                .addon(&id)
                .map(|a| &a.addon)
                .or(self.fetching.get(&id))
                .and_then(manual_url);
            let op = if self.addon(&id).is_some() {
                self.addon_mut(&id).and_then(|a| a.op.as_mut())
            } else {
                self.installing.get_mut(&id)
            };
            if let Some(Op::AwaitingDownload { opened, .. }) = op
                && !*opened
            {
                *opened = true;
                self.open_urls.extend(url);
            }
        }
        for (id, path) in found {
            if self
                .addon(&id)
                .is_some_and(|a| matches!(a.op, Some(Op::AwaitingDownload { .. })))
            {
                self.install_zip(&id, path, true);
            } else if matches!(self.installing.get(&id), Some(Op::AwaitingDownload { .. }))
                && let Some(record) = self.fetching.remove(&id)
            {
                self.install_new_zip(id, record, path, true);
            }
        }
    }

    pub(super) fn progress(&mut self, id: String, done: f32) {
        let op = if done >= 1.0 {
            Op::Installing
        } else {
            Op::Downloading(done)
        };
        if let Some(slot) = self.installing.get_mut(&id) {
            *slot = op;
        } else if let Some(a) = self.addon_mut(&id) {
            a.op = Some(op);
        }
    }

    pub(super) fn updated(&mut self, id: String, result: Result<Addon, String>) {
        match result {
            Ok(addon) => {
                let name = addon.name.clone();
                let version = addon.installed_version.clone().unwrap_or_default();
                if self.addon(&id).is_some() {
                    self.upsert(addon);
                }
                self.toast(Tone::Success, format!("Updated {name}"), version);
            }
            Err(e) => {
                let name = self
                    .addon(&id)
                    .map(|a| a.addon.name.clone())
                    .unwrap_or_default();
                if let Some(a) = self.addon_mut(&id) {
                    a.op = Some(Op::Failed(e.clone()));
                }
                self.toast(Tone::Danger, format!("Could not update {name}"), e);
            }
        }
    }

    pub(super) fn removed(&mut self, id: String, result: Result<(), String>) {
        match result {
            Ok(()) => {
                let name = self
                    .addon(&id)
                    .map(|a| a.addon.name.clone())
                    .unwrap_or_default();
                self.addons.retain(|a| a.addon.id != id);
                if self.selected.as_deref() == Some(&id) {
                    self.selected = None;
                }
                self.toast(Tone::Success, format!("Removed {name}"), "");
            }
            Err(e) => {
                if let Some(a) = self.addon_mut(&id) {
                    a.op = Some(Op::Failed(e.clone()));
                }
                self.toast(Tone::Danger, "Remove failed", e);
            }
        }
    }

    /// Lands an installed or updated record: it owns its folders now (the store's upsert rule).
    pub(super) fn upsert(&mut self, addon: Addon) {
        self.addons.retain(|a| a.addon.id != addon.id);
        for a in &mut self.addons {
            a.addon.folders.retain(|f| !addon.folders.contains(f));
        }
        self.addons.retain(|a| !a.addon.folders.is_empty());
        self.addons.push(AddonUi { addon, op: None });
        self.addons.sort_by_key(|a| a.addon.name.to_lowercase());
    }

    pub(super) fn start_scan(&mut self) {
        self.start_scan_after(|_, _| {});
    }

    /// Scans after `first` ran on the same worker (e.g. writing the key the scan reads).
    pub(super) fn start_scan_after(
        &mut self,
        first: impl FnOnce(&dyn Backend, &worker::Post) + Send + 'static,
    ) {
        // Any scan reads the newest key, so a rescan waiting for one is done.
        self.rescan_pending = false;
        let Some(inst) = self.installation().cloned() else {
            return worker::spawn(self, "settings", first);
        };
        self.job = Job::Scanning;
        worker::spawn(self, "scan", move |b, post| {
            first(b, post);
            post.send(Msg::Scanned {
                result: b.scan(&inst).map_err(err),
                inst: inst.id,
            })
        });
    }

    pub(super) fn start_check(&mut self) {
        let Some(inst) = self.installation().cloned() else {
            return;
        };
        self.job = Job::CheckingUpdates;
        let addons: Vec<Addon> = self.addons.iter().map(|a| a.addon.clone()).collect();
        self.spawn_check(inst, addons, None);
    }

    /// Checks only the addon `id` (after its channel changed, F3.3).
    pub(super) fn check_one(&mut self, id: &str) {
        let Some((inst, addon)) = self.begin_op(id, Op::Checking) else {
            return;
        };
        self.spawn_check(inst, vec![addon], Some(id.to_string()));
    }

    pub(super) fn spawn_check(
        &mut self,
        inst: Installation,
        addons: Vec<Addon>,
        only: Option<String>,
    ) {
        worker::spawn(self, "check", move |b, post| {
            post.send(Msg::Checked {
                result: b.check_updates(&inst, &addons).map_err(err),
                inst: inst.id,
                only,
            })
        });
    }

    pub(super) fn start_update(&mut self, id: &str) {
        let Some((inst, addon)) = self.begin_op(id, Op::Downloading(0.0)) else {
            return;
        };
        worker::spawn(self, "update", move |b, post| {
            let id = addon.id.clone();
            let result = b.update(&inst, &addon, &|done| {
                post.send(Msg::Progress {
                    id: id.clone(),
                    done,
                })
            });
            post.send(Msg::Updated {
                id,
                result: result.map_err(err),
            });
        });
    }

    /// Installs a zip the user fetched by hand as the addon's update (F3.5); `remove` deletes it
    /// afterwards (one Culm found in Downloads, not one the user picked).
    pub(super) fn install_zip(&mut self, id: &str, path: PathBuf, remove: bool) {
        // The wait for this zip ends with it.
        if let Some(a) = self
            .addon_mut(id)
            .filter(|a| matches!(a.op, Some(Op::AwaitingDownload { .. })))
        {
            a.op = None;
        }
        let Some((inst, addon)) = self.begin_op(id, Op::Installing) else {
            return;
        };
        worker::spawn(self, "install-zip", move |b, post| {
            post.send(Msg::Updated {
                id: addon.id.clone(),
                result: b
                    .install_downloaded(&inst, &addon, &path, remove)
                    .map_err(err),
            });
        });
    }

    /// Ends waits that ran out, and looks in Downloads for the awaited zips when due (F3.5,
    /// F4.3). Returns when to look again.
    pub(super) fn look_for_downloads(&mut self) -> Option<f64> {
        let clock = self.clock;
        let mut waiting = vec![];
        for a in &mut self.addons {
            if let Some(Op::AwaitingDownload { until, .. }) = a.op {
                if clock >= until {
                    a.op = None;
                } else if let Some(file) = a.addon.latest.clone() {
                    waiting.push((a.addon.id.clone(), file));
                }
            }
        }
        let fetching = &mut self.fetching;
        self.installing.retain(|key, op| match op {
            Op::AwaitingDownload { until, .. } if clock >= *until => {
                fetching.remove(key);
                false
            }
            Op::AwaitingDownload { .. } => {
                if let Some(file) = fetching.get(key).and_then(|r| r.latest.clone()) {
                    waiting.push((key.clone(), file));
                }
                true
            }
            _ => true,
        });
        if waiting.is_empty() {
            self.downloads_due = None;
            return None;
        }
        let due = *self.downloads_due.get_or_insert(clock);
        if clock < due {
            return Some(due);
        }
        let inst = self.installation().map(|i| i.id.clone())?;
        self.downloads_due = None;
        worker::spawn(self, "downloads", move |b, post| {
            let looked = waiting.iter().map(|(id, _)| id.clone()).collect();
            let found = waiting
                .into_iter()
                .filter_map(|(id, file)| b.find_download(&file).map(|path| (id, path)))
                .collect();
            post.send(Msg::Downloads {
                inst,
                looked,
                found,
            });
        });
        None
    }

    pub(super) fn save_addon(&mut self, id: &str) {
        let (Some(inst), Some(a)) = (self.installation().cloned(), self.addon(id)) else {
            return;
        };
        let addon = a.addon.clone();
        worker::spawn(self, "save-addon", move |b, post| {
            if let Err(e) = b.save_addon(&inst, &addon) {
                post.send(Msg::Failed {
                    title: format!("Could not save {}", addon.name),
                    error: err(e),
                });
            }
        });
    }

    pub(super) fn update_all(&mut self) {
        let ids: Vec<String> = self
            .addons
            .iter()
            .filter(|a| a.status() == Status::UpdateAvailable)
            .map(|a| a.addon.id.clone())
            .collect();
        for id in ids {
            self.start_update(&id);
        }
    }

    pub(super) fn fetch_by_hand(&mut self, id: &str, tx: &mut Tx) {
        let clock = self.clock;
        let Some(a) = self.addon_mut(id) else {
            return;
        };
        if a.op.as_ref().is_some_and(Op::busy) {
            return;
        }
        if a.can_install_by_hand() {
            // A copy may be in Downloads already: the first look decides on the browser.
            a.op = Some(Op::AwaitingDownload {
                until: clock + AWAIT_DOWNLOAD_SECS,
                opened: false,
            });
            self.downloads_due = Some(clock);
        } else if let Some(url) = a.manual_url() {
            let _ = tx.open_url(&url);
        }
    }

    pub(super) fn stop_waiting(&mut self, id: String) {
        if let Some(a) = self.addon_mut(&id)
            && matches!(a.op, Some(Op::AwaitingDownload { .. }))
        {
            a.op = None;
        } else if matches!(self.installing.get(&id), Some(Op::AwaitingDownload { .. })) {
            self.installing.remove(&id);
            self.fetching.remove(&id);
        }
    }

    pub(super) fn open_folder(&mut self, id: &str) {
        let (Some(inst), Some(a)) = (self.installation(), self.addon(id)) else {
            return;
        };
        let dir = inst
            .addons_dir()
            .join(a.addon.folders.first().cloned().unwrap_or_default());
        worker::spawn(self, "open-folder", move |b, post| {
            if let Err(e) = b.open_folder(&dir) {
                post.send(Msg::Failed {
                    title: "Could not open folder".into(),
                    error: err(e),
                });
            }
        });
    }

    pub(super) fn remove(&mut self, id: String) {
        self.confirm_remove = None;
        let Some((inst, addon)) = self.begin_op(&id, Op::Removing) else {
            return;
        };
        worker::spawn(self, "remove", move |b, post| {
            post.send(Msg::Removed {
                id,
                result: b.remove(&inst, &addon).map_err(err),
            })
        });
    }
}
