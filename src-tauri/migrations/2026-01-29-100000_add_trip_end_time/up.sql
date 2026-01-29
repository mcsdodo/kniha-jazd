-- Add end_time column for trip end time (HH:MM format)
-- DEFAULT '' for backward compatibility with older app versions
ALTER TABLE trips ADD COLUMN end_time TEXT NOT NULL DEFAULT '';
