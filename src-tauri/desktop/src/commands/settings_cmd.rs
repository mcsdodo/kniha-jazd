//! Settings Tauri command wrappers.
//!
//! All `_internal` implementations live in
//! [`kniha_jazd_core::commands_internal::settings_cmd`]. The `move_database`
//! and `reset_database_location` commands stay here entirely because they
//! depend on `tauri::AppHandle` (no `_internal` versions).

pub use kniha_jazd_core::commands_internal::settings_cmd::*;

use kniha_jazd_core::app_state::AppState;
use kniha_jazd_core::commands_internal::settings_cmd as inner;
use kniha_jazd_core::db::Database;
use kniha_jazd_core::db_location::DbPaths;
use kniha_jazd_core::models::Settings;
use kniha_jazd_core::settings::{DatePrefillMode, LocalSettings};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Manager, State};

use super::get_app_data_dir;

// ============================================================================
// Database Settings
// ============================================================================

#[tauri::command]
pub fn get_settings(db: State<Arc<Database>>) -> Result<Option<Settings>, String> {
    inner::get_settings_internal(&db)
}

#[tauri::command]
pub fn save_settings(
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    company_name: String,
    company_ico: String,
    buffer_trip_purpose: String,
) -> Result<Settings, String> {
    inner::save_settings_internal(&db, &app_state, company_name, company_ico, buffer_trip_purpose)
}

// ============================================================================
// Window Size
// ============================================================================

/// Returns the optimal window size from tauri.conf.json (embedded at compile time).
///
/// This wrapper calls `parse_optimal_window_size` with the embedded config string;
/// the parser itself lives in core. The `include_str!` macro keeps the path
/// relative to *this* source file, so the read must happen in desktop.
#[tauri::command]
pub fn get_optimal_window_size() -> WindowSize {
    get_optimal_window_size_internal()
}

/// Desktop-side helper kept here so the dispatcher can still call it via
/// `crate::commands::get_optimal_window_size_internal()`.
pub fn get_optimal_window_size_internal() -> WindowSize {
    inner::parse_optimal_window_size(include_str!("../../tauri.conf.json"))
}

// ============================================================================
// Theme
// ============================================================================

#[tauri::command]
pub fn get_theme_preference(app: tauri::AppHandle) -> Result<String, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::get_theme_preference_internal(&app_dir)
}

#[tauri::command]
pub fn set_theme_preference(app: tauri::AppHandle, theme: String) -> Result<(), String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::set_theme_preference_internal(&app_dir, theme)
}

// ============================================================================
// Auto-Update Settings
// ============================================================================

#[tauri::command]
pub fn get_auto_check_updates(app: tauri::AppHandle) -> Result<bool, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::get_auto_check_updates_internal(&app_dir)
}

#[tauri::command]
pub fn set_auto_check_updates(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::set_auto_check_updates_internal(&app_dir, enabled)
}

// ============================================================================
// Date Prefill Mode
// ============================================================================

#[tauri::command]
pub fn get_date_prefill_mode(app: tauri::AppHandle) -> Result<DatePrefillMode, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::get_date_prefill_mode_internal(&app_dir)
}

#[tauri::command]
pub fn set_date_prefill_mode(app: tauri::AppHandle, mode: DatePrefillMode) -> Result<(), String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::set_date_prefill_mode_internal(&app_dir, mode)
}

// ============================================================================
// Hidden Columns
// ============================================================================

#[tauri::command]
pub fn get_hidden_columns(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::get_hidden_columns_internal(&app_dir)
}

#[tauri::command]
pub fn set_hidden_columns(app: tauri::AppHandle, columns: Vec<String>) -> Result<(), String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::set_hidden_columns_internal(&app_dir, columns)
}

// ============================================================================
// Database Location
// ============================================================================

#[tauri::command]
pub fn get_db_location(app_state: State<Arc<AppState>>) -> Result<DbLocationInfo, String> {
    inner::get_db_location_internal(&app_state)
}

#[tauri::command]
pub fn get_app_mode(app_state: State<Arc<AppState>>) -> Result<AppModeInfo, String> {
    inner::get_app_mode_internal(&app_state)
}

#[tauri::command]
pub fn check_target_has_db(target_path: String) -> Result<bool, String> {
    inner::check_target_has_db_internal(target_path)
}

