//! Backup and restore commands.

use crate::app_state::AppState;
use crate::check_read_only;
use crate::constants::{date_formats, paths};
use crate::db::Database;
use crate::models::BackupType;
use crate::settings::{BackupRetention, LocalSettings};
use chrono::Local;
use diesel::RunQueryDsl;
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::State;

use super::{get_app_data_dir, get_db_paths};

// ============================================================================
// Backup Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub filename: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub vehicle_count: i32,
    pub trip_count: i32,
    pub backup_type: String,            // "manual" | "pre-update"
    pub update_version: Option<String>, // e.g., "0.20.0" for pre-update backups
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPreview {
    pub to_delete: Vec<BackupInfo>,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub deleted: Vec<String>,
    pub freed_bytes: u64,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse backup filename to extract type and version
/// Manual: kniha-jazd-backup-2026-01-24-143022.db
/// Pre-update: kniha-jazd-backup-2026-01-24-143022-pre-v0.20.0.db
fn parse_backup_filename(filename: &str) -> (String, Option<String>) {
    if filename.starts_with(paths::BACKUP_PREFIX) {
        let without_prefix = filename.trim_start_matches(paths::BACKUP_PREFIX);
        if let Some(version_start) = without_prefix.find(paths::PRE_UPDATE_MARKER) {
            let version = without_prefix[version_start + paths::PRE_UPDATE_MARKER.len()..]
                .trim_end_matches(paths::BACKUP_EXTENSION);
            return (BackupType::PreUpdate.as_str().to_string(), Some(version.to_string()));
        }
    }
    (BackupType::Manual.as_str().to_string(), None)
}

/// Generate backup filename based on type and version
fn generate_backup_filename(backup_type: &str, update_version: Option<&str>) -> String {
    let timestamp = Local::now().format(date_formats::BACKUP_TIMESTAMP);
    match (backup_type, update_version) {
        (t, Some(version)) if t == BackupType::PreUpdate.as_str() => {
            format!(
                "{}{}{}{}{}",
                paths::BACKUP_PREFIX,
                timestamp,
                paths::PRE_UPDATE_MARKER,
                version,
                paths::BACKUP_EXTENSION
            )
        }
        _ => format!(
            "{}{}{}",
            paths::BACKUP_PREFIX,
            timestamp,
            paths::BACKUP_EXTENSION
        ),
    }
}

/// Get pre-update backups that should be deleted based on keep_count
/// Returns the oldest backups beyond the keep limit (manual backups are never included)
fn get_cleanup_candidates(backups: &[BackupInfo], keep_count: u32) -> Vec<BackupInfo> {
    // Filter to pre-update only
    let mut pre_update_backups: Vec<&BackupInfo> = backups
        .iter()
        .filter(|b| b.backup_type == BackupType::PreUpdate.as_str())
        .collect();

    // Sort by filename (which includes timestamp) - oldest first
    pre_update_backups.sort_by(|a, b| a.filename.cmp(&b.filename));

    let total = pre_update_backups.len();
    let keep = keep_count as usize;

    if total <= keep {
        return vec![];
    }

    // Return the oldest ones (to delete)
    pre_update_backups[0..(total - keep)]
        .iter()
        .map(|b| (*b).clone())
        .collect()
}

// ============================================================================
// Backup Commands
// ============================================================================

#[tauri::command]
pub fn create_backup(
    app: tauri::AppHandle,
    db: State<Database>,
    app_state: State<AppState>,
) -> Result<BackupInfo, String> {
    check_read_only!(app_state);
    let db_paths = get_db_paths(&app)?;

    // Create backup directory if it doesn't exist
    fs::create_dir_all(&db_paths.backups_dir).map_err(|e| e.to_string())?;

    // Generate backup filename with timestamp
    let timestamp = Local::now().format(date_formats::BACKUP_TIMESTAMP);
    let filename = format!(
        "{}{}{}",
        paths::BACKUP_PREFIX,
        timestamp,
        paths::BACKUP_EXTENSION
    );
    let backup_path = db_paths.backups_dir.join(&filename);

    // Copy current database to backup
    fs::copy(&db_paths.db_file, &backup_path).map_err(|e| e.to_string())?;

    // Get file size
    let metadata = fs::metadata(&backup_path).map_err(|e| e.to_string())?;

    // Get counts from current database
    let vehicles = db.get_all_vehicles().map_err(|e| e.to_string())?;
    let vehicle_count = vehicles.len() as i32;

    // Count trips across all vehicles
    let mut trip_count = 0;
    for vehicle in &vehicles {
        let trips = db
            .get_trips_for_vehicle(&vehicle.id.to_string())
            .map_err(|e| e.to_string())?;
        trip_count += trips.len() as i32;
    }

    Ok(BackupInfo {
        filename,
        created_at: Local::now().to_rfc3339(),
        size_bytes: metadata.len(),
        vehicle_count,
        trip_count,
        backup_type: BackupType::Manual.as_str().to_string(),
        update_version: None,
    })
}

/// Create backup with explicit type and optional version
/// Used for pre-update backups that need to record the target version
#[tauri::command]
pub fn create_backup_with_type(
    app: tauri::AppHandle,
    db: State<Database>,
    app_state: State<AppState>,
    backup_type: String,
    update_version: Option<String>,
) -> Result<BackupInfo, String> {
    check_read_only!(app_state);
    let db_paths = get_db_paths(&app)?;

    // Create backup directory if it doesn't exist
    fs::create_dir_all(&db_paths.backups_dir).map_err(|e| e.to_string())?;

    // Generate backup filename with type and version
    let filename = generate_backup_filename(&backup_type, update_version.as_deref());
    let backup_path = db_paths.backups_dir.join(&filename);

    // Copy current database to backup
    fs::copy(&db_paths.db_file, &backup_path).map_err(|e| e.to_string())?;

    // Get file size
    let metadata = fs::metadata(&backup_path).map_err(|e| e.to_string())?;

    // Get counts from current database
    let vehicles = db.get_all_vehicles().map_err(|e| e.to_string())?;
    let vehicle_count = vehicles.len() as i32;

    // Count trips across all vehicles
    let mut trip_count = 0;
    for vehicle in &vehicles {
        let trips = db
            .get_trips_for_vehicle(&vehicle.id.to_string())
            .map_err(|e| e.to_string())?;
        trip_count += trips.len() as i32;
    }

    // Parse the type back from filename to ensure consistency
    let (parsed_type, parsed_version) = parse_backup_filename(&filename);

    Ok(BackupInfo {
        filename,
        created_at: Local::now().to_rfc3339(),
        size_bytes: metadata.len(),
        vehicle_count,
        trip_count,
        backup_type: parsed_type,
        update_version: parsed_version,
    })
}

/// Get preview of pre-update backups that would be deleted
#[tauri::command]
pub fn get_cleanup_preview(
    app: tauri::AppHandle,
    keep_count: u32,
) -> Result<CleanupPreview, String> {
    let all_backups = list_backups(app)?;
    let to_delete = get_cleanup_candidates(&all_backups, keep_count);
    let total_bytes: u64 = to_delete.iter().map(|b| b.size_bytes).sum();

    Ok(CleanupPreview {
        to_delete,
        total_bytes,
    })
}

/// Delete old pre-update backups, keeping the N most recent
#[tauri::command]
pub fn cleanup_pre_update_backups(
    app: tauri::AppHandle,
    app_state: State<AppState>,
    keep_count: u32,
) -> Result<CleanupResult, String> {
    check_read_only!(app_state);
    cleanup_pre_update_backups_internal(&app, keep_count)
}

/// Internal cleanup function for use at startup (no State parameters needed)
pub fn cleanup_pre_update_backups_internal(
    app: &tauri::AppHandle,
    keep_count: u32,
) -> Result<CleanupResult, String> {
    let db_paths = get_db_paths(app)?;

    let all_backups = list_backups(app.clone())?;
    let to_delete = get_cleanup_candidates(&all_backups, keep_count);

    let mut deleted = Vec::new();
    let mut freed_bytes = 0u64;

    for backup in &to_delete {
        let path = db_paths.backups_dir.join(&backup.filename);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
            deleted.push(backup.filename.clone());
            freed_bytes += backup.size_bytes;
        }
    }

    Ok(CleanupResult {
        deleted,
        freed_bytes,
    })
}

