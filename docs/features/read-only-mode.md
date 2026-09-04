# Feature: Read-Only Mode

> A global switch that blocks every write command while leaving reads working, so the app
> can keep serving a database it must not modify.

## Current status: armed by the migration-compatibility check

**The startup migration-compatibility check arms it.**
[web/src/main.rs](../../src-tauri/web/src/main.rs) calls
`Database::check_migration_compatibility()` after opening the database; if the file
carries migrations this build does not know — it was written by a newer image — the
server prints the unknown versions and calls `enable_read_only`. With versioned ghcr
tags, rolling back to an older tag is the realistic way to reach this, and it is more
likely than anything the desktop app offered.

Verified end to end: with a planted future migration, `/api/capabilities` reports
`"read_only": true`, `get_app_mode` returns `ReadOnly` with the Slovak reason, and a
write command is refused.

One earlier trigger is gone for good ([ADR-030](../../DECISIONS.md)):

- **Lock files and the heartbeat thread** — the multi-PC coordination described in earlier
  versions of this doc. [db_location.rs](../../src-tauri/core/src/db_location.rs) no longer
  contains any lock code, there is no `kniha-jazd.lock`, and no heartbeat thread exists
  anywhere in the workspace. One container owns one SQLite file; there is no second writer
  to coordinate with.

Note the check runs at **startup only**. A database replaced underneath a running
container is not re-checked until restart.

## User Flow

1. **Write blocked**: any write command returns a Slovak error instead of acting
2. **Reads unaffected**: the grid, exports and backup listing keep working
3. **Banner**: the layout renders a static warning strip above the page content

**Error message format**, produced by the `check_read_only!` macro:

```
Aplikácia je v režime len na čítanie. [Reason]
```

`[Reason]` is whatever string was passed to `AppState::enable_read_only()`. If no reason is
stored it falls back to `Neznámy dôvod`.

## Technical Implementation

### App State

[app_state.rs](../../src-tauri/core/src/app_state.rs) holds the mode behind an `RwLock`:

- `AppMode` — `Normal` | `ReadOnly`, defaulting to `Normal`
- `AppState::is_read_only()` — what every guard and the capabilities endpoint read
- `AppState::enable_read_only(reason)` — sets the mode and stores the reason together
- `AppState::get_read_only_reason()` — the string the error message interpolates

`AppState` also carries the database path and the PIN-reveal throttle; those are unrelated
to this feature.

### The Guard Macro

`check_read_only!` is defined in
[commands_internal/helpers.rs](../../src-tauri/core/src/commands_internal/helpers.rs) and is
exported at the crate root (`#[macro_export]`). It takes an `&AppState`, and returns early
with the formatted Slovak error when the mode is `ReadOnly`.

The convention is one line at the top of every `*_internal` function that mutates state —
see [rust-backend.md](../../.claude/rules/rust-backend.md), step 3 of "Adding a New Command".

### Guarded Commands

Every write path carries the guard. Grouped by module under
[commands_internal/](../../src-tauri/core/src/commands_internal/):

| Module | Guarded `*_internal` functions |
|--------|-------------------------------|
| [vehicles.rs](../../src-tauri/core/src/commands_internal/vehicles.rs) | `update_vehicle`, `delete_vehicle`, `set_active_vehicle` |
| [trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) | `create_trip`, `update_trip`, `delete_trip` |
| [settings_cmd.rs](../../src-tauri/core/src/commands_internal/settings_cmd.rs) | `save_settings` |
| [backup.rs](../../src-tauri/core/src/commands_internal/backup.rs) | `set_backup_retention`, `restore_backup`, `delete_backup` |
| [receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs) | `set_gemini_api_key`, `set_receipts_folder_path`, `update_receipt`, `delete_receipt`, `unassign_receipt`, `revert_receipt_override`, `scan_receipts`, `sync_receipts`, `reprocess_receipt` |
| [invoices.rs](../../src-tauri/core/src/commands_internal/invoices.rs) | `assign_invoice_to_trip`, `unassign_invoice` |
| [integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs) | `save_ha_settings`, `save_paperless_settings` |
| [route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs) | `save_trip_route`, `delete_trip_route` |

