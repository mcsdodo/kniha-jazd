-- Revert backfill (clear the auto-populated values)
-- Note: This loses data - only use if you need to truly revert
UPDATE receipts
SET original_amount = NULL,
    original_currency = NULL
WHERE original_currency = 'EUR';
