---
paths:
  - "src-tauri/migrations/**/*.sql"
---

# Database Migration Rules

## Strategy: Forward-Only (ADR-012)

We do NOT support older app versions reading newer databases.

## Required Patterns

- **Always** add columns with DEFAULT values (for migration to succeed)
- **Migrations run automatically** on app start
- **Backups are created** before migrations (existing behavior)
- **No legacy field sync** - don't maintain deprecated columns for backward compat

## SQL Examples

```sql
-- Standard migration:
ALTER TABLE trips ADD COLUMN new_field TEXT DEFAULT '';

-- Allowed (if needed for cleanup):
ALTER TABLE trips DROP COLUMN deprecated_field;  -- OK after deprecation period
```

## Note

Users must upgrade the app to use migrated databases. Auto-update ensures this happens quickly.
