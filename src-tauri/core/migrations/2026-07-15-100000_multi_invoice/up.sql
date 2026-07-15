-- Multi-invoice (Task 66): allow 1 Fuel + N Other invoices per trip.
-- BOTH rebuilds live in this single migration so the change is atomic —
-- a partial failure must roll back everything (see _tasks/66 test review C6).
--
-- NOTE: table rebuilds hold an exclusive lock for the whole copy. An external
-- connection open during the upgrade (DB browser, sqlite3 CLI) can cause
-- SQLITE_BUSY and abort startup — close external tools before updating.

-- ============================================================
-- Part 1: receipts — drop trip_id UNIQUE, add applied_amount_cents
-- ============================================================
-- applied_amount_cents: the exact amount (in cents) this receipt added to
-- trip.other_costs_eur at assign time. NULL = nothing applied (link-only or
-- legacy). Unassign subtracts THIS value, never the live total_price_eur,
-- which the user may edit after assigning (test review C7).
CREATE TABLE receipts_new (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT,
    trip_id TEXT,
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
    mismatch_override INTEGER NOT NULL DEFAULT 0,
    applied_amount_cents INTEGER,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id),
    FOREIGN KEY (trip_id) REFERENCES trips(id)
);

-- COALESCE on mismatch_override: the live column is nullable (added via
-- ALTER ... DEFAULT 0 without NOT NULL); a hand-edited/restored DB with a
-- NULL must not abort the migration (test review C3b). The rebuilt column is
-- NOT NULL, which also fixes the schema.rs drift.
INSERT INTO receipts_new (
    id, vehicle_id, trip_id, file_path, file_name, scanned_at, liters,
    total_price_eur, station_name, station_address, source_year, status,
    confidence, raw_ocr_text, error_message, created_at, updated_at,
    vendor_name, cost_description, original_amount, original_currency,
    receipt_datetime, assignment_type, mismatch_override, applied_amount_cents
)
SELECT
    id, vehicle_id, trip_id, file_path, file_name, scanned_at, liters,
    total_price_eur, station_name, station_address, source_year, status,
    confidence, raw_ocr_text, error_message, created_at, updated_at,
    vendor_name, cost_description, original_amount, original_currency,
    receipt_datetime, assignment_type, COALESCE(mismatch_override, 0),
    NULL  -- legacy assignments never subtract on unassign (today's behavior)
FROM receipts;

DROP TABLE receipts;
ALTER TABLE receipts_new RENAME TO receipts;

CREATE INDEX idx_receipts_status ON receipts(status);
CREATE INDEX idx_receipts_trip ON receipts(trip_id);
CREATE INDEX idx_receipts_vehicle ON receipts(vehicle_id);
CREATE INDEX idx_receipts_datetime ON receipts(receipt_datetime);
-- One Fuel receipt per trip; Other receipts unlimited.
CREATE UNIQUE INDEX idx_receipts_trip_fuel ON receipts(trip_id)
WHERE trip_id IS NOT NULL AND assignment_type = 'Fuel';

-- ============================================================
-- Part 2: paperless_trip_links — new PK, type + amount snapshots
-- ============================================================
-- New PK: paperless_document_id (one trip per doc — unchanged semantics).
-- assignment_type + amount_eur/title snapshots taken at assign time so the
-- grid's sum-mismatch check works offline (grid never calls Paperless).
CREATE TABLE paperless_trip_links_new (
    paperless_document_id INTEGER PRIMARY KEY,
    trip_id TEXT NOT NULL,
    assignment_type TEXT NOT NULL,
    amount_eur REAL,
    title TEXT,
    applied_amount_cents INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (trip_id) REFERENCES trips(id) ON DELETE CASCADE
);

-- Backfill heuristic (docs' data lives on the Paperless server, not locally):
-- 'Fuel' if the linked trip has fuel and no Fuel receipt already attached,
-- else 'Other'. NULL fuel_liters compares falsy -> 'Other'. amount_eur stays
-- NULL = unknown -> that trip is excluded from the sum-mismatch check (no
-- false warnings). applied_amount_cents = NULL (legacy links never subtract
-- on unassign). NOTE: this runs AFTER Part 1, reading the rebuilt receipts
-- table — same transaction, same data.
INSERT INTO paperless_trip_links_new (
    paperless_document_id, trip_id, assignment_type, amount_eur, title,
    applied_amount_cents, created_at, updated_at
)
SELECT
    l.paperless_document_id,
    l.trip_id,
    CASE WHEN EXISTS (
            SELECT 1 FROM trips t
            WHERE t.id = l.trip_id AND t.fuel_liters > 0
         )
         AND NOT EXISTS (
            SELECT 1 FROM receipts r
            WHERE r.trip_id = l.trip_id AND r.assignment_type = 'Fuel'
         )
    THEN 'Fuel' ELSE 'Other' END,
    NULL,
    NULL,
    NULL,
    l.created_at,
    l.updated_at
FROM paperless_trip_links l;

DROP TABLE paperless_trip_links;
ALTER TABLE paperless_trip_links_new RENAME TO paperless_trip_links;

CREATE INDEX idx_paperless_links_trip ON paperless_trip_links(trip_id);
CREATE UNIQUE INDEX idx_paperless_links_trip_fuel ON paperless_trip_links(trip_id)
WHERE assignment_type = 'Fuel';
