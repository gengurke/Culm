//! The app's log file: errors and crashes, kept for bug reports (F7.7).
//!
//! One file, appended to; once it passes [`MAX_BYTES`] (checked at start and after each entry) it becomes
//! `<name>.old.log`, replacing the older one, so at most two small files exist. The last [`RECENT`] errors and
//! crashes (from earlier runs too) stay in memory for "Copy debug info". Every entry is [`scrub`]bed of the
//! home folder and the user name.
//!
//! Writing is one append to an already open file, cheap enough for the UI thread.

use crate::util;
use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, TryLockError};

/// A log file past this size is moved aside.
pub const MAX_BYTES: u64 = 256 * 1024;
/// How many errors and crashes [`recent_errors`] keeps.
pub const RECENT: usize = 10;

struct Log {
    path: Option<PathBuf>,
    file: Option<File>,
    bytes: u64,
    home: Option<PathBuf>,
    recent: VecDeque<String>,
}

static LOG: Mutex<Log> = Mutex::new(Log {
    path: None,
    file: None,
    bytes: 0,
    home: None,
    recent: VecDeque::new(),
});

fn log() -> std::sync::MutexGuard<'static, Log> {
    LOG.lock().unwrap_or_else(|e| e.into_inner())
}

/// Starts logging to `path` and writes crashes there before the process dies. Until then (tests,
/// headless runs) entries only go to memory.
pub fn init(path: &Path, home: Option<&Path>) {
    let mut log = log();
    log.path = Some(path.to_path_buf());
    log.home = home.map(Path::to_path_buf);
    log.open();
    let earlier = [&old(path), path]
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect::<String>();
    log.recent = errors(&earlier);
    drop(log);

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("(no message)");
        let place = info
            .location()
            .map(|l| format!(" at {}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let thread = std::thread::current();
        let backtrace = std::backtrace::Backtrace::force_capture().to_string();
        let entry = format!(
            "{message}{place} (thread {})\n{backtrace}",
            thread.name().unwrap_or("unnamed")
        );
        // A panic inside the logger itself still holds the lock on this thread: skip the file then.
        let log = match LOG.try_lock() {
            Ok(log) => Some(log),
            Err(TryLockError::Poisoned(e)) => Some(e.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        };
        if let Some(mut log) = log {
            log.write("crash", &entry);
        }
        previous(info);
    }));
}

/// Logs an error the user saw (a red toast).
pub fn error(message: &str) {
    log().write("error", message);
}

/// Logs a plain event, e.g. the start of a run.
pub fn info(message: &str) {
    log().write("info", message);
}

/// The last errors and crashes, oldest first, each an entry as the file holds it.
pub fn recent_errors() -> Vec<String> {
    log().recent.iter().cloned().collect()
}

/// `text` with `secret` (an API key) hidden, `home` shown as `~` and the user name (home's last part) as
/// `<user>` wherever it is a whole path segment, e.g. `/run/media/<user>/Games` (any case on Windows).
pub fn scrub(text: &str, home: Option<&Path>, secret: Option<&str>) -> String {
    let mut text = text.to_string();
    if let Some(secret) = secret.filter(|s| !s.is_empty()) {
        text = text.replace(secret, "(hidden)");
    }
    let Some(home) = home else { return text };
    let shown = home.display().to_string();
    if !shown.is_empty() {
        text = text.replace(&shown, "~");
    }
    match home.file_name().and_then(|n| n.to_str()) {
        Some(user) => hide_segment(&text, user),
        None => text,
    }
}

/// `text` with `name` replaced by `<user>` where a path separator comes before it and no name character after.
fn hide_segment(text: &str, name: &str) -> String {
    // ASCII folding keeps byte offsets the same in `hay` and `text`.
    let fold = |s: &str| {
        if cfg!(windows) {
            s.to_ascii_lowercase()
        } else {
            s.to_string()
        }
    };
    let (hay, needle) = (fold(text), fold(name));
    let mut out = String::new();
    let mut last = 0;
    for (i, _) in hay.match_indices(&needle) {
        let end = i + needle.len();
        let after = hay[end..].chars().next();
        if hay[..i].ends_with(['/', '\\'])
            && !after.is_some_and(|c| c.is_alphanumeric() || "-_.".contains(c))
        {
            out.push_str(&text[last..i]);
            out.push_str("<user>");
            last = end;
        }
    }
    out.push_str(&text[last..]);
    out
}

fn old(path: &Path) -> PathBuf {
    path.with_extension("old.log")
}

impl Log {
    /// Opens the file for appending, moving it aside first if it is full.
    fn open(&mut self) {
        let Some(path) = &self.path else { return };
        self.file = None;
        if std::fs::metadata(path).is_ok_and(|m| m.len() > MAX_BYTES) {
            let _ = std::fs::rename(path, old(path));
        }
        self.file = path.parent().and_then(|dir| {
            std::fs::create_dir_all(dir).ok()?;
            File::options().create(true).append(true).open(path).ok()
        });
        self.bytes = std::fs::metadata(path).map_or(0, |m| m.len());
    }

    fn write(&mut self, level: &str, message: &str) {
        let message = scrub(message.trim_end(), self.home.as_deref(), None);
        // Lines after the first are indented, so an entry always starts at a timestamp.
        let entry = format!(
            "{} {level}: {}\n",
            util::now_rfc3339(),
            message.replace('\n', "\n  ")
        );
        if let Some(file) = &mut self.file
            && file.write_all(entry.as_bytes()).is_ok()
        {
            self.bytes += entry.len() as u64;
            if self.bytes > MAX_BYTES {
                self.open();
            }
        }
        if level != "info" {
            self.recent.push_back(entry.trim_end().to_string());
            self.recent
                .drain(..self.recent.len().saturating_sub(RECENT));
        }
    }
}

/// The last [`RECENT`] error and crash entries of a log file's text.
fn errors(text: &str) -> VecDeque<String> {
    let mut entries: Vec<String> = Vec::new();
    for line in text.lines() {
        match entries.last_mut() {
            Some(entry) if line.starts_with(' ') => {
                entry.push('\n');
                entry.push_str(line);
            }
            _ => entries.push(line.to_string()),
        }
    }
    entries.retain(|e| {
        e.split_once(' ')
            .is_some_and(|(_, rest)| rest.starts_with("error:") || rest.starts_with("crash:"))
    });
    let skip = entries.len().saturating_sub(RECENT);
    entries.into_iter().skip(skip).collect()
}
