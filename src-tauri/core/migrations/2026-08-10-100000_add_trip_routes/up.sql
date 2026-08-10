-- Task 70: generated route maps, one per trip.
-- Stores only what the export needs to re-render: the OSRM polyline plus
-- minimal metadata. Rendered PNGs and OSM tiles live in a disposable
-- app-data cache, never here — see _tasks/70-route-map-integration/02-design.md.
CREATE TABLE trip_routes (
    trip_id TEXT PRIMARY KEY,
    waypoints TEXT NOT NULL,
    polyline TEXT NOT NULL,
    target_km REAL NOT NULL,
    road_km REAL NOT NULL,
    dataset_version TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (trip_id) REFERENCES trips(id) ON DELETE CASCADE
);
