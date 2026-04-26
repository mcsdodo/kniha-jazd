-- SQLite doesn't support DROP COLUMN directly
-- This would require table recreation, which is destructive
-- For now, we leave the columns in place (they're nullable and harmless)

-- If needed, a full table recreation would be:
-- CREATE TABLE vehicles_backup AS SELECT [original columns] FROM vehicles;
-- DROP TABLE vehicles;
-- ALTER TABLE vehicles_backup RENAME TO vehicles;
