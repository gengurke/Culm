//! The window's one `State`, its explicit phases, every user intent ([`dispatch`]) and every
//! worker result ([`State::apply`]). Views only read this; handlers call `dispatch`.
//!
//! The two routers stay here, one `match` each; what a screen does lives in its file:
//! `addons` (My Addons), `get_addons`, `maintenance` and `app_update`.

mod addons;
mod app_update;
mod get_addons;
mod maintenance;

pub use addons::{AddonUi, Op, Saves, Status, manual_url};
pub use app_update::AppUpdate;
pub use get_addons::{
    Detail, Fetch, Input, LinkAnswer, SearchAnswer, by_hand_file, install_key, remote_key, single,
};
pub use maintenance::{MaintCard, MaintPhase};

use crate::backend::{AddonOrder, Backend, Backup, Found, Settings, SortBy};
use crate::theme::ThemeId;
use crate::worker::{self, Msg};
use addon_core::discover::Entry;
use addon_core::{Addon, Category, Channel, IgnoreReason, Installation, RemoteAddon, Sort, manual};
use get_addons::thumbs;
use rune::{FileDialogOptions, Image, ImageCache, Latest, TextEdit, Tx, Waker};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    MyAddons,
    GetAddons,
    Maintenance,
    Settings,
    /// Every colour, text role and surface of the theme, for theme authors (scene `swatches`).
    Swatches,
}

/// The installation-wide background job (one at a time).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    /// Waiting for the first frame, then loading settings and installations.
    Booting,
    Idle,
    Scanning,
    CheckingUpdates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Success,
    Info,
    Danger,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub tone: Tone,
    pub title: String,
    pub body: String,
    /// Clock time it hides at.
    pub due: f64,
}

/// The one open popup: a Get Addons dropdown or the sidebar's installation menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Category,
    Sort,
    Installation,
}

/// Everything that starts work or changes data. Views call [`dispatch`] with these; UI-only state
/// (selection, open menus, toggles) is set in the handler directly.
#[derive(Debug, Clone)]
pub enum Intent {
    Navigate(Screen),
    SelectInstallation(String),
    AddInstallation,
    AddInstallationPath(PathBuf),
    ImportWowUp,
    Rescan,
    CheckUpdates,
    Update(String),
    UpdateAll,
    Reinstall(String),
    SetIgnored(String, bool),
    SetChannel(String, Channel),
    OpenWebsite(String),
    /// An update its author hands out only through the source's app or website (F3.5): open its
    /// download in the browser and install it once it lands in Downloads.
    FetchByHand(String),
    /// Stop waiting for that download.
    StopWaiting(String),
    /// Pick the downloaded zip in a file dialog.
    InstallFromFile(String),
    InstallZip(String, PathBuf),
    /// Pick the downloaded zip of a search result fetched by hand (F4.3).
    InstallNewFromFile(Box<RemoteAddon>),
    InstallNewZip(Box<RemoteAddon>, PathBuf),
    OpenFolder(String),
    /// First press arms the confirm; `ConfirmRemove` does it.
    Remove(String),
    ConfirmRemove(String),
    /// The Get Addons field or a filter changed: search, browse or resolve a link.
    Search,
    /// Enter in the Get Addons field: install a resolved link, else search now.
    Submit,
    SetCategory(Option<u32>),
    SetSort(Sort),
    /// Back to every category and class and the default order (F4.6); the text stays.
    ClearFilters,
    /// Opens (or, for the open one, closes) the detail panel of a result.
    ShowRemote(Entry),
    /// Picks the detail panel's source to install from (index into its sources).
    PickSource(usize),
    CloseRemote,
    OpenUrl(String),
    Install(Box<RemoteAddon>),
    Maintain(MaintCard),
    ConfirmMaintain(MaintCard),
    SaveApiKey,
    SetDefaultChannel(Channel),
    SetTheme(ThemeId),
    /// A My Addons column header: order by it, or flip the order if it already is (F2.6).
    SortAddons(SortBy),
    /// Settings → About: ask GitHub for a newer app version (F8.4).
    CheckAppUpdate,
    /// Settings → About: copy version, system, installations and recent errors for a bug report (F7.7).
    CopyDebugInfo,
    /// Download the offered version over the running AppImage (or, if it can't, open its page).
    InstallAppUpdate,
    /// Start the updated app and quit this one.
    RestartApp,
}

