-- SQLite does not support DROP COLUMN directly.
-- The datetime column will remain but be ignored by older app versions.
-- This is intentional for forward compatibility.
SELECT 1;
