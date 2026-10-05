//! Database layer using Diesel ORM for compile-time SQL safety.
//!
//! All CRUD operations use Diesel's type-safe query builder.
//! Row structs (VehicleRow, etc.) map directly to DB; conversions
//! to domain models (Vehicle, etc.) happen via From implementations.

use crate::models::{
    AssignmentType, NewPlaceRow, NewRouteMapRow, NewRouteRow, NewSettingsRow, NewTripRow,
    NewVehicleRow, PaperlessLink, PlaceRow, Route, RouteMap, RouteMapRow, RouteRow, Settings,
    SettingsRow, Trip, TripInvoiceCoverage, TripRow, Vehicle, VehicleRow,
};
use crate::schema::{places, routes, settings, trip_routes, trips, vehicles};
use chrono::{NaiveDate, NaiveDateTime, Utc};
use diesel::migration::MigrationSource;
use diesel::prelude::*;
use diesel::result::QueryResult;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

// Embed migrations at compile time
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

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

/// Place id -> display name. Loaded once per read: tens of rows, and a trip
/// list always needs most of them.
fn place_names(conn: &mut SqliteConnection) -> QueryResult<HashMap<String, String>> {
    Ok(places::table
        .select((places::id, places::name))
        .load::<(String, String)>(conn)?
        .into_iter()
        .collect())
}

pub struct Database {
    conn: Mutex<SqliteConnection>,
}

/// Result of `Database::delete_place_if_unused`.
#[derive(Debug, PartialEq)]
pub enum DeletePlaceOutcome {
    Deleted,
    InUse(i64),
    NotFound,
}

impl Database {
    pub fn new(path: PathBuf) -> Result<Self, diesel::ConnectionError> {
        // Detect a pre-existing database file BEFORE establishing the
        // connection — SQLite creates an empty file on open, so checking
        // afterwards would also match brand-new databases.
        let file_pre_exists = path.metadata().map(|m| m.len() > 0).unwrap_or(false);

        let path_str = path.to_str().unwrap_or("");
        let mut conn = SqliteConnection::establish(path_str)?;
        prepare_connection(&mut conn)
            .map_err(|e| diesel::ConnectionError::BadConnection(e.to_string()))?;

        // Safety net: snapshot the existing DB file before applying pending
        // migrations, so a failed or buggy migration can be recovered from.
        // A failed backup must NOT block startup — it's a net, not a gate.
        if file_pre_exists {
            match conn.has_pending_migration(MIGRATIONS) {
                Ok(true) => {
                    if let Err(e) = Self::create_pre_migration_backup(&path) {
                        log::warn!(
                            "Pre-migration backup failed, continuing with migrations: {}",
                            e
                        );
                    }
                }
                Ok(false) => {}
                Err(e) => {
                    log::warn!(
                        "Could not check pending migrations for pre-migration backup: {}",
                        e
                    );
                }
            }
        }

        // Run any pending migrations on startup
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Copy the database file into the sibling `backups/` directory before
    /// migrations run. A plain `fs::copy` is safe here: this is the first
    /// connection to the file during startup and no writes have happened yet.
    /// Uses the same naming convention as user-facing backups
    /// (`kniha-jazd-backup-{timestamp}-pre-migration-v{app_version}.db`).
    fn create_pre_migration_backup(db_file: &std::path::Path) -> Result<(), String> {
        let parent = db_file
            .parent()
            .ok_or_else(|| "Database file has no parent directory".to_string())?;
        let backups_dir = parent.join(crate::constants::paths::BACKUPS_DIR);
        std::fs::create_dir_all(&backups_dir).map_err(|e| e.to_string())?;

        let filename = crate::commands_internal::backup::generate_backup_filename(
            crate::models::BackupType::PreMigration.as_str(),
            Some(env!("CARGO_PKG_VERSION")),
        );
        let backup_path = backups_dir.join(&filename);
        std::fs::copy(db_file, &backup_path).map_err(|e| e.to_string())?;

        log::info!("Pre-migration backup created: {}", backup_path.display());
        Ok(())
    }

    // Test helper for in-memory databases
    #[allow(dead_code)]
    pub fn in_memory() -> Result<Self, diesel::ConnectionError> {
        let mut conn = SqliteConnection::establish(":memory:")?;
        prepare_connection(&mut conn)
            .map_err(|e| diesel::ConnectionError::BadConnection(e.to_string()))?;

        // Run embedded migrations for tests
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Open an arbitrary database file (for backup inspection)
    /// Does NOT run migrations - for reading existing backups only.
    pub fn from_path(path: &std::path::Path) -> Result<Self, diesel::ConnectionError> {
        let path_str = path.to_str().unwrap_or("");
        let conn = SqliteConnection::establish(path_str)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Replace the live database file with `source`, then reopen and migrate it.
    ///
    /// The copy, the reopen and the migrations run in one critical section, so
    /// no request writes to the file in between. The old connection is dropped:
    /// its schema cache describes the file as it was before the copy. Migrations
    /// run here because an older backup would otherwise stay on its old schema
    /// until the next process start. No pre-migration snapshot is taken - the
    /// source backup file is still on disk.
    ///
    /// The caller must reject a source with unknown migrations first
    /// (`check_backup_restorable`). That keeps the startup read-only flag
    /// correct: the migrated file only has migrations this build knows.
    pub fn restore_from_file(
        &self,
        source: &std::path::Path,
        db_file: &std::path::Path,
    ) -> Result<(), String> {
        let mut conn = self.conn.lock().unwrap();
        std::fs::copy(source, db_file).map_err(|e| e.to_string())?;
        let path_str = db_file
            .to_str()
            .ok_or_else(|| "Invalid database path encoding".to_string())?;
        *conn = SqliteConnection::establish(path_str).map_err(|e| e.to_string())?;
        prepare_connection(&mut conn).map_err(|e| e.to_string())?;
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|e| format!("Migrácia obnovenej zálohy zlyhala: {}", e))?;
        Ok(())
    }

    /// Versions recorded in `__diesel_schema_migrations`.
    ///
    /// Unlike [`Self::check_migration_compatibility`], an unreadable table is an
    /// error, not "no migrations": a file without migration history is not a
    /// kniha-jazd database. Startup keeps the lenient check on purpose.
    pub(crate) fn applied_migration_versions(&self) -> QueryResult<Vec<String>> {
        #[derive(diesel::QueryableByName)]
        struct MigrationRow {
            #[diesel(sql_type = diesel::sql_types::Text)]
            version: String,
        }

        let conn = &mut *self.conn.lock().unwrap();
        let rows: Vec<MigrationRow> =
            diesel::sql_query("SELECT version FROM __diesel_schema_migrations ORDER BY version")
                .load(conn)?;
        Ok(rows.into_iter().map(|r| r.version).collect())
    }

    /// Get a raw connection for direct SQL (backup inspection, etc.)
    pub fn connection(&self) -> std::sync::MutexGuard<'_, SqliteConnection> {
        self.conn.lock().unwrap()
    }

    // ========================================================================
    // Migration Compatibility
    // ========================================================================

    /// Get the list of embedded migration versions (known to this app version).
    /// Returns a set of migration version strings.
    ///
    /// Note: This uses MigrationSource trait to get embedded migrations.
    pub fn get_embedded_migration_versions() -> std::collections::HashSet<String> {
        let mut versions = std::collections::HashSet::new();
        // Get migrations from the embedded source using the MigrationSource trait
        if let Ok(migrations) =
            <EmbeddedMigrations as MigrationSource<diesel::sqlite::Sqlite>>::migrations(&MIGRATIONS)
        {
            for migration in migrations {
                versions.insert(migration.name().version().to_string());
            }
        }
        versions
    }

    /// Check if database has unknown migrations (from newer app version).
    /// Returns Ok(()) if compatible, Err with list of unknown migration names.
    pub fn check_migration_compatibility(&self) -> Result<(), Vec<String>> {
        let conn = &mut *self.conn.lock().unwrap();
        let embedded = Self::get_embedded_migration_versions();

        // Query applied migrations from __diesel_schema_migrations table
        #[derive(diesel::QueryableByName)]
        struct MigrationRow {
            #[diesel(sql_type = diesel::sql_types::Text)]
            version: String,
        }

        let applied: Vec<MigrationRow> =
            diesel::sql_query("SELECT version FROM __diesel_schema_migrations ORDER BY version")
                .load(conn)
                .unwrap_or_default();

        let unknown: Vec<String> = applied
            .into_iter()
            .filter(|m| !embedded.contains(&m.version))
            .map(|m| m.version)
            .collect();

        if unknown.is_empty() {
            Ok(())
        } else {
            Err(unknown)
        }
    }

    // ========================================================================
    // Vehicle CRUD Operations
    // ========================================================================

    pub fn create_vehicle(&self, vehicle: &Vehicle) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        let id_str = vehicle.id.to_string();
        let created_at_str = vehicle.created_at.to_rfc3339();
        let updated_at_str = vehicle.updated_at.to_rfc3339();

        let new_vehicle = NewVehicleRow {
            id: &id_str,
            name: &vehicle.name,
            license_plate: &vehicle.license_plate,
            vehicle_type: vehicle.to_vehicle_type_str(),
            tank_size_liters: vehicle.tank_size_liters,
            tp_consumption: vehicle.tp_consumption,
            battery_capacity_kwh: vehicle.battery_capacity_kwh,
            baseline_consumption_kwh: vehicle.baseline_consumption_kwh,
            initial_battery_percent: vehicle.initial_battery_percent,
            initial_odometer: vehicle.initial_odometer,
            is_active: if vehicle.is_active { 1 } else { 0 },
            created_at: &created_at_str,
            updated_at: &updated_at_str,
            vin: vehicle.vin.as_deref(),
            driver_name: vehicle.driver_name.as_deref(),
            ha_odo_sensor: vehicle.ha_odo_sensor.as_deref(),
            ha_fillup_sensor: vehicle.ha_fillup_sensor.as_deref(),
            ha_fuel_level_sensor: vehicle.ha_fuel_level_sensor.as_deref(),
        };

        diesel::insert_into(vehicles::table)
            .values(&new_vehicle)
            .execute(conn)?;

        Ok(())
    }