pub struct State {
    pub backend: Arc<dyn Backend>,
    pub screen: Screen,
    pub job: Job,
    pub settings: Settings,
    pub installations: Vec<Installation>,
    pub current: Option<String>,

    // My Addons
    pub addons: Vec<AddonUi>,
    pub filter: TextEdit,
    pub selected: Option<String>,
    pub confirm_remove: Option<String>,

    // Get Addons
    /// The one field: search text or an install link (F4.1, F4.4).
    pub query: TextEdit,
    pub results: Fetch<Found>,
    pub featured: Fetch<Found>,
    /// The addon the link in `query` names.
    pub link: Fetch<RemoteAddon>,
    pub categories: Fetch<Vec<Category>>,
    pub category: Option<u32>,
    pub sort: Sort,
    pub menu: Option<Menu>,
    pub detail: Option<Detail>,
    /// Install progress of search results, by [`install_key`].
    pub installing: HashMap<String, Op>,
    /// Records of search results awaiting a download by hand (F4.3), by [`install_key`]; their
    /// `latest` is the file Culm looks for.
    fetching: HashMap<String, Addon>,
    pub search: Latest<SearchAnswer>,
    pub resolve: Latest<LinkAnswer>,

    // Maintenance
    pub game_running: bool,
    pub maint: HashMap<MaintCard, MaintPhase>,
    pub cache_wdb_only: bool,
    pub backups: Vec<Backup>,

    // Settings
    pub api_key: TextEdit,
    pub api_key_revealed: bool,

    /// The CurseForge key changed while a job ran: rescan once it ends.
    rescan_pending: bool,
    pub app_update: AppUpdate,

    pub toasts: Vec<Toast>,
    next_toast: u64,
    /// Decodes thumbnails and theme textures; None until `setup` (tests set their own).
    pub images: Option<ImageCache>,
    /// Thumbnail per URL: requested (None) or loaded.
    pub thumbs: HashMap<String, Option<Image>>,
    /// The original class icons per slug; None until the class chips first show (asked once per run).
    pub class_icons: Option<HashMap<String, Image>>,
    /// Seconds on the window clock as of the last poll.
    pub clock: f64,
    /// When the Downloads folder is looked at next for updates fetched by hand (F3.5); None while
    /// a look is running or nothing is awaited.
    downloads_due: Option<f64>,
    /// Pages a worker result asks to open in the browser; `poll` opens them.
    open_urls: Vec<String>,
    pub(crate) msgs: (Sender<Msg>, Receiver<Msg>),
    pub(crate) waker: Waker,
    /// The newest settings not yet written. Writers run on workers in any order: each takes the newest under
    /// the lock, so an older snapshot never overwrites a newer one (#7).
    unsaved: Arc<std::sync::Mutex<Option<Settings>>>,
    /// Record saves not yet written, in order (#136).
    pub saves: Arc<Saves>,
}

impl State {
    pub fn new(backend: Arc<dyn Backend>) -> State {
        // The theme is read before the first frame so the window never flashes the default one;
        // the rest of the settings come with the boot worker.
        let theme = backend.load_settings().theme;
        State {
            backend,
            screen: Screen::MyAddons,
            job: Job::Booting,
            settings: Settings {
                theme,
                ..Settings::default()
            },
            installations: vec![],
            current: None,
            addons: vec![],
            filter: TextEdit::new(""),
            selected: None,
            confirm_remove: None,
            query: TextEdit::new(""),
            results: Fetch::Idle,
            featured: Fetch::Idle,
            link: Fetch::Idle,
            categories: Fetch::Idle,
            category: None,
            sort: Sort::default(),
            menu: None,
            detail: None,
            installing: HashMap::new(),
            fetching: HashMap::new(),
            search: Latest::default().debounce(Duration::from_millis(300)),
            resolve: Latest::default().debounce(Duration::from_millis(300)),
            game_running: false,
            maint: HashMap::new(),
            cache_wdb_only: false,
            backups: vec![],
            api_key: TextEdit::new("").secure(),
            api_key_revealed: false,
            app_update: AppUpdate::Unknown,
            toasts: vec![],
            next_toast: 1,
            images: None,
            thumbs: HashMap::new(),
            class_icons: None,
            clock: 0.0,
            downloads_due: None,
            open_urls: vec![],
            msgs: channel(),
            waker: Arc::new(|| {}),
            unsaved: Arc::default(),
            saves: Arc::default(),
            rescan_pending: false,
        }
    }

