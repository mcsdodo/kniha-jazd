# Custom Database Location Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Allow users to store the database on Google Drive/NAS for multi-PC usage, with version mismatch protection and concurrent access prevention.

**Architecture:** Extend `LocalSettings` with `custom_db_path`, create new `db_location.rs` module for path resolution/lock files/version checking. Add `AppMode` managed state for read-only mode. Refactor startup flow in `lib.rs`.

**Tech Stack:** Rust (Tauri backend), SvelteKit (frontend), Diesel (migrations), SQLite

---

## Phase 1: Backend Foundation

### Task 1: Add `custom_db_path` to LocalSettings

**Files:**
- Modify: `src-tauri/src/settings.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_load_with_custom_db_path() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("local.settings.json");
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(b"{\"custom_db_path\": \"G:\\\\GoogleDrive\\\\kniha-jazd\"}").unwrap();

    let settings = LocalSettings::load(&dir.path().to_path_buf());
    assert_eq!(settings.custom_db_path, Some("G:\\GoogleDrive\\kniha-jazd".to_string()));
}
```

**Step 2: Run test to verify it fails**

```bash
cd src-tauri && cargo test test_load_with_custom_db_path -- --nocapture
```
Expected: FAIL - `custom_db_path` field doesn't exist

**Step 3: Add field to LocalSettings struct**

In `src-tauri/src/settings.rs`, add to `LocalSettings`:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalSettings {
    pub gemini_api_key: Option<String>,
    pub receipts_folder_path: Option<String>,
    pub theme: Option<String>,
    pub auto_check_updates: Option<bool>,
    pub custom_db_path: Option<String>,  // NEW
}
```

**Step 4: Run test to verify it passes**

```bash
cd src-tauri && cargo test test_load_with_custom_db_path -- --nocapture
```
Expected: PASS

**Step 5: Add save functionality**

Add method to `LocalSettings`:

```rust
impl LocalSettings {
    // ... existing load() ...

