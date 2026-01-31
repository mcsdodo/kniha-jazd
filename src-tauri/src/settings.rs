//! Local settings override file support
//! Priority: local.settings.json > database settings

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Backup retention settings for automatic pre-update backup cleanup
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRetention {
    pub enabled: bool,
    pub keep_count: u32,
}

/// Date prefill mode for new trip entries
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DatePrefillMode {
    #[default]
    Previous, // Prefill with last trip date + 1 day
    Today, // Prefill with today's date
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalSettings {
    pub gemini_api_key: Option<String>,
    pub receipts_folder_path: Option<String>,
    pub theme: Option<String>,            // "system" | "light" | "dark"
    pub auto_check_updates: Option<bool>, // true by default if None
    pub custom_db_path: Option<String>,   // Custom database location (e.g., Google Drive, NAS)
    pub backup_retention: Option<BackupRetention>, // Backup retention settings for auto-cleanup
    pub date_prefill_mode: Option<DatePrefillMode>, // Date prefill for new trip entries
    pub hidden_columns: Option<Vec<String>>, // Hidden trip grid columns (e.g., ["time", "fuelConsumed"])
    // Home Assistant integration
    pub ha_url: Option<String>, // Home Assistant URL (e.g., "http://homeassistant.local:8123")
    pub ha_api_token: Option<String>, // Long-lived access token
}

impl LocalSettings {
    /// Load from local.settings.json in app data dir
    /// Returns default (empty) if file doesn't exist
    pub fn load(app_data_dir: &PathBuf) -> Self {
        let path = app_data_dir.join("local.settings.json");
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    /// Save settings to local.settings.json in app data dir
    pub fn save(&self, app_data_dir: &PathBuf) -> std::io::Result<()> {
        use std::io::Write;
        // Ensure the directory exists before writing
        fs::create_dir_all(app_data_dir)?;
        let path = app_data_dir.join("local.settings.json");
        let json = serde_json::to_string_pretty(self)?;
        // Use File::create + write + sync_all to ensure data is flushed to disk
        let mut file = fs::File::create(&path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
