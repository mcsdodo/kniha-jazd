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

/// Same, on the pre-multi-invoice schema, with a caller-chosen `created_at`.
/// The old table stamps the time the user made the link, and the multi-invoice
/// backfill copies that value forward -- so a late upgrade produces backfilled
/// rows with a recent timestamp.
fn seed_paperless_link_at(db: &Database, doc_id: i64, trip_id: &str, created_at: &str) {
    exec(
        db,
        &format!(
            "INSERT INTO paperless_trip_links (trip_id, paperless_document_id, \
                                               created_at, updated_at) \
             VALUES ('{trip_id}', {doc_id}, '{created_at}', '{created_at}')"
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
// Task 85 -- per-country avoid list (2026-09-29-100000)
// ============================================================================

#[test]
fn existing_route_maps_backfill_an_empty_avoid_list() {
    let db = open_db_legacy_before("2026-09-29-100000");
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
    assert!(map.avoid.is_empty(), "a route saved before the avoid column must read as no avoid");
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
// (the UPDATE at the top of 2026-09-11-130000_drop_receipts)
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

/// The version the repair lives in. A test that needs to stand right before the
/// repair uses this cutoff: `multi_invoice` has run (so `assignment_type` exists
/// and the link table takes more than one row per trip), and `receipts` is still
/// there for the repair to read.
const DROP_RECEIPTS_VERSION: &str = "2026-09-11-130000";

/// Seed a BACKFILLED link on the post-multi-invoice schema, where
/// `assignment_type` is NOT NULL and a trip may carry several links. The INSERT
/// omits `title`, so it stays NULL -- that is what the backfill wrote and what
/// the repair keys on. `amount_eur` / `applied_amount_cents` stay NULL unless
/// given, the backfill's second marker. Use `seed_assigned_link` for a row the
/// app itself wrote.
#[allow(clippy::too_many_arguments)]
fn seed_typed_link(
    db: &Database,
    doc_id: i64,
    trip_id: &str,
    assignment_type: &str,
    created_at: &str,
    amount_eur: Option<f64>,
) {
    let amount = amount_eur.map_or("NULL".to_string(), |v| v.to_string());
    exec(
        db,
        &format!(
            "INSERT INTO paperless_trip_links (trip_id, paperless_document_id, \
                                               assignment_type, amount_eur, \
                                               created_at, updated_at) \
             VALUES ('{trip_id}', {doc_id}, '{assignment_type}', {amount}, \
                     '{created_at}', '{created_at}')"
        ),
    );
}

/// Two candidate links on one trip must not both be promoted: the partial
/// unique index allows a single Fuel link per trip, so the second promotion
/// would abort the whole upgrade with a UNIQUE violation and panic on startup.
///
/// This stands at the repair's own boundary. The pre-multi-invoice link table
/// had `trip_id` as PRIMARY KEY and could not hold two links for one trip, so
/// seeding this shape needs the rebuilt table.
#[test]
fn repair_does_not_add_a_second_fuel_link() {
    let db = open_db_legacy_before(DROP_RECEIPTS_VERSION);
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t-fueled", "v1", Some(45.0));
    seed_typed_link(&db, 402, "t-fueled", "Other", "2026-04-01T08:00:00", None);
    seed_typed_link(&db, 401, "t-fueled", "Other", "2026-04-02T08:00:00", None);
    seed_receipt(&db, "r1", "t-fueled", "Fuel");

    // Must not panic: a UNIQUE violation here aborts every upgrade.
    migrate_to_current(&db);

    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) AS cnt FROM paperless_trip_links \
             WHERE trip_id = 't-fueled' AND assignment_type = 'Fuel'"
        ),
        1,
        "exactly one link may be promoted per trip"
    );
    assert_eq!(
        link_assignment_type(&db, 401),
        "Fuel",
        "the lowest document id wins, so the choice is deterministic"
    );
    assert_eq!(link_assignment_type(&db, 402), "Other");
}

/// A trip carrying BOTH a Fuel and an Other receipt is ambiguous: the old
/// relink script inserted one link per trip, and it may have been the Other
/// document. Promoting it would file a parking or toll document as the trip's
/// fuel invoice, so the repair must skip the trip entirely.
#[test]
fn repair_skips_trip_with_ambiguous_other_receipt() {
    let db = open_db_legacy_before(DROP_RECEIPTS_VERSION);
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t-both", "v1", Some(45.0));
    seed_typed_link(&db, 403, "t-both", "Other", "2026-04-01T08:00:00", None);
    seed_receipt(&db, "r-fuel", "t-both", "Fuel");
    seed_receipt(&db, "r-other", "t-both", "Other");

    migrate_to_current(&db);

    assert_eq!(
        link_assignment_type(&db, 403),
        "Other",
        "the link may be the Other document; the repair must not guess"
    );
}

/// The repair's main safety property: a link assigned after Task 66 carries an
/// explicit type and amount snapshots. It is a deliberate user choice and must
/// never be retyped, even on a trip that otherwise matches.
#[test]
fn repair_skips_link_with_post_task66_snapshots() {
    let db = open_db_legacy_before(DROP_RECEIPTS_VERSION);
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t-snap", "v1", Some(45.0));
    seed_typed_link(&db, 404, "t-snap", "Other", "2026-04-01T08:00:00", Some(12.34));
    seed_receipt(&db, "r1", "t-snap", "Fuel");

    migrate_to_current(&db);

    assert_eq!(
        link_assignment_type(&db, 404),
        "Other",
        "an amount snapshot marks a deliberate post-Task-66 assignment"
    );
}

/// Seed a link the APP wrote, as `upsert_paperless_link` writes it: with a
/// title. The title is the marker that keeps the repair off it, whatever its
/// amounts are -- a document Paperless read no amount from leaves both amount
/// columns NULL, exactly like the backfill.
fn seed_assigned_link(
    db: &Database,
    doc_id: i64,
    trip_id: &str,
    assignment_type: &str,
    created_at: &str,
    title: &str,
) {
    exec(
        db,
        &format!(
            "INSERT INTO paperless_trip_links (trip_id, paperless_document_id, \
                                               assignment_type, title, \
                                               created_at, updated_at) \
             VALUES ('{trip_id}', {doc_id}, '{assignment_type}', '{title}', \
                     '{created_at}', '{created_at}')"
        ),
    );
}

/// A backfilled link carries the timestamp of the day the USER made it, not of
/// the day the backfill ran: the multi-invoice migration copies `l.created_at`
/// forward. So its `created_at` says nothing about which rows the backfill
/// touched, and a repair keyed on a calendar cutoff misses every row on a
/// database that upgrades late. This is that database, end to end: the link is
/// made on the old schema in August, and both migrations run afterwards.
#[test]
fn delayed_upgrade_still_repairs_a_backfilled_link() {
    let db = open_db_legacy();
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t-late-upgrade", "v1", Some(45.0));
    seed_paperless_link_at(&db, 405, "t-late-upgrade", "2026-08-20T10:00:00");
    seed_receipt(&db, "r1", "t-late-upgrade", "Fuel");

    migrate_to_current(&db);

    assert_eq!(
        link_assignment_type(&db, 405),
        "Fuel",
        "the backfill mislabelled this link whatever its created_at says; \
         dropping the receipts without the repair loses the trip's fuel coverage"
    );
}

/// The repair's main safety property, second half: a link the app wrote must
/// never be retyped, even with no amount snapshots (the document carried no
/// amount) and even on a trip that otherwise matches. The title separates it
/// from a backfilled row.
#[test]
fn repair_skips_an_app_written_link_without_amounts() {
    let db = open_db_legacy_before(DROP_RECEIPTS_VERSION);
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t-late", "v1", Some(45.0));
    seed_assigned_link(&db, 406, "t-late", "Other", "2026-08-01T08:00:00", "Parkovanie");
    seed_receipt(&db, "r1", "t-late", "Fuel");

    migrate_to_current(&db);

    assert_eq!(
        link_assignment_type(&db, 406),
        "Other",
        "a title marks a deliberate assignment by the app"
    );
}

// ============================================================================
// Task 86 -- the provider a route was computed with (2026-09-29-110000)
// ============================================================================

#[test]
fn existing_route_maps_backfill_the_provider_they_can_be_proven_to_use() {
    let db = open_db_legacy_before("2026-09-29-110000");
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t1", "v1", None);
    seed_trip(&db, "t2", "v1", None);
    seed_trip(&db, "t3", "v1", None);
    // Before the first Sygic commit: no code could have used Sygic.
    exec(
        &db,
        "INSERT INTO trip_routes (trip_id, waypoints, polyline, target_km, road_km, \
                                  dataset_version, created_at, avoid) \
         VALUES ('t1', '[]', 'abc', 100.0, 98.0, NULL, \
                 '2026-09-10T14:21:06.645630932+00:00', '[]')",
    );
    // An avoid list needs Sygic.
    exec(
        &db,
        "INSERT INTO trip_routes (trip_id, waypoints, polyline, target_km, road_km, \
                                  dataset_version, created_at, avoid) \
         VALUES ('t2', '[]', 'abc', 100.0, 98.0, NULL, \
                 '2026-09-29T13:42:21.677781185+00:00', '[\"cze:tolls\"]')",
    );
    // After the Sygic commit, no avoid list: either provider, so unknown.
    exec(
        &db,
        "INSERT INTO trip_routes (trip_id, waypoints, polyline, target_km, road_km, \
                                  dataset_version, created_at, avoid) \
         VALUES ('t3', '[]', 'abc', 100.0, 98.0, NULL, \
                 '2026-09-29T12:00:00+00:00', '[]')",
    );

    migrate_to_current(&db);

    use crate::route_map::RouteProviderKind;
    assert_eq!(db.get_route_map("t1").unwrap().unwrap().provider, Some(RouteProviderKind::Osrm));
    assert_eq!(db.get_route_map("t2").unwrap().unwrap().provider, Some(RouteProviderKind::Sygic));
    assert_eq!(db.get_route_map("t3").unwrap().unwrap().provider, None);
}

// ============================================================================
// Task 88 -- kj_normalise is available to SQL on every connection
// ============================================================================

#[derive(diesel::QueryableByName)]
struct KjRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    v: String,
}

