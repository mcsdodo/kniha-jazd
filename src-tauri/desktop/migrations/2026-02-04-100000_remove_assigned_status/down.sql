-- Restore "Assigned" status for receipts that have trip_id set
UPDATE receipts SET status = 'Assigned' WHERE trip_id IS NOT NULL;
