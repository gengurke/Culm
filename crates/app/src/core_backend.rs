//! The real [`Backend`]: `addon-core` over the app's own folders and prefs. Everything here runs
//! on workers; the stored addon lists are read-modify-written under one lock.

use crate::backend::{
    AddonOrder, Backend, Backup, Found, Maintenance, Settings, Warned, curseforge_only,
    sort_entries,
};
use crate::brand;
use crate::theme::ThemeId;
use crate::ui::bytes;
use addon_core::discover::{self, Discovery};
use addon_core::providers::{self, CurseForge, GitHub, Provider, ProviderConfig};
use addon_core::self_update::Release;
use addon_core::{
    Addon, Category, Channel, Flavor, Installation, RemoteAddon, RemoteFile, SearchFilter,
    class_icons, http, install, maintenance, manual, scan, self_update, store, updates, wow,
    wowup_import,
};
use anyhow::{Context, Result, bail};
use rune::prefs::Prefs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Seek;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};

/// Where the backend reads and writes. Tests build one over temp dirs and a fake server.
pub struct Config {
    /// `installations.json`, `addons/<id>.json`, `wtf-backups/<id>/`.
    pub data_dir: PathBuf,
    /// Thumbnail cache.
    pub cache_dir: PathBuf,
    pub prefs: Prefs,
    /// Where to look for Forever clients.
    pub search_roots: Vec<PathBuf>,
    /// WowUp config dirs to import records from.
    pub wowup_dirs: Vec<PathBuf>,
    /// Provider endpoints; the CurseForge key comes from the settings.
    pub providers: ProviderConfig,
    /// Base URLs of the class icons, tried in order ([`class_icons::DEFAULT_SOURCES`]).
    pub class_icon_sources: Vec<String>,
    /// GitHub API base asked for the app's own releases (F8.4).
    pub app_releases_url: String,
    /// The AppImage this process runs from (`$APPIMAGE`); None: the app can't update itself.
    pub appimage: Option<PathBuf>,
    /// The user's Downloads folder, watched for updates fetched by hand (F3.5).
    pub downloads: Option<PathBuf>,
}

impl Config {
    /// The app's XDG folders and prefs (`RUNE_DIRS_ROOT` relocates them), the usual WoW and
    /// WowUp places, and the real provider endpoints unless `ADDONS_CURSEFORGE_URL`,
    /// `ADDONS_GITHUB_URL`, `ADDONS_HUB_URL` or `ADDONS_WOWINTERFACE_URL` point elsewhere (a fake server, or a
    /// closed port for an offline run); `ADDONS_CLASS_ICON_URLS` (comma-separated bases) likewise
    /// replaces the class icon sources, and `ADDONS_APP_RELEASES_URL` the API asked for Culm's own releases.
    pub fn from_env() -> Result<Config> {
        let dirs =
            rune::dirs::AppDirs::new(brand::APP_ID).context("no home folder to keep data in")?;
        let mut providers = ProviderConfig::default();
        if let Ok(url) = std::env::var("ADDONS_CURSEFORGE_URL") {
            providers.curseforge_base_url = url;
        }
        if let Ok(url) = std::env::var("ADDONS_GITHUB_URL") {
            providers.github_base_url = url;
        }
        if let Ok(url) = std::env::var("ADDONS_HUB_URL") {
            providers.hub_base_url = url;
        }
        if let Ok(url) = std::env::var("ADDONS_WOWINTERFACE_URL") {
            providers.wowinterface_base_url = url;
        }
        let class_icon_sources = match std::env::var("ADDONS_CLASS_ICON_URLS") {
            Ok(list) => list.split(',').map(|u| u.trim().to_string()).collect(),
            Err(_) => class_icons::DEFAULT_SOURCES.map(String::from).to_vec(),
        };
        Ok(Config {
            data_dir: dirs.data().to_path_buf(),
            cache_dir: dirs.cache().to_path_buf(),
            prefs: Prefs::open(brand::APP_ID),
            search_roots: wow::default_search_roots(),
            wowup_dirs: wowup_import::default_wowup_dirs(),
            providers,
            class_icon_sources,
            app_releases_url: std::env::var("ADDONS_APP_RELEASES_URL")
                .unwrap_or_else(|_| self_update::DEFAULT_API_BASE.into()),
            // Set by the AppImage runtime to the image's own path.
            appimage: std::env::var_os("APPIMAGE").map(PathBuf::from),
            downloads: dirs.downloads().map(Path::to_path_buf),
        })
    }
}