fn kj(db: &Database, input: &str) -> String {
    let conn = &mut *db.connection();
    diesel::sql_query("SELECT kj_normalise(?) AS v")
        .bind::<diesel::sql_types::Text, _>(input)
        .get_result::<KjRow>(conn)
        .expect("kj_normalise must be registered")
        .v
}

#[test]
fn kj_normalise_matches_the_rust_fold_on_a_fresh_db() {
    let db = Database::in_memory().unwrap();
    assert_eq!(kj(&db, "  Hlavná 5,   Žilina "), crate::places::normalise("  Hlavná 5,   Žilina "));
    assert_eq!(kj(&db, "Košice"), "kosice");
}

#[test]
fn kj_normalise_is_registered_on_a_legacy_db() {
    let db = open_db_legacy_before("2026-09-07");
    assert_eq!(kj(&db, "Trenčín"), "trencin");
}

#[test]
fn restore_registers_kj_normalise() {
    let dir = tempfile::tempdir().unwrap();
    let live = dir.path().join("live.db");
    let backup = dir.path().join("backup.db");
    let db = Database::new(live.clone()).unwrap();
    std::fs::copy(&live, &backup).unwrap();
    db.restore_from_file(&backup, &live).unwrap();
    assert_eq!(kj(&db, "Žilina"), "zilina");
}

