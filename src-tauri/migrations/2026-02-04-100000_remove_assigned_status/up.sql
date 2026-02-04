-- Remove "Assigned" status - assignment is now determined by trip_id
-- Convert legacy "Assigned" values to "Parsed" (they were parseable to be assigned)
UPDATE receipts SET status = 'Parsed' WHERE status = 'Assigned';
