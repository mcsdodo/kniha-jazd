-- SQLite cannot drop a column that an index uses: drop the index first.
DROP INDEX IF EXISTS idx_places_single_home;
ALTER TABLE places DROP COLUMN is_home;
