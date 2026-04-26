-- This migration is not reversible (forward-only per ADR-012)
-- Legacy columns cannot be restored without the original data
-- To rollback, restore from backup
SELECT 'Migration 2026-01-30-100000_drop_legacy_datetime_columns is not reversible' AS warning;
