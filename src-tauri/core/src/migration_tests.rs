//! Data-integrity tests for the multi-invoice migration
//! (`2026-07-15-100000_multi_invoice`) — Task 66.
//!
//! Every test opens a LEGACY-schema in-memory DB (all migrations up to but
//! excluding the multi-invoice one), seeds legacy-shaped rows via raw SQL
//! (the legacy schema has no Rust structs anymore), runs the remaining
//! migrations, and asserts the data survived intact. Any failure here is a
//! REAL migration bug — fix the SQL, never the test.

use super::*;
use diesel::sql_types::{BigInt, Double, Integer, Nullable, Text};

// ============================================================================
// Raw-SQL helpers (legacy schema — no Rust structs exist for it)
// ============================================================================

fn exec(db: &Database, sql: &str) {
    let conn = &mut *db.connection();
    diesel::sql_query(sql)
        .execute(conn)
        .unwrap_or_else(|e| panic!("SQL failed: {e}\n{sql}"));
}

fn seed_vehicle(db: &Database, id: &str) {
    exec(
        db,
        &format!(
            "INSERT INTO vehicles (id, name, license_plate, created_at, updated_at) \
             VALUES ('{id}', 'Test Vehicle', 'BA123XY', \
                     '2026-01-01T00:00:00', '2026-01-01T00:00:00')"
        ),
    );
}

fn seed_trip(db: &Database, id: &str, vehicle_id: &str, fuel_liters: Option<f64>) {
    let fuel = fuel_liters.map_or("NULL".to_string(), |v| v.to_string());
    exec(
        db,
        &format!(
            "INSERT INTO trips (id, vehicle_id, origin, destination, distance_km, odometer, \
                                purpose, fuel_liters, start_datetime, end_datetime, \
                                created_at, updated_at) \
             VALUES ('{id}', '{vehicle_id}', 'BA', 'TT', 50.0, 12345.0, 'test', {fuel}, \
                     '2026-01-01T08:00:00', '2026-01-01T10:00:00', \
                     '2026-01-01T00:00:00', '2026-01-01T00:00:00')"
        ),
    );
}

/// Minimal legacy receipt: only NOT-NULL-without-default columns filled,
/// plus optional trip assignment. Everything else takes column defaults.
fn seed_minimal_receipt(
    db: &Database,
    id: &str,
    trip_id: Option<&str>,
    assignment_type: Option<&str>,
) {
    let trip = trip_id.map_or("NULL".to_string(), |t| format!("'{t}'"));
    let atype = assignment_type.map_or("NULL".to_string(), |a| format!("'{a}'"));
    exec(
        db,
        &format!(
            "INSERT INTO receipts (id, trip_id, file_path, file_name, scanned_at, \
                                   created_at, updated_at, assignment_type) \
             VALUES ('{id}', {trip}, '/scans/{id}.jpg', '{id}.jpg', '2026-03-01T10:00:00', \
                     '2026-03-01T10:00:00', '2026-03-01T10:00:00', {atype})"
        ),
    );
}

/// Seed a row that violates referential integrity (hand-edited/restored DB
/// shape). The bundled SQLite enforces foreign keys on every connection
/// (libsqlite3-sys builds with SQLITE_DEFAULT_FOREIGN_KEYS=1), so integrity
/// checking is suspended for the insert and restored right after — the
/// migration itself must run with FKs ON, exactly like production.
fn exec_with_fk_off(db: &Database, sql: &str) {
    exec(db, "PRAGMA foreign_keys = OFF");
    exec(db, sql);
    exec(db, "PRAGMA foreign_keys = ON");
}

fn seed_paperless_link(db: &Database, doc_id: i64, trip_id: &str) {
    exec(
        db,
        &format!(
            "INSERT INTO paperless_trip_links (trip_id, paperless_document_id, \
                                               created_at, updated_at) \
             VALUES ('{trip_id}', {doc_id}, '2026-04-01T08:00:00', '2026-04-02T09:00:00')"
        ),
    );
}

#[derive(diesel::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    cnt: i64,
}

/// `sql` must project a single column aliased `cnt`.
fn count(db: &Database, sql: &str) -> i64 {
    let conn = &mut *db.connection();
    diesel::sql_query(sql)
        .get_result::<CountRow>(conn)
        .unwrap_or_else(|e| panic!("SQL failed: {e}\n{sql}"))
        .cnt
}

#[derive(diesel::QueryableByName)]
struct TextRow {
    #[diesel(sql_type = Text)]
    value: String,
}