pub struct CoreBackend {
    data_dir: PathBuf,
    cache_dir: PathBuf,
    search_roots: Vec<PathBuf>,
    wowup_dirs: Vec<PathBuf>,
    endpoints: ProviderConfig,
    class_icon_sources: Vec<String>,
    app_releases_url: String,
    appimage: Option<PathBuf>,
    downloads: Option<PathBuf>,
    prefs: Mutex<Prefs>,
    /// Serializes load-modify-save of the stored lists (workers run concurrently).
    store: Mutex<()>,
    /// AddOns folder changes (install, update, remove) share it; a scan takes it alone, so it never
    /// sees a folder mid-swap (which would drop that addon's link as "folder gone").
    disk: RwLock<()>,
}

const KEY: &str = "curseforge_key";
const CHANNEL: &str = "default_channel";
const INSTALLATION: &str = "installation";
const THEME: &str = "theme";
const ADDON_ORDER: &str = "addon_order";

impl CoreBackend {
    pub fn new(c: Config) -> CoreBackend {
        // Earlier versions stored CurseForge data here (categories, thumbnails mixed with others);
        // its terms don't allow that. Best effort: both are rebuilt when needed.
        let _ = std::fs::remove_file(c.cache_dir.join("curseforge-categories.json"));
        let _ = std::fs::remove_dir_all(c.cache_dir.join("thumbnails"));
        CoreBackend {
            data_dir: c.data_dir,
            cache_dir: c.cache_dir,
            search_roots: c.search_roots,
            wowup_dirs: c.wowup_dirs,
            endpoints: c.providers,
            class_icon_sources: c.class_icon_sources,
            app_releases_url: c.app_releases_url,
            appimage: c.appimage,
            downloads: c.downloads,
            prefs: Mutex::new(c.prefs),
            store: Mutex::new(()),
            disk: RwLock::new(()),
        }
    }

    fn provider_config(&self) -> ProviderConfig {
        ProviderConfig {
            curseforge_api_key: self.load_settings().curseforge_key,
            ..self.endpoints.clone()
        }
    }

    /// Built per call from the current settings, so a new API key applies at once.
    fn providers(&self) -> Vec<Box<dyn Provider>> {
        providers::all(&self.provider_config())
    }

    fn curseforge(&self) -> CurseForge {
        let c = self.provider_config();
        CurseForge {
            api_key: c.curseforge_api_key,
            base_url: c.curseforge_base_url,
        }
    }

    fn github(&self) -> GitHub {
        let c = self.provider_config();
        GitHub {
            token: c.github_token,
            api_base: c.github_base_url,
        }
    }

    /// Load, change and save an installation's addon list under the store lock.
    fn edit<R>(&self, inst: &Installation, f: impl FnOnce(&mut Vec<Addon>) -> R) -> Result<R> {
        let _lock = self.store.lock().unwrap_or_else(|e| e.into_inner());
        let mut list = store::load_addons(&self.data_dir, &inst.id)?;
        let r = f(&mut list);
        store::save_addons(&self.data_dir, &inst.id, &list)?;
        Ok(r)
    }

    /// WowUp's records for `inst` from the first WowUp config that has any.
    fn wowup_records(&self, inst: &Installation) -> Result<Vec<Addon>> {
        for dir in &self.wowup_dirs {
            let records = wowup_import::import_wowup(dir, inst)?;
            if !records.is_empty() {
                return Ok(records);
            }
        }
        Ok(vec![])
    }

    /// Streams `url` into an anonymous temp file in the cache (deleted when dropped), so big
    /// addons (audio packs pass 512 MB) never sit in memory.
    fn download(&self, url: &str, progress: &dyn Fn(f32)) -> Result<Box<dyn install::Zip>> {
        let dir = self.cache_dir.join("downloads");
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let mut file =
            tempfile::tempfile_in(&dir).context("creating a temp file for the download")?;
        // CurseForge's CDN wants the user's key; it never goes to any other host.
        let key = self.load_settings().curseforge_key;
        if key.is_none() && providers::curseforge::is_curseforge_host(url) {
            bail!("CurseForge downloads need an API key (Settings)");
        }
        let header = key.as_deref().map(|value| http::ScopedHeader {
            name: "x-api-key",
            value,
            to: providers::curseforge::is_curseforge_host,
        });
        http::download_to(url, header.as_ref(), &mut file, &mut |done, total| {
            if let Some(total) = total.filter(|t| *t > 0) {
                progress((done as f32 / total as f32).min(1.0));
            }
        })?;
        file.rewind().context("reading the download")?;
        Ok(Box::new(file))
    }