    /// Save to local.settings.json in app data dir
    pub fn save(&self, app_data_dir: &PathBuf) -> std::io::Result<()> {
        let path = app_data_dir.join("local.settings.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, json)
    }
}
```

**Step 6: Commit**

```bash
git add src-tauri/src/settings.rs
git commit -m "feat(settings): add custom_db_path to LocalSettings"
```

---

### Task 2: Create `db_location.rs` Module - Path Resolution

**Files:**
- Create: `src-tauri/src/db_location.rs`
- Modify: `src-tauri/src/lib.rs` (add module declaration)

**Step 1: Create module file with path resolution test**

Create `src-tauri/src/db_location.rs`:

```rust
//! Database location management for multi-PC support.
//!
//! Handles:
//! - Custom database path resolution
//! - Lock file management for concurrent access prevention
//! - Migration version compatibility checking

use std::path::PathBuf;

/// Resolved database paths
#[derive(Debug, Clone)]
pub struct DbPaths {
    pub db_file: PathBuf,
    pub lock_file: PathBuf,
    pub backups_dir: PathBuf,
}

impl DbPaths {
    /// Create paths from a base directory
    pub fn from_dir(base_dir: &PathBuf) -> Self {
        Self {
            db_file: base_dir.join("kniha-jazd.db"),
            lock_file: base_dir.join("kniha-jazd.db.lock"),
            backups_dir: base_dir.join("backups"),
        }
    }
}

/// Resolve the database directory based on settings
/// Returns (db_paths, is_custom_path)
pub fn resolve_db_paths(
    app_data_dir: &PathBuf,
    custom_db_path: Option<&str>,
) -> (DbPaths, bool) {
    match custom_db_path {
        Some(custom_path) => {
            let custom_dir = PathBuf::from(custom_path);
            (DbPaths::from_dir(&custom_dir), true)
        }
        None => (DbPaths::from_dir(app_data_dir), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_default_path() {
        let app_dir = PathBuf::from("C:\\Users\\Test\\AppData\\Roaming\\kniha-jazd");
        let (paths, is_custom) = resolve_db_paths(&app_dir, None);

        assert!(!is_custom);
        assert_eq!(paths.db_file, app_dir.join("kniha-jazd.db"));
        assert_eq!(paths.lock_file, app_dir.join("kniha-jazd.db.lock"));
        assert_eq!(paths.backups_dir, app_dir.join("backups"));
    }

    #[test]
    fn test_resolve_custom_path() {
        let app_dir = PathBuf::from("C:\\Users\\Test\\AppData\\Roaming\\kniha-jazd");
        let custom = "G:\\GoogleDrive\\kniha-jazd";
        let (paths, is_custom) = resolve_db_paths(&app_dir, Some(custom));

        assert!(is_custom);
        assert_eq!(paths.db_file, PathBuf::from("G:\\GoogleDrive\\kniha-jazd\\kniha-jazd.db"));
        assert_eq!(paths.lock_file, PathBuf::from("G:\\GoogleDrive\\kniha-jazd\\kniha-jazd.db.lock"));
    }
}
```

**Step 2: Add module to lib.rs**

In `src-tauri/src/lib.rs`, add after other mod declarations:

```rust
mod db_location;
```

**Step 3: Run tests**

```bash
cd src-tauri && cargo test db_location -- --nocapture
```
Expected: PASS

**Step 4: Commit**

```bash
git add src-tauri/src/db_location.rs src-tauri/src/lib.rs
git commit -m "feat(db_location): add path resolution module"
```

---

### Task 3: Add Lock File Mechanism

**Files:**
- Modify: `src-tauri/src/db_location.rs`

**Step 1: Add lock file types and tests**

Add to `db_location.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;

/// Lock file content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockFile {
    pub pc_name: String,
    pub opened_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub app_version: String,
    pub pid: u32,
}

/// Result of checking lock file
#[derive(Debug, Clone, PartialEq)]
pub enum LockStatus {
    /// No lock file exists
    Free,
    /// Lock exists but is stale (app crashed)
    Stale { pc_name: String },
    /// Lock is active (another instance running)
    Locked { pc_name: String, since: DateTime<Utc> },
}

const STALE_THRESHOLD_MINUTES: i64 = 5;

impl LockFile {
    /// Create a new lock file for this instance
    pub fn new(app_version: &str) -> Self {
        let now = Utc::now();
        Self {
            pc_name: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "Unknown".to_string()),
            opened_at: now,
            last_heartbeat: now,
            app_version: app_version.to_string(),
            pid: std::process::id(),
        }
    }

    /// Check if this lock is stale
    pub fn is_stale(&self) -> bool {
        let age = Utc::now() - self.last_heartbeat;
        age.num_minutes() >= STALE_THRESHOLD_MINUTES
    }
}

/// Check the status of a lock file
pub fn check_lock(lock_path: &PathBuf) -> LockStatus {
    if !lock_path.exists() {
        return LockStatus::Free;
    }

    match fs::read_to_string(lock_path) {
        Ok(content) => match serde_json::from_str::<LockFile>(&content) {
            Ok(lock) => {
                if lock.is_stale() {
                    LockStatus::Stale { pc_name: lock.pc_name }
                } else {
                    LockStatus::Locked {
                        pc_name: lock.pc_name,
                        since: lock.opened_at,
                    }
                }
            }
            Err(_) => LockStatus::Free, // Corrupted lock file, treat as free
        },
        Err(_) => LockStatus::Free,
    }
}

/// Create or update the lock file
pub fn acquire_lock(lock_path: &PathBuf, app_version: &str) -> std::io::Result<()> {
    let lock = LockFile::new(app_version);
    let json = serde_json::to_string_pretty(&lock)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(lock_path, json)
}

/// Update the heartbeat timestamp
pub fn refresh_lock(lock_path: &PathBuf) -> std::io::Result<()> {
    if !lock_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(lock_path)?;
    let mut lock: LockFile = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    lock.last_heartbeat = Utc::now();

    let json = serde_json::to_string_pretty(&lock)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(lock_path, json)
}

/// Release the lock file
pub fn release_lock(lock_path: &PathBuf) -> std::io::Result<()> {
    if lock_path.exists() {
        fs::remove_file(lock_path)?;
    }
    Ok(())
}
```

**Step 2: Add tests for lock mechanism**

Add to tests module in `db_location.rs`:

```rust
use tempfile::tempdir;

#[test]
fn test_check_lock_free() {
    let dir = tempdir().unwrap();
    let lock_path = dir.path().join("test.lock");

    assert_eq!(check_lock(&lock_path), LockStatus::Free);
}

#[test]
fn test_acquire_and_check_lock() {
    let dir = tempdir().unwrap();
    let lock_path = dir.path().join("test.lock");

    acquire_lock(&lock_path, "0.17.0").unwrap();

    match check_lock(&lock_path) {
        LockStatus::Locked { pc_name, .. } => {
            assert!(!pc_name.is_empty());
        }
        other => panic!("Expected Locked, got {:?}", other),
    }
}

#[test]
fn test_release_lock() {
    let dir = tempdir().unwrap();
    let lock_path = dir.path().join("test.lock");

    acquire_lock(&lock_path, "0.17.0").unwrap();
    assert!(lock_path.exists());

    release_lock(&lock_path).unwrap();
    assert!(!lock_path.exists());
}

#[test]
fn test_stale_lock_detection() {
    let dir = tempdir().unwrap();
    let lock_path = dir.path().join("test.lock");

    // Create a lock with old heartbeat
    let old_lock = LockFile {
        pc_name: "OLD-PC".to_string(),
        opened_at: Utc::now() - chrono::Duration::minutes(10),
        last_heartbeat: Utc::now() - chrono::Duration::minutes(10),
        app_version: "0.15.0".to_string(),
        pid: 12345,
    };
    let json = serde_json::to_string_pretty(&old_lock).unwrap();
    fs::write(&lock_path, json).unwrap();

    match check_lock(&lock_path) {
        LockStatus::Stale { pc_name } => {
            assert_eq!(pc_name, "OLD-PC");
        }
        other => panic!("Expected Stale, got {:?}", other),
    }
}
```

**Step 3: Add hostname dependency to Cargo.toml**

```bash
cd src-tauri && cargo add hostname
```

**Step 4: Run tests**

```bash
cd src-tauri && cargo test db_location -- --nocapture
```
Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/src/db_location.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat(db_location): add lock file mechanism"
```

---

### Task 4: Add Migration Version Compatibility Check

**Files:**
- Modify: `src-tauri/src/db_location.rs`
- Modify: `src-tauri/src/db.rs`

**Step 1: Add migration check function to db.rs**

In `src-tauri/src/db.rs`, add after the `MIGRATIONS` constant:

```rust
use std::collections::HashSet;

/// Get the set of migration versions embedded in this app
pub fn get_embedded_migration_versions() -> HashSet<String> {
    MIGRATIONS
        .migrations()
        .unwrap()
        .iter()
        .map(|m| m.name().version().to_string())
        .collect()
}

/// Check if a database has migrations unknown to this app version
/// Returns Ok(()) if compatible, Err with unknown migration names if not
pub fn check_migration_compatibility(
    conn: &mut SqliteConnection,
) -> Result<(), Vec<String>> {
    use diesel::sql_query;
    use diesel::sql_types::Text;

    #[derive(QueryableByName)]
    struct MigrationVersion {
        #[diesel(sql_type = Text)]
        version: String,
    }

    let db_migrations: Vec<MigrationVersion> = sql_query(
        "SELECT version FROM __diesel_schema_migrations ORDER BY version"
    )
    .load(conn)
    .unwrap_or_default();

    let embedded = get_embedded_migration_versions();
    let unknown: Vec<String> = db_migrations
        .into_iter()
        .filter(|m| !embedded.contains(&m.version))
        .map(|m| m.version)
        .collect();

    if unknown.is_empty() {
        Ok(())
    } else {
        Err(unknown)
    }
}
```

**Step 2: Add test for migration compatibility**

In `src-tauri/src/db.rs` tests section:

```rust
#[test]
fn test_get_embedded_migration_versions() {
    let versions = get_embedded_migration_versions();
    // Should have at least the baseline migration
    assert!(!versions.is_empty());
    assert!(versions.iter().any(|v| v.contains("baseline")));
}

#[test]
fn test_migration_compatibility_check_passes() {
    let db = Database::in_memory().unwrap();
    let mut conn = db.conn.lock().unwrap();

    // In-memory DB has same migrations as embedded, should pass
    let result = check_migration_compatibility(&mut conn);
    assert!(result.is_ok());
}
```

**Step 3: Run tests**

```bash
cd src-tauri && cargo test migration_compatibility -- --nocapture
cd src-tauri && cargo test get_embedded_migration -- --nocapture
```
Expected: PASS

**Step 4: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat(db): add migration compatibility checking"
```

---

## Phase 2: App State Management

### Task 5: Add AppMode Managed State

**Files:**
- Create: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Create app state module**

Create `src-tauri/src/app_state.rs`:

```rust
//! Application runtime state management.

use std::sync::RwLock;
use std::path::PathBuf;

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AppMode {
    #[default]
    Normal,
    ReadOnly,
}

/// Application state managed by Tauri
pub struct AppState {
    mode: RwLock<AppMode>,
    db_path: RwLock<Option<PathBuf>>,
    is_custom_path: RwLock<bool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: RwLock::new(AppMode::Normal),
            db_path: RwLock::new(None),
            is_custom_path: RwLock::new(false),
        }
    }

    pub fn set_mode(&self, mode: AppMode) {
        *self.mode.write().unwrap() = mode;
    }

    pub fn get_mode(&self) -> AppMode {
        *self.mode.read().unwrap()
    }

    pub fn is_read_only(&self) -> bool {
        self.get_mode() == AppMode::ReadOnly
    }

    pub fn set_db_path(&self, path: PathBuf, is_custom: bool) {
        *self.db_path.write().unwrap() = Some(path);
        *self.is_custom_path.write().unwrap() = is_custom;
    }

    pub fn get_db_path(&self) -> Option<PathBuf> {
        self.db_path.read().unwrap().clone()
    }

    pub fn is_custom_path(&self) -> bool {
        *self.is_custom_path.read().unwrap()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_default_mode() {
        let state = AppState::new();
        assert_eq!(state.get_mode(), AppMode::Normal);
        assert!(!state.is_read_only());
    }

    #[test]
    fn test_app_state_set_read_only() {
        let state = AppState::new();
        state.set_mode(AppMode::ReadOnly);
        assert_eq!(state.get_mode(), AppMode::ReadOnly);
        assert!(state.is_read_only());
    }

    #[test]
    fn test_app_state_db_path() {
        let state = AppState::new();
        let path = PathBuf::from("C:\\test\\db.db");
        state.set_db_path(path.clone(), true);

        assert_eq!(state.get_db_path(), Some(path));
        assert!(state.is_custom_path());
    }
}
```

**Step 2: Add module to lib.rs**

In `src-tauri/src/lib.rs`:

```rust
mod app_state;
```

**Step 3: Run tests**

```bash
cd src-tauri && cargo test app_state -- --nocapture
```
Expected: PASS

**Step 4: Commit**

```bash
git add src-tauri/src/app_state.rs src-tauri/src/lib.rs
git commit -m "feat(app_state): add AppMode for read-only tracking"
```

---

### Task 6: Add Read-Only Guards to Write Commands

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Step 1: Add helper macro for read-only check**

At the top of `src-tauri/src/commands.rs` (after imports):

```rust
use crate::app_state::AppState;

