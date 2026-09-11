//! Data-integrity tests for the multi-invoice migration
//! (`2026-07-15-100000_multi_invoice`) — Task 66.
//!
//! Every test opens a LEGACY-schema in-memory DB (all migrations up to but
//! excluding the multi-invoice one), seeds legacy-shaped rows via raw SQL
//! (the legacy schema has no Rust structs anymore), runs the remaining
//! migrations, and asserts the data survived intact. Any failure here is a
//! REAL migration bug — fix the SQL, never the test.

use super::*;
use crate::models::RouteMode;
use diesel::sql_types::{BigInt, Double, Nullable, Text};

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
    // cannot be carried over: trip_id is NOT NULL, and copying it verbatim
    // would fail the FK check and brick startup. It is dropped -- the same
    // outcome its ON DELETE CASCADE would have produced in-app. Healthy links
    // survive.
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

// ============================================================================
// Task 76 — dropping the stored route counters (2026-09-06-110000)
// ============================================================================

/// A database written before the counters were dropped must still produce
/// autocomplete suggestions afterwards, with counts that now reflect its trips
/// rather than the stored number the old write paths kept getting wrong — and
/// a row those trips no longer justify must stop being offered.
#[test]
fn dropping_the_route_counters_keeps_the_suggestions() {
    let db = open_db_legacy();
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t1", "v1", None);
    seed_trip(&db, "t2", "v1", None);
    // A legacy row carrying the kind of inflated counter this migration deletes.
    exec(
        &db,
        "INSERT INTO routes (id, vehicle_id, origin, destination, distance_km, \
                             usage_count, last_used) \
         VALUES ('r1', 'v1', 'BA', 'TT', 50.0, 126, '2026-01-01T00:00:00+00:00')",
    );
    // ...and an orphan of the kind production carries: no trip matches it.
    exec(
        &db,
        "INSERT INTO routes (id, vehicle_id, origin, destination, distance_km, \
                             usage_count, last_used) \
         VALUES ('r2', 'v1', '', '', 0.0, 7, '2026-01-01T00:00:00+00:00')",
    );

    migrate_to_current(&db);

    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) AS cnt FROM pragma_table_info('routes') \
             WHERE name IN ('usage_count', 'last_used')"
        ),
        0,
        "both stored counters must be gone from the table"
    );

    let routes = db.get_routes_for_vehicle("v1").unwrap();
    assert_eq!(routes.len(), 1, "the orphan row is no longer suggested");
    assert_eq!(routes[0].origin, "BA");
    assert_eq!(
        routes[0].usage_count, 2,
        "counted from the two trips, not from the stored 126"
    );
    assert_eq!(routes[0].distance_km, 50.0, "distance_km survives the drop");
}

// ============================================================================
// Task 72 -- persisting the route mode (2026-09-07-110000)
// ============================================================================

/// Every route saved by Task 70 IS a loop, so the DEFAULT backfills correctly
/// by construction. This test is what proves that claim rather than assuming it.
#[test]
fn existing_route_maps_become_loop_mode() {
    let db = open_db_legacy_before("2026-09-07-110000");
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t1", "v1", None);
    exec(
        &db,
        "INSERT INTO trip_routes (trip_id, waypoints, polyline, target_km, road_km, \
                                  dataset_version, created_at) \
         VALUES ('t1', '[]', 'abc', 100.0, 98.0, '2026-05-03', \
                 '2026-01-01T00:00:00+00:00')",
    );

    migrate_to_current(&db);

    let map = db.get_route_map("t1").unwrap().unwrap();
    assert_eq!(map.mode, RouteMode::Loop);
}

// ============================================================================
// Task 20 -- persisting the round-trip flag (2026-09-07-120000)
// ============================================================================