/// `sql` must project a single TEXT column aliased `value`.
fn text_values(db: &Database, sql: &str) -> Vec<String> {
    let conn = &mut *db.connection();
    diesel::sql_query(sql)
        .load::<TextRow>(conn)
        .unwrap_or_else(|e| panic!("SQL failed: {e}\n{sql}"))
        .into_iter()
        .map(|r| r.value)
        .collect()
}

// ============================================================================
// Receipts: row/column preservation
// ============================================================================

/// Every legacy receipts column, snapshot-able both before and after the
/// migration (the column set is a strict subset of the rebuilt table).
#[derive(diesel::QueryableByName, Debug, Clone, PartialEq)]
struct ReceiptSnapshot {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Nullable<Text>)]
    vehicle_id: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    trip_id: Option<String>,
    #[diesel(sql_type = Text)]
    file_path: String,
    #[diesel(sql_type = Text)]
    file_name: String,
    #[diesel(sql_type = Text)]
    scanned_at: String,
    #[diesel(sql_type = Nullable<Double>)]
    liters: Option<f64>,
    #[diesel(sql_type = Nullable<Double>)]
    total_price_eur: Option<f64>,
    #[diesel(sql_type = Nullable<Text>)]
    station_name: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    station_address: Option<String>,
    #[diesel(sql_type = Nullable<Integer>)]
    source_year: Option<i32>,
    #[diesel(sql_type = Text)]
    status: String,
    #[diesel(sql_type = Text)]
    confidence: String,
    #[diesel(sql_type = Nullable<Text>)]
    raw_ocr_text: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    error_message: Option<String>,
    #[diesel(sql_type = Text)]
    created_at: String,
    #[diesel(sql_type = Text)]
    updated_at: String,
    #[diesel(sql_type = Nullable<Text>)]
    vendor_name: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    cost_description: Option<String>,
    #[diesel(sql_type = Nullable<Double>)]
    original_amount: Option<f64>,
    #[diesel(sql_type = Nullable<Text>)]
    original_currency: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    receipt_datetime: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    assignment_type: Option<String>,
    #[diesel(sql_type = Nullable<Integer>)]
    mismatch_override: Option<i32>,
}

const RECEIPT_SNAPSHOT_SELECT: &str =
    "SELECT id, vehicle_id, trip_id, file_path, file_name, scanned_at, liters, \
            total_price_eur, station_name, station_address, source_year, status, \
            confidence, raw_ocr_text, error_message, created_at, updated_at, \
            vendor_name, cost_description, original_amount, original_currency, \
            receipt_datetime, assignment_type, mismatch_override \
     FROM receipts ORDER BY id";

fn snapshot_receipts(db: &Database) -> Vec<ReceiptSnapshot> {
    let conn = &mut *db.connection();
    diesel::sql_query(RECEIPT_SNAPSHOT_SELECT)
        .load(conn)
        .expect("receipt snapshot query")
}

