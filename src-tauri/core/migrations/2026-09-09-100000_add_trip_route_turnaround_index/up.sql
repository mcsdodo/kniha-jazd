-- Task 78: a round trip is now two routed legs, and the saved row is the two
-- of them joined. This column is the index, in `waypoints`, of the point
-- where the outbound leg ends and the return leg begins.
--
-- NULL is the correct backfill and needs no data migration. Every round trip
-- saved before today was closed by appending exactly one clone of the first
-- waypoint, so `[A, ...vias, B, A]` splits at `len - 2` with no ambiguity.
-- NULL means "use that rule"; a value means "split here".
ALTER TABLE trip_routes ADD COLUMN turnaround_index INTEGER DEFAULT NULL;
