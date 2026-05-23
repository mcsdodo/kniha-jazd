//! Tauri-flavored helpers for resolving the static frontend directory.
//!
//! Lives in the desktop crate because it depends on tauri::App / AppHandle.
//! The web binary passes a plain PathBuf and doesn't need these.
//!
//! Production layout: `tauri.conf.json` declares `bundle.resources` mapping the
//! staged SvelteKit build into `<resource_dir>/spa/`. A `beforeBundleCommand`
//! copies `<repo>/build/` into `<desktop_crate>/spa/` so the glob has something
//! to pick up. See [`prod_static_dir`] and [`SPA_SUBDIR`].

use std::path::{Path, PathBuf};

/// Subdirectory under Tauri's resource dir where the SPA assets land in prod.
/// Must match the destination in `tauri.conf.json` -> `bundle.resources`.
pub const SPA_SUBDIR: &str = "spa";

/// Compute the prod static dir from a resolved Tauri resource directory.
/// Extracted so it's testable without instantiating a `tauri::App`.
pub fn prod_static_dir(resource_dir: &Path) -> PathBuf {
    resource_dir.join(SPA_SUBDIR)
}

/// Resolve the static frontend directory for serving SPA files.
/// Debug builds serve from `../../build` (SvelteKit output, relative to desktop crate root),
/// production from `<resource_dir>/spa` (staged via `beforeBundleCommand` and
/// declared in `bundle.resources`).
pub fn resolve_static_dir(app: &tauri::App) -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../build")
    } else {
        use tauri::Manager;
        prod_static_dir(&app.path().resource_dir().unwrap_or_default())
    }
}

/// Resolve the static frontend directory from an AppHandle (for use after setup).
pub fn resolve_static_dir_from_handle(app: &tauri::AppHandle) -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../build")
    } else {
        use tauri::Manager;
        prod_static_dir(&app.path().resource_dir().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prod_static_dir_resolves_under_spa_subdir() {
        let resource_dir = PathBuf::from("/some/resource_dir");
        let result = prod_static_dir(&resource_dir);
        assert_eq!(result, PathBuf::from("/some/resource_dir/spa"));
    }

    /// Catches the most likely regression: bundling config gets edited and the
    /// SPA mapping is dropped, leaving the HTTP server with no `index.html` in
    /// the resource dir (prod 404 on `/`).
    #[test]
    fn tauri_conf_bundles_spa_resources_into_matching_subdir() {
        let conf_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let raw = std::fs::read_to_string(&conf_path).expect("read tauri.conf.json");
        let v: serde_json::Value =
            serde_json::from_str(&raw).expect("parse tauri.conf.json");

        let resources = v.pointer("/bundle/resources").unwrap_or_else(|| {
            panic!(
                "tauri.conf.json must declare bundle.resources so the HTTP server can \
                 find index.html in production (see src-tauri/desktop/src/static_dir.rs)"
            )
        });

        let arr = resources
            .as_array()
            .expect("bundle.resources must be an array of glob strings");

        let stages_spa = arr.iter().filter_map(|x| x.as_str()).any(|s| {
            // Either a glob under spa/ (preferred), or an explicit spa/index.html entry.
            s.starts_with(&format!("{}/", SPA_SUBDIR)) || s == SPA_SUBDIR
        });

        assert!(
            stages_spa,
            "bundle.resources must include `{SPA_SUBDIR}/**/*` (or similar) so the \
             SvelteKit build lands under <resource_dir>/{SPA_SUBDIR}. Current value: {arr:?}"
        );
    }
}
