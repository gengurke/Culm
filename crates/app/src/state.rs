//! The window's one `State`, its explicit phases, every user intent ([`dispatch`]) and every
//! worker result ([`State::apply`]). Views only read this; handlers call `dispatch`.

use crate::backend::{Backend, Backup, Maintenance, Settings};
use crate::theme::ThemeId;
use crate::worker::{self, Msg};
use addon_core::providers::link_source;
use addon_core::{
    Addon, Category, Channel, IgnoreReason, Installation, RemoteAddon, SearchFilter, Sort,
};
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

/// A running or failed operation on one addon (or one search result being installed).
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
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
    Working,
    Ignored,
    Unknown,
    Error,
}

impl AddonUi {
    pub fn status(&self) -> Status {
        match (&self.op, self.addon.ignored) {
            (Some(Op::Failed(_)), _) => Status::Error,
            (Some(_), _) => Status::Working,
            (None, Some(IgnoreReason::Unknown)) => Status::Unknown,
            (None, Some(_)) => Status::Ignored,
            (None, None) if self.addon.update_available() => Status::UpdateAvailable,
            (None, None) if self.addon.latest.is_none() => Status::NotChecked,
            (None, None) => Status::UpToDate,
        }
    }
}

/// A maintenance card's phase (F6).
#[derive(Debug, Clone, PartialEq)]
pub enum MaintPhase {
    Idle,
    Confirm,
    Running,
    Done(String),
    Failed(String),
}

/// Which maintenance card (the cache card's WDB toggle lives in `State::cache_wdb_only`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaintCard {
    Cache,
    Leftovers,
    Wtf,
}

impl MaintCard {
    pub fn job(self, wdb_only: bool) -> Maintenance {
        match self {
            MaintCard::Cache => Maintenance::ClearCache { wdb_only },
            MaintCard::Leftovers => Maintenance::CleanBackups,
            MaintCard::Wtf => Maintenance::BackupWtf,
        }
    }
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

/// Remote list fetch state (search results, popular list).
#[derive(Debug, Clone, PartialEq)]
pub enum Fetch<T> {
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

/// An answer from the search worker: the filter it answers and the hits.
pub type SearchAnswer = (SearchFilter, Result<Vec<RemoteAddon>, String>);
/// An answer from the link worker: the link it answers and the addon it names.
pub type LinkAnswer = (String, Result<RemoteAddon, String>);

/// What the Get Addons field holds: nothing, search text, or an install link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input<'a> {
    Empty,
    Search(&'a str),
    /// A link and the provider it names ("GitHub").
    Link(&'a str, &'static str),
}

/// The open Get Addons dropdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Category,
    Sort,
}

/// The remote addon shown in the Get Addons detail panel (F4.5).
// shortcut: one source per addon; when several sources list the same addon this becomes a list.
#[derive(Debug, Clone, PartialEq)]
pub struct Detail {
    pub addon: RemoteAddon,
    /// The provider's long description, fetched when the panel opens (Ready(None): it has none).
    pub description: Fetch<Option<String>>,
}

/// `(provider, external_id)`: a remote addon's identity across lists.
pub fn remote_key(r: &RemoteAddon) -> (String, String) {
    (r.provider.clone(), r.external_id.clone())
}

/// Everything the user can ask for. Views call [`dispatch`] with these; the integration step
/// keeps this list and only swaps the [`Backend`].
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
    /// Opens (or, for the open one, closes) the detail panel of a result.
    ShowRemote(RemoteAddon),
    CloseRemote,
    OpenUrl(String),
    Install(RemoteAddon),
    Maintain(MaintCard),
    ConfirmMaintain(MaintCard),
    SaveApiKey,
    SetDefaultChannel(Channel),
    SetTheme(ThemeId),
}

pub struct State {
    pub backend: Arc<dyn Backend>,
    pub screen: Screen,
    pub job: Job,
    pub settings: Settings,
    pub installations: Vec<Installation>,
    pub current: Option<String>,
    pub inst_menu: bool,

    // My Addons
    pub addons: Vec<AddonUi>,
    pub filter: TextEdit,
    pub selected: Option<String>,
    pub confirm_remove: Option<String>,

