-- Add VIN and driver name metadata to vehicles
-- These fields are optional for better vehicle documentation

ALTER TABLE vehicles ADD COLUMN vin TEXT;
ALTER TABLE vehicles ADD COLUMN driver_name TEXT;
