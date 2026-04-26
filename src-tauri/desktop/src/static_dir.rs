//! Tauri-flavored helpers for resolving the static frontend directory.
//!
//! Lives in the desktop crate because it depends on tauri::App / AppHandle.
//! The web binary passes a plain PathBuf and doesn't need these.

use std::path::PathBuf;

/// Resolve the static frontend directory for serving SPA files.
/// Debug builds serve from `../../build` (SvelteKit output, relative to desktop crate root),
/// production from Tauri's resource dir.
pub fn resolve_static_dir(app: &tauri::App) -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../build")
    } else {
        use tauri::Manager;
        app.path().resource_dir().unwrap_or_default().join("_up_")
    }
}

/// Resolve the static frontend directory from an AppHandle (for use after setup).
pub fn resolve_static_dir_from_handle(app: &tauri::AppHandle) -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../build")
    } else {
        use tauri::Manager;
        app.path().resource_dir().unwrap_or_default().join("_up_")
    }
}
