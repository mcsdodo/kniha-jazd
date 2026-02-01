-- Migration: replace_receipt_date_with_datetime
-- Upgrade receipts from date-only to full datetime for trip range validation

-- Add new datetime column
ALTER TABLE receipts ADD COLUMN receipt_datetime TEXT DEFAULT NULL;

-- Backfill from existing receipt_date (assume midnight for existing data)
UPDATE receipts
SET receipt_datetime = receipt_date || 'T00:00:00'
WHERE receipt_date IS NOT NULL;

-- Drop index on old column first (required before dropping column)
DROP INDEX IF EXISTS idx_receipts_date;

-- Drop legacy column (requires SQLite 3.35.0+)
ALTER TABLE receipts DROP COLUMN receipt_date;

-- Create index on new datetime column
CREATE INDEX IF NOT EXISTS idx_receipts_datetime ON receipts(receipt_datetime);
