-- Drop legacy datetime columns (date, datetime, end_time)
-- Data was already migrated to start_datetime and end_datetime in previous migration
-- SQLite requires table rebuild to drop columns

-- Disable foreign key checks during rebuild
PRAGMA foreign_keys = OFF;

-- Create new table without legacy columns
CREATE TABLE trips_new (
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
    full_tank INTEGER NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 0,
    energy_kwh REAL,
    energy_cost_eur REAL,
    full_charge INTEGER DEFAULT 0,
    soc_override_percent REAL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    start_datetime TEXT NOT NULL,
    end_datetime TEXT,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id)
);

-- Copy data from old table (excluding dropped columns)
INSERT INTO trips_new (
    id, vehicle_id, origin, destination, distance_km, odometer, purpose,
    fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note,
    full_tank, sort_order, energy_kwh, energy_cost_eur, full_charge,
    soc_override_percent, created_at, updated_at, start_datetime, end_datetime
)
SELECT
    id, vehicle_id, origin, destination, distance_km, odometer, purpose,
    fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note,
    full_tank, sort_order, energy_kwh, energy_cost_eur, full_charge,
    soc_override_percent, created_at, updated_at, start_datetime, end_datetime
FROM trips;

-- Drop old table
DROP TABLE trips;

-- Rename new table
ALTER TABLE trips_new RENAME TO trips;

-- Recreate index (using start_datetime instead of date)
CREATE INDEX IF NOT EXISTS idx_trips_vehicle_start_datetime ON trips(vehicle_id, start_datetime);

-- Re-enable foreign key checks
PRAGMA foreign_keys = ON;
