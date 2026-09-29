-- Task 86: the routing service a route was computed with ('osrm' or 'sygic').
-- NULL means unknown; the page then shows the server default.
--
-- Backfill only what can be proven:
-- * Every route saved before the first Sygic commit (c80e15e,
--   2026-09-29T07:39:53Z) came from OSRM -- no code could call Sygic before
--   that. `created_at` is RFC 3339 in UTC (`+00:00`), so a text comparison
--   orders it correctly.
-- * A route with an avoid list came from Sygic -- OSRM refuses an avoid list.
ALTER TABLE trip_routes ADD COLUMN provider TEXT DEFAULT NULL;
UPDATE trip_routes SET provider = 'osrm' WHERE created_at < '2026-09-29T07:39:53';
UPDATE trip_routes SET provider = 'sygic' WHERE avoid <> '[]';