/// Macro to check read-only mode and return error if active
macro_rules! check_read_only {
    ($app_state:expr) => {
        if $app_state.is_read_only() {
            return Err("Aplikácia je v režime len na čítanie. Aktualizujte aplikáciu.".to_string());
        }
    };
}
```

**Step 2: Add read-only checks to write commands**

Add `State<AppState>` parameter and `check_read_only!` to each write command. Example for `create_vehicle`:

```rust
#[tauri::command]
pub async fn create_vehicle(
    // ... existing params ...
    app_state: tauri::State<'_, AppState>,  // ADD THIS
) -> Result<Vehicle, String> {
    check_read_only!(app_state);  // ADD THIS
    // ... rest of function unchanged ...
}
```

Commands to update:
- `create_vehicle`
- `update_vehicle`
- `delete_vehicle`
- `set_active_vehicle`
- `create_trip`
- `update_trip`
- `delete_trip`
- `reorder_trip`
- `save_settings`
- `create_backup`
- `restore_backup`
- `delete_backup`
- `update_receipt`
- `delete_receipt`
- `scan_receipts`
- `sync_receipts`
- `process_pending_receipts`
- `reprocess_receipt`
- `assign_receipt_to_trip`

**Step 3: Update lib.rs to manage AppState**

In `lib.rs` setup hook, add:

```rust
let app_state = app_state::AppState::new();
app.manage(app_state);
```

**Step 4: Run backend tests**

```bash
cd src-tauri && cargo test
```
Expected: PASS (tests use Database::in_memory, no AppState needed)

**Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(commands): add read-only mode guards to write operations"
```

