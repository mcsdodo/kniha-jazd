-- Add start_datetime and end_datetime columns (stored as TEXT in ISO 8601 format)
-- Keep old columns (date, datetime, end_time) for backward compatibility

-- start_datetime: copy from existing datetime field (preserves existing start times)
ALTER TABLE trips ADD COLUMN start_datetime TEXT NOT NULL DEFAULT '';

-- end_datetime: nullable, will be populated from date + end_time
ALTER TABLE trips ADD COLUMN end_datetime TEXT DEFAULT NULL;

-- Populate start_datetime from existing datetime field
UPDATE trips SET start_datetime = datetime WHERE start_datetime = '';

-- Populate end_datetime from date + end_time (if end_time exists and is not empty)
-- Format: "YYYY-MM-DD" + "T" + "HH:MM" + ":00"
UPDATE trips
SET end_datetime = date || 'T' || end_time || ':00'
WHERE end_time IS NOT NULL AND end_time != '';

-- For trips without end_time, set to date + 00:00:00
UPDATE trips
SET end_datetime = date || 'T00:00:00'
WHERE end_datetime IS NULL;
