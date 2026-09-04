//! Settings command implementations (framework-free).
//!
//! Internal logic for managing application settings, preferences, and
//! configuration.

use serde::Serialize;
use std::path::Path;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::check_read_only;
use crate::db::Database;
use crate::models::{Settings, Theme};
use crate::settings::{DatePrefillMode, LocalSettings};
use chrono::Utc;

// ============================================================================
// Database Settings
// ============================================================================

pub fn get_settings_internal(db: &Database) -> Result<Option<Settings>, String> {
    db.get_settings().map_err(|e| e.to_string())
}

pub fn save_settings_internal(
    db: &Database,
    app_state: &AppState,
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
// Theme
// ============================================================================

pub fn get_theme_preference_internal(app_dir: &Path) -> Result<String, String> {
    let settings = LocalSettings::load(app_dir);
    Ok(settings
        .theme
        .unwrap_or_else(|| Theme::System.as_str().to_string()))
}

pub fn set_theme_preference_internal(app_dir: &Path, theme: String) -> Result<(), String> {
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

    let mut settings = LocalSettings::load(app_dir);
    settings.theme = Some(theme);

    // Save to file
    let settings_path = app_dir.join("local.settings.json");
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, json).map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Date Prefill Mode
// ============================================================================

pub fn get_date_prefill_mode_internal(app_dir: &Path) -> Result<DatePrefillMode, String> {
    let settings = LocalSettings::load(app_dir);
    // Default to Previous if not set
    Ok(settings.date_prefill_mode.unwrap_or_default())
}

pub fn set_date_prefill_mode_internal(app_dir: &Path, mode: DatePrefillMode) -> Result<(), String> {
    let mut settings = LocalSettings::load(app_dir);
    settings.date_prefill_mode = Some(mode);

    // Save to file
    let settings_path = app_dir.join("local.settings.json");
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, json).map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Hidden Columns
// ============================================================================

pub fn get_hidden_columns_internal(app_dir: &Path) -> Result<Vec<String>, String> {
    let settings = LocalSettings::load(app_dir);
    // Default to empty array (all columns visible) if not set
    Ok(settings.hidden_columns.unwrap_or_default())
}

pub fn set_hidden_columns_internal(app_dir: &Path, columns: Vec<String>) -> Result<(), String> {
    let mut settings = LocalSettings::load(app_dir);
    settings.hidden_columns = Some(columns);
    settings.save(app_dir).map_err(|e| e.to_string())
}

// ============================================================================
// Time Inference Toggle
// ============================================================================

pub fn get_infer_trip_times_internal(app_dir: &Path) -> Result<bool, String> {
    let settings = LocalSettings::load(app_dir);
    // Default OFF: None and Some(false) both mean disabled.
    Ok(settings.infer_trip_times.unwrap_or(false))
}

pub fn set_infer_trip_times_internal(app_dir: &Path, enabled: bool) -> Result<(), String> {
    let mut settings = LocalSettings::load(app_dir);
    settings.infer_trip_times = Some(enabled);
    settings.save(app_dir).map_err(|e| e.to_string())
}

// ============================================================================
// Database Location
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

pub fn get_db_location_internal(app_state: &AppState) -> Result<DbLocationInfo, String> {
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

pub fn get_app_mode_internal(app_state: &AppState) -> Result<AppModeInfo, String> {
    let mode = app_state.get_mode();
    let is_read_only = app_state.is_read_only();
    let read_only_reason = app_state.get_read_only_reason();

    Ok(AppModeInfo {
        mode: format!("{:?}", mode),
        is_read_only,
        read_only_reason,
    })
}

