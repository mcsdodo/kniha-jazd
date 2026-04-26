//! Tauri commands to expose Rust functionality to the frontend
//!
//! This module is being refactored into feature-based submodules (ADR-011).

pub mod backup;
pub mod export_cmd;
pub mod integrations;
pub mod receipts_cmd;
pub mod server_cmd;
pub mod settings_cmd;
pub mod statistics;
pub mod trips;
pub mod vehicles;

// Re-export all commands from submodules
pub use backup::*;
pub use export_cmd::*;
pub use integrations::*;
pub use receipts_cmd::*;
pub use server_cmd::*;
pub use settings_cmd::*;
pub use statistics::*;
pub use trips::*;
pub use vehicles::*;

// Re-export pure helpers from core. The macro check_read_only! is auto-exported
// via #[macro_export] on its definition in core; #[macro_use] makes it available
// at desktop crate root, so existing call sites keep working.
pub use kniha_jazd_core::commands_internal::{
    calculate_odometer_start, calculate_trip_numbers, generate_month_end_rows,
    get_db_paths_for_dir, parse_iso_datetime,
};

use kniha_jazd_core::app_state::AppState;
use kniha_jazd_core::constants::env_vars;
#[cfg(test)]
use kniha_jazd_core::db::Database;
use kniha_jazd_core::db_location::DbPaths;
use std::path::PathBuf;
use tauri::Manager;

/// Get the app data directory, respecting the KNIHA_JAZD_DATA_DIR environment variable.
/// This ensures consistency between database operations and other file operations (backups, settings).
pub(crate) fn get_app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    match std::env::var(env_vars::DATA_DIR) {
        Ok(path) => Ok(PathBuf::from(path)),
        Err(_) => app.path().app_data_dir().map_err(|e| e.to_string()),
    }
}

/// Get resolved database paths from a Tauri AppHandle.
/// Convenience wrapper that resolves the app data dir first.
pub(crate) fn get_db_paths(app: &tauri::AppHandle) -> Result<DbPaths, String> {
    let app_dir = get_app_data_dir(app)?;
    get_db_paths_for_dir(&app_dir)
}

