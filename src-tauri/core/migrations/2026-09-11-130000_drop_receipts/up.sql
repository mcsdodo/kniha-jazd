-- Task 84: local receipts are removed; Paperless is the only invoice source.
--
-- This migration does two things, and the order is load-bearing:
--   1. repair links the multi-invoice backfill mislabelled as 'Other'
--   2. DROP TABLE receipts
-- Step 1 reads `receipts`, so it cannot run after step 2. Both statements live
-- in this one folder so they cannot be separated or reordered: a separate
-- repair migration would be skipped on any database that already applied the
-- drop, and would then fail with "no such table: receipts".
--
-- The drop intentionally discards local receipt rows. Receipts already assigned
-- to a trip survive as paperless_trip_links rows; every other row is gone.
-- Export the table BEFORE upgrading if you still need it:
--     sqlite3 -header -csv data/kniha-jazd.db "SELECT * FROM receipts;" > receipts.csv
-- Indexes drop with the table.

-- ---------------------------------------------------------------------------
-- 1. Repair mislabelled fuel links
-- ---------------------------------------------------------------------------
-- The 2026-07-15 multi_invoice migration added `assignment_type` and backfilled
-- every pre-existing link with:
--     'Fuel' if (trip has fuel AND NOT EXISTS a Fuel receipt on that trip)
--     else 'Other'
-- Those links were created before `assignment_type` existed (they came from the
-- one-off local-to-Paperless relink script, which was non-destructive and left
-- the local Fuel receipts in place). Because the Fuel receipt still existed, the
-- guard `NOT EXISTS (Fuel receipt)` was false and the fuel document's link was
-- stored as 'Other'.
--
-- After the drop below, those trips lose `has_fuel` and every fill-up is
-- reported as a missing invoice. This repair prevents that.
--
-- Only links the backfill could have mislabelled are touched:
--   * assignment_type = 'Other'
--   * title IS NULL. This is the durable marker of a backfilled row: the
--     backfill wrote NULL into the title slot, and `upsert_paperless_link`
--     (db.rs) -- the only INSERT into this table -- always writes the document
--     title. Nothing later nulls it: setting an override updates two columns,
--     unassign deletes the row. A calendar cutoff cannot do this job, because
--     the backfill COPIES the old link's created_at instead of stamping the
--     migration time, so a database that applies multi_invoice late carries
--     backfilled rows with a recent created_at.
--   * amount_eur / applied_amount_cents are NULL (the backfill wrote NULL;
--     kept as a second guard -- a document with no amount also leaves both
--     NULL, so this alone cannot identify a backfilled row)
--   * the trip has fuel, has a Fuel receipt, has no Fuel link yet, and has no
--     Other receipt (an ambiguous trip could promote a parking or toll document
--     into the fuel slot, so it is skipped)
--   * at most ONE link per trip is promoted -- the partial unique index
--     idx_paperless_links_trip_fuel allows a single Fuel link per trip, and the
--     subquery below is uncorrelated, so without the MIN() guard a trip holding
--     two candidate links would abort the whole upgrade with a UNIQUE violation
-- It is a no-op on any database that has no such rows.
UPDATE paperless_trip_links
SET assignment_type = 'Fuel',
    updated_at = strftime('%Y-%m-%dT%H:%M:%S', 'now')
WHERE assignment_type = 'Other'
  AND title IS NULL
  AND amount_eur IS NULL
  AND applied_amount_cents IS NULL
  AND paperless_document_id = (
      SELECT MIN(p2.paperless_document_id)
      FROM paperless_trip_links p2
      WHERE p2.trip_id = paperless_trip_links.trip_id
        AND p2.assignment_type = 'Other'
        AND p2.title IS NULL
        AND p2.amount_eur IS NULL
        AND p2.applied_amount_cents IS NULL
  )
  AND trip_id IN (
      SELECT t.id
      FROM trips t
      WHERE t.fuel_liters > 0
        AND EXISTS (
            SELECT 1 FROM receipts r
            WHERE r.trip_id = t.id AND r.assignment_type = 'Fuel'
        )
        AND NOT EXISTS (
            SELECT 1 FROM receipts r2
            WHERE r2.trip_id = t.id AND r2.assignment_type = 'Other'
        )
        AND NOT EXISTS (
            SELECT 1 FROM paperless_trip_links p
            WHERE p.trip_id = t.id AND p.assignment_type = 'Fuel'
        )
  );

-- ---------------------------------------------------------------------------
-- 2. Drop the local receipt store
-- ---------------------------------------------------------------------------
DROP TABLE receipts;
