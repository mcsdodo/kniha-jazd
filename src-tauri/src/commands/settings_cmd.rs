//! Settings Commands
//!
//! Commands for managing application settings, preferences, and configuration.

use serde::Serialize;
use std::path::PathBuf;
use tauri::{Manager, State};
use uuid::Uuid;

use crate::check_read_only;
use crate::db::Database;
use crate::db_location::DbPaths;
use crate::models::{Settings, Theme};
use crate::settings::{DatePrefillMode, LocalSettings};
use chrono::Utc;

use super::AppState;

// ============================================================================
// Database Settings Commands
// ============================================================================

#[tauri::command]
pub fn get_settings(db: State<Database>) -> Result<Option<Settings>, String> {
    db.get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(
    db: State<Database>,
    app_state: State<AppState>,
    company_name: String,
    company_ico: String,
    buffer_trip_purpose: String,
) -> Result<Settings, String> {
    check_read_only!(app_state);
    let settings = Settings {
        id: Uuid::new_v4(),
        company_name,
        company_ico,
        buffer_trip_purpose,
        updated_at: Utc::now(),
    };

    db.save_settings(&settings).map_err(|e| e.to_string())?;
    Ok(settings)
}

// ============================================================================
// Window Commands
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

/// Returns the optimal window size from tauri.conf.json (embedded at compile time)
#[tauri::command]
pub fn get_optimal_window_size() -> WindowSize {
    // Parse tauri.conf.json at compile time
    let config_str = include_str!("../../tauri.conf.json");

    // Simple parsing - find width and height values
    let width = config_str
        .find("\"width\":")
        .and_then(|i| {
            let rest = &config_str[i + 8..];
            let end = rest.find(',')?;
            rest[..end].trim().parse().ok()
        })
        .unwrap_or(1980);

    let height = config_str
        .find("\"height\":")
        .and_then(|i| {
            let rest = &config_str[i + 9..];
            let end = rest.find(',')?;
            rest[..end].trim().parse().ok()
        })
        .unwrap_or(1080);

    WindowSize { width, height }
}

// ============================================================================
// Theme Commands
// ============================================================================

#[tauri::command]
pub fn get_theme_preference(app_handle: tauri::AppHandle) -> Result<String, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_data_dir);
    Ok(settings
        .theme
        .unwrap_or_else(|| Theme::System.as_str().to_string()))
}

#[tauri::command]
pub fn set_theme_preference(app_handle: tauri::AppHandle, theme: String) -> Result<(), String> {
    // Validate using Theme enum
    if Theme::from_str(&theme).is_none() {
        return Err(format!(
            "Invalid theme: {}. Must be {}, {}, or {}",
            theme,
            Theme::System.as_str(),
            Theme::Light.as_str(),
            Theme::Dark.as_str()
        ));
    }

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.theme = Some(theme);

    // Save to file
    let settings_path = app_data_dir.join("local.settings.json");
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, json).map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Auto-Update Settings Commands
// ============================================================================

#[tauri::command]
pub fn get_auto_check_updates(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_data_dir);
    // Default to true if not set
    Ok(settings.auto_check_updates.unwrap_or(true))
}

#[tauri::command]
pub fn set_auto_check_updates(app_handle: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.auto_check_updates = Some(enabled);

    // Save to file
    let settings_path = app_data_dir.join("local.settings.json");
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, json).map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Date Prefill Mode Commands
// ============================================================================

#[tauri::command]
pub fn get_date_prefill_mode(app_handle: tauri::AppHandle) -> Result<DatePrefillMode, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_data_dir);
    // Default to Previous if not set
    Ok(settings.date_prefill_mode.unwrap_or_default())
}

#[tauri::command]
pub fn set_date_prefill_mode(
    app_handle: tauri::AppHandle,
    mode: DatePrefillMode,
) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.date_prefill_mode = Some(mode);

    // Save to file
    let settings_path = app_data_dir.join("local.settings.json");
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, json).map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Hidden Columns Commands
// ============================================================================

#[tauri::command]
pub fn get_hidden_columns(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_data_dir);
    // Default to empty array (all columns visible) if not set
    Ok(settings.hidden_columns.unwrap_or_default())
}

#[tauri::command]
pub fn set_hidden_columns(
    app_handle: tauri::AppHandle,
    columns: Vec<String>,
) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.hidden_columns = Some(columns);
    settings.save(&app_data_dir).map_err(|e| e.to_string())
}

// ============================================================================
// Database Location Commands
// ============================================================================

/// Information about the current database location.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbLocationInfo {
    /// Full path to the database file
    pub db_path: String,
    /// Whether using a custom (non-default) database path
    pub is_custom_path: bool,
    /// Path to the backups directory
    pub backups_path: String,
}

/// Information about the current application mode.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppModeInfo {
    /// Current mode: "Normal" or "ReadOnly"
    pub mode: String,
    /// Whether the app is in read-only mode
    pub is_read_only: bool,
    /// Reason for read-only mode (if applicable)
    pub read_only_reason: Option<String>,
}

#[tauri::command]
pub fn get_db_location(app_state: State<AppState>) -> Result<DbLocationInfo, String> {
    let db_path = app_state
        .get_db_path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let is_custom = app_state.is_custom_path();

    // Derive backups path from db path
    let backups_path = app_state
        .get_db_path()
        .map(|p| {
            p.parent()
                .map(|parent| parent.join("backups").to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    Ok(DbLocationInfo {
        db_path,
        is_custom_path: is_custom,
        backups_path,
    })
}

#[tauri::command]
pub fn get_app_mode(app_state: State<AppState>) -> Result<AppModeInfo, String> {
    let mode = app_state.get_mode();
    let is_read_only = app_state.is_read_only();
    let read_only_reason = app_state.get_read_only_reason();

    Ok(AppModeInfo {
        mode: format!("{:?}", mode),
        is_read_only,
        read_only_reason,
    })
}

/// Check if a target directory already contains a database.
#[tauri::command]
pub fn check_target_has_db(target_path: String) -> Result<bool, String> {
    let db_file = PathBuf::from(&target_path).join("kniha-jazd.db");
    Ok(db_file.exists())
}

/// Result of a database move operation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveDbResult {
    pub success: bool,
    pub new_path: String,
    pub files_moved: usize,
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
    app_state: State<AppState>,
    target_folder: String,
) -> Result<MoveDbResult, String> {
    use crate::db_location::{acquire_lock, release_lock};

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
    app_state: State<AppState>,
) -> Result<MoveDbResult, String> {
    use crate::db_location::{acquire_lock, release_lock};

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
