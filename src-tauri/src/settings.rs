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
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_missing_file_returns_default() {
        let dir = tempdir().unwrap();
        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert!(settings.gemini_api_key.is_none());
        assert!(settings.receipts_folder_path.is_none());
        assert!(settings.theme.is_none());
        assert!(settings.custom_db_path.is_none());
    }

    #[test]
    fn test_load_existing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.settings.json");
        let mut file = fs::File::create(&path).unwrap();
        // Use forward slashes which work on all platforms and don't need escaping
        file.write_all(br#"{"gemini_api_key": "test-key", "receipts_folder_path": "C:/test"}"#)
            .unwrap();

        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(settings.gemini_api_key, Some("test-key".to_string()));
        assert_eq!(settings.receipts_folder_path, Some("C:/test".to_string()));
    }

    #[test]
    fn test_load_partial_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.settings.json");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(br#"{"gemini_api_key": "only-key"}"#)
            .unwrap();

        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(settings.gemini_api_key, Some("only-key".to_string()));
        assert!(settings.receipts_folder_path.is_none());
    }

    #[test]
    fn test_load_with_theme() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.settings.json");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(br#"{"theme": "dark"}"#).unwrap();

        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(settings.theme, Some("dark".to_string()));
    }

    #[test]
    fn test_load_with_custom_db_path() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.settings.json");
        let mut file = fs::File::create(&path).unwrap();
        // Use forward slashes for cross-platform compatibility
        file.write_all(br#"{"custom_db_path": "D:/GoogleDrive/kniha-jazd"}"#)
            .unwrap();

        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(
            settings.custom_db_path,
            Some("D:/GoogleDrive/kniha-jazd".to_string())
        );
    }

    #[test]
    fn test_save_creates_file() {
        let dir = tempdir().unwrap();
        let settings = LocalSettings {
            custom_db_path: Some("D:/GoogleDrive/kniha-jazd".to_string()),
            ..Default::default()
        };

        settings.save(&dir.path().to_path_buf()).unwrap();

        let path = dir.path().join("local.settings.json");
        assert!(path.exists());

        let loaded = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(
            loaded.custom_db_path,
            Some("D:/GoogleDrive/kniha-jazd".to_string())
        );
    }

    #[test]
    fn test_save_preserves_all_fields() {
        let dir = tempdir().unwrap();
        let settings = LocalSettings {
            gemini_api_key: Some("my-key".to_string()),
            receipts_folder_path: Some("C:/receipts".to_string()),
            theme: Some("dark".to_string()),
            auto_check_updates: Some(false),
            custom_db_path: Some("D:/NAS/data".to_string()),
            backup_retention: None,
            date_prefill_mode: Some(DatePrefillMode::Today),
            hidden_columns: Some(vec!["time".to_string(), "fuelConsumed".to_string()]),
            ha_url: Some("http://ha.local:8123".to_string()),
            ha_api_token: Some("token123".to_string()),
        };

        settings.save(&dir.path().to_path_buf()).unwrap();

        let loaded = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(loaded.gemini_api_key, Some("my-key".to_string()));
        assert_eq!(loaded.receipts_folder_path, Some("C:/receipts".to_string()));
        assert_eq!(loaded.theme, Some("dark".to_string()));
        assert_eq!(loaded.auto_check_updates, Some(false));
        assert_eq!(loaded.custom_db_path, Some("D:/NAS/data".to_string()));
        assert_eq!(loaded.date_prefill_mode, Some(DatePrefillMode::Today));
        assert_eq!(
            loaded.hidden_columns,
            Some(vec!["time".to_string(), "fuelConsumed".to_string()])
        );
        assert_eq!(loaded.ha_url, Some("http://ha.local:8123".to_string()));
        assert_eq!(loaded.ha_api_token, Some("token123".to_string()));
    }

    #[test]
    fn test_load_with_backup_retention() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.settings.json");
        let mut file = fs::File::create(&path).unwrap();
        // BackupRetention uses camelCase for JSON fields (keepCount not keep_count)
        file.write_all(br#"{"backup_retention": {"enabled": true, "keepCount": 5}}"#)
            .unwrap();

        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert!(settings.backup_retention.is_some());
        let retention = settings.backup_retention.unwrap();
        assert!(retention.enabled);
        assert_eq!(retention.keep_count, 5);
    }

    #[test]
    fn test_backup_retention_defaults() {
        let dir = tempdir().unwrap();
        let settings = LocalSettings::load(&dir.path().to_path_buf());
        // When missing, should be None (not enabled by default)
        assert!(settings.backup_retention.is_none());
    }

    // DatePrefillMode tests
    #[test]
    fn test_date_prefill_mode_default() {
        // When missing from JSON, should default to Previous
        let dir = tempdir().unwrap();
        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert!(settings.date_prefill_mode.is_none());
        // When None, the default should be Previous
        assert_eq!(
            settings.date_prefill_mode.unwrap_or_default(),
            DatePrefillMode::Previous
        );
    }

    #[test]
    fn test_date_prefill_mode_serialization() {
        // "today" in JSON should deserialize to Today variant
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.settings.json");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(br#"{"date_prefill_mode": "today"}"#)
            .unwrap();

        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(settings.date_prefill_mode, Some(DatePrefillMode::Today));
    }

    #[test]
    fn test_date_prefill_mode_round_trip() {
        // Save Today, load, verify it's still Today
        let dir = tempdir().unwrap();
        let settings = LocalSettings {
            date_prefill_mode: Some(DatePrefillMode::Today),
            ..Default::default()
        };

        settings.save(&dir.path().to_path_buf()).unwrap();

        let loaded = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(loaded.date_prefill_mode, Some(DatePrefillMode::Today));
    }

    // Hidden columns tests
    #[test]
    fn test_hidden_columns_default() {
        let dir = tempdir().unwrap();
        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert!(settings.hidden_columns.is_none());
    }

    #[test]
    fn test_hidden_columns_serialization() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.settings.json");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(br#"{"hidden_columns": ["time", "fuelConsumed", "otherCostsNote"]}"#)
            .unwrap();

        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(
            settings.hidden_columns,
            Some(vec![
                "time".to_string(),
                "fuelConsumed".to_string(),
                "otherCostsNote".to_string()
            ])
        );
    }

    #[test]
    fn test_hidden_columns_empty_array() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.settings.json");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(br#"{"hidden_columns": []}"#).unwrap();

        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(settings.hidden_columns, Some(vec![]));
    }

    #[test]
    fn test_hidden_columns_round_trip() {
        let dir = tempdir().unwrap();
        let settings = LocalSettings {
            hidden_columns: Some(vec!["time".to_string(), "fuelRemaining".to_string()]),
            ..Default::default()
        };

        settings.save(&dir.path().to_path_buf()).unwrap();

        let loaded = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(
            loaded.hidden_columns,
            Some(vec!["time".to_string(), "fuelRemaining".to_string()])
        );
    }

    // Home Assistant tests
    #[test]
    fn test_ha_settings_default() {
        let dir = tempdir().unwrap();
        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert!(settings.ha_url.is_none());
        assert!(settings.ha_api_token.is_none());
    }

    #[test]
    fn test_ha_settings_serialization() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.settings.json");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(
            br#"{"ha_url": "http://homeassistant.local:8123", "ha_api_token": "secret-token"}"#,
        )
        .unwrap();

        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(
            settings.ha_url,
            Some("http://homeassistant.local:8123".to_string())
        );
        assert_eq!(settings.ha_api_token, Some("secret-token".to_string()));
    }

    #[test]
    fn test_ha_settings_round_trip() {
        let dir = tempdir().unwrap();
        let settings = LocalSettings {
            ha_url: Some("https://my-ha.duckdns.org".to_string()),
            ha_api_token: Some("my-long-lived-token".to_string()),
            ..Default::default()
        };

        settings.save(&dir.path().to_path_buf()).unwrap();

        let loaded = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(loaded.ha_url, Some("https://my-ha.duckdns.org".to_string()));
        assert_eq!(loaded.ha_api_token, Some("my-long-lived-token".to_string()));
    }
}
