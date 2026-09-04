//! Database location resolution.
//!
//! Resolves where the database, and its backups directory, live for a given
//! application data directory.

use crate::constants::paths;
use std::path::{Path, PathBuf};

/// Database file paths resolved based on custom settings.
#[derive(Debug, Clone)]
pub struct DbPaths {
    /// Path to the main database file (kniha-jazd.db)
    pub db_file: PathBuf,
    /// Path to the backups directory
    pub backups_dir: PathBuf,
}

impl DbPaths {
    /// Create DbPaths from a base directory.
    pub fn from_dir(base_dir: &Path) -> Self {
        Self {
            db_file: base_dir.join(paths::DB_FILENAME),
            backups_dir: base_dir.join(paths::BACKUPS_DIR),
        }
    }
}

/// Resolve database paths based on custom path setting.
///
/// # Arguments
/// * `app_data_dir` - The application data directory (fallback location)
/// * `custom_db_path` - Optional custom path for database storage
///
/// # Returns
/// A tuple of (DbPaths, is_custom) where is_custom is true if using custom path.
pub fn resolve_db_paths(app_data_dir: &Path, custom_db_path: Option<&str>) -> (DbPaths, bool) {
    match custom_db_path {
        Some(custom_path) if !custom_path.is_empty() => {
            let base = PathBuf::from(custom_path);
            (DbPaths::from_dir(&base), true)
        }
        _ => (DbPaths::from_dir(app_data_dir), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_default_paths() {
        let app_dir = PathBuf::from("/app/data");
        let (paths, is_custom) = resolve_db_paths(&app_dir, None);

        assert!(!is_custom);
        assert_eq!(paths.db_file, PathBuf::from("/app/data/kniha-jazd.db"));
        assert_eq!(paths.backups_dir, PathBuf::from("/app/data/backups"));
    }

    #[test]
    fn test_resolve_custom_paths() {
        let app_dir = PathBuf::from("/app/data");
        let (paths, is_custom) = resolve_db_paths(&app_dir, Some("D:/GoogleDrive/kniha-jazd"));

        assert!(is_custom);
        assert_eq!(
            paths.db_file,
            PathBuf::from("D:/GoogleDrive/kniha-jazd/kniha-jazd.db")
        );
        assert_eq!(
            paths.backups_dir,
            PathBuf::from("D:/GoogleDrive/kniha-jazd/backups")
        );
    }

    #[test]
    fn test_resolve_empty_custom_path_uses_default() {
        let app_dir = PathBuf::from("/app/data");
        let (paths, is_custom) = resolve_db_paths(&app_dir, Some(""));

        assert!(!is_custom);
        assert_eq!(paths.db_file, PathBuf::from("/app/data/kniha-jazd.db"));
    }

    #[test]
    fn test_resolve_none_custom_path_uses_default() {
        let app_dir = PathBuf::from("/app/data");
        let (paths, is_custom) = resolve_db_paths(&app_dir, None);

        assert!(!is_custom);
        assert_eq!(paths.db_file, PathBuf::from("/app/data/kniha-jazd.db"));
    }
}