#[test]
fn test_receipts_migration_preserves_every_row_and_column() {
    let db = open_db_legacy();
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t-fuel", "v1", Some(40.5));
    seed_trip(&db, "t-other", "v1", None);

    // r1: assigned Fuel — every single column populated
    exec(
        &db,
        r#"INSERT INTO receipts (id, vehicle_id, trip_id, file_path, file_name, scanned_at,
               liters, total_price_eur, station_name, station_address, source_year, status,
               confidence, raw_ocr_text, error_message, created_at, updated_at, vendor_name,
               cost_description, original_amount, original_currency, receipt_datetime,
               assignment_type, mismatch_override)
           VALUES ('r1-fuel', 'v1', 't-fuel', '/scans/r1.jpg', 'r1.jpg', '2026-03-01T10:00:00',
               40.5, 62.99, 'Slovnaft', 'Hlavna 1, Bratislava', 2026, 'Parsed',
               '{"liters":"High","totalPrice":"High","date":"High"}',
               'NATURAL 95   40.5 l', 'parse warning kept verbatim', '2026-03-01T10:05:00',
               '2026-03-01T10:06:00', 'Slovnaft a.s.', 'fuel fill-up', 1580.25, 'CZK',
               '2026-03-01T09:55:00', 'Fuel', 1)"#,
    );
    // r2: assigned Other
    exec(
        &db,
        r#"INSERT INTO receipts (id, vehicle_id, trip_id, file_path, file_name, scanned_at,
               total_price_eur, status, created_at, updated_at, vendor_name, cost_description,
               receipt_datetime, assignment_type)
           VALUES ('r2-other', 'v1', 't-other', '/scans/r2.jpg', 'r2.jpg', '2026-03-02T11:00:00',
               12.34, 'Parsed', '2026-03-02T11:01:00', '2026-03-02T11:02:00',
               'NDS a.s.', 'dialnicna znamka', '2026-03-02T10:55:00', 'Other')"#,
    );
    // r3: unassigned (trip_id NULL) but otherwise populated
    exec(
        &db,
        r#"INSERT INTO receipts (id, vehicle_id, file_path, file_name, scanned_at, liters,
               total_price_eur, status, created_at, updated_at)
           VALUES ('r3-unassigned', 'v1', '/scans/r3.jpg', 'r3.jpg', '2026-03-03T12:00:00',
               33.3, 51.20, 'Parsed', '2026-03-03T12:01:00', '2026-03-03T12:02:00')"#,
    );
    // r4: every optional column NULL (defaults for status/confidence/mismatch_override)
    seed_minimal_receipt(&db, "r4-nulls", None, None);
    // r5: orphaned trip_id — trip deleted behind SQLite's back (hand-edited DB)
    exec_with_fk_off(
        &db,
        "INSERT INTO receipts (id, trip_id, file_path, file_name, scanned_at, \
                               created_at, updated_at, assignment_type, total_price_eur) \
         VALUES ('r5-orphan', 'trip-deleted', '/scans/r5.jpg', 'r5.jpg', '2026-03-05T10:00:00', \
                 '2026-03-05T10:00:00', '2026-03-05T10:00:00', 'Other', 9.99)",
    );
    // r6: non-ASCII text survives byte-identical
    exec(
        &db,
        r#"INSERT INTO receipts (id, file_path, file_name, scanned_at, status, raw_ocr_text,
               station_name, created_at, updated_at)
           VALUES ('r6-utf8', '/scans/r6.jpg', 'r6.jpg', '2026-03-06T09:00:00', 'Error',
               'Košice — čerpacia stanica č. 5, ľubovoľný text s diakritikou: žŕďočšťým €',
               'Šaľa', '2026-03-06T09:01:00', '2026-03-06T09:02:00')"#,
    );

    let before = snapshot_receipts(&db);
    assert_eq!(before.len(), 6, "seed sanity check");

    migrate_to_current(&db);

    // Expected: everything byte-identical, EXCEPT the orphaned receipt which
    // is healed to unassigned (trip_id + assignment_type NULLed) — with FKs
    // enforced (SQLITE_DEFAULT_FOREIGN_KEYS=1) preserving the dangling
    // pointer verbatim would abort the migration and brick startup.
    let expected: Vec<ReceiptSnapshot> = before
        .into_iter()
        .map(|mut r| {
            if r.id == "r5-orphan" {
                r.trip_id = None;
                r.assignment_type = None;
            }
            r
        })
        .collect();
    let after = snapshot_receipts(&db);
    assert_eq!(
        expected, after,
        "every legacy receipts row and column must survive the rebuild \
         byte-identical (orphans healed to unassigned, nothing else touched)"
    );
    // The new column starts NULL for ALL migrated rows (legacy assignments
    // never subtract on unassign — today's behavior).
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) AS cnt FROM receipts WHERE applied_amount_cents IS NULL"
        ),
        6,
        "applied_amount_cents must be NULL for every migrated receipt"
    );
}

// ============================================================================
// Paperless links: preservation
// ============================================================================

#[derive(diesel::QueryableByName, Debug, PartialEq)]
struct LinkSnapshot {
    #[diesel(sql_type = BigInt)]
    paperless_document_id: i64,
    #[diesel(sql_type = Text)]
    trip_id: String,
    #[diesel(sql_type = Nullable<Double>)]
    amount_eur: Option<f64>,
    #[diesel(sql_type = Nullable<Text>)]
    title: Option<String>,
    #[diesel(sql_type = Nullable<BigInt>)]
    applied_amount_cents: Option<i64>,
    #[diesel(sql_type = Text)]
    created_at: String,
    #[diesel(sql_type = Text)]
    updated_at: String,
}

