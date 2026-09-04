# Feature: Backup System

> Database backup and restoration: manual snapshots from Settings, an automatic
> pre-migration snapshot at startup, and a retention policy for inherited pre-update backups.

## User Flow

1. **Manual Backup**: User navigates to Settings → Backup section, clicks "Create Backup"
2. **View Backups**: List shows all backups with date, size, type (manual/pre-update/pre-migration), and version tag
3. **Restore Backup**: Click "Restore" → confirmation dialog shows vehicle/trip counts → confirm → page reloads
4. **Delete Backup**: Click "Delete" → confirmation → backup removed
5. **Retention Settings**: Enable auto-cleanup, select keep count (3/5/10), optionally run cleanup now

There is no "show in Explorer" action and no file picker — the server has no desktop shell,
and its filesystem is not the viewer's ([ADR-030](../../DECISIONS.md)). Backups live in the
`backups/` directory next to the database inside the mounted `/data` volume; an operator
reaches them on the host, not through the UI.

**Read-only mode**: restore, delete and the retention setter are blocked when the app is
read-only. Creating a backup is deliberately *not* blocked — see
[read-only-mode.md](./read-only-mode.md).

## Technical Implementation

### Backup Creation

Three types exist. `BackupType` in [models.rs](../../src-tauri/core/src/models.rs) is the
enum, and the type is recovered from the filename by `parse_backup_filename` rather than
stored anywhere.

**Manual Backup** (`create_backup`):
- Snapshots the live database into the `backups/` directory beside it
- Filename format: `kniha-jazd-backup-YYYY-MM-DD-HHMMSS.db` (no marker)
- Returns `BackupInfo` with live vehicle/trip counts from the current database

**Pre-Migration Backup** (automatic, no command):
- Filename format: `kniha-jazd-backup-YYYY-MM-DD-HHMMSS-pre-migration-v{version}.db`
- Created by `Database::create_pre_migration_backup` in
  [db.rs](../../src-tauri/core/src/db.rs), from `Database::new`, **only** when the database
  file already existed and Diesel reports pending migrations
- A plain file copy is safe there: it is the first connection during startup and no writes
  have happened yet
- A failed snapshot logs a warning and startup continues — it is a net, not a gate

**Pre-Update Backup** (`create_backup_with_type`):
- Filename format: `kniha-jazd-backup-YYYY-MM-DD-HHMMSS-pre-v{version}.db`
- **Nothing creates one any more.** The only caller was the desktop auto-updater's install
  step, deleted with the desktop app. The command and the type survive so that backups made
  by older desktop installs keep their label and stay subject to retention cleanup.

Filename generation (`generate_backup_filename`) and parsing both live in
[commands_internal/backup.rs](../../src-tauri/core/src/commands_internal/backup.rs).

**Snapshot mechanism:** `snapshot_database_to` uses SQLite `VACUUM INTO`, not a file copy.
`VACUUM INTO` produces a transactionally consistent, self-contained database even while the
source connection is in active use, and it replaces an existing target (two backups within
the same second land on the same timestamped path). There is no copy fallback: a failed
snapshot is an error, because a silently inconsistent "backup" is worse than none.

### Backup Listing

`list_backups`:
- Scans the `backups/` directory for `*.db` files
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
- Validates the filename (`validate_backup_filename`), then copies backup → current database
- Blocked in read-only mode
- Frontend triggers a page reload after success, so every view re-reads the restored database

The server refuses to start if the file it opens is not the one restore would write to.
`verify_db_path_consistency` in
[db_location.rs](../../src-tauri/core/src/db_location.rs) compares `DATABASE_PATH` against
the path resolved from the data dir plus any `custom_db_path`. Without that check a restore
could report success while the running instance kept serving the old database.

### Retention & Cleanup

**Settings**: Retention configuration is stored in `local.settings.json`. The
`BackupRetention` struct in [settings.rs](../../src-tauri/core/src/settings.rs) holds an
`enabled` flag and `keep_count` (3, 5, or 10).

**Cleanup Logic** (`get_cleanup_candidates`):
- Filters to **pre-update** backups only — manual *and* pre-migration backups are never deleted
- Sorts by filename (oldest first)
- Returns oldest backups beyond the keep limit

Because nothing creates pre-update backups any more, cleanup now applies **only to backups
inherited from an old desktop install**. On a container that never held a desktop database
it will always find nothing to delete. The code and its tests remain correct; the input set
is simply empty for new deployments.