    pub fn get_vehicle(&self, id: &str) -> QueryResult<Option<Vehicle>> {
        let conn = &mut *self.conn.lock().unwrap();

        let row = vehicles::table
            .filter(vehicles::id.eq(id))
            .first::<VehicleRow>(conn)
            .optional()?;

        Ok(row.map(Vehicle::from))
    }

    pub fn get_all_vehicles(&self) -> QueryResult<Vec<Vehicle>> {
        let conn = &mut *self.conn.lock().unwrap();

        let rows = vehicles::table
            .order(vehicles::created_at.desc())
            .load::<VehicleRow>(conn)?;

        Ok(rows.into_iter().map(Vehicle::from).collect())
    }

    pub fn get_active_vehicle(&self) -> QueryResult<Option<Vehicle>> {
        let conn = &mut *self.conn.lock().unwrap();

        let row = vehicles::table
            .filter(vehicles::is_active.eq(1))
            .first::<VehicleRow>(conn)
            .optional()?;

        Ok(row.map(Vehicle::from))
    }

    pub fn update_vehicle(&self, vehicle: &Vehicle) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        let id_str = vehicle.id.to_string();
        let updated_at_str = vehicle.updated_at.to_rfc3339();

        diesel::update(vehicles::table.filter(vehicles::id.eq(&id_str)))
            .set((
                vehicles::name.eq(&vehicle.name),
                vehicles::license_plate.eq(&vehicle.license_plate),
                vehicles::vehicle_type.eq(vehicle.to_vehicle_type_str()),
                vehicles::tank_size_liters.eq(vehicle.tank_size_liters),
                vehicles::tp_consumption.eq(vehicle.tp_consumption),
                vehicles::battery_capacity_kwh.eq(vehicle.battery_capacity_kwh),
                vehicles::baseline_consumption_kwh.eq(vehicle.baseline_consumption_kwh),
                vehicles::initial_battery_percent.eq(vehicle.initial_battery_percent),
                vehicles::initial_odometer.eq(vehicle.initial_odometer),
                vehicles::is_active.eq(if vehicle.is_active { 1 } else { 0 }),
                vehicles::vin.eq(&vehicle.vin),
                vehicles::driver_name.eq(&vehicle.driver_name),
                vehicles::ha_odo_sensor.eq(&vehicle.ha_odo_sensor),
                vehicles::ha_fillup_sensor.eq(&vehicle.ha_fillup_sensor),
                vehicles::ha_fuel_level_sensor.eq(&vehicle.ha_fuel_level_sensor),
                vehicles::updated_at.eq(&updated_at_str),
            ))
            .execute(conn)?;

