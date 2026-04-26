//! Kniha Jázd core library — Tauri-free.
//!
//! Houses all business logic, persistence, HTTP server, and command
//! internals (`*_internal` functions). Both kniha-jazd-desktop and
//! kniha-jazd-web depend on this crate.

pub mod app_state;
pub mod calculations;
pub mod constants;
pub mod db;
pub mod db_location;
pub mod export;
pub mod models;
pub mod schema;
pub mod settings;
pub mod suggestions;
