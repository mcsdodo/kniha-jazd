-- Task 20: persist the round-trip checkbox instead of treating it as a
-- generation input only.
-- DEFAULT 0 backfills correctly by construction: every route saved before
-- today is either a loop (already closed) or a one-way direct route, so
-- neither ever asked for a return leg.
ALTER TABLE trip_routes ADD COLUMN round_trip BOOLEAN NOT NULL DEFAULT 0;
