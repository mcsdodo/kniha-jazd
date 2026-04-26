//! Framework-free command implementations.
//!
//! Each `*_internal` function takes plain types (`&Database`, `&AppState`,
//! plain args). The Tauri-flavored `#[tauri::command]` wrappers in
//! kniha-jazd-desktop's `commands/` module call these. The HTTP RPC
//! dispatcher in `kniha_jazd_core::server::dispatcher` also calls these
//! directly.

pub mod helpers;
pub use helpers::*;

pub mod backup;
pub use backup::*;