/// Every route saved before this migration is either a loop (already closed)
/// or a one-way direct route, so `DEFAULT 0` backfills correctly by
/// construction. This test is what proves that claim rather than assuming it.
#[test]
fn existing_route_maps_backfill_round_trip_false() {
    let db = open_db_legacy_before("2026-09-07-120000");
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t1", "v1", None);
    exec(
        &db,
        "INSERT INTO trip_routes (trip_id, waypoints, polyline, target_km, road_km, \
                                  dataset_version, created_at) \
         VALUES ('t1', '[]', 'abc', 100.0, 98.0, '2026-05-03', \
                 '2026-01-01T00:00:00+00:00')",
    );

    migrate_to_current(&db);

    let map = db.get_route_map("t1").unwrap().unwrap();
    assert!(
        !map.round_trip,
        "a route saved before round_trip existed must backfill to false"
    );
}

// ============================================================================
// Task 84 -- local receipts are removed (2026-09-11-130000)
// ============================================================================

/// The legacy chain still builds a `receipts` table for the multi-invoice
/// migration, then the drop migration removes it. A fresh install never creates
/// it. This is the only assertion that the table is really gone.
#[test]
fn receipts_table_is_dropped_by_the_migration_chain() {
    let db = open_db_legacy();
    migrate_to_current(&db);

    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) AS cnt FROM sqlite_master \
             WHERE type = 'table' AND name = 'receipts'"
        ),
        0,
        "the local receipts table must not survive the migration chain"
    );
}

// ============================================================================
// Task 84 -- repair links the multi-invoice backfill mislabelled
// (2026-09-11-125000)
// ============================================================================

/// Seed a legacy receipt row. The legacy schema still carries the receipt
/// columns the multi-invoice migration reads.
fn seed_receipt(db: &Database, id: &str, trip_id: &str, assignment_type: &str) {
    exec(
        db,
        &format!(
            "INSERT INTO receipts (id, vehicle_id, trip_id, file_path, file_name, \
                                    scanned_at, status, assignment_type, created_at, updated_at) \
             VALUES ('{id}', 'v1', '{trip_id}', '/tmp/{id}.pdf', '{id}.pdf', \
                     '2026-01-01T00:00:00', 'Parsed', '{assignment_type}', \
                     '2026-01-01T00:00:00', '2026-01-01T00:00:00')"
        ),
    );
}

/// A pre-Task-66 link on a fuel trip that still had a Fuel receipt was
/// backfilled to 'Other'. Task 84 must retype it to 'Fuel' before dropping the
/// receipts, or the trip loses its fuel coverage.
#[test]
fn legacy_fuel_receipt_link_is_retyped_to_fuel() {
    let db = open_db_legacy();
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t-fueled", "v1", Some(45.0));
    seed_paperless_link(&db, 301, "t-fueled");
    seed_receipt(&db, "r1", "t-fueled", "Fuel");

    migrate_to_current(&db);

    assert_eq!(
        link_assignment_type(&db, 301),
        "Fuel",
        "the backfill labelled a fuel document's link 'Other' only because a \
         Fuel receipt existed; the repair must retype it before the drop"
    );
}

/// A legacy link on a trip whose only receipt is an Other expense must stay
/// 'Other' -- the repair keys on a Fuel receipt, so it must not touch it.
#[test]
fn legacy_other_receipt_link_stays_other() {
    let db = open_db_legacy();
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t-other", "v1", None);
    seed_paperless_link(&db, 302, "t-other");
    seed_receipt(&db, "r2", "t-other", "Other");

    migrate_to_current(&db);

    assert_eq!(link_assignment_type(&db, 302), "Other");
}

/// The repair must not produce two Fuel links on one trip.
#[test]
fn repair_does_not_add_a_second_fuel_link() {
    let db = open_db_legacy();
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t-fueled", "v1", Some(45.0));
    seed_paperless_link(&db, 401, "t-fueled");
    seed_receipt(&db, "r1", "t-fueled", "Fuel");

    migrate_to_current(&db);

    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) AS cnt FROM paperless_trip_links \
             WHERE trip_id = 't-fueled' AND assignment_type = 'Fuel'"
        ),
        1
    );
}
