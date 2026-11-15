//! UI-free core of the addon manager. Everything here is plain blocking Rust:
//! the app calls it from worker threads, tests call it directly.

pub mod fingerprint;
pub mod http;
pub mod install;
pub mod maintenance;
pub mod model;
pub mod providers;
pub mod scan;
pub mod store;
pub mod toc;
pub mod updates;
mod util;
pub mod wow;
pub mod wowup_import;

pub use model::*;
