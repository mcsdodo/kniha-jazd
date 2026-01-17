//! Database location resolution and lock file management.
//!
//! Supports custom database paths for multi-PC setups (Google Drive, NAS, etc.)
//! with lock files to prevent concurrent access conflicts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// Database file paths resolved based on custom settings.
#[derive(Debug, Clone)]
pub struct DbPaths {
    /// Path to the main database file (kniha-jazd.db)
    pub db_file: PathBuf,
    /// Path to the lock file (kniha-jazd.lock)
    pub lock_file: PathBuf,
    /// Path to the backups directory
    pub backups_dir: PathBuf,
}

impl DbPaths {
    /// Create DbPaths from a base directory.
    pub fn from_dir(base_dir: &PathBuf) -> Self {
        Self {
            db_file: base_dir.join("kniha-jazd.db"),
            lock_file: base_dir.join("kniha-jazd.lock"),
            backups_dir: base_dir.join("backups"),
        }
    }
}

/// Lock file content for concurrent access prevention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockFile {
    /// Name of the PC holding the lock
    pub pc_name: String,
    /// When the lock was acquired
    pub opened_at: DateTime<Utc>,
    /// Last heartbeat timestamp (updated periodically to detect stale locks)
    pub last_heartbeat: DateTime<Utc>,
    /// Version of the app holding the lock
    pub app_version: String,
    /// Process ID of the app holding the lock
    pub pid: u32,
}

/// Status of the lock file.
#[derive(Debug, Clone)]
pub enum LockStatus {
    /// No lock file exists - free to acquire
    Free,
    /// Lock exists but is stale (heartbeat too old)
    Stale { pc_name: String },
    /// Lock is actively held by another instance
    Locked {
        pc_name: String,
        since: DateTime<Utc>,
    },
}

/// Threshold in seconds after which a lock is considered stale (2 minutes)
const STALE_THRESHOLD_SECONDS: i64 = 120;

/// Resolve database paths based on custom path setting.
///
/// # Arguments
/// *  - The application data directory (fallback location)
/// *  - Optional custom path for database storage
///
/// # Returns
/// A tuple of (DbPaths, is_custom) where is_custom is true if using custom path.
pub fn resolve_db_paths(app_data_dir: &PathBuf, custom_db_path: Option<&str>) -> (DbPaths, bool) {
    match custom_db_path {
        Some(custom_path) if !custom_path.is_empty() => {
            let base = PathBuf::from(custom_path);
            let paths = DbPaths {
                db_file: base.join("kniha-jazd.db"),
                lock_file: base.join("kniha-jazd.lock"),
                backups_dir: base.join("backups"),
            };
            (paths, true)
        }
        _ => {
            let paths = DbPaths {
                db_file: app_data_dir.join("kniha-jazd.db"),
                lock_file: app_data_dir.join("kniha-jazd.lock"),
                backups_dir: app_data_dir.join("backups"),
            };
            (paths, false)
        }
    }
}

/// Check the status of the lock file.
///
/// # Arguments
/// *  - Path to the lock file
///
/// # Returns
/// The current lock status.
pub fn check_lock(lock_path: &PathBuf) -> LockStatus {
    if !lock_path.exists() {
        return LockStatus::Free;
    }

    match fs::read_to_string(lock_path) {
        Ok(content) => match serde_json::from_str::<LockFile>(&content) {
            Ok(lock) => {
                let now = Utc::now();
                let elapsed = now.signed_duration_since(lock.last_heartbeat);

                if elapsed.num_seconds() > STALE_THRESHOLD_SECONDS {
                    LockStatus::Stale {
                        pc_name: lock.pc_name,
                    }
                } else {
                    LockStatus::Locked {
                        pc_name: lock.pc_name,
                        since: lock.opened_at,
                    }
                }
            }
            Err(_) => {
                // Corrupted lock file - treat as free
                LockStatus::Free
            }
        },
        Err(_) => {
            // Cannot read file - treat as free
            LockStatus::Free
        }
    }
}

/// Acquire a lock on the database.
///
/// # Arguments
/// *  - Path to the lock file
/// *  - Current app version
///
/// # Returns
/// Ok(()) if lock acquired successfully, Err with description if failed.
pub fn acquire_lock(lock_path: &PathBuf, app_version: &str) -> io::Result<()> {
    let now = Utc::now();
    let pc_name = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let lock = LockFile {
        pc_name,
        opened_at: now,
        last_heartbeat: now,
        app_version: app_version.to_string(),
        pid: std::process::id(),
    };

    let json = serde_json::to_string_pretty(&lock)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Ensure parent directory exists
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(lock_path)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;

    Ok(())
}