---

## Phase 3: New Commands

### Task 7: Add Database Location Commands

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs` (register commands)

**Step 1: Add get_db_location command**

In `src-tauri/src/commands.rs`:

```rust
#[derive(Serialize)]
pub struct DbLocationInfo {
    pub path: String,
    pub is_custom: bool,
    pub is_read_only: bool,
}

#[tauri::command]
pub fn get_db_location(
    app_state: tauri::State<'_, AppState>,
) -> Result<DbLocationInfo, String> {
    let path = app_state
        .get_db_path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(DbLocationInfo {
        path,
        is_custom: app_state.is_custom_path(),
        is_read_only: app_state.is_read_only(),
    })
}
```

**Step 2: Add get_app_mode command**

```rust
#[derive(Serialize)]
pub struct AppModeInfo {
    pub is_read_only: bool,
    pub reason: Option<String>,
}

#[tauri::command]
pub fn get_app_mode(
    app_state: tauri::State<'_, AppState>,
) -> AppModeInfo {
    AppModeInfo {
        is_read_only: app_state.is_read_only(),
        reason: if app_state.is_read_only() {
            Some("Databáza bola aktualizovaná novšou verziou aplikácie.".to_string())
        } else {
            None
        },
    }
}
```

**Step 3: Register commands in lib.rs**

Add to `invoke_handler`:

```rust
commands::get_db_location,
commands::get_app_mode,
```

**Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(commands): add get_db_location and get_app_mode"
```

---

### Task 8: Add move_database Command

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Step 1: Add move_database command**

```rust
use crate::db_location::{DbPaths, acquire_lock, release_lock};

#[derive(Serialize)]
pub struct MoveDbResult {
    pub success: bool,
    pub new_path: String,
    pub files_moved: usize,
}

#[tauri::command]
pub async fn move_database(
    app_handle: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
    target_folder: String,
) -> Result<MoveDbResult, String> {
    check_read_only!(app_state);

    let target_dir = PathBuf::from(&target_folder);

    // Validate target directory
    if !target_dir.exists() {
        std::fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Nepodarilo sa vytvoriť priečinok: {}", e))?;
    }

    // Get current paths
    let current_path = app_state.get_db_path()
        .ok_or("Cesta k databáze nie je nastavená")?;
    let current_dir = current_path.parent()
        .ok_or("Neplatná cesta k databáze")?;

    let source_paths = DbPaths::from_dir(&current_dir.to_path_buf());
    let target_paths = DbPaths::from_dir(&target_dir);

    let mut files_moved = 0;

    // Copy database file
    if source_paths.db_file.exists() {
        std::fs::copy(&source_paths.db_file, &target_paths.db_file)
            .map_err(|e| format!("Nepodarilo sa skopírovať databázu: {}", e))?;
        files_moved += 1;
    }

    // Copy backups directory
    if source_paths.backups_dir.exists() {
        copy_dir_all(&source_paths.backups_dir, &target_paths.backups_dir)
            .map_err(|e| format!("Nepodarilo sa skopírovať zálohy: {}", e))?;
        files_moved += count_files(&target_paths.backups_dir);
    }

    // Update local.settings.json
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut settings = crate::settings::LocalSettings::load(&app_data_dir);
    settings.custom_db_path = Some(target_folder.clone());
    settings.save(&app_data_dir)
        .map_err(|e| format!("Nepodarilo sa uložiť nastavenia: {}", e))?;

    // Create lock file in new location
    let version = env!("CARGO_PKG_VERSION");
    acquire_lock(&target_paths.lock_file, version)
        .map_err(|e| format!("Nepodarilo sa vytvoriť zámok: {}", e))?;

    // Release old lock
    let _ = release_lock(&source_paths.lock_file);

    // Delete old files (after successful copy)
    let _ = std::fs::remove_file(&source_paths.db_file);
    let _ = std::fs::remove_dir_all(&source_paths.backups_dir);

    // Update app state
    app_state.set_db_path(target_paths.db_file, true);

    Ok(MoveDbResult {
        success: true,
        new_path: target_folder,
        files_moved,
    })
}

// Helper functions
fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn count_files(dir: &PathBuf) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| entries.count())
        .unwrap_or(0)
}
```

