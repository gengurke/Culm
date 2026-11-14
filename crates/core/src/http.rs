//! Tiny blocking HTTP client (ureq + rustls) shared by providers and installs.

use anyhow::{Context, Result, bail};
use serde::{Serialize, de::DeserializeOwned};
use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;

/// Max JSON response size (CurseForge fingerprint answers for big AddOns folders are a few MB).
const JSON_LIMIT: u64 = 64 << 20;
/// Max download size; addon zips are far below this.
const DOWNLOAD_LIMIT: u64 = 512 << 20;

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .user_agent(concat!("addon-manager/", env!("CARGO_PKG_VERSION")))
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

/// Download `url` into memory. `progress(done, total)` is called as bytes arrive.
pub fn download(url: &str, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<Vec<u8>> {
    let res = check(url, agent().get(url).call())?;
    let total = res.body().content_length();
    let mut reader = res
        .into_body()
        .into_with_config()
        .limit(DOWNLOAD_LIMIT)
        .reader();
    let mut out = Vec::with_capacity(total.unwrap_or(0).min(DOWNLOAD_LIMIT) as usize);
    let mut buf = [0u8; 64 * 1024];
    progress(0, total);
    loop {
        let n = reader
            .read(&mut buf)
            .with_context(|| format!("download {url}"))?;
        if n == 0 {
            break;
        }
        out.extend_from_slice(&buf[..n]);
        progress(out.len() as u64, total);
    }
    Ok(out)
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
