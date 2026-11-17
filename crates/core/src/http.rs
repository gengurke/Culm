//! Tiny blocking HTTP client (ureq + rustls) shared by providers and installs.

use anyhow::{Context, Result, bail};
use serde::{Serialize, de::DeserializeOwned};
use std::io::{Read, Write};
use std::sync::OnceLock;
use std::time::Duration;

/// Max JSON response size (CurseForge fingerprint answers for big AddOns folders are a few MB).
const JSON_LIMIT: u64 = 64 << 20;
/// Max size of a download kept in memory (thumbnails, icons).
const MEMORY_LIMIT: u64 = 64 << 20;
/// Max size of a download streamed to a file: addon zips with audio packs pass 512 MB.
const FILE_LIMIT: u64 = 8 << 30;

/// Sent with every request. Generic on purpose: the product name lives in the app crate only.
pub const USER_AGENT: &str = concat!(
    "addon-manager/",
    env!("CARGO_PKG_VERSION"),
    " (World of Warcraft: Forever addon manager)"
);

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .user_agent(USER_AGENT)
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_recv_response(Some(Duration::from_secs(30)))
            // Per read, not total: large downloads on slow links must still finish.
            .timeout_per_read(Some(Duration::from_secs(30)))
            .build()
            .into()
    })
}

/// Like `agent()`, but hands redirects back so `stream` can decide per hop which headers to send.
fn download_agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .user_agent(USER_AGENT)
            .http_status_as_error(false)
            .max_redirects(0)
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_recv_response(Some(Duration::from_secs(30)))
            // Per read, not total: large downloads on slow links must still finish.
            .timeout_per_read(Some(Duration::from_secs(30)))
            .build()
            .into()
    })
}

type Headers<'a> = &'a [(&'a str, &'a str)];

/// A header (an API key) sent only to the hosts `to` accepts, checked again on every redirect,
/// so a key never follows a redirect to someone else's server.
pub struct ScopedHeader<'a> {
    pub name: &'a str,
    pub value: &'a str,
    pub to: fn(&str) -> bool,
}

/// Redirects followed by a download before giving up.
const MAX_REDIRECTS: usize = 10;

/// GET `url` with `query` params (URL-encoded) and parse the JSON answer.
pub fn get_json<T: DeserializeOwned>(
    url: &str,
    query: &[(&str, &str)],
    headers: Headers,
) -> Result<T> {
    let mut req = agent().get(url).header("Accept", "application/json");
    for (k, v) in query {
        req = req.query(*k, *v);
    }
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    parse_json(url, req.call())
}

/// GET `url` as text (at most the JSON size limit).
pub fn get_text(url: &str, headers: Headers) -> Result<String> {
    let mut req = agent().get(url);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    check(url, req.call())?
        .body_mut()
        .with_config()
        .limit(JSON_LIMIT)
        .read_to_string()
        .with_context(|| format!("read {url}"))
}

