-- Revert: restore receipt_date column from receipt_datetime

-- Add back the date-only column
ALTER TABLE receipts ADD COLUMN receipt_date TEXT DEFAULT NULL;

-- Extract date portion from datetime
UPDATE receipts
SET receipt_date = substr(receipt_datetime, 1, 10)
WHERE receipt_datetime IS NOT NULL;

-- Drop index on datetime column
DROP INDEX IF EXISTS idx_receipts_datetime;

-- Drop the datetime column
ALTER TABLE receipts DROP COLUMN receipt_datetime;

-- Recreate index on date column
CREATE INDEX IF NOT EXISTS idx_receipts_date ON receipts(receipt_date);
