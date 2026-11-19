//! Product naming lives here only.
pub const APP_NAME: &str = "Culm";
/// The line under the wordmark.
pub const TAGLINE: &str = "Addon Manager";
/// Release stage shown beside the version (Settings → About); empty from 1.0 on.
pub const STAGE: &str = "Beta";
/// Where releases are published ("owner/name" on GitHub); the app checks it for its own updates (F8.4).
pub const REPO: &str = "gengurke/Culm";
/// Used for XDG dirs, prefs, Wayland app_id and single-instance lock.
pub const APP_ID: &str = "dev.culm.app";
/// Shown at the bottom of Settings → About.
pub const DISCLAIMER: &str = "World of Warcraft is a trademark of Blizzard Entertainment. Culm is a community project, \
     not affiliated with or endorsed by Blizzard.";

/// "v0.1.0 · Beta" while a release stage is set, "v1.0.0" after.
pub fn version_label() -> String {
    let v = env!("CARGO_PKG_VERSION");
    match STAGE {
        "" => format!("v{v}"),
        stage => format!("v{v} · {stage}"),
    }
}

/// The third-party licences of this version (`THIRD-PARTY-LICENSES.md` at its release tag, #52).
pub fn licences_url() -> String {
    format!(
        "https://github.com/{REPO}/blob/v{}/THIRD-PARTY-LICENSES.md",
        env!("CARGO_PKG_VERSION")
    )
}