    pub fn installation(&self) -> Option<&Installation> {
        self.installations
            .iter()
            .find(|i| Some(&i.id) == self.current.as_ref())
    }

    /// The installation's name as shown: its label, plus [`State::inst_tell`] in brackets.
    pub fn inst_name(&self, inst: &Installation) -> String {
        match self.inst_tell(inst) {
            Some(tell) => format!("{} ({tell})", inst.label),
            None => inst.label.clone(),
        }
    }

    /// What tells an installation apart while another one has the same label: its Wine prefix
    /// (`battlenet`), else its place in the list. None while its label is unique.
    pub fn inst_tell(&self, inst: &Installation) -> Option<String> {
        let twins: Vec<&Installation> = self
            .installations
            .iter()
            .filter(|i| i.label == inst.label && i.id != inst.id)
            .collect();
        if twins.is_empty() {
            return None;
        }
        let prefix = addon_core::wow::prefix_name(&inst.root);
        match prefix {
            Some(p)
                if twins
                    .iter()
                    .all(|i| addon_core::wow::prefix_name(&i.root).as_ref() != Some(&p)) =>
            {
                Some(p)
            }
            _ => {
                let n = self
                    .installations
                    .iter()
                    .position(|i| i.id == inst.id)
                    .unwrap_or(0);
                Some((n + 1).to_string())
            }
        }
    }

    pub fn curseforge_ready(&self) -> bool {
        self.settings
            .curseforge_key
            .as_deref()
            .is_some_and(|k| !k.trim().is_empty())
    }

    pub fn toast(&mut self, tone: Tone, title: impl Into<String>, body: impl Into<String>) {
        let (title, body) = (title.into(), body.into());
        if tone == Tone::Danger {
            let key = self.settings.curseforge_key.as_deref();
            addon_core::log::error(&addon_core::log::scrub(
                &format!("{title}: {body}"),
                None,
                key,
            ));
        }
        let (id, due) = (
            self.next_toast,
            self.clock + if tone == Tone::Danger { 8.0 } else { 4.5 },
        );
        self.next_toast += 1;
        self.toasts.push(Toast {
            id,
            tone,
            title,
            body,
            due,
        });
        self.toasts.drain(..self.toasts.len().saturating_sub(4));
    }

    pub fn dismiss_toast(&mut self, id: u64) {
        self.toasts.retain(|t| t.id != id);
    }

    /// Whether a result tagged with installation `inst` still belongs to the screen.
    fn is_current(&self, inst: &str) -> bool {
        self.current.as_deref() == Some(inst)
    }

