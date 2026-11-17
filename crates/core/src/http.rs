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

type Headers<'a> = &'a [(&'a str, &'a str)];

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
    stream(url, MEMORY_LIMIT, &mut out, progress)?;
    Ok(out)
}

/// Download `url` into `out` without holding it in memory (addon zips). Returns the byte count.
pub fn download_to(
    url: &str,
    out: &mut dyn Write,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<u64> {
    stream(url, FILE_LIMIT, out, progress)
}

fn stream(
    url: &str,
    limit: u64,
    out: &mut dyn Write,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<u64> {
    let res = check(url, agent().get(url).call())?;
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
