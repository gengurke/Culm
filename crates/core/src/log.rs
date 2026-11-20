//! The app's log file: errors and crashes, kept for bug reports (F7.7).
//!
//! One file, appended to; at start a file over [`MAX_BYTES`] becomes `<name>.old.log` (replacing the older
//! one), so at most two small files exist. The last [`RECENT`] errors and crashes (from earlier runs too)
//! stay in memory for "Copy debug info". Every entry has the home folder replaced by `~`.
//!
//! Writing is one append to an already open file, cheap enough for the UI thread.

use crate::util;
use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A log file this big at start is moved aside.
pub const MAX_BYTES: u64 = 256 * 1024;
/// How many errors and crashes [`recent_errors`] keeps.
pub const RECENT: usize = 10;

struct Log {
    file: Option<File>,
    home: Option<PathBuf>,
    recent: VecDeque<String>,
}

static LOG: Mutex<Log> = Mutex::new(Log {
    file: None,
    home: None,
    recent: VecDeque::new(),
});

fn log() -> std::sync::MutexGuard<'static, Log> {
    LOG.lock().unwrap_or_else(|e| e.into_inner())
}

/// Starts logging to `path` (rotating it first) and writes crashes there before the process dies.
/// `home` is replaced by `~` in every entry. Without it (tests), entries only go to memory.
pub fn init(path: &Path, home: Option<&Path>) {
    let old = path.with_extension("old.log");
    if std::fs::metadata(path).is_ok_and(|m| m.len() > MAX_BYTES) {
        let _ = std::fs::rename(path, &old);
    }
    let earlier = [&old, path]
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect::<String>();
    let mut log = log();
    log.recent = errors(&earlier);
    log.home = home.map(Path::to_path_buf);
    log.file = path.parent().and_then(|dir| {
        std::fs::create_dir_all(dir).ok()?;
        File::options().create(true).append(true).open(path).ok()
    });
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
        // A panic inside the logger itself still holds the lock: skip the file then.
        if let Ok(mut log) = LOG.try_lock() {
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

/// `text` with `home` shown as `~` and `secret` (an API key) hidden.
pub fn scrub(text: &str, home: Option<&Path>, secret: Option<&str>) -> String {
    let mut text = text.to_string();
    if let Some(secret) = secret.filter(|s| !s.is_empty()) {
        text = text.replace(secret, "(hidden)");
    }
    if let Some(home) = home
        .map(|h| h.display().to_string())
        .filter(|h| !h.is_empty())
    {
        text = text.replace(&home, "~");
    }
    text
}

impl Log {
    fn write(&mut self, level: &str, message: &str) {
        let message = scrub(message.trim_end(), self.home.as_deref(), None);
        // Lines after the first are indented, so an entry always starts at a timestamp.
        let entry = format!(
            "{} {level}: {}",
            util::now_rfc3339(),
            message.replace('\n', "\n  ")
        );
        if let Some(file) = &mut self.file {
            let _ = writeln!(file, "{entry}");
        }
        if level != "info" {
            self.recent.push_back(entry);
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