Two deliberate exceptions in [backup.rs](../../src-tauri/core/src/commands_internal/backup.rs), both marked with a `NOTE:` comment in the source:
`create_backup_internal` and `create_backup_with_type_internal` carry **no** guard. Creating
a backup only reads the database and writes a new file, so it must keep working precisely
when the database is protected. Restore and delete stay gated.

`cleanup_pre_update_backups_internal` is likewise unguarded — it deletes only backup files,
never the database.

### Frontend

**Store:** [appMode.ts](../../src/lib/stores/appMode.ts) — `appModeStore` calls
`get_app_mode` and exposes `{ mode, isReadOnly, readOnlyReason }`. A failed call logs and
leaves the permissive default in place.

**Banner:** [+layout.svelte](../../src/routes/+layout.svelte) renders, when
`$appModeStore.isReadOnly` is true, a static strip containing a warning icon and the
`settings.readOnlyBanner` translation. It has **no buttons** — there is nothing to click,
because there is no updater to invoke and no lock to release.

The Slovak text is *"Databáza bola aktualizovaná novšou verziou aplikácie. Režim len na
čítanie."*, which presumes the migration-compatibility trigger described above. If the
mode is ever armed for a different reason, that string needs revisiting.

### Capabilities Endpoint

`GET /api/capabilities` includes `"read_only"`, read straight from `AppState::is_read_only()`
(see [server/mod.rs](../../src-tauri/core/src/server/mod.rs)). The UI does not use it — it
reads the same state through `get_app_mode` — but it lets an operator check the mode with
`curl`.

## Key Files

| File | Purpose |
|------|---------|
| [app_state.rs](../../src-tauri/core/src/app_state.rs) | `AppMode` enum, `AppState`, `enable_read_only`, `is_read_only` |
| [commands_internal/helpers.rs](../../src-tauri/core/src/commands_internal/helpers.rs) | The `check_read_only!` macro |
| [commands_internal/](../../src-tauri/core/src/commands_internal/) | Guarded `*_internal` write functions |
| [db.rs](../../src-tauri/core/src/db.rs) | `check_migration_compatibility()` — present, currently uncalled |
| [commands_internal/settings_cmd.rs](../../src-tauri/core/src/commands_internal/settings_cmd.rs) | `get_app_mode_internal`, `AppModeInfo` |
| [server/mod.rs](../../src-tauri/core/src/server/mod.rs) | `read_only` in the capabilities response |
| [appMode.ts](../../src/lib/stores/appMode.ts) | Frontend read-only state |
| [+layout.svelte](../../src/routes/+layout.svelte) | The banner |

## Design Decisions

### Why Block All Writes on Unknown Migrations?

The rule the check was written for ([ADR-012](../../DECISIONS.md) forward-only migrations): if the database
carries migrations this build does not recognise, it was written by a newer version.
Writing to it anyway could corrupt data whose column semantics changed, or produce rows the
newer version cannot read. Refusing all writes is the safe fallback — the user can still
view data and export reports.

### Why Backup Creation Stays Unguarded

A read-only database is exactly the state in which you most want a snapshot. Creating one
touches nothing the guard protects, so gating it would trade real safety for consistency of
appearance.

### Why the Guard Survived the Desktop Deletion

The desktop shell used to be the only thing that armed it, so deleting the shell briefly
left the guard unreachable. It was rewired into the web binary rather than dropped: the
guard is one line per write command, it is exercised by unit tests, and the failure it
prevents (silent corruption of a legal-compliance record) is the worst one available.

### Error Message in Slovak

The app is primarily for Slovak users. Error messages use Slovak to match the UI language.

## Related

- [ADR-012](../../DECISIONS.md): Forward-only migrations
- [ADR-030](../../DECISIONS.md): Tauri desktop app removed; the container is the only target
