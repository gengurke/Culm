//! The real [`Backend`]: `addon-core` over the app's own folders and prefs. Everything here runs
//! on workers; the stored addon lists are read-modify-written under one lock.

use crate::backend::{Backend, Backup, Maintenance, Settings, Warned};
use crate::brand;
use crate::ui::bytes;
use addon_core::providers::{self, CurseForge, Provider, ProviderConfig};
use addon_core::{
    Addon, Channel, Flavor, Installation, RemoteAddon, http, install, maintenance, scan, store,
    updates, wow, wowup_import,
};
use anyhow::{Context, Result, bail};
use rune::prefs::Prefs;
use std::hash::{DefaultHasher, Hash, Hasher};
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
}

impl Config {
    /// The app's XDG folders and prefs (`RUNE_DIRS_ROOT` relocates them), the usual WoW and
    /// WowUp places, and the real provider endpoints unless `ADDONS_CURSEFORGE_URL`,
    /// `ADDONS_GITHUB_URL` or `ADDONS_WOWINTERFACE_URL` point elsewhere (a fake server, or a
    /// closed port for an offline run).
    pub fn from_env() -> Result<Config> {
        let dirs = rune::dirs::AppDirs::new(brand::APP_ID).context("no home folder to keep data in")?;
        let mut providers = ProviderConfig::default();
        if let Ok(url) = std::env::var("ADDONS_CURSEFORGE_URL") {
            providers.curseforge_base_url = url;
        }
        if let Ok(url) = std::env::var("ADDONS_GITHUB_URL") {
            providers.github_base_url = url;
        }
        if let Ok(url) = std::env::var("ADDONS_WOWINTERFACE_URL") {
            providers.wowinterface_base_url = url;
        }
        Ok(Config {
            data_dir: dirs.data().to_path_buf(),
            cache_dir: dirs.cache().to_path_buf(),
            prefs: Prefs::open(brand::APP_ID),
            search_roots: wow::default_search_roots(),
            wowup_dirs: wowup_import::default_wowup_dirs(),
            providers,
        })
    }
}

pub struct CoreBackend {
    data_dir: PathBuf,
    cache_dir: PathBuf,
    search_roots: Vec<PathBuf>,
    wowup_dirs: Vec<PathBuf>,
    endpoints: ProviderConfig,
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

impl CoreBackend {
    pub fn new(c: Config) -> CoreBackend {
        CoreBackend {
            data_dir: c.data_dir,
            cache_dir: c.cache_dir,
            search_roots: c.search_roots,
            wowup_dirs: c.wowup_dirs,
            endpoints: c.providers,
            prefs: Mutex::new(c.prefs),
            store: Mutex::new(()),
            disk: RwLock::new(()),
        }
    }

    fn provider_config(&self) -> ProviderConfig {
        ProviderConfig { curseforge_api_key: self.load_settings().curseforge_key, ..self.endpoints.clone() }
    }

    /// Built per call from the current settings, so a new API key applies at once.
    fn providers(&self) -> Vec<Box<dyn Provider>> {
        providers::all(&self.provider_config())
    }

