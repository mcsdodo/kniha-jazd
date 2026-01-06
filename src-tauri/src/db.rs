use crate::models::{ConfidenceLevel, FieldConfidence, Receipt, ReceiptStatus, Route, Settings, Trip, Vehicle, VehicleType};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{Connection, OptionalExtension, Result};
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

/// Parse vehicle type from database string
fn parse_vehicle_type(s: &str) -> VehicleType {
    match s {
        "Bev" => VehicleType::Bev,
        "Phev" => VehicleType::Phev,
        _ => VehicleType::Ice, // Default to ICE for existing vehicles
    }
}

/// Convert vehicle type to database string
fn vehicle_type_to_string(vt: &VehicleType) -> &'static str {
    match vt {
        VehicleType::Ice => "Ice",
        VehicleType::Bev => "Bev",
        VehicleType::Phev => "Phev",
    }
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Run initial schema migration
        conn.execute_batch(include_str!("../migrations/001_initial.sql"))?;

        // Run migration to add initial_odometer column (ignore if already exists)
        // SQLite errors if column exists, so we catch and ignore that specific error
        let _ = conn.execute(
            "ALTER TABLE vehicles ADD COLUMN initial_odometer REAL NOT NULL DEFAULT 0",
            [],
        );

        // Run migration to add sort_order column (ignore if already exists)
        let _ = conn.execute(
            "ALTER TABLE trips ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0",
            [],
        );

        // Initialize sort_order for existing trips based on chronological order (newest = 0)
        // This only affects rows where sort_order = 0 and there are multiple trips
        let _ = conn.execute_batch(
            "UPDATE trips SET sort_order = (
                SELECT COUNT(*) FROM trips t2
                WHERE t2.vehicle_id = trips.vehicle_id
                AND (t2.date > trips.date OR (t2.date = trips.date AND t2.odometer > trips.odometer))
            ) WHERE sort_order = 0",
        );

        // Rename filler_trip_purpose to buffer_trip_purpose (ignore if already renamed)
        let _ = conn.execute(
            "ALTER TABLE settings RENAME COLUMN filler_trip_purpose TO buffer_trip_purpose",
            [],
        );

        // Add full_tank column (true = full tank fillup, false = partial)
        // Default 1 ensures existing data is treated as full tank
        let _ = conn.execute(
            "ALTER TABLE trips ADD COLUMN full_tank INTEGER NOT NULL DEFAULT 1",
            [],
        );

        // Add receipts table
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS receipts (
                id TEXT PRIMARY KEY,
                vehicle_id TEXT,
                trip_id TEXT UNIQUE,
                file_path TEXT NOT NULL UNIQUE,
                file_name TEXT NOT NULL,
                scanned_at TEXT NOT NULL,
                liters REAL,
                total_price_eur REAL,
                receipt_date TEXT,
                station_name TEXT,
                station_address TEXT,
                status TEXT NOT NULL DEFAULT 'Pending',
                confidence TEXT NOT NULL DEFAULT '{\"liters\":\"Unknown\",\"total_price\":\"Unknown\",\"date\":\"Unknown\"}',
                raw_ocr_text TEXT,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (vehicle_id) REFERENCES vehicles(id),
                FOREIGN KEY (trip_id) REFERENCES trips(id)
            );
            CREATE INDEX IF NOT EXISTS idx_receipts_status ON receipts(status);
            CREATE INDEX IF NOT EXISTS idx_receipts_trip ON receipts(trip_id);
            CREATE INDEX IF NOT EXISTS idx_receipts_date ON receipts(receipt_date);"
        )?;

        // Add source_year column for year-based folder structure support
        // None = flat folder, Some(year) = from year subfolder (e.g., 2024/)
        let _ = conn.execute(
            "ALTER TABLE receipts ADD COLUMN source_year INTEGER",
            [],
        );

        // === Migration 005: Add Electric Vehicle (EV) support ===
        // Vehicle EV fields
        let _ = conn.execute(
            "ALTER TABLE vehicles ADD COLUMN vehicle_type TEXT NOT NULL DEFAULT 'Ice'",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE vehicles ADD COLUMN battery_capacity_kwh REAL",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE vehicles ADD COLUMN baseline_consumption_kwh REAL",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE vehicles ADD COLUMN initial_battery_percent REAL",
            [],
        );
        // Trip energy fields
        let _ = conn.execute(
            "ALTER TABLE trips ADD COLUMN energy_kwh REAL",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE trips ADD COLUMN energy_cost_eur REAL",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE trips ADD COLUMN full_charge INTEGER DEFAULT 0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE trips ADD COLUMN soc_override_percent REAL",
            [],
        );
        // Index for vehicle type queries
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_vehicles_type ON vehicles(vehicle_type)",
            [],
        );

        // === Migration 006: Make fuel fields nullable for BEV support ===
        // Check if tank_size_liters is still NOT NULL by trying to insert NULL
        // SQLite doesn't have a direct way to check column nullability
        let needs_migration = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='vehicles'",
                [],
                |row| row.get::<_, String>(0),
            )
            .map(|sql| sql.contains("tank_size_liters REAL NOT NULL"))
            .unwrap_or(false);

        if needs_migration {
            // Recreate vehicles table with nullable fuel fields
            // Must disable foreign keys to drop the old table (trips references vehicles)
            conn.execute_batch(
                "PRAGMA foreign_keys = OFF;

                DROP TABLE IF EXISTS vehicles_new;

                -- Create new table with nullable fuel fields
                CREATE TABLE vehicles_new (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    license_plate TEXT NOT NULL,
                    vehicle_type TEXT NOT NULL DEFAULT 'Ice',
                    tank_size_liters REAL,
                    tp_consumption REAL,
                    battery_capacity_kwh REAL,
                    baseline_consumption_kwh REAL,
                    initial_battery_percent REAL,
                    initial_odometer REAL NOT NULL DEFAULT 0,
                    is_active INTEGER NOT NULL DEFAULT 1,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                -- Copy data from old table
                INSERT INTO vehicles_new (
                    id, name, license_plate, vehicle_type,
                    tank_size_liters, tp_consumption,
                    battery_capacity_kwh, baseline_consumption_kwh, initial_battery_percent,
                    initial_odometer, is_active, created_at, updated_at
                )
                SELECT
                    id, name, license_plate, vehicle_type,
                    tank_size_liters, tp_consumption,
                    battery_capacity_kwh, baseline_consumption_kwh, initial_battery_percent,
                    initial_odometer, is_active, created_at, updated_at
                FROM vehicles;

                -- Drop old table
                DROP TABLE vehicles;

                -- Rename new table
                ALTER TABLE vehicles_new RENAME TO vehicles;

                -- Recreate index
                CREATE INDEX IF NOT EXISTS idx_vehicles_type ON vehicles(vehicle_type);

                PRAGMA foreign_keys = ON;"
            )?;
        }

        Ok(())
    }

    /// Populate routes table from existing trips (used after backup restore)
    pub fn populate_routes_from_trips(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();

        // Only populate if routes table is empty
        let routes_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM routes", [], |row| row.get(0))?;

        if routes_count > 0 {
            return Ok(0); // Already has routes, don't overwrite
        }

        // Populate routes from trips (group by vehicle_id, origin, destination)
        let rows_inserted = conn.execute(
            "INSERT OR IGNORE INTO routes (id, vehicle_id, origin, destination, distance_km, usage_count, last_used)
             SELECT
                 lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab',abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
                 vehicle_id,
                 origin,
                 destination,
                 AVG(distance_km),
                 COUNT(*),
                 MAX(date)
             FROM trips
             WHERE origin != '' AND destination != ''
             GROUP BY vehicle_id, origin, destination",
            [],
        )?;

        Ok(rows_inserted)
    }

    pub fn connection(&self) -> std::sync::MutexGuard<Connection> {
        self.conn.lock().unwrap()
    }

    // Vehicle CRUD operations

    /// Create a new vehicle in the database
    pub fn create_vehicle(&self, vehicle: &Vehicle) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO vehicles (id, name, license_plate, tank_size_liters, tp_consumption, initial_odometer, is_active, created_at, updated_at)
             VALUES (:id, :name, :license_plate, :tank_size_liters, :tp_consumption, :initial_odometer, :is_active, :created_at, :updated_at)",
            rusqlite::named_params! {
                ":id": vehicle.id.to_string(),
                ":name": vehicle.name,
                ":license_plate": vehicle.license_plate,
                ":tank_size_liters": vehicle.tank_size_liters,
                ":tp_consumption": vehicle.tp_consumption,
                ":initial_odometer": vehicle.initial_odometer,
                ":is_active": vehicle.is_active,
                ":created_at": vehicle.created_at.to_rfc3339(),
                ":updated_at": vehicle.updated_at.to_rfc3339(),
            },
        )?;
        Ok(())
    }

    /// Get a vehicle by its UUID string
    pub fn get_vehicle(&self, id: &str) -> Result<Option<Vehicle>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, license_plate, tank_size_liters, tp_consumption, initial_odometer, is_active, created_at, updated_at,
                    COALESCE(vehicle_type, 'Ice') as vehicle_type,
                    battery_capacity_kwh, baseline_consumption_kwh, initial_battery_percent
             FROM vehicles WHERE id = ?1",
        )?;

        let vehicle = stmt
            .query_row([id], |row| {
                let vehicle_type_str: String = row.get(9)?;
                Ok(Vehicle {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    name: row.get(1)?,
                    license_plate: row.get(2)?,
                    vehicle_type: parse_vehicle_type(&vehicle_type_str),
                    tank_size_liters: row.get(3)?,
                    tp_consumption: row.get(4)?,
                    battery_capacity_kwh: row.get(10)?,
                    baseline_consumption_kwh: row.get(11)?,
                    initial_battery_percent: row.get(12)?,
                    initial_odometer: row.get(5)?,
                    is_active: row.get(6)?,
                    created_at: row.get::<_, String>(7)?.parse().unwrap(),
                    updated_at: row.get::<_, String>(8)?.parse().unwrap(),
                })
            })
            .optional()?;

        Ok(vehicle)
    }

    /// Get all vehicles from the database
    pub fn get_all_vehicles(&self) -> Result<Vec<Vehicle>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, license_plate, tank_size_liters, tp_consumption, initial_odometer, is_active, created_at, updated_at,
                    COALESCE(vehicle_type, 'Ice') as vehicle_type,
                    battery_capacity_kwh, baseline_consumption_kwh, initial_battery_percent
             FROM vehicles ORDER BY created_at DESC",
        )?;

        let vehicles = stmt
            .query_map([], |row| {
                let vehicle_type_str: String = row.get(9)?;
                Ok(Vehicle {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    name: row.get(1)?,
                    license_plate: row.get(2)?,
                    vehicle_type: parse_vehicle_type(&vehicle_type_str),
                    tank_size_liters: row.get(3)?,
                    tp_consumption: row.get(4)?,
                    battery_capacity_kwh: row.get(10)?,
                    baseline_consumption_kwh: row.get(11)?,
                    initial_battery_percent: row.get(12)?,
                    initial_odometer: row.get(5)?,
                    is_active: row.get(6)?,
                    created_at: row.get::<_, String>(7)?.parse().unwrap(),
                    updated_at: row.get::<_, String>(8)?.parse().unwrap(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(vehicles)
    }

    /// Get the currently active vehicle
    pub fn get_active_vehicle(&self) -> Result<Option<Vehicle>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, license_plate, tank_size_liters, tp_consumption, initial_odometer, is_active, created_at, updated_at,
                    COALESCE(vehicle_type, 'Ice') as vehicle_type,
                    battery_capacity_kwh, baseline_consumption_kwh, initial_battery_percent
             FROM vehicles WHERE is_active = 1 LIMIT 1",
        )?;

        let vehicle = stmt
            .query_row([], |row| {
                let vehicle_type_str: String = row.get(9)?;
                Ok(Vehicle {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    name: row.get(1)?,
                    license_plate: row.get(2)?,
                    vehicle_type: parse_vehicle_type(&vehicle_type_str),
                    tank_size_liters: row.get(3)?,
                    tp_consumption: row.get(4)?,
                    battery_capacity_kwh: row.get(10)?,
                    baseline_consumption_kwh: row.get(11)?,
                    initial_battery_percent: row.get(12)?,
                    initial_odometer: row.get(5)?,
                    is_active: row.get(6)?,
                    created_at: row.get::<_, String>(7)?.parse().unwrap(),
                    updated_at: row.get::<_, String>(8)?.parse().unwrap(),
                })
            })
            .optional()?;

        Ok(vehicle)
    }

    /// Update an existing vehicle
    pub fn update_vehicle(&self, vehicle: &Vehicle) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE vehicles
             SET name = :name,
                 license_plate = :license_plate,
                 tank_size_liters = :tank_size_liters,
                 tp_consumption = :tp_consumption,
                 initial_odometer = :initial_odometer,
                 is_active = :is_active,
                 updated_at = :updated_at
             WHERE id = :id",
            rusqlite::named_params! {
                ":id": vehicle.id.to_string(),
                ":name": vehicle.name,
                ":license_plate": vehicle.license_plate,
                ":tank_size_liters": vehicle.tank_size_liters,
                ":tp_consumption": vehicle.tp_consumption,
                ":initial_odometer": vehicle.initial_odometer,
                ":is_active": vehicle.is_active,
                ":updated_at": vehicle.updated_at.to_rfc3339(),
            },
        )?;
        Ok(())
    }

    /// Delete a vehicle by its UUID string
    pub fn delete_vehicle(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM vehicles WHERE id = ?1", [id])?;
        Ok(())
    }

    // Trip CRUD operations

    /// Create a new trip in the database
    pub fn create_trip(&self, trip: &Trip) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO trips (id, vehicle_id, date, origin, destination, distance_km, odometer, purpose, fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note, full_tank, sort_order, created_at, updated_at)
             VALUES (:id, :vehicle_id, :date, :origin, :destination, :distance_km, :odometer, :purpose, :fuel_liters, :fuel_cost_eur, :other_costs_eur, :other_costs_note, :full_tank, :sort_order, :created_at, :updated_at)",
            rusqlite::named_params! {
                ":id": trip.id.to_string(),
                ":vehicle_id": trip.vehicle_id.to_string(),
                ":date": trip.date.to_string(),
                ":origin": trip.origin,
                ":destination": trip.destination,
                ":distance_km": trip.distance_km,
                ":odometer": trip.odometer,
                ":purpose": trip.purpose,
                ":fuel_liters": trip.fuel_liters,
                ":fuel_cost_eur": trip.fuel_cost_eur,
                ":other_costs_eur": trip.other_costs_eur,
                ":other_costs_note": trip.other_costs_note,
                ":full_tank": trip.full_tank,
                ":sort_order": trip.sort_order,
                ":created_at": trip.created_at.to_rfc3339(),
                ":updated_at": trip.updated_at.to_rfc3339(),
            },
        )?;
        Ok(())
    }

    /// Get a trip by its UUID string
    pub fn get_trip(&self, id: &str) -> Result<Option<Trip>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, vehicle_id, date, origin, destination, distance_km, odometer, purpose, fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note, full_tank, sort_order, created_at, updated_at,
                    energy_kwh, energy_cost_eur, COALESCE(full_charge, 0) as full_charge, soc_override_percent
             FROM trips WHERE id = ?1",
        )?;

        let trip = stmt
            .query_row([id], |row| {
                Ok(Trip {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    vehicle_id: row.get::<_, String>(1)?.parse().unwrap(),
                    date: row.get::<_, String>(2)?.parse().unwrap(),
                    origin: row.get(3)?,
                    destination: row.get(4)?,
                    distance_km: row.get(5)?,
                    odometer: row.get(6)?,
                    purpose: row.get(7)?,
                    fuel_liters: row.get(8)?,
                    fuel_cost_eur: row.get(9)?,
                    full_tank: row.get(12)?,
                    energy_kwh: row.get(16)?,
                    energy_cost_eur: row.get(17)?,
                    full_charge: row.get(18)?,
                    soc_override_percent: row.get(19)?,
                    other_costs_eur: row.get(10)?,
                    other_costs_note: row.get(11)?,
                    sort_order: row.get(13)?,
                    created_at: row.get::<_, String>(14)?.parse().unwrap(),
                    updated_at: row.get::<_, String>(15)?.parse().unwrap(),
                })
            })
            .optional()?;

        Ok(trip)
    }

    /// Get all trips for a vehicle, ordered by sort_order ASC (0 = top/newest)
    pub fn get_trips_for_vehicle(&self, vehicle_id: &str) -> Result<Vec<Trip>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, vehicle_id, date, origin, destination, distance_km, odometer, purpose, fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note, full_tank, sort_order, created_at, updated_at,
                    energy_kwh, energy_cost_eur, COALESCE(full_charge, 0) as full_charge, soc_override_percent
             FROM trips WHERE vehicle_id = ?1 ORDER BY sort_order ASC",
        )?;

        let trips = stmt
            .query_map([vehicle_id], |row| {
                Ok(Trip {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    vehicle_id: row.get::<_, String>(1)?.parse().unwrap(),
                    date: row.get::<_, String>(2)?.parse().unwrap(),
                    origin: row.get(3)?,
                    destination: row.get(4)?,
                    distance_km: row.get(5)?,
                    odometer: row.get(6)?,
                    purpose: row.get(7)?,
                    fuel_liters: row.get(8)?,
                    fuel_cost_eur: row.get(9)?,
                    full_tank: row.get(12)?,
                    energy_kwh: row.get(16)?,
                    energy_cost_eur: row.get(17)?,
                    full_charge: row.get(18)?,
                    soc_override_percent: row.get(19)?,
                    other_costs_eur: row.get(10)?,
                    other_costs_note: row.get(11)?,
                    sort_order: row.get(13)?,
                    created_at: row.get::<_, String>(14)?.parse().unwrap(),
                    updated_at: row.get::<_, String>(15)?.parse().unwrap(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(trips)
    }

    /// Get all trips for a vehicle in a specific year, ordered by sort_order ASC
    pub fn get_trips_for_vehicle_in_year(&self, vehicle_id: &str, year: i32) -> Result<Vec<Trip>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, vehicle_id, date, origin, destination, distance_km, odometer, purpose, fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note, full_tank, sort_order, created_at, updated_at,
                    energy_kwh, energy_cost_eur, COALESCE(full_charge, 0) as full_charge, soc_override_percent
             FROM trips
             WHERE vehicle_id = ?1 AND strftime('%Y', date) = ?2
             ORDER BY sort_order ASC",
        )?;

        let trips = stmt
            .query_map([vehicle_id, &year.to_string()], |row| {
                Ok(Trip {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    vehicle_id: row.get::<_, String>(1)?.parse().unwrap(),
                    date: row.get::<_, String>(2)?.parse().unwrap(),
                    origin: row.get(3)?,
                    destination: row.get(4)?,
                    distance_km: row.get(5)?,
                    odometer: row.get(6)?,
                    purpose: row.get(7)?,
                    fuel_liters: row.get(8)?,
                    fuel_cost_eur: row.get(9)?,
                    full_tank: row.get(12)?,
                    energy_kwh: row.get(16)?,
                    energy_cost_eur: row.get(17)?,
                    full_charge: row.get(18)?,
                    soc_override_percent: row.get(19)?,
                    other_costs_eur: row.get(10)?,
                    other_costs_note: row.get(11)?,
                    sort_order: row.get(13)?,
                    created_at: row.get::<_, String>(14)?.parse().unwrap(),
                    updated_at: row.get::<_, String>(15)?.parse().unwrap(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(trips)
    }

    /// Get distinct years that have trips for a vehicle, ordered DESC (newest first)
    pub fn get_years_with_trips(&self, vehicle_id: &str) -> Result<Vec<i32>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT CAST(strftime('%Y', date) AS INTEGER) as year
             FROM trips WHERE vehicle_id = ?1 ORDER BY year DESC",
        )?;

        let years = stmt
            .query_map([vehicle_id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<i32>, _>>()?;

        Ok(years)
    }

    /// Update an existing trip
    pub fn update_trip(&self, trip: &Trip) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE trips
             SET vehicle_id = :vehicle_id,
                 date = :date,
                 origin = :origin,
                 destination = :destination,
                 distance_km = :distance_km,
                 odometer = :odometer,
                 purpose = :purpose,
                 fuel_liters = :fuel_liters,
                 fuel_cost_eur = :fuel_cost_eur,
                 other_costs_eur = :other_costs_eur,
                 other_costs_note = :other_costs_note,
                 full_tank = :full_tank,
                 sort_order = :sort_order,
                 updated_at = :updated_at
             WHERE id = :id",
            rusqlite::named_params! {
                ":id": trip.id.to_string(),
                ":vehicle_id": trip.vehicle_id.to_string(),
                ":date": trip.date.to_string(),
                ":origin": trip.origin,
                ":destination": trip.destination,
                ":distance_km": trip.distance_km,
                ":odometer": trip.odometer,
                ":purpose": trip.purpose,
                ":fuel_liters": trip.fuel_liters,
                ":fuel_cost_eur": trip.fuel_cost_eur,
                ":other_costs_eur": trip.other_costs_eur,
                ":other_costs_note": trip.other_costs_note,
                ":full_tank": trip.full_tank,
                ":sort_order": trip.sort_order,
                ":updated_at": trip.updated_at.to_rfc3339(),
            },
        )?;
        Ok(())
    }

    /// Delete a trip by its UUID string
    pub fn delete_trip(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM trips WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Reorder a trip to a new position, adjusting other trips' sort_order
    pub fn reorder_trip(
        &self,
        trip_id: &str,
        new_sort_order: i32,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Get current trip info
        let (vehicle_id, old_sort_order): (String, i32) = conn.query_row(
            "SELECT vehicle_id, sort_order FROM trips WHERE id = ?1",
            [trip_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        if old_sort_order < new_sort_order {
            // Moving down: decrement sort_order for trips between old and new position
            conn.execute(
                "UPDATE trips SET sort_order = sort_order - 1
                 WHERE vehicle_id = ?1 AND sort_order > ?2 AND sort_order <= ?3",
                rusqlite::params![vehicle_id, old_sort_order, new_sort_order],
            )?;
        } else if old_sort_order > new_sort_order {
            // Moving up: increment sort_order for trips between new and old position
            conn.execute(
                "UPDATE trips SET sort_order = sort_order + 1
                 WHERE vehicle_id = ?1 AND sort_order >= ?2 AND sort_order < ?3",
                rusqlite::params![vehicle_id, new_sort_order, old_sort_order],
            )?;
        }

        // Update the moved trip's sort_order only (keep date unchanged)
        conn.execute(
            "UPDATE trips SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![
                new_sort_order,
                chrono::Utc::now().to_rfc3339(),
                trip_id
            ],
        )?;

        Ok(())
    }

    /// Shift all trips at or after a position down by 1 (for insertion)
    pub fn shift_trips_from_position(&self, vehicle_id: &str, from_position: i32) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE trips SET sort_order = sort_order + 1
             WHERE vehicle_id = ?1 AND sort_order >= ?2",
            rusqlite::params![vehicle_id, from_position],
        )?;
        Ok(())
    }

    // Route CRUD operations

    /// Create a new route in the database
    pub fn create_route(&self, route: &Route) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO routes (id, vehicle_id, origin, destination, distance_km, usage_count, last_used)
             VALUES (:id, :vehicle_id, :origin, :destination, :distance_km, :usage_count, :last_used)",
            rusqlite::named_params! {
                ":id": route.id.to_string(),
                ":vehicle_id": route.vehicle_id.to_string(),
                ":origin": route.origin,
                ":destination": route.destination,
                ":distance_km": route.distance_km,
                ":usage_count": route.usage_count,
                ":last_used": route.last_used.to_rfc3339(),
            },
        )?;
        Ok(())
    }

    /// Get a route by its UUID string
    pub fn get_route(&self, id: &str) -> Result<Option<Route>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, vehicle_id, origin, destination, distance_km, usage_count, last_used
             FROM routes WHERE id = ?1",
        )?;

        let route = stmt
            .query_row([id], |row| {
                Ok(Route {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    vehicle_id: row.get::<_, String>(1)?.parse().unwrap(),
                    origin: row.get(2)?,
                    destination: row.get(3)?,
                    distance_km: row.get(4)?,
                    usage_count: row.get(5)?,
                    last_used: row.get::<_, String>(6)?.parse().unwrap(),
                })
            })
            .optional()?;

        Ok(route)
    }

    /// Get all routes for a vehicle, ordered by usage_count DESC (most used first for autocomplete)
    pub fn get_routes_for_vehicle(&self, vehicle_id: &str) -> Result<Vec<Route>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, vehicle_id, origin, destination, distance_km, usage_count, last_used
             FROM routes WHERE vehicle_id = ?1 ORDER BY usage_count DESC",
        )?;

        let routes = stmt
            .query_map([vehicle_id], |row| {
                Ok(Route {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    vehicle_id: row.get::<_, String>(1)?.parse().unwrap(),
                    origin: row.get(2)?,
                    destination: row.get(3)?,
                    distance_km: row.get(4)?,
                    usage_count: row.get(5)?,
                    last_used: row.get::<_, String>(6)?.parse().unwrap(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(routes)
    }

    /// Get all unique trip purposes for a vehicle (across all years)
    pub fn get_purposes_for_vehicle(&self, vehicle_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT TRIM(purpose) as purpose
             FROM trips
             WHERE vehicle_id = ?1 AND TRIM(purpose) != ''
             ORDER BY purpose",
        )?;

        let purposes = stmt
            .query_map([vehicle_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(purposes)
    }

    /// Update an existing route (e.g., increment usage_count)
    pub fn update_route(&self, route: &Route) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE routes
             SET vehicle_id = :vehicle_id,
                 origin = :origin,
                 destination = :destination,
                 distance_km = :distance_km,
                 usage_count = :usage_count,
                 last_used = :last_used
             WHERE id = :id",
            rusqlite::named_params! {
                ":id": route.id.to_string(),
                ":vehicle_id": route.vehicle_id.to_string(),
                ":origin": route.origin,
                ":destination": route.destination,
                ":distance_km": route.distance_km,
                ":usage_count": route.usage_count,
                ":last_used": route.last_used.to_rfc3339(),
            },
        )?;
        Ok(())
    }

    /// Delete a route by its UUID string
    pub fn delete_route(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM routes WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Find existing route with same origin/destination, or create new one
    /// If found: increment usage_count, update last_used
    /// If not found: create new route with usage_count=1
    pub fn find_or_create_route(
        &self,
        vehicle_id: &str,
        origin: &str,
        destination: &str,
        distance_km: f64,
    ) -> Result<Route> {
        let conn = self.conn.lock().unwrap();

        // Try to find existing route
        let mut stmt = conn.prepare(
            "SELECT id, vehicle_id, origin, destination, distance_km, usage_count, last_used
             FROM routes
             WHERE vehicle_id = ?1 AND origin = ?2 AND destination = ?3",
        )?;

        let existing_route = stmt
            .query_row([vehicle_id, origin, destination], |row| {
                Ok(Route {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    vehicle_id: row.get::<_, String>(1)?.parse().unwrap(),
                    origin: row.get(2)?,
                    destination: row.get(3)?,
                    distance_km: row.get(4)?,
                    usage_count: row.get(5)?,
                    last_used: row.get::<_, String>(6)?.parse().unwrap(),
                })
            })
            .optional()?;

        if let Some(mut route) = existing_route {
            // Update existing route: increment usage_count and update last_used
            route.usage_count += 1;
            route.last_used = chrono::Utc::now();

            conn.execute(
                "UPDATE routes
                 SET usage_count = :usage_count,
                     last_used = :last_used
                 WHERE id = :id",
                rusqlite::named_params! {
                    ":id": route.id.to_string(),
                    ":usage_count": route.usage_count,
                    ":last_used": route.last_used.to_rfc3339(),
                },
            )?;

            Ok(route)
        } else {
            // Create new route
            let route = Route {
                id: uuid::Uuid::new_v4(),
                vehicle_id: vehicle_id.parse().unwrap(),
                origin: origin.to_string(),
                destination: destination.to_string(),
                distance_km,
                usage_count: 1,
                last_used: chrono::Utc::now(),
            };

            conn.execute(
                "INSERT INTO routes (id, vehicle_id, origin, destination, distance_km, usage_count, last_used)
                 VALUES (:id, :vehicle_id, :origin, :destination, :distance_km, :usage_count, :last_used)",
                rusqlite::named_params! {
                    ":id": route.id.to_string(),
                    ":vehicle_id": route.vehicle_id.to_string(),
                    ":origin": route.origin,
                    ":destination": route.destination,
                    ":distance_km": route.distance_km,
                    ":usage_count": route.usage_count,
                    ":last_used": route.last_used.to_rfc3339(),
                },
            )?;

            Ok(route)
        }
    }

    // Settings CRUD operations

    /// Get settings from database (returns None if not found)
    pub fn get_settings(&self) -> Result<Option<Settings>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, company_name, company_ico, buffer_trip_purpose, updated_at
             FROM settings LIMIT 1",
        )?;

        let settings = stmt
            .query_row([], |row| {
                Ok(Settings {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    company_name: row.get(1)?,
                    company_ico: row.get(2)?,
                    buffer_trip_purpose: row.get(3)?,
                    updated_at: row.get::<_, String>(4)?.parse().unwrap(),
                })
            })
            .optional()?;

        Ok(settings)
    }

    /// Save settings (upsert - insert or update)
    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Check if settings exist
        let exists: bool = conn
            .query_row("SELECT COUNT(*) > 0 FROM settings", [], |row| row.get(0))
            .unwrap_or(false);

        if exists {
            // Update existing settings
            conn.execute(
                "UPDATE settings
                 SET company_name = :company_name,
                     company_ico = :company_ico,
                     buffer_trip_purpose = :buffer_trip_purpose,
                     updated_at = :updated_at",
                rusqlite::named_params! {
                    ":company_name": settings.company_name,
                    ":company_ico": settings.company_ico,
                    ":buffer_trip_purpose": settings.buffer_trip_purpose,
                    ":updated_at": settings.updated_at.to_rfc3339(),
                },
            )?;
        } else {
            // Insert new settings
            conn.execute(
                "INSERT INTO settings (id, company_name, company_ico, buffer_trip_purpose, updated_at)
                 VALUES (:id, :company_name, :company_ico, :buffer_trip_purpose, :updated_at)",
                rusqlite::named_params! {
                    ":id": settings.id.to_string(),
                    ":company_name": settings.company_name,
                    ":company_ico": settings.company_ico,
                    ":buffer_trip_purpose": settings.buffer_trip_purpose,
                    ":updated_at": settings.updated_at.to_rfc3339(),
                },
            )?;
        }

        Ok(())
    }

    // ========================================================================
    // Receipt Operations
    // ========================================================================

    /// Helper to avoid row-to-struct duplication
    fn row_to_receipt(row: &rusqlite::Row) -> rusqlite::Result<Receipt> {
        // Column indices:
        // 0: id, 1: vehicle_id, 2: trip_id, 3: file_path, 4: file_name, 5: scanned_at
        // 6: liters, 7: total_price_eur, 8: receipt_date, 9: station_name, 10: station_address
        // 11: source_year, 12: status, 13: confidence, 14: raw_ocr_text, 15: error_message
        // 16: created_at, 17: updated_at
        let status_str: String = row.get(12)?;
        let status = match status_str.as_str() {
            "Pending" => ReceiptStatus::Pending,
            "Parsed" => ReceiptStatus::Parsed,
            "NeedsReview" => ReceiptStatus::NeedsReview,
            "Assigned" => ReceiptStatus::Assigned,
            _ => ReceiptStatus::Pending,
        };

        let confidence_str: String = row.get(13)?;
        let confidence: FieldConfidence = serde_json::from_str(&confidence_str).unwrap_or_default();

        Ok(Receipt {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            vehicle_id: row.get::<_, Option<String>>(1)?.map(|s| Uuid::parse_str(&s).unwrap()),
            trip_id: row.get::<_, Option<String>>(2)?.map(|s| Uuid::parse_str(&s).unwrap()),
            file_path: row.get(3)?,
            file_name: row.get(4)?,
            scanned_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                .unwrap()
                .with_timezone(&Utc),
            liters: row.get(6)?,
            total_price_eur: row.get(7)?,
            receipt_date: row.get::<_, Option<String>>(8)?
                .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").unwrap()),
            station_name: row.get(9)?,
            station_address: row.get(10)?,
            source_year: row.get(11)?,
            status,
            confidence,
            raw_ocr_text: row.get(14)?,
            error_message: row.get(15)?,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(16)?)
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(17)?)
                .unwrap()
                .with_timezone(&Utc),
        })
    }

    const RECEIPT_SELECT_COLS: &'static str =
        "id, vehicle_id, trip_id, file_path, file_name, scanned_at,
         liters, total_price_eur, receipt_date, station_name, station_address,
         source_year, status, confidence, raw_ocr_text, error_message, created_at, updated_at";

    pub fn create_receipt(&self, receipt: &Receipt) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let status_str = match receipt.status {
            ReceiptStatus::Pending => "Pending",
            ReceiptStatus::Parsed => "Parsed",
            ReceiptStatus::NeedsReview => "NeedsReview",
            ReceiptStatus::Assigned => "Assigned",
        };
        conn.execute(
            "INSERT INTO receipts (id, vehicle_id, trip_id, file_path, file_name, scanned_at,
                liters, total_price_eur, receipt_date, station_name, station_address,
                source_year, status, confidence, raw_ocr_text, error_message, created_at, updated_at)
             VALUES (:id, :vehicle_id, :trip_id, :file_path, :file_name, :scanned_at,
                :liters, :total_price_eur, :receipt_date, :station_name, :station_address,
                :source_year, :status, :confidence, :raw_ocr_text, :error_message, :created_at, :updated_at)",
            rusqlite::named_params! {
                ":id": receipt.id.to_string(),
                ":vehicle_id": receipt.vehicle_id.map(|id| id.to_string()),
                ":trip_id": receipt.trip_id.map(|id| id.to_string()),
                ":file_path": &receipt.file_path,
                ":file_name": &receipt.file_name,
                ":scanned_at": receipt.scanned_at.to_rfc3339(),
                ":liters": receipt.liters,
                ":total_price_eur": receipt.total_price_eur,
                ":receipt_date": receipt.receipt_date.map(|d| d.to_string()),
                ":station_name": &receipt.station_name,
                ":station_address": &receipt.station_address,
                ":source_year": receipt.source_year,
                ":status": status_str,
                ":confidence": serde_json::to_string(&receipt.confidence).unwrap(),
                ":raw_ocr_text": &receipt.raw_ocr_text,
                ":error_message": &receipt.error_message,
                ":created_at": receipt.created_at.to_rfc3339(),
                ":updated_at": receipt.updated_at.to_rfc3339(),
            },
        )?;
        Ok(())
    }

    pub fn get_all_receipts(&self) -> Result<Vec<Receipt>> {
        let conn = self.conn.lock().unwrap();
        let sql = format!("SELECT {} FROM receipts ORDER BY scanned_at DESC", Self::RECEIPT_SELECT_COLS);
        let mut stmt = conn.prepare(&sql)?;
        let receipts = stmt.query_map([], Self::row_to_receipt)?.collect::<Result<Vec<_>, _>>()?;
        Ok(receipts)
    }

    pub fn get_unassigned_receipts(&self) -> Result<Vec<Receipt>> {
        let conn = self.conn.lock().unwrap();
        let sql = format!(
            "SELECT {} FROM receipts WHERE trip_id IS NULL ORDER BY receipt_date DESC, scanned_at DESC",
            Self::RECEIPT_SELECT_COLS
        );
        let mut stmt = conn.prepare(&sql)?;
        let receipts = stmt.query_map([], Self::row_to_receipt)?.collect::<Result<Vec<_>, _>>()?;
        Ok(receipts)
    }

    pub fn get_pending_receipts(&self) -> Result<Vec<Receipt>> {
        let conn = self.conn.lock().unwrap();
        let sql = format!(
            "SELECT {} FROM receipts WHERE status = 'Pending' ORDER BY scanned_at ASC",
            Self::RECEIPT_SELECT_COLS
        );
        let mut stmt = conn.prepare(&sql)?;
        let receipts = stmt.query_map([], Self::row_to_receipt)?.collect::<Result<Vec<_>, _>>()?;
        Ok(receipts)
    }

    pub fn update_receipt(&self, receipt: &Receipt) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let status_str = match receipt.status {
            ReceiptStatus::Pending => "Pending",
            ReceiptStatus::Parsed => "Parsed",
            ReceiptStatus::NeedsReview => "NeedsReview",
            ReceiptStatus::Assigned => "Assigned",
        };
        conn.execute(
            "UPDATE receipts SET
                vehicle_id = :vehicle_id, trip_id = :trip_id, liters = :liters, total_price_eur = :total_price_eur,
                receipt_date = :receipt_date, station_name = :station_name, station_address = :station_address,
                source_year = :source_year, status = :status, confidence = :confidence, raw_ocr_text = :raw_ocr_text,
                error_message = :error_message, updated_at = :updated_at
             WHERE id = :id",
            rusqlite::named_params! {
                ":id": receipt.id.to_string(),
                ":vehicle_id": receipt.vehicle_id.map(|id| id.to_string()),
                ":trip_id": receipt.trip_id.map(|id| id.to_string()),
                ":liters": receipt.liters,
                ":total_price_eur": receipt.total_price_eur,
                ":receipt_date": receipt.receipt_date.map(|d| d.to_string()),
                ":station_name": &receipt.station_name,
                ":station_address": &receipt.station_address,
                ":source_year": receipt.source_year,
                ":status": status_str,
                ":confidence": serde_json::to_string(&receipt.confidence).unwrap(),
                ":raw_ocr_text": &receipt.raw_ocr_text,
                ":error_message": &receipt.error_message,
                ":updated_at": Utc::now().to_rfc3339(),
            },
        )?;
        Ok(())
    }

    pub fn delete_receipt(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM receipts WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn get_receipt_by_file_path(&self, file_path: &str) -> Result<Option<Receipt>> {
        let conn = self.conn.lock().unwrap();
        let sql = format!("SELECT {} FROM receipts WHERE file_path = ?1", Self::RECEIPT_SELECT_COLS);
        let mut stmt = conn.prepare(&sql)?;
        stmt.query_row([file_path], Self::row_to_receipt).optional()
    }

    pub fn get_receipt_by_id(&self, id: &str) -> Result<Option<Receipt>> {
        let conn = self.conn.lock().unwrap();
        let sql = format!("SELECT {} FROM receipts WHERE id = ?1", Self::RECEIPT_SELECT_COLS);
        let mut stmt = conn.prepare(&sql)?;
        stmt.query_row([id], Self::row_to_receipt).optional()
    }

    /// Get receipts filtered by year.
    /// Filtering logic:
    /// - If receipt_date is set: include if receipt_date.year() == year
    /// - If receipt_date is None but source_year is set: include if source_year == year
    /// - If both are None: include in ALL years (flat mode, unprocessed receipts)
    pub fn get_receipts_for_year(&self, year: i32) -> Result<Vec<Receipt>> {
        let conn = self.conn.lock().unwrap();
        // SQL logic:
        // 1. receipt_date exists and year matches -> include
        // 2. receipt_date is NULL, source_year exists and matches -> include
        // 3. both NULL -> include (flat mode)
        let sql = format!(
            "SELECT {} FROM receipts WHERE
                (receipt_date IS NOT NULL AND CAST(strftime('%Y', receipt_date) AS INTEGER) = ?1)
                OR (receipt_date IS NULL AND source_year = ?1)
                OR (receipt_date IS NULL AND source_year IS NULL)
             ORDER BY receipt_date DESC, scanned_at DESC",
            Self::RECEIPT_SELECT_COLS
        );
        let mut stmt = conn.prepare(&sql)?;
        let receipts = stmt
            .query_map([year], Self::row_to_receipt)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(receipts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = Database::in_memory().expect("Failed to create database");
        let conn = db.connection();

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"vehicles".to_string()));
        assert!(tables.contains(&"trips".to_string()));
        assert!(tables.contains(&"routes".to_string()));
        assert!(tables.contains(&"settings".to_string()));
    }

    // Helper to create test vehicles
    fn create_test_vehicle(name: &str) -> Vehicle {
        Vehicle::new(name.to_string(), "BA123XY".to_string(), 66.0, 5.1, 0.0)
    }

    /// Consolidated vehicle CRUD test - covers create, retrieve, update, delete, and queries
    #[test]
    fn test_vehicle_crud_lifecycle() {
        let db = Database::in_memory().expect("Failed to create database");

        // CREATE + RETRIEVE
        let vehicle = create_test_vehicle("Test Car");
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        let retrieved = db
            .get_vehicle(&vehicle.id.to_string())
            .expect("Failed to get vehicle")
            .expect("Vehicle not found");
        assert_eq!(retrieved.name, "Test Car");
        assert!(retrieved.is_active);

        // GET ALL + ACTIVE VEHICLE
        let mut v2 = create_test_vehicle("Inactive Car");
        v2.is_active = false;
        db.create_vehicle(&v2).expect("Failed to create v2");

        let all = db.get_all_vehicles().expect("Failed to get all");
        assert_eq!(all.len(), 2);

        let active = db.get_active_vehicle().expect("Failed to get active").unwrap();
        assert_eq!(active.id, vehicle.id); // First vehicle is active

        // UPDATE
        let mut updated = retrieved;
        updated.name = "Updated Name".to_string();
        updated.tp_consumption = Some(6.5);
        db.update_vehicle(&updated).expect("Failed to update");

        let after_update = db.get_vehicle(&vehicle.id.to_string()).unwrap().unwrap();
        assert_eq!(after_update.name, "Updated Name");
        assert_eq!(after_update.tp_consumption, Some(6.5));

        // DELETE
        db.delete_vehicle(&vehicle.id.to_string()).expect("Failed to delete");
        assert!(db.get_vehicle(&vehicle.id.to_string()).unwrap().is_none());
    }

    // Helper to create test trips
    fn create_test_trip(vehicle_id: uuid::Uuid, date: &str) -> Trip {
        use chrono::NaiveDate;
        let now = chrono::Utc::now();
        Trip {
            id: uuid::Uuid::new_v4(),
            vehicle_id,
            date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
            origin: "Prague".to_string(),
            destination: "Brno".to_string(),
            distance_km: 200.0,
            odometer: 50000.0,
            purpose: "Business meeting".to_string(),
            fuel_liters: Some(15.0),
            fuel_cost_eur: Some(25.5),
            full_tank: true,
            energy_kwh: None,
            energy_cost_eur: None,
            full_charge: false,
            soc_override_percent: None,
            other_costs_eur: Some(5.0),
            other_costs_note: Some("Parking fee".to_string()),
            sort_order: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Consolidated trip CRUD test - covers create, retrieve, update, delete, ordering, and optional fields
    #[test]
    fn test_trip_crud_lifecycle() {
        let db = Database::in_memory().expect("Failed to create database");
        let vehicle = create_test_vehicle("Test Car");
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        // CREATE + RETRIEVE (with all fields)
        let trip = create_test_trip(vehicle.id, "2024-12-01");
        db.create_trip(&trip).expect("Failed to create trip");

        let retrieved = db.get_trip(&trip.id.to_string()).unwrap().unwrap();
        assert_eq!(retrieved.origin, "Prague");
        assert_eq!(retrieved.fuel_liters, Some(15.0));
        assert_eq!(retrieved.other_costs_note, Some("Parking fee".to_string()));

        // CREATE with None optional fields (tests NULL handling)
        let now = chrono::Utc::now();
        let trip_no_fuel = Trip {
            id: uuid::Uuid::new_v4(),
            vehicle_id: vehicle.id,
            date: chrono::NaiveDate::from_ymd_opt(2024, 12, 2).unwrap(),
            origin: "Vienna".to_string(),
            destination: "Graz".to_string(),
            distance_km: 150.0,
            odometer: 50200.0,
            purpose: "Personal".to_string(),
            fuel_liters: None,
            fuel_cost_eur: None,
            full_tank: false,
            energy_kwh: None,
            energy_cost_eur: None,
            full_charge: false,
            soc_override_percent: None,
            other_costs_eur: None,
            other_costs_note: None,
            sort_order: 1,
            created_at: now,
            updated_at: now,
        };
        db.create_trip(&trip_no_fuel).expect("Failed to create trip without fuel");
        let retrieved_no_fuel = db.get_trip(&trip_no_fuel.id.to_string()).unwrap().unwrap();
        assert_eq!(retrieved_no_fuel.fuel_liters, None);

        // LIST with sort_order
        let trips = db.get_trips_for_vehicle(&vehicle.id.to_string()).unwrap();
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[0].sort_order, 0); // First by sort_order ASC
        assert_eq!(trips[1].sort_order, 1);

        // UPDATE
        let mut updated = retrieved;
        updated.origin = "Berlin".to_string();
        updated.fuel_liters = Some(25.0);
        db.update_trip(&updated).expect("Failed to update");

        let after_update = db.get_trip(&trip.id.to_string()).unwrap().unwrap();
        assert_eq!(after_update.origin, "Berlin");
        assert_eq!(after_update.fuel_liters, Some(25.0));

        // DELETE
        db.delete_trip(&trip.id.to_string()).expect("Failed to delete");
        assert!(db.get_trip(&trip.id.to_string()).unwrap().is_none());
    }

    /// Tests year-based filtering (non-trivial date logic)
    #[test]
    fn test_get_trips_for_vehicle_in_year() {
        let db = Database::in_memory().expect("Failed to create database");
        let vehicle = Vehicle::new("Test Car".to_string(), "BA123XY".to_string(), 66.0, 5.1, 0.0);
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        // Create trips in different years
        let trip1 = create_test_trip(vehicle.id, "2024-12-01");
        let trip2 = create_test_trip(vehicle.id, "2024-06-15");
        let trip3 = create_test_trip(vehicle.id, "2023-12-10");
        let trip4 = create_test_trip(vehicle.id, "2024-01-05");

        db.create_trip(&trip1).expect("Failed to create trip1");
        db.create_trip(&trip2).expect("Failed to create trip2");
        db.create_trip(&trip3).expect("Failed to create trip3");
        db.create_trip(&trip4).expect("Failed to create trip4");

        // Get trips for 2024
        let trips_2024 = db
            .get_trips_for_vehicle_in_year(&vehicle.id.to_string(), 2024)
            .expect("Failed to get trips for 2024");

        assert_eq!(trips_2024.len(), 3);

        // Verify ordering: ASC (chronological for export)
        assert_eq!(trips_2024[0].date.to_string(), "2024-01-05");
        assert_eq!(trips_2024[1].date.to_string(), "2024-06-15");
        assert_eq!(trips_2024[2].date.to_string(), "2024-12-01");

        // Get trips for 2023
        let trips_2023 = db
            .get_trips_for_vehicle_in_year(&vehicle.id.to_string(), 2023)
            .expect("Failed to get trips for 2023");

        assert_eq!(trips_2023.len(), 1);
        assert_eq!(trips_2023[0].date.to_string(), "2023-12-10");
    }

    // Route tests

    fn create_test_route(vehicle_id: uuid::Uuid, origin: &str, destination: &str, distance_km: f64) -> Route {
        Route {
            id: uuid::Uuid::new_v4(),
            vehicle_id,
            origin: origin.to_string(),
            destination: destination.to_string(),
            distance_km,
            usage_count: 1,
            last_used: chrono::Utc::now(),
        }
    }

    /// Consolidated route CRUD test - covers create, retrieve, update, delete, and usage ordering
    #[test]
    fn test_route_crud_lifecycle() {
        let db = Database::in_memory().expect("Failed to create database");
        let vehicle = create_test_vehicle("Test Car");
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        // CREATE + RETRIEVE
        let route = create_test_route(vehicle.id, "Bratislava", "Košice", 400.0);
        db.create_route(&route).expect("Failed to create route");

        let retrieved = db.get_route(&route.id.to_string()).unwrap().unwrap();
        assert_eq!(retrieved.origin, "Bratislava");
        assert_eq!(retrieved.usage_count, 1);

        // CREATE more routes with different usage counts for ordering test
        let mut route2 = create_test_route(vehicle.id, "A", "B", 50.0);
        route2.usage_count = 10;
        db.create_route(&route2).expect("Failed to create route2");

        // LIST - ordered by usage_count DESC
        let routes = db.get_routes_for_vehicle(&vehicle.id.to_string()).unwrap();
        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].usage_count, 10); // Most used first
        assert_eq!(routes[1].usage_count, 1);

        // UPDATE
        let mut updated = retrieved;
        updated.usage_count = 5;
        updated.distance_km = 405.0;
        db.update_route(&updated).expect("Failed to update");

        let after_update = db.get_route(&route.id.to_string()).unwrap().unwrap();
        assert_eq!(after_update.usage_count, 5);
        assert_eq!(after_update.distance_km, 405.0);

        // DELETE
        db.delete_route(&route.id.to_string()).expect("Failed to delete");
        assert!(db.get_route(&route.id.to_string()).unwrap().is_none());
    }

    /// Tests find_or_create upsert logic - non-trivial business behavior
    #[test]
    fn test_find_or_create_route_upsert() {
        let db = Database::in_memory().expect("Failed to create database");
        let vehicle = create_test_vehicle("Test Car");
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        // First call: creates new route
        let route1 = db
            .find_or_create_route(&vehicle.id.to_string(), "Budapest", "Prague", 500.0)
            .expect("Failed to create route");
        assert_eq!(route1.usage_count, 1);

        // Second call: finds existing, increments usage
        let route2 = db
            .find_or_create_route(&vehicle.id.to_string(), "Budapest", "Prague", 500.0)
            .expect("Failed to find route");
        assert_eq!(route2.id, route1.id); // Same route
        assert_eq!(route2.usage_count, 2); // Incremented

        // Verify persistence
        let retrieved = db.get_route(&route2.id.to_string()).unwrap().unwrap();
        assert_eq!(retrieved.usage_count, 2);
    }

    // Receipt tests

    #[test]
    fn test_receipt_crud() {
        let db = Database::in_memory().unwrap();

        // Create receipt
        let receipt = Receipt::new(
            "C:\\test\\receipt.jpg".to_string(),
            "receipt.jpg".to_string(),
        );
        db.create_receipt(&receipt).unwrap();

        // Get all receipts
        let receipts = db.get_all_receipts().unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].file_name, "receipt.jpg");
        assert_eq!(receipts[0].status, ReceiptStatus::Pending);

        // Get by file path
        let found = db.get_receipt_by_file_path("C:\\test\\receipt.jpg").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, receipt.id);

        // Update receipt
        let mut updated = receipt.clone();
        updated.liters = Some(45.5);
        updated.total_price_eur = Some(72.50);
        updated.status = ReceiptStatus::Parsed;
        db.update_receipt(&updated).unwrap();

        let receipts = db.get_all_receipts().unwrap();
        assert_eq!(receipts[0].liters, Some(45.5));
        assert_eq!(receipts[0].status, ReceiptStatus::Parsed);

        // Delete receipt
        db.delete_receipt(&receipt.id.to_string()).unwrap();
        let receipts = db.get_all_receipts().unwrap();
        assert_eq!(receipts.len(), 0);
    }

    #[test]
    fn test_get_unassigned_receipts() {
        let db = Database::in_memory().unwrap();

        // First create a vehicle and trip to satisfy foreign key
        let vehicle = Vehicle::new("Test Car".to_string(), "BA123XY".to_string(), 66.0, 5.1, 0.0);
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        let trip = create_test_trip(vehicle.id, "2024-12-01");
        db.create_trip(&trip).expect("Failed to create trip");

        // Create two receipts
        let receipt1 = Receipt::new("path1.jpg".to_string(), "1.jpg".to_string());
        let mut receipt2 = Receipt::new("path2.jpg".to_string(), "2.jpg".to_string());

        // Assign one to the real trip
        receipt2.trip_id = Some(trip.id);
        receipt2.vehicle_id = Some(vehicle.id);
        receipt2.status = ReceiptStatus::Assigned;

        db.create_receipt(&receipt1).unwrap();
        db.create_receipt(&receipt2).unwrap();

        // Only unassigned should be returned
        let unassigned = db.get_unassigned_receipts().unwrap();
        assert_eq!(unassigned.len(), 1);
        assert_eq!(unassigned[0].file_name, "1.jpg");
    }

    #[test]
    fn test_get_pending_receipts() {
        let db = Database::in_memory().unwrap();

        // Create receipts with different statuses
        let receipt1 = Receipt::new("path1.jpg".to_string(), "pending1.jpg".to_string());
        let mut receipt2 = Receipt::new("path2.jpg".to_string(), "pending2.jpg".to_string());
        let mut receipt3 = Receipt::new("path3.jpg".to_string(), "parsed.jpg".to_string());
        let mut receipt4 = Receipt::new("path4.jpg".to_string(), "needs_review.jpg".to_string());

        // receipt1 stays Pending (default)
        // receipt2 stays Pending
        receipt3.status = ReceiptStatus::Parsed;
        receipt4.status = ReceiptStatus::NeedsReview;

        db.create_receipt(&receipt1).unwrap();
        db.create_receipt(&receipt2).unwrap();
        db.create_receipt(&receipt3).unwrap();
        db.create_receipt(&receipt4).unwrap();

        // Only pending receipts should be returned
        let pending = db.get_pending_receipts().unwrap();
        assert_eq!(pending.len(), 2);

        // Should be ordered by scanned_at ASC
        let file_names: Vec<&str> = pending.iter().map(|r| r.file_name.as_str()).collect();
        assert!(file_names.contains(&"pending1.jpg"));
        assert!(file_names.contains(&"pending2.jpg"));
    }

    #[test]
    fn test_get_pending_receipts_empty() {
        let db = Database::in_memory().unwrap();

        // Create only non-pending receipts
        let mut receipt = Receipt::new("path.jpg".to_string(), "parsed.jpg".to_string());
        receipt.status = ReceiptStatus::Parsed;
        db.create_receipt(&receipt).unwrap();

        let pending = db.get_pending_receipts().unwrap();
        assert_eq!(pending.len(), 0);
    }

    /// Test Receipt creation with source_year (year folder support)
    #[test]
    fn test_receipt_with_source_year() {
        // Test creation with source_year
        let receipt = Receipt::new_with_source_year(
            "C:\\2024\\receipt.jpg".to_string(),
            "receipt.jpg".to_string(),
            Some(2024),
        );
        assert_eq!(receipt.source_year, Some(2024));
        assert_eq!(receipt.file_name, "receipt.jpg");

        // Test creation without source_year (flat folder)
        let receipt_flat = Receipt::new_with_source_year(
            "C:\\receipts\\receipt.jpg".to_string(),
            "receipt.jpg".to_string(),
            None,
        );
        assert_eq!(receipt_flat.source_year, None);

        // Original constructor should default to None
        let receipt_default = Receipt::new(
            "C:\\receipts\\receipt.jpg".to_string(),
            "receipt.jpg".to_string(),
        );
        assert_eq!(receipt_default.source_year, None);
    }

    /// Test DB round-trip with source_year field
    #[test]
    fn test_receipt_db_roundtrip_with_source_year() {
        let db = Database::in_memory().unwrap();

        // Create receipt with source_year
        let receipt = Receipt::new_with_source_year(
            "C:\\2024\\receipt1.jpg".to_string(),
            "receipt1.jpg".to_string(),
            Some(2024),
        );
        db.create_receipt(&receipt).unwrap();

        // Retrieve and verify source_year is preserved
        let retrieved = db.get_receipt_by_id(&receipt.id.to_string()).unwrap().unwrap();
        assert_eq!(retrieved.source_year, Some(2024));
        assert_eq!(retrieved.file_name, "receipt1.jpg");

        // Create receipt without source_year
        let receipt_flat = Receipt::new_with_source_year(
            "C:\\flat\\receipt2.jpg".to_string(),
            "receipt2.jpg".to_string(),
            None,
        );
        db.create_receipt(&receipt_flat).unwrap();

        // Retrieve and verify source_year is None
        let retrieved_flat = db.get_receipt_by_id(&receipt_flat.id.to_string()).unwrap().unwrap();
        assert_eq!(retrieved_flat.source_year, None);

        // Update receipt and verify source_year persists
        let mut updated = retrieved;
        updated.liters = Some(45.0);
        db.update_receipt(&updated).unwrap();

        let after_update = db.get_receipt_by_id(&receipt.id.to_string()).unwrap().unwrap();
        assert_eq!(after_update.source_year, Some(2024));
        assert_eq!(after_update.liters, Some(45.0));
    }

    // ========================================================================
    // Year filtering tests for get_receipts_for_year
    // ========================================================================

    /// Helper to create a receipt with specific date and source_year
    fn create_receipt_for_year_test(
        file_path: &str,
        receipt_date: Option<NaiveDate>,
        source_year: Option<i32>,
    ) -> Receipt {
        let mut receipt = Receipt::new_with_source_year(
            file_path.to_string(),
            file_path.to_string(),
            source_year,
        );
        receipt.receipt_date = receipt_date;
        receipt
    }

    /// Test: Receipt with receipt_date in 2024 should be included for year=2024
    #[test]
    fn test_get_receipts_for_year_filters_by_receipt_date() {
        let db = Database::in_memory().unwrap();

        // Receipt dated 2024-05-01
        let receipt = create_receipt_for_year_test(
            "r1.jpg",
            Some(NaiveDate::from_ymd_opt(2024, 5, 1).unwrap()),
            Some(2024),
        );
        db.create_receipt(&receipt).unwrap();

        // Receipt dated 2023-12-31 (should NOT be included for 2024)
        let receipt2 = create_receipt_for_year_test(
            "r2.jpg",
            Some(NaiveDate::from_ymd_opt(2023, 12, 31).unwrap()),
            Some(2024), // source_year is 2024, but date is 2023
        );
        db.create_receipt(&receipt2).unwrap();

        let receipts_2024 = db.get_receipts_for_year(2024).unwrap();
        assert_eq!(receipts_2024.len(), 1);
        assert_eq!(receipts_2024[0].file_name, "r1.jpg");
    }

    /// Test: Receipt with receipt_date takes precedence over source_year
    #[test]
    fn test_get_receipts_for_year_date_overrides_source_year() {
        let db = Database::in_memory().unwrap();

        // Receipt dated 2024 but from 2025 folder - should be in 2024 (date wins)
        let receipt = create_receipt_for_year_test(
            "r1.jpg",
            Some(NaiveDate::from_ymd_opt(2024, 5, 1).unwrap()),
            Some(2025), // From 2025 folder, but date is 2024
        );
        db.create_receipt(&receipt).unwrap();

        let receipts_2024 = db.get_receipts_for_year(2024).unwrap();
        assert_eq!(receipts_2024.len(), 1);

        let receipts_2025 = db.get_receipts_for_year(2025).unwrap();
        assert_eq!(receipts_2025.len(), 0);
    }

    /// Test: Receipt with no receipt_date falls back to source_year
    #[test]
    fn test_get_receipts_for_year_fallback_to_source_year() {
        let db = Database::in_memory().unwrap();

        // No date, source_year = 2024
        let receipt = create_receipt_for_year_test(
            "r1.jpg",
            None, // No receipt_date
            Some(2024),
        );
        db.create_receipt(&receipt).unwrap();

        // No date, source_year = 2025 (should NOT be in 2024)
        let receipt2 = create_receipt_for_year_test(
            "r2.jpg",
            None,
            Some(2025),
        );
        db.create_receipt(&receipt2).unwrap();

        let receipts_2024 = db.get_receipts_for_year(2024).unwrap();
        assert_eq!(receipts_2024.len(), 1);
        assert_eq!(receipts_2024[0].file_name, "r1.jpg");
    }

    /// Test: Receipt with neither receipt_date nor source_year shows in ALL years (flat mode)
    #[test]
    fn test_get_receipts_for_year_flat_mode_shows_everywhere() {
        let db = Database::in_memory().unwrap();

        // Flat mode receipt: no date, no source_year
        let receipt = create_receipt_for_year_test(
            "flat.jpg",
            None,
            None,
        );
        db.create_receipt(&receipt).unwrap();

        // Should appear in any year query
        let receipts_2024 = db.get_receipts_for_year(2024).unwrap();
        assert_eq!(receipts_2024.len(), 1);
        assert_eq!(receipts_2024[0].file_name, "flat.jpg");

        let receipts_2023 = db.get_receipts_for_year(2023).unwrap();
        assert_eq!(receipts_2023.len(), 1);

        let receipts_2025 = db.get_receipts_for_year(2025).unwrap();
        assert_eq!(receipts_2025.len(), 1);
    }

    /// Test: Combined scenario with all filtering cases
    #[test]
    fn test_get_receipts_for_year_combined_filtering() {
        let db = Database::in_memory().unwrap();

        // Case 1: Date 2024-05-01, source_year 2024 -> Show in 2024 (date matches)
        let r1 = create_receipt_for_year_test(
            "r1.jpg",
            Some(NaiveDate::from_ymd_opt(2024, 5, 1).unwrap()),
            Some(2024),
        );
        db.create_receipt(&r1).unwrap();

        // Case 2: Date 2024-05-01, source_year 2025 -> Show in 2024 (date wins)
        let r2 = create_receipt_for_year_test(
            "r2.jpg",
            Some(NaiveDate::from_ymd_opt(2024, 5, 1).unwrap()),
            Some(2025),
        );
        db.create_receipt(&r2).unwrap();

        // Case 3: No date, source_year 2024 -> Show in 2024 (fallback)
        let r3 = create_receipt_for_year_test("r3.jpg", None, Some(2024));
        db.create_receipt(&r3).unwrap();

        // Case 4: No date, source_year 2025 -> NOT in 2024
        let r4 = create_receipt_for_year_test("r4.jpg", None, Some(2025));
        db.create_receipt(&r4).unwrap();

        // Case 5: No date, no source_year -> Show in ALL years (flat)
        let r5 = create_receipt_for_year_test("r5.jpg", None, None);
        db.create_receipt(&r5).unwrap();

        // Case 6: Date 2023-12-31, source_year 2024 -> NOT in 2024 (date is 2023)
        let r6 = create_receipt_for_year_test(
            "r6.jpg",
            Some(NaiveDate::from_ymd_opt(2023, 12, 31).unwrap()),
            Some(2024),
        );
        db.create_receipt(&r6).unwrap();

        // Query for 2024: Should include r1, r2, r3, r5
        let receipts_2024 = db.get_receipts_for_year(2024).unwrap();
        let names: Vec<&str> = receipts_2024.iter().map(|r| r.file_name.as_str()).collect();

        assert_eq!(receipts_2024.len(), 4, "Expected 4 receipts for 2024, got: {:?}", names);
        assert!(names.contains(&"r1.jpg"), "r1 should be included (date matches)");
        assert!(names.contains(&"r2.jpg"), "r2 should be included (date wins over source_year)");
        assert!(names.contains(&"r3.jpg"), "r3 should be included (fallback to source_year)");
        assert!(names.contains(&"r5.jpg"), "r5 should be included (flat mode)");
        assert!(!names.contains(&"r4.jpg"), "r4 should NOT be included (source_year is 2025)");
        assert!(!names.contains(&"r6.jpg"), "r6 should NOT be included (date is 2023)");
    }
}
