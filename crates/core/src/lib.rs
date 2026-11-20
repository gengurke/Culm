//! UI-free core of the addon manager. Everything here is plain blocking Rust:
//! the app calls it from worker threads, tests call it directly.

pub mod class_icons;
pub mod discover;
pub mod fingerprint;
pub mod html;
pub mod http;
pub mod install;
pub mod log;
pub mod maintenance;
pub mod manual;
pub mod model;
pub mod providers;
pub mod scan;
pub mod self_update;
pub mod store;
pub mod toc;
pub mod updates;
mod util;
pub mod wow;
pub mod wowup_import;

pub use model::*;