**Step 2: Add check_target_has_db command**

```rust
#[tauri::command]
pub fn check_target_has_db(target_folder: String) -> bool {
    let target_dir = PathBuf::from(&target_folder);
    let db_path = target_dir.join("kniha-jazd.db");
    db_path.exists()
}
```

**Step 3: Register commands**

Add to `invoke_handler`:

```rust
commands::move_database,
commands::check_target_has_db,
```

**Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(commands): add move_database and check_target_has_db"
```

---

## Phase 4: Startup Flow Refactor

### Task 9: Refactor lib.rs for New Startup Sequence

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Rewrite setup hook**

Replace the setup hook in `lib.rs`:

```rust
use crate::app_state::{AppMode, AppState};
use crate::db_location::{resolve_db_paths, check_lock, acquire_lock, LockStatus};
use crate::settings::LocalSettings;

.setup(|app| {
    if cfg!(debug_assertions) {
        app.handle().plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )?;
    }

    // Initialize app state
    let app_state = AppState::new();

    // Get app data directory
    let app_dir = match std::env::var("KNIHA_JAZD_DATA_DIR") {
        Ok(path) => PathBuf::from(path),
        Err(_) => app.path().app_data_dir().expect("Failed to get app data dir"),
    };
    std::fs::create_dir_all(&app_dir).expect("Failed to create app data directory");

    // Load local settings to check for custom path
    let local_settings = LocalSettings::load(&app_dir);

    // Resolve database paths
    let (db_paths, is_custom) = resolve_db_paths(
        &app_dir,
        local_settings.custom_db_path.as_deref(),
    );

    // Ensure target directory exists
    if let Some(parent) = db_paths.db_file.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // Check lock file (warn but don't block for now)
    match check_lock(&db_paths.lock_file) {
        LockStatus::Locked { pc_name, since } => {
            log::warn!(
                "Database appears to be open on {} since {}",
                pc_name,
                since
            );
            // TODO: Emit event to frontend to show warning dialog
        }
        LockStatus::Stale { pc_name } => {
            log::info!("Taking over stale lock from {}", pc_name);
        }
        LockStatus::Free => {}
    }

    // Initialize database
    let db = db::Database::new(db_paths.db_file.clone())
        .expect("Failed to initialize database");

    // Check migration compatibility
    {
        let mut conn = db.connection();
        match db::check_migration_compatibility(&mut conn) {
            Ok(()) => {
                log::info!("Database migration compatibility: OK");
            }
            Err(unknown_migrations) => {
                log::warn!(
                    "Database has unknown migrations: {:?}. Entering read-only mode.",
                    unknown_migrations
                );
                app_state.set_mode(AppMode::ReadOnly);
            }
        }
    }

    // Acquire lock
    let version = env!("CARGO_PKG_VERSION");
    if let Err(e) = acquire_lock(&db_paths.lock_file, version) {
        log::warn!("Failed to acquire lock: {}", e);
    }

    // Store paths in state
    app_state.set_db_path(db_paths.db_file, is_custom);

    // Manage state
    app.manage(db);
    app.manage(app_state);

    Ok(())
})
```

**Step 2: Add cleanup on exit**

Add before `.run()`:

```rust
.on_event(|app, event| {
    if let tauri::RunEvent::Exit = event {
        // Release lock file on clean exit
        if let Some(app_state) = app.try_state::<AppState>() {
            if let Some(db_path) = app_state.get_db_path() {
                if let Some(parent) = db_path.parent() {
                    let lock_path = parent.join("kniha-jazd.db.lock");
                    let _ = db_location::release_lock(&lock_path);
                }
            }
        }
    }
})
```

**Step 3: Run backend tests**

```bash
cd src-tauri && cargo test
```
Expected: PASS

**Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(startup): implement new startup flow with lock/version checks"
```

---

## Phase 5: Frontend Implementation

### Task 10: Add i18n Translations

**Files:**
- Modify: `src/lib/i18n/sk/index.ts`
- Modify: `src/lib/i18n/en/index.ts`

**Step 1: Add Slovak translations**

Add to `src/lib/i18n/sk/index.ts` in the `settings` section:

