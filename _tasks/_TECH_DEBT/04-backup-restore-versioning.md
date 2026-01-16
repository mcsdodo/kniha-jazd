# Tech Debt: Backup/Restore Version Compatibility

**Date:** 2026-01-16
**Priority:** Medium
**Effort:** Medium (2-4h)
**Component:** `src-tauri/src/commands.rs` (backup functions)
**Status:** Open

## Problem

When restoring a backup, there's no check for whether the backup was created by a newer app version. This could lead to:
1. Restoring a backup with unknown schema → app crashes or corrupts data
2. User confusion when restored data doesn't display correctly

## Impact

- Data corruption risk when restoring backups from newer versions
- No warning to user about version mismatch
- Inconsistent with the read-only mode protection for live databases

## Root Cause

The backup/restore feature was implemented before the multi-PC/version compatibility feature. It simply copies the database file without checking migration versions.

## Recommended Solution

1. **Store version metadata in backup**
   - When creating backup, store app version in filename or metadata
   - Format: `kniha-jazd-backup-YYYY-MM-DD-HH-MM-SS-v0.17.0.db`

2. **Check version before restore**
   - Read `__diesel_schema_migrations` from backup file
   - Compare against current app's embedded migrations
   - If backup has unknown migrations, show warning:
     "Táto záloha bola vytvorená novšou verziou (v0.18.0). Obnovenie môže spôsobiť problémy."
   - Options: [Obnoviť napriek tomu] [Zrušiť]

3. **Alternative: Block restore of newer backups**
   - Simpler but more restrictive
   - "Táto záloha vyžaduje aplikáciu v0.18.0 alebo novšiu."

## Implementation Notes

- Reuse `db::check_migration_compatibility()` for version checking
- Consider extracting backup metadata without full restore
- Update i18n for new warning messages

## Related

- `_tasks/39-custom-db-location/` - Custom DB location feature
- `src-tauri/src/db.rs:check_migration_compatibility()` - Version checking logic

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-16 | Created analysis | Identified during custom DB location feature implementation |