    /// Lands one worker result.
    pub fn apply(&mut self, msg: Msg) {
        match msg {
            // A late answer about the installation the user switched away from.
            Msg::Scanned { ref inst, .. }
            | Msg::Checked { ref inst, .. }
            | Msg::GameRunning { ref inst, .. }
            | Msg::Maintained { ref inst, .. }
            | Msg::Backups { ref inst, .. }
            | Msg::Imported { ref inst, .. }
                if !self.is_current(inst) => {}
            Msg::Installed {
                ref inst, ref key, ..
            } if !self.is_current(inst) => {
                self.installing.remove(key);
                self.fetching.remove(key);
            }
            Msg::Booted {
                settings,
                installations,
            } => self.booted(settings, installations),
            Msg::Scanned {
                result: Ok((addons, warnings)),
                ..
            } => {
                self.set_addons(addons);
                self.warn(warnings);
                self.start_check();
            }
            Msg::Scanned { result: Err(e), .. } => {
                self.job = Job::Idle;
                self.toast(Tone::Danger, "Scan failed", e);
            }
            Msg::Checked { only, result, .. } => self.checked(only, result),
            Msg::Downloads {
                inst,
                looked,
                found,
            } => self.downloads_looked(&inst, looked, found),
            Msg::Progress { id, done } => self.progress(id, done),
            Msg::Updated { id, result } => self.updated(id, result),
            Msg::Installed { key, result, .. } => self.installed_landed(key, result),
            Msg::Removed { id, result } => self.removed(id, result),
            Msg::Featured(r) => {
                if let Ok(found) = &r {
                    self.load_thumbs(thumbs(found));
                }
                self.featured = r.into();
            }
            Msg::Categories(r) => self.categories = r.into(),
            Msg::Description { addon, result } => {
                if let Some(d) = self
                    .detail
                    .as_mut()
                    .filter(|d| remote_key(d.addon()) == addon)
                {
                    d.description = result.into();
                }
            }
            Msg::ClassIcon { slug, bytes } => {
                if let (Some(icons), Some(cache)) = (&mut self.class_icons, &self.images) {
                    icons.insert(slug, cache.load(bytes));
                }
            }
            Msg::Thumbnail { url, bytes } => {
                let image = bytes.and_then(|b| Some(self.images.as_ref()?.load(b)));
                self.thumbs.insert(url, image);
            }
            Msg::GameRunning { running, .. } => self.game_running = running,
            Msg::Maintained { card, result, .. } => {
                let phase = match result {
                    Ok(line) => MaintPhase::Done(line),
                    Err(e) => MaintPhase::Failed(e),
                };
                self.maint.insert(card, phase);
            }
            Msg::Backups { list, .. } => self.backups = list,
            Msg::InstallationAdded(Ok(inst)) => self.installation_added(inst),
            Msg::InstallationAdded(Err(e)) => {
                self.toast(Tone::Danger, "Not a Forever installation", e)
            }
            Msg::Imported { result: Ok(n), .. } => {
                self.toast(
                    Tone::Success,
                    "Imported from WowUp",
                    format!("{n} addon records"),
                );
                self.start_scan();
            }
            Msg::Imported { result: Err(e), .. } => {
                self.toast(Tone::Danger, "WowUp import failed", e)
            }
            Msg::Failed { title, error } => self.toast(Tone::Danger, title, error),
            Msg::AppRelease { asked, result } => self.app_release(asked, result),
            Msg::AppProgress(p) => {
                if let AppUpdate::Downloading { done, .. } = &mut self.app_update {
                    *done = p;
                }
            }
            Msg::AppInstalled { version, result } => self.app_installed(version, result),
        }
    }

    fn booted(&mut self, settings: Settings, installations: Vec<Installation>) {
        // A key saved while the boot ran (Settings is open by then) wins over the one loaded before
        // it. Its save wrote the defaults around it, so the merged settings get written again.
        let key_saved = self.settings.curseforge_key.take();
        self.settings = settings;
        let rewrite = key_saved.is_some();
        if rewrite {
            self.settings.curseforge_key = key_saved;
        }
        self.api_key
            .replace_text(self.settings.curseforge_key.clone().unwrap_or_default());
        self.installations = installations;
        let saved = self
            .settings
            .installation
            .as_ref()
            .filter(|id| self.installations.iter().any(|i| &i.id == *id));
        self.current = saved.or(self.installations.first().map(|i| &i.id)).cloned();
        self.job = Job::Idle;
        if rewrite {
            let save = self.queue_settings();
            self.start_scan_after(save);
        } else if self.current.is_some() {
            self.start_scan();
        }
        // One GitHub request per start, only from builds without another way to hear of a new version.
        if self.backend.checks_app_updates() {
            self.check_app_update(false);
        }
    }

    fn installation_added(&mut self, inst: Installation) {
        self.toast(
            Tone::Success,
            "Installation added",
            inst.root.display().to_string(),
        );
        self.current = Some(inst.id.clone());
        self.settings.installation = Some(inst.id.clone());
        self.save_settings();
        self.installations.retain(|i| i.id != inst.id);
        self.installations.push(inst);
        self.addons.clear();
        self.start_scan();
    }

    fn select_installation(&mut self, id: String) {
        self.menu = None;
        if self.current.as_deref() != Some(&id) {
            self.current = Some(id.clone());
            self.settings.installation = Some(id);
            self.save_settings();
            self.addons.clear();
            self.installing
                .retain(|_, op| !matches!(op, Op::AwaitingDownload { .. }));
            self.fetching.clear();
            self.selected = None;
            self.confirm_remove = None;
            self.maint.clear();
            self.backups.clear();
            self.start_scan();
            if self.screen == Screen::Maintenance {
                self.refresh_maintenance();
            }
        }
    }

