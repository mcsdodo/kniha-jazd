**Date:** 2026-10-05
**Subject:** Implementation plan for task 88, places as entities
**Status:** Planning

# Places as Entities Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A place is a record with an ID, a name and a position, and trips and routes point to it by ID.

**Architecture:** One plain-SQL Diesel migration rebuilds `places`, `trips` and `routes`. It uses a SQLite scalar function `kj_normalise`, which Rust registers on each connection before the migrations run. The read side keeps `Trip.origin` and `Trip.destination` as display names, which Rust fills from a place-name map. All writes take place IDs. The frontend gets a new `/miesta` tab, and the trip form accepts only existing places.

**Tech Stack:** Rust (Diesel 2.3.5, bundled SQLite from `libsqlite3-sys` 0.30), Axum JSON-RPC, SvelteKit + TypeScript, typesafe-i18n, WebdriverIO.

**Spec:** [01-task.md](./01-task.md). Read it before each task. This plan argues from it.

## Global Constraints

- Logic stays in `kniha-jazd-core` (ADR-008). The frontend displays and selects. It does not fold or compare place names.
- Test first for each backend change. Write the failing test, see it fail, then write the code.
- Migration folder name: `2026-10-05-100000_places_as_entities`.
- SQLite function name: `kj_normalise`. It calls `crate::places::normalise`.
- Rust `Place` (API, camelCase): `id: Uuid, name: String, normalised_name: String, lat: Option<f64>, lon: Option<f64>, source: Option<PlaceSource>, uses: i64`.
- Rust `PlaceRow` (DB row): `id: String, name: String, normalised_name: String, lat: Option<f64>, lon: Option<f64>, source: Option<String>, created_at: String`.
- `Trip` gets `origin_place_id: Uuid, destination_place_id: Uuid`. It keeps `origin: String, destination: String` as display names. `Route` gets the same two ID fields.
- DB API that task 89 uses: `Database::get_place(&self, id: &str) -> QueryResult<Option<PlaceRow>>`, `Database::all_places(&self) -> QueryResult<Vec<PlaceRow>>`, `Database::get_trips_for_vehicle(&self, vehicle_id: &str) -> QueryResult<Vec<Trip>>` (with names).
- RPC names: `list_places`, `create_place {name, lat, lon, source}`, `rename_place {id, name}`, `set_place_position {id, lat, lon, source}`, `delete_place {id}`, `find_place {name}`. Trip writes take `originPlaceId` and `destinationPlaceId`.
- Frontend `Place { id, name, lat, lon, source, uses }` (plus `normalisedName`, which the Rust struct also serialises). Route `/miesta`. Test IDs `places-list`, `places-filter`, and `place-row` with `data-place-id` on each row.
- Each write command calls `check_read_only!(app_state)` first.
- Foreign keys are ON for every connection, also while migrations run. The bundled `libsqlite3-sys` 0.30.1 builds SQLite with `-DSQLITE_DEFAULT_FOREIGN_KEYS=1` (its `build.rs:123`). Do not add a pragma, and do not turn the pragma off in a migration.
- All UI strings go through i18n, Slovak first. Run `npm run i18n` after an edit to `src/lib/i18n/{sk,en}/index.ts`.
- Keyboard-typable characters only in code comments, docs and commit messages. Slovak diacritics are allowed.
- Use `cargo test --manifest-path src-tauri/Cargo.toml ...`. Never `cd && cargo`.
- Stage only the files of the current task. Never `git add -A`.
- This repo is public. No homelab host, IP or real trip data in code, tests or docs.

## Review Focus

1. **An old backup restored after the upgrade.** `restore_from_file` must run the new migration with `kj_normalise` registered, or the restore fails with "no such function". Test: Task 1, Step 5.
2. **A user types `bratislava` and the place is `Bratislava`.** The user expects the trip to save on the existing place, not a block. Test: Task 4 (`find_place` folds case and diacritics) and Task 6 (the integration flow).
3. **A rename that changes only case or diacritics** (`Kosice` to `Košice`). This is the same place, so the rename must succeed, not fail as a collision. Test: Task 4.
4. **An upgrade of a book with saved route maps and Paperless links.** With foreign keys on, `DROP TABLE trips` deletes every child row unless the migration copies them first. The user expects all maps and links after the upgrade. Test: Task 2 (`trip_children_survive_the_trips_rebuild`).
5. **Delete a place that only an orphan route uses** (all its trips are deleted). The list shows `uses = 0`, so the user expects the delete to work. Test: Task 4.

---

## File Structure

| File | Responsibility | Task |
|------|----------------|------|
| [src-tauri/core/src/db.rs](../../src-tauri/core/src/db.rs) | `prepare_connection`, place CRUD, trip and route reads by ID, name fill | 1, 3, 4 |
| [src-tauri/core/migrations/2026-10-05-100000_places_as_entities/](../../src-tauri/core/migrations/) | `up.sql`, `down.sql` | 2 |
| [src-tauri/core/src/migration_tests.rs](../../src-tauri/core/src/migration_tests.rs) | Migration tests | 1, 2 |
| [src-tauri/core/src/schema.rs](../../src-tauri/core/src/schema.rs) | Diesel tables | 3 |
| [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) | `Trip`, `TripRow`, `Route`, `RouteRow`, `PlaceRow`, `Place`, `CopiedTripDefaults` | 3 |
| [src-tauri/core/src/commands_internal/places_cmd.rs](../../src-tauri/core/src/commands_internal/places_cmd.rs) | Place commands | 3, 4 |
| [src-tauri/core/src/commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) | Trip writes by place ID, time inference by ID | 3 |
| [src-tauri/core/src/commands_internal/route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs) | `mode_for`, `placed_endpoint` by ID | 3 |
| [src-tauri/core/src/calculations/trip_copy.rs](../../src-tauri/core/src/calculations/trip_copy.rs) | Copied defaults carry place IDs | 3 |
| [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) | RPC args | 3, 4 |
| [src/lib/types.ts](../../src/lib/types.ts), [src/lib/api.ts](../../src/lib/api.ts) | Types and RPC wrappers | 5 |
| [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte), [TripGrid.svelte](../../src/lib/components/TripGrid.svelte) | Trip form by place ID | 5, 6 |
| [src/routes/mapa/+page.svelte](../../src/routes/mapa/+page.svelte) | Position a legacy place by ID | 5 |
| `src/routes/miesta/+page.svelte` (new), [src/routes/+layout.svelte](../../src/routes/+layout.svelte) | Miesta tab and nav link | 7 |
| [src/routes/settings/+page.svelte](../../src/routes/settings/+page.svelte) | Remove the place section | 7 |
| [tests/integration/utils/db.ts](../../tests/integration/utils/db.ts), [forms.ts](../../tests/integration/utils/forms.ts) | `ensurePlace`, seed by name | 5 |
| [tests/integration/wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) | `resetDatabase` also deletes places | 5 |
| [tests/integration/specs/tier2/places.spec.ts](../../tests/integration/specs/tier2/places.spec.ts) | Miesta tab flows, trip-form block | 6, 7 |
| [docs/features/](../../docs/features/), READMEs, [DECISIONS.md](../../DECISIONS.md), [CHANGELOG.md](../../CHANGELOG.md) | Docs | 8, 9 |

**Build state between tasks.** Tasks 2 and 3 commit together: the migration changes the tables, and the Rust code reads the new tables, so neither half is green alone. From Task 3 to Task 5, the backend is green, but the frontend sends the old trip args, so the integration suite is red. Do this work on a feature branch. Do not push to `main` before Task 9 passes.

---

### Task 1: Register `kj_normalise` on every connection

**Files:**
- Modify: [src-tauri/core/src/db.rs:56-170](../../src-tauri/core/src/db.rs#L56) (`new`, `in_memory`, `restore_from_file`), [db.rs:1339](../../src-tauri/core/src/db.rs#L1339) (`open_db_legacy_before`)
- Test: [src-tauri/core/src/migration_tests.rs](../../src-tauri/core/src/migration_tests.rs)

**Interfaces:**
- Produces: `pub(crate) fn prepare_connection(conn: &mut SqliteConnection) -> QueryResult<()>` in `db.rs`. It registers `kj_normalise(Text) -> Text`. Each path that opens a connection calls it before `run_pending_migrations`.

- [ ] **Step 1: Write the failing test**

Add to `migration_tests.rs`:

```rust
// ============================================================================
// Task 88 -- kj_normalise is available to SQL on every connection
// ============================================================================

#[derive(diesel::QueryableByName)]
struct TextRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    v: String,
}

fn kj(db: &Database, input: &str) -> String {
    let conn = &mut *db.connection();
    diesel::sql_query("SELECT kj_normalise(?) AS v")
        .bind::<diesel::sql_types::Text, _>(input)
        .get_result::<TextRow>(conn)
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
```

- [ ] **Step 2: Run the tests and see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core kj_normalise`
Expected: FAIL. The panic says `kj_normalise must be registered` with `no such function: kj_normalise`.

- [ ] **Step 3: Write the code**

In `db.rs`, below the `MIGRATIONS` constant:

```rust
diesel::define_sql_function! {
    /// `places::normalise`, callable from SQL. The places migration
    /// (2026-10-05-100000) keys trips onto places with it, and SQLite has no
    /// function of its own that folds Slovak diacritics.
    fn kj_normalise(x: diesel::sql_types::Text) -> diesel::sql_types::Text;
}

/// Set up one connection before anything runs on it: register the SQL
/// functions the migrations need. Every path that opens a connection and
/// migrates it must call this first, or a migration fails with
/// "no such function".
pub(crate) fn prepare_connection(conn: &mut SqliteConnection) -> QueryResult<()> {
    kj_normalise_utils::register_impl(conn, |x: String| crate::places::normalise(&x))
}
```

Call it in four places, each one directly after `SqliteConnection::establish(...)`:

```rust
// Database::new, after `let mut conn = SqliteConnection::establish(path_str)?;`
prepare_connection(&mut conn)
    .map_err(|e| diesel::ConnectionError::BadConnection(e.to_string()))?;

// Database::in_memory, same position, same line.

// Database::restore_from_file, after `*conn = SqliteConnection::establish(path_str)...?;`
prepare_connection(&mut conn).map_err(|e| e.to_string())?;

// open_db_legacy_before, after `let mut conn = SqliteConnection::establish(":memory:")...;`
prepare_connection(&mut conn).expect("Failed to register SQL functions");
```

`from_path` runs no migrations. Leave it unchanged.

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core kj_normalise`
Expected: PASS, 2 tests.

- [ ] **Step 5: Write the restore-path test**

This test checks only that the restore path registers the function. Task 3, Step 1 adds the full test: restore an old backup file and check that it migrates (Review Focus 1). That test needs `Trip.origin_place_id`, which does not exist yet. Add to `migration_tests.rs`:

```rust
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
```

`tempfile = "3"` is already a dependency of core ([Cargo.toml:38](../../src-tauri/core/Cargo.toml)).

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core restore_registers_kj_normalise`
Expected: PASS.

- [ ] **Step 6: Run the whole backend suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/core/src/db.rs src-tauri/core/src/migration_tests.rs
git commit -m "feat(db): register kj_normalise SQL function on every connection"
```

---

### Task 2: The migration (commit with Task 3)

**Files:**
- Create: `src-tauri/core/migrations/2026-10-05-100000_places_as_entities/up.sql`
- Create: `src-tauri/core/migrations/2026-10-05-100000_places_as_entities/down.sql`
- Test: [src-tauri/core/src/migration_tests.rs](../../src-tauri/core/src/migration_tests.rs)

**Interfaces:**
- Consumes: `kj_normalise` (Task 1).
- Produces: the tables below. Task 3 mirrors them in `schema.rs`. **The column order is the order of the positional `Queryable` structs.**

```
places (id TEXT PK NOT NULL, name TEXT NOT NULL, normalised_name TEXT NOT NULL UNIQUE,
        lat REAL, lon REAL, source TEXT, created_at TEXT NOT NULL)
trips  (id, vehicle_id, origin_place_id, destination_place_id, distance_km, ...
        the other columns in today's order ...)
routes (id, vehicle_id, origin_place_id, destination_place_id, distance_km,
        UNIQUE(vehicle_id, origin_place_id, destination_place_id))
```

**Rules that the SQL implements** (from the spec, plus three that the spec does not state; Task 9 records them in the ADR):
- Place name: the most-used spelling of the key, then the byte-wise smaller spelling. This is the same rule as `list_places_internal` today ([places_cmd.rs:55-66](../../src-tauri/core/src/commands_internal/places_cmd.rs#L55)). SQLite `BINARY` collation compares bytes, the same as Rust `String` `<`.
- Coordinates and `source` come from the old `places` row with the same key. A key without an old row gets NULL coordinates and a NULL source.
- An old `places` row that no trip uses becomes a place with its `display_name` as the name.
- **Not in the spec:** an empty or whitespace-only trip endpoint maps to one place named `Neznáme miesto`. Without this, the trip has no place to point to and the migration would drop it. The live data has none.
- **Not in the spec:** a `routes` row whose pair no trip uses is dropped. Today `get_routes_for_vehicle` already hides it through an inner join ([db.rs:720-733](../../src-tauri/core/src/db.rs#L720)), so nothing visible changes. Without this rule, a route-only string would need a place with no position (the live copy has 9 such strings).
- Routes that collapse onto one place pair: keep the row whose exact old strings the most recent trip on that pair used. If no row has them, keep the first row by `id`.
- Foreign keys are ON while the migration runs (see Global Constraints). `DROP TABLE trips` would delete every `trip_routes` and `paperless_trip_links` row through `ON DELETE CASCADE`. The SQL copies both tables into TEMP tables before the drop and inserts them back after the rename. Do not use `run_in_transaction = false` with `PRAGMA foreign_keys = OFF`: Diesel writes the version row outside the SQL file. The `places` and `routes` rebuilds have no child tables with `REFERENCES`, so they need no copy.
- The trip copy uses LEFT JOINs. If a trip key has no place, the `NOT NULL` column aborts the transaction. A trip is never dropped without notice.

- [ ] **Step 1: Write the failing migration tests**

Add to `migration_tests.rs`. They use the existing helpers `exec`, `seed_vehicle` and `seed_trip` in that file. Read `seed_trip` first: if it does not take origin and destination, add the local helper below.

```rust
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
```

The existing `test_migrated_schema_identical_to_fresh_schema` ([migration_tests.rs:315](../../src-tauri/core/src/migration_tests.rs#L315)) also covers the new DDL. Keep it.

- [ ] **Step 2: Run the tests and see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core migration_tests`
Expected: the new tests FAIL with `no such column: name` or `no such column: origin_place_id`.

- [ ] **Step 3: Write `up.sql`**

```sql
-- Task 88: places become entities; trips and routes point at them by id.
-- Plain SQL, run in one transaction by Diesel. kj_normalise is
-- places::normalise, registered from Rust by db::prepare_connection.
-- `diesel migration run` from the CLI fails here with
-- "no such function: kj_normalise": only the app can run this migration.
-- Foreign keys are ON (the bundled SQLite is built with
-- SQLITE_DEFAULT_FOREIGN_KEYS=1), so DROP TABLE trips would cascade into
-- trip_routes and paperless_trip_links. Step 6 copies both first.

-- 1. Every spelling a trip uses, with its use count. A blank endpoint is
--    renamed to one shared placeholder place so its trip keeps a target.
CREATE TEMP TABLE kj_spellings AS
SELECT raw, kj_normalise(raw) AS key, SUM(uses) AS uses FROM (
    SELECT CASE WHEN kj_normalise(origin) = '' THEN 'Neznáme miesto' ELSE origin END AS raw,
           COUNT(*) AS uses
      FROM trips GROUP BY 1
    UNION ALL
    SELECT CASE WHEN kj_normalise(destination) = '' THEN 'Neznáme miesto' ELSE destination END,
           COUNT(*)
      FROM trips GROUP BY 1
) GROUP BY raw;

-- 2. The new places table.
CREATE TABLE places_new (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    normalised_name TEXT NOT NULL UNIQUE,
    lat REAL,
    lon REAL,
    source TEXT,
    created_at TEXT NOT NULL
);

-- 3. One place per key a trip uses. Name: most uses, then the byte-wise
--    smaller spelling (BINARY collation), the rule list_places used.
WITH ranked AS (
    SELECT key, raw,
           ROW_NUMBER() OVER (PARTITION BY key ORDER BY uses DESC, raw ASC) AS rn
      FROM kj_spellings
)
INSERT INTO places_new (id, name, normalised_name, lat, lon, source, created_at)
SELECT lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' ||
             substr(hex(randomblob(2)), 2) || '-' ||
             substr('89ab', 1 + (abs(random()) % 4), 1) ||
             substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))),
       r.raw, r.key, p.lat, p.lon,
       CASE WHEN p.lat IS NULL THEN NULL ELSE p.source END,
       strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
  FROM ranked r
  LEFT JOIN places p ON p.normalised_name = r.key
 WHERE r.rn = 1;

-- 4. Old places rows that no trip names stay as places.
INSERT INTO places_new (id, name, normalised_name, lat, lon, source, created_at)
SELECT lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' ||
             substr(hex(randomblob(2)), 2) || '-' ||
             substr('89ab', 1 + (abs(random()) % 4), 1) ||
             substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))),
       p.display_name, p.normalised_name, p.lat, p.lon, p.source,
       strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
  FROM places p
 WHERE p.normalised_name NOT IN (SELECT normalised_name FROM places_new);

DROP TABLE places;
ALTER TABLE places_new RENAME TO places;

-- 5. Routes, built while trips still carry their strings. Only pairs a trip
--    uses survive (the others were already hidden by the trips join).
--    Duplicates on one place pair keep the row whose exact strings the
--    latest trip on that pair used, else the first by id.
CREATE TABLE routes_new (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT NOT NULL,
    origin_place_id TEXT NOT NULL,
    destination_place_id TEXT NOT NULL,
    distance_km REAL NOT NULL,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id),
    FOREIGN KEY (origin_place_id) REFERENCES places(id),
    FOREIGN KEY (destination_place_id) REFERENCES places(id),
    UNIQUE(vehicle_id, origin_place_id, destination_place_id)
);

