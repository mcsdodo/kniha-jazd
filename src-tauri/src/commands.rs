//! Tauri commands to expose Rust functionality to the frontend

use crate::calculations::{
    calculate_consumption_rate, calculate_margin_percent, calculate_spotreba, calculate_zostatok,
    is_within_legal_limit,
};
use crate::db::Database;
use crate::models::{Route, Settings, Trip, TripStats, Vehicle};
use crate::suggestions::{build_compensation_suggestion, CompensationSuggestion};
use chrono::{NaiveDate, Utc, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{Manager, State};
use uuid::Uuid;

// ============================================================================
// Backup Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub filename: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub vehicle_count: i32,
    pub trip_count: i32,
}

// ============================================================================
// Vehicle Commands
// ============================================================================

#[tauri::command]
pub fn get_vehicles(db: State<Database>) -> Result<Vec<Vehicle>, String> {
    db.get_all_vehicles().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_active_vehicle(db: State<Database>) -> Result<Option<Vehicle>, String> {
    db.get_active_vehicle().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_vehicle(
    db: State<Database>,
    name: String,
    license_plate: String,
    tank_size: f64,
    tp_consumption: f64,
    initial_odometer: f64,
) -> Result<Vehicle, String> {
    let vehicle = Vehicle::new(name, license_plate, tank_size, tp_consumption, initial_odometer);
    db.create_vehicle(&vehicle).map_err(|e| e.to_string())?;
    Ok(vehicle)
}

#[tauri::command]
pub fn update_vehicle(db: State<Database>, vehicle: Vehicle) -> Result<(), String> {
    db.update_vehicle(&vehicle).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_vehicle(db: State<Database>, id: String) -> Result<(), String> {
    db.delete_vehicle(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_active_vehicle(db: State<Database>, id: String) -> Result<(), String> {
    // First, get all vehicles
    let vehicles = db.get_all_vehicles().map_err(|e| e.to_string())?;

    // Set all to inactive
    for mut vehicle in vehicles {
        let should_be_active = vehicle.id.to_string() == id;
        if vehicle.is_active != should_be_active {
            vehicle.is_active = should_be_active;
            vehicle.updated_at = Utc::now();
            db.update_vehicle(&vehicle).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

// ============================================================================
// Trip Commands
// ============================================================================

#[tauri::command]
pub fn get_trips(db: State<Database>, vehicle_id: String) -> Result<Vec<Trip>, String> {
    db.get_trips_for_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_trips_for_year(
    db: State<Database>,
    vehicle_id: String,
    year: i32,
) -> Result<Vec<Trip>, String> {
    db.get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_trip(
    db: State<Database>,
    vehicle_id: String,
    date: String,
    origin: String,
    destination: String,
    distance_km: f64,
    odometer: f64,
    purpose: String,
    fuel_liters: Option<f64>,
    fuel_cost: Option<f64>,
    other_costs: Option<f64>,
    other_costs_note: Option<String>,
    full_tank: Option<bool>,
    insert_at_position: Option<i32>,
) -> Result<Trip, String> {
    let vehicle_uuid = Uuid::parse_str(&vehicle_id).map_err(|e| e.to_string())?;
    let trip_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|e| e.to_string())?;

    // Determine sort_order
    let sort_order = if let Some(position) = insert_at_position {
        // Shift existing trips down to make room
        db.shift_trips_from_position(&vehicle_id, position)
            .map_err(|e| e.to_string())?;
        position
    } else {
        // Insert at top (sort_order = 0), shift all existing down
        db.shift_trips_from_position(&vehicle_id, 0)
            .map_err(|e| e.to_string())?;
        0
    };

    let now = Utc::now();
    let trip = Trip {
        id: Uuid::new_v4(),
        vehicle_id: vehicle_uuid,
        date: trip_date,
        origin: origin.clone(),
        destination: destination.clone(),
        distance_km,
        odometer,
        purpose,
        fuel_liters,
        fuel_cost_eur: fuel_cost,
        other_costs_eur: other_costs,
        other_costs_note,
        full_tank: full_tank.unwrap_or(true), // Default to true (full tank)
        sort_order,
        created_at: now,
        updated_at: now,
    };

    db.create_trip(&trip).map_err(|e| e.to_string())?;

    // Update or create route for autocomplete
    db.find_or_create_route(&vehicle_id, &origin, &destination, distance_km)
        .map_err(|e| e.to_string())?;

    Ok(trip)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_trip(
    db: State<Database>,
    id: String,
    date: String,
    origin: String,
    destination: String,
    distance_km: f64,
    odometer: f64,
    purpose: String,
    fuel_liters: Option<f64>,
    fuel_cost_eur: Option<f64>,
    other_costs_eur: Option<f64>,
    other_costs_note: Option<String>,
    full_tank: Option<bool>,
) -> Result<Trip, String> {
    let trip_uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let trip_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|e| e.to_string())?;

    // Get the existing trip to preserve vehicle_id and created_at
    let existing = db
        .get_trip(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip not found: {}", id))?;

    let trip = Trip {
        id: trip_uuid,
        vehicle_id: existing.vehicle_id,
        date: trip_date,
        origin,
        destination,
        distance_km,
        odometer,
        purpose,
        fuel_liters,
        fuel_cost_eur,
        other_costs_eur,
        other_costs_note,
        full_tank: full_tank.unwrap_or(existing.full_tank), // Preserve existing if not provided
        sort_order: existing.sort_order,
        created_at: existing.created_at,
        updated_at: Utc::now(),
    };

    db.update_trip(&trip).map_err(|e| e.to_string())?;
    Ok(trip)
}

#[tauri::command]
pub fn delete_trip(db: State<Database>, id: String) -> Result<(), String> {
    db.delete_trip(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_trip(
    db: State<Database>,
    trip_id: String,
    new_sort_order: i32,
) -> Result<Vec<Trip>, String> {
    // Get the trip to find its vehicle_id
    let trip = db
        .get_trip(&trip_id)
        .map_err(|e| e.to_string())?
        .ok_or("Trip not found")?;

    // Reorder trips in database (only changes sort_order, not date)
    db.reorder_trip(&trip_id, new_sort_order)
        .map_err(|e| e.to_string())?;

    // Return updated trip list
    db.get_trips_for_vehicle(&trip.vehicle_id.to_string())
        .map_err(|e| e.to_string())
}

// ============================================================================
// Route Commands
// ============================================================================

#[tauri::command]
pub fn get_routes(db: State<Database>, vehicle_id: String) -> Result<Vec<Route>, String> {
    db.get_routes_for_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())
}

// ============================================================================
// Calculation Commands
// ============================================================================

#[tauri::command]
pub fn get_compensation_suggestion(
    db: State<Database>,
    vehicle_id: String,
    buffer_km: f64,
    current_location: String,
) -> Result<CompensationSuggestion, String> {
    // Get routes for this vehicle
    let routes = db
        .get_routes_for_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?;

    // For buffer trip purpose, we'll use "služobná cesta" as default
    // In a full implementation, this would come from Settings
    let buffer_purpose = "služobná cesta";

    let suggestion = build_compensation_suggestion(&routes, buffer_km, &current_location, buffer_purpose);

    Ok(suggestion)
}

// ============================================================================
// Settings Commands
// ============================================================================

#[tauri::command]
pub fn get_settings(db: State<Database>) -> Result<Option<Settings>, String> {
    db.get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(
    db: State<Database>,
    company_name: String,
    company_ico: String,
    buffer_trip_purpose: String,
) -> Result<Settings, String> {
    let settings = Settings {
        id: Uuid::new_v4(),
        company_name,
        company_ico,
        buffer_trip_purpose,
        updated_at: Utc::now(),
    };

    db.save_settings(&settings).map_err(|e| e.to_string())?;
    Ok(settings)
}

// ============================================================================
// Trip Statistics Commands
// ============================================================================

#[tauri::command]
pub fn calculate_trip_stats(
    vehicle_id: String,
    db: State<Database>,
) -> Result<TripStats, String> {
    // Get vehicle
    let vehicle_uuid = Uuid::parse_str(&vehicle_id).map_err(|e| e.to_string())?;
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    // Get all trips for this vehicle, sorted by date + odometer (for same-day trips)
    let mut trips = db
        .get_trips_for_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?;
    trips.sort_by(|a, b| {
        a.date.cmp(&b.date).then_with(|| a.odometer.partial_cmp(&b.odometer).unwrap_or(std::cmp::Ordering::Equal))
    });

    // If no trips, return default values
    if trips.is_empty() {
        return Ok(TripStats {
            zostatok_liters: vehicle.tank_size_liters,
            avg_consumption_rate: 0.0,
            last_consumption_rate: 0.0,
            margin_percent: None,
            is_over_limit: false,
            total_km: 0.0,
            total_fuel_liters: 0.0,
            total_fuel_cost_eur: 0.0,
        });
    }

    // Calculate totals
    let total_fuel: f64 = trips.iter().filter_map(|t| t.fuel_liters).sum();
    let total_fuel_cost: f64 = trips.iter().filter_map(|t| t.fuel_cost_eur).sum();
    let total_km: f64 = trips.iter().map(|t| t.distance_km).sum();
    let avg_consumption_rate = if total_km > 0.0 {
        (total_fuel / total_km) * 100.0
    } else {
        0.0
    };

    // Find the last fill-up to calculate current consumption rate
    let mut last_fillup_idx = None;
    for (idx, trip) in trips.iter().enumerate().rev() {
        if trip.is_fillup() {
            last_fillup_idx = Some(idx);
            break;
        }
    }

    // Calculate last consumption rate from last fill-up (for current period tracking)
    let last_consumption_rate = if let Some(idx) = last_fillup_idx {
        let fillup_trip = &trips[idx];
        let fuel_liters = fillup_trip.fuel_liters.unwrap();

        // Calculate total distance since previous fill-up
        let mut km_since_last_fillup = 0.0;
        let mut prev_fillup_idx = None;
        for i in (0..idx).rev() {
            if trips[i].is_fillup() {
                prev_fillup_idx = Some(i);
                break;
            }
        }

        // Sum up distances from previous fill-up to current fill-up
        let start_idx = prev_fillup_idx.map(|i| i + 1).unwrap_or(0);
        for trip in &trips[start_idx..=idx] {
            km_since_last_fillup += trip.distance_km;
        }

        calculate_consumption_rate(fuel_liters, km_since_last_fillup)
    } else {
        // No fill-up yet, use TP consumption
        vehicle.tp_consumption
    };

    // Calculate current zostatok by processing all trips sequentially
    // Note: For accurate zostatok, we should use per-period rates, but for header display
    // we use the last consumption rate as a reasonable approximation
    let mut current_zostatok = vehicle.tank_size_liters; // Start with full tank

    for trip in &trips {
        // Calculate spotreba for this trip
        let spotreba = calculate_spotreba(trip.distance_km, last_consumption_rate);

        // Update zostatok
        current_zostatok = calculate_zostatok(
            current_zostatok,
            spotreba,
            trip.fuel_liters,
            vehicle.tank_size_liters,
        );
    }

    // Check if over legal limit based on AVERAGE consumption (legal compliance)
    let avg_margin = calculate_margin_percent(avg_consumption_rate, vehicle.tp_consumption);
    let is_over_limit = if total_fuel > 0.0 {
        !is_within_legal_limit(avg_margin)
    } else {
        false
    };

    // Use average margin for legal compliance display
    let display_margin = if total_fuel > 0.0 { Some(avg_margin) } else { None };

    Ok(TripStats {
        zostatok_liters: current_zostatok,
        avg_consumption_rate,
        last_consumption_rate,
        margin_percent: display_margin,
        is_over_limit,
        total_km,
        total_fuel_liters: total_fuel,
        total_fuel_cost_eur: total_fuel_cost,
    })
}

// ============================================================================
// Backup Commands
// ============================================================================

#[tauri::command]
pub fn create_backup(app: tauri::AppHandle, db: State<Database>) -> Result<BackupInfo, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup_dir = app_dir.join("backups");

    // Create backup directory if it doesn't exist
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    // Generate backup filename with timestamp
    let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
    let filename = format!("kniha-jazd-backup-{}.db", timestamp);
    let backup_path = backup_dir.join(&filename);

    // Copy current database to backup
    let db_path = app_dir.join("kniha-jazd.db");
    fs::copy(&db_path, &backup_path).map_err(|e| e.to_string())?;

    // Get file size
    let metadata = fs::metadata(&backup_path).map_err(|e| e.to_string())?;

    // Get counts from current database
    let vehicles = db.get_all_vehicles().map_err(|e| e.to_string())?;
    let vehicle_count = vehicles.len() as i32;

    // Count trips across all vehicles
    let mut trip_count = 0;
    for vehicle in &vehicles {
        let trips = db.get_trips_for_vehicle(&vehicle.id.to_string()).map_err(|e| e.to_string())?;
        trip_count += trips.len() as i32;
    }

    Ok(BackupInfo {
        filename,
        created_at: Local::now().to_rfc3339(),
        size_bytes: metadata.len(),
        vehicle_count,
        trip_count,
    })
}

#[tauri::command]
pub fn list_backups(app: tauri::AppHandle) -> Result<Vec<BackupInfo>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup_dir = app_dir.join("backups");

    if !backup_dir.exists() {
        return Ok(vec![]);
    }

    let mut backups = Vec::new();

    for entry in fs::read_dir(&backup_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.extension().map(|e| e == "db").unwrap_or(false) {
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;

            // Parse timestamp from filename: kniha-jazd-backup-YYYY-MM-DD-HHMMSS.db
            let created_at = if filename.starts_with("kniha-jazd-backup-") {
                let date_part = filename
                    .trim_start_matches("kniha-jazd-backup-")
                    .trim_end_matches(".db");
                // Convert YYYY-MM-DD-HHMMSS to ISO format
                if date_part.len() >= 17 {
                    format!(
                        "{}-{}-{}T{}:{}:{}",
                        &date_part[0..4],
                        &date_part[5..7],
                        &date_part[8..10],
                        &date_part[11..13],
                        &date_part[13..15],
                        &date_part[15..17]
                    )
                } else {
                    Local::now().to_rfc3339()
                }
            } else {
                Local::now().to_rfc3339()
            };

            // We can't easily get counts from backup without opening it
            // So we'll return 0 for now - the get_backup_info command will show actual counts
            backups.push(BackupInfo {
                filename,
                created_at,
                size_bytes: metadata.len(),
                vehicle_count: 0,
                trip_count: 0,
            });
        }
    }

    // Sort by filename descending (newest first)
    backups.sort_by(|a, b| b.filename.cmp(&a.filename));

    Ok(backups)
}

#[tauri::command]
pub fn get_backup_info(app: tauri::AppHandle, filename: String) -> Result<BackupInfo, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup_path = app_dir.join("backups").join(&filename);

    if !backup_path.exists() {
        return Err(format!("Backup not found: {}", filename));
    }

    let metadata = fs::metadata(&backup_path).map_err(|e| e.to_string())?;

    // Open backup database to get counts
    let conn = rusqlite::Connection::open(&backup_path).map_err(|e| e.to_string())?;

    let vehicle_count: i32 = conn
        .query_row("SELECT COUNT(*) FROM vehicles", [], |row| row.get(0))
        .unwrap_or(0);

    let trip_count: i32 = conn
        .query_row("SELECT COUNT(*) FROM trips", [], |row| row.get(0))
        .unwrap_or(0);

    // Parse timestamp from filename
    let created_at = if filename.starts_with("kniha-jazd-backup-") {
        let date_part = filename
            .trim_start_matches("kniha-jazd-backup-")
            .trim_end_matches(".db");
        if date_part.len() >= 17 {
            format!(
                "{}-{}-{}T{}:{}:{}",
                &date_part[0..4],
                &date_part[5..7],
                &date_part[8..10],
                &date_part[11..13],
                &date_part[13..15],
                &date_part[15..17]
            )
        } else {
            Local::now().to_rfc3339()
        }
    } else {
        Local::now().to_rfc3339()
    };

    Ok(BackupInfo {
        filename,
        created_at,
        size_bytes: metadata.len(),
        vehicle_count,
        trip_count,
    })
}

#[tauri::command]
pub fn restore_backup(app: tauri::AppHandle, filename: String) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup_path = app_dir.join("backups").join(&filename);
    let db_path = app_dir.join("kniha-jazd.db");

    if !backup_path.exists() {
        return Err(format!("Backup not found: {}", filename));
    }

    // Auto-backup current database before restore
    let backup_dir = app_dir.join("backups");
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
    let auto_backup_filename = format!("kniha-jazd-pre-restore-{}.db", timestamp);
    let auto_backup_path = backup_dir.join(&auto_backup_filename);

    fs::copy(&db_path, &auto_backup_path).map_err(|e| format!("Failed to create pre-restore backup: {}", e))?;

    // Copy backup over current database
    fs::copy(&backup_path, &db_path).map_err(|e| e.to_string())?;

    Ok(())
}