    /// Asks for a folder and adds it as an installation.
    fn add_installation(&mut self, tx: &mut Tx) {
        self.menu = None;
        let options = FileDialogOptions {
            title: "Choose your World of Warcraft folder".into(),
            ..Default::default()
        };
        tx.open_folder(options, |s: &mut State, picked, tx| match picked {
            Ok(Some(path)) => dispatch(s, Intent::AddInstallationPath(path), tx),
            Ok(None) => {}
            Err(e) => s.toast(Tone::Danger, "Folder picker unavailable", e.to_string()),
        });
    }

    fn import_wowup(&mut self) {
        let Some(inst) = self.installation().cloned() else {
            return;
        };
        worker::spawn(self, "wowup-import", move |b, post| {
            post.send(Msg::Imported {
                result: b.import_wowup(&inst).map_err(err),
                inst: inst.id,
            })
        });
    }

    fn save_api_key(&mut self) {
        let key = self.api_key.text.trim().to_string();
        self.settings.curseforge_key = (!key.is_empty()).then_some(key);
        let save = self.queue_settings();
        self.featured = Fetch::Idle;
        self.results = Fetch::Idle;
        self.categories = Fetch::Idle;
        if self.curseforge_ready() {
            self.toast(
                Tone::Success,
                "CurseForge key saved",
                "Search and updates from CurseForge are on",
            );
            // Rescan once the key is written (providers read it from the settings): fingerprints can
            // now match CurseForge, then updates get checked. A running job ends first (`poll`).
            if self.job == Job::Idle {
                self.start_scan_after(save);
            } else {
                worker::spawn(self, "settings", save);
                self.rescan_pending = true;
            }
        } else {
            worker::spawn(self, "settings", save);
            self.rescan_pending = false;
            self.toast(
                Tone::Info,
                "CurseForge key removed",
                "CurseForge search is off",
            );
        }
    }

    fn warn(&mut self, warnings: Vec<String>) {
        let body = warnings.join("\n");
        // Offline, the scan and the update check that follows report the same failure: say it once.
        if !warnings.is_empty() && !self.toasts.iter().any(|t| t.body == body) {
            self.toast(Tone::Danger, "Some sources could not be reached", body);
        }
    }

    /// Fetches thumbnails not requested yet (each URL once per run).
    fn load_thumbs(&mut self, urls: Vec<String>) {
        // Headless runs (snapshots, scripts) stay offline and deterministic: no thumbnails.
        if self.images.is_none() || rune::tooling::headless_run().is_some() {
            return;
        }
        for url in urls {
            if self.thumbs.contains_key(&url) {
                continue;
            }
            self.thumbs.insert(url.clone(), None);
            worker::spawn(self, "thumbnail", move |b, post| {
                let bytes = b.thumbnail(&url).ok();
                post.send(Msg::Thumbnail { url, bytes });
            });
        }
    }

    /// The original icon of a class slug, once loaded (F4.6).
    pub fn class_icon(&self, slug: &str) -> Option<&Image> {
        self.class_icons.as_ref()?.get(slug)
    }

    /// The loaded thumbnail for `url`, if any.
    pub fn thumb(&self, url: Option<&str>) -> Option<&Image> {
        self.thumbs.get(url?)?.as_ref()
    }

    fn save_settings(&mut self) {
        let flush = self.queue_settings();
        worker::spawn(self, "settings", move |b, post| flush(b, post));
    }

    /// Queues the current settings and returns the worker step that writes the newest queued ones.
    fn queue_settings(&mut self) -> impl FnOnce(&dyn Backend, &worker::Post) + Send + 'static {
        let unsaved = self.unsaved.clone();
        *unsaved.lock().unwrap_or_else(|e| e.into_inner()) = Some(self.settings.clone());
        move |b, post| {
            let mut slot = unsaved.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(settings) = slot.take()
                && let Err(e) = b.save_settings(&settings)
            {
                post.send(Msg::Failed {
                    title: "Could not save settings".into(),
                    error: err(e),
                });
            }
        }
    }
}

