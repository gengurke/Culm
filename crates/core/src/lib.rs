//! UI-free core of the addon manager. Everything here is plain blocking Rust:
//! the app calls it from worker threads, tests call it directly.

pub mod model;
pub mod providers;

pub use model::*;
