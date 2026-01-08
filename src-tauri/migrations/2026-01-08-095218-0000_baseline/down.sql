-- Reverse of baseline migration
-- Drop tables in reverse order of creation (due to foreign keys)

DROP INDEX IF EXISTS idx_receipts_vehicle;
DROP INDEX IF EXISTS idx_receipts_date;
DROP INDEX IF EXISTS idx_receipts_trip;
DROP INDEX IF EXISTS idx_receipts_status;
DROP INDEX IF EXISTS idx_vehicles_type;
DROP INDEX IF EXISTS idx_routes_vehicle;
DROP INDEX IF EXISTS idx_trips_vehicle_date;

DROP TABLE IF EXISTS receipts;
DROP TABLE IF EXISTS settings;
DROP TABLE IF EXISTS routes;
DROP TABLE IF EXISTS trips;
DROP TABLE IF EXISTS vehicles;
