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

   # Add (no r2d2 - connection pooling is YAGNI for single-user desktop app)
   diesel = { version = "2.2", features = ["sqlite"] }
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
- Create: `src-tauri/migrations/00000000000000_diesel_initial_setup/` (Diesel setup)
- Create: `src-tauri/migrations/00000000000001_baseline/` (current schema)

**Steps:**
1. Copy production DB to dev location for schema generation:
   ```powershell
   # Windows - copy from AppData
   copy "$env:APPDATA\com.notavailable.kniha-jazd\kniha-jazd.db" src-tauri\kniha-jazd.db
   ```
2. Set DATABASE_URL:
   ```powershell
   # Windows
   $env:DATABASE_URL = "sqlite:./kniha-jazd.db"
   ```
   ```bash
   # Unix
   export DATABASE_URL=sqlite:./kniha-jazd.db
   ```
3. Run: `diesel print-schema > src/schema.rs`
4. Run: `diesel setup` to create migrations directory structure
5. Create baseline migration with **actual schema** (not empty):
   ```bash
   diesel migration generate baseline
   ```
   In `up.sql`, copy the CREATE TABLE statements from existing `001_initial.sql`:
   - All 5 tables: vehicles, trips, routes, settings, receipts
   - This allows `diesel migration run` on fresh DB to create full schema
6. In `down.sql`, add DROP TABLE statements (reverse order)
7. Verify generated schema matches all 5 tables:
   - vehicles, trips, routes, settings, receipts

**Note:** Existing production DBs won't run migrations (already have tables).
Fresh DBs (tests, new installs) will run migrations to create schema.

**Verification:**
- `schema.rs` contains all tables with correct column types
- `diesel migration run` on empty DB creates all tables

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
4. Handle type mappings with explicit implementations:

   **VehicleType enum** - use `diesel_derive_enum` or manual conversion:
   ```rust
   // Option A: Keep as String in struct, convert in db layer
   #[derive(Queryable)]
   pub struct Vehicle {
       pub vehicle_type: String,  // Convert to/from VehicleType in methods
   }

   // Option B: Implement FromSql/ToSql (more type-safe)
   use diesel::deserialize::{self, FromSql};
   use diesel::serialize::{self, ToSql, Output};
   use diesel::sqlite::Sqlite;

   impl ToSql<diesel::sql_types::Text, Sqlite> for VehicleType {
       fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
           let s = match self {
               VehicleType::Passenger => "passenger",
               // ... other variants
           };
           <str as ToSql<diesel::sql_types::Text, Sqlite>>::to_sql(s, out)
       }
   }
   ```

   **ReceiptStatus enum** - same pattern as VehicleType

   **FieldConfidence (JSON)** - store as TEXT, use serde:
   ```rust
   #[derive(Queryable)]
   pub struct Receipt {
       pub confidence: String,  // JSON string, parse with serde_json
   }

   impl Receipt {
       pub fn get_confidence(&self) -> FieldConfidence {
           serde_json::from_str(&self.confidence).unwrap_or_default()
       }
   }
   ```

   **UUID** - store as TEXT (already works this way)

   **DateTime<Utc>** - store as RFC3339 TEXT string (already works this way)

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
2. Update constructor with **Diesel 2.x API**:
   ```rust
   use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

   // Embed migrations at compile time
   pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

   impl Database {
       pub fn new(path: PathBuf) -> Result<Self, diesel::ConnectionError> {
           let mut conn = SqliteConnection::establish(path.to_str().unwrap())?;
           // Run any pending migrations on startup
           conn.run_pending_migrations(MIGRATIONS)
               .expect("Failed to run migrations");
           Ok(Self { conn: Mutex::new(conn) })
       }

       pub fn in_memory() -> Result<Self, diesel::ConnectionError> {
           let mut conn = SqliteConnection::establish(":memory:")?;
           // Run embedded migrations for tests
           conn.run_pending_migrations(MIGRATIONS)
               .expect("Failed to run migrations");
           Ok(Self { conn: Mutex::new(conn) })
       }
   }
   ```

   **Note:** Diesel 2.x uses `embed_migrations!` macro and `MigrationHarness` trait.
   The old `diesel_migrations::run_pending_migrations(&mut conn)` is Diesel 1.x API.

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

**Verification:** All db.rs vehicle tests pass: `cargo test --lib`

---

## Task 6: Migrate Trip CRUD Operations

**Files:**
- Modify: `src-tauri/src/db.rs`

**Steps:**
1. Rewrite all trip methods:
   - `create_trip`
   - `get_trip`
   - `get_trips_for_vehicle`
   - `get_trips_for_vehicle_in_year` - **uses raw SQL** (see below)
   - `get_years_with_trips` - **uses raw SQL** (see below)
   - `update_trip`
   - `delete_trip`
   - `reorder_trip` - **wrap in transaction** (see below)
   - `shift_trips_from_position`

