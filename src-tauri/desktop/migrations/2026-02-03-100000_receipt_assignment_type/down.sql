-- Rollback: remove assignment type and mismatch override fields
-- Note: SQLite doesn't support DROP COLUMN in older versions
-- This is a forward-only migration per ADR-012

-- For reference, a proper rollback would:
-- 1. Create new table without the columns
-- 2. Copy data
-- 3. Drop old table
-- 4. Rename new table
-- Not implemented since we use forward-only strategy