        Ok(())
    }

    pub fn delete_vehicle(&self, id: &str) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();

        // Cascade-delete autocomplete routes for this vehicle. Routes are
        // internal helper data created when trips are saved with new origin/
        // destination pairs; they have no value once the vehicle is gone.
        diesel::delete(routes::table.filter(routes::vehicle_id.eq(id))).execute(conn)?;

        diesel::delete(vehicles::table.filter(vehicles::id.eq(id))).execute(conn)?;

        Ok(())
    }

    // ========================================================================
    // Trip CRUD Operations
    // ========================================================================

    pub fn create_trip(&self, trip: &Trip) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        let id_str = trip.id.to_string();
        let vehicle_id_str = trip.vehicle_id.to_string();
        let start_datetime_str = trip.start_datetime.format("%Y-%m-%dT%H:%M:%S").to_string();
        let end_datetime_str = trip
            .end_datetime
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string());
        let created_at_str = trip.created_at.to_rfc3339();
        let updated_at_str = trip.updated_at.to_rfc3339();
        let other_costs_note_ref = trip.other_costs_note.as_deref();
        let origin_place_id_str = trip.origin_place_id.to_string();
        let destination_place_id_str = trip.destination_place_id.to_string();

        let new_trip = NewTripRow {
            id: &id_str,
            vehicle_id: &vehicle_id_str,
            origin_place_id: &origin_place_id_str,
            destination_place_id: &destination_place_id_str,
            distance_km: trip.distance_km,
            odometer: trip.odometer,
            purpose: &trip.purpose,
            fuel_liters: trip.fuel_liters,
            fuel_cost_eur: trip.fuel_cost_eur,
            other_costs_eur: trip.other_costs_eur,
            other_costs_note: other_costs_note_ref,
            full_tank: if trip.full_tank { 1 } else { 0 },
            energy_kwh: trip.energy_kwh,
            energy_cost_eur: trip.energy_cost_eur,
            full_charge: Some(if trip.full_charge { 1 } else { 0 }),
            soc_override_percent: trip.soc_override_percent,
            created_at: &created_at_str,
            updated_at: &updated_at_str,
            start_datetime: &start_datetime_str,
            end_datetime: end_datetime_str.as_deref(),
        };

        diesel::insert_into(trips::table)
            .values(&new_trip)
            .execute(conn)?;

        Ok(())
    }

    pub fn get_trip(&self, id: &str) -> QueryResult<Option<Trip>> {
        let conn = &mut *self.conn.lock().unwrap();

        let row = trips::table
            .filter(trips::id.eq(id))
            .first::<TripRow>(conn)
            .optional()?;
        let names = place_names(conn)?;

        Ok(row.map(|r| Trip::from_row(r, &names)))
    }

    pub fn get_trips_for_vehicle(&self, vehicle_id: &str) -> QueryResult<Vec<Trip>> {
        let conn = &mut *self.conn.lock().unwrap();

        let rows = trips::table
            .filter(trips::vehicle_id.eq(vehicle_id))
            .order((trips::start_datetime.desc(), trips::created_at.asc()))
            .load::<TripRow>(conn)?;
        let names = place_names(conn)?;

        Ok(rows.into_iter().map(|r| Trip::from_row(r, &names)).collect())
    }

    /// Get trips for a vehicle in a specific year
    pub fn get_trips_for_vehicle_in_year(
        &self,
        vehicle_id: &str,
        year: i32,
    ) -> QueryResult<Vec<Trip>> {
        use crate::schema::trips::dsl;
        let conn = &mut *self.conn.lock().unwrap();

        // Use start_datetime range - format is "YYYY-MM-DDTHH:MM:SS"
        let start_date = format!("{}-01-01T00:00:00", year);
        let end_date = format!("{}-12-31T23:59:59", year);

        let rows = dsl::trips
            .filter(dsl::vehicle_id.eq(vehicle_id))
            .filter(dsl::start_datetime.ge(&start_date))
            .filter(dsl::start_datetime.le(&end_date))
            .order((dsl::start_datetime.desc(), dsl::created_at.asc()))
            .load::<TripRow>(conn)?;
        let names = place_names(conn)?;

        Ok(rows.into_iter().map(|r| Trip::from_row(r, &names)).collect())
    }

    /// Trips of a vehicle whose start is in `from..=to` (both days inclusive),
    /// oldest first. Same "YYYY-MM-DDTHH:MM:SS" string range as the year query.
    pub fn get_trips_for_vehicle_in_range(
        &self,
        vehicle_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> QueryResult<Vec<Trip>> {
        use crate::schema::trips::dsl;
        let conn = &mut *self.conn.lock().unwrap();

        let start = format!("{}T00:00:00", from.format("%Y-%m-%d"));
        let end = format!("{}T23:59:59", to.format("%Y-%m-%d"));

        let rows = dsl::trips
            .filter(dsl::vehicle_id.eq(vehicle_id))
            .filter(dsl::start_datetime.ge(&start))
            .filter(dsl::start_datetime.le(&end))
            .order((dsl::start_datetime.asc(), dsl::odometer.asc()))
            .load::<TripRow>(conn)?;
        let names = place_names(conn)?;

        Ok(rows.into_iter().map(|r| Trip::from_row(r, &names)).collect())
    }

    /// Get distinct years that have trips for a vehicle
    pub fn get_years_with_trips(&self, vehicle_id: &str) -> QueryResult<Vec<i32>> {
        let conn = &mut *self.conn.lock().unwrap();

        // Raw SQL needed for strftime - use start_datetime now
        #[derive(QueryableByName)]
        struct YearRow {
            #[diesel(sql_type = diesel::sql_types::Integer)]
            year: i32,
        }

        let rows = diesel::sql_query(
            "SELECT DISTINCT CAST(strftime('%Y', start_datetime) AS INTEGER) as year
             FROM trips WHERE vehicle_id = ? ORDER BY year DESC",
        )
        .bind::<diesel::sql_types::Text, _>(vehicle_id)
        .load::<YearRow>(conn)?;

        Ok(rows.into_iter().map(|r| r.year).collect())
    }

    pub fn update_trip(&self, trip: &Trip) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        let id_str = trip.id.to_string();
        let vehicle_id_str = trip.vehicle_id.to_string();
        let start_datetime_str = trip.start_datetime.format("%Y-%m-%dT%H:%M:%S").to_string();
        let end_datetime_str = trip
            .end_datetime
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string());
        let updated_at_str = trip.updated_at.to_rfc3339();
        let origin_place_id_str = trip.origin_place_id.to_string();
        let destination_place_id_str = trip.destination_place_id.to_string();

        diesel::update(trips::table.filter(trips::id.eq(&id_str)))
            .set((
                trips::vehicle_id.eq(&vehicle_id_str),
                trips::origin_place_id.eq(&origin_place_id_str),
                trips::destination_place_id.eq(&destination_place_id_str),
                trips::distance_km.eq(trip.distance_km),
                trips::odometer.eq(trip.odometer),
                trips::purpose.eq(&trip.purpose),
                trips::fuel_liters.eq(trip.fuel_liters),
                trips::fuel_cost_eur.eq(trip.fuel_cost_eur),
                trips::other_costs_eur.eq(trip.other_costs_eur),
                trips::other_costs_note.eq(&trip.other_costs_note),
                trips::full_tank.eq(if trip.full_tank { 1 } else { 0 }),
                trips::energy_kwh.eq(trip.energy_kwh),
                trips::energy_cost_eur.eq(trip.energy_cost_eur),
                trips::full_charge.eq(Some(if trip.full_charge { 1 } else { 0 })),
                trips::soc_override_percent.eq(trip.soc_override_percent),
                trips::updated_at.eq(&updated_at_str),
                trips::start_datetime.eq(&start_datetime_str),
                trips::end_datetime.eq(end_datetime_str.as_deref()),
            ))
            .execute(conn)?;

        Ok(())
    }

    pub fn delete_trip(&self, id: &str) -> QueryResult<()> {
        use crate::schema::paperless_trip_links::dsl as p;
        let conn = &mut *self.conn.lock().unwrap();
        diesel::delete(p::paperless_trip_links.filter(p::trip_id.eq(id))).execute(conn)?;
        diesel::delete(trips::table.filter(trips::id.eq(id))).execute(conn)?;
        Ok(())
    }

    /// Apply the odometer shifts of one cascade inside an open transaction.
    /// A shift naming a row that is not there is an error, not a silent
    /// no-op: it means the caller planned against a book that has since moved.
    fn apply_odometer_shifts(
        tx: &mut SqliteConnection,
        shifts: &[(String, f64)],
        updated_at: &str,
    ) -> QueryResult<()> {
        for (shift_id, new_odometer) in shifts {
            let rows = diesel::update(trips::table.filter(trips::id.eq(shift_id)))
                .set((
                    trips::odometer.eq(new_odometer),
                    trips::updated_at.eq(updated_at),
                ))
                .execute(tx)?;
            if rows != 1 {
                return Err(diesel::result::Error::NotFound);
            }
        }
        Ok(())
    }

    /// Write one full row and move the odometer of others, in one transaction.
    ///
    /// A cascading save is one correction to a legal record, so it commits
    /// whole or not at all (task 81). The shifted rows change their odometer
    /// and their `updated_at` and nothing else -- never their distance, which
    /// is what the book records as driven.
    ///
    /// A shift naming a row that is not in the table is an error, not a
    /// silent no-op: it means the caller planned against a book that has
    /// since moved.
    pub fn update_trip_with_odometer_shift(
        &self,
        trip: &Trip,
        shifts: &[(String, f64)],
    ) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        let updated_at_str = trip.updated_at.to_rfc3339();

        conn.transaction::<_, diesel::result::Error, _>(|tx| {
            Self::update_trip_tx(tx, trip, &updated_at_str)?;
            Self::apply_odometer_shifts(tx, shifts, &updated_at_str)
        })
    }

    /// Write every column of one trip row inside an open transaction.
    fn update_trip_tx(tx: &mut SqliteConnection, trip: &Trip, updated_at: &str) -> QueryResult<()> {
        let id_str = trip.id.to_string();
        let vehicle_id_str = trip.vehicle_id.to_string();
        let start_datetime_str = trip.start_datetime.format("%Y-%m-%dT%H:%M:%S").to_string();
        let end_datetime_str = trip
            .end_datetime
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string());
        let origin_place_id_str = trip.origin_place_id.to_string();
        let destination_place_id_str = trip.destination_place_id.to_string();

        diesel::update(trips::table.filter(trips::id.eq(&id_str)))
            .set((
                trips::vehicle_id.eq(&vehicle_id_str),
                trips::origin_place_id.eq(&origin_place_id_str),
                trips::destination_place_id.eq(&destination_place_id_str),
                trips::distance_km.eq(trip.distance_km),
                trips::odometer.eq(trip.odometer),
                trips::purpose.eq(&trip.purpose),
                trips::fuel_liters.eq(trip.fuel_liters),
                trips::fuel_cost_eur.eq(trip.fuel_cost_eur),
                trips::other_costs_eur.eq(trip.other_costs_eur),
                trips::other_costs_note.eq(&trip.other_costs_note),
                trips::full_tank.eq(if trip.full_tank { 1 } else { 0 }),
                trips::energy_kwh.eq(trip.energy_kwh),
                trips::energy_cost_eur.eq(trip.energy_cost_eur),
                trips::full_charge.eq(Some(if trip.full_charge { 1 } else { 0 })),
                trips::soc_override_percent.eq(trip.soc_override_percent),
                trips::updated_at.eq(updated_at),
                trips::start_datetime.eq(&start_datetime_str),
                trips::end_datetime.eq(end_datetime_str.as_deref()),
            ))
            .execute(tx)?;
        Ok(())
    }

    /// Insert one full row and move the odometer of others, in one
    /// transaction. See `update_trip_with_odometer_shift` for why this must
    /// be all-or-nothing and why a shift naming an unknown row is an error.
    pub fn create_trip_with_odometer_shift(
        &self,
        trip: &Trip,
        shifts: &[(String, f64)],
    ) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        let id_str = trip.id.to_string();
        let vehicle_id_str = trip.vehicle_id.to_string();
        let start_datetime_str = trip.start_datetime.format("%Y-%m-%dT%H:%M:%S").to_string();
        let end_datetime_str = trip
            .end_datetime
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string());
        let created_at_str = trip.created_at.to_rfc3339();
        let updated_at_str = trip.updated_at.to_rfc3339();
        let other_costs_note_ref = trip.other_costs_note.as_deref();
        let origin_place_id_str = trip.origin_place_id.to_string();
        let destination_place_id_str = trip.destination_place_id.to_string();

        let new_trip = NewTripRow {
            id: &id_str,
            vehicle_id: &vehicle_id_str,
            origin_place_id: &origin_place_id_str,
            destination_place_id: &destination_place_id_str,
            distance_km: trip.distance_km,
            odometer: trip.odometer,
            purpose: &trip.purpose,
            fuel_liters: trip.fuel_liters,
            fuel_cost_eur: trip.fuel_cost_eur,
            other_costs_eur: trip.other_costs_eur,
            other_costs_note: other_costs_note_ref,
            full_tank: if trip.full_tank { 1 } else { 0 },
            energy_kwh: trip.energy_kwh,
            energy_cost_eur: trip.energy_cost_eur,
            full_charge: Some(if trip.full_charge { 1 } else { 0 }),
            soc_override_percent: trip.soc_override_percent,
            created_at: &created_at_str,
            updated_at: &updated_at_str,
            start_datetime: &start_datetime_str,
            end_datetime: end_datetime_str.as_deref(),
        };

        conn.transaction::<_, diesel::result::Error, _>(|tx| {
            diesel::insert_into(trips::table)
                .values(&new_trip)
                .execute(tx)?;

            Self::apply_odometer_shifts(tx, shifts, &updated_at_str)
        })
    }

    /// Delete one row and move the odometer of others, in one transaction.
    /// See `update_trip_with_odometer_shift` for why this must be
    /// all-or-nothing and why a shift naming an unknown row is an error.
    pub fn delete_trip_with_odometer_shift(
        &self,
        id: &str,
        shifts: &[(String, f64)],
    ) -> QueryResult<()> {
        use crate::schema::paperless_trip_links::dsl as p;
        let conn = &mut *self.conn.lock().unwrap();
        let updated_at_str = Utc::now().to_rfc3339();

        conn.transaction::<_, diesel::result::Error, _>(|tx| {
            diesel::delete(p::paperless_trip_links.filter(p::trip_id.eq(id))).execute(tx)?;
            diesel::delete(trips::table.filter(trips::id.eq(id))).execute(tx)?;

            Self::apply_odometer_shifts(tx, shifts, &updated_at_str)
        })
    }

    // ========================================================================
    // Route CRUD Operations
    // ========================================================================

    /// Autocomplete suggestions for a vehicle, most-used first.
    ///
    /// `usage_count` and `last_used` are computed from `trips` rather than
    /// stored (ADR-033): three write paths were meant to keep stored copies
    /// current and none did, leaving 52 of 96 rows wrong in production.
    ///
    /// The join is INNER by design. A `routes` row whose trips have all been
    /// deleted is a suggestion for a journey the logbook no longer contains,
    /// so it drops out here instead of needing a cleanup pass.
    pub fn get_routes_for_vehicle(&self, vehicle_id: &str) -> QueryResult<Vec<Route>> {
        let conn = &mut *self.conn.lock().unwrap();

        #[derive(QueryableByName)]
        struct DerivedRouteRow {
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            id: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Text)]
            vehicle_id: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            origin_place_id: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            destination_place_id: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            origin: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            destination: String,
            #[diesel(sql_type = diesel::sql_types::Double)]
            distance_km: f64,
            #[diesel(sql_type = diesel::sql_types::Integer)]
            usage_count: i32,
            #[diesel(sql_type = diesel::sql_types::Text)]
            last_used: String,
        }

        // `last_used` arrives as a `trips.start_datetime` string, not the
        // RFC 3339 the dropped column held, so it is parsed the way
        // `find_most_recent_trip_times_for_route` parses the same field.
        fn derived_route(row: DerivedRouteRow) -> Route {
            Route {
                id: Uuid::parse_str(row.id.as_deref().unwrap_or_default())
                    .unwrap_or_else(|_| Uuid::new_v4()),
                vehicle_id: Uuid::parse_str(&row.vehicle_id).unwrap_or_else(|_| Uuid::new_v4()),
                origin_place_id: Uuid::parse_str(&row.origin_place_id)
                    .unwrap_or_else(|_| Uuid::nil()),
                destination_place_id: Uuid::parse_str(&row.destination_place_id)
                    .unwrap_or_else(|_| Uuid::nil()),
                origin: row.origin,
                destination: row.destination,
                distance_km: row.distance_km,
                usage_count: row.usage_count,
                last_used: NaiveDateTime::parse_from_str(&row.last_used, "%Y-%m-%dT%H:%M:%S")
                    .map(|dt| dt.and_utc())
                    .unwrap_or_else(|_| Utc::now()),
            }
        }

        let rows = diesel::sql_query(
            "SELECT r.id, r.vehicle_id, r.origin_place_id, r.destination_place_id,
                    po.name AS origin, pd.name AS destination, r.distance_km,
                    COUNT(t.id) AS usage_count,
                    MAX(t.start_datetime) AS last_used
               FROM routes r
               JOIN places po ON po.id = r.origin_place_id
               JOIN places pd ON pd.id = r.destination_place_id
               JOIN trips t
                 ON t.vehicle_id = r.vehicle_id
                AND t.origin_place_id = r.origin_place_id
                AND t.destination_place_id = r.destination_place_id
              WHERE r.vehicle_id = ?
              GROUP BY r.id
              ORDER BY usage_count DESC",
        )
        .bind::<diesel::sql_types::Text, _>(vehicle_id)
        .load::<DerivedRouteRow>(conn)?;

        Ok(rows.into_iter().map(derived_route).collect())
    }

    /// Get all unique trip purposes for a vehicle (raw SQL for DISTINCT TRIM)
    pub fn get_purposes_for_vehicle(&self, vehicle_id: &str) -> QueryResult<Vec<String>> {
        let conn = &mut *self.conn.lock().unwrap();

        #[derive(QueryableByName)]
        struct PurposeRow {
            #[diesel(sql_type = diesel::sql_types::Text)]
            purpose: String,
        }

        let rows = diesel::sql_query(
            "SELECT DISTINCT TRIM(purpose) as purpose
             FROM trips
             WHERE vehicle_id = ? AND TRIM(purpose) != ''
             ORDER BY purpose",
        )
        .bind::<diesel::sql_types::Text, _>(vehicle_id)
        .load::<PurposeRow>(conn)?;

        Ok(rows.into_iter().map(|r| r.purpose).collect())
    }

    /// Find the most recent trip's `(start_datetime, end_datetime)` for a given
    /// vehicle and place pair. Excludes trips with a null `end_datetime`.
    /// Returns `None` if no completed match exists.
    pub fn find_most_recent_trip_times_for_route(
        &self,
        vehicle_id: &str,
        origin_place_id: &str,
        destination_place_id: &str,
    ) -> QueryResult<Option<(NaiveDateTime, NaiveDateTime)>> {
        use crate::schema::trips::dsl;
        let conn = &mut *self.conn.lock().unwrap();

        let row = dsl::trips
            .filter(dsl::vehicle_id.eq(vehicle_id))
            .filter(dsl::origin_place_id.eq(origin_place_id))
            .filter(dsl::destination_place_id.eq(destination_place_id))
            .filter(dsl::end_datetime.is_not_null())
            .order(dsl::start_datetime.desc())
            .first::<TripRow>(conn)
            .optional()?;

        Ok(row.and_then(|r| {
            let start =
                NaiveDateTime::parse_from_str(&r.start_datetime, "%Y-%m-%dT%H:%M:%S").ok()?;
            let end_str = r.end_datetime?;
            let end = NaiveDateTime::parse_from_str(&end_str, "%Y-%m-%dT%H:%M:%S").ok()?;
            Some((start, end))
        }))
    }

    /// Find the route for this vehicle and place pair, or create it.
    ///
    /// Returns the stored row, which carries no usage figures: how often a pair
    /// is driven is derived from `trips` by `get_routes_for_vehicle` (ADR-033).
    pub fn find_or_create_route(
        &self,
        vehicle_id: &str,
        origin_place_id: &str,
        destination_place_id: &str,
        distance_km: f64,
    ) -> QueryResult<RouteRow> {
        let conn = &mut *self.conn.lock().unwrap();

        let existing = routes::table
            .filter(routes::vehicle_id.eq(vehicle_id))
            .filter(routes::origin_place_id.eq(origin_place_id))
            .filter(routes::destination_place_id.eq(destination_place_id))
            .first::<RouteRow>(conn)
            .optional()?;

        if let Some(row) = existing {
            // The pair is already recorded and nothing about it needs updating:
            // saving the same trip again used to bump a counter here, and that
            // counter is what made the table lie.
            return Ok(row);
        }

        let id = Uuid::new_v4().to_string();
        let new_route = NewRouteRow {
            id: &id,
            vehicle_id,
            origin_place_id,
            destination_place_id,
            distance_km,
        };

        diesel::insert_into(routes::table)
            .values(&new_route)
            .execute(conn)?;

        Ok(RouteRow {
            id: Some(id),
            vehicle_id: vehicle_id.to_string(),
            origin_place_id: origin_place_id.to_string(),
            destination_place_id: destination_place_id.to_string(),
            distance_km,
        })
    }

    /// Rows as stored, bypassing the trips join. Tests about the table itself
    /// need this; nothing in the application does.
    #[cfg(test)]
    pub fn all_route_rows_for_test(&self, vehicle_id: &str) -> QueryResult<Vec<RouteRow>> {
        let conn = &mut *self.conn.lock().unwrap();
        routes::table
            .filter(routes::vehicle_id.eq(vehicle_id))
            .load::<RouteRow>(conn)
    }

    // ========================================================================
    // Settings CRUD Operations
    // ========================================================================

    pub fn get_settings(&self) -> QueryResult<Option<Settings>> {
        let conn = &mut *self.conn.lock().unwrap();

        let row = settings::table.first::<SettingsRow>(conn).optional()?;

        Ok(row.map(Settings::from))
    }

    pub fn save_settings(&self, s: &Settings) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();

        // Check if settings exist
        let exists: i64 = settings::table.count().get_result(conn)?;

        let id_str = s.id.to_string();
        let updated_at_str = s.updated_at.to_rfc3339();

        if exists > 0 {
            // Update existing settings
            diesel::update(settings::table)
                .set((
                    settings::company_name.eq(&s.company_name),
                    settings::company_ico.eq(&s.company_ico),
                    settings::buffer_trip_purpose.eq(&s.buffer_trip_purpose),
                    settings::updated_at.eq(&updated_at_str),
                ))
                .execute(conn)?;
        } else {
            // Insert new settings
            let new_settings = NewSettingsRow {
                id: &id_str,
                company_name: &s.company_name,
                company_ico: &s.company_ico,
                buffer_trip_purpose: &s.buffer_trip_purpose,
                updated_at: &updated_at_str,
            };

            diesel::insert_into(settings::table)
                .values(&new_settings)
                .execute(conn)?;
        }

        Ok(())
    }

    // ========================================================================
    // Paperless trip links — one trip per doc, N docs per trip (Task 66)
    // ========================================================================

    /// Upsert a paperless doc→trip link, keyed on `paperless_document_id`
    /// ONLY: any prior row for THIS doc (possibly on another trip) is
    /// replaced. A trip may hold many docs — never delete by trip_id.
    /// The one-Fuel-per-trip rule is enforced by the partial unique index
    /// `idx_paperless_links_trip_fuel` (insert fails with a constraint error).
    pub fn upsert_paperless_link(&self, link: &PaperlessLink) -> QueryResult<()> {
        use crate::schema::paperless_trip_links::dsl as p;
        let conn = &mut *self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        conn.transaction::<_, diesel::result::Error, _>(|tx| {
            // Clear any prior link to THIS doc (might be on a different trip).
            diesel::delete(
                p::paperless_trip_links
                    .filter(p::paperless_document_id.eq(link.paperless_document_id)),
            )
            .execute(tx)?;
            diesel::insert_into(p::paperless_trip_links)
                .values((
                    p::paperless_document_id.eq(link.paperless_document_id),
                    p::trip_id.eq(&link.trip_id),
                    p::assignment_type.eq(link.assignment_type.as_str()),
                    p::amount_eur.eq(link.amount_eur),
                    p::title.eq(&link.title),
                    p::applied_amount_cents.eq(link.applied_amount_cents),
                    p::receipt_datetime
                        .eq(link.receipt_datetime.map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string())),
                    p::mismatch_override.eq(link.mismatch_override),
                    p::created_at.eq(&now),
                    p::updated_at.eq(&now),
                ))
                .execute(tx)?;
            Ok(())
        })
    }

    /// Set the mismatch_override flag for one link.
    pub fn set_paperless_override(&self, doc_id: i64, value: bool) -> QueryResult<()> {
        use crate::schema::paperless_trip_links::dsl as p;
        let conn = &mut *self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        diesel::update(p::paperless_trip_links.filter(p::paperless_document_id.eq(doc_id)))
            .set((p::mismatch_override.eq(value), p::updated_at.eq(now)))
            .execute(conn)
            .map(|_| ())
    }

    pub fn delete_paperless_link_for_doc(&self, doc_id: i64) -> QueryResult<()> {
        use crate::schema::paperless_trip_links::dsl as p;
        let conn = &mut *self.conn.lock().unwrap();
        diesel::delete(p::paperless_trip_links.filter(p::paperless_document_id.eq(doc_id)))
            .execute(conn)
            .map(|_| ())
    }

    /// Full link row for a doc (one trip per doc — PK lookup).
    pub fn get_paperless_link(&self, doc_id: i64) -> QueryResult<Option<PaperlessLink>> {
        use crate::schema::paperless_trip_links::dsl as p;
        let conn = &mut *self.conn.lock().unwrap();
        p::paperless_trip_links
            .filter(p::paperless_document_id.eq(doc_id))
            .select((
                p::paperless_document_id,
                p::trip_id,
                p::assignment_type,
                p::amount_eur,
                p::title,
                p::applied_amount_cents,
                p::receipt_datetime,
                p::mismatch_override,
            ))
            .first::<PaperlessLinkRow>(conn)
            .optional()
            .map(|row| row.map(paperless_link_from_row))
    }

    /// All link rows attached to a trip (N docs per trip since Task 66).
    pub fn get_paperless_links_for_trip(&self, trip_id: &str) -> QueryResult<Vec<PaperlessLink>> {
        use crate::schema::paperless_trip_links::dsl as p;
        let conn = &mut *self.conn.lock().unwrap();
        p::paperless_trip_links
            .filter(p::trip_id.eq(trip_id))
            .select((
                p::paperless_document_id,
                p::trip_id,
                p::assignment_type,
                p::amount_eur,
                p::title,
                p::applied_amount_cents,
                p::receipt_datetime,
                p::mismatch_override,
            ))
            .load::<PaperlessLinkRow>(conn)
            .map(|rows| rows.into_iter().map(paperless_link_from_row).collect())
    }

    pub fn list_paperless_links_for_docs(
        &self,
        doc_ids: &[i64],
    ) -> QueryResult<Vec<PaperlessLink>> {
        use crate::schema::paperless_trip_links::dsl as p;
        let conn = &mut *self.conn.lock().unwrap();
        p::paperless_trip_links
            .filter(p::paperless_document_id.eq_any(doc_ids))
            .select((
                p::paperless_document_id,
                p::trip_id,
                p::assignment_type,
                p::amount_eur,
                p::title,
                p::applied_amount_cents,
                p::receipt_datetime,
                p::mismatch_override,
            ))
            .load::<PaperlessLinkRow>(conn)
            .map(|rows| rows.into_iter().map(paperless_link_from_row).collect())
    }

    /// All link rows (grid datetime warnings read the whole set once).
    pub fn get_all_paperless_links(&self) -> QueryResult<Vec<PaperlessLink>> {
        use crate::schema::paperless_trip_links::dsl as p;
        let conn = &mut *self.conn.lock().unwrap();
        p::paperless_trip_links
            .select((
                p::paperless_document_id,
                p::trip_id,
                p::assignment_type,
                p::amount_eur,
                p::title,
                p::applied_amount_cents,
                p::receipt_datetime,
                p::mismatch_override,
            ))
            .load::<PaperlessLinkRow>(conn)
            .map(|rows| rows.into_iter().map(paperless_link_from_row).collect())
    }

    #[cfg(test)]
    pub fn count_paperless_links(&self) -> QueryResult<i64> {
        use crate::schema::paperless_trip_links::dsl as p;
        let conn = &mut *self.conn.lock().unwrap();
        p::paperless_trip_links.count().get_result(conn)
    }

    // ========================================================================
    // Source-agnostic invoice attachment query
    // ========================================================================

    /// Per-type invoice coverage for every trip that has at least one invoice.
    /// Paperless links only; sums use integer cents. Other amounts come from
    /// paperless assign-time `amount_eur` snapshots; a NULL amount on any Other
    /// invoice sets `has_unknown_amount` so the sum-mismatch check can be skipped.
    pub fn get_trip_invoice_coverage(&self) -> QueryResult<HashMap<String, TripInvoiceCoverage>> {
        use crate::calculations::to_cents;
        use crate::schema::paperless_trip_links::dsl as p;
        let conn = &mut *self.conn.lock().unwrap();

        let mut coverage: HashMap<String, TripInvoiceCoverage> = HashMap::new();

        // Paperless links (amounts read from assign-time snapshots)
        let link_rows: Vec<(String, String, Option<f64>)> = p::paperless_trip_links
            .select((p::trip_id, p::assignment_type, p::amount_eur))
            .load(conn)?;
        for (trip_id, assignment_type, amount) in link_rows {
            let entry = coverage.entry(trip_id).or_default();
            match AssignmentType::from_str(&assignment_type) {
                Some(AssignmentType::Fuel) => entry.has_fuel = true,
                // assignment_type is NOT NULL and app-constrained to
                // Fuel/Other; treat anything unexpected as Other.
                _ => {
                    entry.has_other = true;
                    match amount {
                        Some(a) => entry.other_sum_cents += to_cents(a),
                        None => entry.has_unknown_amount = true,
                    }
                }
            }
        }

        Ok(coverage)
    }

    // ========================================================================
    // Route map CRUD — one generated map per trip (Task 70)
    // ========================================================================

    /// Upsert the generated map for a trip. `trip_routes.trip_id` is the
    /// primary key, so re-generating a map must replace the old row rather
    /// than fail on the PK. Delete + insert run inside one transaction so a
    /// failed insert can never leave the trip mapless.
    pub fn save_route_map(&self, map: &RouteMap) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        conn.transaction::<_, diesel::result::Error, _>(|tx| Self::insert_route_map_tx(tx, map))
    }

    /// Save a route map and write the distance it implies, in one transaction
    /// (task 87). `trip` is `None` when the rounded road distance already equals
    /// the trip's: the map is saved and the trip row is not touched. A shift
    /// naming an unknown row rolls back the map too -- a saved map whose distance
    /// did not reach the trip is the state this method exists to prevent.
    pub fn save_route_map_with_trip_distance(
        &self,
        map: &RouteMap,
        trip: Option<&Trip>,
        shifts: &[(String, f64)],
    ) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        conn.transaction::<_, diesel::result::Error, _>(|tx| {
            Self::insert_route_map_tx(tx, map)?;
            if let Some(trip) = trip {
                let updated_at = trip.updated_at.to_rfc3339();
                Self::update_trip_tx(tx, trip, &updated_at)?;
                Self::apply_odometer_shifts(tx, shifts, &updated_at)?;
            }
            Ok(())
        })
    }

    /// Delete + insert the map row for one trip inside an open transaction.
    fn insert_route_map_tx(tx: &mut SqliteConnection, map: &RouteMap) -> QueryResult<()> {
        let trip_id_str = map.trip_id.to_string();
        let waypoints_json = serde_json::to_string(&map.waypoints)
            .map_err(|e| diesel::result::Error::SerializationError(Box::new(e)))?;
        let avoid_json = serde_json::to_string(&map.avoid)
            .map_err(|e| diesel::result::Error::SerializationError(Box::new(e)))?;
        let created_at_str = map.created_at.to_rfc3339();

        diesel::delete(trip_routes::table.filter(trip_routes::trip_id.eq(&trip_id_str)))
            .execute(tx)?;
        diesel::insert_into(trip_routes::table)
            .values(&NewRouteMapRow {
                trip_id: &trip_id_str,
                waypoints: &waypoints_json,
                polyline: &map.polyline,
                target_km: map.target_km,
                road_km: map.road_km,
                dataset_version: map.dataset_version.as_deref(),
                created_at: &created_at_str,
                mode: map.mode.as_str(),
                round_trip: map.round_trip,
                turnaround_index: map.turnaround_index,
                avoid: &avoid_json,
                provider: map.provider.map(|k| k.as_str()),
            })
            .execute(tx)?;
        Ok(())
    }

    pub fn get_route_map(&self, trip_id: &str) -> QueryResult<Option<RouteMap>> {
        let conn = &mut *self.conn.lock().unwrap();

        let row = trip_routes::table
            .filter(trip_routes::trip_id.eq(trip_id))
            .first::<RouteMapRow>(conn)
            .optional()?;

        Ok(row.map(RouteMap::from))
    }

    /// Deleting a map that was never generated is a no-op, not an error.
    pub fn delete_route_map(&self, trip_id: &str) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        diesel::delete(trip_routes::table.filter(trip_routes::trip_id.eq(trip_id)))
            .execute(conn)
            .map(|_| ())
    }

    /// Maps for many trips in ONE query — the export renders a whole year and
    /// must not issue a query per trip. Trips without a map are simply absent
    /// from the returned map, so callers can look up by trip id directly.
    pub fn get_route_maps_for_trips(
        &self,
        trip_ids: &[String],
    ) -> QueryResult<HashMap<String, RouteMap>> {
        let conn = &mut *self.conn.lock().unwrap();

        let rows = trip_routes::table
            .filter(trip_routes::trip_id.eq_any(trip_ids))
            .load::<RouteMapRow>(conn)?;

        Ok(rows
            .into_iter()
            .map(|row| (row.trip_id.clone(), RouteMap::from(row)))
            .collect())
    }

    /// The trips in `trip_ids` whose saved map is a round trip. Journey
    /// grouping needs only this flag, so it does not load the geometry.
    pub fn get_round_trip_ids(&self, trip_ids: &[String]) -> QueryResult<Vec<String>> {
        let conn = &mut *self.conn.lock().unwrap();
        trip_routes::table
            .filter(trip_routes::trip_id.eq_any(trip_ids))
            .filter(trip_routes::round_trip.eq(true))
            .select(trip_routes::trip_id)
            .load(conn)
    }

    // ========================================================================
    // Places -- an entity with an id, a name and a position (Task 88)
    // ========================================================================

    pub fn get_place(&self, id: &str) -> QueryResult<Option<PlaceRow>> {
        let conn = &mut *self.conn.lock().unwrap();
        places::table
            .filter(places::id.eq(id))
            .select(PlaceRow::as_select())
            .first(conn)
            .optional()
    }

    pub fn get_place_by_key(&self, normalised_name: &str) -> QueryResult<Option<PlaceRow>> {
        let conn = &mut *self.conn.lock().unwrap();
        places::table
            .filter(places::normalised_name.eq(normalised_name))
            .select(PlaceRow::as_select())
            .first(conn)
            .optional()
    }

    /// Every place in the book. Small by construction (tens of rows), so the
    /// caller indexes it in memory rather than querying per place.
    pub fn all_places(&self) -> QueryResult<Vec<PlaceRow>> {
        let conn = &mut *self.conn.lock().unwrap();
        places::table.select(PlaceRow::as_select()).load(conn)
    }

    /// The place marked as home, if any (task 89). The partial unique index
    /// `idx_places_single_home` guarantees at most one row.
    pub fn get_home_place(&self) -> QueryResult<Option<PlaceRow>> {
        let conn = &mut *self.conn.lock().unwrap();
        places::table
            .filter(places::is_home.eq(true))
            .select(PlaceRow::as_select())
            .first(conn)
            .optional()
    }

    /// Move the home mark to `id`, or clear it with `None`. One transaction:
    /// the old mark goes first, so the unique index never sees two homes.
    /// An unknown ID rolls back and returns `NotFound`, so the old mark stays.
    pub fn set_home_place(&self, id: Option<&str>) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        conn.transaction::<_, diesel::result::Error, _>(|tx| {
            diesel::update(places::table.filter(places::is_home.eq(true)))
                .set(places::is_home.eq(false))
                .execute(tx)?;
            if let Some(id) = id {
                let changed = diesel::update(places::table.filter(places::id.eq(id)))
                    .set(places::is_home.eq(true))
                    .execute(tx)?;
                if changed == 0 {
                    return Err(diesel::result::Error::NotFound);
                }
            }
            Ok(())
        })
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
        diesel::insert_into(places::table)
            .values(place)
            .execute(conn)
            .map(|_| ())
    }

    pub fn rename_place(&self, id: &str, name: &str, normalised_name: &str) -> QueryResult<usize> {
        let conn = &mut *self.conn.lock().unwrap();
        diesel::update(places::table.filter(places::id.eq(id)))
            .set((places::name.eq(name), places::normalised_name.eq(normalised_name)))
            .execute(conn)
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
                routes::table
                    .filter(routes::origin_place_id.eq(id).or(routes::destination_place_id.eq(id))),
            )
            .execute(tx)?;
            let n = diesel::delete(places::table.filter(places::id.eq(id))).execute(tx)?;
            Ok(if n == 1 { DeletePlaceOutcome::Deleted } else { DeletePlaceOutcome::NotFound })
        })
    }

    pub fn set_place_position(
        &self,
        id: &str,
        lat: f64,
        lon: f64,
        source: &str,
    ) -> QueryResult<usize> {
        let conn = &mut *self.conn.lock().unwrap();
        diesel::update(places::table.filter(places::id.eq(id)))
            .set((
                places::lat.eq(Some(lat)),
                places::lon.eq(Some(lon)),
                places::source.eq(Some(source)),
            ))
            .execute(conn)
    }
}

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
            id: &id_str,
            name,
            normalised_name: &key,
            lat: Some(48.15),
            lon: Some(17.11),
            source: Some("manual"),
            created_at: &now,
        })
        .unwrap();
        id
    }

    /// A copy of `trip` whose nil place ids are filled from its names, by
    /// `ensure_place_for_test`. Tests build trips by name; foreign keys are
    /// on, so each insert or update of such a trip goes through this.
    pub fn with_places_for_test(&self, trip: &Trip) -> Trip {
        let mut placed = trip.clone();
        if placed.origin_place_id.is_nil() {
            placed.origin_place_id = self.ensure_place_for_test(&trip.origin);
        }
        if placed.destination_place_id.is_nil() {
            placed.destination_place_id = self.ensure_place_for_test(&trip.destination);
        }
        placed
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
            id: &id_str,
            name,
            normalised_name: &key,
            lat: None,
            lon: None,
            source: None,
            created_at: &now,
        })
        .unwrap();
        id
    }
}