2. **Year filtering requires raw SQL** (Diesel DSL doesn't support `strftime`):
   ```rust
   use diesel::sql_query;
   use diesel::sql_types::Text;

   pub fn get_trips_for_vehicle_in_year(&self, vehicle_id: &str, year: i32)
       -> QueryResult<Vec<Trip>>
   {
       let conn = &mut *self.conn.lock().unwrap();
       sql_query("SELECT * FROM trips WHERE vehicle_id = ? AND strftime('%Y', date) = ?")
           .bind::<Text, _>(vehicle_id)
           .bind::<Text, _>(year.to_string())
           .load::<Trip>(conn)
   }
   ```

3. **reorder_trip must use transaction** for atomicity:
   ```rust
   pub fn reorder_trip(&self, trip_id: &str, new_position: i32) -> QueryResult<()> {
       let conn = &mut *self.conn.lock().unwrap();
       conn.transaction(|conn| {
           // Get current trip info
           // Update other trips' sort_order
           // Update the moved trip
           Ok(())
       })
   }
   ```

**Verification:** All db.rs trip tests pass: `cargo test --lib`

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
   - `get_purposes_for_vehicle` - **uses raw SQL** (see below)

2. **get_purposes_for_vehicle requires raw SQL** (uses `DISTINCT TRIM()`):
   ```rust
   pub fn get_purposes_for_vehicle(&self, vehicle_id: &str) -> QueryResult<Vec<String>> {
       let conn = &mut *self.conn.lock().unwrap();
       // Raw SQL needed for DISTINCT TRIM()
       sql_query("SELECT DISTINCT TRIM(purpose) as purpose FROM routes WHERE vehicle_id = ?")
           .bind::<Text, _>(vehicle_id)
           .load::<PurposeRow>(conn)
           .map(|rows| rows.into_iter().map(|r| r.purpose).collect())
   }

   // Helper struct for raw query result
   #[derive(QueryableByName)]
   struct PurposeRow {
       #[diesel(sql_type = Text)]
       purpose: String,
   }
   ```

**Verification:** All db.rs route tests pass: `cargo test --lib`

---

## Task 8: Migrate Settings CRUD Operations

**Files:**
- Modify: `src-tauri/src/db.rs`

**Steps:**
1. Rewrite settings methods:
   - `get_settings`
   - `save_settings` (upsert logic)

**Verification:** All db.rs settings tests pass: `cargo test --lib`

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

**Verification:** All db.rs receipt tests pass: `cargo test --lib`

---

## Task 10: Migrate Helper Methods

**Files:**
- Modify: `src-tauri/src/db.rs`

**Steps:**
1. Rewrite `populate_routes_from_trips` - **uses raw SQL** (complex INSERT...SELECT):
   ```rust
   pub fn populate_routes_from_trips(&self, vehicle_id: &str) -> QueryResult<usize> {
       let conn = &mut *self.conn.lock().unwrap();
       // Complex aggregation with INSERT...SELECT, AVG, COUNT, MAX, randomblob()
       // Must use raw SQL - Diesel DSL doesn't support this pattern
       diesel::sql_query(r#"
           INSERT INTO routes (id, vehicle_id, start_location, end_location, distance_km, purpose, usage_count, last_used)
           SELECT
               lower(hex(randomblob(16))),
               vehicle_id,
               start_location,
               end_location,
               ROUND(AVG(distance_km), 1),
               purpose,
               COUNT(*),
               MAX(date)
           FROM trips
           WHERE vehicle_id = ?
           GROUP BY vehicle_id, start_location, end_location, purpose
           ON CONFLICT(vehicle_id, start_location, end_location, purpose) DO UPDATE SET
               distance_km = excluded.distance_km,
               usage_count = excluded.usage_count,
               last_used = excluded.last_used
       "#)
       .bind::<Text, _>(vehicle_id)
       .execute(conn)
   }
   ```

2. Remove `connection()` method if no longer needed
3. Clean up any remaining rusqlite imports

**Verification:** All db.rs tests pass: `cargo test --lib`

---

## Task 11: Update Error Types and Backup Inspection

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/error.rs`

**Steps:**
1. Update `error.rs` - change rusqlite to Diesel error type:
   ```rust
   // Before
   #[error("Database error: {0}")]
   Database(#[from] rusqlite::Error),

   // After
   #[error("Database error: {0}")]
   Database(#[from] diesel::result::Error),
   ```

2. Update `commands.rs` error handling throughout

3. **Migrate backup inspection to Diesel** (currently uses direct rusqlite at line ~1339):
   ```rust
   // Before: rusqlite::Connection::open(&backup_path)
   // After: Create Database::from_path() method that opens arbitrary DB files

   impl Database {
       /// Open an arbitrary database file (for backup inspection)
       pub fn from_path(path: &Path) -> Result<Self, diesel::ConnectionError> {
           let conn = SqliteConnection::establish(path.to_str().unwrap())?;
           // Don't run migrations - this is for reading existing backups
           Ok(Self { conn: Mutex::new(conn) })
       }
   }
   ```

   Update `get_backup_info` to use this method instead of rusqlite.

4. Remove all rusqlite imports from codebase

**Verification:** `cargo build` succeeds with no rusqlite references, app launches, backup info works

---

## Task 12: Run Full Test Suite

**Files:**
- None (verification only)

**Steps:**
1. Run all backend tests: `cargo test`
2. Run integration tests: `npm run test:integration:build`
3. Manual smoke test: Launch app, CRUD a vehicle, trip, receipt

**Verification:** All tests pass (108 backend + 61 integration), app functions correctly

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

**Pre-migration safeguard:**
```bash
# Save current Cargo.lock before starting (Diesel adds many transitive deps)
cp src-tauri/Cargo.lock src-tauri/Cargo.lock.backup
```

If migration fails partway:
1. `git stash` or `git checkout -- .` to revert changes
2. Restore Cargo.lock: `cp src-tauri/Cargo.lock.backup src-tauri/Cargo.lock`
3. rusqlite is still in Cargo.toml until final commit
4. Existing `.db` files are never modified

## Estimated Scope

- **Lines changed:** ~1500 (mostly db.rs rewrite)
- **New files:** 3 (schema.rs, diesel.toml, migrations/)
- **Risk:** Medium - well-tested DB layer, but significant rewrite
