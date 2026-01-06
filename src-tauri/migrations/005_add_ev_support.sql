-- Migration 005: Add Electric Vehicle (EV) support
-- Adds vehicle_type, battery fields to vehicles, and energy fields to trips

-- Vehicle EV fields
ALTER TABLE vehicles ADD COLUMN vehicle_type TEXT NOT NULL DEFAULT 'Ice';
ALTER TABLE vehicles ADD COLUMN battery_capacity_kwh REAL;
ALTER TABLE vehicles ADD COLUMN baseline_consumption_kwh REAL;
ALTER TABLE vehicles ADD COLUMN initial_battery_percent REAL;

-- Trip energy fields
ALTER TABLE trips ADD COLUMN energy_kwh REAL;
ALTER TABLE trips ADD COLUMN energy_cost_eur REAL;
ALTER TABLE trips ADD COLUMN full_charge INTEGER DEFAULT 0;
ALTER TABLE trips ADD COLUMN soc_override_percent REAL;

-- Index for filtering by vehicle type
CREATE INDEX IF NOT EXISTS idx_vehicles_type ON vehicles(vehicle_type);
