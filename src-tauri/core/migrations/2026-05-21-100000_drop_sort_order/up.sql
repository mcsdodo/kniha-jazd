-- Drop sort_order column from trips table
-- Trips are now ordered chronologically by (start_datetime, created_at) — see ADR for task 65.
-- SQLite 3.35.0+ supports ALTER TABLE DROP COLUMN.

ALTER TABLE trips DROP COLUMN sort_order;