/// Tuple row for paperless link selects (created_at/updated_at are
/// DB-managed and not part of the domain struct).
type PaperlessLinkRow = (
    i64,
    String,
    String,
    Option<f64>,
    Option<String>,
    Option<i64>,
    Option<String>,
    bool,
);

fn paperless_link_from_row(row: PaperlessLinkRow) -> PaperlessLink {
    let (
        paperless_document_id,
        trip_id,
        assignment_type,
        amount_eur,
        title,
        applied_amount_cents,
        receipt_datetime,
        mismatch_override,
    ) = row;
    PaperlessLink {
        paperless_document_id,
        trip_id,
        // NOT NULL + app-constrained; fall back to Other for unexpected values.
        assignment_type: AssignmentType::from_str(&assignment_type)
            .unwrap_or(AssignmentType::Other),
        amount_eur,
        title,
        applied_amount_cents,
        receipt_datetime: receipt_datetime.as_deref().and_then(|s| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok()
        }),
        mismatch_override,
    }
}

// ============================================================================
// Test-only migration harness (Task 66: migration data-integrity tests)
// ============================================================================

/// Version cutoff for the multi-invoice migration: every migration whose
/// version sorts strictly below this date belongs to the "legacy"
/// (pre-multi-invoice) schema. Kept in the human-readable dashed form;
/// [`open_db_legacy`] normalizes it the way diesel does before comparing.
#[cfg(test)]
pub(crate) const MULTI_INVOICE_VERSION: &str = "2026-07-15";

