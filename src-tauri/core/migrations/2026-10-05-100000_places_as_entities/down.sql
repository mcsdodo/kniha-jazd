-- Forward-only in practice (ADR-012). Rebuilds the string columns from
-- the place names so a developer can step back one migration. The other
-- spellings of a place are gone; each trip gets the place name.
-- Foreign keys are ON: keep the child rows across DROP TABLE trips.
CREATE TEMP TABLE kj_tr AS SELECT * FROM trip_routes;
CREATE TEMP TABLE kj_ptl AS SELECT * FROM paperless_trip_links;

CREATE TABLE trips_old (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT NOT NULL,
    origin TEXT NOT NULL,
    destination TEXT NOT NULL,
    distance_km REAL NOT NULL,
    odometer REAL NOT NULL,
    purpose TEXT NOT NULL,
    fuel_liters REAL,
    fuel_cost_eur REAL,
    other_costs_eur REAL,
    other_costs_note TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    full_tank INTEGER NOT NULL DEFAULT 1,
    energy_kwh REAL,
    energy_cost_eur REAL,
    full_charge INTEGER DEFAULT 0,
    soc_override_percent REAL,
    start_datetime TEXT NOT NULL DEFAULT '',
    end_datetime TEXT DEFAULT NULL,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id)
);
INSERT INTO trips_old
SELECT t.id, t.vehicle_id, po.name, pd.name, t.distance_km, t.odometer, t.purpose,
       t.fuel_liters, t.fuel_cost_eur, t.other_costs_eur, t.other_costs_note,
       t.created_at, t.updated_at, t.full_tank, t.energy_kwh, t.energy_cost_eur,
       t.full_charge, t.soc_override_percent, t.start_datetime, t.end_datetime
  FROM trips t
  JOIN places po ON po.id = t.origin_place_id
  JOIN places pd ON pd.id = t.destination_place_id;
DROP TABLE trips;
ALTER TABLE trips_old RENAME TO trips;
CREATE INDEX idx_trips_vehicle_start_datetime ON trips(vehicle_id, start_datetime);
INSERT INTO trip_routes SELECT * FROM kj_tr;
INSERT INTO paperless_trip_links SELECT * FROM kj_ptl;
DROP TABLE kj_tr;
DROP TABLE kj_ptl;

CREATE TABLE routes_old (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT NOT NULL,
    origin TEXT NOT NULL,
    destination TEXT NOT NULL,
    distance_km REAL NOT NULL,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id),
    UNIQUE(vehicle_id, origin, destination)
);
INSERT INTO routes_old
SELECT r.id, r.vehicle_id, po.name, pd.name, r.distance_km
  FROM routes r
  JOIN places po ON po.id = r.origin_place_id
  JOIN places pd ON pd.id = r.destination_place_id;
DROP TABLE routes;
ALTER TABLE routes_old RENAME TO routes;
CREATE INDEX idx_routes_vehicle ON routes(vehicle_id);

CREATE TABLE places_old (
    normalised_name TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL,
    lat REAL,
    lon REAL,
    source TEXT NOT NULL
);
INSERT INTO places_old
SELECT normalised_name, name, lat, lon, source FROM places WHERE lat IS NOT NULL;
DROP TABLE places;
ALTER TABLE places_old RENAME TO places;
