//! Test helpers shared by `addon-core` and `addon-app` tests:
//! a local fake HTTP server, provider-shaped JSON builders, fixtures and a zip builder.

use serde_json::{Value, json};
use std::io::{Cursor, Write};
use std::sync::{Arc, Mutex};

/// Forever's CurseForge game version type id.
pub const CF_FOREVER: u32 = 88568;

/// A request the fake server received.
#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    /// Path without the query string, e.g. `/v1/mods/search`.
    pub path: String,
    /// Raw query string (no `?`), still URL-encoded.
    pub query: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    /// Header value (case-insensitive name).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Decoded query parameter.
    pub fn query_param(&self, name: &str) -> Option<String> {
        self.query.split('&').find_map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(k) == name).then(|| decode(v))
        })
    }

    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("request body is JSON")
    }
}

#[derive(Clone)]
struct Route {
    method: String,
    /// Exact path, or a prefix when it ends with `*`.
    path: String,
    status: u16,
    content_type: String,
    body: Vec<u8>,
}

/// Local HTTP server on 127.0.0.1:<random>. Unrouted requests get 404. Stops when dropped.
pub struct FakeServer {
    url: String,
    server: Arc<tiny_http::Server>,
    routes: Arc<Mutex<Vec<Route>>>,
    requests: Arc<Mutex<Vec<Request>>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl FakeServer {
    pub fn start() -> Self {
        let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").expect("bind fake server"));
        let url = format!(
            "http://{}",
            server.server_addr().to_ip().expect("ip address")
        );
        let routes: Arc<Mutex<Vec<Route>>> = Arc::default();
        let requests: Arc<Mutex<Vec<Request>>> = Arc::default();
        let thread = {
            let (server, routes, requests) = (server.clone(), routes.clone(), requests.clone());
            std::thread::spawn(move || {
                for mut req in server.incoming_requests() {
                    let mut body = Vec::new();
                    let _ = req.as_reader().read_to_end(&mut body);
                    let (path, query) = req.url().split_once('?').unwrap_or((req.url(), ""));
                    let recorded = Request {
                        method: req.method().as_str().to_uppercase(),
                        path: path.to_string(),
                        query: query.to_string(),
                        headers: req
                            .headers()
                            .iter()
                            .map(|h| (h.field.as_str().to_string(), h.value.as_str().to_string()))
                            .collect(),
                        body,
                    };
                    // Last registered route wins, so tests can override defaults.
                    let route = routes
                        .lock()
                        .unwrap()
                        .iter()
                        .rev()
                        .find(|r| r.matches(&recorded))
                        .cloned();
                    requests.lock().unwrap().push(recorded);
                    let res = match route {
                        Some(r) => tiny_http::Response::from_data(r.body)
                            .with_status_code(r.status)
                            .with_header(
                                tiny_http::Header::from_bytes("Content-Type", r.content_type)
                                    .unwrap(),
                            ),
                        None => tiny_http::Response::from_data(b"no route".to_vec())
                            .with_status_code(404),
                    };
                    let _ = req.respond(res);
                }
            })
        };
        Self {
            url,
            server,
            routes,
            requests,
            thread: Some(thread),
        }
    }

    /// Base URL, e.g. `http://127.0.0.1:41234` (no trailing slash).
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Register a response. `path` is exact, or a prefix if it ends with `*`.
    pub fn respond(
        &self,
        method: &str,
        path: &str,
        status: u16,
        content_type: &str,
        body: impl Into<Vec<u8>>,
    ) {
        self.routes.lock().unwrap().push(Route {
            method: method.to_uppercase(),
            path: path.into(),
            status,
            content_type: content_type.into(),
            body: body.into(),
        });
    }

    pub fn json(&self, method: &str, path: &str, value: &Value) {
        self.respond(method, path, 200, "application/json", value.to_string());
    }

    pub fn bytes(&self, method: &str, path: &str, body: impl Into<Vec<u8>>) {
        self.respond(method, path, 200, "application/octet-stream", body);
    }

    /// All requests received so far.
    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }

