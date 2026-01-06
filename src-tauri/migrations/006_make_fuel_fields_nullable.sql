-- Migration 006: Make fuel fields nullable for BEV support
-- SQLite doesn't support ALTER COLUMN, so we need to recreate the table

-- Step 1: Create new table with nullable fuel fields
CREATE TABLE vehicles_new (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    license_plate TEXT NOT NULL,
    vehicle_type TEXT NOT NULL DEFAULT 'Ice',
    -- Fuel fields (nullable for BEV)
    tank_size_liters REAL,
    tp_consumption REAL,
    -- Battery fields (nullable for ICE)
    battery_capacity_kwh REAL,
    baseline_consumption_kwh REAL,
    initial_battery_percent REAL,
    -- Common fields
    initial_odometer REAL NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Step 2: Copy data from old table
INSERT INTO vehicles_new (
    id, name, license_plate, vehicle_type,
    tank_size_liters, tp_consumption,
    battery_capacity_kwh, baseline_consumption_kwh, initial_battery_percent,
    initial_odometer, is_active, created_at, updated_at
)
SELECT
    id, name, license_plate, vehicle_type,
    tank_size_liters, tp_consumption,
    battery_capacity_kwh, baseline_consumption_kwh, initial_battery_percent,
    initial_odometer, is_active, created_at, updated_at
FROM vehicles;

-- Step 3: Drop old table
DROP TABLE vehicles;

-- Step 4: Rename new table
ALTER TABLE vehicles_new RENAME TO vehicles;

-- Step 5: Recreate indexes
CREATE INDEX IF NOT EXISTS idx_vehicles_type ON vehicles(vehicle_type);
