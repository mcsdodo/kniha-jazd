-- Add initial_odometer column to vehicles table (for existing databases)
-- SQLite doesn't support IF NOT EXISTS for ALTER TABLE, so we use a workaround

-- Check if column exists by trying to select it, if it fails the column doesn't exist
-- This is handled in Rust code - this migration adds the column

ALTER TABLE vehicles ADD COLUMN initial_odometer REAL NOT NULL DEFAULT 0;
