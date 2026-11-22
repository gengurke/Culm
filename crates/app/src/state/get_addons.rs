//! Get Addons: search, the popular list, links, the detail panel and installs (F4).

use super::addons::AWAIT_DOWNLOAD_SECS;
use super::{AddonUi, Op, Screen, State, Tone, err, manual_url};
use crate::backend::Found;
use crate::worker::{self, Msg};
use addon_core::discover::{Entry, source_ids};
use addon_core::providers::link_source;
use addon_core::{
    Addon, Category, Channel, RemoteAddon, RemoteFile, SearchFilter, Sort, install, manual,
};
use rune::Tx;
use std::collections::HashMap;
use std::path::PathBuf;

/// The file a new install of `remote` takes ([`install::pick_file`]) when its author allows
/// downloads only through the source's own app or website, so it is fetched by hand (F4.3).
pub fn by_hand_file(remote: &RemoteAddon, channel: Channel) -> Option<&RemoteFile> {
    install::pick_file(remote, channel)
        .ok()
        .filter(|f| f.download_url.is_empty())
}

/// Remote list fetch state (search results, popular list).
#[derive(Debug, Clone, PartialEq)]
pub enum Fetch<T> {
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

impl<T> From<Result<T, String>> for Fetch<T> {
    fn from(r: Result<T, String>) -> Self {
        match r {
            Ok(v) => Fetch::Ready(v),
            Err(e) => Fetch::Failed(e),
        }
    }
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

/// The thumbnails a result list shows (the preferred source's).
pub(super) fn thumbs(found: &Found) -> Vec<String> {
    found
        .entries
        .iter()
        .filter_map(|e| e.preferred().thumbnail_url.clone())
        .collect()
}

impl State {
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

    /// Asks for the original class icons once per run, when the class chips first show
    /// (Get Addons with the category list loaded); never at startup otherwise.
    pub(super) fn load_class_icons(&mut self) {
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

    /// Installs a zip the user fetched by hand as a new addon, `record` ([`manual::new_record`])
    /// (F4.3); `remove` as in [`Self::install_zip`]. Lands as `Msg::Installed`.
    pub(super) fn install_new_zip(
        &mut self,
        key: String,
        record: Addon,
        path: PathBuf,
        remove: bool,
    ) {
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

    /// The popular list (every source) and, with a key, the category list; once per key.
    pub(super) fn load_featured(&mut self) {
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
    pub(super) fn start_search(&mut self) {
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

    pub(super) fn show_remote(&mut self, entry: Entry) {
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
    pub(super) fn pick_source(&mut self, index: usize) {
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

    /// Lands the newest link answer, if one arrived and the field still holds that link.
    pub fn take_link(&mut self) -> bool {
        let Some((url, result)) = self.resolve.take() else {
            return false;
        };
        if !matches!(self.input(), Input::Link(u, _) if u == url) {
            return false;
        }
        if let Ok(r) = &result {
            self.load_thumbs(r.thumbnail_url.clone().into_iter().collect());
        }
        self.link = result.into();
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
        if let Ok(found) = &result {
            self.load_thumbs(thumbs(found));
        }
        self.results = result.into();
        true
    }

    pub(super) fn installed_landed(&mut self, key: String, result: Result<Addon, String>) {
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

    pub(super) fn install(&mut self, remote: Box<RemoteAddon>, tx: &mut Tx) {
        let Some(inst) = self.installation().cloned() else {
            return;
        };
        let key = install_key(&remote);
        if self.installing.contains_key(&key) || self.is_installed(&remote) {
            return;
        }
        let channel = self.settings.default_channel;
        if by_hand_file(&remote, channel).is_some() {
            // Fetched by hand (F4.3): look in Downloads first, like an update (F3.5).
            let record = match manual::new_record(&remote, channel) {
                Ok(record) => record,
                Err(e) => return self.toast(Tone::Danger, "Install failed", err(e)),
            };
            if record.latest.as_ref().is_some_and(|f| f.check.is_some()) {
                self.installing.insert(
                    key.clone(),
                    Op::AwaitingDownload {
                        until: self.clock + AWAIT_DOWNLOAD_SECS,
                        opened: false,
                    },
                );
                self.fetching.insert(key, record);
                self.downloads_due = Some(self.clock);
            } else if let Some(url) = manual_url(&record) {
                let _ = tx.open_url(&url);
            }
            return;
        }
        self.installing.insert(key.clone(), Op::Downloading(0.0));
        worker::spawn(self, "install", move |b, post| {
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
}
