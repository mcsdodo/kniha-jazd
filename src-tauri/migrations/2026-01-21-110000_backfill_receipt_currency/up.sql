-- Backfill existing receipts with EUR currency
-- All receipts processed before multi-currency support were EUR
UPDATE receipts
SET original_amount = total_price_eur,
    original_currency = 'EUR'
WHERE total_price_eur IS NOT NULL
  AND original_amount IS NULL;
