//! Database layer using Diesel ORM for compile-time SQL safety.
//!
//! All CRUD operations use Diesel's type-safe query builder.
//! Row structs (VehicleRow, etc.) map directly to DB; conversions
//! to domain models (Vehicle, etc.) happen via From implementations.

use crate::models::{
    NewReceiptRow, NewRouteRow, NewSettingsRow, NewTripRow, NewVehicleRow, Receipt, ReceiptRow,
    Route, RouteRow, Settings, SettingsRow, Trip, TripRow, Vehicle, VehicleRow,
};
use crate::schema::{receipts, routes, settings, trips, vehicles};
use chrono::Utc;
use diesel::prelude::*;
use diesel::result::QueryResult;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

// Embed migrations at compile time
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

// ============================================================================
// Location Normalization
// ============================================================================

/// Normalize a location string for consistent storage and matching.
///
/// Currently performs:
/// - Trimming leading/trailing whitespace
/// - Collapsing multiple consecutive spaces into single space
///
/// This prevents duplicates like "Bratislava" vs "Bratislava " (trailing space)
/// which was observed in production data.
///
/// Note: Based on real data analysis, Slovak diacritics are NOT normalized
/// because users consistently type ASCII-only (Kosice, not Košice).
pub fn normalize_location(location: &str) -> String {
    location
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct Database {
    conn: Mutex<SqliteConnection>,
}

impl Database {
    pub fn new(path: PathBuf) -> Result<Self, diesel::ConnectionError> {
        let path_str = path.to_str().unwrap_or("");
        let mut conn = SqliteConnection::establish(path_str)?;

        // Run any pending migrations on startup
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    // Test helper for in-memory databases
    #[allow(dead_code)]
    pub fn in_memory() -> Result<Self, diesel::ConnectionError> {
        let mut conn = SqliteConnection::establish(":memory:")?;

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

    /// Get a raw connection for direct SQL (backup inspection, etc.)
    pub fn connection(&self) -> std::sync::MutexGuard<'_, SqliteConnection> {
        self.conn.lock().unwrap()
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
                vehicles::baseline_consumption_kwh
                    .eq(vehicle.baseline_consumption_kwh),
                vehicles::initial_battery_percent
                    .eq(vehicle.initial_battery_percent),
                vehicles::initial_odometer.eq(vehicle.initial_odometer),
                vehicles::is_active.eq(if vehicle.is_active { 1 } else { 0 }),
                vehicles::vin.eq(&vehicle.vin),
                vehicles::driver_name.eq(&vehicle.driver_name),
                vehicles::updated_at.eq(&updated_at_str),
            ))
            .execute(conn)?;

        Ok(())
    }

    pub fn delete_vehicle(&self, id: &str) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();

        // Unassign all receipts from this vehicle before deletion
        diesel::update(receipts::table.filter(receipts::vehicle_id.eq(id)))
            .set(receipts::vehicle_id.eq::<Option<String>>(None))
            .execute(conn)?;

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
        let date_str = trip.date.to_string();
        let created_at_str = trip.created_at.to_rfc3339();
        let updated_at_str = trip.updated_at.to_rfc3339();
        let other_costs_note_ref = trip.other_costs_note.as_deref();

        let new_trip = NewTripRow {
            id: &id_str,
            vehicle_id: &vehicle_id_str,
            date: &date_str,
            origin: &trip.origin,
            destination: &trip.destination,
            distance_km: trip.distance_km,
            odometer: trip.odometer,
            purpose: &trip.purpose,
            fuel_liters: trip.fuel_liters,
            fuel_cost_eur: trip.fuel_cost_eur,
            other_costs_eur: trip.other_costs_eur,
            other_costs_note: other_costs_note_ref,
            full_tank: if trip.full_tank { 1 } else { 0 },
            sort_order: trip.sort_order,
            energy_kwh: trip.energy_kwh,
            energy_cost_eur: trip.energy_cost_eur,
            full_charge: Some(if trip.full_charge { 1 } else { 0 }),
            soc_override_percent: trip.soc_override_percent,
            created_at: &created_at_str,
            updated_at: &updated_at_str,
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

        Ok(row.map(Trip::from))
    }

    pub fn get_trips_for_vehicle(&self, vehicle_id: &str) -> QueryResult<Vec<Trip>> {
        let conn = &mut *self.conn.lock().unwrap();

        let rows = trips::table
            .filter(trips::vehicle_id.eq(vehicle_id))
            .order(trips::sort_order.asc())
            .load::<TripRow>(conn)?;

        Ok(rows.into_iter().map(Trip::from).collect())
    }

    /// Get trips for a vehicle in a specific year (uses raw SQL for strftime)
    pub fn get_trips_for_vehicle_in_year(
        &self,
        vehicle_id: &str,
        year: i32,
    ) -> QueryResult<Vec<Trip>> {
        let conn = &mut *self.conn.lock().unwrap();

        // Raw SQL needed for strftime year extraction
        let rows = diesel::sql_query(
            "SELECT id, vehicle_id, date, origin, destination, distance_km, odometer, purpose,
                    fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note, full_tank,
                    sort_order, energy_kwh, energy_cost_eur, full_charge, soc_override_percent,
                    created_at, updated_at
             FROM trips
             WHERE vehicle_id = ? AND strftime('%Y', date) = ?
             ORDER BY sort_order ASC",
        )
        .bind::<diesel::sql_types::Text, _>(vehicle_id)
        .bind::<diesel::sql_types::Text, _>(year.to_string())
        .load::<TripRow>(conn)?;

        Ok(rows.into_iter().map(Trip::from).collect())
    }

    /// Get distinct years that have trips for a vehicle
    pub fn get_years_with_trips(&self, vehicle_id: &str) -> QueryResult<Vec<i32>> {
        let conn = &mut *self.conn.lock().unwrap();

        // Raw SQL needed for strftime
        #[derive(QueryableByName)]
        struct YearRow {
            #[diesel(sql_type = diesel::sql_types::Integer)]
            year: i32,
        }

        let rows = diesel::sql_query(
            "SELECT DISTINCT CAST(strftime('%Y', date) AS INTEGER) as year
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
        let date_str = trip.date.to_string();
        let updated_at_str = trip.updated_at.to_rfc3339();

        diesel::update(trips::table.filter(trips::id.eq(&id_str)))
            .set((
                trips::vehicle_id.eq(&vehicle_id_str),
                trips::date.eq(&date_str),
                trips::origin.eq(&trip.origin),
                trips::destination.eq(&trip.destination),
                trips::distance_km.eq(trip.distance_km),
                trips::odometer.eq(trip.odometer),
                trips::purpose.eq(&trip.purpose),
                trips::fuel_liters.eq(trip.fuel_liters),
                trips::fuel_cost_eur.eq(trip.fuel_cost_eur),
                trips::other_costs_eur.eq(trip.other_costs_eur),
                trips::other_costs_note.eq(&trip.other_costs_note),
                trips::full_tank.eq(if trip.full_tank { 1 } else { 0 }),
                trips::sort_order.eq(trip.sort_order),
                trips::energy_kwh.eq(trip.energy_kwh),
                trips::energy_cost_eur.eq(trip.energy_cost_eur),
                trips::full_charge.eq(Some(if trip.full_charge { 1 } else { 0 })),
                trips::soc_override_percent.eq(trip.soc_override_percent),
                trips::updated_at.eq(&updated_at_str),
            ))
            .execute(conn)?;

        Ok(())
    }

    pub fn delete_trip(&self, id: &str) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        diesel::delete(trips::table.filter(trips::id.eq(id))).execute(conn)?;
        Ok(())
    }

    /// Reorder a trip to a new position (uses transaction for atomicity)
    pub fn reorder_trip(&self, trip_id: &str, new_sort_order: i32) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();

        conn.transaction(|conn| {
            // Get current trip info
            #[derive(QueryableByName)]
            struct TripInfo {
                #[diesel(sql_type = diesel::sql_types::Text)]
                vehicle_id: String,
                #[diesel(sql_type = diesel::sql_types::Integer)]
                sort_order: i32,
            }

            let info: TripInfo = diesel::sql_query(
                "SELECT vehicle_id, sort_order FROM trips WHERE id = ?",
            )
            .bind::<diesel::sql_types::Text, _>(trip_id)
            .get_result(conn)?;

            let old_sort_order = info.sort_order;
            let vehicle_id = info.vehicle_id;

            if old_sort_order < new_sort_order {
                // Moving down: decrement sort_order for trips between old and new position
                diesel::sql_query(
                    "UPDATE trips SET sort_order = sort_order - 1
                     WHERE vehicle_id = ? AND sort_order > ? AND sort_order <= ?",
                )
                .bind::<diesel::sql_types::Text, _>(&vehicle_id)
                .bind::<diesel::sql_types::Integer, _>(old_sort_order)
                .bind::<diesel::sql_types::Integer, _>(new_sort_order)
                .execute(conn)?;
            } else if old_sort_order > new_sort_order {
                // Moving up: increment sort_order for trips between new and old position
                diesel::sql_query(
                    "UPDATE trips SET sort_order = sort_order + 1
                     WHERE vehicle_id = ? AND sort_order >= ? AND sort_order < ?",
                )
                .bind::<diesel::sql_types::Text, _>(&vehicle_id)
                .bind::<diesel::sql_types::Integer, _>(new_sort_order)
                .bind::<diesel::sql_types::Integer, _>(old_sort_order)
                .execute(conn)?;
            }

            // Update the moved trip's sort_order
            let now = Utc::now().to_rfc3339();
            diesel::sql_query("UPDATE trips SET sort_order = ?, updated_at = ? WHERE id = ?")
                .bind::<diesel::sql_types::Integer, _>(new_sort_order)
                .bind::<diesel::sql_types::Text, _>(&now)
                .bind::<diesel::sql_types::Text, _>(trip_id)
                .execute(conn)?;

            Ok(())
        })
    }

    /// Shift all trips at or after a position down by 1 (for insertion)
    pub fn shift_trips_from_position(&self, vehicle_id: &str, from_position: i32) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();

        diesel::sql_query(
            "UPDATE trips SET sort_order = sort_order + 1
             WHERE vehicle_id = ? AND sort_order >= ?",
        )
        .bind::<diesel::sql_types::Text, _>(vehicle_id)
        .bind::<diesel::sql_types::Integer, _>(from_position)
        .execute(conn)?;

        Ok(())
    }

    // ========================================================================
    // Route CRUD Operations
    // ========================================================================

    pub fn get_routes_for_vehicle(&self, vehicle_id: &str) -> QueryResult<Vec<Route>> {
        let conn = &mut *self.conn.lock().unwrap();

        let rows = routes::table
            .filter(routes::vehicle_id.eq(vehicle_id))
            .order(routes::usage_count.desc())
            .load::<RouteRow>(conn)?;

        Ok(rows.into_iter().map(Route::from).collect())
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

    /// Find existing route with same origin/destination, or create new one.
    ///
    /// Input locations are normalized (trimmed, whitespace collapsed) before
    /// lookup and storage to prevent duplicates like "Bratislava" vs "Bratislava ".
    pub fn find_or_create_route(
        &self,
        vehicle_id: &str,
        origin: &str,
        destination: &str,
        distance_km: f64,
    ) -> QueryResult<Route> {
        // Normalize inputs to prevent whitespace-based duplicates
        let origin = normalize_location(origin);
        let destination = normalize_location(destination);

        let conn = &mut *self.conn.lock().unwrap();

        // Try to find existing route with normalized values
        let existing = routes::table
            .filter(routes::vehicle_id.eq(vehicle_id))
            .filter(routes::origin.eq(&origin))
            .filter(routes::destination.eq(&destination))
            .first::<RouteRow>(conn)
            .optional()?;

        if let Some(row) = existing {
            // Update existing route: increment usage_count
            let new_count = row.usage_count + 1;
            let now = Utc::now().to_rfc3339();

            diesel::update(routes::table.filter(routes::id.eq(&row.id)))
                .set((
                    routes::usage_count.eq(new_count),
                    routes::last_used.eq(&now),
                ))
                .execute(conn)?;

            // Return updated route
            let mut route = Route::from(row);
            route.usage_count = new_count;
            route.last_used = Utc::now();
            Ok(route)
        } else {
            // Create new route with normalized values
            let route = Route {
                id: Uuid::new_v4(),
                vehicle_id: vehicle_id.parse().unwrap(),
                origin: origin.clone(),
                destination: destination.clone(),
                distance_km,
                usage_count: 1,
                last_used: Utc::now(),
            };

            let id_str = route.id.to_string();
            let vehicle_id_str = route.vehicle_id.to_string();
            let last_used_str = route.last_used.to_rfc3339();

            let new_route = NewRouteRow {
                id: &id_str,
                vehicle_id: &vehicle_id_str,
                origin: &route.origin,
                destination: &route.destination,
                distance_km: route.distance_km,
                usage_count: route.usage_count,
                last_used: &last_used_str,
            };

            diesel::insert_into(routes::table)
                .values(&new_route)
                .execute(conn)?;

            Ok(route)
        }
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
    // Receipt CRUD Operations
    // ========================================================================

    pub fn create_receipt(&self, receipt: &Receipt) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();

        let id_str = receipt.id.to_string();
        let vehicle_id_str = receipt.vehicle_id.map(|id| id.to_string());
        let trip_id_str = receipt.trip_id.map(|id| id.to_string());
        let scanned_at_str = receipt.scanned_at.to_rfc3339();
        let receipt_date_str = receipt.receipt_date.map(|d| d.to_string());
        let created_at_str = receipt.created_at.to_rfc3339();
        let updated_at_str = receipt.updated_at.to_rfc3339();
        let confidence_json = receipt.confidence_to_json();

        let new_receipt = NewReceiptRow {
            id: &id_str,
            vehicle_id: vehicle_id_str.as_deref(),
            trip_id: trip_id_str.as_deref(),
            file_path: &receipt.file_path,
            file_name: &receipt.file_name,
            scanned_at: &scanned_at_str,
            liters: receipt.liters,
            total_price_eur: receipt.total_price_eur,
            receipt_date: receipt_date_str.as_deref(),
            station_name: receipt.station_name.as_deref(),
            station_address: receipt.station_address.as_deref(),
            source_year: receipt.source_year,
            status: receipt.status_to_str(),
            confidence: &confidence_json,
            raw_ocr_text: receipt.raw_ocr_text.as_deref(),
            error_message: receipt.error_message.as_deref(),
            created_at: &created_at_str,
            updated_at: &updated_at_str,
            vendor_name: receipt.vendor_name.as_deref(),
            cost_description: receipt.cost_description.as_deref(),
        };

        diesel::insert_into(receipts::table)
            .values(&new_receipt)
            .execute(conn)?;

        Ok(())
    }

    pub fn get_all_receipts(&self) -> QueryResult<Vec<Receipt>> {
        let conn = &mut *self.conn.lock().unwrap();

        let rows = receipts::table
            .order(receipts::scanned_at.desc())
            .load::<ReceiptRow>(conn)?;

        Ok(rows.into_iter().map(Receipt::from).collect())
    }

    pub fn get_unassigned_receipts(&self) -> QueryResult<Vec<Receipt>> {
        let conn = &mut *self.conn.lock().unwrap();

        let rows = receipts::table
            .filter(receipts::trip_id.is_null())
            .order((receipts::receipt_date.desc(), receipts::scanned_at.desc()))
            .load::<ReceiptRow>(conn)?;

        Ok(rows.into_iter().map(Receipt::from).collect())
    }

    pub fn get_pending_receipts(&self) -> QueryResult<Vec<Receipt>> {
        let conn = &mut *self.conn.lock().unwrap();

        let rows = receipts::table
            .filter(receipts::status.eq("Pending"))
            .order(receipts::scanned_at.asc())
            .load::<ReceiptRow>(conn)?;

        Ok(rows.into_iter().map(Receipt::from).collect())
    }

    pub fn update_receipt(&self, receipt: &Receipt) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();

        let id_str = receipt.id.to_string();
        let vehicle_id_str = receipt.vehicle_id.map(|id| id.to_string());
        let trip_id_str = receipt.trip_id.map(|id| id.to_string());
        let receipt_date_str = receipt.receipt_date.map(|d| d.to_string());
        let updated_at_str = Utc::now().to_rfc3339();
        let confidence_json = receipt.confidence_to_json();

        diesel::update(receipts::table.filter(receipts::id.eq(&id_str)))
            .set((
                receipts::vehicle_id.eq(vehicle_id_str),
                receipts::trip_id.eq(trip_id_str),
                receipts::liters.eq(receipt.liters),
                receipts::total_price_eur.eq(receipt.total_price_eur),
                receipts::receipt_date.eq(receipt_date_str),
                receipts::station_name.eq(&receipt.station_name),
                receipts::station_address.eq(&receipt.station_address),
                receipts::source_year.eq(receipt.source_year),
                receipts::status.eq(receipt.status_to_str()),
                receipts::confidence.eq(&confidence_json),
                receipts::raw_ocr_text.eq(&receipt.raw_ocr_text),
                receipts::error_message.eq(&receipt.error_message),
                receipts::updated_at.eq(&updated_at_str),
                receipts::vendor_name.eq(&receipt.vendor_name),
                receipts::cost_description.eq(&receipt.cost_description),
            ))
            .execute(conn)?;

        Ok(())
    }

    pub fn delete_receipt(&self, id: &str) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        diesel::delete(receipts::table.filter(receipts::id.eq(id))).execute(conn)?;
        Ok(())
    }

    pub fn get_receipt_by_file_path(&self, file_path: &str) -> QueryResult<Option<Receipt>> {
        let conn = &mut *self.conn.lock().unwrap();

        let row = receipts::table
            .filter(receipts::file_path.eq(file_path))
            .first::<ReceiptRow>(conn)
            .optional()?;

        Ok(row.map(Receipt::from))
    }

    pub fn get_receipt_by_id(&self, id: &str) -> QueryResult<Option<Receipt>> {
        let conn = &mut *self.conn.lock().unwrap();

        let row = receipts::table
            .filter(receipts::id.eq(id))
            .first::<ReceiptRow>(conn)
            .optional()?;

        Ok(row.map(Receipt::from))
    }

    /// Get receipts filtered by year (raw SQL for strftime)
    pub fn get_receipts_for_year(&self, year: i32) -> QueryResult<Vec<Receipt>> {
        let conn = &mut *self.conn.lock().unwrap();

        let rows = diesel::sql_query(
            "SELECT id, vehicle_id, trip_id, file_path, file_name, scanned_at,
                    liters, total_price_eur, receipt_date, station_name, station_address,
                    source_year, status, confidence, raw_ocr_text, error_message,
                    created_at, updated_at, vendor_name, cost_description
             FROM receipts WHERE
                (receipt_date IS NOT NULL AND CAST(strftime('%Y', receipt_date) AS INTEGER) = ?)
                OR (receipt_date IS NULL AND source_year = ?)
                OR (receipt_date IS NULL AND source_year IS NULL)
             ORDER BY receipt_date DESC, scanned_at DESC",
        )
        .bind::<diesel::sql_types::Integer, _>(year)
        .bind::<diesel::sql_types::Integer, _>(year)
        .load::<ReceiptRow>(conn)?;

        Ok(rows.into_iter().map(Receipt::from).collect())
    }

    /// Get receipts filtered by vehicle (unassigned + own)
    pub fn get_receipts_for_vehicle(
        &self,
        vehicle_id: &Uuid,
        year: Option<i32>,
    ) -> QueryResult<Vec<Receipt>> {
        let conn = &mut *self.conn.lock().unwrap();
        let vehicle_id_str = vehicle_id.to_string();

        let rows = match year {
            Some(y) => diesel::sql_query(
                "SELECT id, vehicle_id, trip_id, file_path, file_name, scanned_at,
                        liters, total_price_eur, receipt_date, station_name, station_address,
                        source_year, status, confidence, raw_ocr_text, error_message,
                        created_at, updated_at, vendor_name, cost_description
                 FROM receipts
                 WHERE (vehicle_id IS NULL OR vehicle_id = ?)
                   AND (
                     (receipt_date IS NOT NULL AND CAST(strftime('%Y', receipt_date) AS INTEGER) = ?)
                     OR (receipt_date IS NULL AND source_year = ?)
                     OR (receipt_date IS NULL AND source_year IS NULL)
                   )
                 ORDER BY receipt_date DESC, scanned_at DESC",
            )
            .bind::<diesel::sql_types::Text, _>(&vehicle_id_str)
            .bind::<diesel::sql_types::Integer, _>(y)
            .bind::<diesel::sql_types::Integer, _>(y)
            .load::<ReceiptRow>(conn)?,
            None => diesel::sql_query(
                "SELECT id, vehicle_id, trip_id, file_path, file_name, scanned_at,
                        liters, total_price_eur, receipt_date, station_name, station_address,
                        source_year, status, confidence, raw_ocr_text, error_message,
                        created_at, updated_at, vendor_name, cost_description
                 FROM receipts
                 WHERE (vehicle_id IS NULL OR vehicle_id = ?)
                 ORDER BY receipt_date DESC, scanned_at DESC",
            )
            .bind::<diesel::sql_types::Text, _>(&vehicle_id_str)
            .load::<ReceiptRow>(conn)?,
        };

        Ok(rows.into_iter().map(Receipt::from).collect())
    }
}

#[cfg(test)]
#[path = "db_tests.rs"]
mod tests;
