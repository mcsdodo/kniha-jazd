//! Kniha Jázd core library — Tauri-free.
//!
//! Houses all business logic, persistence, HTTP server, and command
//! internals (`*_internal` functions). Both kniha-jazd-desktop and
//! kniha-jazd-web depend on this crate.

pub mod calculations;
pub mod constants;
pub mod db;
pub mod models;
pub mod schema;
