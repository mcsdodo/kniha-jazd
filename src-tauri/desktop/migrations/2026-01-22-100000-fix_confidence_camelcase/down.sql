-- Revert camelCase back to snake_case (not needed in production)
UPDATE receipts
SET confidence = REPLACE(confidence, '"totalPrice":', '"total_price":')
WHERE confidence LIKE '%"totalPrice":%';
