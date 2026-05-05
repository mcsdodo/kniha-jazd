//! Kniha Jázd core library — Tauri-free.
//!
//! Houses all business logic, persistence, HTTP server, and command
//! internals (`*_internal` functions). Both kniha-jazd-desktop and
//! kniha-jazd-web depend on this crate.

pub mod app_state;
pub mod calculations;
pub mod commands_internal;
pub mod constants;
pub mod db;
pub mod db_location;
pub mod export;
pub mod gemini;
pub mod invoice;
pub mod models;
pub mod paperless;
pub mod receipts;
pub mod schema;
pub mod server;
pub mod settings;
pub mod suggestions;

// Re-export the db tests module under the crate root so other test modules
// can import test helpers as `crate::db_tests::*`. Only available under #[cfg(test)].
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use crate::db::db_tests;