    /// Requests to `path` (exact).
    pub fn requests_to(&self, path: &str) -> Vec<Request> {
        self.requests()
            .into_iter()
            .filter(|r| r.path == path)
            .collect()
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Route {
    fn matches(&self, req: &Request) -> bool {
        self.method == req.method
            && match self.path.strip_suffix('*') {
                Some(prefix) => req.path.starts_with(prefix),
                None => self.path == req.path,
            }
    }
}

fn decode(s: &str) -> String {
    let s = s.replace('+', " ");
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| u8::from_str_radix(std::str::from_utf8(h).ok()?, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A committed fixture from `crates/testkit/fixtures/<name>`, parsed as JSON.
pub fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Like [`fixture`], with every `http://FAKE` URL pointed at `base` (a [`FakeServer::url`]).
pub fn fixture_at(name: &str, base: &str) -> Value {
    serde_json::from_str(&fixture(name).to_string().replace("http://FAKE", base)).unwrap()
}

/// Wrap `data` in CurseForge's `{"data": ...}` envelope.
pub fn cf_envelope(data: Value) -> Value {
    json!({ "data": data })
}

/// A CurseForge file for Forever. `release_type`: 1 release, 2 beta, 3 alpha.
/// `folders` become `modules` with fingerprints `fingerprint_base + index`.
pub fn cf_file(
    file_id: u64,
    version: &str,
    release_type: u8,
    folders: &[&str],
    fingerprint_base: u32,
) -> Value {
    json!({
        "id": file_id,
        "gameId": 1,
        "isAvailable": true,
        "displayName": version,
        "fileName": format!("{}.zip", version.replace(' ', "-")),
        "releaseType": release_type,
        "fileStatus": 4,
        "fileDate": format!("2026-10-{:02}T12:00:00Z", 1 + file_id % 28),
        "fileLength": 1024,
        "downloadCount": 10,
        "downloadUrl": format!("https://edge.forgecdn.net/files/{}/{}/{}.zip", file_id / 1000, file_id % 1000, version),
        "gameVersions": ["1.60.1", "Forever"],
        "sortableGameVersions": [{
            "gameVersionName": "1.60.1",
            "gameVersionPadded": "0000000001.0000000060.0000000001",
            "gameVersion": "1.60.1",
            "gameVersionReleaseDate": "2026-09-01T00:00:00Z",
            "gameVersionTypeId": CF_FOREVER
        }],
        "dependencies": [],
        "exposeAsAlternative": null,
        "fileFingerprint": 1000 + file_id,
        "modules": folders
            .iter()
            .enumerate()
            .map(|(i, f)| json!({ "name": f, "fingerprint": fingerprint_base + i as u32 }))
            .collect::<Vec<_>>()
    })
}

/// A CurseForge mod with one stable Forever file. Its module fingerprints are `id * 100 + index`.
pub fn cf_mod(id: u64, name: &str, folders: &[&str], file_id: u64, version: &str) -> Value {
    cf_mod_with_files(
        id,
        name,
        vec![cf_file(file_id, version, 1, folders, id as u32 * 100)],
    )
}

/// A CurseForge mod with explicit `latestFiles`.
pub fn cf_mod_with_files(id: u64, name: &str, files: Vec<Value>) -> Value {
    let slug = name.to_lowercase().replace(' ', "-");
    json!({
        "id": id,
        "gameId": 1,
        "name": name,
        "slug": slug,
        "links": { "websiteUrl": format!("https://www.curseforge.com/wow/addons/{slug}") },
        "summary": format!("{name} summary"),
        "status": 4,
        "downloadCount": 1000 * id,
        "isFeatured": false,
        "primaryCategoryId": 1001,
        "classId": 1,
        "authors": [
            { "id": 1, "name": "Author", "url": "https://www.curseforge.com/members/author" },
            { "id": 2, "name": "_forgeuser12345", "url": "" }
        ],
        "logo": { "id": 1, "modId": id, "thumbnailUrl": format!("https://media.forgecdn.net/{id}/thumb.png"), "url": "" },
        "latestFiles": files,
        "latestFilesIndexes": [],
        "dateReleased": "2026-10-01T12:00:00Z",
        "allowModDistribution": true,
        "isAvailable": true
    })
}

/// A WowUp Hub addon (`repository` = `https://github.com/<repo>`) with one stable Forever release
/// (GitHub asset id `asset_id`) that installs `folders` from `download_url`.
pub fn hub_addon(
    id: u64,
    repo: &str,
    folders: &[&str],
    asset_id: u64,
    download_url: &str,
) -> Value {
    let (owner, name) = repo.split_once('/').expect("owner/repo");
    json!({
        "id": id,
        "repository": format!("https://github.com/{repo}"),
        "repository_name": name,
        "source": "GitHub",
        "description": format!("<p>{name} on the Hub</p>"),
        "homepage": "",
        "image_url": null,
        "owner_image_url": format!("https://avatars.example/{owner}.png"),
        "owner_name": owner,
        "total_download_count": 100 * id,
        "funding_links": [],
        "topics": [],
        "releases": [{
            "id": 900_000 + id,
            "external_id": asset_id.to_string(),
            "tag_name": format!("v{asset_id}"),
            "name": format!("{name}-v{asset_id}.zip"),
            "download_url": download_url,
            "download_count": 5,
            "prerelease": false,
            "published_at": format!("2026-11-{:02}T00:00:00.000Z", 1 + asset_id % 28),
            "game_type": "forever",
            "game_versions": [{ "game_type": "forever", "interface": "16001", "title": name, "version": "1", "authors": owner }],
            "addonFolders": folders.iter().map(|f| json!({ "folder_name": f, "fingerprint": "00" })).collect::<Vec<_>>()
        }]
    })
}

/// Build an in-memory zip from `(path, contents)` pairs (deflate).
pub fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default();
    for (path, contents) in entries {
        zip.start_file(*path, opts).expect("zip entry");
        zip.write_all(contents).expect("zip write");
    }
    zip.finish().expect("zip finish").into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn server_routes_and_records() {
        let s = FakeServer::start();
        s.json("GET", "/a", &json!({"ok": true}));
        s.bytes("GET", "/files/*", b"zip".to_vec());
        let addr = s.url().trim_start_matches("http://").to_string();
        let get = |path: &str| {
            let mut c = std::net::TcpStream::connect(&addr).unwrap();
            write!(
                c,
                "GET {path} HTTP/1.1\r\nHost: x\r\nX-Test: 1\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            let mut out = String::new();
            c.read_to_string(&mut out).unwrap();
            out
        };
        assert!(get("/a?q=hello%20world").contains("{\"ok\":true}"));
        assert!(get("/files/1/x.zip").ends_with("zip"));
        assert!(get("/nope").starts_with("HTTP/1.1 404"));
        let r = &s.requests()[0];
        assert_eq!((r.path.as_str(), r.header("x-test")), ("/a", Some("1")));
        assert_eq!(r.query_param("q").as_deref(), Some("hello world"));
    }

    #[test]
    fn zip_roundtrip() {
        let bytes = make_zip(&[("A/A.toc", b"## Title: A")]);
        let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut s = String::new();
        z.by_name("A/A.toc")
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        assert_eq!(s, "## Title: A");
    }

    #[test]
    fn fixtures_parse() {
        for f in [
            "cf_search.json",
            "cf_featured.json",
            "cf_fingerprints.json",
            "github_repo.json",
            "github_releases.json",
            "github_release.json",
            "wowi_filedetails.json",
            "hub_search.json",
            "hub_featured.json",
            "hub_batch.json",
            "hub_fingerprint.json",
        ] {
            fixture(f);
        }
    }
}
