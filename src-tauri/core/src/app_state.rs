//! Application state management for multi-PC database support.
//!
//! Tracks app mode (Normal/ReadOnly), database path, and whether using custom path.
//! Read-only mode is enabled when:
//! - Database has unknown migrations (from newer app version)
//! - Lock cannot be acquired (another PC is using the database)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};

/// Consecutive wrong PINs before reveal locks out.
const REVEAL_FAILURES_PER_LOCKOUT: u32 = 5;

/// Lockout ladder, in seconds, indexed by how many lockouts have already fired.
/// The last entry is the cap.
const REVEAL_LOCKOUT_LADDER: [u64; 4] = [60, 300, 900, 3600];

/// Brute-force guard for PIN-gated secret reveal.
///
/// The counter is deliberately **global rather than per-IP**: a per-IP counter is
/// defeated by rotating source addresses, which is trivial on a LAN. The cost is
/// that an attacker can lock the operator out — on a closed network, denial is the
/// better failure than disclosure. See task 69's design doc.
#[derive(Default)]
struct RevealThrottle {
    consecutive_failures: u32,
    lockouts_fired: u32,
    locked_until: Option<Instant>,
}

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
    /// Brute-force guard for PIN-gated secret reveal
    reveal_throttle: Mutex<RevealThrottle>,
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
            reveal_throttle: Mutex::new(RevealThrottle::default()),
        }
    }

    /// Is a PIN attempt currently allowed?
    ///
    /// Call BEFORE comparing the PIN, so a lockout can't be distinguished by
    /// timing the comparison.
    pub fn reveal_check(&self) -> Result<(), String> {
        let throttle = self.reveal_throttle.lock().unwrap_or_else(|p| p.into_inner());
        match throttle.locked_until {
            Some(until) if until > Instant::now() => {
                let secs = (until - Instant::now()).as_secs() + 1;
                Err(format!(
                    "Too many incorrect PIN attempts. Try again in {secs} seconds."
                ))
            }
            _ => Ok(()),
        }
    }

    /// Record a wrong PIN, locking out once the threshold is reached.
    pub fn reveal_record_failure(&self) {
        let mut t = self.reveal_throttle.lock().unwrap_or_else(|p| p.into_inner());
        t.consecutive_failures += 1;
        if t.consecutive_failures >= REVEAL_FAILURES_PER_LOCKOUT {
            let idx = (t.lockouts_fired as usize).min(REVEAL_LOCKOUT_LADDER.len() - 1);
            let secs = REVEAL_LOCKOUT_LADDER[idx];
            // Report the ladder value exactly; reveal_check adds 1s of rounding
            // slack, so lock slightly past the boundary.
            t.locked_until = Some(Instant::now() + Duration::from_secs(secs));
            t.lockouts_fired += 1;
            t.consecutive_failures = 0;
        }
    }

    /// Record a correct PIN — clears the lockout and the counter.
    pub fn reveal_record_success(&self) {
        let mut t = self.reveal_throttle.lock().unwrap_or_else(|p| p.into_inner());
        *t = RevealThrottle::default();
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
    fn reveal_throttle_allows_until_threshold() {
        let s = AppState::new();
        for _ in 0..4 {
            assert!(s.reveal_check().is_ok());
            s.reveal_record_failure();
        }
        // 4 failures recorded — still allowed to try
        assert!(s.reveal_check().is_ok());
        s.reveal_record_failure();
        // 5th failure trips the lockout
        assert!(s.reveal_check().is_err());
    }

    #[test]
    fn reveal_throttle_success_resets() {
        let s = AppState::new();
        for _ in 0..5 {
            s.reveal_record_failure();
        }
        assert!(s.reveal_check().is_err());
        s.reveal_record_success();
        assert!(s.reveal_check().is_ok());
    }

    #[test]
    fn reveal_throttle_error_names_remaining_seconds() {
        let s = AppState::new();
        for _ in 0..5 {
            s.reveal_record_failure();
        }
        let e = s.reveal_check().unwrap_err();
        assert!(e.contains("60"), "lockout error must tell the operator how long: {e}");
    }

    #[test]
    fn reveal_throttle_escalates_across_rounds() {
        let s = AppState::new();
        for _ in 0..5 {
            s.reveal_record_failure();
        }
        assert!(s.reveal_check().unwrap_err().contains("60"));
        // A second round of 5 escalates to the 300s step
        for _ in 0..5 {
            s.reveal_record_failure();
        }
        assert!(s.reveal_check().unwrap_err().contains("300"));
    }

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
        assert_eq!(
            state.get_db_path(),
            Some(PathBuf::from("/custom/db.sqlite"))
        );
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
