//! Backup and restore Tauri command wrappers.
//!
//! All `_internal` implementations live in
//! [`kniha_jazd_core::commands_internal::backup`]. These wrappers translate
//! Tauri-specific types (`AppHandle`, `State`) into plain types and delegate.

use kniha_jazd_core::app_state::AppState;
use kniha_jazd_core::commands_internal::backup::{
    self as inner, BackupInfo, CleanupPreview, CleanupResult,
};
use kniha_jazd_core::db::Database;
use kniha_jazd_core::settings::BackupRetention;
use std::sync::Arc;
use tauri::State;

use super::get_app_data_dir;

#[tauri::command]
pub fn create_backup(
    app: tauri::AppHandle,
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
) -> Result<BackupInfo, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::create_backup_internal(&app_dir, &db, &app_state)
}

#[tauri::command]
pub fn create_backup_with_type(
    app: tauri::AppHandle,
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    backup_type: String,
    update_version: Option<String>,
) -> Result<BackupInfo, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::create_backup_with_type_internal(&app_dir, &db, &app_state, backup_type, update_version)
}

#[tauri::command]
pub fn get_cleanup_preview(
    app: tauri::AppHandle,
    keep_count: u32,
) -> Result<CleanupPreview, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::get_cleanup_preview_internal(&app_dir, keep_count)
}

#[tauri::command]
pub fn cleanup_pre_update_backups(
    app: tauri::AppHandle,
    app_state: State<Arc<AppState>>,
    keep_count: u32,
) -> Result<CleanupResult, String> {
    use kniha_jazd_core::check_read_only;
    check_read_only!(app_state);
    let app_dir = get_app_data_dir(&app)?;
    inner::cleanup_pre_update_backups_internal(&app_dir, keep_count)
}

#[tauri::command]
pub fn get_backup_retention(app: tauri::AppHandle) -> Result<Option<BackupRetention>, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::get_backup_retention_internal(&app_dir)
}

#[tauri::command]
pub fn set_backup_retention(
    app: tauri::AppHandle,
    app_state: State<Arc<AppState>>,
    retention: BackupRetention,
) -> Result<(), String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::set_backup_retention_internal(&app_dir, &app_state, retention)
}

#[tauri::command]
pub fn list_backups(app: tauri::AppHandle) -> Result<Vec<BackupInfo>, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::list_backups_internal(&app_dir)
}

#[tauri::command]
pub fn get_backup_info(app: tauri::AppHandle, filename: String) -> Result<BackupInfo, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::get_backup_info_internal(&app_dir, filename)
}

#[tauri::command]
pub fn restore_backup(
    app: tauri::AppHandle,
    app_state: State<Arc<AppState>>,
    filename: String,
) -> Result<(), String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::restore_backup_internal(&app_dir, &app_state, filename)
}

#[tauri::command]
pub fn delete_backup(
    app: tauri::AppHandle,
    app_state: State<Arc<AppState>>,
    filename: String,
) -> Result<(), String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::delete_backup_internal(&app_dir, &app_state, filename)
}

#[tauri::command]
pub fn get_backup_path(app: tauri::AppHandle, filename: String) -> Result<String, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::get_backup_path_internal(&app_dir, filename)
}
