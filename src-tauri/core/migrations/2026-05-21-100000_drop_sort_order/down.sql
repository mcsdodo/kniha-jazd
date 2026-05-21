-- Revert: add sort_order column back (defaults to 0 for all rows)
ALTER TABLE trips ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
