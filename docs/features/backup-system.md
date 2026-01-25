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
- See `commands.rs:L1597-1602` for filename generation logic

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

Triggered from update store (`update.ts`) during the install flow. The store imports `createBackupWithType` from the API module and calls it with type `'pre-update'` and the target version before downloading the update.

Backup step states: `pending` → `in-progress` → `done` | `failed` | `skipped`

If backup fails, user can choose to "Continue Without Backup" or cancel.

### Retention & Cleanup

**Settings**: Retention configuration stored in `local.settings.json`. See `settings.rs:L11-14` for `BackupRetention` struct with `enabled` flag and `keep_count` (3, 5, or 10).

**Cleanup Logic** (`get_cleanup_candidates`):
- Filters to **pre-update** backups only (manual backups never deleted)
- Sorts by filename (oldest first)
- Returns oldest backups beyond keep limit

**Startup Auto-Cleanup**: Runs in background thread at app startup when retention is enabled. See `lib.rs:L140-146`.

**Manual Cleanup**:
- Preview shows which backups will be deleted and total bytes
- "Clean Now" button triggers immediate cleanup

## Data Structures

Core backup types are defined in:
- **Rust**: `commands.rs:L74-82` (`BackupInfo` struct)
- **TypeScript**: `types.ts:L92+` (`BackupType`, `BackupInfo`, `CleanupPreview`, `CleanupResult`)

Key fields in `BackupInfo`:
- `filename` - Full filename with extension
- `createdAt` - ISO timestamp parsed from filename
- `sizeBytes` - File size on disk
- `vehicleCount` / `tripCount` - 0 in list view, actual counts loaded via `get_backup_info`
- `backupType` - `'manual'` or `'pre-update'` (parsed from filename)
- `updateVersion` - Version string for pre-update backups (e.g., "0.20.0")

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
