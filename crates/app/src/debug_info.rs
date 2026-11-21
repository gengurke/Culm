//! Settings → About → "Copy debug info": plain text for the bug report form's "Debug info" field (F7.7).
//! The home folder shows as `~`, the user name as `<user>`, and the CurseForge key never appears.

use crate::brand;
use crate::state::State;
use addon_core::log;

/// Backtrace lines of the latest crash that are copied (about 15 frames); the log keeps all of them.
const TRACE_LINES: usize = 30;

/// Version, system, installations and the last errors from the log: one line each, plus the start of the
/// latest crash's backtrace.
pub fn text(s: &State) -> String {
    let mut out = format!(
        "{} {}\nSystem: {}\nInstallations:\n",
        brand::APP_NAME,
        brand::version_label(),
        system(),
    );
    if s.installations.is_empty() {
        out.push_str("  none\n");
    }
    for inst in &s.installations {
        out.push_str(&format!(
            "  {} ({:?} {}): {}\n",
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
    let crash = errors.iter().rposition(|e| e.contains(" crash: "));
    for (i, e) in errors.iter().enumerate() {
        let mut lines = e.lines();
        out.push_str(&format!("  {}\n", lines.next().unwrap_or_default()));
        if Some(i) == crash {
            for line in lines.take(TRACE_LINES) {
                out.push_str(&format!("  {line}\n"));
            }
        }
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
