# Custom Database Location with Multi-PC Support

## Overview

Allow users to store the database on Google Drive, NAS, or any custom folder to enable multi-PC usage. The app handles version mismatches gracefully (read-only mode) and prevents concurrent access conflicts via lock files.

## User Stories

1. As a user, I want to store my database on Google Drive so I can access my logbook from multiple PCs
2. As a user, I want to be warned if I open the app on a second PC while it's already open elsewhere
3. As a user, I want to still view my data if I'm running an older app version than the database requires

## Configuration Storage

Custom DB path stored in `local.settings.json` (each PC keeps its own pointer):

```json
{
  "theme": "dark",
  "gemini_api_key": "...",
  "custom_db_path": "G:\\GoogleDrive\\kniha-jazd\\",
  "receipts_folder_path": null
}
```

**When `custom_db_path` is set:**
- Database file: `{custom_db_path}/kniha-jazd.db`
- Lock file: `{custom_db_path}/kniha-jazd.db.lock`
- Backups folder: `{custom_db_path}/backups/`

**When `custom_db_path` is `null` (default):**
- Uses standard AppData location (`%APPDATA%\com.notavailable.kniha-jazd\`)

## Startup Flow

```
1. Load local.settings.json
   ↓
2. Is custom_db_path set?
   ├─ No → Use default AppData location, continue to step 5
   └─ Yes → Continue to step 3
   ↓
3. Is custom path accessible?
   ├─ No → Show dialog: "Cannot access database at {path}"
   │        Options: [Retry] [Use Local DB] [Change Path in Settings]
   │        Block until user chooses
   └─ Yes → Continue to step 4
   ↓
4. Check lock file (kniha-jazd.db.lock)
   ├─ Exists & fresh (< 5 min) → Show warning:
   │   "Database appears to be open on {PC-NAME} since {time}.
   │    Opening anyway may cause issues."
   │    Options: [Open Anyway] [Cancel]
   └─ No lock / stale → Continue
   ↓
5. Check DB version (Diesel migrations)
   ├─ DB has unknown migrations → READ-ONLY MODE
   │   Show banner: "This database was updated by a newer app version.
   │                 Running in read-only mode. Please update the app."
   │                 [Check for Updates] button
   └─ DB is same or older → Run pending migrations, continue normally
   ↓
6. Create/update lock file with current PC name + timestamp
   ↓
7. App ready
```

## Version Detection (Diesel Migrations)

Uses Diesel's built-in `__diesel_schema_migrations` table:

```rust
// At app startup, before running migrations:
1. Read all migration versions from DB's __diesel_schema_migrations
2. Compare against migrations embedded in THIS app version
3. If DB has migrations the app doesn't know about → newer DB!

// Example:
App v0.15.0 knows migrations: [baseline, vehicle_meta]
DB has migrations: [baseline, vehicle_meta, receipt_cost]  // Added in v0.17.0

→ DB has "receipt_cost" which app doesn't recognize
→ Trigger read-only mode + update warning
```

## Read-Only Mode

**When Triggered:**
- DB contains migrations unknown to the current app version

**UI Behavior:**

1. **Persistent banner** at top of app (yellow/warning color):
   ```
   ⚠️ Databáza bola aktualizovaná novšou verziou aplikácie.
      Režim len na čítanie. [Skontrolovať aktualizácie]
   ```

2. **Disabled actions:**
   - All "Save" / "Add" / "Delete" buttons greyed out
   - Trip grid: no inline editing
   - Vehicle management: view only
   - Settings: view only (except DB path change)
   - Receipts: can view, cannot import new

3. **Still functional:**
   - Viewing all data (trips, vehicles, reports)
   - Exporting reports (PDF/HTML)
   - Viewing receipts already imported

**Implementation:**
- Add `is_read_only: bool` to app state (Tauri managed state)
- Frontend reads via `get_app_mode()` command
- Components check flag to disable mutations
- All write commands return error if read-only

## Lock File Mechanism

**Lock file: `kniha-jazd.db.lock`**

```json
{
  "pc_name": "PC-WORK",
  "opened_at": "2026-01-16T10:30:00Z",
  "last_heartbeat": "2026-01-16T10:32:00Z",
  "app_version": "0.17.0",
  "pid": 12345
}
```

**Lifecycle:**
- Created on app start
- Heartbeat updated every 2 minutes
- Deleted on clean app exit

**Stale detection:**
- Lock older than 5 minutes = stale (app crashed)
- Show warning but allow taking over

## Settings UI

New "Database Location" section in Settings page:

```
┌─────────────────────────────────────────────────────────────┐
│ Umiestnenie databázy                                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│ Aktuálne umiestnenie:                                       │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ C:\Users\Dodo\AppData\Roaming\com.notavailable...\     │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
│ [Zmeniť umiestnenie...]    [Otvoriť priečinok]             │
│                                                             │
│ ℹ️ Databázu môžete presunúť na Google Drive, NAS alebo     │
│   iný zdieľaný priečinok pre použitie na viacerých PC.     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Move Database Flow:**
1. Native folder picker opens
2. User selects target folder
3. Confirmation dialog shows what will be moved (DB + backups)
4. On confirm:
   - Copy DB file to new location
   - Copy backups folder
   - Verify integrity
   - Update `local.settings.json`
   - Delete old files
5. Success message, app continues with new location

**Edge case - target has existing DB:**
- Dialog: "Cieľový priečinok už obsahuje databázu."
- Options: [Použiť existujúcu] [Nahradiť mojou] [Zrušiť]

## Files to Modify

**New Files:**
- `src-tauri/src/db_location.rs` - Lock file logic, path resolution, version checking

**Modified Files:**

| File | Changes |
|------|---------|
| `src-tauri/src/lib.rs` | New startup flow with path/lock/version checks |
| `src-tauri/src/settings.rs` | Add `custom_db_path` to `LocalSettings` |
| `src-tauri/src/commands.rs` | New commands: `get_db_location`, `set_db_location`, `move_database`, `get_app_mode` |
| `src-tauri/src/db.rs` | Add `check_migration_compatibility()` function |
| `src/lib/components/Settings.svelte` | New "Database Location" section |
| `src/lib/i18n/sk/index.ts` | Slovak translations for new UI |
| `src/lib/i18n/en/index.ts` | English translations |
| `CLAUDE.md` | Add migration best practices rule |
| `_TECH_DEBT.md` | Add backup/restore versioning investigation item |

## Integration Tests

| Test | Scenario |
|------|----------|
| `custom_db_path_setting` | Set path in settings, verify DB moves correctly |
| `unavailable_path_dialog` | Mock unavailable path, verify dialog appears |
| `lock_file_creation` | Verify lock created on start, deleted on exit |
| `lock_file_stale_detection` | Create old lock, verify stale warning |
| `lock_file_active_warning` | Create fresh lock, verify "open on other PC" warning |
| `read_only_mode_activation` | Add unknown migration to DB, verify read-only mode |
| `read_only_mode_ui` | In read-only mode, verify save buttons disabled |
| `version_upgrade_migration` | Open older DB, verify migrations run |
| `existing_db_in_target` | Move to folder with existing DB, verify options shown |

## Tech Debt

**To create in `_TECH_DEBT.md`:**
- Investigate backup/restore behavior with DB version mismatch
  - What happens when restoring backup from newer app version?
  - Should restore check migration compatibility?
  - Consider showing warning if backup is from newer version

## Implementation Notes

**IMPORTANT: Migration Best Practices**

All database migrations MUST be non-destructive and backward compatible:
- Always add columns with DEFAULT values
- Never remove columns (mark as deprecated if needed)
- Never rename columns
- Older app versions must be able to READ newer schemas (just not write)

This ensures read-only mode actually works - older apps can still display data.

## Decision Log

- **Config storage:** `local.settings.json` - allows each PC to have different path to same DB
- **Version detection:** Diesel migrations table - zero maintenance, automatic
- **Mismatch behavior:** Read-only mode + update warning - user can still access data
- **Concurrent access:** Lock file with heartbeat - clear "open on PC-X" message
- **Unavailable path:** Block with dialog - prevents accidental work in wrong DB