    fn backups_dir(&self, inst: &Installation) -> PathBuf {
        self.data_dir.join("wtf-backups").join(&inst.id)
    }

    fn save_record(&self, inst: &Installation, addon: &Addon) -> Result<()> {
        self.edit(inst, |list| store::upsert_addon(list, addon.clone()))
    }

    fn disk_shared(&self) -> std::sync::RwLockReadGuard<'_, ()> {
        self.disk.read().unwrap_or_else(|e| e.into_inner())
    }
}

/// Grouped results with the failed sources by name; an error when all `asked` sources failed.
fn found(d: Discovery, asked: usize) -> Result<Found> {
    if d.errors.len() >= asked
        && let Some((name, e)) = d.errors.first()
    {
        bail!("{}: {e}", crate::backend::source_label(name));
    }
    Ok(Found {
        entries: d.entries,
        failed: d
            .errors
            .into_iter()
            .map(|(name, e)| (name.to_string(), e))
            .collect(),
    })
}

impl Backend for CoreBackend {
    fn load_settings(&self) -> Settings {
        let prefs = self.prefs.lock().unwrap_or_else(|e| e.into_inner());
        Settings {
            curseforge_key: prefs.get::<String>(KEY).filter(|k| !k.trim().is_empty()),
            default_channel: prefs.get::<Channel>(CHANNEL).unwrap_or_default(),
            installation: prefs.get(INSTALLATION),
            theme: prefs
                .get::<String>(THEME)
                .map(|id| ThemeId::from_id(&id))
                .unwrap_or_default(),
            addon_order: prefs
                .get::<String>(ADDON_ORDER)
                .map(|id| AddonOrder::from_id(&id))
                .unwrap_or_default(),
        }
    }

    fn save_settings(&self, s: &Settings) -> Result<()> {
        let mut prefs = self.prefs.lock().unwrap_or_else(|e| e.into_inner());
        match &s.curseforge_key {
            Some(k) => prefs.set(KEY, k)?,
            None => prefs.remove(KEY)?,
        }
        prefs.set(CHANNEL, &s.default_channel)?;
        match &s.installation {
            Some(id) => prefs.set(INSTALLATION, id)?,
            None => prefs.remove(INSTALLATION)?,
        }
        prefs.set(THEME, s.theme.id())?;
        prefs.set(ADDON_ORDER, &s.addon_order.id())?;
        Ok(())
    }

    fn installations(&self) -> Result<Vec<Installation>> {
        let _lock = self.store.lock().unwrap_or_else(|e| e.into_inner());
        let mut list = store::load_installations(&self.data_dir)?;
        let stored = list.len();
        // shortcut: labels stored by earlier builds named the Wine prefix ("WoW Forever (battlenet)"); the
        // view adds it only when needed now. Drop once nobody upgrades from those.
        let mut renamed = false;
        for i in &mut list {
            if i.label.starts_with("WoW Forever (") {
                i.label = "WoW Forever".into();
                renamed = true;
            }
        }
        for found in wow::detect_installations(&self.search_roots) {
            if !list.iter().any(|i| i.id == found.id) {
                list.push(found);
            }
        }
        if renamed || list.len() != stored {
            store::save_installations(&self.data_dir, &list)?;
        }
        Ok(list)
    }

    fn add_installation(&self, path: &Path) -> Result<Installation> {
        let inst = wow::installation_from_path(path)?;
        let _lock = self.store.lock().unwrap_or_else(|e| e.into_inner());
        let mut list = store::load_installations(&self.data_dir)?;
        list.retain(|i| i.id != inst.id);
        list.push(inst.clone());
        store::save_installations(&self.data_dir, &list)?;
        Ok(inst)
    }

