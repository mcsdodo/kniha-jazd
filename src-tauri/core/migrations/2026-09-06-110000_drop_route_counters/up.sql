-- Task 76: usage_count and last_used were stored aggregates of trips, maintained
-- by write paths that never agreed - 52 of 96 rows were wrong. Both are computed
-- in get_routes_for_vehicle now (ADR-033), so the stored copies are deleted
-- rather than corrected: there is nothing left that could drift.
--
-- SQLite 3.35.0+ supports ALTER TABLE DROP COLUMN; neither column is indexed.
--
-- distance_km stays. It is the value typed for the pair, not an aggregate.
ALTER TABLE routes DROP COLUMN usage_count;
ALTER TABLE routes DROP COLUMN last_used;
