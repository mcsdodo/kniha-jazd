//! Framework-free command implementations.
//!
//! Each `*_internal` function takes plain types (`&Database`, `&AppState`,
//! plain args). The Tauri-flavored `#[tauri::command]` wrappers in
//! kniha-jazd-desktop's `commands/` module call these. The HTTP RPC
//! dispatcher in `kniha_jazd_core::server::dispatcher` also calls these
//! directly.

pub mod helpers;
pub use helpers::*;

// Per-file modules added incrementally in Tasks 16-22:
// pub mod backup;
// pub mod trips;
// pub mod vehicles;
// ...
