-- Task 72: which producer built a route map.
-- DEFAULT 'loop' backfills correctly by construction: every route saved before
-- this migration came from the Task 70 genetic-algorithm loop.
ALTER TABLE trip_routes ADD COLUMN mode TEXT NOT NULL DEFAULT 'loop';