```typescript
// Database location
dbLocation: {
    sectionTitle: 'Umiestnenie databázy',
    currentPath: 'Aktuálne umiestnenie',
    customPath: 'Vlastná cesta',
    defaultPath: 'Predvolená cesta',
    changeLocation: 'Zmeniť umiestnenie...',
    openFolder: 'Otvoriť priečinok',
    hint: 'Databázu môžete presunúť na Google Drive, NAS alebo iný zdieľaný priečinok pre použitie na viacerých PC.',
    moving: 'Presúvam databázu...',
    moveSuccess: 'Databáza bola úspešne presunutá',
    moveError: 'Nepodarilo sa presunúť databázu: {error}',
    confirmMove: 'Presunúť databázu',
    confirmMoveMessage: 'Budú presunuté:',
    confirmMoveDb: 'Databáza (kniha-jazd.db)',
    confirmMoveBackups: 'Zálohy ({count} súborov)',
    targetHasDb: 'Cieľový priečinok už obsahuje databázu',
    useExisting: 'Použiť existujúcu',
    replaceWithMine: 'Nahradiť mojou',
},

// Read-only mode
readOnly: {
    banner: 'Databáza bola aktualizovaná novšou verziou aplikácie. Režim len na čítanie.',
    checkUpdates: 'Skontrolovať aktualizácie',
},

// Lock file warnings
lockWarning: {
    title: 'Databáza je otvorená inde',
    message: 'Databáza sa zdá byť otvorená na počítači {pcName} od {time}. Otvorenie môže spôsobiť problémy.',
    openAnyway: 'Otvoriť napriek tomu',
    cancel: 'Zrušiť',
    stale: 'Predchádzajúca inštancia na {pcName} nebola korektne ukončená.',
},

// Path unavailable
pathUnavailable: {
    title: 'Databáza nedostupná',
    message: 'Nie je možné pristúpiť k databáze na ceste: {path}',
    retry: 'Skúsiť znova',
    useLocal: 'Použiť lokálnu databázu',
    changeSettings: 'Zmeniť cestu v nastaveniach',
},
```

**Step 2: Add English translations**

Add equivalent translations to `src/lib/i18n/en/index.ts`.

**Step 3: Commit**

```bash
git add src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts
git commit -m "feat(i18n): add translations for database location feature"
```

---

### Task 11: Add API Functions

**Files:**
- Modify: `src/lib/api.ts`
- Modify: `src/lib/types.ts`

**Step 1: Add types**

In `src/lib/types.ts`:

```typescript
export interface DbLocationInfo {
    path: string;
    isCustom: boolean;
    isReadOnly: boolean;
}

export interface AppModeInfo {
    isReadOnly: boolean;
    reason: string | null;
}

export interface MoveDbResult {
    success: boolean;
    newPath: string;
    filesMoved: number;
}
```

**Step 2: Add API functions**

In `src/lib/api.ts`:

```typescript
export async function getDbLocation(): Promise<DbLocationInfo> {
    return invoke('get_db_location');
}

export async function getAppMode(): Promise<AppModeInfo> {
    return invoke('get_app_mode');
}

export async function moveDatabase(targetFolder: string): Promise<MoveDbResult> {
    return invoke('move_database', { targetFolder });
}

export async function checkTargetHasDb(targetFolder: string): Promise<boolean> {
    return invoke('check_target_has_db', { targetFolder });
}
```

**Step 3: Commit**

```bash
git add src/lib/api.ts src/lib/types.ts
git commit -m "feat(api): add database location API functions"
```

---

### Task 12: Add Database Location Section to Settings

**Files:**
- Modify: `src/routes/settings/+page.svelte`

**Step 1: Add state and imports**

Add to script section:

```typescript
import { open } from '@tauri-apps/plugin-dialog';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import * as api from '$lib/api';
import type { DbLocationInfo } from '$lib/types';

// Database location state
let dbLocation: DbLocationInfo | null = null;
let movingDb = false;
```

**Step 2: Load database location in onMount**

Add to the async IIFE in onMount:

```typescript
dbLocation = await api.getDbLocation();
```

**Step 3: Add handler functions**

```typescript
async function handleChangeDbLocation() {
    const selected = await open({
        directory: true,
        multiple: false,
        title: $LL.settings.dbLocation.changeLocation(),
    });

    if (!selected) return;

    const targetFolder = selected as string;

    // Check if target has existing DB
    const hasExisting = await api.checkTargetHasDb(targetFolder);
    if (hasExisting) {
        // TODO: Show dialog with options
        toast.info($LL.settings.dbLocation.targetHasDb());
        return;
    }

    movingDb = true;
    try {
        const result = await api.moveDatabase(targetFolder);
        if (result.success) {
            dbLocation = await api.getDbLocation();
            toast.success($LL.settings.dbLocation.moveSuccess());
        }
    } catch (error) {
        toast.error($LL.settings.dbLocation.moveError({ error: String(error) }));
    } finally {
        movingDb = false;
    }
}

async function handleOpenDbFolder() {
    if (dbLocation?.path) {
        await revealItemInDir(dbLocation.path);
    }
}
```

**Step 4: Add UI section**

Add after Backup Section in the template:

