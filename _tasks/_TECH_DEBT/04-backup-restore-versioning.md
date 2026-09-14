# Tech Debt: Backup/Restore Version Compatibility

**Date:** 2026-01-16
**Priority:** Medium
**Effort:** Medium (2-8h)
**Component:** [backup.rs](../../src-tauri/core/src/commands_internal/backup.rs) (`restore_backup_internal`), [db.rs](../../src-tauri/core/src/db.rs) (`restore_from_file`)
**Status:** Fixed

## Problem

`restore_backup_internal` did only `fs::copy(backup, db_file)`. The server process
did not restart after a restore, so:

1. No migrations ran on the restored file. `Database::new` runs them only at
   process start.
2. `check_migration_compatibility` did not run on the restored file, so read-only
   mode stayed off.
3. The copy wrote over the live file without the `Database` mutex, under a
   connection that had cached the old schema.

## Impact

Both schema directions had no guard:

- **Backup from an older build.** This is the common case: every `pre-migration`
  backup has an older schema by design. After a restore, the live DB had 26 of 27
  migrations (proved by `test_restore_older_backup_applies_missing_migrations`
  before the fix). Queries for new columns failed until the container restarted.
- **Backup from a newer build.** In Docker this happens after an image rollback:
  the backups stay on the `/data` volume. The server served an unknown schema
  read-write.
- **Not a database.** `check_migration_compatibility` reads an unreadable
  `__diesel_schema_migrations` as "no migrations", which means "compatible".

## Root Cause

The backup/restore feature was written for the desktop app, before the migration
compatibility check. On desktop, a restore was followed by an app restart, which
ran migrations. The web server does not restart, and the frontend only reloads the
page (`handleConfirmRestore` in [+page.svelte](../../src/routes/settings/+page.svelte)).

## Solution (implemented)

1. `check_backup_restorable` opens the backup with `Database::from_path` (no
   migrations) and reads its migration history with the strict
   `Database::applied_migration_versions`.
   - An unreadable history rejects the file: "Súbor nie je platná záloha knihy jázd".
   - A migration this build does not embed blocks the restore: "Záloha bola
     vytvorená novšou verziou aplikácie ...". There is no "restore anyway" option.
     Startup handles the same case with read-only mode, never with a prompt.
2. `Database::restore_from_file` holds the connection lock. Under the lock, it
   copies the file, replaces the connection, and runs pending migrations. So an
   older backup is usable at once, and no request sees a half-restored file.
3. `check_migration_compatibility` stays lenient: [main.rs](../../src-tauri/web/src/main.rs)
   depends on it.

The error strings are Slovak and come from Rust. The settings page shows them
through `errorRestoreBackup`, so the frontend did not change.

Tests in [backup.rs](../../src-tauri/core/src/commands_internal/backup.rs):
`test_restore_rejects_backup_from_newer_build`,
`test_restore_older_backup_applies_missing_migrations`,
`test_restore_rejects_backup_without_migration_history`. The current-schema
roundtrip is `restore_backup_roundtrip` in
[dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs).

## Known limit (not fixed)

`restore_backup_internal` starts with `check_read_only!`. If the live DB comes
from a newer build, the server is read-only, so the user cannot restore an older
backup to recover from the rollback. To recover, stop the container and copy the
backup over `kniha-jazd.db` by hand. Then start the container: `Database::new`
migrates the file.

## Related

- [db.rs](../../src-tauri/core/src/db.rs) - `check_migration_compatibility`, `restore_from_file`, `applied_migration_versions`
- [main.rs](../../src-tauri/web/src/main.rs) - startup read-only check
- ADR-031 in [DECISIONS.md](../../DECISIONS.md) - image tags, the source of rollbacks

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-16 | Created analysis | Identified during custom DB location feature implementation |
| 2026-09-14 | Rewrote the analysis for the web server | The desktop framing and the `src-tauri/src/commands.rs` path were stale; the older-backup case was missing |
| 2026-09-14 | Block newer backups, migrate older backups, reject unreadable files | Matches the startup behavior; a "restore anyway" prompt would serve a schema this build cannot reason about |
| 2026-09-14 | Fixed | Restore checks the backup, then copies, reopens and migrates under the connection lock |