WITH latest AS (
    SELECT vehicle_id, origin, destination,
           kj_normalise(origin) AS ko, kj_normalise(destination) AS kd,
           ROW_NUMBER() OVER (
               PARTITION BY vehicle_id, kj_normalise(origin), kj_normalise(destination)
               ORDER BY start_datetime DESC, created_at DESC) AS rn
      FROM trips
),
ranked AS (
    SELECT r.id, r.vehicle_id, po.id AS o_id, pd.id AS d_id, r.distance_km,
           ROW_NUMBER() OVER (
               PARTITION BY r.vehicle_id, po.id, pd.id
               ORDER BY CASE WHEN r.origin = l.origin AND r.destination = l.destination
                             THEN 0 ELSE 1 END,
                        r.id) AS rn
      FROM routes r
      JOIN latest l ON l.rn = 1 AND l.vehicle_id = r.vehicle_id
                   AND l.ko = kj_normalise(r.origin) AND l.kd = kj_normalise(r.destination)
      JOIN places po ON po.normalised_name = kj_normalise(r.origin)
      JOIN places pd ON pd.normalised_name = kj_normalise(r.destination)
)
INSERT INTO routes_new (id, vehicle_id, origin_place_id, destination_place_id, distance_km)
SELECT id, vehicle_id, o_id, d_id, distance_km FROM ranked WHERE rn = 1;

DROP TABLE routes;
ALTER TABLE routes_new RENAME TO routes;
CREATE INDEX idx_routes_vehicle ON routes(vehicle_id);

-- 6. Trips: same columns in the same order, the two strings replaced by ids.
--    Keep the child rows: DROP TABLE trips cascades with foreign keys on.
CREATE TEMP TABLE kj_tr AS SELECT * FROM trip_routes;
CREATE TEMP TABLE kj_ptl AS SELECT * FROM paperless_trip_links;

CREATE TABLE trips_new (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT NOT NULL,
    origin_place_id TEXT NOT NULL,
    destination_place_id TEXT NOT NULL,
    distance_km REAL NOT NULL,
    odometer REAL NOT NULL,
    purpose TEXT NOT NULL,
    fuel_liters REAL,
    fuel_cost_eur REAL,
    other_costs_eur REAL,
    other_costs_note TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    full_tank INTEGER NOT NULL DEFAULT 1,
    energy_kwh REAL,
    energy_cost_eur REAL,
    full_charge INTEGER DEFAULT 0,
    soc_override_percent REAL,
    start_datetime TEXT NOT NULL DEFAULT '',
    end_datetime TEXT DEFAULT NULL,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id),
    FOREIGN KEY (origin_place_id) REFERENCES places(id),
    FOREIGN KEY (destination_place_id) REFERENCES places(id)
);

INSERT INTO trips_new
SELECT t.id, t.vehicle_id, po.id, pd.id, t.distance_km, t.odometer, t.purpose,
       t.fuel_liters, t.fuel_cost_eur, t.other_costs_eur, t.other_costs_note,
       t.created_at, t.updated_at, t.full_tank, t.energy_kwh, t.energy_cost_eur,
       t.full_charge, t.soc_override_percent, t.start_datetime, t.end_datetime
  FROM trips t
  -- LEFT JOIN: a key with no place gives NULL, and NOT NULL aborts the
  -- transaction instead of dropping the trip.
  LEFT JOIN places po ON po.normalised_name = kj_normalise(
       CASE WHEN kj_normalise(t.origin) = '' THEN 'Neznáme miesto' ELSE t.origin END)
  LEFT JOIN places pd ON pd.normalised_name = kj_normalise(
       CASE WHEN kj_normalise(t.destination) = '' THEN 'Neznáme miesto' ELSE t.destination END);

DROP TABLE trips;
ALTER TABLE trips_new RENAME TO trips;
CREATE INDEX idx_trips_vehicle_start_datetime ON trips(vehicle_id, start_datetime);
CREATE INDEX idx_trips_origin_place ON trips(origin_place_id);
CREATE INDEX idx_trips_destination_place ON trips(destination_place_id);

INSERT INTO trip_routes SELECT * FROM kj_tr;
INSERT INTO paperless_trip_links SELECT * FROM kj_ptl;
DROP TABLE kj_tr;
DROP TABLE kj_ptl;
DROP TABLE kj_spellings;
```

Before you run it, check the `trips` DDL against the live column order: `sqlite3 <a scratch copy of a DB> ".schema trips"`. The `CREATE TABLE trips_new` column list must keep the order that `TripRow` binds in ([models.rs:847-870](../../src-tauri/core/src/models.rs#L847)). If any `CREATE INDEX` on `trips` or `routes` exists in an earlier migration and is not listed above, add it (`grep -rn "CREATE INDEX\|CREATE UNIQUE INDEX" src-tauri/core/migrations | grep -E "trips|routes"`).

- [ ] **Step 4: Write `down.sql`**

```sql
-- Forward-only in practice (ADR-012). Rebuilds the string columns from
-- the place names so a developer can step back one migration. The other
-- spellings of a place are gone; each trip gets the place name.
-- Foreign keys are ON: keep the child rows across DROP TABLE trips.
CREATE TEMP TABLE kj_tr AS SELECT * FROM trip_routes;
CREATE TEMP TABLE kj_ptl AS SELECT * FROM paperless_trip_links;

CREATE TABLE trips_old (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT NOT NULL,
    origin TEXT NOT NULL,
    destination TEXT NOT NULL,
    distance_km REAL NOT NULL,
    odometer REAL NOT NULL,
    purpose TEXT NOT NULL,
    fuel_liters REAL,
    fuel_cost_eur REAL,
    other_costs_eur REAL,
    other_costs_note TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    full_tank INTEGER NOT NULL DEFAULT 1,
    energy_kwh REAL,
    energy_cost_eur REAL,
    full_charge INTEGER DEFAULT 0,
    soc_override_percent REAL,
    start_datetime TEXT NOT NULL DEFAULT '',
    end_datetime TEXT DEFAULT NULL,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id)
);
INSERT INTO trips_old
SELECT t.id, t.vehicle_id, po.name, pd.name, t.distance_km, t.odometer, t.purpose,
       t.fuel_liters, t.fuel_cost_eur, t.other_costs_eur, t.other_costs_note,
       t.created_at, t.updated_at, t.full_tank, t.energy_kwh, t.energy_cost_eur,
       t.full_charge, t.soc_override_percent, t.start_datetime, t.end_datetime
  FROM trips t
  JOIN places po ON po.id = t.origin_place_id
  JOIN places pd ON pd.id = t.destination_place_id;
DROP TABLE trips;
ALTER TABLE trips_old RENAME TO trips;
CREATE INDEX idx_trips_vehicle_start_datetime ON trips(vehicle_id, start_datetime);
INSERT INTO trip_routes SELECT * FROM kj_tr;
INSERT INTO paperless_trip_links SELECT * FROM kj_ptl;
DROP TABLE kj_tr;
DROP TABLE kj_ptl;

CREATE TABLE routes_old (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT NOT NULL,
    origin TEXT NOT NULL,
    destination TEXT NOT NULL,
    distance_km REAL NOT NULL,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id),
    UNIQUE(vehicle_id, origin, destination)
);
INSERT INTO routes_old
SELECT r.id, r.vehicle_id, po.name, pd.name, r.distance_km
  FROM routes r
  JOIN places po ON po.id = r.origin_place_id
  JOIN places pd ON pd.id = r.destination_place_id;
DROP TABLE routes;
ALTER TABLE routes_old RENAME TO routes;
CREATE INDEX idx_routes_vehicle ON routes(vehicle_id);