```svelte
<!-- Database Location Section -->
<section class="settings-section">
    <h2>{$LL.settings.dbLocation.sectionTitle()}</h2>
    <div class="section-content">
        <div class="form-group">
            <label>{$LL.settings.dbLocation.currentPath()}</label>
            <div class="path-display">
                <span class="path-text">{dbLocation?.path || '...'}</span>
                {#if dbLocation?.isCustom}
                    <span class="badge">{$LL.settings.dbLocation.customPath()}</span>
                {:else}
                    <span class="badge secondary">{$LL.settings.dbLocation.defaultPath()}</span>
                {/if}
            </div>
        </div>

        <div class="button-row">
            <button
                class="button"
                on:click={handleChangeDbLocation}
                disabled={movingDb || dbLocation?.isReadOnly}
            >
                {movingDb ? $LL.settings.dbLocation.moving() : $LL.settings.dbLocation.changeLocation()}
            </button>
            <button class="button-small" on:click={handleOpenDbFolder}>
                {$LL.settings.dbLocation.openFolder()}
            </button>
        </div>

        <p class="hint">{$LL.settings.dbLocation.hint()}</p>
    </div>
</section>
```

**Step 5: Add styles**

```css
.path-display {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem;
    background: var(--bg-surface-alt);
    border-radius: 4px;
    border: 1px solid var(--border-default);
}

.path-text {
    flex: 1;
    font-family: monospace;
    font-size: 0.875rem;
    color: var(--text-primary);
    word-break: break-all;
}

.badge.secondary {
    background-color: var(--bg-surface-alt);
    color: var(--text-secondary);
}

.button-row {
    display: flex;
    gap: 0.5rem;
}
```

**Step 6: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(settings): add Database Location section"
```

---

### Task 13: Add Read-Only Mode Banner

**Files:**
- Modify: `src/routes/+layout.svelte`
- Create: `src/lib/stores/appMode.ts`

**Step 1: Create app mode store**

Create `src/lib/stores/appMode.ts`:

```typescript
import { writable } from 'svelte/store';
import type { AppModeInfo } from '$lib/types';
import { getAppMode } from '$lib/api';

function createAppModeStore() {
    const { subscribe, set } = writable<AppModeInfo>({
        isReadOnly: false,
        reason: null,
    });

    return {
        subscribe,
        async refresh() {
            const mode = await getAppMode();
            set(mode);
        },
    };
}

export const appModeStore = createAppModeStore();
```

**Step 2: Add banner to layout**

In `src/routes/+layout.svelte`, add after the header:

```svelte
<script>
import { appModeStore } from '$lib/stores/appMode';
import { onMount } from 'svelte';
import { updateStore } from '$lib/stores/update';
import LL from '$lib/i18n/i18n-svelte';

onMount(() => {
    appModeStore.refresh();
});
</script>

