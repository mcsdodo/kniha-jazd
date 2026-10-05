-- Task 88: places become entities; trips and routes point at them by id.
-- Plain SQL, run in one transaction by Diesel. kj_normalise is
-- places::normalise, registered from Rust by db::prepare_connection.
-- `diesel migration run` from the CLI fails here with
-- "no such function: kj_normalise": only the app can run this migration.
-- Foreign keys are ON (the bundled SQLite is built with
-- SQLITE_DEFAULT_FOREIGN_KEYS=1), so DROP TABLE trips would cascade into
-- trip_routes and paperless_trip_links. Step 6 copies both first.

-- 1. Every spelling a trip uses, with its use count. A blank endpoint is
--    renamed to one shared placeholder place so its trip keeps a target.
CREATE TEMP TABLE kj_spellings AS
SELECT raw, kj_normalise(raw) AS key, SUM(uses) AS uses FROM (
    SELECT CASE WHEN kj_normalise(origin) = '' THEN 'Neznáme miesto' ELSE origin END AS raw,
           COUNT(*) AS uses
      FROM trips GROUP BY 1
    UNION ALL
    SELECT CASE WHEN kj_normalise(destination) = '' THEN 'Neznáme miesto' ELSE destination END,
           COUNT(*)
      FROM trips GROUP BY 1
) GROUP BY raw;

-- 2. The new places table.
CREATE TABLE places_new (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    normalised_name TEXT NOT NULL UNIQUE,
    lat REAL,
    lon REAL,
    source TEXT,
    created_at TEXT NOT NULL
);

-- 3. One place per key a trip uses. Name: most uses, then the byte-wise
--    smaller spelling (BINARY collation), the rule list_places used.
WITH ranked AS (
    SELECT key, raw,
           ROW_NUMBER() OVER (PARTITION BY key ORDER BY uses DESC, raw ASC) AS rn
      FROM kj_spellings
)
INSERT INTO places_new (id, name, normalised_name, lat, lon, source, created_at)
SELECT lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' ||
             substr(hex(randomblob(2)), 2) || '-' ||
             substr('89ab', 1 + (abs(random()) % 4), 1) ||
             substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))),
       r.raw, r.key, p.lat, p.lon,
       CASE WHEN p.lat IS NULL THEN NULL ELSE p.source END,
       strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
  FROM ranked r
  LEFT JOIN places p ON p.normalised_name = r.key
 WHERE r.rn = 1;

-- 4. Old places rows that no trip names stay as places.
INSERT INTO places_new (id, name, normalised_name, lat, lon, source, created_at)
SELECT lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' ||
             substr(hex(randomblob(2)), 2) || '-' ||
             substr('89ab', 1 + (abs(random()) % 4), 1) ||
             substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))),
       p.display_name, p.normalised_name, p.lat, p.lon, p.source,
       strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
  FROM places p
 WHERE p.normalised_name NOT IN (SELECT normalised_name FROM places_new);

DROP TABLE places;
ALTER TABLE places_new RENAME TO places;

-- 5. Routes, built while trips still carry their strings. Only pairs a trip
--    uses survive (the others were already hidden by the trips join).
--    Duplicates on one place pair keep the row whose exact strings the
--    latest trip on that pair used, else the first by id.
CREATE TABLE routes_new (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT NOT NULL,
    origin_place_id TEXT NOT NULL,
    destination_place_id TEXT NOT NULL,
    distance_km REAL NOT NULL,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id),
    FOREIGN KEY (origin_place_id) REFERENCES places(id),
    FOREIGN KEY (destination_place_id) REFERENCES places(id),
    UNIQUE(vehicle_id, origin_place_id, destination_place_id)
);

WITH latest AS (
    SELECT vehicle_id, origin, destination,
           kj_normalise(origin) AS ko, kj_normalise(destination) AS kd,
           ROW_NUMBER() OVER (
               PARTITION BY vehicle_id, kj_normalise(origin), kj_normalise(destination)
               ORDER BY start_datetime DESC, created_at DESC) AS rn
      FROM trips
),
ranked AS (
    SELECT r.id, r.vehicle_id, po.id AS o_id, pd.id AS d_id, r.distance_km,
           ROW_NUMBER() OVER (
               PARTITION BY r.vehicle_id, po.id, pd.id
               ORDER BY CASE WHEN r.origin = l.origin AND r.destination = l.destination
                             THEN 0 ELSE 1 END,
                        r.id) AS rn
      FROM routes r
      JOIN latest l ON l.rn = 1 AND l.vehicle_id = r.vehicle_id
                   AND l.ko = kj_normalise(r.origin) AND l.kd = kj_normalise(r.destination)
      JOIN places po ON po.normalised_name = kj_normalise(r.origin)
      JOIN places pd ON pd.normalised_name = kj_normalise(r.destination)
)
INSERT INTO routes_new (id, vehicle_id, origin_place_id, destination_place_id, distance_km)
SELECT id, vehicle_id, o_id, d_id, distance_km FROM ranked WHERE rn = 1;

DROP TABLE routes;
ALTER TABLE routes_new RENAME TO routes;
CREATE INDEX idx_routes_vehicle ON routes(vehicle_id);

-- 6. Trips: same columns in the same order, the two strings replaced by ids.
--    Keep the child rows: DROP TABLE trips cascades with foreign keys on.
CREATE TEMP TABLE kj_tr AS SELECT * FROM trip_routes;
CREATE TEMP TABLE kj_ptl AS SELECT * FROM paperless_trip_links;

CREATE TABLE trips_new (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT NOT NULL,
    origin_place_id TEXT NOT NULL,
    destination_place_id TEXT NOT NULL,
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
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id),
    FOREIGN KEY (origin_place_id) REFERENCES places(id),
    FOREIGN KEY (destination_place_id) REFERENCES places(id)
);

INSERT INTO trips_new
SELECT t.id, t.vehicle_id, po.id, pd.id, t.distance_km, t.odometer, t.purpose,
       t.fuel_liters, t.fuel_cost_eur, t.other_costs_eur, t.other_costs_note,
       t.created_at, t.updated_at, t.full_tank, t.energy_kwh, t.energy_cost_eur,
       t.full_charge, t.soc_override_percent, t.start_datetime, t.end_datetime
  FROM trips t
  -- LEFT JOIN: a key with no place gives NULL, and NOT NULL aborts the
  -- transaction instead of dropping the trip.
  LEFT JOIN places po ON po.normalised_name = kj_normalise(
       CASE WHEN kj_normalise(t.origin) = '' THEN 'Neznáme miesto' ELSE t.origin END)
  LEFT JOIN places pd ON pd.normalised_name = kj_normalise(
       CASE WHEN kj_normalise(t.destination) = '' THEN 'Neznáme miesto' ELSE t.destination END);

DROP TABLE trips;
ALTER TABLE trips_new RENAME TO trips;
CREATE INDEX idx_trips_vehicle_start_datetime ON trips(vehicle_id, start_datetime);
CREATE INDEX idx_trips_origin_place ON trips(origin_place_id);
CREATE INDEX idx_trips_destination_place ON trips(destination_place_id);

INSERT INTO trip_routes SELECT * FROM kj_tr;
INSERT INTO paperless_trip_links SELECT * FROM kj_ptl;
DROP TABLE kj_tr;
DROP TABLE kj_ptl;
DROP TABLE kj_spellings;
