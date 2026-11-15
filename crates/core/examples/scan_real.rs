//! READ-ONLY sanity run against the real machine: detects Forever installs under the default
//! roots, scans their AddOns (TOC + fingerprints, no network) and maps WowUp's records.
//! `cargo run -p addon-core --example scan_real`

use addon_core::{scan, store, wow, wowup_import};
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let t = Instant::now();
    let installs = wow::detect_installations(&wow::default_search_roots());
    println!(
        "detected {} install(s) in {:?}",
        installs.len(),
        t.elapsed()
    );
    for inst in &installs {
        println!(
            "\n== {} [{}] {} version {:?}",
            inst.label,
            inst.id,
            inst.root.display(),
            inst.version
        );
        println!("   game running: {}", wow::is_game_running(inst));
        let t = Instant::now();
        let folders = scan::scan_folders(inst)?;
        println!("   scanned {} folders in {:?}", folders.len(), t.elapsed());
        for f in &folders {
            let toc = f.toc.as_ref();
            println!(
                "   {:<34} {:>10}  {:<24} {:<10} {}",
                f.name,
                f.fingerprint,
                toc.map_or("-", |t| t.file_name.as_str()),
                toc.and_then(|t| t.forever_game_version())
                    .unwrap_or_default(),
                toc.and_then(|t| t.version.clone()).unwrap_or_default(),
            );
        }
        let local = scan::build_addons(inst.flavor, &folders, &[]).addons;
        println!("   {} local groups (no provider):", local.len());
        for a in &local {
            println!("   - {} {:?}", a.name, a.folders);
        }
        for dir in wowup_import::default_wowup_dirs() {
            let imported = wowup_import::import_wowup(&dir, inst)?;
            println!(
                "   WowUp import from {}: {} addons",
                dir.display(),
                imported.len()
            );
            for a in &imported {
                println!(
                    "   * {} ({} {}) {:?} v{}",
                    a.name,
                    a.provider.as_deref().unwrap_or("?"),
                    a.external_id.as_deref().unwrap_or("?"),
                    a.folders,
                    a.installed_version.as_deref().unwrap_or("?")
                );
            }
            let merged = store::merge_rescan(&imported, local.clone());
            let linked = merged.iter().filter(|a| a.provider.is_some()).count();
            println!(
                "   merged with scan: {} entries, {} provider-linked, {} unknown",
                merged.len(),
                linked,
                merged.len() - linked
            );
        }
    }
    Ok(())
}
