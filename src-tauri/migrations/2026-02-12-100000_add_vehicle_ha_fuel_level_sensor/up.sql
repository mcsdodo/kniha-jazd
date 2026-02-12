-- Add Home Assistant fuel level sensor entity ID to vehicles
-- This field stores the sensor entity ID (e.g., "sensor.car_fuel_level")
-- for fetching real fuel level percentage from Home Assistant

ALTER TABLE vehicles ADD COLUMN ha_fuel_level_sensor TEXT DEFAULT NULL;