/// Refresh the lock heartbeat.
///
/// # Arguments
/// *  - Path to the lock file
///
/// # Returns
/// Ok(()) if heartbeat updated successfully, Err if failed.
pub fn refresh_lock(lock_path: &PathBuf) -> io::Result<()> {
    let content = fs::read_to_string(lock_path)?;
    let mut lock: LockFile = serde_json::from_str(&content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    lock.last_heartbeat = Utc::now();

    let json = serde_json::to_string_pretty(&lock)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let mut file = fs::File::create(lock_path)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;

    Ok(())
}

/// Release the lock.
///
/// # Arguments
/// *  - Path to the lock file
///
/// # Returns
/// Ok(()) if lock released successfully, Err if failed.
pub fn release_lock(lock_path: &PathBuf) -> io::Result<()> {
    if lock_path.exists() {
        fs::remove_file(lock_path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_resolve_default_paths() {
        let app_dir = PathBuf::from("/app/data");
        let (paths, is_custom) = resolve_db_paths(&app_dir, None);

        assert!(!is_custom);
        assert_eq!(paths.db_file, PathBuf::from("/app/data/kniha-jazd.db"));
        assert_eq!(paths.lock_file, PathBuf::from("/app/data/kniha-jazd.lock"));
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
            paths.lock_file,
            PathBuf::from("D:/GoogleDrive/kniha-jazd/kniha-jazd.lock")
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

    #[test]
    fn test_check_lock_free_when_no_file() {
        let dir = tempdir().unwrap();
        let lock_path = dir.path().join("test.lock");

        match check_lock(&lock_path) {
            LockStatus::Free => (),
            _ => panic!("Expected Free status"),
        }
    }

    #[test]
    fn test_acquire_and_check_lock() {
        let dir = tempdir().unwrap();
        let lock_path = dir.path().join("test.lock");

        acquire_lock(&lock_path, "1.0.0").unwrap();

        match check_lock(&lock_path) {
            LockStatus::Locked { pc_name, since: _ } => {
                assert!(!pc_name.is_empty());
            }
            _ => panic!("Expected Locked status"),
        }
    }

    #[test]
    fn test_release_lock() {
        let dir = tempdir().unwrap();
        let lock_path = dir.path().join("test.lock");

        acquire_lock(&lock_path, "1.0.0").unwrap();
        assert!(lock_path.exists());

        release_lock(&lock_path).unwrap();
        assert!(!lock_path.exists());

        match check_lock(&lock_path) {
            LockStatus::Free => (),
            _ => panic!("Expected Free status after release"),
        }
    }

    #[test]
    fn test_refresh_lock() {
        let dir = tempdir().unwrap();
        let lock_path = dir.path().join("test.lock");

        acquire_lock(&lock_path, "1.0.0").unwrap();

        // Read initial heartbeat
        let content = fs::read_to_string(&lock_path).unwrap();
        let lock1: LockFile = serde_json::from_str(&content).unwrap();

        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        refresh_lock(&lock_path).unwrap();

        // Read updated heartbeat
        let content = fs::read_to_string(&lock_path).unwrap();
        let lock2: LockFile = serde_json::from_str(&content).unwrap();

        assert!(lock2.last_heartbeat >= lock1.last_heartbeat);
        assert_eq!(lock1.opened_at, lock2.opened_at); // opened_at should not change
    }

    #[test]
    fn test_stale_lock_detection() {
        let dir = tempdir().unwrap();
        let lock_path = dir.path().join("test.lock");

        // Create a lock with an old heartbeat
        let old_time = Utc::now() - chrono::Duration::seconds(STALE_THRESHOLD_SECONDS + 10);
        let lock = LockFile {
            pc_name: "old-pc".to_string(),
            opened_at: old_time,
            last_heartbeat: old_time,
            app_version: "1.0.0".to_string(),
            pid: 12345,
        };

        let json = serde_json::to_string(&lock).unwrap();
        fs::write(&lock_path, json).unwrap();

        match check_lock(&lock_path) {
            LockStatus::Stale { pc_name } => {
                assert_eq!(pc_name, "old-pc");
            }
            _ => panic!("Expected Stale status"),
        }
    }

    #[test]
    fn test_corrupted_lock_treated_as_free() {
        let dir = tempdir().unwrap();
        let lock_path = dir.path().join("test.lock");

        // Write corrupted JSON
        fs::write(&lock_path, "not valid json").unwrap();

        match check_lock(&lock_path) {
            LockStatus::Free => (),
            _ => panic!("Expected Free status for corrupted lock"),
        }
    }

    #[test]
    fn test_lock_file_contains_correct_data() {
        let dir = tempdir().unwrap();
        let lock_path = dir.path().join("test.lock");

        acquire_lock(&lock_path, "1.2.3").unwrap();

        let content = fs::read_to_string(&lock_path).unwrap();
        let lock: LockFile = serde_json::from_str(&content).unwrap();

        assert_eq!(lock.app_version, "1.2.3");
        assert_eq!(lock.pid, std::process::id());
        assert!(!lock.pc_name.is_empty());
    }
}