pub(crate) fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

/// Asks for a downloaded zip, then dispatches `then(path)`.
fn pick_zip(s: &State, tx: &mut Tx, then: impl Fn(PathBuf) -> Intent + 'static) {
    let options = FileDialogOptions {
        title: "Choose the downloaded zip".into(),
        directory: s.backend.downloads_dir(),
        ..Default::default()
    }
    .filter("Zip archives", &["zip"]);
    tx.open_file(options, move |s: &mut State, picked, tx| match picked {
        Ok(Some(path)) => dispatch(s, then(path), tx),
        Ok(None) => {}
        Err(e) => s.toast(Tone::Danger, "File picker unavailable", e.to_string()),
    });
}

pub fn dispatch(s: &mut State, intent: Intent, tx: &mut Tx) {
    s.clock = tx.clock();
    match intent {
        Intent::Navigate(screen) => {
            s.screen = screen;
            s.menu = None;
            match screen {
                // Results from before a key change are gone: search again.
                Screen::GetAddons if s.results == Fetch::Idle => s.start_search(),
                Screen::GetAddons => s.load_featured(),
                Screen::Maintenance => s.refresh_maintenance(),
                _ => {}
            }
        }
        Intent::SelectInstallation(id) => s.select_installation(id),
        Intent::AddInstallation => s.add_installation(tx),
        Intent::AddInstallationPath(path) => {
            worker::spawn(s, "add-installation", move |b, post| {
                post.send(Msg::InstallationAdded(
                    b.add_installation(&path).map_err(err),
                ))
            });
        }
        Intent::ImportWowUp => s.import_wowup(),
        Intent::Rescan => {
            if s.job == Job::Idle {
                s.start_scan();
            }
        }
        Intent::CheckUpdates => {
            if s.job == Job::Idle {
                s.start_check();
            }
        }
        Intent::Update(id) | Intent::Reinstall(id) => s.start_update(&id),
        Intent::UpdateAll => s.update_all(),
        Intent::SetIgnored(id, ignored) => {
            if let Some(a) = s.addon_mut(&id) {
                a.addon.ignored = ignored.then_some(IgnoreReason::User);
                a.op = None;
                s.save_addon(&id);
            }
        }
        Intent::SetChannel(id, channel) => {
            if let Some(a) = s.addon_mut(&id) {
                a.addon.channel = channel;
                s.save_addon(&id);
                s.check_one(&id);
            }
        }
        Intent::OpenWebsite(id) => {
            if let Some(url) = s.addon(&id).and_then(|a| a.addon.url.clone()) {
                let _ = tx.open_url(&url);
            }
        }
        Intent::FetchByHand(id) => s.fetch_by_hand(&id, tx),
        Intent::StopWaiting(id) => s.stop_waiting(id),
        Intent::InstallFromFile(id) => {
            pick_zip(s, tx, move |path| Intent::InstallZip(id.clone(), path))
        }
        Intent::InstallZip(id, path) => s.install_zip(&id, path, false),
        Intent::InstallNewFromFile(remote) => pick_zip(s, tx, move |path| {
            Intent::InstallNewZip(remote.clone(), path)
        }),
        Intent::InstallNewZip(remote, path) => {
            if s.is_installed(&remote) {
                return;
            }
            match manual::new_record(&remote, s.settings.default_channel) {
                Ok(record) => s.install_new_zip(install_key(&remote), record, path, false),
                Err(e) => s.toast(Tone::Danger, "Install failed", err(e)),
            }
        }
        Intent::OpenFolder(id) => s.open_folder(&id),
        Intent::Remove(id) => s.confirm_remove = Some(id),
        Intent::ConfirmRemove(id) => s.remove(id),
        Intent::Search => s.start_search(),
        Intent::Submit => match (&s.link, s.input()) {
            (Fetch::Ready(r), Input::Link(..)) => {
                dispatch(s, Intent::Install(Box::new(r.clone())), tx)
            }
            _ => s.start_search(),
        },
        Intent::SetCategory(category) => {
            s.menu = None;
            s.category = category;
            s.start_search();
        }
        Intent::SetSort(sort) => {
            s.menu = None;
            s.sort = sort;
            s.start_search();
        }
        Intent::ClearFilters => {
            s.menu = None;
            s.category = None;
            s.sort = Sort::default();
            s.start_search();
        }
        Intent::ShowRemote(entry) => {
            s.menu = None;
            s.show_remote(entry);
        }
        Intent::PickSource(index) => s.pick_source(index),
        Intent::CloseRemote => s.detail = None,
        Intent::CopyDebugInfo => {
            tx.copy(crate::debug_info::text(s));
            s.toast(
                Tone::Success,
                "Debug info copied",
                "Paste it into your bug report",
            );
        }
        Intent::OpenUrl(url) => {
            if url.starts_with("https://") || url.starts_with("http://") {
                let _ = tx.open_url(&url);
            }
        }
        Intent::Install(remote) => s.install(remote, tx),
        Intent::Maintain(card) if card.confirms() => {
            if !s.game_running {
                s.maint.insert(card, MaintPhase::Confirm);
            }
        }
        Intent::Maintain(card) | Intent::ConfirmMaintain(card) => s.maintain(card),
        Intent::SaveApiKey => s.save_api_key(),
        Intent::SetDefaultChannel(channel) => {
            s.settings.default_channel = channel;
            s.save_settings();
        }
        Intent::SortAddons(by) => {
            let o = s.settings.addon_order;
            s.settings.addon_order = AddonOrder {
                by,
                desc: o.by == by && !o.desc,
            };
            s.save_settings();
        }
        Intent::SetTheme(theme) => {
            s.settings.theme = theme;
            s.save_settings();
        }
        Intent::CheckAppUpdate => {
            if !matches!(
                s.app_update,
                AppUpdate::Checking | AppUpdate::Downloading { .. } | AppUpdate::Ready(_)
            ) {
                s.check_app_update(true);
            }
        }
        Intent::InstallAppUpdate => s.install_app_update(tx),
        Intent::RestartApp => match s.backend.launch_app() {
            Ok(()) => tx.quit(),
            Err(e) => s.toast(Tone::Danger, "Could not restart", err(e)),
        },
    }
}