/// Get backup retention settings
#[tauri::command]
pub fn get_backup_retention(app: tauri::AppHandle) -> Result<Option<BackupRetention>, String> {
    let app_dir = get_app_data_dir(&app)?;
    let settings = LocalSettings::load(&app_dir);
    Ok(settings.backup_retention)
}

/// Set backup retention settings
#[tauri::command]
pub fn set_backup_retention(
    app: tauri::AppHandle,
    app_state: State<AppState>,
    retention: BackupRetention,
) -> Result<(), String> {
    check_read_only!(app_state);
    let app_dir = get_app_data_dir(&app)?;
    let mut settings = LocalSettings::load(&app_dir);
    settings.backup_retention = Some(retention);
    settings.save(&app_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_backups(app: tauri::AppHandle) -> Result<Vec<BackupInfo>, String> {
    let db_paths = get_db_paths(&app)?;

    if !db_paths.backups_dir.exists() {
        return Ok(vec![]);
    }

    let mut backups = Vec::new();

    for entry in fs::read_dir(&db_paths.backups_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.extension().map(|e| e == "db").unwrap_or(false) {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;

            // Parse timestamp from filename: kniha-jazd-backup-YYYY-MM-DD-HHMMSS.db
            let created_at = if filename.starts_with(paths::BACKUP_PREFIX) {
                let date_part = filename
                    .trim_start_matches(paths::BACKUP_PREFIX)
                    .trim_end_matches(paths::BACKUP_EXTENSION);
                // Convert YYYY-MM-DD-HHMMSS to ISO format
                if date_part.len() >= 17 {
                    format!(
                        "{}-{}-{}T{}:{}:{}",
                        &date_part[0..4],
                        &date_part[5..7],
                        &date_part[8..10],
                        &date_part[11..13],
                        &date_part[13..15],
                        &date_part[15..17]
                    )
                } else {
                    Local::now().to_rfc3339()
                }
            } else {
                Local::now().to_rfc3339()
            };

            // Parse backup type and version from filename
            let (backup_type, update_version) = parse_backup_filename(&filename);

            // We can't easily get counts from backup without opening it
            // So we'll return 0 for now - the get_backup_info command will show actual counts
            backups.push(BackupInfo {
                filename,
                created_at,
                size_bytes: metadata.len(),
                vehicle_count: 0,
                trip_count: 0,
                backup_type,
                update_version,
            });
        }
    }

    // Sort by filename descending (newest first)
    backups.sort_by(|a, b| b.filename.cmp(&a.filename));

    Ok(backups)
}

#[tauri::command]
pub fn get_backup_info(app: tauri::AppHandle, filename: String) -> Result<BackupInfo, String> {
    let db_paths = get_db_paths(&app)?;
    let backup_path = db_paths.backups_dir.join(&filename);

    if !backup_path.exists() {
        return Err(format!("Backup not found: {}", filename));
    }

    let metadata = fs::metadata(&backup_path).map_err(|e| e.to_string())?;

    // Open backup database to get counts using Diesel
    let backup_db = crate::db::Database::from_path(&backup_path).map_err(|e| e.to_string())?;
    let conn = &mut *backup_db.connection();

    // Use raw SQL for simple count queries
    #[derive(diesel::QueryableByName)]
    struct CountRow {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        count: i32,
    }

    let vehicle_count: i32 = diesel::sql_query("SELECT COUNT(*) as count FROM vehicles")
        .get_result::<CountRow>(conn)
        .map(|r| r.count)
        .unwrap_or(0);

    let trip_count: i32 = diesel::sql_query("SELECT COUNT(*) as count FROM trips")
        .get_result::<CountRow>(conn)
        .map(|r| r.count)
        .unwrap_or(0);

    // Parse timestamp from filename
    let created_at = if filename.starts_with(paths::BACKUP_PREFIX) {
        let date_part = filename
            .trim_start_matches(paths::BACKUP_PREFIX)
            .trim_end_matches(paths::BACKUP_EXTENSION);
        // Handle pre-update suffix: -pre-vX.X.X
        let date_part = if let Some(pred_pos) = date_part.find(paths::PRE_UPDATE_MARKER) {
            &date_part[..pred_pos]
        } else {
            date_part
        };
        if date_part.len() >= 17 {
            format!(
                "{}-{}-{}T{}:{}:{}",
                &date_part[0..4],
                &date_part[5..7],
                &date_part[8..10],
                &date_part[11..13],
                &date_part[13..15],
                &date_part[15..17]
            )
        } else {
            Local::now().to_rfc3339()
        }
    } else {
        Local::now().to_rfc3339()
    };

    // Parse backup type and version from filename
    let (backup_type, update_version) = parse_backup_filename(&filename);

    Ok(BackupInfo {
        filename,
        created_at,
        size_bytes: metadata.len(),
        vehicle_count,
        trip_count,
        backup_type,
        update_version,
    })
}

#[tauri::command]
pub fn restore_backup(
    app: tauri::AppHandle,
    app_state: State<AppState>,
    filename: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    let db_paths = get_db_paths(&app)?;
    let backup_path = db_paths.backups_dir.join(&filename);

    if !backup_path.exists() {
        return Err(format!("Backup not found: {}", filename));
    }

    // Copy backup over current database
    fs::copy(&backup_path, &db_paths.db_file).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_backup(
    app: tauri::AppHandle,
    app_state: State<AppState>,
    filename: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    let db_paths = get_db_paths(&app)?;
    let backup_path = db_paths.backups_dir.join(&filename);

    if !backup_path.exists() {
        return Err(format!("Backup not found: {}", filename));
    }

    fs::remove_file(&backup_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_backup_path(app: tauri::AppHandle, filename: String) -> Result<String, String> {
    let db_paths = get_db_paths(&app)?;
    let backup_path = db_paths.backups_dir.join(&filename);

    if !backup_path.exists() {
        return Err(format!("Backup not found: {}", filename));
    }

    backup_path
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid path encoding".to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_backup_filename_manual() {
        let (backup_type, version) =
            parse_backup_filename("kniha-jazd-backup-2026-01-24-143022.db");
        assert_eq!(backup_type, "manual");
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_backup_filename_pre_update() {
        let (backup_type, version) =
            parse_backup_filename("kniha-jazd-backup-2026-01-24-143022-pre-v0.20.0.db");
        assert_eq!(backup_type, "pre-update");
        assert_eq!(version, Some("0.20.0".to_string()));
    }

    #[test]
    fn test_generate_backup_filename_manual() {
        let filename = generate_backup_filename("manual", None);
        assert!(filename.starts_with("kniha-jazd-backup-"));
        assert!(filename.ends_with(".db"));
        assert!(!filename.contains("-pre-v"));
    }

    #[test]
    fn test_generate_backup_filename_pre_update() {
        let filename = generate_backup_filename("pre-update", Some("0.20.0"));
        assert!(filename.starts_with("kniha-jazd-backup-"));
        assert!(filename.ends_with("-pre-v0.20.0.db"));
    }

    #[test]
    fn test_get_cleanup_candidates_empty() {
        let backups: Vec<BackupInfo> = vec![];
        let candidates = get_cleanup_candidates(&backups, 3);
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_get_cleanup_candidates_below_limit() {
        let backups = vec![
            BackupInfo {
                filename: "kniha-jazd-backup-2026-01-24-143022-pre-v0.20.0.db".to_string(),
                created_at: "2026-01-24T14:30:22".to_string(),
                size_bytes: 1000,
                vehicle_count: 1,
                trip_count: 10,
                backup_type: "pre-update".to_string(),
                update_version: Some("0.20.0".to_string()),
            },
            BackupInfo {
                filename: "kniha-jazd-backup-2026-01-25-143022-pre-v0.21.0.db".to_string(),
                created_at: "2026-01-25T14:30:22".to_string(),
                size_bytes: 1000,
                vehicle_count: 1,
                trip_count: 10,
                backup_type: "pre-update".to_string(),
                update_version: Some("0.21.0".to_string()),
            },
        ];
        let candidates = get_cleanup_candidates(&backups, 3);
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_get_cleanup_candidates_above_limit() {
        let backups = vec![
            BackupInfo {
                filename: "kniha-jazd-backup-2026-01-24-143022-pre-v0.20.0.db".to_string(),
                created_at: "2026-01-24T14:30:22".to_string(),
                size_bytes: 1000,
                vehicle_count: 1,
                trip_count: 10,
                backup_type: "pre-update".to_string(),
                update_version: Some("0.20.0".to_string()),
            },
            BackupInfo {
                filename: "kniha-jazd-backup-2026-01-25-143022-pre-v0.21.0.db".to_string(),
                created_at: "2026-01-25T14:30:22".to_string(),
                size_bytes: 1000,
                vehicle_count: 1,
                trip_count: 10,
                backup_type: "pre-update".to_string(),
                update_version: Some("0.21.0".to_string()),
            },
            BackupInfo {
                filename: "kniha-jazd-backup-2026-01-26-143022-pre-v0.22.0.db".to_string(),
                created_at: "2026-01-26T14:30:22".to_string(),
                size_bytes: 1000,
                vehicle_count: 1,
                trip_count: 10,
                backup_type: "pre-update".to_string(),
                update_version: Some("0.22.0".to_string()),
            },
        ];
        let candidates = get_cleanup_candidates(&backups, 2);
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].filename,
            "kniha-jazd-backup-2026-01-24-143022-pre-v0.20.0.db"
        );
    }

    #[test]
    fn test_get_cleanup_candidates_ignores_manual() {
        let backups = vec![
            BackupInfo {
                filename: "kniha-jazd-backup-2026-01-24-143022.db".to_string(),
                created_at: "2026-01-24T14:30:22".to_string(),
                size_bytes: 1000,
                vehicle_count: 1,
                trip_count: 10,
                backup_type: "manual".to_string(),
                update_version: None,
            },
            BackupInfo {
                filename: "kniha-jazd-backup-2026-01-25-143022-pre-v0.21.0.db".to_string(),
                created_at: "2026-01-25T14:30:22".to_string(),
                size_bytes: 1000,
                vehicle_count: 1,
                trip_count: 10,
                backup_type: "pre-update".to_string(),
                update_version: Some("0.21.0".to_string()),
            },
        ];
        // Keep 1 pre-update, but we only have 1 pre-update, so nothing to delete
        let candidates = get_cleanup_candidates(&backups, 1);
        assert!(candidates.is_empty());
    }
}
