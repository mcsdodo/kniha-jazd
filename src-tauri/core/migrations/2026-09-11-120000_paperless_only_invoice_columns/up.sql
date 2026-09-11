-- Task 84: Paperless is the only invoice source. Local receipts are removed.
--
-- This migration adds two columns only, so the link can carry the data the
-- local-receipt grid features used: the invoice datetime (grid datetime
-- warning) and the user-confirmed mismatch flag (grid override marker).
-- Existing rows get NULL / 0, so no false warning appears on upgrade.
--
-- The DROP TABLE receipts statement moves to a later migration (Task 7),
-- because receipt code still runs while this column work lands.
ALTER TABLE paperless_trip_links ADD COLUMN receipt_datetime TEXT DEFAULT NULL;
ALTER TABLE paperless_trip_links ADD COLUMN mismatch_override INTEGER NOT NULL DEFAULT 0;