#[test]
fn test_paperless_links_migration_preserves_rows() {
    let db = open_db_legacy();
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t1", "v1", None);
    seed_trip(&db, "t2", "v1", None);
    seed_paperless_link(&db, 101, "t1");
    exec(
        &db,
        "INSERT INTO paperless_trip_links (trip_id, paperless_document_id, created_at, updated_at) \
         VALUES ('t2', 202, '2026-04-03T10:00:00', '2026-04-04T11:00:00')",
    );

    migrate_to_current(&db);

    let rows: Vec<LinkSnapshot> = {
        let conn = &mut *db.connection();
        diesel::sql_query(
            "SELECT paperless_document_id, trip_id, amount_eur, title, applied_amount_cents, \
                    created_at, updated_at \
             FROM paperless_trip_links ORDER BY paperless_document_id",
        )
        .load(conn)
        .expect("link snapshot query")
    };
    assert_eq!(
        rows,
        vec![
            LinkSnapshot {
                paperless_document_id: 101,
                trip_id: "t1".into(),
                amount_eur: None,
                title: None,
                applied_amount_cents: None,
                created_at: "2026-04-01T08:00:00".into(),
                updated_at: "2026-04-02T09:00:00".into(),
            },
            LinkSnapshot {
                paperless_document_id: 202,
                trip_id: "t2".into(),
                amount_eur: None,
                title: None,
                applied_amount_cents: None,
                created_at: "2026-04-03T10:00:00".into(),
                updated_at: "2026-04-04T11:00:00".into(),
            },
        ],
        "doc ids, trip ids and timestamps preserved; snapshots start NULL"
    );
}

#[test]
fn test_paperless_links_migration_drops_orphaned_links() {
    // A link whose trip was deleted behind SQLite's back (hand-edited DB)
    // cannot be carried over: trip_id is NOT NULL so it cannot be healed
    // like an orphaned receipt, and copying it verbatim would fail the FK
    // check and brick startup. It is dropped — the same outcome its
    // ON DELETE CASCADE would have produced in-app. Healthy links survive.
    let db = open_db_legacy();
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t-alive", "v1", None);
    seed_paperless_link(&db, 401, "t-alive");
    exec_with_fk_off(
        &db,
        "INSERT INTO paperless_trip_links (trip_id, paperless_document_id, \
                                           created_at, updated_at) \
         VALUES ('trip-deleted', 402, '2026-04-05T08:00:00', '2026-04-05T08:00:00')",
    );

    migrate_to_current(&db);

    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) AS cnt FROM paperless_trip_links \
             WHERE paperless_document_id = 401"
        ),
        1,
        "healthy link must survive"
    );
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) AS cnt FROM paperless_trip_links \
             WHERE paperless_document_id = 402"
        ),
        0,
        "orphaned link must be dropped, not brick the migration"
    );
}

// ============================================================================
// Index recreation
// ============================================================================

#[test]
fn test_receipts_migration_recreates_indexes() {
    let db = open_db_legacy();
    migrate_to_current(&db);

    let names = text_values(
        &db,
        "SELECT name AS value FROM sqlite_master \
         WHERE type = 'index' AND tbl_name = 'receipts' AND name NOT LIKE 'sqlite_%' \
         ORDER BY name",
    );
    assert_eq!(
        names,
        vec![
            "idx_receipts_datetime",
            "idx_receipts_status",
            "idx_receipts_trip",
            "idx_receipts_trip_fuel",
            "idx_receipts_vehicle",
        ],
        "rebuild must recreate the LIVE index set (idx_receipts_datetime, not the \
         dead idx_receipts_date) plus the new partial unique index"
    );
}

// ============================================================================
// NULL mismatch_override tolerance (hand-edited / restored DBs)
// ============================================================================

#[test]
fn test_receipts_migration_tolerates_null_mismatch_override() {
    let db = open_db_legacy();
    // The legacy column is nullable (ALTER ... DEFAULT 0 without NOT NULL);
    // force a NULL as a hand-edited/restored DB would have.
    exec(
        &db,
        "INSERT INTO receipts (id, file_path, file_name, scanned_at, created_at, updated_at, \
                               mismatch_override) \
         VALUES ('r-null-override', '/scans/rno.jpg', 'rno.jpg', '2026-03-01T10:00:00', \
                 '2026-03-01T10:00:00', '2026-03-01T10:00:00', NULL)",
    );

    // Without COALESCE in the migration this panics (NOT NULL violation)
    // and would brick startup on a real DB.
    migrate_to_current(&db);

    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) AS cnt FROM receipts \
             WHERE id = 'r-null-override' AND mismatch_override = 0"
        ),
        1,
        "NULL mismatch_override must migrate to 0"
    );
}

// ============================================================================
// Paperless backfill heuristic
// ============================================================================

