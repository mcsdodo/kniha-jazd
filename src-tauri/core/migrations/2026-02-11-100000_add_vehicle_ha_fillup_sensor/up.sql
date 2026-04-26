-- Add Home Assistant fillup sensor entity ID to vehicles
-- This field stores the sensor entity ID (e.g., "sensor.kniha_jazd_fillup")
-- for pushing suggested fillup text to Home Assistant

ALTER TABLE vehicles ADD COLUMN ha_fillup_sensor TEXT DEFAULT NULL;
