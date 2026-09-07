-- Restores the columns but not their values: the numbers they held were wrong,
-- and recreating them accurately would mean recomputing from trips, which is
-- what dropping them made unnecessary.
ALTER TABLE routes ADD COLUMN usage_count INTEGER NOT NULL DEFAULT 1;
ALTER TABLE routes ADD COLUMN last_used TEXT NOT NULL DEFAULT '';
