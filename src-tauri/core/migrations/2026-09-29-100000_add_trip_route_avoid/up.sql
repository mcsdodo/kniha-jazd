-- Task 85: the "avoid paid roads" countries a route was computed with, as a
-- JSON array of `<iso3>:tolls` strings. '[]' is the correct backfill: every
-- route saved before today was computed with no avoid list.
ALTER TABLE trip_routes ADD COLUMN avoid TEXT NOT NULL DEFAULT '[]';
