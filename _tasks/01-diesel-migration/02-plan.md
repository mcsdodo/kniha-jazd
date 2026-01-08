# Diesel Migration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace rusqlite with Diesel ORM for compile-time SQL safety

**Architecture:** Sync Diesel with SQLite, schema generated from existing DB, derives enforce CRUD completeness

---

## Task 1: Setup Diesel Dependencies and Configuration

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/diesel.toml`

**Steps:**
1. Replace rusqlite dependency with diesel:
   ```toml
   # Remove
   rusqlite = { version = "0.33", features = ["bundled"] }

   # Add
   diesel = { version = "2.2", features = ["sqlite", "r2d2"] }
   diesel_migrations = "2.2"
   ```
2. Create `diesel.toml`:
   ```toml
   [print_schema]
   file = "src/schema.rs"

   [migrations_directory]
   dir = "migrations"
   ```
3. Install Diesel CLI locally: `cargo install diesel_cli --no-default-features --features sqlite`

**Verification:** `cargo check` passes (will have errors until schema.rs exists)

---

## Task 2: Generate Schema from Existing Database

**Files:**
- Create: `src-tauri/src/schema.rs`
- Create: `src-tauri/migrations/` (baseline)

**Steps:**
1. Copy production DB to dev location for schema generation
2. Set DATABASE_URL: `export DATABASE_URL=sqlite:./kniha-jazd.db`
3. Run: `diesel print-schema > src/schema.rs`
4. Create baseline migration (empty, marks current state):
   ```bash
   diesel migration generate baseline
   # Leave up.sql and down.sql empty - existing DB is baseline
   ```
5. Verify generated schema matches all 5 tables:
   - vehicles, trips, routes, settings, receipts

**Verification:** `schema.rs` contains all tables with correct column types

---

## Task 3: Update Models with Diesel Derives

**Files:**
- Modify: `src-tauri/src/models.rs`

**Steps:**
1. Add diesel prelude import:
   ```rust
   use diesel::prelude::*;
   use crate::schema::*;
   ```
2. Add derives to Vehicle:
   ```rust
   #[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
   #[diesel(table_name = vehicles)]
   pub struct Vehicle { ... }

   #[derive(Debug, Insertable)]
   #[diesel(table_name = vehicles)]
   pub struct NewVehicle<'a> { ... }
   ```
3. Repeat for: Trip/NewTrip, Route/NewRoute, Settings/NewSettings, Receipt/NewReceipt
4. Handle type mappings:
   - UUID → String (store as TEXT)
   - DateTime<Utc> → String (store as RFC3339 TEXT)
   - VehicleType enum → String (implement ToSql/FromSql or use String)
   - ReceiptStatus enum → String

**Verification:** `cargo check` passes with new derives

---

## Task 4: Rewrite Database Struct and Connection

**Files:**
- Modify: `src-tauri/src/db.rs`

**Steps:**
1. Replace connection type:
   ```rust
   use diesel::prelude::*;
   use diesel::sqlite::SqliteConnection;
   use std::sync::Mutex;

   pub struct Database {
       conn: Mutex<SqliteConnection>,
   }
   ```
2. Update constructor:
   ```rust
   impl Database {
       pub fn new(path: PathBuf) -> Result<Self, diesel::ConnectionError> {
           let conn = SqliteConnection::establish(path.to_str().unwrap())?;
           Ok(Self { conn: Mutex::new(conn) })
       }

       pub fn in_memory() -> Result<Self, diesel::ConnectionError> {
           let mut conn = SqliteConnection::establish(":memory:")?;
           // Run embedded migrations for tests
           diesel_migrations::run_pending_migrations(&mut conn)
               .expect("Failed to run migrations");
           Ok(Self { conn: Mutex::new(conn) })
       }
   }
   ```
3. Remove old `run_migrations()` method (Diesel handles this)

**Verification:** Database struct compiles, connection established

---

## Task 5: Migrate Vehicle CRUD Operations

**Files:**
- Modify: `src-tauri/src/db.rs`

**Steps:**
1. Rewrite `create_vehicle`:
   ```rust
   pub fn create_vehicle(&self, vehicle: &NewVehicle) -> QueryResult<()> {
       use crate::schema::vehicles;
       let conn = &mut *self.conn.lock().unwrap();
       diesel::insert_into(vehicles::table)
           .values(vehicle)
           .execute(conn)?;
       Ok(())
   }
   ```
2. Rewrite `get_vehicle`, `get_all_vehicles`, `get_active_vehicle`
3. Rewrite `update_vehicle` using AsChangeset
4. Rewrite `delete_vehicle`

**Verification:** Vehicle-related tests pass: `cargo test vehicle`

---

## Task 6: Migrate Trip CRUD Operations

**Files:**
- Modify: `src-tauri/src/db.rs`

**Steps:**
1. Rewrite all trip methods:
   - `create_trip`
   - `get_trip`
   - `get_trips_for_vehicle`
   - `get_trips_for_vehicle_in_year`
   - `get_years_with_trips`
   - `update_trip`
   - `delete_trip`
   - `reorder_trip`
   - `shift_trips_from_position`

**Verification:** Trip-related tests pass: `cargo test trip`

---

## Task 7: Migrate Route CRUD Operations

**Files:**
- Modify: `src-tauri/src/db.rs`

**Steps:**
1. Rewrite all route methods:
   - `create_route`
   - `get_route`
   - `get_routes_for_vehicle`
   - `update_route`
   - `delete_route`
   - `find_or_create_route`
   - `get_purposes_for_vehicle`

**Verification:** Route-related tests pass: `cargo test route`

---

## Task 8: Migrate Settings CRUD Operations

**Files:**
- Modify: `src-tauri/src/db.rs`

**Steps:**
1. Rewrite settings methods:
   - `get_settings`
   - `save_settings` (upsert logic)

**Verification:** Settings-related tests pass: `cargo test settings`

---

## Task 9: Migrate Receipt CRUD Operations

**Files:**
- Modify: `src-tauri/src/db.rs`

**Steps:**
1. Rewrite all receipt methods:
   - `create_receipt`
   - `get_all_receipts`
   - `get_unassigned_receipts`
   - `get_pending_receipts`
   - `update_receipt`
   - `delete_receipt`
   - `get_receipt_by_file_path`
   - `get_receipt_by_id`
   - `get_receipts_for_year`
   - `get_receipts_for_vehicle`
2. Remove `row_to_receipt` helper (Queryable derive handles this)

**Verification:** Receipt-related tests pass: `cargo test receipt`

---

## Task 10: Migrate Helper Methods

**Files:**
- Modify: `src-tauri/src/db.rs`

**Steps:**
1. Rewrite `populate_routes_from_trips` (complex INSERT...SELECT)
2. Remove `connection()` method if no longer needed
3. Clean up any remaining rusqlite imports

**Verification:** All db.rs tests pass: `cargo test --lib`

---

## Task 11: Update Commands.rs Error Handling

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Steps:**
1. Update error types from `rusqlite::Error` to `diesel::result::Error`
2. Verify all Tauri commands still work
3. Update any direct connection access

**Verification:** `cargo build` succeeds, app launches

---

## Task 12: Run Full Test Suite

**Files:**
- None (verification only)

**Steps:**
1. Run all backend tests: `cargo test`
2. Run integration tests: `npm run test:integration:build`
3. Manual smoke test: Launch app, CRUD a vehicle, trip, receipt

**Verification:** All 108+ tests pass, app functions correctly

---

## Task 13: Update CI/CD Configuration

**Files:**
- Modify: `.github/workflows/test.yml` (if needed)

**Steps:**
1. Ensure CI can run Diesel tests (in-memory DB, no external deps)
2. Remove any rusqlite-specific configuration

**Verification:** CI pipeline passes

---

## Task 14: Cleanup and Documentation

**Files:**
- Modify: `CLAUDE.md`
- Modify: `CHANGELOG.md`

**Steps:**
1. Update CLAUDE.md with new Diesel patterns:
   - How to add a new column (migration workflow)
   - Schema regeneration command
2. Run `/changelog` to document the migration
3. Commit all changes

**Verification:** Documentation reflects new workflow

---

## Rollback Plan

If migration fails partway:
1. `git stash` or `git checkout -- .` to revert changes
2. rusqlite is still in Cargo.toml until final commit
3. Existing `.db` files are never modified

## Estimated Scope

- **Lines changed:** ~1500 (mostly db.rs rewrite)
- **New files:** 3 (schema.rs, diesel.toml, migrations/)
- **Risk:** Medium - well-tested DB layer, but significant rewrite