    fn scan(&self, inst: &Installation) -> Result<Warned<Vec<Addon>>> {
        let _disk = self.disk.write().unwrap_or_else(|e| e.into_inner());
        let fresh = scan::scan_installation(inst, &self.providers())?;
        let mut warnings = fresh.errors;
        let addons = self.edit(inst, |stored| {
            // First scan of this installation: start from WowUp's records, so provider links are
            // known without an API key.
            if stored.is_empty() {
                match self.wowup_records(inst) {
                    Ok(records) => *stored = records,
                    Err(e) => warnings.push(format!("WowUp import: {e:#}")),
                }
            }
            *stored = store::merge_rescan(stored, fresh.addons);
            stored.clone()
        })?;
        Ok((addons, warnings))
    }

    fn check_updates(&self, inst: &Installation, addons: &[Addon]) -> Result<Warned<Vec<Addon>>> {
        let mut list = addons.to_vec();
        let warnings = updates::check_updates(inst.flavor, &mut list, &self.providers());
        // Only the checked fields: an update may have finished meanwhile.
        self.edit(inst, |stored| {
            for s in stored.iter_mut() {
                if let Some(a) = list.iter().find(|a| a.id == s.id) {
                    s.latest = a.latest.clone();
                    (s.name, s.author, s.summary) =
                        (a.name.clone(), a.author.clone(), a.summary.clone());
                    (s.url, s.thumbnail_url) = (a.url.clone(), a.thumbnail_url.clone());
                }
            }
        })?;
        Ok((list, warnings))
    }

    /// Update and reinstall both install `addon.latest` (fresh from the update check). Only the
    /// stored folders no other record lists are replaced; channel / ignore changes made while it
    /// downloaded are kept.
    fn update(
        &self,
        inst: &Installation,
        addon: &Addon,
        _reinstall: bool,
        progress: &dyn Fn(f32),
    ) -> Result<Addon> {
        let _disk = self.disk_shared();
        let folders = self.edit(inst, |list| store::owned_folders(list, addon))?;
        let mut a = install::update_addon(
            inst,
            &Addon {
                folders,
                ..addon.clone()
            },
            &|url| self.download(url, progress),
        )?;
        self.edit(inst, |list| {
            if let Some(s) = list.iter().find(|s| s.id == a.id) {
                (a.channel, a.ignored, a.auto_update) = (s.channel, s.ignored, s.auto_update);
            }
            store::upsert_addon(list, a.clone());
        })?;
        Ok(a)
    }

    fn find_download(&self, file: &RemoteFile) -> Option<PathBuf> {
        manual::find_in(self.downloads.as_deref()?, file.check.as_ref()?)
            .into_iter()
            .next()
    }

    fn downloads_dir(&self) -> Option<PathBuf> {
        self.downloads.clone()
    }

    fn install_downloaded(
        &self,
        inst: &Installation,
        addon: &Addon,
        path: &Path,
        remove: bool,
    ) -> Result<Addon> {
        let _disk = self.disk_shared();
        let folders = self.edit(inst, |list| store::owned_folders(list, addon))?;
        let mut a = manual::install(
            inst,
            &Addon {
                folders,
                ..addon.clone()
            },
            path,
        )?;
        self.edit(inst, |list| {
            if let Some(s) = list.iter().find(|s| s.id == a.id) {
                (a.channel, a.ignored, a.auto_update) = (s.channel, s.ignored, s.auto_update);
            }
            store::upsert_addon(list, a.clone());
        })?;
        if remove {
            // Installed and recorded: the zip and any copy the browser saved twice have served
            // their purpose. A file that can't be deleted only stays in Downloads.
            let copies = match (
                &self.downloads,
                addon.latest.as_ref().and_then(|f| f.check.as_ref()),
            ) {
                (Some(dir), Some(check)) => manual::find_in(dir, check),
                _ => vec![],
            };
            for p in std::iter::once(path.to_path_buf()).chain(copies) {
                let _ = std::fs::remove_file(p);
            }
        }
        Ok(a)
    }

    fn install(
        &self,
        inst: &Installation,
        remote: &RemoteAddon,
        channel: Channel,
        progress: &dyn Fn(f32),
    ) -> Result<Addon> {
        let _disk = self.disk_shared();
        let a =
            install::install_remote(inst, remote, channel, &|url| self.download(url, progress))?;
        self.save_record(inst, &a)?;
        Ok(a)
    }

