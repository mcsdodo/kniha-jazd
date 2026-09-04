---
paths:
  - "src-tauri/core/migrations/**/*.sql"
---

# Database Migration Rules

## Strategy: Forward-Only ([ADR-012](../../DECISIONS.md))

We do NOT support older app versions reading newer databases.

## Required Patterns

- **Always** add columns with DEFAULT values (for migration to succeed)
- **Migrations run automatically** on server start — `Database::new` calls
  `run_pending_migrations` before handing back a connection
- **A backup is created first** — when the DB file already exists and Diesel reports pending
  migrations, `Database::create_pre_migration_backup` writes
  `kniha-jazd-backup-{timestamp}-pre-migration-v{version}.db` into `backups/`. A failed
  backup logs a warning and does not block startup
- **No legacy field sync** - don't maintain deprecated columns for backward compat

## SQL Examples

```sql
-- Standard migration:
ALTER TABLE trips ADD COLUMN new_field TEXT DEFAULT '';

-- Allowed (if needed for cleanup):
ALTER TABLE trips DROP COLUMN deprecated_field;  -- OK after deprecation period
```

## Note

Users must upgrade to use migrated databases. There is no auto-update: the deployment is a
Docker image, and upgrading means pulling a newer tag and restarting the container
([ADR-030](../../DECISIONS.md)). The `/data` volume carries the database across, and migrations run on the next
start.

An operator who rolls *back* to an older image is the case [ADR-012](../../DECISIONS.md) refuses to support — the
older binary will not recognise the newer migrations. `Database::check_migration_compatibility`
exists to detect exactly that, but nothing currently calls it, so a rollback fails loudly at
the query level rather than degrading to read-only.