CREATE TABLE places_old (
    normalised_name TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL,
    lat REAL,
    lon REAL,
    source TEXT NOT NULL
);
INSERT INTO places_old
SELECT normalised_name, name, lat, lon, source FROM places WHERE lat IS NOT NULL;
DROP TABLE places;
ALTER TABLE places_old RENAME TO places;
```

Check the old `places` DDL comment lines in [2026-09-07-100000_add_places/up.sql](../../src-tauri/core/migrations/2026-09-07-100000_add_places/up.sql). `down.sql` does not need them.

- [ ] **Step 5: Add the `down.sql` round-trip test**

```rust
#[test]
fn down_sql_gives_back_the_trip_strings() {
    let db = open_db_legacy_before(PLACES_AS_ENTITIES);
    seed_vehicle(&db, "v1");
    seed_trip_at(&db, "t1", "v1", "Nitra", "Levice", "2026-01-01T08:00:00");
    migrate_to_current(&db);
    {
        let conn = &mut *db.connection();
        conn.revert_last_migration(crate::db::MIGRATIONS).unwrap();
    }
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Text)]
        origin: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        destination: String,
    }
    let conn = &mut *db.connection();
    let rows: Vec<Row> = diesel::sql_query("SELECT origin, destination FROM trips")
        .load(conn)
        .unwrap();
    assert_eq!((rows[0].origin.as_str(), rows[0].destination.as_str()), ("Nitra", "Levice"));
}
```

Also check that the child rows survive the revert: add the `trip_routes` insert from `trip_children_survive_the_trips_rebuild` before `migrate_to_current`, and assert `db.get_route_map("t1")` is `Some` after the revert.

`revert_last_migration` reverts the newest migration. If task 89 or another later migration is already merged when you run this, revert in a loop until this migration's version is reverted.

- [ ] **Step 6: Run the migration tests**

Before you run them, check the `paperless_trip_links` column list in the live schema: `PRAGMA table_info(paperless_trip_links)` on a scratch DB. If the test insert names a column that does not exist, fix the insert. Also check with the reviewer's run that the TEMP copy keeps the rows: on a copy of a real-shaped DB, `PRAGMA foreign_key_check` and `PRAGMA integrity_check` must return nothing and `ok`.

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core migration_tests`
Expected: the Task 88 migration tests PASS. Other suites fail at runtime now, because `schema.rs` still names `origin`. That is expected. **Do not commit.** Go to Task 3.

---

### Task 3: Switch the Rust code to place IDs (commit Tasks 2 and 3 together)

**Files:**
- Modify: [schema.rs](../../src-tauri/core/src/schema.rs), [models.rs](../../src-tauri/core/src/models.rs), [db.rs](../../src-tauri/core/src/db.rs), [commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs), [commands_internal/places_cmd.rs](../../src-tauri/core/src/commands_internal/places_cmd.rs), [commands_internal/route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs), [calculations/trip_copy.rs](../../src-tauri/core/src/calculations/trip_copy.rs), [commands_internal/statistics.rs:1530-1570](../../src-tauri/core/src/commands_internal/statistics.rs#L1530), [commands_internal/export_cmd.rs:56](../../src-tauri/core/src/commands_internal/export_cmd.rs#L56), [server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs), [server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs)
- Modify (doc comments only): [places/normalise.rs:8](../../src-tauri/core/src/places/normalise.rs#L8)
- Test: [db_tests.rs](../../src-tauri/core/src/db_tests.rs), [commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs), [commands_internal/places_cmd_tests.rs](../../src-tauri/core/src/commands_internal/places_cmd_tests.rs), [commands_internal/route_maps_tests.rs](../../src-tauri/core/src/commands_internal/route_maps_tests.rs), [export_tests.rs](../../src-tauri/core/src/export_tests.rs), [invoice_tests.rs](../../src-tauri/core/src/invoice_tests.rs), [calculations/tests.rs](../../src-tauri/core/src/calculations/tests.rs), [migration_tests.rs](../../src-tauri/core/src/migration_tests.rs), the inline tests in `models.rs`, `dispatcher.rs`, `dispatcher_async.rs` and `trip_copy.rs`

**`Trip` struct literals.** `Trip` has no `Default`, so each `Trip { ... }` literal fails to **compile** until it has `origin_place_id` and `destination_place_id`. There are about 55 literals in these 12 files (`grep -rc "Trip {$" src-tauri/core/src --include=*.rs | grep -v ":0"`): `models.rs` (4), `db_tests.rs` (9), `commands_internal/commands_tests.rs` (40), `invoice_tests.rs` (3), `export_tests.rs` (2), `calculations/tests.rs` (2), `calculations/trip_copy.rs` (2), `commands_internal/route_maps_tests.rs` (2), `commands_internal/statistics.rs` (2), `commands_internal/trips.rs` (3), `commands_internal/export_cmd.rs` (1), `server/dispatcher_async.rs` (1). Add `origin_place_id: Uuid::nil(), destination_place_id: Uuid::nil(),` to each literal. A literal that is then inserted into the DB needs real IDs (Step 9.3).

**Interfaces:**
- Consumes: the tables from Task 2.
- Produces:
  - `PlaceRow`, `Place` as in Global Constraints.
  - `NewPlaceRow<'a> { id: &'a str, name: &'a str, normalised_name: &'a str, lat: Option<f64>, lon: Option<f64>, source: Option<&'a str>, created_at: &'a str }`.
  - `Trip { ..., origin_place_id: Uuid, destination_place_id: Uuid, origin: String, destination: String, ... }`. `origin` and `destination` are filled on read and ignored on write.
  - `Route { id, vehicle_id, origin_place_id: Uuid, destination_place_id: Uuid, origin: String, destination: String, distance_km, usage_count, last_used }`.
  - `CopiedTripDefaults` gets `origin_place_id: Uuid, destination_place_id: Uuid` next to the names.
  - `Database::get_place(&self, id: &str) -> QueryResult<Option<PlaceRow>>`
  - `Database::all_places(&self) -> QueryResult<Vec<PlaceRow>>`
  - `Database::place_uses(&self) -> QueryResult<HashMap<String, i64>>` (place id to the count of trip endpoints)
  - `Database::find_or_create_route(&self, vehicle_id: &str, origin_place_id: &str, destination_place_id: &str, distance_km: f64) -> QueryResult<RouteRow>`
  - `Database::find_most_recent_trip_times_for_route(&self, vehicle_id: &str, origin_place_id: &str, destination_place_id: &str)`
  - `create_trip_internal(..., origin_place_id: String, destination_place_id: String, ...)`, and the same for `update_trip_internal`, `create_trip_cascade_internal`, `update_trip_cascade_internal`. An unknown ID returns `Err("Miesto neexistuje: <id>")`.
  - `get_inferred_trip_time_for_route_internal(db, app_dir, vehicle_id, origin_place_id, destination_place_id, row_date)`.
  - `#[cfg(test)] Database::ensure_place_for_test(&self, name: &str) -> Uuid`: it returns the ID of the place with that normalised name, or creates the place at `(48.15, 17.11)` with source `manual`. Each test that inserts a trip sets `origin_place_id` and `destination_place_id` from it.
  - Row-to-API constructors: `Trip::from_row(row: TripRow, names: &HashMap<String, String>) -> Trip`, `Place::from_row(row: PlaceRow, uses: i64) -> Place`, and the private `fn place_names(conn: &mut SqliteConnection) -> QueryResult<HashMap<String, String>>` in `db.rs`.

- [ ] **Step 1: Write the failing tests for the new behaviour**

The real helpers are `create_test_vehicle(name) -> Vehicle` ([db_tests.rs:93](../../src-tauri/core/src/db_tests.rs#L93), not inserted), `create_test_trip(vehicle_id, "YYYY-MM-DD") -> Trip` ([db_tests.rs:257](../../src-tauri/core/src/db_tests.rs#L257), origin `Prague`, destination `Brno`, not inserted) and `setup_db_with_vehicle() -> (Database, Vehicle)` ([commands_tests.rs:2267](../../src-tauri/core/src/commands_internal/commands_tests.rs#L2267)). Add to `db_tests.rs`:

```rust
/// Insert `trip` after pointing it at places named like its old strings.
fn insert_trip_with_places(db: &Database, trip: &mut Trip) {
    trip.origin_place_id = db.ensure_place_for_test(&trip.origin);
    trip.destination_place_id = db.ensure_place_for_test(&trip.destination);
    db.create_trip(trip).unwrap();
}

#[test]
fn a_trip_reads_back_its_place_names() {
    let db = Database::in_memory().unwrap();
    let vehicle = create_test_vehicle("Car");
    db.create_vehicle(&vehicle).unwrap();
    let mut trip = create_test_trip(vehicle.id, "2026-03-01");
    insert_trip_with_places(&db, &mut trip);

    let back = db.get_trip(&trip.id.to_string()).unwrap().unwrap();
    assert_eq!((back.origin.as_str(), back.destination.as_str()), ("Prague", "Brno"));
    assert_eq!(back.origin_place_id, db.ensure_place_for_test("prague"));
}

#[test]
fn a_rename_shows_on_every_trip_of_the_place() {
    let db = Database::in_memory().unwrap();
    let vehicle = create_test_vehicle("Car");
    db.create_vehicle(&vehicle).unwrap();
    let mut trip = create_test_trip(vehicle.id, "2026-03-01");
    insert_trip_with_places(&db, &mut trip);
    db.rename_place(&trip.origin_place_id.to_string(), "Praha", "praha").unwrap();
    assert_eq!(db.get_trip(&trip.id.to_string()).unwrap().unwrap().origin, "Praha");
}

#[test]
fn routes_are_found_by_place_ids() {
    let db = Database::in_memory().unwrap();
    let vehicle = create_test_vehicle("Car");
    db.create_vehicle(&vehicle).unwrap();
    let a = db.ensure_place_for_test("Nitra").to_string();
    let b = db.ensure_place_for_test("Levice").to_string();
    let v = vehicle.id.to_string();
    let first = db.find_or_create_route(&v, &a, &b, 40.0).unwrap();
    let second = db.find_or_create_route(&v, &a, &b, 99.0).unwrap();
    assert_eq!(first.id, second.id);
    assert_eq!(second.distance_km, 40.0, "an existing pair is not overwritten");
}

/// Regression guard: the bundled SQLite turns foreign keys on by default
/// (SQLITE_DEFAULT_FOREIGN_KEYS=1). If that ever changes, place ids stop
/// being enforced and this test fails.
#[test]
fn a_trip_cannot_point_at_a_missing_place() {
    let db = Database::in_memory().unwrap();
    let vehicle = create_test_vehicle("Car");
    db.create_vehicle(&vehicle).unwrap();
    let mut trip = create_test_trip(vehicle.id, "2026-03-01");
    trip.origin_place_id = Uuid::new_v4();
    trip.destination_place_id = Uuid::new_v4();
    assert!(db.create_trip(&trip).is_err(), "FOREIGN KEY constraint failed");
}
```

Add to `commands_tests.rs`:

```rust
#[test]
fn create_trip_with_an_unknown_place_id_fails() {
    let (db, vehicle) = setup_db_with_vehicle();
    let app_state = AppState::new();
    let known = db.ensure_place_for_test("Nitra").to_string();
    let err = create_trip_internal(
        &db, &app_state, vehicle.id.to_string(),
        "2026-03-01T08:00:00".into(), "2026-03-01T09:00:00".into(),
        known, Uuid::new_v4().to_string(),
        10.0, 1010.0, "p".into(),
        None, None, None, None, None, None, None, None, None,
    )
    .unwrap_err();
    assert!(err.starts_with("Miesto neexistuje"), "{err}");
}
```

Add to `migration_tests.rs` (Review Focus 1). It builds an old backup file with `VACUUM INTO` and restores it:

```rust
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
```

`seed_vehicle`, `exec`, `seed_trip_at` and `PLACES_AS_ENTITIES` are in `migration_tests.rs` (Task 2). `get_trip` looks up the text `id` column, so the id `t1` works; `Trip::from_row` gives a random UUID for a non-UUID id, so compare names and place IDs only.

- [ ] **Step 2: Run them and see them fail to compile**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core a_trip_reads_back_its_place_names`
Expected: compile errors: `ensure_place_for_test`, `origin_place_id` and `rename_place` do not exist.

- [ ] **Step 3: Update `schema.rs`**

Replace the `routes`, `trips` and `places` blocks. Keep the other columns of `trips` in their current order:

```rust
diesel::table! {
    routes (id) {
        id -> Nullable<Text>,
        vehicle_id -> Text,
        origin_place_id -> Text,
        destination_place_id -> Text,
        distance_km -> Double,
    }
}

diesel::table! {
    trips (id) {
        id -> Nullable<Text>,
        vehicle_id -> Text,
        origin_place_id -> Text,
        destination_place_id -> Text,
        distance_km -> Double,
        // ... the remaining columns exactly as today ...
    }
}

// Rebuilt by migration 2026-10-05-100000_places_as_entities (Task 88).
diesel::table! {
    places (id) {
        id -> Text,
        name -> Text,
        normalised_name -> Text,
        lat -> Nullable<Double>,
        lon -> Nullable<Double>,
        source -> Nullable<Text>,
        created_at -> Text,
    }
}
```

Add `places` to `allow_tables_to_appear_in_same_query!`.

- [ ] **Step 4: Update `models.rs`**

1. `TripRow` and `NewTripRow`: rename `origin` to `origin_place_id` and `destination` to `destination_place_id`. Keep the field position.
2. `RouteRow` and `NewRouteRow`: the same rename.
3. Replace `PlaceRow` and `NewPlaceRow`:

```rust
/// Database row for the places table (Task 88: a place is an entity).
///
/// Still no `AsChangeset`: every update names its columns, so `None` can never
/// be read as "leave this column alone".
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = places)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PlaceRow {
    pub id: String,
    pub name: String,
    pub normalised_name: String,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub source: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = places)]
pub struct NewPlaceRow<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub normalised_name: &'a str,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub source: Option<&'a str>,
    pub created_at: &'a str,
}
```

4. Replace the API `Place`:

```rust
/// One place in the book (Task 88). `lat`/`lon` are None only for a place the
/// migration made from a trip string that had no coordinate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub id: Uuid,
    pub name: String,
    pub normalised_name: String,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub source: Option<PlaceSource>,
    /// How many trip endpoints point at this place.
    pub uses: i64,
}