    /// Deletes the stored record's folders, except any another record also lists.
    fn remove(&self, inst: &Installation, addon: &Addon) -> Result<()> {
        let _disk = self.disk_shared();
        self.edit(inst, |list| {
            let folders = store::owned_folders(list, addon);
            install::remove_addon(
                inst,
                &Addon {
                    folders,
                    ..addon.clone()
                },
            )?;
            list.retain(|a| a.id != addon.id);
            anyhow::Ok(())
        })?
    }

    fn save_addon(&self, inst: &Installation, addon: &Addon) -> Result<()> {
        self.edit(inst, |list| {
            if let Some(slot) = list.iter_mut().find(|a| a.id == addon.id) {
                *slot = addon.clone();
            }
        })
    }

    fn search(&self, filter: &SearchFilter) -> Result<Found> {
        if curseforge_only(filter) {
            let list = self.curseforge().search_filtered(filter)?;
            return Ok(Found {
                entries: discover::group(vec![list]),
                failed: vec![],
            });
        }
        // Like `discover::search`, except that CurseForge also gets the sort order.
        let providers = self.providers();
        let curseforge = self.curseforge();
        let available: Vec<&dyn Provider> = providers
            .iter()
            .filter(|p| p.available())
            .map(|p| p.as_ref())
            .collect();
        let results: Vec<Result<Vec<RemoteAddon>>> = std::thread::scope(|s| {
            let handles: Vec<_> = available
                .iter()
                .map(|p| {
                    let cf = &curseforge;
                    s.spawn(move || {
                        if p.name() == providers::curseforge::NAME {
                            cf.search_filtered(filter)
                        } else {
                            p.search(Flavor::Forever, &filter.text)
                        }
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| {
                    h.join()
                        .unwrap_or_else(|_| Err(anyhow::anyhow!("search thread panicked")))
                })
                .collect()
        });
        let mut lists = Vec::new();
        let mut errors = Vec::new();
        for (p, r) in available.iter().zip(results) {
            match r {
                Ok(list) => lists.push(list),
                Err(e) => errors.push((p.name(), format!("{e:#}"))),
            }
        }
        let mut found = found(
            Discovery {
                entries: discover::group(lists),
                errors,
            },
            available.len(),
        )?;
        sort_entries(&mut found.entries, filter.sort);
        Ok(found)
    }

    /// Fetched fresh: CurseForge's terms don't allow storing API data. The UI asks once per run.
    fn categories(&self) -> Result<Vec<Category>> {
        self.curseforge().categories()
    }

    fn description(&self, remote: &RemoteAddon) -> Result<Option<String>> {
        let text = match remote.provider.as_str() {
            providers::curseforge::NAME => self.curseforge().description(&remote.external_id)?,
            providers::github::NAME => self.github().readme(&remote.external_id)?,
            // Hub addons are GitHub repos: their README is the long description.
            providers::hub::NAME => match remote.source_url.as_deref() {
                Some(repo) if discover::github_repo(repo).is_some() => {
                    self.github().readme(repo)?
                }
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        Ok((!text.is_empty()).then_some(text))
    }

    fn featured(&self) -> Result<Found> {
        let providers = self.providers();
        let asked = providers.iter().filter(|p| p.available()).count();
        found(discover::featured(Flavor::Forever, &providers), asked)
    }

    fn resolve_url(&self, url: &str) -> Result<RemoteAddon> {
        for p in self.providers() {
            if let Some(addon) = p.from_url(Flavor::Forever, url)? {
                return Ok(addon);
            }
        }
        bail!("Not a CurseForge, WowUp Hub or GitHub addon URL")
    }

    fn game_running(&self, inst: &Installation) -> bool {
        wow::is_game_running(inst)
    }

    fn maintenance(&self, inst: &Installation, job: Maintenance) -> Result<String> {
        Ok(match job {
            Maintenance::ClearCache { wdb_only } => {
                let c = maintenance::clear_cache(inst, wdb_only)?;
                format!(
                    "Freed {} from {}",
                    bytes(c.bytes),
                    if wdb_only { "Cache/WDB" } else { "Cache" }
                )
            }
            Maintenance::CleanBackups => {
                let c = maintenance::clean_backup_files(inst)?;
                format!(
                    "Removed {} .bak and .old files ({})",
                    c.files,
                    bytes(c.bytes)
                )
            }
            Maintenance::BackupWtf => {
                let path = maintenance::backup_wtf(inst, &self.backups_dir(inst))?;
                format!(
                    "Saved {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                )
            }
        })
    }

    fn backups(&self, inst: &Installation) -> Result<Vec<Backup>> {
        Ok(maintenance::list_wtf_backups(&self.backups_dir(inst))?
            .into_iter()
            .map(|b| Backup {
                name: b.name(),
                created_at: b.created_rfc3339().unwrap_or_default(),
                size_bytes: b.size,
            })
            .collect())
    }

    fn import_wowup(&self, inst: &Installation) -> Result<usize> {
        if self.wowup_dirs.is_empty() {
            bail!("No WowUp settings found (~/.config/WowUpCf or ~/.config/WowUp)");
        }
        let records = self.wowup_records(inst)?;
        if records.is_empty() {
            bail!("WowUp has no addons for {}", inst.root.display());
        }
        let n = records.len();
        self.edit(inst, |list| {
            records
                .into_iter()
                .for_each(|r| store::upsert_addon(list, r))
        })?;
        Ok(n)
    }

    fn open_folder(&self, path: &Path) -> Result<()> {
        let mut child = std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .context("running xdg-open")?;
        // Reap it in the background so it doesn't linger as a zombie.
        std::thread::spawn(move || child.wait());
        Ok(())
    }

    fn thumbnail(&self, url: &str) -> Result<Vec<u8>> {
        if !url.starts_with("https://") && !url.starts_with("http://") {
            bail!("not a web URL: {url}");
        }
        let mut h = DefaultHasher::new();
        url.hash(&mut h);
        let path = self
            .cache_dir
            .join("images")
            .join(format!("{:016x}", h.finish()));
        // CurseForge's terms don't allow storing its data, so its images stay in memory (the
        // image cache) and are fetched again next run.
        let keep = !providers::curseforge::is_curseforge_host(url);
        if keep && let Ok(bytes) = std::fs::read(&path) {
            return Ok(bytes);
        }
        let bytes = http::download(url, &mut |_, _| {})?;
        if !keep {
            return Ok(bytes);
        }
        // Best effort: a failed cache write only costs a refetch next time.
        let tmp = path.with_extension("part");
        if std::fs::create_dir_all(path.parent().unwrap_or(&self.cache_dir)).is_ok()
            && std::fs::write(&tmp, &bytes).is_ok()
        {
            let _ = std::fs::rename(&tmp, &path);
        }
        Ok(bytes)
    }

    fn class_icons(&self, each: &mut dyn FnMut(&str, Option<Vec<u8>>)) {
        class_icons::load(&self.data_dir, &self.class_icon_sources, each);
    }

    fn app_release(&self) -> Result<Option<Release>> {
        let suffix = format!("-{}.AppImage", std::env::consts::ARCH);
        let release = self_update::latest(&self.app_releases_url, brand::REPO, |n| {
            n.ends_with(&suffix)
        })?;
        Ok(self_update::is_newer(&release.version, env!("CARGO_PKG_VERSION")).then_some(release))
    }

    fn self_updating(&self) -> bool {
        self.appimage.is_some()
    }

    fn install_app_release(&self, release: &Release, progress: &dyn Fn(f32)) -> Result<()> {
        let file = self
            .appimage
            .as_deref()
            .context("this build can't update itself")?;
        self_update::replace(file, release, &mut |done, total| {
            if let Some(total) = total.filter(|t| *t > 0) {
                progress((done as f32 / total as f32).min(1.0));
            }
        })
    }

    fn launch_app(&self) -> Result<()> {
        let file = self
            .appimage
            .as_deref()
            .context("this build can't restart itself")?;
        let mut cmd = std::process::Command::new(file);
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        // The AppImage runtime hands its mount to children through an inherited descriptor (fd 1023);
        // the new version would keep the old image mounted, and its helper process alive, until it
        // quits. Close everything past stdio before the new image starts. Kernels before 5.9 lack
        // close_range: then the old mount just lives a little longer, nothing breaks.
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::process::CommandExt;
            // SAFETY: close_range is async-signal-safe and touches only this child's descriptors.
            unsafe {
                cmd.pre_exec(|| {
                    libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 0u32);
                    Ok(())
                });
            }
        }
        cmd.spawn()
            .with_context(|| format!("start {}", file.display()))?;
        Ok(())
    }
}