**No startup auto-cleanup**: cleanup runs only when the user triggers it from Settings.
The background thread that once ran it at startup lived in the deleted desktop entry point;
[core/src/lib.rs](../../src-tauri/core/src/lib.rs) is now just module declarations, and
[web/src/main.rs](../../src-tauri/web/src/main.rs) starts the HTTP server without it.

**Manual Cleanup**:
- `get_cleanup_preview` shows which backups would be deleted and the total bytes
- "Clean Now" calls `cleanup_pre_update_backups` for immediate cleanup

## Data Structures

Core backup types are defined in:
- **Rust**: `BackupInfo`, `CleanupPreview` and `CleanupResult` in
  [commands_internal/backup.rs](../../src-tauri/core/src/commands_internal/backup.rs);
  the `BackupType` enum in [models.rs](../../src-tauri/core/src/models.rs)
- **TypeScript**: `BackupType`, `BackupInfo`, `CleanupPreview`, `CleanupResult` in
  [types.ts](../../src/lib/types.ts)

Key fields in `BackupInfo`:
- `filename` - Full filename with extension
- `createdAt` - ISO timestamp parsed from filename
- `sizeBytes` - File size on disk
- `vehicleCount` / `tripCount` - 0 in list view, actual counts loaded via `get_backup_info`
- `backupType` - `'manual'`, `'pre-update'` or `'pre-migration'` (parsed from filename)
- `updateVersion` - Version string for pre-update and pre-migration backups (e.g., "0.20.0")

## Key Files

| File | Purpose |
|------|---------|
| [commands_internal/backup.rs](../../src-tauri/core/src/commands_internal/backup.rs) | Backup commands (create, list, restore, delete, cleanup) + `validate_backup_filename` |
| [settings.rs](../../src-tauri/core/src/settings.rs) | `BackupRetention` struct and JSON persistence |
| [db.rs](../../src-tauri/core/src/db.rs) | `create_pre_migration_backup` — the automatic startup snapshot |
| [models.rs](../../src-tauri/core/src/models.rs) | `BackupType` enum (`manual` / `pre-update` / `pre-migration`) |
| [api.ts](../../src/lib/api.ts) | Frontend API functions for backup operations |
| [types.ts](../../src/lib/types.ts) | TypeScript interfaces for backup data |
| [+page.svelte](../../src/routes/settings/+page.svelte) | Backup UI (list, create, restore, retention settings) |

## API Functions

| Function | Description |
|----------|-------------|
| `createBackup()` | Create manual backup |
| `createBackupWithType(type, version)` | Create typed backup — retained for the `pre-update` type; nothing calls it now |
| `listBackups()` | Get all backups (lightweight) |
| `getBackupInfo(filename)` | Get backup with actual counts |
| `restoreBackup(filename)` | Restore database from backup |
| `deleteBackup(filename)` | Delete a backup file |
| `getBackupRetention()` | Get retention settings |
| `setBackupRetention(settings)` | Save retention settings |
| `getCleanupPreview(keepCount)` | Preview what would be deleted |
| `cleanupPreUpdateBackups(keepCount)` | Execute cleanup |

## Design Decisions

1. **Filename-encoded metadata**: Type and version stored in filename, not separate metadata file
   - Enables simple file-based backup management
   - Backups remain self-contained and portable

2. **Manual backups never auto-deleted**: Only pre-update backups are subject to retention cleanup
   - Manual backups are intentional user actions
   - Prevents accidental data loss
   - Pre-migration backups are excluded too — they exist precisely for the upgrade that
     might need undoing

3. **Pre-migration snapshot is automatic and unconditional**: it costs one file copy on the
   startups that actually migrate, and it is the only thing standing between a buggy
   migration and an unrecoverable legal-compliance record

4. **Lazy count loading**: List returns 0 for counts, actual query only on restore confirmation
   - Fast list loading (no need to open each backup database)
   - Full info shown when user needs to make restore decision

5. **Backup location follows database**: resolved from `KNIHA_JAZD_DATA_DIR` (`/data` in the
   container) plus any `custom_db_path`
   - Backups always sit in `backups/` beside the database file
   - The mounted volume carries both across container restarts and image upgrades

6. **Three retention options**: 3, 5, or 10 backups
   - Simple choices that cover common needs
   - Prevents configuration paralysis

7. **Retention only ever deletes pre-update backups**: a manual backup is an explicit
   user act, so it is never reclaimed automatically.
