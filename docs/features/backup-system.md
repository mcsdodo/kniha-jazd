# Feature: Backup System

> Complete database backup and restoration with automatic pre-update backups and configurable retention policies.

## User Flow

1. **Manual Backup**: User navigates to Settings → Backup section, clicks "Create Backup"
2. **View Backups**: List shows all backups with date, size, type (manual/pre-update), and version tag
3. **Restore Backup**: Click "Restore" → confirmation dialog shows vehicle/trip counts → confirm → app reloads
4. **Delete Backup**: Click "Delete" → confirmation → backup removed
5. **Reveal Backup**: Click "Show in Explorer/Finder" to open backup folder
6. **Retention Settings**: Enable auto-cleanup, select keep count (3/5/10), optionally run cleanup now
7. **Pre-Update Backup**: Automatic backup before update installation (triggered from update modal)

**Read-only mode**: create/restore/delete/cleanup actions are blocked when the app is read-only.

## Technical Implementation

### Backup Creation

**Manual Backup** (`create_backup`):
- Copies current database to `{app_data_dir}/backups/` folder
- Filename format: `kniha-jazd-backup-YYYY-MM-DD-HHMMSS.db`
- Returns `BackupInfo` with live vehicle/trip counts from current database

**Pre-Update Backup** (`create_backup_with_type`):
- Called from update store before downloading update
- Filename format: `kniha-jazd-backup-YYYY-MM-DD-HHMMSS-pre-v{version}.db`
- Version encoded in filename for identification

```rust
// Filename generation
fn generate_backup_filename(backup_type: &str, update_version: Option<&str>) -> String {
    let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
    match (backup_type, update_version) {
        ("pre-update", Some(version)) => format!("kniha-jazd-backup-{}-pre-v{}.db", timestamp, version),
        _ => format!("kniha-jazd-backup-{}.db", timestamp),
    }
}
```

### Backup Listing

`list_backups`:
- Scans `{backups_dir}/*.db` files
- Parses filename to extract timestamp and type
- Pre-update filenames keep the `-pre-vX` suffix and may fall back to a “now” timestamp in the list
- Returns lightweight `BackupInfo` (counts are 0 for performance)
- Sorted by filename descending (newest first)

`get_backup_info`:
- Opens backup database file with Diesel
- Queries actual vehicle/trip counts via SQL
- Used when user clicks "Restore" to show confirmation details

### Backup Restoration

`restore_backup`:
- Simple file copy: backup → current database
- Frontend triggers page reload after success
- App re-initializes with restored data

### Pre-Update Backups

Triggered from [update.ts](src/lib/stores/update.ts) during update flow:

```typescript
// In update store install() method
const { createBackupWithType } = await import('$lib/api');
await createBackupWithType('pre-update', updateObject.version);
```

Backup step states: `pending` → `in-progress` → `done` | `failed` | `skipped`

If backup fails, user can choose to "Continue Without Backup" or cancel.

### Retention & Cleanup

**Settings** (stored in `local.settings.json`, optional):
```typescript
interface BackupRetention {
  enabled: boolean;
  keepCount: number; // 3, 5, or 10
}
```

**Cleanup Logic** (`get_cleanup_candidates`):
- Filters to **pre-update** backups only (manual backups never deleted)
- Sorts by filename (oldest first)
- Returns oldest backups beyond keep limit

**Startup Auto-Cleanup** (in `lib.rs`):
```rust
// Runs in background thread at app startup
if retention.enabled && retention.keep_count > 0 {
    commands::cleanup_pre_update_backups_internal(&cleanup_app_handle, retention.keep_count);
}
```

**Manual Cleanup**:
- Preview shows which backups will be deleted and total bytes
- "Clean Now" button triggers immediate cleanup

## Data Structures

```typescript
type BackupType = 'manual' | 'pre-update';

interface BackupInfo {
  filename: string;           // Full filename with extension
  createdAt: string;          // ISO timestamp parsed from filename
  sizeBytes: number;          // File size on disk
  vehicleCount: number;       // 0 in list, actual in get_backup_info
  tripCount: number;          // 0 in list, actual in get_backup_info
  backupType: BackupType;     // Parsed from filename
  updateVersion: string | null; // e.g., "0.20.0" for pre-update
}

interface CleanupPreview {
  toDelete: BackupInfo[];
  totalBytes: number;
}

interface CleanupResult {
  deleted: string[];          // Filenames that were deleted
  freedBytes: number;
}
```

## Key Files

| File | Purpose |
|------|---------|
| [commands.rs](src-tauri/src/commands.rs) | Backend backup commands (create, list, restore, delete, cleanup) |
| [settings.rs](src-tauri/src/settings.rs) | `BackupRetention` struct and JSON persistence |
| [lib.rs](src-tauri/src/lib.rs) | Post-update cleanup trigger at startup |
| [api.ts](src/lib/api.ts) | Frontend API functions for backup operations |
| [types.ts](src/lib/types.ts) | TypeScript interfaces for backup data |
| [update.ts](src/lib/stores/update.ts) | Pre-update backup trigger during update flow |
| [+page.svelte](src/routes/settings/+page.svelte) | Backup UI (list, create, restore, retention settings) |

## API Functions

| Function | Description |
|----------|-------------|
| `createBackup()` | Create manual backup |
| `createBackupWithType(type, version)` | Create typed backup (pre-update) |
| `listBackups()` | Get all backups (lightweight) |
| `getBackupInfo(filename)` | Get backup with actual counts |
| `restoreBackup(filename)` | Restore database from backup |
| `deleteBackup(filename)` | Delete a backup file |
| `revealBackup(filename)` | Open backup in file explorer |
| `getBackupRetention()` | Get retention settings |
| `setBackupRetention(settings)` | Save retention settings |
| `getCleanupPreview(keepCount)` | Preview what would be deleted |
| `cleanupPreUpdateBackups(keepCount)` | Execute cleanup |

## Design Decisions

1. **Filename-encoded metadata**: Type and version stored in filename, not separate metadata file
   - Enables simple file-based backup management
   - Backups remain self-contained and portable

2. **Manual backups never auto-deleted**: Only pre-update backups subject to retention cleanup
   - Manual backups are intentional user actions
   - Prevents accidental data loss

3. **Startup cleanup**: Runs on every app startup when retention is enabled
   - No UI interruption
   - Logged for debugging

4. **Lazy count loading**: List returns 0 for counts, actual query only on restore confirmation
   - Fast list loading (no need to open each backup database)
   - Full info shown when user needs to make restore decision

5. **Backup location follows database**: Uses app data dir and respects `KNIHA_JAZD_DATA_DIR` overrides
   - Backups stored alongside database (e.g., on Google Drive, NAS)
   - Consistent data locality

6. **Three retention options**: 3, 5, or 10 backups
   - Simple choices that cover common needs
   - Prevents configuration paralysis

7. **Pre-update backup in update flow**: Backup created before download starts
   - Captures database state at known-good version
   - User can skip if backup fails (with warning)