/// Move the database to a new location (e.g., Google Drive, NAS).
///
/// This command:
/// 1. Copies the database file to the new location
/// 2. Copies the backups directory to the new location
/// 3. Updates local.settings.json with the new custom path
/// 4. Creates a lock file in the new location
/// 5. Releases the old lock and deletes old files
#[tauri::command]
pub fn move_database(
    app_handle: tauri::AppHandle,
    app_state: State<Arc<AppState>>,
    target_folder: String,
) -> Result<MoveDbResult, String> {
    use kniha_jazd_core::check_read_only;
    use kniha_jazd_core::db_location::{acquire_lock, release_lock};

    check_read_only!(app_state);

    let target_dir = PathBuf::from(&target_folder);

    // Validate and create target directory
    if !target_dir.exists() {
        std::fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Nepodarilo sa vytvoriť priečinok: {}", e))?;
    }

    // Get current database path from app state
    let current_path = app_state
        .get_db_path()
        .ok_or("Cesta k databáze nie je nastavená")?;
    let current_dir = current_path
        .parent()
        .ok_or("Neplatná cesta k databáze")?
        .to_path_buf();

    // Don't allow moving to the same location
    if current_dir == target_dir {
        return Err("Databáza sa už nachádza v tomto priečinku".to_string());
    }

    let source_paths = DbPaths::from_dir(&current_dir);
    let target_paths = DbPaths::from_dir(&target_dir);

    let mut files_moved = 0;

    // Copy database file
    if source_paths.db_file.exists() {
        std::fs::copy(&source_paths.db_file, &target_paths.db_file)
            .map_err(|e| format!("Nepodarilo sa skopírovať databázu: {}", e))?;
        files_moved += 1;
    } else {
        return Err("Zdrojová databáza neexistuje".to_string());
    }

    // Copy backups directory if it exists
    if source_paths.backups_dir.exists() {
        copy_dir_all(&source_paths.backups_dir, &target_paths.backups_dir)
            .map_err(|e| format!("Nepodarilo sa skopírovať zálohy: {}", e))?;
        files_moved += count_files(&target_paths.backups_dir);
    }

    // Update local.settings.json with new custom path
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.custom_db_path = Some(target_folder.clone());
    settings
        .save(&app_data_dir)
        .map_err(|e| format!("Nepodarilo sa uložiť nastavenia: {}", e))?;

    // Create lock file in new location
    let version = env!("CARGO_PKG_VERSION");
    acquire_lock(&target_paths.lock_file, version)
        .map_err(|e| format!("Nepodarilo sa vytvoriť zámok: {}", e))?;

    // Release old lock (ignore errors - file may not exist)
    let _ = release_lock(&source_paths.lock_file);

    // Delete old files (after successful copy)
    let _ = std::fs::remove_file(&source_paths.db_file);
    if source_paths.backups_dir.exists() {
        let _ = std::fs::remove_dir_all(&source_paths.backups_dir);
    }

    // Update app state with new path
    app_state.set_db_path(target_paths.db_file, true);

    Ok(MoveDbResult {
        success: true,
        new_path: target_folder,
        files_moved,
    })
}

/// Reset database location to default app data directory.
#[tauri::command]
pub fn reset_database_location(
    app_handle: tauri::AppHandle,
    app_state: State<Arc<AppState>>,
) -> Result<MoveDbResult, String> {
    use kniha_jazd_core::check_read_only;
    use kniha_jazd_core::db_location::{acquire_lock, release_lock};

    check_read_only!(app_state);

    // Get app data directory (default location)
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;

    // Get current database path
    let current_path = app_state
        .get_db_path()
        .ok_or("Cesta k databáze nie je nastavená")?;
    let current_dir = current_path
        .parent()
        .ok_or("Neplatná cesta k databáze")?
        .to_path_buf();

    // Don't reset if already in default location
    if current_dir == app_data_dir {
        return Err("Databáza sa už nachádza v predvolenom umiestnení".to_string());
    }

    let source_paths = DbPaths::from_dir(&current_dir);
    let target_paths = DbPaths::from_dir(&app_data_dir);

    let mut files_moved = 0;

    // Copy database file
    if source_paths.db_file.exists() {
        std::fs::copy(&source_paths.db_file, &target_paths.db_file)
            .map_err(|e| format!("Nepodarilo sa skopírovať databázu: {}", e))?;
        files_moved += 1;
    }

    // Copy backups directory
    if source_paths.backups_dir.exists() {
        copy_dir_all(&source_paths.backups_dir, &target_paths.backups_dir)
            .map_err(|e| format!("Nepodarilo sa skopírovať zálohy: {}", e))?;
        files_moved += count_files(&target_paths.backups_dir);
    }

    // Clear custom_db_path in settings
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.custom_db_path = None;
    settings
        .save(&app_data_dir)
        .map_err(|e| format!("Nepodarilo sa uložiť nastavenia: {}", e))?;

    // Create lock in new location
    let version = env!("CARGO_PKG_VERSION");
    acquire_lock(&target_paths.lock_file, version)
        .map_err(|e| format!("Nepodarilo sa vytvoriť zámok: {}", e))?;

    // Release old lock
    let _ = release_lock(&source_paths.lock_file);

    // Delete old files
    let _ = std::fs::remove_file(&source_paths.db_file);
    if source_paths.backups_dir.exists() {
        let _ = std::fs::remove_dir_all(&source_paths.backups_dir);
    }

    // Update app state
    app_state.set_db_path(target_paths.db_file, false);

    let new_path = app_data_dir.to_string_lossy().to_string();
    Ok(MoveDbResult {
        success: true,
        new_path,
        files_moved,
    })
}

/// Helper: Recursively copy a directory.
fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Helper: Count files in a directory.
fn count_files(dir: &PathBuf) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .count()
        })
        .unwrap_or(0)
}
