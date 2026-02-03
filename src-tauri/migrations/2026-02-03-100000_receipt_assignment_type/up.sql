-- Add receipt assignment type and mismatch override fields
-- Part of Task 51: Receipt-Trip State Model Redesign

-- Assignment type: "Fuel" or "Other" - explicitly set by user when assigning receipt to trip
-- Uses serde default serialization (stores exact enum variant name as TEXT)
ALTER TABLE receipts ADD COLUMN assignment_type TEXT DEFAULT NULL;

-- Mismatch override: user has confirmed that receipt data mismatch is intentional
-- 0 = no override (show warning if mismatch), 1 = user confirmed (suppress warning)
ALTER TABLE receipts ADD COLUMN mismatch_override INTEGER DEFAULT 0;

-- Unassign any existing receipts (rare edge case for existing users)
-- Users will need to manually reassign with explicit FUEL/OTHER type selection
-- This ensures data invariant: trip_id SET ↔ assignment_type SET
UPDATE receipts SET trip_id = NULL WHERE trip_id IS NOT NULL;