fn link_assignment_type(db: &Database, doc_id: i64) -> String {
    let values = text_values(
        db,
        &format!(
            "SELECT assignment_type AS value FROM paperless_trip_links \
             WHERE paperless_document_id = {doc_id}"
        ),
    );
    assert_eq!(values.len(), 1, "exactly one link expected for doc {doc_id}");
    values.into_iter().next().unwrap()
}

#[test]
fn test_backfill_fuel_when_trip_fueled_and_no_fuel_receipt() {
    let db = open_db_legacy();
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t-fueled", "v1", Some(45.0));
    seed_paperless_link(&db, 301, "t-fueled");

    migrate_to_current(&db);

    assert_eq!(link_assignment_type(&db, 301), "Fuel");
}

#[test]
fn test_backfill_other_when_fuel_receipt_already_attached() {
    let db = open_db_legacy();
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t-fueled", "v1", Some(45.0));
    seed_minimal_receipt(&db, "r-fuel", Some("t-fueled"), Some("Fuel"));
    seed_paperless_link(&db, 302, "t-fueled");

    migrate_to_current(&db);

    // Also proves no cross-source double-Fuel state exists after upgrade:
    // the receipt keeps the Fuel slot, the link demotes to Other.
    assert_eq!(link_assignment_type(&db, 302), "Other");
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) AS cnt FROM paperless_trip_links \
             WHERE trip_id = 't-fueled' AND assignment_type = 'Fuel'"
        ),
        0
    );
}

#[test]
fn test_backfill_other_when_fuel_liters_null_or_zero() {
    let db = open_db_legacy();
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t-null-fuel", "v1", None);
    seed_trip(&db, "t-zero-fuel", "v1", Some(0.0));
    seed_paperless_link(&db, 303, "t-null-fuel");
    seed_paperless_link(&db, 304, "t-zero-fuel");

    migrate_to_current(&db);

    // SQL `NULL > 0` is falsy -> Other; 0 > 0 is false -> Other.
    assert_eq!(link_assignment_type(&db, 303), "Other");
    assert_eq!(link_assignment_type(&db, 304), "Other");
}

// ============================================================================
// Atomicity: ONE migration directory = ONE transaction
// ============================================================================

#[test]
fn test_multi_invoice_migration_is_single_atomic_unit() {
    let db = open_db_legacy();
    let before = count(&db, "SELECT COUNT(*) AS cnt FROM __diesel_schema_migrations");

    migrate_to_current(&db);

    let after = count(&db, "SELECT COUNT(*) AS cnt FROM __diesel_schema_migrations");
    assert_eq!(
        after - before,
        1,
        "both table rebuilds must ship as exactly ONE migration (= one \
         transaction); two directories would allow a half-migrated DB"
    );
    // ...and that single unit rebuilt BOTH tables.
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) AS cnt FROM pragma_table_info('receipts') \
             WHERE name = 'applied_amount_cents'"
        ),
        1,
        "receipts must be rebuilt (applied_amount_cents present)"
    );
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) AS cnt FROM pragma_table_info('paperless_trip_links') \
             WHERE name = 'assignment_type'"
        ),
        1,
        "paperless_trip_links must be rebuilt (assignment_type present)"
    );
}

// ============================================================================
// Schema parity: legacy->migrated == fresh chain
// ============================================================================

#[derive(diesel::QueryableByName)]
struct SchemaRow {
    #[diesel(sql_type = Text)]
    kind: String,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    sql: String,
}

fn normalize_sql(sql: &str) -> String {
    sql.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// All user tables + indexes as (type, name, normalized DDL), skipping
/// sqlite_* internals (autoindexes/sequence have NULL or internal sql).
fn schema_entries(db: &Database) -> Vec<(String, String, String)> {
    let conn = &mut *db.connection();
    let rows: Vec<SchemaRow> = diesel::sql_query(
        "SELECT type AS kind, name, sql FROM sqlite_master \
         WHERE name NOT LIKE 'sqlite_%' AND sql IS NOT NULL \
         ORDER BY type, name",
    )
    .load(conn)
    .expect("sqlite_master query");
    rows.into_iter()
        .map(|r| (r.kind, r.name, normalize_sql(&r.sql)))
        .collect()
}

#[test]
fn test_migrated_schema_identical_to_fresh_schema() {
    let fresh = Database::in_memory().expect("fresh in-memory DB");

    let migrated = open_db_legacy();
    migrate_to_current(&migrated);

    assert_eq!(
        schema_entries(&fresh),
        schema_entries(&migrated),
        "a legacy DB migrated forward must end up with EXACTLY the schema a \
         fresh install gets — any diff is schema.rs/DDL drift"
    );
}
