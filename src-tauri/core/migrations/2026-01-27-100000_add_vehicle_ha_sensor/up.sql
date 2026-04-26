-- Add Home Assistant ODO sensor entity ID to vehicles
-- This field stores the sensor entity ID (e.g., "sensor.skoda_octavia_odometer")
-- for fetching real-time ODO from Home Assistant

ALTER TABLE vehicles ADD COLUMN ha_odo_sensor TEXT DEFAULT NULL;
