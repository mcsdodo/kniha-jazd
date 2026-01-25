# Feature: Read-Only Mode

> Protects database integrity when the app detects it cannot safely write to the database (newer migrations or another PC holding the lock).

## User Flow

1. **App Launch**: During startup, the app checks two conditions before enabling write access:
    - **Lock file status**: Is the database locked by another instance? (informational only)
    - **Migration compatibility**: Are there unknown migrations in the database?

2. **Normal Mode**: If migration compatibility passes, the app acquires a lock and operates normally with full read/write access, even if a fresh lock was detected (warning logged).

3. **Read-Only Mode Triggered**: If migration compatibility fails:
    - Write operations are blocked with a user-friendly Slovak error message
    - The reason is stored (used in command errors)
    - Read operations continue to work normally
    - UI shows a static banner with a “check updates” button

4. **Error Message Format**: When a write operation is attempted in read-only mode:
   ```
   Aplikácia je v režime len na čítanie. [Reason]
   ```
    Where `[Reason]` explains why (e.g., "Databáza bola aktualizovaná novšou verziou aplikácie.").
    If no reason is stored, it falls back to `Neznámy dôvod`.

## Technical Implementation

### Lock File Mechanism

The lock file (`kniha-jazd.lock`) prevents concurrent write access from multiple PCs sharing a database via cloud storage (Google Drive, NAS, etc.).

**Location**: Same directory as the database file
- Default: `%APPDATA%/com.notavailable.kniha-jazd/kniha-jazd.lock`
- Dev mode: `%APPDATA%/com.notavailable.kniha-jazd.dev/kniha-jazd.lock`
- Custom: `<custom_db_path>/kniha-jazd.lock`

**Lock file format** (JSON):
```json
{
  "pc_name": "DESKTOP-ABC123",
  "opened_at": "2025-01-25T10:00:00Z",
  "last_heartbeat": "2025-01-25T10:05:30Z",
  "app_version": "0.21.0",
  "pid": 12345
}
```

**Lock Status Resolution** (in `db_location.rs`):

| Condition | Status | Action |
|-----------|--------|--------|
| No lock file exists | `Free` | Acquire lock, normal mode |
| Lock file exists, heartbeat > 2 minutes old | `Stale` | Take over lock, normal mode |
| Lock file exists, heartbeat fresh | `Locked` | Log warning; app still attempts to acquire lock |
| Lock file corrupted/unreadable | `Free` | Acquire lock, normal mode |

**Staleness Threshold**: 120 seconds (2 minutes)

### Heartbeat Thread

When the app successfully acquires a lock, it spawns a background thread to keep the lock fresh:

```rust
std::thread::spawn(move || {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(30));
        if let Err(e) = db_location::refresh_lock(&heartbeat_lock_path) {
            log::warn!("Failed to refresh lock: {}", e);
            break;
        }
    }
});
```

**Interval**: 30 seconds  
**Purpose**: Updates `last_heartbeat` timestamp to prove the app is still running

If the heartbeat fails (e.g., lock file deleted externally), the thread exits gracefully.

### Migration Compatibility

The app checks if the database contains migrations that this version doesn't recognize:

```rust
pub fn check_migration_compatibility(&self) -> Result<(), Vec<String>> {
    let embedded = Self::get_embedded_migration_versions();  // Migrations compiled into this app
    let applied = /* query __diesel_schema_migrations table */;
    
    let unknown: Vec<String> = applied
        .filter(|m| !embedded.contains(&m.version))
        .collect();

    if unknown.is_empty() { Ok(()) } else { Err(unknown) }
}
```

**Scenario**: User A runs v0.22.0 which adds a new migration. User B opens the same database with v0.21.0 → Read-only mode activates because v0.21.0 doesn't know about that migration.

**Read-only reason**: `"Databáza bola aktualizovaná novšou verziou aplikácie."`

### Protected Commands

All write operations are guarded by the `check_read_only!` macro. When invoked in read-only mode, these commands return an error immediately.

**Vehicle Commands:**
- `create_vehicle` - Create new vehicle
- `update_vehicle` - Update vehicle details
- `delete_vehicle` - Delete vehicle (currently not guarded)
- `set_active_vehicle` - Change active vehicle

**Trip Commands:**
- `create_trip` - Add new trip
- `update_trip` - Modify trip data
- `delete_trip` - Remove trip
- `reorder_trip` - Change trip sort order

**Settings Commands:**
- `save_settings` - Save app settings

**Backup Commands:**
- `create_backup` - Create manual backup
- `create_backup_with_type` - Create typed backup (pre-update)
- `cleanup_pre_update_backups` - Delete old pre-update backups
- `set_backup_retention` - Configure retention policy
- `restore_backup` - Restore from backup
- `delete_backup` - Remove backup file

**Receipt Commands:**
- `scan_receipts` - Scan for new receipt images
- `sync_receipts` - Process receipts with OCR
- `update_receipt` - Update receipt data
- `delete_receipt` - Remove receipt
- `reprocess_receipt` - Re-run OCR on receipt
- `assign_receipt_to_trip` - Link receipt to trip

## Key Files

| File | Purpose |
|------|---------|
| [src-tauri/src/app_state.rs](../../src-tauri/src/app_state.rs) | `AppMode` enum, `AppState` struct with RwLock-protected fields |
| [src-tauri/src/db_location.rs](../../src-tauri/src/db_location.rs) | Lock file operations: `check_lock`, `acquire_lock`, `refresh_lock`, `release_lock` |
| [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs) | Startup initialization, lock acquisition, heartbeat thread spawn |
| [src-tauri/src/commands.rs](../../src-tauri/src/commands.rs) | `check_read_only!` macro, protected command implementations |
| [src-tauri/src/db.rs](../../src-tauri/src/db.rs) | `check_migration_compatibility()` function |
| [src/lib/stores/app.ts](../../src/lib/stores/app.ts) | Read-only state and reason on the frontend |
| [src/routes/+layout.svelte](../../src/routes/+layout.svelte) | Read-only banner UI and update check action |
| [src/lib/api.ts](../../src/lib/api.ts) | `check_updates` command wrapper used by the banner |

## Design Decisions

### Why Lock Files Instead of Database Locks?

SQLite's built-in locking doesn't work reliably over network filesystems (Google Drive, OneDrive, NAS). A separate lock file with periodic heartbeats provides more reliable cross-PC coordination.

### Why 2 Minutes Staleness Threshold?

- **Too short** (< 1 min): Risk of false stale detection during brief network hiccups
- **Too long** (> 5 min): User waits too long after a crash before regaining access
- **2 minutes**: Balanced - 4x the heartbeat interval provides buffer for transient issues

### Why 30 Second Heartbeat?

- Frequent enough to detect crashed instances within 2-3 minutes
- Infrequent enough to minimize disk I/O on shared storage
- Standard "lease renewal" pattern (refresh at ~1/4 of expiry time)

### Why Block All Writes on Unknown Migrations?

Allowing writes with unknown migrations could:
1. Corrupt data if new migration changed column semantics
2. Cause crashes in the newer app version expecting certain data formats
3. Make rollback impossible if older version modifies incompatible data

Read-only mode is a safe fallback - user can still view data and export reports.

### Error Message in Slovak

The app is primarily for Slovak users. Error messages use Slovak to match the UI language.
