//! The window's one `State`, its explicit phases, every user intent ([`dispatch`]) and every
//! worker result ([`State::apply`]). Views only read this; handlers call `dispatch`.

use crate::backend::{AddonOrder, Backend, Backup, Found, Maintenance, Settings, SortBy};
use crate::brand::{self, version_label};
use crate::theme::ThemeId;
use crate::worker::{self, Msg};
use addon_core::discover::{Entry, source_ids};
use addon_core::providers::link_source;
use addon_core::self_update::Release;
use addon_core::{
    Addon, Category, Channel, IgnoreReason, Installation, RemoteAddon, RemoteFile, SearchFilter,
    Sort, install, manual,
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

/// The file a new install of `remote` takes ([`install::pick_file`]) when its author allows
/// downloads only through the source's own app or website, so it is fetched by hand (F4.3).
pub fn by_hand_file(remote: &RemoteAddon, channel: Channel) -> Option<&RemoteFile> {
    install::pick_file(remote, channel)
        .ok()
        .filter(|f| f.download_url.is_empty())
}

/// How long Culm looks for an addon fetched by hand, and how often (F3.5).
const AWAIT_DOWNLOAD_SECS: f64 = 600.0;
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
pub type SearchAnswer = (SearchFilter, Result<Found, String>);
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

/// The result shown in the Get Addons detail panel (F4.5): every source that has it, and the one
/// picked to install from (F4.8).
#[derive(Debug, Clone, PartialEq)]
pub struct Detail {
    pub entry: Entry,
    /// Index into `entry.sources`; starts at the preferred one (0).
    pub source: usize,
    /// The picked source's long description, fetched when it is picked (Ready(None): it has none).
    pub description: Fetch<Option<String>>,
}

impl Detail {
    /// The picked source.
    pub fn addon(&self) -> &RemoteAddon {
        self.entry
            .sources
            .get(self.source)
            .unwrap_or_else(|| self.entry.preferred())
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

/// A single result (a resolved link) as an entry of its own.
pub fn single(addon: RemoteAddon) -> Entry {
    Entry {
        ids: source_ids(&addon),
        sources: vec![addon],
    }
}

/// `(provider, external_id)`: a remote addon's identity across lists.
pub fn remote_key(r: &RemoteAddon) -> (String, String) {
    (r.provider.clone(), r.external_id.clone())
}

/// Key of a result being installed (`installing`): ids are only unique per provider.
pub fn install_key(r: &RemoteAddon) -> String {
    format!("{}:{}", r.provider, r.external_id)
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
    pub inst_menu: bool,

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

    pub fn is_installed(&self, remote: &RemoteAddon) -> bool {
        self.installed(remote).is_some()
    }

    /// The installed addon a remote one is: same provider and id, else the one that already has
    /// all of the folders its latest file installs (the same addon from another source, which an
    /// install would replace).
    pub fn installed(&self, remote: &RemoteAddon) -> Option<&AddonUi> {
        let same = self.addons.iter().find(|a| {
            a.addon.provider.as_deref() == Some(remote.provider.as_str())
                && a.addon.external_id.as_deref() == Some(remote.external_id.as_str())
        });
        let folders = remote
            .files
            .first()
            .map_or(&[][..], |f| f.folders.as_slice());
        same.or_else(|| {
            (!folders.is_empty()).then_some(())?;
            self.addons.iter().find(|a| {
                folders
                    .iter()
                    .all(|f| a.addon.folders.iter().any(|g| g.eq_ignore_ascii_case(f)))
            })
        })
    }

    /// The installed addon a result is, from whichever of its sources.
    pub fn installed_entry(&self, entry: &Entry) -> Option<&AddonUi> {
        entry.sources.iter().find_map(|r| self.installed(r))
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

    /// The search the field and filters ask for (a link searches nothing). The filters are
    /// CurseForge's: without a key only the text counts.
    pub fn search_filter(&self) -> SearchFilter {
        let ready = self.curseforge_ready();
        SearchFilter {
            text: match self.input() {
                Input::Search(t) => t.to_string(),
                _ => String::new(),
            },
            category: self.category.filter(|_| ready),
            sort: if ready { self.sort } else { Sort::default() },
        }
    }

    /// Whether a class, category or order other than the default is picked (F4.6).
    pub fn filters_set(&self) -> bool {
        self.category.is_some() || self.sort != Sort::default()
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
                ref inst, ref key, ..
            } if !self.is_current(inst) => {
                self.installing.remove(key);
                self.fetching.remove(key);
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
                // Only a build that can update itself asks on its own (one GitHub request per start).
                if self.backend.self_updating() {
                    self.check_app_update(false);
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
            Msg::Downloads {
                inst,
                looked,
                found,
            } => {
                self.downloads_due = Some(self.clock + DOWNLOADS_CHECK_SECS);
                if self.current.as_deref() != Some(inst.as_str()) {
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
            Msg::Installed { key, result, .. } => {
                self.installing.remove(&key);
                self.fetching.remove(&key);
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
                    Ok(found) => {
                        self.load_thumbs(thumbs(&found));
                        Fetch::Ready(found)
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
                    .filter(|d| remote_key(d.addon()) == addon)
                {
                    d.description = match result {
                        Ok(text) => Fetch::Ready(text),
                        Err(e) => Fetch::Failed(e),
                    };
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
            Msg::AppRelease { asked, result } => match result {
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
            },
            Msg::AppProgress(p) => {
                if let AppUpdate::Downloading { done, .. } = &mut self.app_update {
                    *done = p;
                }
            }
            Msg::AppInstalled { version, result } => {
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

    /// Asks for the original class icons once per run, when the class chips first show
    /// (Get Addons with the category list loaded); never at startup otherwise.
    fn load_class_icons(&mut self) {
        let shown = self.screen == Screen::GetAddons && matches!(self.categories, Fetch::Ready(_));
        if !shown || self.class_icons.is_some() || self.images.is_none() {
            return;
        }
        self.class_icons = Some(HashMap::new());
        worker::spawn(self, "class icons", |b, post| {
            b.class_icons(&mut |slug, bytes| {
                if let Some(bytes) = bytes {
                    post.send(Msg::ClassIcon {
                        slug: slug.into(),
                        bytes,
                    });
                }
            })
        });
    }

    /// The original icon of a class slug, once loaded (F4.6).
    pub fn class_icon(&self, slug: &str) -> Option<&Image> {
        self.class_icons.as_ref()?.get(slug)
    }

    /// The loaded thumbnail for `url`, if any.
    pub fn thumb(&self, url: Option<&str>) -> Option<&Image> {
        self.thumbs.get(url?)?.as_ref()
    }

    fn start_scan(&mut self) {
        self.start_scan_after(|_, _| {});
    }

    /// Scans after `first` ran on the same worker (e.g. writing the key the scan reads).
    fn start_scan_after(
        &mut self,
        first: impl FnOnce(&dyn Backend, &worker::Post) + Send + 'static,
    ) {
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

    /// Installs a zip the user fetched by hand as the addon's update (F3.5); `remove` deletes it
    /// afterwards (one Culm found in Downloads, not one the user picked).
    fn install_zip(&mut self, id: &str, path: PathBuf, remove: bool) {
        let (Some(inst), Some(a)) = (self.installation().cloned(), self.addon_mut(id)) else {
            return;
        };
        if a.op
            .as_ref()
            .is_some_and(|op| op.busy() && !matches!(op, Op::AwaitingDownload { .. }))
        {
            return;
        }
        a.op = Some(Op::Installing);
        let addon = a.addon.clone();
        worker::spawn(self, "install-zip", move |b, post| {
            post.send(Msg::Updated {
                id: addon.id.clone(),
                result: b
                    .install_downloaded(&inst, &addon, &path, remove)
                    .map_err(err),
            });
        });
    }

    /// Installs a zip the user fetched by hand as a new addon, `record` ([`manual::new_record`])
    /// (F4.3); `remove` as in [`Self::install_zip`]. Lands as `Msg::Installed`.
    fn install_new_zip(&mut self, key: String, record: Addon, path: PathBuf, remove: bool) {
        let Some(inst) = self.installation().cloned() else {
            return;
        };
        if self
            .installing
            .get(&key)
            .is_some_and(|op| !matches!(op, Op::AwaitingDownload { .. }))
        {
            return;
        }
        self.installing.insert(key.clone(), Op::Installing);
        worker::spawn(self, "install-zip", move |b, post| {
            post.send(Msg::Installed {
                result: b
                    .install_downloaded(&inst, &record, &path, remove)
                    .map_err(err),
                inst: inst.id,
                key,
            });
        });
    }

    /// Ends waits that ran out, and looks in Downloads for the awaited zips when due (F3.5,
    /// F4.3). Returns when to look again.
    fn look_for_downloads(&mut self) -> Option<f64> {
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

    /// The popular list (every source) and, with a key, the category list; once per key.
    fn load_featured(&mut self) {
        if self.curseforge_ready() && !matches!(self.categories, Fetch::Loading | Fetch::Ready(_)) {
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
        if filter.is_default() {
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

    fn show_remote(&mut self, entry: Entry) {
        if self
            .detail
            .as_ref()
            .is_some_and(|d| d.entry.ids == entry.ids)
        {
            self.detail = None;
            return;
        }
        self.load_thumbs(
            entry
                .sources
                .iter()
                .filter_map(|r| r.thumbnail_url.clone())
                .collect(),
        );
        self.detail = Some(Detail {
            entry,
            source: 0,
            description: Fetch::Idle,
        });
        self.pick_source(0);
    }

    /// Picks the detail panel's source and fetches its description.
    fn pick_source(&mut self, index: usize) {
        let Some(d) = self.detail.as_mut() else {
            return;
        };
        if index >= d.entry.sources.len() || (index == d.source && d.description != Fetch::Idle) {
            return;
        }
        d.source = index;
        d.description = Fetch::Loading;
        let addon = d.addon().clone();
        let key = remote_key(&addon);
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
        let flush = self.queue_settings();
        worker::spawn(self, "settings", move |b, post| flush(b, post));
    }

    /// Asks for the app's newest release; `asked`: the user pressed the button (say "up to date").
    fn check_app_update(&mut self, asked: bool) {
        self.app_update = AppUpdate::Checking;
        worker::spawn(self, "app-update", move |b, post| {
            post.send(Msg::AppRelease {
                asked,
                result: b.app_release().map_err(err),
            })
        });
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
            Ok(found) => {
                self.load_thumbs(thumbs(&found));
                Fetch::Ready(found)
            }
            Err(e) => Fetch::Failed(e),
        };
        true
    }
}

/// The thumbnails a result list shows (the preferred source's).
fn thumbs(found: &Found) -> Vec<String> {
    found
        .entries
        .iter()
        .filter_map(|e| e.preferred().thumbnail_url.clone())
        .collect()
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
                // Results from before a key change are gone: search again.
                Screen::GetAddons if s.results == Fetch::Idle => s.start_search(),
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
                s.installing
                    .retain(|_, op| !matches!(op, Op::AwaitingDownload { .. }));
                s.fetching.clear();
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
        Intent::FetchByHand(id) => {
            let clock = s.clock;
            let Some(a) = s.addon_mut(&id) else {
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
                s.downloads_due = Some(clock);
            } else if let Some(url) = a.manual_url() {
                let _ = tx.open_url(&url);
            }
        }
        Intent::StopWaiting(id) => {
            if let Some(a) = s.addon_mut(&id)
                && matches!(a.op, Some(Op::AwaitingDownload { .. }))
            {
                a.op = None;
            } else if matches!(s.installing.get(&id), Some(Op::AwaitingDownload { .. })) {
                s.installing.remove(&id);
                s.fetching.remove(&id);
            }
        }
        Intent::InstallFromFile(id) => {
            let options = FileDialogOptions {
                title: "Choose the downloaded zip".into(),
                directory: s.backend.downloads_dir(),
                ..Default::default()
            }
            .filter("Zip archives", &["zip"]);
            tx.open_file(options, move |s: &mut State, picked, tx| match picked {
                Ok(Some(path)) => dispatch(s, Intent::InstallZip(id.clone(), path), tx),
                Ok(None) => {}
                Err(e) => s.toast(Tone::Danger, "File picker unavailable", e.to_string()),
            });
        }
        Intent::InstallZip(id, path) => s.install_zip(&id, path, false),
        Intent::InstallNewFromFile(remote) => {
            let options = FileDialogOptions {
                title: "Choose the downloaded zip".into(),
                directory: s.backend.downloads_dir(),
                ..Default::default()
            }
            .filter("Zip archives", &["zip"]);
            tx.open_file(options, move |s: &mut State, picked, tx| match picked {
                Ok(Some(path)) => dispatch(s, Intent::InstallNewZip(remote.clone(), path), tx),
                Ok(None) => {}
                Err(e) => s.toast(Tone::Danger, "File picker unavailable", e.to_string()),
            });
        }
        Intent::InstallNewZip(remote, path) => {
            if s.is_installed(&remote) {
                return;
            }
            match manual::new_record(&remote, s.settings.default_channel) {
                Ok(record) => s.install_new_zip(install_key(&remote), record, path, false),
                Err(e) => s.toast(Tone::Danger, "Install failed", err(e)),
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
        Intent::OpenUrl(url) => {
            if url.starts_with("https://") || url.starts_with("http://") {
                let _ = tx.open_url(&url);
            }
        }
        Intent::Install(remote) => {
            let Some(inst) = s.installation().cloned() else {
                return;
            };
            let key = install_key(&remote);
            if s.installing.contains_key(&key) || s.is_installed(&remote) {
                return;
            }
            let channel = s.settings.default_channel;
            if by_hand_file(&remote, channel).is_some() {
                // Fetched by hand (F4.3): look in Downloads first, like an update (F3.5).
                let Ok(record) = manual::new_record(&remote, channel) else {
                    return;
                };
                if record.latest.as_ref().is_some_and(|f| f.check.is_some()) {
                    s.installing.insert(
                        key.clone(),
                        Op::AwaitingDownload {
                            until: s.clock + AWAIT_DOWNLOAD_SECS,
                            opened: false,
                        },
                    );
                    s.fetching.insert(key, record);
                    s.downloads_due = Some(s.clock);
                } else if let Some(url) = manual_url(&record) {
                    let _ = tx.open_url(&url);
                }
                return;
            }
            s.installing.insert(key.clone(), Op::Downloading(0.0));
            worker::spawn(s, "install", move |b, post| {
                let result = b.install(&inst, &remote, channel, &|done| {
                    post.send(Msg::Progress {
                        id: key.clone(),
                        done,
                    })
                });
                post.send(Msg::Installed {
                    inst: inst.id,
                    key,
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
            let save = s.queue_settings();
            s.featured = Fetch::Idle;
            s.results = Fetch::Idle;
            s.categories = Fetch::Idle;
            if s.curseforge_ready() {
                s.toast(
                    Tone::Success,
                    "CurseForge key saved",
                    "Search and updates from CurseForge are on",
                );
                // Rescan once the key is written (providers read it from the settings): fingerprints can
                // now match CurseForge, then updates get checked.
                if s.job == Job::Idle {
                    s.start_scan_after(save);
                } else {
                    worker::spawn(s, "settings", save);
                }
            } else {
                worker::spawn(s, "settings", save);
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
        Intent::InstallAppUpdate => {
            let Some(release) = s.app_update.release().cloned() else {
                return;
            };
            if matches!(s.app_update, AppUpdate::Downloading { .. }) {
                return;
            }
            if !s.backend.self_updating() {
                let _ = tx.open_url(&release.page);
                return;
            }
            s.app_update = AppUpdate::Downloading {
                release: release.clone(),
                done: 0.0,
            };
            worker::spawn(s, "app-update", move |b, post| {
                let result = b
                    .install_app_release(&release, &|p| post.send(Msg::AppProgress(p)))
                    .map_err(err);
                post.send(Msg::AppInstalled {
                    version: release.version,
                    result,
                });
            });
        }
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
