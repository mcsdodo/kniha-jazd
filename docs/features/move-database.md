# Feature: Move Database

> Allows users to relocate the database to a custom path (e.g., Google Drive, NAS) for multi-PC access with automatic lock file management.

## User Flow

1. **Navigate** to Settings → Database Location
2. **Click "Change..."** button
3. **Select target folder** via directory picker
4. **Confirm** in modal dialog (shows target path + warning)
5. **Wait** for move operation (indeterminate progress shown)
6. **App reloads** automatically with new database location

**Reset to default**: When a custom path is active, a “Reset to default” action is available and triggers the same move flow back to the default app data directory.

**Failure cases**:
- Moving to the same folder is rejected by the backend.
- Target folder already contains a database (blocked in the frontend pre-check).

## Technical Implementation

### Frontend

**Settings Page:** `src/routes/settings/+page.svelte`
- `handleChangeDbLocation()` — Opens directory picker, validates target
- `handleConfirmMove()` — Calls backend, handles success/error, triggers reload
- `checkTargetHasDb()` — Pre-validation to prevent overwriting existing database

**Confirmation Modal:** `src/lib/components/MoveDatabaseModal.svelte`
- Shows target path and warning message
- Progress indicator during move operation

**API Wrapper:** `src/lib/api.ts`
- `moveDatabase()` - Invokes backend move command
- `resetDatabaseLocation()` - Resets to default path

### Backend (Rust)

**Main Command:** `move_database` in `commands.rs:L3276-3355`
- Validates permissions, copies files + backups, updates settings, manages locks, cleans up source

**Lock File Module:** `src-tauri/src/db_location.rs`
- `acquire_lock()` — Creates lock file with PC name, timestamp, PID
- `release_lock()` — Removes lock file
- `check_lock()` — Returns `Free`, `Stale`, or `Locked`

**Helper Functions:** `commands.rs:L3433-3452`
- `copy_dir_all()` — Recursive directory copy
- `count_files()` — Count files in directory for progress reporting

### Data Flow

```
User clicks "Change..." → Directory Picker → Frontend validation
        ↓
Confirmation Modal → invoke("move_database")
        ↓
Rust Backend: validate → copy files → update settings → manage locks → cleanup
        ↓
MoveDbResult → Frontend toast → App reload
```

For implementation details, see `commands.rs:L3276-3355`.

## Key Files

| File | Purpose |
|------|---------|
| `src/routes/settings/+page.svelte` | Settings UI with move button |
| `src/lib/components/MoveDatabaseModal.svelte` | Confirmation dialog |
| `src/lib/api.ts` | TypeScript API wrappers |
| `src-tauri/src/commands.rs:L3276-3355` | `move_database` command |
| `src-tauri/src/commands.rs:L3359-3430` | `reset_database_location` command |
| `src-tauri/src/db_location.rs` | Lock file management |
| `src-tauri/src/settings.rs` | `LocalSettings` struct, custom path storage |

## Lock File Structure

Located at `<db_folder>/kniha-jazd.lock`. See `db_location.rs:L36-47` for `LockFile` struct.

```json
{
  "pc_name": "DESKTOP-ABC123",
  "opened_at": "2024-01-15T10:30:00Z",
  "last_heartbeat": "2024-01-15T10:35:00Z",
  "app_version": "1.2.0",
  "pid": 12345
}
```

**Staleness:** Lock considered stale if `last_heartbeat` > 2 minutes old.

**Heartbeat:** Lock is refreshed every 30 seconds while the app is running.

## Design Decisions

- **Why copy-then-delete?** — Prevents data loss if move fails mid-operation. Only delete source after successful copy.

- **Why app reload instead of reconnection?** — Database connection established at startup. Clean reload simpler and safer than complex reconnection logic across all modules.

- **Why lock files?** — Enables multi-PC support for shared storage (Google Drive, NAS). Prevents simultaneous writes that could corrupt database.

- **Why store path in local.settings.json?** — Survives app reinstalls. Located in AppData, not alongside database.

## Related

- **Read-only mode:** Triggered by unknown migrations, not by lock conflicts (lock conflicts log a warning and continue)
- **Backup system:** Backups folder moves with database
- **Settings persistence:** `local.settings.json` in `%APPDATA%\com.notavailable.kniha-jazd\`
