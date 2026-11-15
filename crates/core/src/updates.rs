//! Update check (F3.1): one `get_many` batch per provider for all managed addons.

use crate::model::{Addon, Flavor};
use crate::providers::Provider;

/// Sets `latest` (newest file for the addon's channel) on every managed addon (provider-linked,
/// not ignored) and fills empty display fields from the provider. Providers that are unavailable
/// (no API key) are skipped; a failing provider is reported and does not stop the others.
pub fn check_updates(
    flavor: Flavor,
    addons: &mut [Addon],
    providers: &[Box<dyn Provider>],
) -> Vec<String> {
    let mut errors = Vec::new();
    for p in providers.iter().filter(|p| p.available()) {
        let mine = |a: &Addon| a.ignored.is_none() && a.provider.as_deref() == Some(p.name());
        let mut ids: Vec<String> = addons
            .iter()
            .filter(|a| mine(a))
            .filter_map(|a| a.external_id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        if ids.is_empty() {
            continue;
        }
        let remotes = match p.get_many(flavor, &ids) {
            Ok(r) => r,
            Err(e) => {
                errors.push(format!("{}: {e:#}", p.name()));
                continue;
            }
        };
        for a in addons.iter_mut().filter(|a| mine(a)) {
            let Some(r) = remotes
                .iter()
                .find(|r| Some(&r.external_id) == a.external_id.as_ref())
            else {
                continue;
            };
            a.latest = r.latest_for(a.channel).cloned();
            fill(&mut a.name, &r.name);
            fill(&mut a.author, &r.author);
            fill(&mut a.summary, &r.summary);
            if a.url.is_none() {
                a.url = r.url.clone();
            }
            if a.thumbnail_url.is_none() {
                a.thumbnail_url = r.thumbnail_url.clone();
            }
        }
    }
    errors
}

fn fill(field: &mut String, value: &str) {
    if field.is_empty() {
        *field = value.to_string();
    }
}