/// POST `body` as JSON and parse the JSON answer.
pub fn post_json<T: DeserializeOwned>(
    url: &str,
    body: &impl Serialize,
    headers: Headers,
) -> Result<T> {
    let mut req = agent()
        .post(url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json");
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    parse_json(url, req.send(&serde_json::to_vec(body)?[..]))
}

/// Download `url` into memory (small files only). `progress(done, total)` is called as bytes arrive.
pub fn download(url: &str, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    stream(url, None, MEMORY_LIMIT, &mut out, progress)?;
    Ok(out)
}

/// Download `url` into `out` without holding it in memory (addon zips), sending `header` where
/// it may go. Returns the byte count.
pub fn download_to(
    url: &str,
    header: Option<&ScopedHeader>,
    out: &mut dyn Write,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<u64> {
    stream(url, header, FILE_LIMIT, out, progress)
}

fn stream(
    url: &str,
    header: Option<&ScopedHeader>,
    limit: u64,
    out: &mut dyn Write,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<u64> {
    let mut url = url.to_string();
    let mut hops = 0;
    let res = loop {
        let mut req = download_agent().get(&url);
        if let Some(h) = header.filter(|h| (h.to)(&url)) {
            req = req.header(h.name, h.value);
        }
        let res = req.call().with_context(|| format!("request {url}"))?;
        if !res.status().is_redirection() {
            break check(&url, Ok(res))?;
        }
        let location = res
            .headers()
            .get("location")
            .and_then(|l| l.to_str().ok())
            .with_context(|| format!("HTTP {} from {url} without a target", res.status()))?;
        hops += 1;
        if hops > MAX_REDIRECTS {
            bail!("too many redirects from {url}");
        }
        url = join_location(&url, location);
    };
    let url = url.as_str();
    let total = res.body().content_length();
    let too_large = || anyhow::anyhow!("the file is too large (over {} MB)", limit >> 20);
    if total.is_some_and(|t| t > limit) {
        return Err(too_large());
    }
    // The limit is checked here, so the error reads the same with or without a Content-Length.
    let mut reader = res.into_body().into_with_config().limit(u64::MAX).reader();
    let mut buf = vec![0u8; 256 * 1024];
    let mut done = 0u64;
    progress(0, total);
    loop {
        let n = reader
            .read(&mut buf)
            .with_context(|| format!("download {url}"))?;
        if n == 0 {
            break;
        }
        if done + n as u64 > limit {
            return Err(too_large());
        }
        out.write_all(&buf[..n]).context("writing the download")?;
        done += n as u64;
        progress(done, total);
    }
    out.flush().context("writing the download")?;
    Ok(done)
}

type Res = ureq::http::Response<ureq::Body>;

/// The URL a `Location` header points to, seen from `base` (absolute, `//host/…`, `/path` or relative).
fn join_location(base: &str, location: &str) -> String {
    if location.starts_with("https://") || location.starts_with("http://") {
        return location.to_string();
    }
    let (scheme, rest) = base.split_once("://").unwrap_or(("https", base));
    if let Some(net) = location.strip_prefix("//") {
        return format!("{scheme}://{net}");
    }
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    if location.starts_with('/') {
        return format!("{scheme}://{host}{location}");
    }
    let path = rest.split(['?', '#']).next().unwrap_or(rest);
    let dir = path.rsplit_once('/').map_or(host, |(d, _)| d);
    format!("{scheme}://{dir}/{location}")
}

fn check(url: &str, res: Result<Res, ureq::Error>) -> Result<Res> {
    let mut res = res.with_context(|| format!("request {url}"))?;
    let status = res.status();
    if !status.is_success() {
        let body = res
            .body_mut()
            .with_config()
            .limit(4096)
            .read_to_string()
            .unwrap_or_default();
        bail!("HTTP {status} from {url}: {}", body.trim());
    }
    Ok(res)
}

fn parse_json<T: DeserializeOwned>(url: &str, res: Result<Res, ureq::Error>) -> Result<T> {
    let bytes = check(url, res)?
        .body_mut()
        .with_config()
        .limit(JSON_LIMIT)
        .read_to_vec()
        .with_context(|| format!("read {url}"))?;
    serde_json::from_slice(&bytes).with_context(|| format!("invalid JSON from {url}"))
}

/// The host of an http(s) URL, lower-cased, without port; `None` for anything else or with user info.
pub fn host(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.contains('@') {
        return None;
    }
    let host = authority.rsplit_once(':').map_or(authority, |(h, _)| h);
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redirect_targets_resolve_like_a_browser() {
        let b = "https://a.example/files/1/x.zip?q=1";
        assert_eq!(
            join_location(b, "https://b.example/y"),
            "https://b.example/y"
        );
        assert_eq!(join_location(b, "//c.example/z"), "https://c.example/z");
        assert_eq!(join_location(b, "/top.zip"), "https://a.example/top.zip");
        assert_eq!(join_location(b, "y.zip"), "https://a.example/files/1/y.zip");
    }

    #[test]
    fn hosts_drop_ports_and_reject_user_info() {
        assert_eq!(
            host("https://Edge.ForgeCDN.net:443/f").as_deref(),
            Some("edge.forgecdn.net")
        );
        assert_eq!(host("https://edge.forgecdn.net@evil.example/f"), None);
        assert_eq!(host("ftp://edge.forgecdn.net/f"), None);
    }
}
