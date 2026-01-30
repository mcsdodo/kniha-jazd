-- Drop legacy datetime columns (date, datetime, end_time)
-- Data was already migrated to start_datetime and end_datetime in previous migration
-- Using ALTER TABLE DROP COLUMN (SQLite 3.35.0+, March 2021)

-- Drop the old index that references 'date' column
DROP INDEX IF EXISTS idx_trips_vehicle_date;

-- Drop legacy columns one by one
ALTER TABLE trips DROP COLUMN date;
ALTER TABLE trips DROP COLUMN datetime;
ALTER TABLE trips DROP COLUMN end_time;

-- Create new index using start_datetime
CREATE INDEX IF NOT EXISTS idx_trips_vehicle_start_datetime ON trips(vehicle_id, start_datetime);
