-- Fix confidence JSON field name from snake_case to camelCase
-- The Rust struct uses #[serde(rename_all = "camelCase")] since commit 0cb9e40
-- but existing receipts have "total_price" instead of "totalPrice"
UPDATE receipts
SET confidence = REPLACE(confidence, '"total_price":', '"totalPrice":')
WHERE confidence LIKE '%"total_price":%';
