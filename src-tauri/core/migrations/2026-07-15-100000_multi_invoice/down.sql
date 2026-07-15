-- ============================================================
-- WARNING — LOSSY, NEVER EXECUTED IN PRACTICE (forward-only, ADR-012;
-- no diesel CLI revert exists in this repo or CI).
-- * paperless: collapsing to trip_id PRIMARY KEY keeps ONE arbitrary link
--   per trip (MIN(doc_id)) and discards the rest + all snapshots.
-- * receipts: restoring trip_id UNIQUE FAILS OUTRIGHT if any trip holds
--   more than one receipt; applied_amount_cents is discarded.
-- ============================================================

-- ============================================================
-- Part 1 (reverse of up Part 2): paperless_trip_links back to trip_id PK
-- ============================================================
CREATE TABLE paperless_trip_links_old (
    trip_id TEXT PRIMARY KEY,
    paperless_document_id INTEGER NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (trip_id) REFERENCES trips(id) ON DELETE CASCADE
);

INSERT INTO paperless_trip_links_old (
    trip_id, paperless_document_id, created_at, updated_at
)
SELECT trip_id, MIN(paperless_document_id), MIN(created_at), MIN(updated_at)
FROM paperless_trip_links
GROUP BY trip_id;

DROP TABLE paperless_trip_links;
ALTER TABLE paperless_trip_links_old RENAME TO paperless_trip_links;

CREATE INDEX idx_paperless_links_doc ON paperless_trip_links(paperless_document_id);

-- ============================================================
-- Part 2 (reverse of up Part 1): receipts back to trip_id UNIQUE,
-- without applied_amount_cents
-- ============================================================
CREATE TABLE receipts_old (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT,
    trip_id TEXT UNIQUE,
    file_path TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    scanned_at TEXT NOT NULL,
    liters REAL,
    total_price_eur REAL,
    station_name TEXT,
    station_address TEXT,
    source_year INTEGER,
    status TEXT NOT NULL DEFAULT 'Pending',
    confidence TEXT NOT NULL DEFAULT '{"liters":"Unknown","totalPrice":"Unknown","date":"Unknown"}',
    raw_ocr_text TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    vendor_name TEXT,
    cost_description TEXT,
    original_amount REAL DEFAULT NULL,
    original_currency TEXT DEFAULT NULL,
    receipt_datetime TEXT DEFAULT NULL,
    assignment_type TEXT DEFAULT NULL,
    mismatch_override INTEGER DEFAULT 0,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id),
    FOREIGN KEY (trip_id) REFERENCES trips(id)
);

INSERT INTO receipts_old (
    id, vehicle_id, trip_id, file_path, file_name, scanned_at, liters,
    total_price_eur, station_name, station_address, source_year, status,
    confidence, raw_ocr_text, error_message, created_at, updated_at,
    vendor_name, cost_description, original_amount, original_currency,
    receipt_datetime, assignment_type, mismatch_override
)
SELECT
    id, vehicle_id, trip_id, file_path, file_name, scanned_at, liters,
    total_price_eur, station_name, station_address, source_year, status,
    confidence, raw_ocr_text, error_message, created_at, updated_at,
    vendor_name, cost_description, original_amount, original_currency,
    receipt_datetime, assignment_type, mismatch_override
FROM receipts;

DROP TABLE receipts;
ALTER TABLE receipts_old RENAME TO receipts;

CREATE INDEX idx_receipts_status ON receipts(status);
CREATE INDEX idx_receipts_trip ON receipts(trip_id);
CREATE INDEX idx_receipts_vehicle ON receipts(vehicle_id);
CREATE INDEX idx_receipts_datetime ON receipts(receipt_datetime);
