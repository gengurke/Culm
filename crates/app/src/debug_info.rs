//! Settings → About → "Copy debug info": plain text for the bug report form's "Debug info" field (F7.7).
//! The home folder shows as `~` and the CurseForge key never appears.

use crate::brand;
use crate::state::State;
use addon_core::log;

/// Version, system, Rune rev, installations and the last errors from the log.
pub fn text(s: &State) -> String {
    let mut out = format!(
        "{} {}\nSystem: {}\nRune: {}\nInstallations:\n",
        brand::APP_NAME,
        brand::version_label(),
        system(),
        env!("RUNE_REV"),
    );
    if s.installations.is_empty() {
        out.push_str("  none\n");
    }
    for inst in &s.installations {
        let addons = if s.current.as_ref() == Some(&inst.id) {
            format!(", {} addons", s.addons.len())
        } else {
            String::new()
        };
        out.push_str(&format!(
            "  {} ({:?} {}{addons}): {}\n",
            inst.label,
            inst.flavor,
            inst.version.as_deref().unwrap_or("version unknown"),
            inst.root.display()
        ));
    }
    out.push_str("Recent errors:\n");
    let errors = log::recent_errors();
    if errors.is_empty() {
        out.push_str("  none\n");
    }
    for e in errors {
        out.push_str(&format!("  {}\n", e.replace('\n', "\n  ")));
    }
    let home = rune::dirs::home();
    log::scrub(&out, home.as_deref(), s.settings.curseforge_key.as_deref())
}

/// OS and CPU, plus the desktop and session type on Linux (`KDE, wayland`).
fn system() -> String {
    let base = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    if cfg!(target_os = "linux") {
        let var = |k| std::env::var(k).unwrap_or_else(|_| "unknown".into());
        return format!(
            "{base}, {} ({})",
            var("XDG_CURRENT_DESKTOP"),
            var("XDG_SESSION_TYPE")
        );
    }
    base
}