    fn curseforge(&self) -> CurseForge {
        let c = self.provider_config();
        CurseForge { api_key: c.curseforge_api_key, base_url: c.curseforge_base_url }
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

    fn download(&self, url: &str, progress: &dyn Fn(f32)) -> Result<Vec<u8>> {
        http::download(url, &mut |done, total| {
            if let Some(total) = total.filter(|t| *t > 0) {
                progress((done as f32 / total as f32).min(1.0));
            }
        })
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

impl Backend for CoreBackend {
    fn load_settings(&self) -> Settings {
        let prefs = self.prefs.lock().unwrap_or_else(|e| e.into_inner());
        Settings {
            curseforge_key: prefs.get::<String>(KEY).filter(|k| !k.trim().is_empty()),
            default_channel: prefs.get::<Channel>(CHANNEL).unwrap_or_default(),
            installation: prefs.get(INSTALLATION),
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
        Ok(())
    }

    fn installations(&self) -> Result<Vec<Installation>> {
        let _lock = self.store.lock().unwrap_or_else(|e| e.into_inner());
        let mut list = store::load_installations(&self.data_dir)?;
        let stored = list.len();
        for found in wow::detect_installations(&self.search_roots) {
            if !list.iter().any(|i| i.id == found.id) {
                list.push(found);
            }
        }
        if list.len() != stored {
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
                    (s.name, s.author, s.summary) = (a.name.clone(), a.author.clone(), a.summary.clone());
                    (s.url, s.thumbnail_url) = (a.url.clone(), a.thumbnail_url.clone());
                }
            }
        })?;
        Ok((list, warnings))
    }

    /// Update and reinstall both install `addon.latest` (fresh from the update check). Only the
    /// stored folders no other record lists are replaced; channel / ignore changes made while it
    /// downloaded are kept.
    fn update(&self, inst: &Installation, addon: &Addon, _reinstall: bool, progress: &dyn Fn(f32)) -> Result<Addon> {
        let _disk = self.disk_shared();
        let folders = self.edit(inst, |list| store::owned_folders(list, addon))?;
        let mut a = install::update_addon(inst, &Addon { folders, ..addon.clone() }, &|url| self.download(url, progress))?;
        self.edit(inst, |list| {
            if let Some(s) = list.iter().find(|s| s.id == a.id) {
                (a.channel, a.ignored, a.auto_update) = (s.channel, s.ignored, s.auto_update);
            }
            store::upsert_addon(list, a.clone());
        })?;
        Ok(a)
    }

    fn install(&self, inst: &Installation, remote: &RemoteAddon, channel: Channel, progress: &dyn Fn(f32)) -> Result<Addon> {
        let _disk = self.disk_shared();
        let a = install::install_remote(inst, remote, channel, &|url| self.download(url, progress))?;
        self.save_record(inst, &a)?;
        Ok(a)
    }

    /// Deletes the stored record's folders, except any another record also lists.
    fn remove(&self, inst: &Installation, addon: &Addon) -> Result<()> {
        let _disk = self.disk_shared();
        self.edit(inst, |list| {
            let folders = store::owned_folders(list, addon);
            install::remove_addon(inst, &Addon { folders, ..addon.clone() })?;
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

    fn search(&self, query: &str) -> Result<Vec<RemoteAddon>> {
        self.curseforge().search(Flavor::Forever, query)
    }

    fn featured(&self) -> Result<Vec<RemoteAddon>> {
        self.curseforge().featured(Flavor::Forever)
    }

    fn resolve_url(&self, url: &str) -> Result<RemoteAddon> {
        for p in self.providers() {
            if let Some(addon) = p.from_url(Flavor::Forever, url)? {
                return Ok(addon);
            }
        }
        bail!("Not a CurseForge, GitHub or WowInterface addon URL")
    }

    fn game_running(&self, inst: &Installation) -> bool {
        wow::is_game_running(inst)
    }

    fn maintenance(&self, inst: &Installation, job: Maintenance) -> Result<String> {
        Ok(match job {
            Maintenance::ClearCache { wdb_only } => {
                let c = maintenance::clear_cache(inst, wdb_only)?;
                format!("Freed {} from {}", bytes(c.bytes), if wdb_only { "Cache/WDB" } else { "Cache" })
            }
            Maintenance::CleanBackups => {
                let c = maintenance::clean_backup_files(inst)?;
                format!("Removed {} .bak and .old files ({})", c.files, bytes(c.bytes))
            }
            Maintenance::BackupWtf => {
                let path = maintenance::backup_wtf(inst, &self.backups_dir(inst))?;
                format!("Saved {}", path.file_name().unwrap_or_default().to_string_lossy())
            }
        })
    }

    fn backups(&self, inst: &Installation) -> Result<Vec<Backup>> {
        Ok(maintenance::list_wtf_backups(&self.backups_dir(inst))?
            .into_iter()
            .map(|b| Backup { name: b.name(), created_at: b.created_rfc3339().unwrap_or_default(), size_bytes: b.size })
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
        self.edit(inst, |list| records.into_iter().for_each(|r| store::upsert_addon(list, r)))?;
        Ok(n)
    }

    fn open_folder(&self, path: &Path) -> Result<()> {
        let mut child = std::process::Command::new("xdg-open").arg(path).spawn().context("running xdg-open")?;
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
        let path = self.cache_dir.join("thumbnails").join(format!("{:016x}", h.finish()));
        if let Ok(bytes) = std::fs::read(&path) {
            return Ok(bytes);
        }
        let bytes = http::download(url, &mut |_, _| {})?;
        // Best effort: a failed cache write only costs a refetch next time.
        let tmp = path.with_extension("part");
        if std::fs::create_dir_all(path.parent().unwrap_or(&self.cache_dir)).is_ok() && std::fs::write(&tmp, &bytes).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
        Ok(bytes)
    }
}