/// Runs once a frame: boots after the first frame, lands worker results and expires toasts.
pub fn poll(s: &mut State, clock: f64, tx: &mut Tx) -> bool {
    s.clock = clock;
    let mut changed = false;
    if s.job == Job::Booting {
        s.job = Job::Scanning;
        worker::spawn(s, "boot", |b, post| {
            let settings = b.load_settings();
            let result = b.installations();
            match result {
                Ok(installations) => post.send(Msg::Booted {
                    settings,
                    installations,
                }),
                Err(e) => {
                    post.send(Msg::Booted {
                        settings,
                        installations: vec![],
                    });
                    post.send(Msg::Failed {
                        title: "Could not look for installations".into(),
                        error: err(e),
                    });
                }
            }
        });
        changed = true;
    }
    while let Ok(msg) = s.msgs.1.try_recv() {
        s.apply(msg);
        changed = true;
    }
    // The CurseForge key changed while a job ran: rescan now that it ended (written again first).
    if s.job == Job::Idle && std::mem::take(&mut s.rescan_pending) {
        let save = s.queue_settings();
        s.start_scan_after(save);
        changed = true;
    }
    s.load_class_icons();
    changed |= s.take_search();
    changed |= s.take_link();
    changed |= crate::theme::textures_landed();
    for url in std::mem::take(&mut s.open_urls) {
        let _ = tx.open_url(&url);
    }
    if let Some(due) = s.look_for_downloads() {
        tx.wake_at(due);
    }
    let before = s.toasts.len();
    s.toasts.retain(|t| t.due > clock);
    changed |= s.toasts.len() != before;
    if let Some(due) = s.toasts.iter().map(|t| t.due).reduce(f64::min) {
        tx.wake_at(due);
    }
    changed
}

pub fn setup(s: &mut State, waker: Waker) {
    s.search.set_waker(waker.clone());
    s.resolve.set_waker(waker.clone());
    s.images = Some(ImageCache::new(waker.clone()));
    // Headless runs make textures in the view, so the first snapshot already shows them.
    if rune::tooling::headless_run().is_none() {
        crate::theme::prewarm(&waker);
    }
    s.waker = waker;
}
