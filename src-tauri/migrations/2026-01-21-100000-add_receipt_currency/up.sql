-- Add multi-currency support for receipts
-- Stores original amount and currency from OCR, separate from EUR value

ALTER TABLE receipts ADD COLUMN original_amount REAL DEFAULT NULL;
ALTER TABLE receipts ADD COLUMN original_currency TEXT DEFAULT NULL;