// ============================================================================
// Task 88 -- places as entities (2026-10-05-100000)
// ============================================================================

const PLACES_AS_ENTITIES: &str = "2026-10-05-100000";

fn seed_trip_at(db: &Database, id: &str, vehicle: &str, origin: &str, dest: &str, start: &str) {
    exec(
        db,
        &format!(
            "INSERT INTO trips (id, vehicle_id, origin, destination, distance_km, odometer, \
             purpose, created_at, updated_at, start_datetime) VALUES \
             ('{id}', '{vehicle}', '{origin}', '{dest}', 10.0, 1000.0, 'p', \
              '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00', '{start}')"
        ),
    );
}

#[derive(diesel::QueryableByName, Debug, PartialEq)]
struct PlaceView {
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    normalised_name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Double>)]
    lat: Option<f64>,
}

fn places_view(db: &Database) -> Vec<PlaceView> {
    let conn = &mut *db.connection();
    diesel::sql_query("SELECT name, normalised_name, lat FROM places ORDER BY normalised_name")
        .load(conn)
        .unwrap()
}

#[derive(diesel::QueryableByName)]
struct TripNames {
    #[diesel(sql_type = diesel::sql_types::Text)]
    id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    o: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    d: String,
}

fn trip_names(db: &Database) -> Vec<(String, String, String)> {
    let conn = &mut *db.connection();
    diesel::sql_query(
        "SELECT t.id, po.name AS o, pd.name AS d FROM trips t \
         JOIN places po ON po.id = t.origin_place_id \
         JOIN places pd ON pd.id = t.destination_place_id ORDER BY t.id",
    )
    .load::<TripNames>(conn)
    .unwrap()
    .into_iter()
    .map(|r| (r.id, r.o, r.d))
    .collect()
}