{#if $appModeStore.isReadOnly}
    <div class="read-only-banner">
        <span class="banner-icon">⚠️</span>
        <span class="banner-text">{$LL.settings.readOnly.banner()}</span>
        <button class="banner-button" on:click={() => updateStore.checkManual()}>
            {$LL.settings.readOnly.checkUpdates()}
        </button>
    </div>
{/if}
```

**Step 3: Add banner styles**

```css
.read-only-banner {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    background: var(--accent-warning-bg, #fef3c7);
    border-bottom: 1px solid var(--accent-warning, #f59e0b);
    color: var(--accent-warning-text, #92400e);
}

.banner-icon {
    font-size: 1.25rem;
}

.banner-text {
    flex: 1;
    font-weight: 500;
}

.banner-button {
    padding: 0.5rem 1rem;
    background: var(--accent-warning, #f59e0b);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 500;
}

.banner-button:hover {
    opacity: 0.9;
}
```

**Step 4: Commit**

```bash
git add src/lib/stores/appMode.ts src/routes/+layout.svelte
git commit -m "feat(ui): add read-only mode banner"
```

---

## Phase 6: Documentation & Tech Debt

### Task 14: Update CLAUDE.md with Migration Best Practices

**Files:**
- Modify: `CLAUDE.md`

**Step 1: Add migration best practices section**

Add after "### Common Pitfalls" section:

```markdown
### Database Migration Best Practices

**IMPORTANT:** All database migrations MUST be non-destructive and backward compatible:

- **Always** add columns with DEFAULT values
- **Never** remove columns (mark as deprecated if needed)
- **Never** rename columns
- **Never** change column types to incompatible types

**Why?** The app supports read-only mode for older versions accessing newer databases. Older app versions must be able to READ (not write) data from databases migrated by newer versions.

**Example - Good migration:**
```sql
-- Add new column with default
ALTER TABLE trips ADD COLUMN new_field TEXT DEFAULT '';
```

**Example - Bad migration (DO NOT DO):**
```sql
-- Removes column - older apps will crash!
ALTER TABLE trips DROP COLUMN old_field;

-- Renames column - older apps won't find it!
ALTER TABLE trips RENAME COLUMN old_name TO new_name;
```
```

**Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs(claude): add database migration best practices"
```

---

### Task 15: Create Tech Debt Item for Backup Versioning

**Files:**
- Create: `_tasks/_TECH_DEBT/04-backup-restore-versioning.md`

**Step 1: Create tech debt file**

```markdown
# Tech Debt: Backup/Restore Version Compatibility

**Date:** 2026-01-16
**Priority:** Medium
**Effort:** Medium (2-8h)
**Component:** `src-tauri/src/commands.rs` (backup functions)
**Status:** Open

## Problem

When restoring a backup, there's no check for whether the backup was created by a newer app version. This could lead to:
1. Restoring a backup with unknown schema → app crashes or corrupts data
2. User confusion when restored data doesn't display correctly

## Impact

- Data corruption risk when restoring backups from newer versions
- No warning to user about version mismatch
- Inconsistent with the read-only mode protection for live databases

## Root Cause

The backup/restore feature was implemented before the multi-PC/version compatibility feature. It simply copies the database file without checking migration versions.

## Recommended Solution

1. **Store version metadata in backup**
   - When creating backup, store app version in filename or metadata
   - Format: `kniha-jazd-backup-YYYY-MM-DD-HH-MM-SS-v0.17.0.db`

2. **Check version before restore**
   - Read `__diesel_schema_migrations` from backup file
   - Compare against current app's embedded migrations
   - If backup has unknown migrations, show warning:
     "Táto záloha bola vytvorená novšou verziou (v0.18.0). Obnovenie môže spôsobiť problémy."
   - Options: [Obnoviť napriek tomu] [Zrušiť]

3. **Alternative: Block restore of newer backups**
   - Simpler but more restrictive
   - "Táto záloha vyžaduje aplikáciu v0.18.0 alebo novšiu."

## Alternative Options

1. **Always allow restore with warning** - Less safe but more flexible
2. **Create "downgrade" migrations** - Too complex, not worth it

## Related

- `_tasks/39-custom-db-location/01-design.md` - Original design doc
- `src-tauri/src/db.rs:check_migration_compatibility()` - Version checking logic

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-16 | Created analysis | Identified during custom DB location feature design |
```

**Step 2: Commit**

```bash
git add _tasks/_TECH_DEBT/04-backup-restore-versioning.md
git commit -m "docs(tech-debt): add backup/restore versioning investigation item"
```

---

## Phase 7: Integration Tests

### Task 16: Add Integration Tests for Database Location

**Files:**
- Create: `tests/integration/specs/database-location.spec.ts`

**Step 1: Create test file**

```typescript
import { expect } from '@wdio/globals';
import { setupTestEnvironment, cleanupTestEnvironment } from '../helpers/setup';

describe('Database Location', () => {
    beforeEach(async () => {
        await setupTestEnvironment();
    });

    afterEach(async () => {
        await cleanupTestEnvironment();
    });

    describe('Settings UI', () => {
        it('should display current database path', async () => {
            // Navigate to settings
            await $('[data-testid="nav-settings"]').click();

            // Find database location section
            const section = await $('[data-testid="db-location-section"]');
            await expect(section).toBeDisplayed();

            // Verify path is shown
            const pathText = await $('[data-testid="db-path"]');
            await expect(pathText).toHaveTextContaining('kniha-jazd');
        });

        it('should show default badge for non-custom path', async () => {
            await $('[data-testid="nav-settings"]').click();

            const badge = await $('[data-testid="db-path-badge"]');
            await expect(badge).toHaveTextContaining('Predvolená');
        });
    });

    describe('Read-Only Mode', () => {
        // Note: These tests require mocking migration compatibility
        // which is complex in integration tests. Consider unit tests instead.

        it.skip('should show banner when in read-only mode', async () => {
            // Would need to inject unknown migration into test DB
        });

        it.skip('should disable save buttons in read-only mode', async () => {
            // Would need to inject unknown migration into test DB
        });
    });

    describe('App Mode API', () => {
        it('should return normal mode by default', async () => {
            // This tests the API command directly
            const result = await browser.executeAsync(async (done) => {
                const { invoke } = await import('@tauri-apps/api/core');
                const mode = await invoke('get_app_mode');
                done(mode);
            });

            expect(result.isReadOnly).toBe(false);
        });
    });
});
```

**Step 2: Add to test config if needed**

Ensure the new spec is included in `wdio.conf.ts` specs pattern.

**Step 3: Commit**

```bash
git add tests/integration/specs/database-location.spec.ts
git commit -m "test(integration): add database location tests"
```

---

### Task 17: Run Full Test Suite and Fix Issues

**Step 1: Run backend tests**

```bash
cd src-tauri && cargo test
```

**Step 2: Run integration tests**

```bash
npm run test:integration:build
npm run test:integration:tier1
```

**Step 3: Fix any failing tests**

Address issues as they arise.

**Step 4: Final commit**

```bash
git add -A
git commit -m "fix: address test failures from database location feature"
```

---

### Task 18: Update Changelog

**Step 1: Run changelog skill**

```
/changelog
```

Add entry for the new feature in [Unreleased] section.

**Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: update changelog for custom database location feature"
```

---

## Summary

**Total Tasks:** 18
**Estimated Effort:** 2-3 days

**Key Implementation Order:**
1. Backend foundation (settings, db_location module)
2. Lock file mechanism
3. Migration compatibility check
4. App state management
5. Commands
6. Startup flow refactor
7. Frontend UI
8. Tests and documentation

**Testing Strategy:**
- Unit tests for each Rust module (TDD)
- Integration tests for UI flows
- Manual testing for lock file and multi-PC scenarios
