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


/// Check that the database the server will actually open is the same file the
/// backup/restore commands operate on.
///
/// There are two independent answers to "where is the database": the web binary
/// reads `DATABASE_PATH`, while [`crate::commands_internal::get_db_paths_for_dir`]
/// resolves it from the data dir plus any `custom_db_path` in
/// `local.settings.json`. When those disagree, `restore_backup` copies over a file
/// nothing has open and reports success — the operator is told their data was
/// restored while the running instance still serves the old database. That is the
/// worst failure shape available for a legal-compliance record, so refuse to start
/// rather than serve a deployment where restore silently does nothing.
pub fn verify_db_path_consistency(
    app_data_dir: &Path,
    configured_db_path: &Path,
    custom_db_path: Option<&str>,
) -> Result<(), String> {
    let (resolved, _is_custom) = resolve_db_paths(app_data_dir, custom_db_path);
    if resolved.db_file == configured_db_path {
        return Ok(());
    }
    Err(format!(
        "Database path conflict: the server would open {} but backup/restore would operate on {}. A restore would report success while changing nothing. Point DATABASE_PATH at {} (or clear custom_db_path in local.settings.json).",
        configured_db_path.display(),
        resolved.db_file.display(),
        resolved.db_file.display(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consistent_db_path_is_accepted() {
        let app_dir = PathBuf::from("/data");
        assert!(verify_db_path_consistency(
            &app_dir,
            &PathBuf::from("/data").join("kniha-jazd.db"),
            None
        )
        .is_ok());
    }

    /// The shape that makes a restore lie: DATABASE_PATH names a file the
    /// backup commands never touch, so `restore_backup` returns Ok having
    /// written somewhere the server does not read.
    #[test]
    fn diverging_database_path_is_rejected() {
        let app_dir = PathBuf::from("/data");
        let err = verify_db_path_consistency(&app_dir, &PathBuf::from("/data/logbook.db"), None)
            .expect_err("a DATABASE_PATH the backup commands ignore must not be accepted");
        assert!(err.contains("logbook.db"), "error should name the configured path: {err}");
        assert!(err.contains("kniha-jazd.db"), "error should name the resolved path: {err}");
    }

    /// A `custom_db_path` left behind by a desktop install steers backup/restore
    /// away from the file the container opens, with no DATABASE_PATH involved.
    #[test]
    fn leftover_custom_db_path_is_rejected() {
        let app_dir = PathBuf::from("/data");
        let err = verify_db_path_consistency(
            &app_dir,
            &PathBuf::from("/data").join("kniha-jazd.db"),
            Some("/mnt/gdrive/kniha-jazd"),
        )
        .expect_err("a stale custom_db_path must not be accepted silently");
        assert!(err.contains("gdrive"), "error should name the custom location: {err}");
    }

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
