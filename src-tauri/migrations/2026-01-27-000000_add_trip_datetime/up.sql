-- Add datetime column to trips table
-- Combines date + time into single field, time defaults to 00:00:00
ALTER TABLE trips ADD COLUMN datetime TEXT NOT NULL DEFAULT '';

-- Populate from existing date column
UPDATE trips SET datetime = date || 'T00:00:00';