#[test]
fn spellings_fold_into_one_place_named_by_the_most_used_spelling() {
    let db = open_db_legacy_before(PLACES_AS_ENTITIES);
    seed_vehicle(&db, "v1");
    seed_trip_at(&db, "t1", "v1", "Košice", "Hlavná 5, Žilina", "2026-01-01T08:00:00");
    seed_trip_at(&db, "t2", "v1", "Kosice", "Hlavna 5, Zilina", "2026-01-02T08:00:00");
    seed_trip_at(&db, "t3", "v1", "Kosice", "Hlavná 5, Žilina", "2026-01-03T08:00:00");
    exec(&db, "INSERT INTO places (normalised_name, display_name, lat, lon, source) \
               VALUES ('kosice', 'Kosice', 48.7, 21.2, 'manual')");

    migrate_to_current(&db);

    let places = places_view(&db);
    assert_eq!(places.len(), 2, "two keys, two places: {places:?}");
    // "Kosice" has 2 uses, "Košice" 1: the most-used spelling wins.
    assert_eq!(places[0].name, "Hlavná 5, Žilina");
    assert_eq!(places[1].name, "Kosice");
    assert_eq!(places[1].lat, Some(48.7), "the old coordinate moves across");
    // Each trip now points at the folded place.
    for (_, o, _) in trip_names(&db) {
        assert_eq!(o, "Kosice");
    }
}

#[test]
fn a_tie_goes_to_the_byte_wise_smaller_spelling() {
    let db = open_db_legacy_before(PLACES_AS_ENTITIES);
    seed_vehicle(&db, "v1");
    seed_trip_at(&db, "t1", "v1", "Trnava", "trnava", "2026-01-01T08:00:00");
    migrate_to_current(&db);
    // One use each. "Trnava" < "trnava" byte-wise ('T' = 0x54 < 't' = 0x74).
    assert_eq!(places_view(&db)[0].name, "Trnava");
}

#[test]
fn a_trip_string_without_coordinates_becomes_a_place_with_null_coordinates() {
    let db = open_db_legacy_before(PLACES_AS_ENTITIES);
    seed_vehicle(&db, "v1");
    seed_trip_at(&db, "t1", "v1", "Nitra", "Nitra", "2026-01-01T08:00:00");
    migrate_to_current(&db);
    let places = places_view(&db);
    assert_eq!(places.len(), 1);
    assert_eq!(places[0].lat, None);
}

#[test]
fn an_unused_old_place_row_stays_a_place() {
    let db = open_db_legacy_before(PLACES_AS_ENTITIES);
    exec(&db, "INSERT INTO places (normalised_name, display_name, lat, lon, source) \
               VALUES ('senec', 'Senec', 48.2, 17.4, 'geocoder')");
    migrate_to_current(&db);
    assert_eq!(places_view(&db), vec![PlaceView {
        name: "Senec".into(), normalised_name: "senec".into(), lat: Some(48.2),
    }]);
}

#[test]
fn an_empty_endpoint_maps_to_the_unknown_place_and_the_trip_survives() {
    let db = open_db_legacy_before(PLACES_AS_ENTITIES);
    seed_vehicle(&db, "v1");
    seed_trip_at(&db, "t1", "v1", "   ", "Nitra", "2026-01-01T08:00:00");
    migrate_to_current(&db);
    assert_eq!(trip_names(&db), vec![("t1".into(), "Neznáme miesto".into(), "Nitra".into())]);
}

#[derive(diesel::QueryableByName)]
struct RouteView {
    #[diesel(sql_type = diesel::sql_types::Text)]
    id: String,
    #[diesel(sql_type = diesel::sql_types::Double)]
    distance_km: f64,
}

fn route_rows(db: &Database) -> Vec<(String, f64)> {
    let conn = &mut *db.connection();
    diesel::sql_query("SELECT id, distance_km FROM routes ORDER BY id")
        .load::<RouteView>(conn)
        .unwrap()
        .into_iter()
        .map(|r| (r.id, r.distance_km))
        .collect()
}

#[test]
fn routes_that_collapse_keep_the_row_the_latest_trip_used() {
    let db = open_db_legacy_before(PLACES_AS_ENTITIES);
    seed_vehicle(&db, "v1");
    seed_trip_at(&db, "t1", "v1", "Kosice", "Presov", "2026-01-01T08:00:00");
    seed_trip_at(&db, "t2", "v1", "Košice", "Prešov", "2026-02-01T08:00:00");
    exec(&db, "INSERT INTO routes (id, vehicle_id, origin, destination, distance_km) VALUES \
               ('r-a', 'v1', 'Kosice', 'Presov', 36.0), \
               ('r-b', 'v1', 'Košice', 'Prešov', 37.0)");
    migrate_to_current(&db);
    // t2 is the latest trip and used the spelling of r-b.
    assert_eq!(route_rows(&db), vec![("r-b".to_string(), 37.0)]);
}

