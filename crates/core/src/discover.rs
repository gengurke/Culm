//! Multi-source search (F4.5): asks several providers at once and groups their results into one
//! entry per addon.
//!
//! Grouping is deliberately strict ("rather a duplicate than a wrong merge"): results join only when
//! they share an exact id (`curse:<id>`, `hub:<id>`, `github:<owner/repo>`, ...). A source URL counts
//! as a `github:` id only when it is exactly a GitHub repo URL. Names, slugs, authors and folders are
//! never compared. When a merge would bring two different ids of one kind together (two CurseForge
//! projects naming the same repo), none of those results are merged.

use crate::model::{Flavor, RemoteAddon};
use crate::providers::{Provider, curseforge};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// One addon, possibly offered by several providers.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Never empty. `sources[0]` is the preferred source: CurseForge when present, else the first
    /// provider (in the order given) that has it.
    pub sources: Vec<RemoteAddon>,
    /// The exact ids the entry was grouped by, sorted (`curse:61284`, `github:questie/questie`).
    pub ids: Vec<String>,
}

impl Entry {
    pub fn preferred(&self) -> &RemoteAddon {
        &self.sources[0]
    }
}

/// Grouped results plus the providers that failed (name, error); the others still answer.
#[derive(Debug, Default)]
pub struct Discovery {
    pub entries: Vec<Entry>,
    pub errors: Vec<(&'static str, String)>,
}

/// Searches every available provider in parallel and groups the results.
pub fn search(flavor: Flavor, providers: &[Box<dyn Provider>], query: &str) -> Discovery {
    run(providers, |p| p.search(flavor, query))
}

/// The providers' landing lists (popular / recent), grouped like [`search`].
pub fn featured(flavor: Flavor, providers: &[Box<dyn Provider>]) -> Discovery {
    run(providers, |p| p.featured(flavor))
}

fn run(
    providers: &[Box<dyn Provider>],
    call: impl Fn(&dyn Provider) -> Result<Vec<RemoteAddon>> + Sync,
) -> Discovery {
    let available: Vec<&dyn Provider> = providers
        .iter()
        .filter(|p| p.available())
        .map(|p| p.as_ref())
        .collect();
    let results: Vec<Result<Vec<RemoteAddon>>> = std::thread::scope(|s| {
        let handles: Vec<_> = available.iter().map(|p| s.spawn(|| call(*p))).collect();
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err(anyhow::anyhow!("provider thread panicked")))
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
    Discovery {
        entries: group(lists),
        errors,
    }
}

/// Groups result lists (one per provider, in preference order, each ranked best first).
/// Entries are ordered by their best rank in any list, ties by list order.
pub fn group(lists: Vec<Vec<RemoteAddon>>) -> Vec<Entry> {
    // (list, rank, addon, ids); exact repeats of one provider's addon are dropped.
    let mut seen = HashSet::new();
    let items: Vec<(usize, usize, RemoteAddon, BTreeSet<String>)> = lists
        .into_iter()
        .enumerate()
        .flat_map(|(l, list)| {
            list.into_iter()
                .enumerate()
                .map(move |(rank, a)| (l, rank, a))
        })
        .filter(|(_, _, a)| seen.insert((a.provider.clone(), a.external_id.clone())))
        .map(|(l, rank, a)| {
            let ids = source_ids(&a).into_iter().collect();
            (l, rank, a, ids)
        })
        .collect();

    // Union-find over shared ids.
    let mut parent: Vec<usize> = (0..items.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut owner: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, (.., ids)) in items.iter().enumerate() {
        for id in ids {
            match owner.get(id.as_str()) {
                Some(&j) => {
                    let (a, b) = (root(&mut parent, i), root(&mut parent, j));
                    parent[a] = b;
                }
                None => {
                    owner.insert(id, i);
                }
            }
        }
    }
    let mut components: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..items.len() {
        let r = root(&mut parent, i);
        components.entry(r).or_default().push(i);
    }

    let mut groups: Vec<Vec<usize>> = Vec::new();
    for members in components.into_values() {
        let ids: BTreeSet<&String> = members.iter().flat_map(|&i| &items[i].3).collect();
        let mut kinds = HashSet::new();
        let conflict = ids.iter().any(|id| !kinds.insert(kind(id)));
        if conflict {
            groups.extend(members.into_iter().map(|i| vec![i]));
        } else {
            groups.push(members);
        }
    }

    let best = |g: &Vec<usize>| g.iter().map(|&i| (items[i].1, items[i].0)).min();
    groups.sort_by_key(best);
    groups
        .into_iter()
        .map(|mut g| {
            g.sort_by_key(|&i| {
                let (l, rank, a, _) = &items[i];
                (a.provider != curseforge::NAME, *l, *rank)
            });
            let ids: BTreeSet<String> = g.iter().flat_map(|&i| items[i].3.clone()).collect();
            Entry {
                sources: g.iter().map(|&i| items[i].2.clone()).collect(),
                ids: ids.into_iter().collect(),
            }
        })
        .collect()
}

fn kind(id: &str) -> &str {
    id.split_once(':').map_or(id, |(k, _)| k)
}

/// The exact ids a result is known by: its own provider id, plus `github:<owner/repo>` when its
/// source URL is exactly a GitHub repo.
pub fn source_ids(a: &RemoteAddon) -> Vec<String> {
    let own = match a.provider.as_str() {
        "CurseForge" => format!("curse:{}", a.external_id),
        "WowUpHub" => format!("hub:{}", a.external_id),
        "WowInterface" => format!("wowi:{}", a.external_id),
        "GitHub" => format!("github:{}", a.external_id.to_lowercase()),
        other => format!("{}:{}", other.to_lowercase(), a.external_id),
    };
    let mut ids = vec![own];
    if let Some(repo) = a.source_url.as_deref().and_then(github_repo) {
        let id = format!("github:{repo}");
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

/// `http(s)://[www.]github.com/<owner>/<repo>[/][.git]` → "owner/repo" (lowercase). Anything else
/// (deeper paths, queries, other hosts) is not a repo id.
pub fn github_repo(url: &str) -> Option<String> {
    let (scheme, rest) = url.trim().split_once("://")?;
    if !scheme.eq_ignore_ascii_case("https") && !scheme.eq_ignore_ascii_case("http") {
        return None;
    }
    let rest = rest.to_lowercase();
    let rest = rest.strip_prefix("www.").unwrap_or(&rest);
    let path = rest.strip_prefix("github.com/")?;
    let path = path.strip_suffix('/').unwrap_or(path);
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, repo) = path.split_once('/')?;
    let ok = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    (ok(owner) && ok(repo)).then(|| format!("{owner}/{repo}"))
}

#[cfg(test)]
mod tests {
    use super::github_repo;

    #[test]
    fn github_repo_urls_are_exact() {
        let r = |u| github_repo(u);
        assert_eq!(
            r("https://github.com/Questie/Questie").as_deref(),
            Some("questie/questie")
        );
        assert_eq!(r("http://www.GitHub.com/a/B.git").as_deref(), Some("a/b"));
        assert_eq!(r("https://github.com/a/b/").as_deref(), Some("a/b"));
        assert_eq!(r("https://github.com/a/b/tree/main"), None);
        assert_eq!(r("https://github.com/a"), None);
        assert_eq!(r("https://github.com/a/b?tab=readme"), None);
        assert_eq!(r("https://gitlab.com/a/b"), None);
        assert_eq!(r("github.com/a/b"), None);
    }
}
