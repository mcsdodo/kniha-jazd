-- Task 84: local receipts are removed; Paperless is the only invoice source.
-- This intentionally discards local receipt rows. Run
-- scripts/migrate_local_to_paperless.py BEFORE upgrading if you still need them.
-- Indexes drop with the table.
DROP TABLE receipts;