impl Place {
    pub fn from_row(row: PlaceRow, uses: i64) -> Self {
        Place {
            id: Uuid::parse_str(&row.id).unwrap_or_else(|_| Uuid::nil()),
            name: row.name,
            normalised_name: row.normalised_name,
            lat: row.lat,
            lon: row.lon,
            source: row.source.as_deref().and_then(PlaceSource::parse),
            uses,
        }
    }
}
```

5. `Trip`: add `pub origin_place_id: Uuid, pub destination_place_id: Uuid` directly before `origin`. Add a doc comment on `origin` and `destination`: "Display name of the place, filled on read. A write uses the `*_place_id` fields and ignores this." Set both IDs to `Uuid::nil()` in `test_ice_trip` and its sibling constructors.
6. Replace `impl From<TripRow> for Trip` with `impl Trip { pub fn from_row(row: TripRow, names: &HashMap<String, String>) -> Self { ... } }`. It does the same parsing, and also:

```rust
origin: names.get(&row.origin_place_id).cloned().unwrap_or_default(),
destination: names.get(&row.destination_place_id).cloned().unwrap_or_default(),
origin_place_id: Uuid::parse_str(&row.origin_place_id).unwrap_or_else(|_| Uuid::nil()),
destination_place_id: Uuid::parse_str(&row.destination_place_id).unwrap_or_else(|_| Uuid::nil()),
```

Update the `make_trip_row` tests at [models.rs:1302](../../src-tauri/core/src/models.rs#L1302) to call `Trip::from_row(row, &HashMap::new())`.

7. `Route`: add `origin_place_id: Uuid, destination_place_id: Uuid` before `origin`.
8. `CopiedTripDefaults`: add `origin_place_id: Uuid, destination_place_id: Uuid` before `origin`. In [trip_copy.rs:68](../../src-tauri/core/src/calculations/trip_copy.rs#L68), copy `source.origin_place_id` and `source.destination_place_id`.

- [ ] **Step 5: Update `db.rs`**

1. Delete `normalize_location` ([db.rs:43](../../src-tauri/core/src/db.rs#L43)) and `distinct_trip_places` ([db.rs:1207](../../src-tauri/core/src/db.rs#L1207)). Delete `upsert_place` and the old `delete_place`.
2. Add a private name map and use it in every read that returns `Trip`:

```rust
/// Place id -> display name. Loaded once per read: tens of rows, and a trip
/// list always needs most of them.
fn place_names(conn: &mut SqliteConnection) -> QueryResult<HashMap<String, String>> {
    Ok(places::table
        .select((places::id, places::name))
        .load::<(String, String)>(conn)?
        .into_iter()
        .collect())
}
```

In `get_trip`, `get_trips_for_vehicle` and `get_trips_for_vehicle_in_year`, load the rows, then `let names = place_names(conn)?;` and map with `Trip::from_row(r, &names)`.

3. In `create_trip`, `update_trip`, `update_trip_tx` and `create_trip_with_odometer_shift`, write `origin_place_id: &trip.origin_place_id.to_string()` (bind the strings to locals first) and `trips::origin_place_id.eq(...)`.
4. Place reads and writes:

```rust
pub fn get_place(&self, id: &str) -> QueryResult<Option<PlaceRow>> {
    let conn = &mut *self.conn.lock().unwrap();
    places::table.filter(places::id.eq(id)).select(PlaceRow::as_select()).first(conn).optional()
}

pub fn get_place_by_key(&self, normalised_name: &str) -> QueryResult<Option<PlaceRow>> {
    let conn = &mut *self.conn.lock().unwrap();
    places::table
        .filter(places::normalised_name.eq(normalised_name))
        .select(PlaceRow::as_select())
        .first(conn)
        .optional()
}

pub fn all_places(&self) -> QueryResult<Vec<PlaceRow>> {
    let conn = &mut *self.conn.lock().unwrap();
    places::table.select(PlaceRow::as_select()).load(conn)
}

/// Place id -> number of trip endpoints that point at it. A place no trip
/// uses is absent; the caller reads that as 0.
pub fn place_uses(&self) -> QueryResult<HashMap<String, i64>> {
    let conn = &mut *self.conn.lock().unwrap();
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Text)]
        id: String,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        uses: i64,
    }
    let rows = diesel::sql_query(
        "SELECT id, SUM(n) AS uses FROM (
             SELECT origin_place_id AS id, COUNT(*) AS n FROM trips GROUP BY origin_place_id
             UNION ALL
             SELECT destination_place_id, COUNT(*) FROM trips GROUP BY destination_place_id
         ) GROUP BY id",
    )
    .load::<Row>(conn)?;
    Ok(rows.into_iter().map(|r| (r.id, r.uses)).collect())
}

pub fn insert_place(&self, place: &NewPlaceRow) -> QueryResult<()> {
    let conn = &mut *self.conn.lock().unwrap();
    diesel::insert_into(places::table).values(place).execute(conn).map(|_| ())
}

pub fn rename_place(&self, id: &str, name: &str, normalised_name: &str) -> QueryResult<usize> {
    let conn = &mut *self.conn.lock().unwrap();
    diesel::update(places::table.filter(places::id.eq(id)))
        .set((places::name.eq(name), places::normalised_name.eq(normalised_name)))
        .execute(conn)
}

pub fn set_place_position(&self, id: &str, lat: f64, lon: f64, source: &str) -> QueryResult<usize> {
    let conn = &mut *self.conn.lock().unwrap();
    diesel::update(places::table.filter(places::id.eq(id)))
        .set((places::lat.eq(Some(lat)), places::lon.eq(Some(lon)), places::source.eq(Some(source))))
        .execute(conn)
}
```

`delete_place` comes in Task 4.

5. `find_or_create_route`: take `origin_place_id` and `destination_place_id`, filter on `routes::origin_place_id` and `routes::destination_place_id`, and drop the `normalize_location` calls.
6. `get_routes_for_vehicle`: in the SQL, select `r.origin_place_id, r.destination_place_id, po.name AS origin, pd.name AS destination`, join `places po ON po.id = r.origin_place_id` and `places pd ON pd.id = r.destination_place_id`, and join trips on `t.origin_place_id = r.origin_place_id AND t.destination_place_id = r.destination_place_id`. Add the two ID fields to `DerivedRouteRow` and to `derived_route`.
7. `find_most_recent_trip_times_for_route`: filter on `dsl::origin_place_id` and `dsl::destination_place_id`.
8. Test helper, under `#[cfg(test)] impl Database`:

```rust
#[cfg(test)]
impl Database {
    /// The id of the place with this name's key, created at a fixed
    /// coordinate if it is missing. Tests that insert a trip set both
    /// place ids from it: foreign keys are on, so a nil id fails.
    pub fn ensure_place_for_test(&self, name: &str) -> Uuid {
        let key = crate::places::normalise(name);
        if let Some(row) = self.get_place_by_key(&key).unwrap() {
            return Uuid::parse_str(&row.id).unwrap();
        }
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let now = Utc::now().to_rfc3339();
        self.insert_place(&NewPlaceRow {
            id: &id_str, name, normalised_name: &key,
            lat: Some(48.15), lon: Some(17.11), source: Some("manual"), created_at: &now,
        })
        .unwrap();
        id
    }

    /// Like `ensure_place_for_test`, but the place has no coordinates, as a
    /// migrated legacy place can. Only the migration makes such a place in
    /// the app, so tests need this to reach the "unplaced" paths.
    pub fn ensure_unplaced_place_for_test(&self, name: &str) -> Uuid {
        let key = crate::places::normalise(name);
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let now = Utc::now().to_rfc3339();
        self.insert_place(&NewPlaceRow {
            id: &id_str, name, normalised_name: &key,
            lat: None, lon: None, source: None, created_at: &now,
        })
        .unwrap();
        id
    }
}
```

- [ ] **Step 6: Update `trips.rs`**

1. `build_new_trip` and `build_updated_trip` take `db: &Database` (`build_new_trip` gets it as a new first parameter) and `origin_place_id: &str, destination_place_id: &str` in place of `origin`/`destination`. They resolve both IDs:

```rust
/// The place a trip endpoint points at, or an error the UI can show.
fn resolve_place(db: &Database, id: &str) -> Result<crate::models::PlaceRow, String> {
    db.get_place(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Miesto neexistuje: {id}"))
}
```

Set `origin_place_id: Uuid::parse_str(&origin.id)`, and `origin: origin.name` (so the returned `Trip` carries the name). Do the same for the destination.