    // Get Addons
    /// The one field: search text or an install link (F4.1, F4.4).
    pub query: TextEdit,
    pub results: Fetch<Vec<RemoteAddon>>,
    pub featured: Fetch<Vec<RemoteAddon>>,
    /// The addon the link in `query` names.
    pub link: Fetch<RemoteAddon>,
    pub categories: Fetch<Vec<Category>>,
    pub category: Option<u32>,
    pub sort: Sort,
    pub menu: Option<Menu>,
    pub detail: Option<Detail>,
    /// Install progress of search results, by external id.
    pub installing: HashMap<String, Op>,
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

    pub toasts: Vec<Toast>,
    next_toast: u64,
    /// Decodes thumbnails and theme textures; None until `setup` (tests set their own).
    pub images: Option<ImageCache>,
    /// Thumbnail per URL: requested (None) or loaded.
    pub thumbs: HashMap<String, Option<Image>>,
    /// Seconds on the window clock as of the last poll.
    pub clock: f64,
    pub(crate) msgs: (Sender<Msg>, Receiver<Msg>),
    pub(crate) waker: Waker,
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
            inst_menu: false,
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
            search: Latest::default().debounce(Duration::from_millis(300)),
            resolve: Latest::default().debounce(Duration::from_millis(300)),
            game_running: false,
            maint: HashMap::new(),
            cache_wdb_only: false,
            backups: vec![],
            api_key: TextEdit::new("").secure(),
            api_key_revealed: false,
            toasts: vec![],
            next_toast: 1,
            images: None,
            thumbs: HashMap::new(),
            clock: 0.0,
            msgs: channel(),
            waker: Arc::new(|| {}),
        }
    }

    pub fn installation(&self) -> Option<&Installation> {
        self.installations
            .iter()
            .find(|i| Some(&i.id) == self.current.as_ref())
    }

    pub fn addon(&self, id: &str) -> Option<&AddonUi> {
        self.addons.iter().find(|a| a.addon.id == id)
    }

    fn addon_mut(&mut self, id: &str) -> Option<&mut AddonUi> {
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

    pub fn curseforge_ready(&self) -> bool {
        self.settings
            .curseforge_key
            .as_deref()
            .is_some_and(|k| !k.trim().is_empty())
    }

    /// Addons matching the filter text, in display order (name).
    pub fn visible_addons(&self) -> Vec<&AddonUi> {
        let q = self.filter.text.trim().to_lowercase();
        self.addons
            .iter()
            .filter(|a| {
                q.is_empty()
                    || a.addon.name.to_lowercase().contains(&q)
                    || a.addon.author.to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn is_installed(&self, remote: &RemoteAddon) -> bool {
        self.installed(remote).is_some()
    }

    /// The installed addon a remote one is.
    pub fn installed(&self, remote: &RemoteAddon) -> Option<&AddonUi> {
        self.addons
            .iter()
            .find(|a| a.addon.external_id.as_deref() == Some(remote.external_id.as_str()))
    }

    /// What the Get Addons field holds.
    pub fn input(&self) -> Input<'_> {
        let text = self.query.text.trim();
        match link_source(text) {
            _ if text.is_empty() => Input::Empty,
            Some(source) => Input::Link(text, source),
            None => Input::Search(text),
        }
    }

    /// The search the field and filters ask for (a link searches nothing).
    pub fn search_filter(&self) -> SearchFilter {
        SearchFilter {
            text: match self.input() {
                Input::Search(t) => t.to_string(),
                _ => String::new(),
            },
            category: self.category,
            sort: self.sort,
        }
    }

    pub fn category_by_id(&self, id: u32) -> Option<&Category> {
        match &self.categories {
            Fetch::Ready(list) => list.iter().find(|c| c.id == id),
            _ => None,
        }
    }

    pub fn toast(&mut self, tone: Tone, title: impl Into<String>, body: impl Into<String>) {
        let (id, due) = (
            self.next_toast,
            self.clock + if tone == Tone::Danger { 8.0 } else { 4.5 },
        );
        self.next_toast += 1;
        self.toasts.push(Toast {
            id,
            tone,
            title: title.into(),
            body: body.into(),
            due,
        });
        self.toasts.drain(..self.toasts.len().saturating_sub(4));
    }

    pub fn dismiss_toast(&mut self, id: u64) {
        self.toasts.retain(|t| t.id != id);
    }

    /// Replaces the list (a scan result). Addons with an update or remove still running keep
    /// showing it, so they can't be started twice.
    fn set_addons(&mut self, addons: Vec<Addon>) {
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
        self.load_thumbs(
            self.addons
                .iter()
                .filter_map(|a| a.addon.thumbnail_url.clone())
                .collect(),
        );
        if self
            .selected
            .as_ref()
            .is_some_and(|id| self.addon(id).is_none())
        {
            self.selected = None;
        }
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
                ref inst,
                ref external_id,
                ..
            } if !self.is_current(inst) => {
                self.installing.remove(external_id);
            }
            Msg::Booted {
                settings,
                installations,
            } => {
                self.settings = settings;
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
                if self.current.is_some() {
                    self.start_scan();
                }
            }
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
            Msg::Checked {
                result: Ok((addons, warnings)),
                ..
            } => {
                self.job = Job::Idle;
                for fresh in addons {
                    if let Some(a) = self.addon_mut(&fresh.id) {
                        let a = &mut a.addon;
                        (a.latest, a.name, a.author, a.summary) =
                            (fresh.latest, fresh.name, fresh.author, fresh.summary);
                        (a.url, a.thumbnail_url) = (fresh.url, fresh.thumbnail_url);
                    }
                }
                self.load_thumbs(
                    self.addons
                        .iter()
                        .filter_map(|a| a.addon.thumbnail_url.clone())
                        .collect(),
                );
                self.warn(warnings);
                if let n @ 1.. = self.update_count() {
                    self.toast(
                        Tone::Info,
                        format!("{n} update{} available", if n == 1 { "" } else { "s" }),
                        "",
                    );
                }
            }
            Msg::Checked { result: Err(e), .. } => {
                self.job = Job::Idle;
                self.toast(Tone::Danger, "Update check failed", e);
            }
            Msg::Progress { id, done } => {
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
            Msg::Updated { id, result } => match result {
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
            },
            Msg::Installed {
                external_id,
                result,
                ..
            } => {
                self.installing.remove(&external_id);
                match result {
                    Ok(addon) => {
                        self.toast(
                            Tone::Success,
                            format!("Installed {}", addon.name),
                            addon.installed_version.clone().unwrap_or_default(),
                        );
                        self.upsert(addon);
                    }
                    Err(e) => self.toast(Tone::Danger, "Install failed", e),
                }
            }
            Msg::Removed { id, result } => match result {
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
            },
            Msg::Featured(r) => {
                self.featured = match r {
                    Ok(list) => {
                        self.load_thumbs(
                            list.iter()
                                .filter_map(|r| r.thumbnail_url.clone())
                                .collect(),
                        );
                        Fetch::Ready(list)
                    }
                    Err(e) => Fetch::Failed(e),
                }
            }
            Msg::Categories(r) => {
                self.categories = match r {
                    Ok(list) => Fetch::Ready(list),
                    Err(e) => Fetch::Failed(e),
                }
            }
            Msg::Description { addon, result } => {
                if let Some(d) = self
                    .detail
                    .as_mut()
                    .filter(|d| remote_key(&d.addon) == addon)
                {
                    d.description = match result {
                        Ok(text) => Fetch::Ready(text),
                        Err(e) => Fetch::Failed(e),
                    };
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
            Msg::InstallationAdded(Ok(inst)) => {
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
        }
    }

    /// Lands an installed or updated record: it owns its folders now (the store's upsert rule).
    fn upsert(&mut self, addon: Addon) {
        self.addons.retain(|a| a.addon.id != addon.id);
        for a in &mut self.addons {
            a.addon.folders.retain(|f| !addon.folders.contains(f));
        }
        self.addons.retain(|a| !a.addon.folders.is_empty());
        self.addons.push(AddonUi { addon, op: None });
        self.addons.sort_by_key(|a| a.addon.name.to_lowercase());
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

    /// The loaded thumbnail for `url`, if any.
    pub fn thumb(&self, url: Option<&str>) -> Option<&Image> {
        self.thumbs.get(url?)?.as_ref()
    }

    fn start_scan(&mut self) {
        let Some(inst) = self.installation().cloned() else {
            return;
        };
        self.job = Job::Scanning;
        worker::spawn(self, "scan", move |b, post| {
            post.send(Msg::Scanned {
                result: b.scan(&inst).map_err(err),
                inst: inst.id,
            })
        });
    }

    fn start_check(&mut self) {
        let Some(inst) = self.installation().cloned() else {
            return;
        };
        self.job = Job::CheckingUpdates;
        let addons: Vec<Addon> = self.addons.iter().map(|a| a.addon.clone()).collect();
        worker::spawn(self, "check", move |b, post| {
            post.send(Msg::Checked {
                result: b.check_updates(&inst, &addons).map_err(err),
                inst: inst.id,
            })
        });
    }

    fn start_update(&mut self, id: &str, reinstall: bool) {
        let (Some(inst), Some(a)) = (self.installation().cloned(), self.addon_mut(id)) else {
            return;
        };
        if a.op.as_ref().is_some_and(Op::busy) {
            return;
        }
        a.op = Some(Op::Downloading(0.0));
        let addon = a.addon.clone();
        worker::spawn(self, "update", move |b, post| {
            let id = addon.id.clone();
            let result = b.update(&inst, &addon, reinstall, &|done| {
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

    /// The popular list and the category list, once per key.
    fn load_featured(&mut self) {
        if !self.curseforge_ready() {
            return;
        }
        if !matches!(self.categories, Fetch::Loading | Fetch::Ready(_)) {
            self.categories = Fetch::Loading;
            worker::spawn(self, "categories", |b, post| {
                post.send(Msg::Categories(b.categories().map_err(err)))
            });
        }
        if matches!(self.featured, Fetch::Loading | Fetch::Ready(_)) {
            return;
        }
        self.featured = Fetch::Loading;
        worker::spawn(self, "featured", |b, post| {
            post.send(Msg::Featured(b.featured().map_err(err)))
        });
    }

    /// Starts what the field and filters ask for: resolve a link, search, or show the popular list.
    fn start_search(&mut self) {
        match self.input() {
            Input::Link(url, _) => {
                let url = url.to_string();
                self.search.cancel();
                self.results = Fetch::Idle;
                self.link = Fetch::Loading;
                let b = self.backend.clone();
                self.resolve.request(url, move |url| {
                    let r = b.resolve_url(&url).map_err(err);
                    (url, r)
                });
                return;
            }
            Input::Empty | Input::Search(_) => {
                self.resolve.cancel();
                self.link = Fetch::Idle;
            }
        }
        let filter = self.search_filter();
        if filter.is_default() || !self.curseforge_ready() {
            self.search.cancel();
            self.results = Fetch::Idle;
            self.load_featured();
            return;
        }
        self.results = Fetch::Loading;
        let b = self.backend.clone();
        self.search.request(filter, move |f| {
            let r = b.search(&f).map_err(err);
            (f, r)
        });
    }

    fn show_remote(&mut self, addon: RemoteAddon) {
        let key = remote_key(&addon);
        if self
            .detail
            .as_ref()
            .is_some_and(|d| remote_key(&d.addon) == key)
        {
            self.detail = None;
            return;
        }
        self.detail = Some(Detail {
            addon: addon.clone(),
            description: Fetch::Loading,
        });
        worker::spawn(self, "description", move |b, post| {
            post.send(Msg::Description {
                result: b.description(&addon).map_err(err),
                addon: key,
            })
        });
    }

    fn refresh_maintenance(&mut self) {
        let Some(inst) = self.installation().cloned() else {
            return;
        };
        worker::spawn(self, "maintenance-info", move |b, post| {
            post.send(Msg::GameRunning {
                running: b.game_running(&inst),
                inst: inst.id.clone(),
            });
            post.send(Msg::Backups {
                list: b.backups(&inst).unwrap_or_default(),
                inst: inst.id,
            });
        });
    }

    fn save_settings(&mut self) {
        let settings = self.settings.clone();
        worker::spawn(self, "settings", move |b, post| {
            if let Err(e) = b.save_settings(&settings) {
                post.send(Msg::Failed {
                    title: "Could not save settings".into(),
                    error: err(e),
                });
            }
        });
    }

    fn save_addon(&mut self, id: &str) {
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

    /// Lands the newest link answer, if one arrived and the field still holds that link.
    pub fn take_link(&mut self) -> bool {
        let Some((url, result)) = self.resolve.take() else {
            return false;
        };
        if !matches!(self.input(), Input::Link(u, _) if u == url) {
            return false;
        }
        self.link = match result {
            Ok(r) => {
                self.load_thumbs(r.thumbnail_url.clone().into_iter().collect());
                Fetch::Ready(r)
            }
            Err(e) => Fetch::Failed(e),
        };
        true
    }

    /// Lands the newest search answer, if one arrived. True when it did.
    pub fn take_search(&mut self) -> bool {
        let Some((filter, result)) = self.search.take() else {
            return false;
        };
        if filter != self.search_filter() {
            return false;
        }
        self.results = match result {
            Ok(list) => {
                self.load_thumbs(
                    list.iter()
                        .filter_map(|r| r.thumbnail_url.clone())
                        .collect(),
                );
                Fetch::Ready(list)
            }
            Err(e) => Fetch::Failed(e),
        };
        true
    }
}

pub(crate) fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

pub fn dispatch(s: &mut State, intent: Intent, tx: &mut Tx) {
    s.clock = tx.clock();
    match intent {
        Intent::Navigate(screen) => {
            s.screen = screen;
            s.inst_menu = false;
            s.menu = None;
            match screen {
                Screen::GetAddons => s.load_featured(),
                Screen::Maintenance => s.refresh_maintenance(),
                _ => {}
            }
        }
        Intent::SelectInstallation(id) => {
            s.inst_menu = false;
            if s.current.as_deref() != Some(&id) {
                s.current = Some(id.clone());
                s.settings.installation = Some(id);
                s.save_settings();
                s.addons.clear();
                s.selected = None;
                s.confirm_remove = None;
                s.maint.clear();
                s.backups.clear();
                s.start_scan();
                if s.screen == Screen::Maintenance {
                    s.refresh_maintenance();
                }
            }
        }
        Intent::AddInstallation => {
            s.inst_menu = false;
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
        Intent::AddInstallationPath(path) => {
            worker::spawn(s, "add-installation", move |b, post| {
                post.send(Msg::InstallationAdded(
                    b.add_installation(&path).map_err(err),
                ))
            });
        }
        Intent::ImportWowUp => {
            let Some(inst) = s.installation().cloned() else {
                return;
            };
            worker::spawn(s, "wowup-import", move |b, post| {
                post.send(Msg::Imported {
                    result: b.import_wowup(&inst).map_err(err),
                    inst: inst.id,
                })
            });
        }
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
        Intent::Update(id) => s.start_update(&id, false),
        Intent::Reinstall(id) => s.start_update(&id, true),
        Intent::UpdateAll => {
            let ids: Vec<String> = s
                .addons
                .iter()
                .filter(|a| a.status() == Status::UpdateAvailable)
                .map(|a| a.addon.id.clone())
                .collect();
            for id in ids {
                s.start_update(&id, false);
            }
        }
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
                // shortcut: rechecks every addon; a per-addon check can come with the real core.
                if s.job == Job::Idle {
                    s.start_check();
                }
            }
        }
        Intent::OpenWebsite(id) => {
            if let Some(url) = s.addon(&id).and_then(|a| a.addon.url.clone()) {
                let _ = tx.open_url(&url);
            }
        }
        Intent::OpenFolder(id) => {
            let (Some(inst), Some(a)) = (s.installation(), s.addon(&id)) else {
                return;
            };
            let dir = inst
                .addons_dir()
                .join(a.addon.folders.first().cloned().unwrap_or_default());
            worker::spawn(s, "open-folder", move |b, post| {
                if let Err(e) = b.open_folder(&dir) {
                    post.send(Msg::Failed {
                        title: "Could not open folder".into(),
                        error: err(e),
                    });
                }
            });
        }
        Intent::Remove(id) => s.confirm_remove = Some(id),
        Intent::ConfirmRemove(id) => {
            s.confirm_remove = None;
            let (Some(inst), Some(a)) = (s.installation().cloned(), s.addon_mut(&id)) else {
                return;
            };
            if a.op.as_ref().is_some_and(Op::busy) {
                return;
            }
            a.op = Some(Op::Removing);
            let addon = a.addon.clone();
            worker::spawn(s, "remove", move |b, post| {
                post.send(Msg::Removed {
                    id,
                    result: b.remove(&inst, &addon).map_err(err),
                })
            });
        }
        Intent::Search => s.start_search(),
        Intent::Submit => match (&s.link, s.input()) {
            (Fetch::Ready(r), Input::Link(..)) => dispatch(s, Intent::Install(r.clone()), tx),
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
        Intent::ShowRemote(remote) => {
            s.menu = None;
            s.show_remote(remote);
        }
        Intent::CloseRemote => s.detail = None,
        Intent::OpenUrl(url) => {
            if url.starts_with("https://") || url.starts_with("http://") {
                let _ = tx.open_url(&url);
            }
        }
        Intent::Install(remote) => {
            let Some(inst) = s.installation().cloned() else {
                return;
            };
            if s.installing.contains_key(&remote.external_id) || s.is_installed(&remote) {
                return;
            }
            s.installing
                .insert(remote.external_id.clone(), Op::Downloading(0.0));
            let channel = s.settings.default_channel;
            worker::spawn(s, "install", move |b, post| {
                let external_id = remote.external_id.clone();
                let result = b.install(&inst, &remote, channel, &|done| {
                    post.send(Msg::Progress {
                        id: external_id.clone(),
                        done,
                    })
                });
                post.send(Msg::Installed {
                    inst: inst.id,
                    external_id,
                    result: result.map_err(err),
                });
            });
        }
        Intent::Maintain(card) => {
            if !s.game_running {
                s.maint.insert(card, MaintPhase::Confirm);
            }
        }
        Intent::ConfirmMaintain(card) => {
            let Some(inst) = s.installation().cloned() else {
                return;
            };
            if s.game_running {
                s.maint.insert(
                    card,
                    MaintPhase::Failed("World of Warcraft is running".into()),
                );
                return;
            }
            s.maint.insert(card, MaintPhase::Running);
            let job = card.job(s.cache_wdb_only);
            worker::spawn(s, "maintenance", move |b, post| {
                let result = b.maintenance(&inst, job).map_err(err);
                post.send(Msg::Maintained {
                    inst: inst.id.clone(),
                    card,
                    result,
                });
                if card == MaintCard::Wtf {
                    post.send(Msg::Backups {
                        list: b.backups(&inst).unwrap_or_default(),
                        inst: inst.id,
                    });
                }
            });
        }
        Intent::SaveApiKey => {
            let key = s.api_key.text.trim().to_string();
            s.settings.curseforge_key = (!key.is_empty()).then_some(key);
            s.save_settings();
            s.featured = Fetch::Idle;
            s.categories = Fetch::Idle;
            if s.curseforge_ready() {
                s.toast(
                    Tone::Success,
                    "CurseForge key saved",
                    "Search and updates from CurseForge are on",
                );
                // Rescan: fingerprints can now match CurseForge, then updates get checked.
                if s.job == Job::Idle {
                    s.start_scan();
                }
            } else {
                s.toast(
                    Tone::Info,
                    "CurseForge key removed",
                    "CurseForge search is off",
                );
            }
        }
        Intent::SetDefaultChannel(channel) => {
            s.settings.default_channel = channel;
            s.save_settings();
        }
        Intent::SetTheme(theme) => {
            s.settings.theme = theme;
            s.save_settings();
        }
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
    changed |= s.take_search();
    changed |= s.take_link();
    changed |= crate::theme::textures_landed();
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
