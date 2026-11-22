//! The app's own updates (F8.4): the newest GitHub release of the app, and swapping a file (the
//! running AppImage) for its asset after checking it against the release's `SHA256SUMS`.
//! Product-neutral: the app passes its repo and which asset fits.

use crate::http;
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::Path;

pub const DEFAULT_API_BASE: &str = "https://api.github.com";
/// The checksum list every release carries (`sha256sum` format).
const SUMS: &str = "SHA256SUMS";

/// A published release of the app.
#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    /// The tag without its `v` ("0.2.0").
    pub version: String,
    /// The release page, for anyone who updates by hand.
    pub page: String,
    /// The asset that fits this build, with its download URL.
    pub asset: Option<(String, String)>,
    /// Download URL of `SHA256SUMS`.
    pub sums: Option<String>,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

/// The newest release of `repo` ("owner/name") that is not a draft or pre-release; `fits` picks
/// the asset for this build by name.
pub fn latest(api_base: &str, repo: &str, fits: impl Fn(&str) -> bool) -> Result<Release> {
    let r: GhRelease = http::get_json(
        &format!("{api_base}/repos/{repo}/releases/latest"),
        &[],
        &[("Accept", "application/vnd.github+json")],
        None,
    )?;
    let url = |name: &str| {
        r.assets
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.browser_download_url.clone())
    };
    Ok(Release {
        version: r.tag_name.trim_start_matches('v').to_string(),
        page: r.html_url.clone(),
        asset: r
            .assets
            .iter()
            .find(|a| fits(&a.name))
            .map(|a| (a.name.clone(), a.browser_download_url.clone())),
        sums: url(SUMS),
    })
}

/// Whether `candidate` is a newer SemVer than `current` ("0.10.0" > "0.9.3"; "1.0.0" > "1.0.0-rc.1").
pub fn is_newer(candidate: &str, current: &str) -> bool {
    fn parse(v: &str) -> Option<([u64; 3], bool)> {
        let (core, pre) = match v.trim_start_matches('v').split_once('-') {
            Some((core, _)) => (core, true),
            None => (v.trim_start_matches('v'), false),
        };
        let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
        let n = [parts.next()??, parts.next()??, parts.next()??];
        parts.next().is_none().then_some((n, pre))
    }
    match (parse(candidate), parse(current)) {
        // Same numbers: a release beats its pre-release, never the other way round.
        (Some((a, a_pre)), Some((b, b_pre))) => a > b || (a == b && b_pre && !a_pre),
        _ => false,
    }
}

/// Downloads the release's asset next to `file`, checks it against `SHA256SUMS`, makes it
/// executable and moves it over `file` in one step. A running AppImage keeps running from the old
/// copy; the next start runs the new one. On any error `file` is left as it was.
pub fn replace(
    file: &Path,
    release: &Release,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<()> {
    let (Some((name, url)), Some(sums)) = (&release.asset, &release.sums) else {
        bail!(
            "release {} has no download for this system",
            release.version
        );
    };
    let expected = expected_sum(&http::get_text(sums, &[], None)?, name)
        .with_context(|| format!("{SUMS} of {} lists no {name}", release.version))?;
    let dir = file.parent().context("the app file has no folder")?;
    let file_name = file
        .file_name()
        .context("the app file has no name")?
        .to_string_lossy();
    let part = dir.join(format!(".{file_name}.update"));
    let result = (|| {
        let mut out = HashingWriter {
            file: fs::File::create(&part).with_context(|| format!("create {}", part.display()))?,
            hash: Sha256::new(),
        };
        http::download_to(url, None, &mut out, progress)?;
        out.file.sync_all()?;
        let got = crate::util::hex(&out.hash.finalize());
        ensure!(
            got == expected,
            "the download of {name} is damaged (checksum mismatch)"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&part, fs::Permissions::from_mode(0o755))?;
        }
        fs::rename(&part, file).with_context(|| format!("replace {}", file.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&part);
    }
    result
}

/// The lowercase hex SHA-256 listed for `name` (`<hash>  <name>`, or `<hash> *<name>`).
fn expected_sum(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, file) = line.split_once(char::is_whitespace)?;
        let file = file.trim_start().trim_start_matches('*');
        (file == name && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

struct HashingWriter {
    file: fs::File,
    hash: Sha256,
}

impl Write for HashingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.file.write(buf)?;
        self.hash.update(&buf[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_as_semver() {
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(is_newer("v0.10.0", "0.9.3"));
        assert!(is_newer("1.0.0", "1.0.0-rc.1"));
        assert!(!is_newer("1.0.0-rc.1", "1.0.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.0.9", "0.1.0"));
        assert!(!is_newer("nightly", "0.1.0"));
    }

    #[test]
    fn sums_are_found_by_name() {
        let h = "a".repeat(64);
        let sums = format!(
            "{h}  App-1.AppImage\n{}  App-1.AppImage.zsync\n",
            "b".repeat(64)
        );
        assert_eq!(expected_sum(&sums, "App-1.AppImage"), Some(h.clone()));
        assert_eq!(
            expected_sum(&format!("{h} *App-1.AppImage"), "App-1.AppImage"),
            Some(h)
        );
        assert_eq!(expected_sum(&sums, "Other.AppImage"), None);
    }
}