/// Test harness: open an in-memory DB migrated only UP TO (excluding) the
/// multi-invoice migration, so tests can seed legacy-shaped rows and then
/// run the remaining migrations against real data.
///
/// Implementation notes (verified against diesel_migrations 2.3 source):
/// - `MigrationSource::migrations()` order is NOT contractually sorted —
///   sort by `name().version()` (plain lexical `Cow<str>` ordering, the
///   same ordering `run_pending_migrations` uses) before replaying.
/// - `version()` is the directory name up to the FIRST underscore, with
///   dashes STRIPPED (`migrations_internals::version_from_string`), e.g.
///   "2026-01-09-100000_add_x" -> "20260109100000" — so the cutoff must be
///   normalized identically, and full directory names must never be
///   string-matched.
/// - `run_migration` does NOT create `__diesel_schema_migrations`;
///   `applied_migrations()` does (via `MigrationConnection::setup`), so it
///   must be called first.
/// Open an in-memory DB migrated only up to (excluding) `cutoff`.
///
/// Parameterised so a test can stand at the boundary of ITS OWN migration --
/// the multi-invoice cutoff predates several later tables, and a test for one
/// of those cannot seed rows a legacy DB has no table for.
#[cfg(test)]
pub(crate) fn open_db_legacy_before(cutoff: &str) -> Database {
    let mut conn = SqliteConnection::establish(":memory:")
        .expect("Failed to open in-memory legacy database");
    prepare_connection(&mut conn).expect("Failed to register SQL functions");

    // Create the __diesel_schema_migrations tracking table (run_migration
    // records into it but never creates it).
    conn.applied_migrations()
        .expect("Failed to set up migration tracking table");

    let mut migrations =
        <EmbeddedMigrations as MigrationSource<diesel::sqlite::Sqlite>>::migrations(&MIGRATIONS)
            .expect("Failed to enumerate embedded migrations");
    migrations.sort_by_key(|m| m.name().version().as_owned());

    // Normalize the cutoff exactly like diesel normalizes versions
    // (dashes stripped): "2026-07-15" -> "20260715".
    let normalized_cutoff = cutoff.replace('-', "");
    let cutoff = diesel::migration::MigrationVersion::from(normalized_cutoff.as_str());
    for migration in migrations.iter().filter(|m| m.name().version() < cutoff) {
        conn.run_migration(migration.as_ref()).unwrap_or_else(|e| {
            panic!(
                "Failed to run legacy migration {}: {}",
                migration.name(),
                e
            )
        });
    }

    Database {
        conn: Mutex::new(conn),
    }
}

#[cfg(test)]
pub(crate) fn open_db_legacy() -> Database {
    open_db_legacy_before(MULTI_INVOICE_VERSION)
}

/// Run the remaining migrations on a legacy DB.
#[cfg(test)]
pub(crate) fn migrate_to_current(db: &Database) {
    let conn = &mut *db.conn.lock().unwrap();
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to migrate legacy DB to current schema");
}

#[cfg(test)]
#[path = "db_tests.rs"]
pub(crate) mod db_tests;

#[cfg(test)]
#[path = "migration_tests.rs"]
mod migration_tests;
