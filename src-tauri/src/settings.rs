//! Local settings override file support
//! Priority: local.settings.json > database settings

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalSettings {
    pub gemini_api_key: Option<String>,
    pub receipts_folder_path: Option<String>,
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
    }

    #[test]
    fn test_load_existing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.settings.json");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(b"{\"gemini_api_key\": \"test-key\", \"receipts_folder_path\": \"C:\\\\test\"}")
            .unwrap();

        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(settings.gemini_api_key, Some("test-key".to_string()));
        assert_eq!(settings.receipts_folder_path, Some("C:\\test".to_string()));
    }

    #[test]
    fn test_load_partial_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.settings.json");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(b"{\"gemini_api_key\": \"only-key\"}").unwrap();

        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(settings.gemini_api_key, Some("only-key".to_string()));
        assert!(settings.receipts_folder_path.is_none());
    }
}
