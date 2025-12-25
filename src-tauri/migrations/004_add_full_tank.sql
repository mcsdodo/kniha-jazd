-- Add full_tank column to trips table
-- true (1) = full tank fillup, false (0) = partial fillup
-- Default 1 ensures existing data is treated as full tank

ALTER TABLE trips ADD COLUMN full_tank INTEGER NOT NULL DEFAULT 1;
