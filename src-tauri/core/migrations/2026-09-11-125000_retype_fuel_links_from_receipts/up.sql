-- Task 84 repair: fix paperless links that the multi-invoice backfill
-- mislabelled as 'Other'.
--
-- The 2026-07-15 multi_invoice migration added `assignment_type` and backfilled
-- every pre-existing link with:
--     'Fuel' if (trip has fuel AND NOT EXISTS a Fuel receipt on that trip)
--     else 'Other'
-- Those links were created before `assignment_type` existed (they came from
-- scripts/migrate_local_to_paperless.py, which was non-destructive and left the
-- local Fuel receipts in place). Because the Fuel receipt still existed, the
-- guard `NOT EXISTS (Fuel receipt)` was false and the fuel document's link was
-- stored as 'Other'.
--
-- Task 84 drops the `receipts` table, so after the drop those trips lose
-- `has_fuel` and every fill-up is reported as a missing invoice. This migration
-- repairs the mislabelled links while `receipts` still exists, so it must run
-- BEFORE 2026-09-11-130000_drop_receipts.
--
-- Only links the backfill could have mislabelled are touched:
--   * assignment_type = 'Other'
--   * amount_eur / applied_amount_cents are NULL (the backfill wrote NULL;
--     anything assigned after Task 66 carries an explicit type and snapshots)
--   * created_at predates the multi-invoice migration
--   * the trip has fuel, has a Fuel receipt, and has no Fuel link yet
-- Running before the drop makes the subquery valid; it is a no-op on any
-- database that has no such rows.
UPDATE paperless_trip_links
SET assignment_type = 'Fuel',
    updated_at = strftime('%Y-%m-%dT%H:%M:%S', 'now')
WHERE assignment_type = 'Other'
  AND amount_eur IS NULL
  AND applied_amount_cents IS NULL
  AND created_at < '2026-07-15'
  AND trip_id IN (
      SELECT t.id
      FROM trips t
      WHERE t.fuel_liters > 0
        AND EXISTS (
            SELECT 1 FROM receipts r
            WHERE r.trip_id = t.id AND r.assignment_type = 'Fuel'
        )
        AND NOT EXISTS (
            SELECT 1 FROM paperless_trip_links p
            WHERE p.trip_id = t.id AND p.assignment_type = 'Fuel'
        )
  );