#[test]
fn a_route_no_trip_uses_is_dropped() {
    let db = open_db_legacy_before(PLACES_AS_ENTITIES);
    seed_vehicle(&db, "v1");
    seed_trip_at(&db, "t1", "v1", "Nitra", "Levice", "2026-01-01T08:00:00");
    exec(&db, "INSERT INTO routes (id, vehicle_id, origin, destination, distance_km) VALUES \
               ('r-used', 'v1', 'Nitra', 'Levice', 40.0), \
               ('r-orphan', 'v1', 'Nowhere', 'Levice', 99.0)");
    migrate_to_current(&db);
    assert_eq!(route_rows(&db), vec![("r-used".to_string(), 40.0)]);
}

#[test]
fn trip_children_survive_the_trips_rebuild() {
    let db = open_db_legacy_before(PLACES_AS_ENTITIES);
    seed_vehicle(&db, "v1");
    seed_trip_at(&db, "t1", "v1", "Nitra", "Levice", "2026-01-01T08:00:00");
    exec(&db, "INSERT INTO trip_routes (trip_id, waypoints, polyline, target_km, road_km, \
               dataset_version, created_at, avoid) VALUES \
               ('t1', '[]', 'abc', 40.0, 41.0, NULL, '2026-01-01T00:00:00+00:00', '[]')");
    exec(&db, "INSERT INTO paperless_trip_links (paperless_document_id, trip_id, assignment_type, \
               amount_eur, title, applied_amount_cents, created_at, updated_at) VALUES \
               (7, 't1', 'Fuel', 50.0, 'doc', 5000, '2026-01-01T00:00:00+00:00', \
                '2026-01-01T00:00:00+00:00')");
    migrate_to_current(&db);
    assert!(db.get_route_map("t1").unwrap().is_some(), "DROP TABLE trips must not cascade");
    assert_eq!(db.get_paperless_links_for_trip("t1").unwrap().len(), 1,
        "the Paperless link must survive the trips rebuild");
}

#[test]
fn down_sql_gives_back_the_trip_strings() {
    let db = open_db_legacy_before(PLACES_AS_ENTITIES);
    seed_vehicle(&db, "v1");
    seed_trip_at(&db, "t1", "v1", "Nitra", "Levice", "2026-01-01T08:00:00");
    exec(&db, "INSERT INTO trip_routes (trip_id, waypoints, polyline, target_km, road_km, \
               dataset_version, created_at, avoid) VALUES \
               ('t1', '[]', 'abc', 40.0, 41.0, NULL, '2026-01-01T00:00:00+00:00', '[]')");
    migrate_to_current(&db);
    {
        // Revert down to (and including) this migration, so a later
        // migration on top does not break the test.
        let conn = &mut *db.connection();
        loop {
            let reverted = conn.revert_last_migration(crate::db::MIGRATIONS).unwrap();
            if reverted.to_string().replace('-', "").starts_with(&PLACES_AS_ENTITIES.replace('-', "")) {
                break;
            }
        }
    }
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Text)]
        origin: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        destination: String,
    }
    let rows: Vec<Row> = {
        let conn = &mut *db.connection();
        diesel::sql_query("SELECT origin, destination FROM trips").load(conn).unwrap()
    };
    assert_eq!((rows[0].origin.as_str(), rows[0].destination.as_str()), ("Nitra", "Levice"));
    assert!(db.get_route_map("t1").unwrap().is_some(), "the revert must keep the route map");
}

#[test]
fn restore_of_an_old_backup_runs_the_places_migration() {
    let dir = tempfile::tempdir().unwrap();
    let live = dir.path().join("live.db");
    let backup = dir.path().join("backup.db");

    let legacy = open_db_legacy_before(PLACES_AS_ENTITIES);
    seed_vehicle(&legacy, "v1");
    seed_trip_at(&legacy, "t1", "v1", "Nitra", "Levice", "2026-01-01T08:00:00");
    exec(&legacy, &format!("VACUUM INTO '{}'", backup.display()));

    let db = Database::new(live.clone()).unwrap();
    db.restore_from_file(&backup, &live).unwrap();

    let trip = db.get_trip("t1").unwrap().expect("the restored trip");
    assert_eq!((trip.origin.as_str(), trip.destination.as_str()), ("Nitra", "Levice"));
    assert_ne!(trip.origin_place_id, Uuid::nil());
}