2. `create_trip_internal`, `update_trip_internal`, `create_trip_cascade_internal`, `update_trip_cascade_internal`: rename the two parameters to `origin_place_id: String, destination_place_id: String`, and pass `&trip.origin_place_id.to_string()` to `find_or_create_route`.
3. `get_inferred_trip_time_for_route_internal` and `inferred_trip_time_for_route`: rename the parameters to `origin_place_id`, `destination_place_id`. Delete the `normalize_location` calls at [trips.rs:1065-1066](../../src-tauri/core/src/commands_internal/trips.rs#L1065).

- [ ] **Step 7: Update `places_cmd.rs` `list_places_internal`**

Replace the fold with:

```rust
/// Every place in the book with its use count. Unplaced first (the work left
/// to do), then by use, then by name, so the order is stable across calls.
pub fn list_places_internal(db: &Database) -> Result<Vec<Place>, String> {
    let uses = db.place_uses().map_err(|e| e.to_string())?;
    let mut out: Vec<Place> = db
        .all_places()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|row| {
            let n = uses.get(&row.id).copied().unwrap_or(0);
            Place::from_row(row, n)
        })
        .collect();
    out.sort_by(|a, b| {
        a.lat.is_some().cmp(&b.lat.is_some())
            .then(b.uses.cmp(&a.uses))
            .then(a.name.cmp(&b.name))
    });
    Ok(out)
}
```

Delete `Folded`. Delete `save_place_internal` and `clear_place_internal`, their `save_place` and `clear_place` arms in `dispatcher.rs` ([dispatcher.rs:958-990](../../src-tauri/core/src/server/dispatcher.rs#L958)), and their exports. Task 4 adds the new commands. The frontend still calls the old ones until Task 5; that is the expected red integration state of the branch.

Delete the old place tests in this task, so the commit is green:
- In [places_cmd_tests.rs](../../src-tauri/core/src/commands_internal/places_cmd_tests.rs): the `place_at` helper (:16, it calls `upsert_place`), `only_row` (:222) and every test from :27 to :398 about the derived list, the spelling fold, `save_place` and `clear_place`, including `writes_are_refused_in_read_only_mode` (Task 4 adds a new one). Keep the geocoder section (:400 and after) and the module doc comment, rewritten without `upsert_place`.
- In [db_tests.rs](../../src-tauri/core/src/db_tests.rs): `upsert_place_replaces_the_row_rather_than_merging_it` (:898) and `delete_place_is_a_no_op_for_an_unknown_place` (:933), with their doc comments.
- The migration tests of Task 2 now own the spelling fold.

- [ ] **Step 8: Update `route_maps.rs`**

```rust
/// Loop when the row points at the same place twice, direct otherwise. Place
/// ids decide it now; no string fold is involved.
fn mode_for(trip: &Trip) -> RouteMode {
    if trip.origin_place_id == trip.destination_place_id {
        RouteMode::Loop
    } else {
        RouteMode::Direct
    }
}

/// The place `id` points at, when it has a coordinate.
fn placed_endpoint(places: &[Place], id: Uuid) -> Option<Place> {
    places.iter().find(|p| p.id == id && p.lat.is_some() && p.lon.is_some()).cloned()
}
```

In `start_route_for_trip_internal`, call `mode_for(&trip)` and `placed_endpoint(&places, trip.origin_place_id)`. Delete the `use crate::places::normalise;` import if nothing else uses it.

Update the doc comment at [route_maps.rs:367-372](../../src-tauri/core/src/commands_internal/route_maps.rs#L367). It says `mode_for` compares "NAMES after `places::normalise`" and names `save_place_internal`. Rewrite those lines: `mode_for` compares place IDs, and the book keys a place on `normalised_name` (UNIQUE) through `create_place_internal`, with no coordinate uniqueness. The rest of the comment (two places can hold identical coordinates) stays true.

Update [places/normalise.rs:8-10](../../src-tauri/core/src/places/normalise.rs#L8): delete the paragraph "Not [`crate::db::normalize_location`]: ...". The function is gone. Add one line: "Each place stores this as its `normalised_name`, which is UNIQUE (Task 88)."

`route_maps_tests.rs` imports `save_place_internal` (:12) and calls it at :701, :702, :717 and :731. Its local `seed_trip_between(&db, origin, destination)` inserts a trip by names; change its body to set the IDs with `ensure_place_for_test` (Step 9.3). Then:

1. `:12`: import only `build_trip_grid_data`.
2. `a_direct_row_carries_both_endpoints_from_the_book` (:698): replace the two `save_place_internal` lines with exact positions on the trip's own places:

   ```rust
   db.set_place_position(&trip.origin_place_id.to_string(), 48.1, 17.1, "manual").unwrap();
   db.set_place_position(&trip.destination_place_id.to_string(), 48.7, 21.2, "geocoder").unwrap();
   ```

3. `an_endpoint_is_found_regardless_of_spelling` (:714): **delete** it. A trip no longer holds a spelling, and the fold now happens once, when a place is created or found (`find_place_folds_case_and_diacritics` in Task 4, and the migration tests in Task 2).
4. `an_unplaced_endpoint_is_reported_not_refused` (:728): make the destination an unplaced place **before** the trip is seeded, so `ensure_place_for_test` in `seed_trip_between` finds it by key:

   ```rust
   let db = Database::in_memory().unwrap();
   db.ensure_unplaced_place_for_test("Depot, City B");
   let trip = seed_trip_between(&db, "Office, City A", "Depot, City B");

   let start = start_route_for_trip_internal(&db, trip.id.to_string()).unwrap();

   assert!(start.origin.is_some());
   assert!(start.destination.is_none(), "the unplaced endpoint reports as None");
   ```

5. `a_blank_endpoint_still_fails_the_whole_call` (:740): **delete** it. `mode_for` cannot fail now, and a blank endpoint cannot exist: `create_place` refuses a blank name, and the migration maps a blank endpoint to `Neznáme miesto` (Task 2 test).
6. The Loop test above (:690, `"Domov"` and `"domov "`) keeps working: both names give one key, so one place.

Read each other failing test in the file before you change it.

- [ ] **Step 9: Update the dispatcher and the remaining callers**

1. In `dispatcher.rs`, rename the Args fields `origin`/`destination` to `origin_place_id`/`destination_place_id` in `create_trip` (:130), `update_trip` (:176), `update_trip_cascade` (:222), `create_trip_cascade` (:288) and `get_inferred_trip_time_for_route` (:398). The JSON keys become `originPlaceId` and `destinationPlaceId`.
2. `statistics.rs:1534` (the `"Preview"` trip) and `export_cmd.rs:56` (the `"-"` first row) build in-memory `Trip`s: add `origin_place_id: Uuid::nil(), destination_place_id: Uuid::nil()`. At `statistics.rs:1564`, copy the IDs from `existing` too.
3. Fix each test that inserts a trip. There are about 60 call sites: `grep -rn "\.create_trip(&" src-tauri/core/src | grep -v "^src-tauri/core/src/db.rs\|commands_internal/trips.rs"`. After the literal fix above, the code compiles, but foreign keys are on, so an insert with a nil place ID fails at runtime. At each site, before the insert, set the IDs from the trip's old strings:

   ```rust
   trip.origin_place_id = db.ensure_place_for_test(&trip.origin);
   trip.destination_place_id = db.ensure_place_for_test(&trip.destination);
   db.create_trip(&trip).unwrap();
   ```

   The shared helper `seed_trip_between_on` ([db_tests.rs:149](../../src-tauri/core/src/db_tests.rs#L149)) takes names: change its body the same way, and its callers (including `seed_trip_between` and the place tests) need no change. In `db_tests.rs`, use `insert_trip_with_places` from Step 1. A file with many sites can have its own local helper with the same body. Do the same for `create_trip_with_odometer_shift` and the other insert paths (`grep -rn "create_trip_with_odometer_shift(&\|update_trip(&" src-tauri/core/src --include=*_tests.rs`): an update with a nil ID also fails.
4. A test that calls a `*_trip_internal` function with names: pass `db.ensure_place_for_test("<name>").to_string()`.
5. A test that compares `Route.origin` or `Trip.origin` keeps working: the name is filled by the read.

- [ ] **Step 10: Run the whole backend suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
Expected: all tests pass, including the Task 2 migration tests and `test_migrated_schema_identical_to_fresh_schema`.

Then run: `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 11: Check the migration on a copy of real-shaped data**

Build a scratch DB with the old schema and invented rows (two spellings of one place, one string without coordinates, one orphan route), start the server on it, and call `list_places`:

```bash
export KNIHA_JAZD_DATA_DIR=$(mktemp -d)
# create the DB with the previous commit's binary, add invented trips, then:
cargo run --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web &
curl -s -X POST localhost:3456/api/rpc -H 'Content-Type: application/json' \
  -d '{"command":"list_places","args":{}}'
```

Expected: one entry per key, with `id`, `name` and `uses`. Stop the server.

- [ ] **Step 12: Commit Tasks 2 and 3 together**

```bash
git add src-tauri/core/migrations/2026-10-05-100000_places_as_entities \
  src-tauri/core/src/schema.rs src-tauri/core/src/models.rs src-tauri/core/src/db.rs \
  src-tauri/core/src/db_tests.rs src-tauri/core/src/migration_tests.rs \
  src-tauri/core/src/commands_internal/ src-tauri/core/src/calculations/ \
  src-tauri/core/src/server/ src-tauri/core/src/export_tests.rs src-tauri/core/src/invoice_tests.rs \
  src-tauri/core/src/places/normalise.rs
git status --short   # confirm nothing outside this task is staged
git commit -m "feat(places)!: trips and routes reference places by id"
```

---

### Task 4: Place commands

**Files:**
- Modify: [commands_internal/places_cmd.rs](../../src-tauri/core/src/commands_internal/places_cmd.rs), [db.rs](../../src-tauri/core/src/db.rs), [server/dispatcher.rs:950-990](../../src-tauri/core/src/server/dispatcher.rs#L950), [commands_internal/mod.rs](../../src-tauri/core/src/commands_internal/mod.rs) (exports)
- Test: [commands_internal/places_cmd_tests.rs](../../src-tauri/core/src/commands_internal/places_cmd_tests.rs)

**Interfaces:**
- Consumes: Task 3 DB functions.
- Produces:
  - `create_place_internal(db, app_state, name: String, lat: f64, lon: f64, source: PlaceSource) -> Result<Place, String>`
  - `rename_place_internal(db, app_state, id: String, name: String) -> Result<Place, String>`
  - `set_place_position_internal(db, app_state, id: String, lat: f64, lon: f64, source: PlaceSource) -> Result<Place, String>`
  - `delete_place_internal(db, app_state, id: String) -> Result<(), String>`
  - `find_place_internal(db, name: String) -> Result<Option<Place>, String>` (matches by `normalise(name)`)
  - `Database::delete_place_if_unused(&self, id: &str) -> QueryResult<DeletePlaceOutcome>` with `enum DeletePlaceOutcome { Deleted, InUse(i64), NotFound }`.
  - RPC: `create_place`, `rename_place`, `set_place_position`, `delete_place`, `find_place`. `save_place` and `clear_place` are gone.

- [ ] **Step 1: Write the failing tests**

Task 3 already deleted the old place tests. Add these to `places_cmd_tests.rs`. The helpers are real: `AppState::new()`, `AppState::enable_read_only(&str)` ([app_state.rs:160](../../src-tauri/core/src/app_state.rs#L160)), and `create_test_vehicle` and `seed_trip_between` from `crate::db_tests` (already imported at the top of the file).

```rust
#[test]
fn a_place_needs_a_name() {
    let db = Database::in_memory().unwrap();
    let err = create_place_internal(&db, &AppState::new(), "   ".into(), 48.1, 17.1, PlaceSource::Manual).unwrap_err();
    assert_eq!(err, "Miesto musí mať názov");
}

#[test]
fn create_returns_the_place_with_an_id_and_zero_uses() {
    let db = Database::in_memory().unwrap();
    let p = create_place_internal(&db, &AppState::new(), " Nitra ".into(), 48.3, 18.1, PlaceSource::Geocoder).unwrap();
    assert_eq!(p.name, "Nitra", "the name is trimmed");
    assert_eq!(p.uses, 0);
    assert_eq!(list_places_internal(&db).unwrap().len(), 1);
}

#[test]
fn a_second_place_with_the_same_key_is_refused() {
    let db = Database::in_memory().unwrap();
    let app = AppState::new();
    create_place_internal(&db, &app, "Košice".into(), 48.7, 21.2, PlaceSource::Manual).unwrap();
    let err = create_place_internal(&db, &app, "kosice".into(), 48.7, 21.2, PlaceSource::Manual).unwrap_err();
    assert!(err.contains("Košice"), "the error names the existing place: {err}");
}

#[test]
fn a_rename_that_only_changes_case_or_diacritics_is_allowed() {
    let db = Database::in_memory().unwrap();
    let app = AppState::new();
    let p = create_place_internal(&db, &app, "Kosice".into(), 48.7, 21.2, PlaceSource::Manual).unwrap();
    let renamed = rename_place_internal(&db, &app, p.id.to_string(), "Košice".into()).unwrap();
    assert_eq!(renamed.name, "Košice");
}

#[test]
fn a_rename_onto_another_place_is_refused() {
    let db = Database::in_memory().unwrap();
    let app = AppState::new();
    create_place_internal(&db, &app, "Nitra".into(), 48.3, 18.1, PlaceSource::Manual).unwrap();
    let b = create_place_internal(&db, &app, "Levice".into(), 48.2, 18.6, PlaceSource::Manual).unwrap();
    let err = rename_place_internal(&db, &app, b.id.to_string(), "NITRA".into()).unwrap_err();
    assert!(err.contains("Nitra"), "{err}");
}

#[test]
fn a_place_a_trip_uses_cannot_be_deleted() {
    let db = Database::in_memory().unwrap();
    let vehicle = create_test_vehicle("Car");
    db.create_vehicle(&vehicle).unwrap();
    let trip = seed_trip_between(&db, &vehicle.id, "Nitra", "Levice");
    let err = delete_place_internal(&db, &AppState::new(), trip.origin_place_id.to_string()).unwrap_err();
    assert!(err.contains('1'), "the error gives the trip count: {err}");
}

#[test]
fn a_place_only_an_orphan_route_uses_can_be_deleted() {
    let db = Database::in_memory().unwrap();
    let vehicle = create_test_vehicle("Car");
    db.create_vehicle(&vehicle).unwrap();
    let v = vehicle.id.to_string();
    let a = db.ensure_place_for_test("Nitra").to_string();
    let b = db.ensure_place_for_test("Levice").to_string();
    db.find_or_create_route(&v, &a, &b, 40.0).unwrap();
    delete_place_internal(&db, &AppState::new(), a.clone()).unwrap();
    assert!(db.get_place(&a).unwrap().is_none());
    assert!(db.all_route_rows_for_test(&v).unwrap().is_empty(), "the orphan route goes with it");
}

#[test]
fn find_place_folds_case_and_diacritics() {
    let db = Database::in_memory().unwrap();
    create_place_internal(&db, &AppState::new(), "Bratislava".into(), 48.1, 17.1, PlaceSource::Manual).unwrap();
    let found = find_place_internal(&db, "  bratislava ".into()).unwrap().unwrap();
    assert_eq!(found.name, "Bratislava");
    assert!(find_place_internal(&db, "Senec".into()).unwrap().is_none());
}

#[test]
fn set_position_moves_the_place_and_keeps_its_id() {
    let db = Database::in_memory().unwrap();
    let app = AppState::new();
    let p = create_place_internal(&db, &app, "Nitra".into(), 48.3, 18.1, PlaceSource::Geocoder).unwrap();
    let moved = set_place_position_internal(&db, &app, p.id.to_string(), 48.31, 18.09, PlaceSource::Manual).unwrap();
    assert_eq!((moved.id, moved.lat, moved.source), (p.id, Some(48.31), Some(PlaceSource::Manual)));
}

/// The only test of the "needs a position" marker data. No RPC can make an
/// unplaced place (only the migration does), and a test-only RPC must not
/// exist on a live server, so the marker itself has no integration test.
#[test]
fn an_unplaced_place_is_listed_first_with_null_coordinates() {
    let db = Database::in_memory().unwrap();
    create_place_internal(&db, &AppState::new(), "Nitra".into(), 48.3, 18.1, PlaceSource::Manual).unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();
    db.insert_place(&NewPlaceRow {
        id: &id, name: "Stará adresa", normalised_name: "stara adresa",
        lat: None, lon: None, source: None, created_at: &now,
    })
    .unwrap();
    let list = list_places_internal(&db).unwrap();
    assert_eq!(list[0].name, "Stará adresa");
    assert_eq!((list[0].lat, list[0].source), (None, None));
}

#[test]
fn place_writes_are_refused_in_read_only_mode() {
    let db = Database::in_memory().unwrap();
    let ro = AppState::new();
    ro.enable_read_only("Test read-only");
    let err = create_place_internal(&db, &ro, "Nitra".into(), 48.3, 18.1, PlaceSource::Manual).unwrap_err();
    assert!(err.contains("len na čítanie"), "got: {err}");
    assert!(db.all_places().unwrap().is_empty(), "a read-only create must not write");
}
```

Remove the now unused imports at the top of the file (`PlaceRow`, `normalise`) if the compiler warns.

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core places_cmd`
Expected: compile errors for the missing functions.

- [ ] **Step 3: Write `delete_place_if_unused` in `db.rs`**

```rust
#[derive(Debug, PartialEq)]
pub enum DeletePlaceOutcome {
    Deleted,
    InUse(i64),
    NotFound,
}

/// Delete a place that no trip points at. Routes on it go too: a route with
/// no trip is invisible (get_routes_for_vehicle joins trips), so it is not a
/// use a person could see or act on.
pub fn delete_place_if_unused(&self, id: &str) -> QueryResult<DeletePlaceOutcome> {
    let conn = &mut *self.conn.lock().unwrap();
    conn.transaction(|tx| {
        let uses: i64 = trips::table
            .filter(trips::origin_place_id.eq(id).or(trips::destination_place_id.eq(id)))
            .count()
            .get_result(tx)?;
        if uses > 0 {
            return Ok(DeletePlaceOutcome::InUse(uses));
        }
        diesel::delete(
            routes::table.filter(routes::origin_place_id.eq(id).or(routes::destination_place_id.eq(id))),
        )
        .execute(tx)?;
        let n = diesel::delete(places::table.filter(places::id.eq(id))).execute(tx)?;
        Ok(if n == 1 { DeletePlaceOutcome::Deleted } else { DeletePlaceOutcome::NotFound })
    })
}
```

`uses` here counts trips, not endpoints. That is enough for the error message.

- [ ] **Step 4: Write the commands in `places_cmd.rs`**

```rust
fn place_by_id(db: &Database, id: &str) -> Result<Place, String> {
    let row = db.get_place(id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Miesto neexistuje: {id}"))?;
    let uses = db.place_uses().map_err(|e| e.to_string())?.get(id).copied().unwrap_or(0);
    Ok(Place::from_row(row, uses))
}

/// The trimmed name and its key, or the error for a blank name.
fn name_and_key(name: &str) -> Result<(String, String), String> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let key = normalise(&name);
    if key.is_empty() {
        return Err("Miesto musí mať názov".to_string());
    }
    Ok((name, key))
}

/// Refuse a key that another place already holds. `except` is the place
/// being renamed: its own key is not a collision.
fn ensure_key_free(db: &Database, key: &str, except: Option<&str>) -> Result<(), String> {
    match db.get_place_by_key(key).map_err(|e| e.to_string())? {
        Some(other) if Some(other.id.as_str()) != except => {
            Err(format!("Miesto s týmto názvom už existuje: {}", other.name))
        }
        _ => Ok(()),
    }
}

pub fn create_place_internal(
    db: &Database, app_state: &AppState,
    name: String, lat: f64, lon: f64, source: PlaceSource,
) -> Result<Place, String> {
    check_read_only!(app_state);
    let (name, key) = name_and_key(&name)?;
    ensure_key_free(db, &key, None)?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    db.insert_place(&NewPlaceRow {
        id: &id, name: &name, normalised_name: &key,
        lat: Some(lat), lon: Some(lon), source: Some(source.as_str()), created_at: &now,
    })
    .map_err(|e| e.to_string())?;
    place_by_id(db, &id)
}

pub fn rename_place_internal(
    db: &Database, app_state: &AppState, id: String, name: String,
) -> Result<Place, String> {
    check_read_only!(app_state);
    let (name, key) = name_and_key(&name)?;
    ensure_key_free(db, &key, Some(&id))?;
    if db.rename_place(&id, &name, &key).map_err(|e| e.to_string())? != 1 {
        return Err(format!("Miesto neexistuje: {id}"));
    }
    place_by_id(db, &id)
}

pub fn set_place_position_internal(
    db: &Database, app_state: &AppState,
    id: String, lat: f64, lon: f64, source: PlaceSource,
) -> Result<Place, String> {
    check_read_only!(app_state);
    if db.set_place_position(&id, lat, lon, source.as_str()).map_err(|e| e.to_string())? != 1 {
        return Err(format!("Miesto neexistuje: {id}"));
    }
    place_by_id(db, &id)
}

pub fn delete_place_internal(db: &Database, app_state: &AppState, id: String) -> Result<(), String> {
    check_read_only!(app_state);
    match db.delete_place_if_unused(&id).map_err(|e| e.to_string())? {
        DeletePlaceOutcome::Deleted => Ok(()),
        DeletePlaceOutcome::InUse(n) => Err(format!("Miesto používa {n} jázd, nedá sa zmazať")),
        DeletePlaceOutcome::NotFound => Err(format!("Miesto neexistuje: {id}")),
    }
}

/// The place whose key equals `normalise(name)`. The trip form calls this
/// for typed text, so the fold stays in Rust (ADR-008).
pub fn find_place_internal(db: &Database, name: String) -> Result<Option<Place>, String> {
    let key = normalise(&name);
    if key.is_empty() {
        return Ok(None);
    }
    match db.get_place_by_key(&key).map_err(|e| e.to_string())? {
        Some(row) => place_by_id(db, &row.id).map(Some),
        None => Ok(None),
    }
}
```

Task 3 already deleted `save_place_internal` and `clear_place_internal`. Export the new functions from `commands_internal/mod.rs` the way `list_places_internal` is exported today.

- [ ] **Step 5: Register the RPC commands**

In `dispatcher.rs`, add these arms where the `save_place` and `clear_place` arms were (Task 3 deleted them):

```rust
"create_place" => {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Args { name: String, lat: f64, lon: f64, source: crate::models::PlaceSource }
    let a: Args = parse_args(args)?;
    let v = crate::commands_internal::create_place_internal(&state.db, &state.app_state, a.name, a.lat, a.lon, a.source)?;
    Ok(serde_json::to_value(v).unwrap())
}
"rename_place" => {
    #[derive(serde::Deserialize)]
    struct Args { id: String, name: String }
    let a: Args = parse_args(args)?;
    let v = crate::commands_internal::rename_place_internal(&state.db, &state.app_state, a.id, a.name)?;
    Ok(serde_json::to_value(v).unwrap())
}
"set_place_position" => {
    #[derive(serde::Deserialize)]
    struct Args { id: String, lat: f64, lon: f64, source: crate::models::PlaceSource }
    let a: Args = parse_args(args)?;
    let v = crate::commands_internal::set_place_position_internal(&state.db, &state.app_state, a.id, a.lat, a.lon, a.source)?;
    Ok(serde_json::to_value(v).unwrap())
}
"delete_place" => {
    #[derive(serde::Deserialize)]
    struct Args { id: String }
    let a: Args = parse_args(args)?;
    crate::commands_internal::delete_place_internal(&state.db, &state.app_state, a.id)?;
    Ok(serde_json::to_value(()).unwrap())
}
"find_place" => {
    #[derive(serde::Deserialize)]
    struct Args { name: String }
    let a: Args = parse_args(args)?;
    let v = crate::commands_internal::find_place_internal(&state.db, a.name)?;
    Ok(serde_json::to_value(v).unwrap())
}
```

If the dispatcher has a list of command names (for example a test that checks every arm, or a read-only command list), add the five names there too, and check that `save_place` and `clear_place` are gone: `grep -rn "\"save_place\"" src-tauri/core/src`.

- [ ] **Step 6: Run the tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/core/src/commands_internal/places_cmd.rs src-tauri/core/src/commands_internal/places_cmd_tests.rs \
  src-tauri/core/src/commands_internal/mod.rs src-tauri/core/src/db.rs src-tauri/core/src/server/dispatcher.rs
git commit -m "feat(places): create, rename, position, delete and find commands"
```

---

### Task 5: Frontend API, trip writes by ID, and the test helpers

**Files:**
- Modify: [src/lib/types.ts](../../src/lib/types.ts) (`Trip` :37, `Place` :616, `Route`, `CopiedTripDefaults`), [src/lib/api.ts](../../src/lib/api.ts) (:96, :165, :402, :667-687), [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte), [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte), [src/routes/mapa/+page.svelte](../../src/routes/mapa/+page.svelte) (:18, :44, :824-833, :957-959), [src/lib/components/PlaceModal.svelte](../../src/lib/components/PlaceModal.svelte) (:46, :223, :229)
- Modify: [tests/integration/utils/db.ts](../../tests/integration/utils/db.ts) (:160-262, :493), [tests/integration/utils/forms.ts](../../tests/integration/utils/forms.ts) (:367)

**Interfaces:**
- Consumes: the RPC commands from Tasks 3 and 4.
- Produces:
  - `Place { id: string; name: string; normalisedName: string; lat: number | null; lon: number | null; source: PlaceSource | null; uses: number }`
  - `Trip` gets `originPlaceId: string; destinationPlaceId: string`. `Route` and `CopiedTripDefaults` get the same.
  - `api.ts`: `createPlace(name, lat, lon, source): Promise<Place>`, `renamePlace(id, name): Promise<Place>`, `setPlacePosition(id, lat, lon, source): Promise<Place>`, `deletePlace(id): Promise<void>`, `findPlace(name): Promise<Place | null>`. `savePlace` and `clearPlace` are gone. `createTripCascade`, `updateTripCascade` and `getInferredTripTimeForRoute` take `originPlaceId`/`destinationPlaceId`.
  - Integration helper `ensurePlace(name: string): Promise<string>` (returns the place ID) in `utils/db.ts`.

- [ ] **Step 1: Update the integration helpers first**

In `tests/integration/utils/db.ts`, add:

```ts
/**
 * The id of the place called `name`, created at a fixed position if missing.
 * Lets specs keep naming places as strings (Task 88). No cache: the DB reset
 * between specs deletes places, so a cached id would point at nothing.
 */
export async function ensurePlace(name: string): Promise<string> {
  const found = await rpc<{ id: string } | null>('find_place', { name });
  if (found) return found.id;
  const created = await rpc<{ id: string }>('create_place', {
    name, lat: 48.15, lon: 17.11, source: 'manual',
  });
  return created.id;
}
```

In [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) `resetDatabase` (:174), after the loop that deletes trips and before the vehicles are deleted, delete the places:

```ts
    // Task 88: places are records now and outlive their trips. Delete them so
    // each spec starts with an empty Miesta list. A place that a trip outside
    // the three checked years still uses refuses the delete; ignore that.
    const places = await rpc('list_places') as Array<{ id: string }>;
    for (const p of places) {
      try {
        await rpc('delete_place', { id: p.id });
      } catch { /* ignore: still in use */ }
    }
```

The reset deletes trips for only three years. So a spec must not assert an exact place count. Assert by `data-place-id` (Task 7 does).

In `seedTrip`, replace `origin: data.origin, destination: data.destination` with:

```ts
    originPlaceId: await ensurePlace(data.origin),
    destinationPlaceId: await ensurePlace(data.destination),
```

`SeedTripData` keeps `origin: string; destination: string`, so the about 217 call sites do not change.

In `forms.ts` `fillTripForm`, before the two `fillField` calls for origin and destination, add `await ensurePlace(origin); await ensurePlace(destination);`, so the place exists when the form looks it up. Import `ensurePlace` from `./db`.

- [ ] **Step 2: Update `types.ts` and `api.ts`**

Replace the `Place` interface and its comment:

```ts
/**
 * One place in the book (Task 88). `lat`/`lon` are null only for a place the
 * migration made from a trip string without a coordinate.
 * `uses` counts trip endpoints: A -> B adds 1 to each place.
 */
export interface Place {
  id: string;
  name: string;
  normalisedName: string;
  lat: number | null;
  lon: number | null;
  source: PlaceSource | null;
  uses: number;
}
```

Add `originPlaceId: string; destinationPlaceId: string;` to `Trip`, `Route` and `CopiedTripDefaults`. Replace `savePlace`/`clearPlace` in `api.ts`:

```ts
export async function createPlace(name: string, lat: number, lon: number, source: PlaceSource): Promise<Place> {
  return await apiCall('create_place', { name, lat, lon, source });
}
export async function renamePlace(id: string, name: string): Promise<Place> {
  return await apiCall('rename_place', { id, name });
}
export async function setPlacePosition(id: string, lat: number, lon: number, source: PlaceSource): Promise<Place> {
  return await apiCall('set_place_position', { id, lat, lon, source });
}
export async function deletePlace(id: string): Promise<void> {
  return await apiCall('delete_place', { id });
}
export async function findPlace(name: string): Promise<Place | null> {
  return await apiCall('find_place', { name });
}
```

In `createTripCascade`, `updateTripCascade` and `getInferredTripTimeForRoute`, rename the `origin`/`destination` parameters and keys to `originPlaceId`/`destinationPlaceId`.

- [ ] **Step 3: Update `TripRow.svelte` and `TripGrid.svelte`**

In `TripRow.svelte`:
1. Add `originPlaceId` and `destinationPlaceId` to `formData` (seeded from `trip?.originPlaceId ?? ''`, and from the copied defaults in `copyFrom`).
2. `locationSuggestions = places.map((p) => p.name).sort();`
3. Resolve each ID by **exact** name match against the loaded list. `Autocomplete` has only `onSelect`, and a select also changes `formData.origin`, so one reactive statement covers both a select and typed text:

   ```ts
   // The id of the place whose name the field holds exactly, or ''. An exact
   // match against the list the user picks from is not a fold: typed text in
   // another case or spelling stays '' here and save() asks find_place
   // (Task 6), so the fold stays in Rust (ADR-008).
   $: formData.originPlaceId = places.find((p) => p.name === formData.origin)?.id ?? '';
   $: formData.destinationPlaceId = places.find((p) => p.name === formData.destination)?.id ?? '';
   ```

   Do not clear or set the IDs in `handleOriginSelect`/`handleDestinationSelect`; they keep calling `tryAutoFillDistance()` and `tryInferTimes()`.
4. `tryAutoFillDistance` matches `r.originPlaceId === formData.originPlaceId && r.destinationPlaceId === formData.destinationPlaceId`, and returns early if either ID is `''`.
5. `tryInferTimes` builds the key from the two IDs, returns early if either is `''`, and passes them to `getInferredTripTimeForRoute`.
6. `onSave` sends `originPlaceId` and `destinationPlaceId`. Task 6 adds the `find_place` call for an ID that is still `''` at save time.

In `TripGrid.svelte`, pass `originPlaceId`/`destinationPlaceId` from the saved row to `createTripCascade`/`updateTripCascade`.

- [ ] **Step 4: Update `/mapa` and `PlaceModal`**

1. `PlaceModal.svelte`: `place.displayName` becomes `place.name` (:46, :223, :229). Remove the `onClear` prop and the clear button: a place keeps its position (spec 3). Find the button with `grep -n "onClear\|place-clear" src/lib/components/PlaceModal.svelte`.
2. `/mapa`: the shell for an unplaced endpoint (:828-833) uses the trip's `originPlaceId`/`destinationPlaceId` and `name: trip.origin`. The save at :957-959 calls `setPlacePosition(placeId, coords.lat, coords.lon, coords.source)`. Rename the local `Endpoint.displayName` to `name`.

- [ ] **Step 5: Check types and build**

Run: `npm run i18n && npm run check && npm run typecheck:tests`
Expected: 0 errors.

Before you run it, **remove the place section from Settings in this task**. `settings/+page.svelte` still calls `savePlace` (:910) and `clearPlace` (:924), so `npm run check` fails otherwise. Delete the place state and handlers (:841-940), the markup (:1379-1460), the `PlaceModal` block (:1640-1660), the place CSS, and the imports only that code used (`PlaceModal`, `listPlaces`, `savePlace`, `clearPlace`, `Place`, `PlaceSource`). Task 7 builds the Miesta tab from this code: read it back with `git show HEAD~1:src/routes/settings/+page.svelte` (the commit before this task's commit), or copy it to a scratch file now. Until Task 7, `places.spec.ts` fails, because Settings has no place section. That is part of the red integration state of the branch.

- [ ] **Step 6: Run the trip-flow specs**

```bash
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier1/trip-management.spec.ts \
  --spec tests/integration/specs/tier2/route-autocomplete.spec.ts \
  --spec tests/integration/specs/tier2/time-inference-toggle.spec.ts \
  --spec tests/integration/specs/tier2/copy-trip.spec.ts \
  --spec tests/integration/specs/tier2/route-map.spec.ts
```

Expected: PASS. `route-map.spec.ts` may expect an unplaced endpoint that a seeded trip can no longer create (`ensurePlace` always sets a position). If a test needs an unplaced place, mark it `it.skip` with the comment `// Task 88: an unplaced place now exists only after the migration; the backend migration tests cover it`, and list it in the PR.

- [ ] **Step 7: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts src/lib/components/TripRow.svelte src/lib/components/TripGrid.svelte \
  src/routes/settings/+page.svelte \
  src/lib/components/PlaceModal.svelte src/routes/mapa/+page.svelte \
  tests/integration/utils/db.ts tests/integration/utils/forms.ts tests/integration/wdio.server.conf.ts
git commit -m "feat(trips): the trip form saves place ids"
```

---

### Task 6: The trip form accepts only existing places

**Files:**
- Modify: [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte), [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts), [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts)
- Test: [tests/integration/specs/tier2/places.spec.ts](../../tests/integration/specs/tier2/places.spec.ts)

**Interfaces:**
- Consumes: `findPlace(name)` from Task 5.
- Produces: i18n keys `trips.unknownPlace({ name })` and `trips.unknownPlaceHint()`. Test ID `trip-place-error`.

- [ ] **Step 1: Write the failing integration tests**

In `places.spec.ts`, add this block. It uses the file's own `openNewTripRow()` (:194), `YEAR` (:98) and the `beforeEach` reset (:209), and the shared helpers `seedVehicle`, `setActiveVehicle`, `navigateTo`, `waitForTripGrid`, `fillTripForm`, `fillField`, `getTripGridData`, `ensurePlace` and `TripGrid` (`saveTripBtn: 'button*=Save'`, `tripForm.origin`). Add the missing imports: `fillTripForm`, `fillField` from `../../utils/forms`; `getTripGridData`, `ensurePlace` from `../../utils/db`; `TripGrid` from `../../utils/assertions`.

```ts
describe('Trip Form Uses Existing Places', () => {
  async function openFormForNewVehicle(plate: string): Promise<string> {
    const vehicle = await seedVehicle({
      name: `Place Form ${plate}`,
      licensePlate: plate,
      initialOdometer: 10000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    await setActiveVehicle(vehicle.id as string);
    await navigateTo('trips');
    await waitForTripGrid();
    await openNewTripRow();
    return vehicle.id as string;
  }

  it('should block a save when the origin names no place', async () => {
    const vehicleId = await openFormForNewVehicle('PLC-F01');
    await fillTripForm({
      startDatetime: `${YEAR}-03-01T08:00`,
      origin: 'Iota Square, Testville', // fillTripForm calls ensurePlace
      destination: 'Iota Square, Testville',
      distanceKm: 5,
      purpose: 'Business trip',
    });
    // Overwrite the origin with text that matches no place.
    await fillField(TripGrid.tripForm.origin, 'Nowhere At All');
    await (await $(TripGrid.saveTripBtn)).click();

    const error = await $('[data-testid="trip-place-error"]');
    await error.waitForDisplayed({ timeout: 5000 });
    expect(await error.getText()).toContain('Nowhere At All');
    expect((await getTripGridData(vehicleId, YEAR)).trips.length).toBe(0);
  });

  it('should save typed text that matches a place in another case', async () => {
    await ensurePlace('Kappa Plaza, Testville');
    const vehicleId = await openFormForNewVehicle('PLC-F02');
    await fillTripForm({
      startDatetime: `${YEAR}-03-01T08:00`,
      origin: 'Kappa Plaza, Testville',
      destination: 'Kappa Plaza, Testville',
      distanceKm: 5,
      purpose: 'Business trip',
    });
    await fillField(TripGrid.tripForm.origin, 'KAPPA PLAZA, TESTVILLE');
    await (await $(TripGrid.saveTripBtn)).click();

    await browser.waitUntil(
      async () => (await getTripGridData(vehicleId, YEAR)).trips.length === 1,
      { timeout: 10000, timeoutMsg: 'the trip with a case-folded origin was not saved' }
    );
    expect((await getTripGridData(vehicleId, YEAR)).trips[0].origin).toBe('Kappa Plaza, Testville');
  });
});
```

Check the `TripGridData` shape before you run it: `grep -n "interface TripGridData" -A6 src/lib/types.ts`. If the trip list field is not `trips`, use the real name. If the UI language in this spec is Slovak, the save button text is not "Save": the file calls `ensureLanguage` (:33), so check which language it sets, and use the `TripGrid` selector that matches.

- [ ] **Step 2: Run them and see them fail**

```bash
npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
npx wdio run tests/integration/wdio.server.conf.ts --spec tests/integration/specs/tier2/places.spec.ts
```

Expected: the two new tests FAIL (no `trip-place-error` element; the typed text is not resolved).

- [ ] **Step 3: Write the code**

In `TripRow.svelte`, before `onSave`:

```ts
let placeError = '';

/** Resolve each endpoint to a place id. Picked items carry their id;
 *  typed text asks the backend, which folds case and diacritics. */
async function resolveEndpoints(): Promise<boolean> {
  placeError = '';
  for (const field of ['origin', 'destination'] as const) {
    const idKey = field === 'origin' ? 'originPlaceId' : 'destinationPlaceId';
    if (formData[idKey]) continue;
    const found = await findPlace(formData[field]);
    if (!found) {
      placeError = $LL.trips.unknownPlace({ name: formData[field] });
      return false;
    }
    formData[idKey] = found.id;
    formData[field] = found.name;
  }
  return true;
}
```

Call `if (!(await resolveEndpoints())) return;` at the start of the save handler. Render the error in the row:

```svelte
{#if placeError}
  <div class="place-error" data-testid="trip-place-error" role="alert">
    {placeError} <a href="/miesta">{$LL.trips.unknownPlaceHint()}</a>
  </div>
{/if}
```

Add the i18n keys:

```ts
// sk
unknownPlace: 'Miesto "{name:string}" neexistuje.',
unknownPlaceHint: 'Pridajte ho na karte Miesta.',
// en
unknownPlace: 'The place "{name:string}" does not exist.',
unknownPlaceHint: 'Add it on the Places tab.',
```

- [ ] **Step 4: Run the tests**

Run: `npm run i18n && npm run check`, then the `places.spec.ts` command from Step 2.
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/TripRow.svelte src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts \
  src/lib/i18n/i18n-types.ts tests/integration/specs/tier2/places.spec.ts
git commit -m "feat(trips): a trip accepts only an existing place"
```

---

### Task 7: The Miesta tab

**Files:**
- Create: `src/routes/miesta/+page.svelte`
- Modify: [src/routes/+layout.svelte:127-133](../../src/routes/+layout.svelte#L127), [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts), [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts)
- Test: [tests/integration/specs/tier2/places.spec.ts](../../tests/integration/specs/tier2/places.spec.ts)

**Interfaces:**
- Consumes: `listPlaces`, `createPlace`, `renamePlace`, `setPlacePosition`, `deletePlace`, `geocodePlace`, `PlaceModal`.
- Produces: route `/miesta`. Test IDs: `places-list`, `places-filter`, `place-row` (with `data-place-id`), `place-name`, `place-uses`, `place-unplaced-icon`, `place-edit`, `place-rename`, `place-rename-input`, `place-rename-save`, `place-delete`, `place-add`, `place-add-name`, `nav-places`. i18n `app.nav.places()` and new keys under `places.*`.

- [ ] **Step 1: Rewrite `places.spec.ts` for the tab, with failing tests**

1. Each `browser.url('/settings')` plus scroll to `places-section` becomes `browser.url('/miesta')`. Each `place-item` becomes `place-row`. Each `data-place-name` lookup becomes a lookup by `data-place-id` (get the ID from `ensurePlace`).
2. Delete `should list the places of a seeded trip, marked unplaced` and the clear half of `should pin a place by hand and then clear it again`: a seeded place now has a position, and a place keeps its position.
3. Add:

```ts
describe('Managing Places', () => {
  it('should add a place, then offer it in the trip form', async () => {
    await browser.url('/miesta');
    await (await $('[data-testid="place-add"]')).click();
    // GEOCODED_PLACE is the name the geocoder mock has an answer for (:45).
    await fillField('[data-testid="place-add-name"]', GEOCODED_PLACE);

    // The add flow opens PlaceModal with the name as the query, the same as
    // 'should store a picked candidate and still show it after a reload'.
    const modal = await $('[data-testid="place-modal"]');
    await modal.waitForDisplayed({ timeout: 10000 });
    expect(await $('[data-testid="place-search-input"]').getValue()).toBe(GEOCODED_PLACE);
    const save = await $('[data-testid="place-modal-save"]');
    expect(await save.isEnabled()).toBe(false); // no position yet, no save

    await $('[data-testid="place-search-submit"]').click();
    const candidateList = await $('[data-testid="place-candidates"]');
    await candidateList.waitForDisplayed({ timeout: 10000 });
    const candidates = await $$('[data-testid="place-candidate"]');
    await candidates[0].click();
    expect(await $('[data-testid="place-modal-coords"]').getText()).toBe(GEOCODED_COORDS);
    await save.click();
    await modal.waitForDisplayed({ timeout: 10000, reverse: true });

    // Not a count: the reset keeps a place that a trip outside its three
    // years still uses. Look the new place up by id.
    const added = await rpc<{ id: string } | null>('find_place', { name: GEOCODED_PLACE });
    expect(added).not.toBeNull();
    await (await $(`[data-testid="place-row"][data-place-id="${added!.id}"]`)).waitForDisplayed();

    // The trip form offers it.
    const vehicle = await seedVehicle({
      name: 'Place Add Vehicle', licensePlate: 'PLC-A01',
      initialOdometer: 10000, tankSizeLiters: 50, tpConsumption: 6.5,
    });
    await setActiveVehicle(vehicle.id as string);
    await navigateTo('trips');
    await waitForTripGrid();
    await openNewTripRow();
    const originInput = await $('[data-testid="trip-origin"]');
    await originInput.click();
    await originInput.setValue('Gamma');
    await browser.waitUntil(
      async () => (await $('.autocomplete .dropdown')).isDisplayed(),
      { timeout: 5000, timeoutMsg: 'the new place was not offered' }
    );
    const suggestions = await $$('.autocomplete .dropdown .suggestion');
    const offered: string[] = [];
    for (const suggestion of suggestions) {
      offered.push((await suggestion.getText()).trim());
    }
    expect(offered).toContain(GEOCODED_PLACE);
  });

  it('should rename a place and show the new name in the trip grid', async () => {
    const id = await ensurePlace('Nitra');
    await seedTrip({ vehicleId, startDatetime: `${year}-03-01T08:00`, origin: 'Nitra',
                     destination: 'Nitra', distanceKm: 5, odometer: 1005, purpose: 'p' });
    await browser.url('/miesta');
    const row = await $(`[data-testid="place-row"][data-place-id="${id}"]`);
    await (await row.$('[data-testid="place-rename"]')).click();
    await fillField('[data-testid="place-rename-input"]', 'Nitra - centrum');
    await (await $('[data-testid="place-rename-save"]')).click();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.waitUntil(
      async () => (await $(TripGrid.dataRows).getText()).includes('Nitra - centrum'),
      { timeout: 10000, timeoutMsg: 'the renamed place did not show in the grid' }
    );
  });

  it('should disable delete for a place in use', async () => {
    const id = await ensurePlace('Nitra');
    await seedTrip({ vehicleId, startDatetime: `${year}-03-01T08:00`, origin: 'Nitra',
                     destination: 'Nitra', distanceKm: 5, odometer: 1005, purpose: 'p' });
    await browser.url('/miesta');
    const del = await $(`[data-testid="place-row"][data-place-id="${id}"] [data-testid="place-delete"]`);
    expect(await del.isEnabled()).toBe(false);
  });
});
```

The rename and delete tests need a `vehicleId` and an active vehicle: seed one with `seedVehicle` and call `setActiveVehicle` at the start of each test, as the add test does. Add `'miesta'` to the `navigateTo` path map in [utils/app.ts:26](../../tests/integration/utils/app.ts#L26) (`miesta: '/miesta'`), and use `navigateTo('miesta')` in place of `browser.url('/miesta')` if a full page load drops the app state; the existing specs navigate by link. `GEOCODED_PLACE`, `GEOCODED_COORDS`, `openNewTripRow` and `waitForPlaceRow` are in the file already; change `placeRowSelector` (:121) to take a place ID and build `[data-testid="place-row"][data-place-id="..."]`.

- [ ] **Step 2: Run them and see them fail**

Run the `places.spec.ts` command from Task 6, Step 2.
Expected: FAIL. `/miesta` is a 404 (the SPA fallback shows no list).

- [ ] **Step 3: Write `src/routes/miesta/+page.svelte`**

1. Take the place code that Task 5 removed from Settings (`git show <task-5 commit>~1:src/routes/settings/+page.svelte`, lines :841-940, :1379-1460, :1640-1660 and the place CSS). Bring over the place state and handlers: `places`, `placeFilter`, `visiblePlaces`, `editingPlace`, `loadPlaces`, `openEditPlace`, `closePlaceModal`, `handleSavePlace`. Change `handleSavePlace` to call `setPlacePosition(editingPlace.id, ...)`. Delete `handleClearPlace`.
2. The filter matches `place.name.toLowerCase().includes(needle) || place.normalisedName.includes(needle)`. Keep the comment about the deliberate asymmetry from :857-866.
3. Bring over the markup and the `PlaceModal` block. Key the `#each` and the `{#key}` on `place.id`. On each row, set `data-testid="place-row"` and `data-place-id={place.id}`, and add three controls:
   - Rename: a button `place-rename` that swaps the name for an input `place-rename-input` and a button `place-rename-save`. Save calls `renamePlace`. On an error, show `toast.error(String(error))` (the backend message names the colliding place).
   - Delete: a button `place-delete`, `disabled={place.uses > 0}`, `title={place.uses > 0 ? $LL.places.deleteInUse({ count: place.uses }) : ''}`. Click asks with the existing `ConfirmModal`, then calls `deletePlace` and reloads.
   - The existing edit button `place-edit` opens `PlaceModal` for the position.
4. An add button `place-add` opens a small form: a name input `place-add-name`, then `PlaceModal` for the position. The modal's save calls `createPlace(name, lat, lon, source)`. A place with no position cannot be saved, so the save stays disabled until the modal has a coordinate.
5. Bring over the place CSS rules.

- [ ] **Step 4: Add the nav link and remove the Settings section**

In `+layout.svelte`, between the `/doklady` link and the `/settings` link:

```svelte
<a href="/miesta" class="nav-link" class:active={$page.url.pathname === '/miesta'} data-testid="nav-places">{$LL.app.nav.places()}</a>
```

Task 5 already removed the place section from `settings/+page.svelte`. Check that no anchor link to `#places` is left: `grep -rn "#places" src/`.

i18n keys (Slovak first):

```ts
// sk: app.nav
places: 'Miesta',
// sk: places
add: 'Pridať miesto',
addName: 'Názov miesta',
rename: 'Premenovať',
renameSave: 'Uložiť názov',
delete: 'Zmazať',
deleteInUse: 'Miesto používa {count:number} jázd',
deleteConfirmTitle: 'Zmazať miesto?',
deleteConfirmMessage: 'Miesto "{name:string}" sa zmaže.',
needsPosition: 'Treba doplniť polohu',
// en: same keys
places: 'Places',
add: 'Add place', addName: 'Place name', rename: 'Rename', renameSave: 'Save name',
delete: 'Delete', deleteInUse: 'Used by {count:number} trips',
deleteConfirmTitle: 'Delete place?', deleteConfirmMessage: 'The place "{name:string}" will be deleted.',
needsPosition: 'Needs a position',
```

Check the existing `places.*` keys first and reuse them. Use `needsPosition` as the tooltip of `place-unplaced-icon`.

- [ ] **Step 5: Run the checks and the specs**

```bash
npm run i18n && npm run check && npm run typecheck:tests
npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/places.spec.ts \
  --spec tests/integration/specs/tier2/settings.spec.ts
```

Expected: 0 type errors, and both specs PASS.

- [ ] **Step 6: Commit**

```bash
git add src/routes/miesta/+page.svelte src/routes/+layout.svelte \
  src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts src/lib/i18n/i18n-types.ts \
  tests/integration/specs/tier2/places.spec.ts tests/integration/utils/app.ts
git commit -m "feat(places): Miesta tab with add, rename and delete"
```

---

### Task 8: Feature docs and READMEs

**Files:**
- Modify: all 16 files in [docs/features/](../../docs/features/), [README.md](../../README.md), [README.en.md](../../README.en.md)

- [ ] **Step 1: Find each place reference**

Run: `grep -nE "place|Miesta|origin|destination|normali|save_place|clear_place|display_name|distinct_trip_places|Settings" docs/features/*.md README.md README.en.md`

- [ ] **Step 2: Update the docs the spec names**

| Doc | Change |
|-----|--------|
| [place-book.md](../../docs/features/place-book.md) | Rewrite: a place is an entity (`id`, `name`, `normalised_name`, position). The Miesta tab: add (name and position required), rename (applies to all trips, a key collision is an error), position, delete (only if no trip uses it; orphan routes go with it). The migration rules from Task 2. `find_place` for typed text. Remove the derived-list and ADR-034 display-spelling text, and link the new ADR. |
| [route-maps.md](../../docs/features/route-maps.md) | Endpoints come from `trip.origin_place_id`/`destination_place_id`. Loop mode is ID equality. A legacy place without a position opens the place dialog, which saves with `set_place_position`. |
| [trip-entry-defaults.md](../../docs/features/trip-entry-defaults.md) | The autocomplete offers place names. A save resolves typed text with `find_place` and blocks if no place matches. Distance auto-fill and time inference match on place IDs. |
| [settings-architecture.md](../../docs/features/settings-architecture.md) | Settings has no place section. Link `/miesta`. |
| [backup-system.md](../../docs/features/backup-system.md) | A restore runs the migrations with `kj_normalise` registered, so an old backup becomes the new schema. |
| [read-only-mode.md](../../docs/features/read-only-mode.md) | An image older than this migration opens the migrated DB read-only. |

- [ ] **Step 3: Check the other 10 docs**

For each of `export-system.md`, `home-assistant.md`, `magic-fill.md`, `multi-invoice.md`, `multi-year-state.md`, `paperless-integration.md`, `server-mode.md`, `trip-grid-calculation.md`, `trip-odometer-cascade.md` and `unified-invoice-picker.md`: read each hit from Step 1, and fix any text that says a trip holds a free-text place, names `save_place`/`clear_place`, or puts places in Settings. Write down "updated" or "checked, no change" for each doc. The PR description needs this list (spec, Documentation).

- [ ] **Step 4: Update the READMEs**

In `README.md` (Slovak) and `README.en.md`, add the Miesta tab to the feature list: a place book with add, rename and delete, and a trip that accepts only existing places. Keep both files in step.

- [ ] **Step 5: Commit**

```bash
git add docs/features/*.md README.md README.en.md
git commit -m "docs: places as entities in the feature docs and READMEs"
```

---

### Task 9: Decisions, changelog and the final check

**Files:**
- Modify: [DECISIONS.md](../../DECISIONS.md), [CHANGELOG.md](../../CHANGELOG.md), [_tasks/index.md](../index.md), [01-task.md](./01-task.md) (status)

- [ ] **Step 1: Record the decisions with `/decision`**

Run `/decision` three times:
1. **ADR-055: Places are entities; trips and routes reference them by ID.** Supersedes ADR-033 for places (route counters stay derived) and ADR-034 (the display name is the stored `name`). Check ADR-032 and add a "superseded in part" line if its text names string matching. Reason: one fold, no drift between `normalize_location` and `places::normalise`, and a place can carry more facts (task 89 adds `is_home`).
2. **ADR-056: The places migration uses a SQLite function registered from Rust.** `kj_normalise` on every connection before migrations; foreign keys are ON by default in the bundled SQLite (`SQLITE_DEFAULT_FOREIGN_KEYS=1`), so the migration copies `trip_routes` and `paperless_trip_links` into TEMP tables around `DROP TABLE trips` and never turns the pragma off; the route rules (orphan routes dropped, collapse keeps the latest trip's row, else first by `id`); the `Neznáme miesto` placeholder for a blank endpoint.
3. **BIZ-025: A trip accepts only an existing place.** A rename applies to all trips, including past printed years. A key collision is an error, with no merge. A delete needs zero trip uses, and orphan routes go with the place.

Check the next free numbers first: `grep -n "^### ADR-\|^### BIZ-" DECISIONS.md | head -3`.

- [ ] **Step 2: Update the changelog with `/changelog`**

Under `[Unreleased]`, add the user-visible change (the Miesta tab, the trip-form rule, rename, delete) under `### Pridané` and `### Zmenené`. Replace the `### Pokyny k aktualizácii` block with the five fields of the template ([CHANGELOG.md:17-22](../../CHANGELOG.md), [release-skill](../../.claude/skills/release-skill/SKILL.md) checks them). Write it in Slovak:

```markdown
### Pokyny k aktualizácii
- **Potrebný zásah:** pred aktualizáciou si zálohujte databázu (aplikácia pred migráciou zapíše aj vlastnú zálohu do `<DATA_DIR>/backups/`)
- **Premenné prostredia:** bez zmeny
- **Migrácie databázy:** `2026-10-05-100000_places_as_entities` prestavia tabuľky `places`, `trips` a `routes`. Migrácia je jednosmerná: starší obraz otvorí aktualizovanú databázu len na čítanie.
- **Strata údajov:** iné zápisy toho istého miesta sa stratia, každá jazda ukáže názov miesta (najčastejší zápis) a `down.sql` ich nevie vrátiť; uložené trasy, ktoré nepoužíva žiadna jazda, sa zahodia; prázdny začiatok alebo cieľ jazdy sa zmení na miesto `Neznáme miesto`; miesto jazdy bez súradníc dostane značku "treba doplniť polohu"
- **Obraz, zväzok, port:** bez zmeny
```

Decide **Potrebný zásah** and the major version with the rule in [CHANGELOG.md:8-13](../../CHANGELOG.md): an update that drops data raises the major version. Ask the user before `/release` if the spellings loss counts as "drops data".

- [ ] **Step 3: Run the full verification with `/verify`**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --workspace
npm run check
npm run typecheck:tests
npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
npm run test:integration
```

Expected: all green. If the Docker mode is needed, build the image and run `xvfb-run -a -s "-screen 0 1280x1024x24" npm run test:integration:docker` (see [integration-tests.md](../../.claude/rules/integration-tests.md)).

- [ ] **Step 4: Update the task status**

Set `**Status:** Complete` in [01-task.md](./01-task.md) and in this file. Set the row in [_tasks/index.md](../index.md) to ✅.

- [ ] **Step 5: Commit**

```bash
git add DECISIONS.md CHANGELOG.md _tasks/index.md _tasks/88-places-as-entities/
git commit -m "docs: decisions and changelog for places as entities"
```

---

## Spec Coverage

| Spec section | Task |
|--------------|------|
| 1. Schema (places, trips, routes, foreign keys on by default) | 2, 3 |
| 2. Migration (function, one helper, names, coordinates, legacy rows, blank endpoint, orphan and collapsed routes, child rows, LEFT JOIN abort, down.sql, backup) | 1, 2 |
| 3. Backend API (Trip read, trip writes by ID, routes, inference, route maps, place commands incl. `find_place`, read-only) | 3, 4 |
| 4. UI (Miesta tab, add, rename, delete, needs-position marker, trip form, auto-fill and inference by ID, i18n) | 5, 6, 7 |
| 5. Behaviour rules (rename everywhere, collision error, delete only if no trip uses it) | 4, 7 |
| Tests (migration, restore, unknown ID, rename, delete, find, unplaced place, routes and inference by ID, helpers, call sites) | 1, 2, 3, 4, 5 |
| Integration flows (add then pick, unknown text blocks, rename shows in grid, DB reset deletes places) | 5, 6, 7 |
| Documentation (16 docs, READMEs) | 8 |
| Decisions, upgrade notes (five-field template) | 9 |
