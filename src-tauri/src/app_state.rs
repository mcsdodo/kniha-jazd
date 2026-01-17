//! Application state management for multi-PC database support.
//!
//! Tracks app mode (Normal/ReadOnly), database path, and whether using custom path.
//! Read-only mode is enabled when:
//! - Database has unknown migrations (from newer app version)
//! - Lock cannot be acquired (another PC is using the database)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::RwLock;

/// Application mode determining write permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppMode {
    /// Full read/write access
    Normal,
    /// Read-only mode (newer migrations or locked by another PC)
    ReadOnly,
}

impl Default for AppMode {
    fn default() -> Self {
        AppMode::Normal
    }
}

/// Thread-safe application state.
pub struct AppState {
    /// Current application mode
    mode: RwLock<AppMode>,
    /// Path to the active database file
    db_path: RwLock<Option<PathBuf>>,
    /// Whether using a custom database path
    is_custom_path: RwLock<bool>,
    /// Reason for read-only mode (if applicable)
    read_only_reason: RwLock<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Create new app state with default values.
    pub fn new() -> Self {
        Self {
            mode: RwLock::new(AppMode::Normal),
            db_path: RwLock::new(None),
            is_custom_path: RwLock::new(false),
            read_only_reason: RwLock::new(None),
        }
    }

    /// Set the application mode.
    pub fn set_mode(&self, mode: AppMode) {
        *self.mode.write().unwrap() = mode;
    }

    /// Get the current application mode.
    pub fn get_mode(&self) -> AppMode {
        *self.mode.read().unwrap()
    }

    /// Check if app is in read-only mode.
    pub fn is_read_only(&self) -> bool {
        *self.mode.read().unwrap() == AppMode::ReadOnly
    }

    /// Set the database path and whether it's a custom location.
    pub fn set_db_path(&self, path: PathBuf, is_custom: bool) {
        *self.db_path.write().unwrap() = Some(path);
        *self.is_custom_path.write().unwrap() = is_custom;
    }

    /// Get the database path.
    pub fn get_db_path(&self) -> Option<PathBuf> {
        self.db_path.read().unwrap().clone()
    }

    /// Check if using custom database path.
    pub fn is_custom_path(&self) -> bool {
        *self.is_custom_path.read().unwrap()
    }

    /// Set the reason for read-only mode.
    pub fn set_read_only_reason(&self, reason: Option<String>) {
        *self.read_only_reason.write().unwrap() = reason;
    }

    /// Get the reason for read-only mode.
    pub fn get_read_only_reason(&self) -> Option<String> {
        self.read_only_reason.read().unwrap().clone()
    }

    /// Enable read-only mode with a reason.
    pub fn enable_read_only(&self, reason: &str) {
        self.set_mode(AppMode::ReadOnly);
        self.set_read_only_reason(Some(reason.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode_is_normal() {
        let state = AppState::new();
        assert_eq!(state.get_mode(), AppMode::Normal);
        assert!(!state.is_read_only());
    }

    #[test]
    fn test_set_mode() {
        let state = AppState::new();

        state.set_mode(AppMode::ReadOnly);
        assert_eq!(state.get_mode(), AppMode::ReadOnly);
        assert!(state.is_read_only());

        state.set_mode(AppMode::Normal);
        assert_eq!(state.get_mode(), AppMode::Normal);
        assert!(!state.is_read_only());
    }

    #[test]
    fn test_db_path() {
        let state = AppState::new();
        assert!(state.get_db_path().is_none());

        state.set_db_path(PathBuf::from("/test/db.sqlite"), false);
        assert_eq!(state.get_db_path(), Some(PathBuf::from("/test/db.sqlite")));
        assert!(!state.is_custom_path());

        state.set_db_path(PathBuf::from("/custom/db.sqlite"), true);
        assert_eq!(state.get_db_path(), Some(PathBuf::from("/custom/db.sqlite")));
        assert!(state.is_custom_path());
    }

    #[test]
    fn test_custom_path() {
        let state = AppState::new();
        assert!(!state.is_custom_path());

        // Setting db path with is_custom=true should update is_custom_path
        state.set_db_path(PathBuf::from("/custom/db.sqlite"), true);
        assert!(state.is_custom_path());

        // Setting db path with is_custom=false should update is_custom_path
        state.set_db_path(PathBuf::from("/default/db.sqlite"), false);
        assert!(!state.is_custom_path());
    }

    #[test]
    fn test_read_only_reason() {
        let state = AppState::new();
        assert!(state.get_read_only_reason().is_none());

        state.enable_read_only("Database locked by another PC");
        assert!(state.is_read_only());
        assert_eq!(
            state.get_read_only_reason(),
            Some("Database locked by another PC".to_string())
        );
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(AppState::new());

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let state = Arc::clone(&state);
                thread::spawn(move || {
                    if i % 2 == 0 {
                        state.set_mode(AppMode::ReadOnly);
                    } else {
                        state.set_mode(AppMode::Normal);
                    }
                    state.get_mode();
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Just verify no panics occurred - final state is non-deterministic
        let _ = state.get_mode();
    }
}
