//! Tauri commands to expose Rust functionality to the frontend

use crate::calculations::{
    calculate_buffer_km, calculate_closed_period_totals, calculate_consumption_rate,
    calculate_fuel_level, calculate_fuel_used, calculate_margin_percent, is_within_legal_limit,
};
use crate::calculations_energy::{
    calculate_battery_remaining, calculate_energy_used, kwh_to_percent,
};
use crate::calculations_phev::calculate_phev_trip_consumption;
use crate::db::{normalize_location, Database};
use crate::db_location::{resolve_db_paths, DbPaths};
use crate::export::{generate_html, ExportData, ExportLabels, ExportTotals};
use crate::models::{PreviewResult, Route, Settings, Trip, TripGridData, TripStats, Vehicle, VehicleType};
use crate::settings::LocalSettings;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use chrono::{Datelike, NaiveDate, Utc, Local};
use diesel::RunQueryDsl;
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{Emitter, Manager, State};
use uuid::Uuid;

// ============================================================================
// Helper Functions
// ============================================================================
use crate::app_state::AppState;

// ============================================================================
// Read-Only Guard Macro
// ============================================================================

/// Macro to check if app is in read-only mode before write operations.
/// Returns an error with Slovak message if read-only.
macro_rules! check_read_only {
    ($app_state:expr) => {
        if $app_state.is_read_only() {
            let reason = $app_state.get_read_only_reason()
                .unwrap_or_else(|| "Neznámy dôvod".to_string());
            return Err(format!(
                "Aplikácia je v režime len na čítanie. {}",
                reason
            ));
        }
    };
}


/// Get the app data directory, respecting the KNIHA_JAZD_DATA_DIR environment variable.
/// This ensures consistency between database operations and other file operations (backups, settings).
fn get_app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    match std::env::var("KNIHA_JAZD_DATA_DIR") {
        Ok(path) => Ok(PathBuf::from(path)),
        Err(_) => app.path().app_data_dir().map_err(|e| e.to_string()),
    }
}

/// Get resolved database paths (including backups directory), respecting custom_db_path in local.settings.json.
/// This ensures backups are stored alongside the database, even when using a custom location.
fn get_db_paths(app: &tauri::AppHandle) -> Result<DbPaths, String> {
    let app_dir = get_app_data_dir(app)?;
    let local_settings = LocalSettings::load(&app_dir);
    let (db_paths, _is_custom) = resolve_db_paths(&app_dir, local_settings.custom_db_path.as_deref());
    Ok(db_paths)
}

// ============================================================================
// Backup Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub filename: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub vehicle_count: i32,
    pub trip_count: i32,
    pub backup_type: String,            // "manual" | "pre-update"
    pub update_version: Option<String>, // e.g., "0.20.0" for pre-update backups
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPreview {
    pub to_delete: Vec<BackupInfo>,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub deleted: Vec<String>,
    pub freed_bytes: u64,
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
#[allow(clippy::too_many_arguments)]
pub fn create_vehicle(
    db: State<Database>,
    app_state: State<AppState>,
    name: String,
    license_plate: String,
    initial_odometer: f64,
    // Vehicle type: "Ice", "Bev", or "Phev"
    vehicle_type: Option<String>,
    // Fuel fields (ICE + PHEV)
    tank_size_liters: Option<f64>,
    tp_consumption: Option<f64>,
    // Battery fields (BEV + PHEV)
    battery_capacity_kwh: Option<f64>,
    baseline_consumption_kwh: Option<f64>,

    initial_battery_percent: Option<f64>,
    vin: Option<String>,
    driver_name: Option<String>,
) -> Result<Vehicle, String> {
    check_read_only!(app_state);
    // Parse vehicle type (default to ICE for backward compatibility)
    let vt = match vehicle_type.as_deref() {
        Some("Bev") | Some("BEV") => VehicleType::Bev,
        Some("Phev") | Some("PHEV") => VehicleType::Phev,
        _ => VehicleType::Ice,
    };

    // Validate required fields based on vehicle type
    match vt {
        VehicleType::Ice => {
            if tank_size_liters.is_none() || tp_consumption.is_none() {
                return Err("ICE vehicles require tank_size_liters and tp_consumption".to_string());
            }
        }
        VehicleType::Bev => {
            if battery_capacity_kwh.is_none() || baseline_consumption_kwh.is_none() {
                return Err("BEV vehicles require battery_capacity_kwh and baseline_consumption_kwh".to_string());
            }
        }
        VehicleType::Phev => {
            if tank_size_liters.is_none() || tp_consumption.is_none()
                || battery_capacity_kwh.is_none() || baseline_consumption_kwh.is_none() {
                return Err("PHEV vehicles require both fuel and battery fields".to_string());
            }
        }
    }

    let now = Utc::now();
    let vehicle = Vehicle {
        id: Uuid::new_v4(),
        name,
        license_plate,
        vehicle_type: vt,
        tank_size_liters,
        tp_consumption,
        battery_capacity_kwh,
        baseline_consumption_kwh,
        initial_battery_percent,
        initial_odometer,

        is_active: true,
        vin,
        driver_name,
        created_at: now,
        updated_at: now,
    };

    db.create_vehicle(&vehicle).map_err(|e| e.to_string())?;
    Ok(vehicle)
}

#[tauri::command]
pub fn update_vehicle(db: State<Database>, app_state: State<AppState>, vehicle: Vehicle) -> Result<(), String> {
    check_read_only!(app_state);
    // Check if vehicle type is being changed when trips exist
    let existing = db
        .get_vehicle(&vehicle.id.to_string())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Vehicle not found: {}", vehicle.id))?;

    if existing.vehicle_type != vehicle.vehicle_type {
        // Check if this vehicle has any trips
        let trips = db
            .get_trips_for_vehicle(&vehicle.id.to_string())
            .map_err(|e| e.to_string())?;

        if !trips.is_empty() {
            return Err(
                "Cannot change vehicle type after trips have been recorded. \
                Vehicle type is immutable once data exists."
                    .to_string(),
            );
        }
    }

    db.update_vehicle(&vehicle).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_vehicle(db: State<Database>, app_state: State<AppState>, id: String) -> Result<(), String> {
    check_read_only!(app_state);
    db.delete_vehicle(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_active_vehicle(db: State<Database>, app_state: State<AppState>, id: String) -> Result<(), String> {
    check_read_only!(app_state);
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
pub fn get_years_with_trips(
    db: State<Database>,
    vehicle_id: String,
) -> Result<Vec<i32>, String> {
    db.get_years_with_trips(&vehicle_id).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_trip(
    db: State<Database>,
    app_state: State<AppState>,
    vehicle_id: String,
    date: String,
    origin: String,
    destination: String,
    distance_km: f64,
    odometer: f64,
    purpose: String,
    // Fuel fields (ICE + PHEV)
    fuel_liters: Option<f64>,
    fuel_cost: Option<f64>,
    full_tank: Option<bool>,
    // Energy fields (BEV + PHEV)
    energy_kwh: Option<f64>,
    energy_cost_eur: Option<f64>,
    full_charge: Option<bool>,
    soc_override_percent: Option<f64>,
    // Other
    other_costs: Option<f64>,
    other_costs_note: Option<String>,
    insert_at_position: Option<i32>,
) -> Result<Trip, String> {
    check_read_only!(app_state);
    let vehicle_uuid = Uuid::parse_str(&vehicle_id).map_err(|e| e.to_string())?;
    let trip_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|e| e.to_string())?;

    // Normalize locations to prevent whitespace-based duplicates
    let origin = normalize_location(&origin);
    let destination = normalize_location(&destination);

    // Validate: SoC override must be 0-100 if provided
    if let Some(soc) = soc_override_percent {
        if !(0.0..=100.0).contains(&soc) {
            return Err("SoC override must be between 0 and 100".to_string());
        }
    }

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
        full_tank: full_tank.unwrap_or(true), // Default to full tank
        energy_kwh,
        energy_cost_eur,
        full_charge: full_charge.unwrap_or(false),
        soc_override_percent,
        other_costs_eur: other_costs,
        other_costs_note,
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
    app_state: State<AppState>,
    id: String,
    date: String,
    origin: String,
    destination: String,
    distance_km: f64,
    odometer: f64,
    purpose: String,
    // Fuel fields (ICE + PHEV)
    fuel_liters: Option<f64>,
    fuel_cost_eur: Option<f64>,
    full_tank: Option<bool>,
    // Energy fields (BEV + PHEV)
    energy_kwh: Option<f64>,
    energy_cost_eur: Option<f64>,
    full_charge: Option<bool>,
    soc_override_percent: Option<f64>,
    // Other
    other_costs_eur: Option<f64>,
    other_costs_note: Option<String>,
) -> Result<Trip, String> {
    check_read_only!(app_state);
    let trip_uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let trip_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|e| e.to_string())?;

    // Normalize locations to prevent whitespace-based duplicates
    let origin = normalize_location(&origin);
    let destination = normalize_location(&destination);

    // Validate: SoC override must be 0-100 if provided
    if let Some(soc) = soc_override_percent {
        if !(0.0..=100.0).contains(&soc) {
            return Err("SoC override must be between 0 and 100".to_string());
        }
    }

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
        full_tank: full_tank.unwrap_or(existing.full_tank),
        energy_kwh,
        energy_cost_eur,
        full_charge: full_charge.unwrap_or(existing.full_charge),
        soc_override_percent,
        other_costs_eur,
        other_costs_note,
        sort_order: existing.sort_order,
        created_at: existing.created_at,
        updated_at: Utc::now(),
    };

    db.update_trip(&trip).map_err(|e| e.to_string())?;

    // Update or create route for autocomplete (same as create_trip)
    db.find_or_create_route(&trip.vehicle_id.to_string(), &trip.origin, &trip.destination, distance_km)
        .map_err(|e| e.to_string())?;

    Ok(trip)
}

#[tauri::command]
pub fn delete_trip(db: State<Database>, app_state: State<AppState>, id: String) -> Result<(), String> {
    check_read_only!(app_state);
    db.delete_trip(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_trip(
    db: State<Database>,
    app_state: State<AppState>,
    trip_id: String,
    new_sort_order: i32,
) -> Result<Vec<Trip>, String> {
    check_read_only!(app_state);
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

#[tauri::command]
pub fn get_purposes(db: State<Database>, vehicle_id: String) -> Result<Vec<String>, String> {
    db.get_purposes_for_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())
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
    app_state: State<AppState>,
    company_name: String,
    company_ico: String,
    buffer_trip_purpose: String,
) -> Result<Settings, String> {
    check_read_only!(app_state);
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
    year: i32,
    db: State<Database>,
) -> Result<TripStats, String> {
    // Get vehicle (validate UUID format first)
    let _vehicle_uuid = Uuid::parse_str(&vehicle_id).map_err(|e| e.to_string())?;
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    // Extract fuel fields (ICE vehicles only for now)
    // TODO: Phase 2 will add BEV/PHEV handling based on vehicle.vehicle_type
    let tank_size = vehicle.tank_size_liters.unwrap_or_default();
    let tp_consumption = vehicle.tp_consumption.unwrap_or_default();

    // Get all trips for this vehicle, sorted by date + odometer (for same-day trips)
    let mut trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;
    trips.sort_by(|a, b| {
        a.date.cmp(&b.date).then_with(|| a.odometer.partial_cmp(&b.odometer).unwrap_or(std::cmp::Ordering::Equal))
    });

    // If no trips, return default values
    if trips.is_empty() {
        return Ok(TripStats {
            fuel_remaining_liters: tank_size,
            avg_consumption_rate: 0.0,
            last_consumption_rate: 0.0,
            margin_percent: None,
            is_over_limit: false,
            total_km: 0.0,
            total_fuel_liters: 0.0,
            total_fuel_cost_eur: 0.0,
            buffer_km: 0.0,
        });
    }

    // Calculate totals (all trips for display)
    let total_fuel: f64 = trips.iter().filter_map(|t| t.fuel_liters).sum();
    let total_fuel_cost: f64 = trips.iter().filter_map(|t| t.fuel_cost_eur).sum();
    let total_km: f64 = trips.iter().map(|t| t.distance_km).sum();

    // Calculate average consumption from CLOSED periods only (for accurate margin)
    let (closed_fuel, closed_km) = calculate_closed_period_totals(&trips);
    let avg_consumption_rate = if closed_km > 0.0 {
        (closed_fuel / closed_km) * 100.0
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
        tp_consumption
    };

    // Calculate current fuel level by processing all trips sequentially
    // Note: For accurate fuel level, we should use per-period rates, but for header display
    // we use the last consumption rate as a reasonable approximation
    // Start with carryover from previous year (or full tank if no previous data)
    let mut current_fuel = get_year_start_fuel_remaining(
        &db,
        &vehicle_id,
        year,
        tank_size,
        tp_consumption,
    )?;

    for trip in &trips {
        // Calculate fuel used for this trip
        let fuel_used = calculate_fuel_used(trip.distance_km, last_consumption_rate);

        // Update fuel level
        current_fuel = calculate_fuel_level(
            current_fuel,
            fuel_used,
            trip.fuel_liters,
            tank_size,
        );
    }

    // Check if over legal limit - ANY fill-up window must be within 120% of TP
    // (not just the average, since each window is separately auditable)
    // Use the WORST window's margin for display (that's what triggers the warning)
    let (worst_rate, worst_margin, is_over_limit) = if total_fuel > 0.0 {
        get_worst_period_stats(&trips, tp_consumption)
    } else {
        (0.0, 0.0, false)
    };

    // Calculate buffer km needed to reach 18% target margin for the worst period
    const TARGET_MARGIN: f64 = 0.18; // 18% - safe buffer below 20% legal limit
    let buffer_km = if is_over_limit {
        // Use worst period's fuel/km for buffer calculation
        // Since we track the worst rate, we can derive the needed buffer
        // Buffer = how much more km needed to bring worst_rate down to target
        // For simplicity, use closed totals (conservative estimate)
        calculate_buffer_km(closed_fuel, closed_km, tp_consumption, TARGET_MARGIN)
    } else {
        0.0
    };

    // Show the WORST window's margin (not average) - that's what triggers warnings
    let display_margin = if closed_km > 0.0 && worst_rate > 0.0 {
        Some(worst_margin)
    } else {
        None
    };

    Ok(TripStats {
        fuel_remaining_liters: current_fuel,
        avg_consumption_rate,
        last_consumption_rate,
        margin_percent: display_margin,
        is_over_limit,
        total_km,
        total_fuel_liters: total_fuel,
        total_fuel_cost_eur: total_fuel_cost,
        buffer_km,
    })
}

/// Get the starting fuel remaining for a year (carryover from previous year).
/// If there are trips in the previous year, returns the ending fuel state of that year.
/// Otherwise, returns full tank (initial state for the vehicle).
fn get_year_start_fuel_remaining(
    db: &Database,
    vehicle_id: &str,
    year: i32,
    tank_size: f64,
    tp_consumption: f64,
) -> Result<f64, String> {
    // Try to get trips from previous year
    let prev_year = year - 1;
    let prev_trips = db
        .get_trips_for_vehicle_in_year(vehicle_id, prev_year)
        .map_err(|e| e.to_string())?;

    if prev_trips.is_empty() {
        // No previous year data - start with full tank
        return Ok(tank_size);
    }

    // Sort previous year's trips chronologically
    let mut chronological = prev_trips;
    chronological.sort_by(|a, b| {
        a.date.cmp(&b.date).then_with(|| {
            a.odometer
                .partial_cmp(&b.odometer)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    // Calculate rates for previous year
    let (rates, _) = calculate_period_rates(&chronological, tp_consumption);

    // Get the starting fuel for the previous year (recursive carryover)
    let prev_year_start = get_year_start_fuel_remaining(db, vehicle_id, prev_year, tank_size, tp_consumption)?;

    // Calculate fuel remaining for each trip, then get the last one (year-end state)
    let fuel_remaining = calculate_fuel_remaining(&chronological, &rates, prev_year_start, tank_size);

    // Get the last trip's fuel remaining (year-end state)
    let last_trip_id = chronological.last().map(|t| t.id.to_string());
    let year_end_fuel = last_trip_id
        .and_then(|id| fuel_remaining.get(&id).copied())
        .unwrap_or(tank_size);

    Ok(year_end_fuel)
}

/// Get the starting battery (kWh) for a year (carryover from previous year).
/// If there are trips in the previous year, returns the ending battery of that year.
/// Otherwise, returns the vehicle's initial battery (initial_battery_percent × capacity).
fn get_year_start_battery_remaining(
    db: &Database,
    vehicle_id: &str,
    year: i32,
    vehicle: &Vehicle,
) -> Result<f64, String> {
    let capacity = vehicle.battery_capacity_kwh.unwrap_or(0.0);
    let baseline_rate = vehicle.baseline_consumption_kwh.unwrap_or(0.0);

    if capacity <= 0.0 {
        return Ok(0.0);
    }

    // Try to get trips from previous year
    let prev_year = year - 1;
    let prev_trips = db
        .get_trips_for_vehicle_in_year(vehicle_id, prev_year)
        .map_err(|e| e.to_string())?;

    if prev_trips.is_empty() {
        // No previous year data - start with initial battery
        let initial_percent = vehicle.initial_battery_percent.unwrap_or(100.0);
        return Ok(capacity * initial_percent / 100.0);
    }

    // Sort previous year's trips chronologically
    let mut chronological = prev_trips;
    chronological.sort_by(|a, b| {
        a.date.cmp(&b.date).then_with(|| {
            a.odometer
                .partial_cmp(&b.odometer)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    // Get the starting battery for the previous year (recursive carryover)
    let prev_year_start = get_year_start_battery_remaining(db, vehicle_id, prev_year, vehicle)?;

    // Calculate battery remaining for each trip, then get the last one (year-end state)
    let mut current_battery = prev_year_start;

    for trip in &chronological {
        // Check for SoC override
        if let Some(override_percent) = trip.soc_override_percent {
            current_battery = capacity * override_percent / 100.0;
        }

        // Calculate energy used
        let energy_used = calculate_energy_used(trip.distance_km, baseline_rate);

        // Update battery
        current_battery = calculate_battery_remaining(
            current_battery,
            energy_used,
            trip.energy_kwh,
            capacity,
        );
    }

    Ok(current_battery)
}

/// Get the starting odometer for a year (carryover from previous year).
/// If there are trips in the previous year, returns the ending odometer of that year.
/// Otherwise, returns the vehicle's initial odometer.
fn get_year_start_odometer(
    db: &Database,
    vehicle_id: &str,
    year: i32,
    initial_odometer: f64,
) -> Result<f64, String> {
    // Try to find trips from previous years (up to 10 years back)
    let min_year = year - 10;
    let mut check_year = year - 1;

    while check_year >= min_year {
        let trips = db
            .get_trips_for_vehicle_in_year(vehicle_id, check_year)
            .map_err(|e| e.to_string())?;

        if !trips.is_empty() {
            // Found trips - sort and get the last one's odometer
            let mut chronological = trips;
            chronological.sort_by(|a, b| {
                a.date.cmp(&b.date).then_with(|| {
                    a.odometer
                        .partial_cmp(&b.odometer)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            });
            return Ok(chronological.last().map(|t| t.odometer).unwrap_or(initial_odometer));
        }
        check_year -= 1;
    }

    // No data found in reasonable range - use vehicle's initial odometer
    Ok(initial_odometer)
}

/// Get pre-calculated trip grid data for frontend display.
/// This is the single source of truth for all grid calculations.
#[tauri::command]
pub fn get_trip_grid_data(
    db: State<Database>,
    vehicle_id: String,
    year: i32,
) -> Result<TripGridData, String> {
    // Get vehicle for TP consumption and tank size
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    // Get trips sorted by sort_order (for display)
    let trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;

    // Calculate year starting odometer (carryover from previous year)
    let year_start_odometer = get_year_start_odometer(
        &db,
        &vehicle_id,
        year,
        vehicle.initial_odometer,
    )?;

    if trips.is_empty() {
        return Ok(TripGridData {
            trips: vec![],
            rates: HashMap::new(),
            estimated_rates: HashSet::new(),
            fuel_remaining: HashMap::new(),
            consumption_warnings: HashSet::new(),
            energy_rates: HashMap::new(),
            estimated_energy_rates: HashSet::new(),
            battery_remaining_kwh: HashMap::new(),
            battery_remaining_percent: HashMap::new(),
            soc_override_trips: HashSet::new(),
            date_warnings: HashSet::new(),
            missing_receipts: HashSet::new(),
            year_start_odometer,
        });
    }

    // Get all receipts for matching
    let receipts = db.get_all_receipts().map_err(|e| e.to_string())?;

    // Sort chronologically for calculations (by date, then odometer)
    let mut chronological = trips.clone();
    chronological.sort_by(|a, b| {
        a.date.cmp(&b.date).then_with(|| {
            a.odometer
                .partial_cmp(&b.odometer)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    let tp_consumption = vehicle.tp_consumption.unwrap_or_default();
    let tank_size = vehicle.tank_size_liters.unwrap_or_default();

    // Calculate initial fuel (carryover from previous year or full tank)
    let initial_fuel = get_year_start_fuel_remaining(
        &db,
        &vehicle_id,
        year,
        tank_size,
        tp_consumption,
    )?;

    // Calculate date warnings (trips sorted by sort_order)
    let date_warnings = calculate_date_warnings(&trips);

    // Populate SoC override trips (works for all vehicle types)
    let soc_override_trips: HashSet<String> = trips
        .iter()
        .filter(|t| t.soc_override_percent.is_some())
        .map(|t| t.id.to_string())
        .collect();

    // Calculate missing receipts (trips with fuel but no matching receipt)
    let missing_receipts = calculate_missing_receipts(&trips, &receipts);

    // Calculate initial battery for BEV/PHEV (carryover from previous year)
    let initial_battery = if vehicle.vehicle_type.uses_electricity() {
        get_year_start_battery_remaining(&db, &vehicle_id, year, &vehicle)?
    } else {
        0.0
    };

    // Calculate rates, fuel, and energy based on vehicle type
    let (
        rates,
        estimated_rates,
        fuel_remaining,
        consumption_warnings,
        energy_rates,
        estimated_energy_rates,
        battery_remaining_kwh,
        battery_remaining_percent,
    ) = match vehicle.vehicle_type {
        VehicleType::Ice => {
            // ICE: Fuel calculations only
            let (rates, estimated_rates) =
                calculate_period_rates(&chronological, tp_consumption);
            let fuel_remaining =
                calculate_fuel_remaining(&chronological, &rates, initial_fuel, tank_size);
            let consumption_warnings =
                calculate_consumption_warnings(&trips, &rates, tp_consumption);
            (
                rates,
                estimated_rates,
                fuel_remaining,
                consumption_warnings,
                HashMap::new(),
                HashSet::new(),
                HashMap::new(),
                HashMap::new(),
            )
        }
        VehicleType::Bev => {
            // BEV: Energy calculations only, no fuel
            let (energy_rates, estimated_energy_rates, battery_kwh, battery_percent) =
                calculate_energy_grid_data(&chronological, &vehicle, initial_battery);
            (
                HashMap::new(),
                HashSet::new(),
                HashMap::new(),
                HashSet::new(), // No consumption warnings for BEV
                energy_rates,
                estimated_energy_rates,
                battery_kwh,
                battery_percent,
            )
        }
        VehicleType::Phev => {
            // PHEV: Both fuel and energy, using PHEV-specific calculations
            let phev_data = calculate_phev_grid_data(&chronological, &vehicle, initial_fuel, initial_battery);
            // Calculate consumption warnings for fuel portion only
            let consumption_warnings =
                calculate_consumption_warnings(&trips, &phev_data.fuel_rates, tp_consumption);
            (
                phev_data.fuel_rates,
                phev_data.estimated_fuel_rates,
                phev_data.fuel_remaining,
                consumption_warnings,
                phev_data.energy_rates,
                phev_data.estimated_energy_rates,
                phev_data.battery_remaining_kwh,
                phev_data.battery_remaining_percent,
            )
        }
    };

    Ok(TripGridData {
        trips,
        rates,
        estimated_rates,
        fuel_remaining,
        consumption_warnings,
        energy_rates,
        estimated_energy_rates,
        battery_remaining_kwh,
        battery_remaining_percent,
        soc_override_trips,
        date_warnings,
        missing_receipts,
        year_start_odometer,
    })
}

/// Get accumulated km in the current (open) fillup period.
/// Reuses the same logic as calculate_period_rates.
///
/// If `stop_at_trip_id` is provided, only count km up to and including that trip.
/// This is needed when editing a trip in the middle of a period.
fn get_open_period_km(chronological: &[Trip], stop_at_trip_id: Option<&Uuid>) -> f64 {
    let mut km_in_period = 0.0;

    // Same logic as calculate_period_rates - accumulate km until we find a full tank
    for trip in chronological {
        km_in_period += trip.distance_km;

        // If editing a specific trip, stop after we've counted it
        if let Some(stop_id) = stop_at_trip_id {
            if &trip.id == stop_id {
                break;
            }
        }

        if let Some(fuel) = trip.fuel_liters {
            if fuel > 0.0 && trip.full_tank {
                // Period closed by full tank - reset counter
                km_in_period = 0.0;
            }
        }
    }

    km_in_period
}

/// Calculate suggested fuel liters for magic fill feature.
/// Returns liters that would result in 105-120% of TP consumption rate.
///
/// Parameters:
/// - `current_trip_km`: The km value from the form (for new trips only)
/// - `editing_trip_id`: If editing an existing trip, pass its ID to avoid double-counting
#[tauri::command]
pub fn calculate_magic_fill_liters(
    db: State<Database>,
    vehicle_id: String,
    year: i32,
    current_trip_km: f64,
    editing_trip_id: Option<String>,
) -> Result<f64, String> {
    use rand::Rng;

    // Get vehicle for TP consumption
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    let tp_consumption = vehicle.tp_consumption.unwrap_or(5.0);

    // Get trips sorted chronologically (same as calculate_period_rates)
    let trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;

    let mut chronological = trips;
    chronological.sort_by(|a, b| {
        a.date.cmp(&b.date).then_with(|| {
            a.odometer
                .partial_cmp(&b.odometer)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    // Parse editing_trip_id to Uuid if provided
    let editing_uuid = editing_trip_id
        .as_ref()
        .and_then(|id| Uuid::parse_str(id).ok());

    // Get accumulated km in current open period (reuses period logic)
    // When editing, only count km up to the edited trip (not trips after it)
    let open_period_km = get_open_period_km(&chronological, editing_uuid.as_ref());

    // For existing trips: their km is already in open_period_km, don't add again
    // For new trips: add current_trip_km to the open period
    let total_km = if editing_uuid.is_some() {
        // Existing trip - km already counted in open_period_km
        open_period_km
    } else {
        // New trip - add its km
        open_period_km + current_trip_km
    };

    if total_km <= 0.0 {
        return Ok(0.0);
    }

    // Random target rate between 105-120% of TP consumption
    let mut rng = rand::thread_rng();
    let target_multiplier = rng.gen_range(1.05..=1.20);
    let target_rate = tp_consumption * target_multiplier;

    // Calculate liters: liters = km * rate / 100
    let suggested_liters = (total_km * target_rate) / 100.0;

    // Round to 2 decimal places
    Ok((suggested_liters * 100.0).round() / 100.0)
}

/// Calculate consumption rates for each trip based on fill-up periods.
/// Only calculates actual rate when a period ends with a full tank fillup.
pub(crate) fn calculate_period_rates(
    chronological: &[Trip],
    tp_consumption: f64,
) -> (HashMap<String, f64>, HashSet<String>) {
    let mut rates = HashMap::new();
    let mut estimated = HashSet::new();

    struct Period {
        trip_ids: Vec<String>,
        rate: f64,
        is_estimated: bool,
    }

    let mut periods: Vec<Period> = vec![];
    let mut current_trip_ids: Vec<String> = vec![];
    let mut km_in_period = 0.0;
    let mut fuel_in_period = 0.0;

    for trip in chronological {
        current_trip_ids.push(trip.id.to_string());
        km_in_period += trip.distance_km;

        if let Some(fuel) = trip.fuel_liters {
            if fuel > 0.0 {
                fuel_in_period += fuel;

                // Only close period on full tank fillup
                if trip.full_tank && km_in_period > 0.0 {
                    let rate = (fuel_in_period / km_in_period) * 100.0;
                    periods.push(Period {
                        trip_ids: current_trip_ids.clone(),
                        rate,
                        is_estimated: false,
                    });
                    current_trip_ids.clear();
                    km_in_period = 0.0;
                    fuel_in_period = 0.0;
                }
            }
        }
    }

    // Remaining trips use TP rate (estimated)
    if !current_trip_ids.is_empty() {
        periods.push(Period {
            trip_ids: current_trip_ids,
            rate: tp_consumption,
            is_estimated: true,
        });
    }

    // Assign rates to trips
    for period in periods {
        for trip_id in period.trip_ids {
            rates.insert(trip_id.clone(), period.rate);
            if period.is_estimated {
                estimated.insert(trip_id);
            }
        }
    }

    (rates, estimated)
}

/// Find the worst (highest) fill-up period's consumption rate and margin.
/// Returns (worst_rate, worst_margin, is_over_limit) for the period with highest consumption.
/// This is stricter than checking the average - for legal compliance,
/// each fill-up window must be within the limit, not just the total average.
fn get_worst_period_stats(trips: &[Trip], tp_consumption: f64) -> (f64, f64, bool) {
    if tp_consumption <= 0.0 {
        return (0.0, 0.0, false);
    }

    let limit = tp_consumption * 1.2; // 120% of TP
    let mut worst_rate = 0.0;
    let mut km_in_period = 0.0;
    let mut fuel_in_period = 0.0;

    for trip in trips {
        km_in_period += trip.distance_km;

        if let Some(fuel) = trip.fuel_liters {
            if fuel > 0.0 {
                fuel_in_period += fuel;

                // Calculate rate when period closes (full tank fillup)
                if trip.full_tank && km_in_period > 0.0 {
                    let rate = (fuel_in_period / km_in_period) * 100.0;
                    if rate > worst_rate {
                        worst_rate = rate;
                    }
                    // Reset for next period
                    km_in_period = 0.0;
                    fuel_in_period = 0.0;
                }
            }
        }
    }

    let worst_margin = calculate_margin_percent(worst_rate, tp_consumption);
    let is_over_limit = worst_rate > limit;

    (worst_rate, worst_margin, is_over_limit)
}

/// Check if any closed fill-up period exceeds the legal consumption limit.
/// Returns true if ANY period's consumption rate is > 120% of TP.
#[cfg(test)]
fn has_any_period_over_limit(trips: &[Trip], tp_consumption: f64) -> bool {
    let (_, _, is_over) = get_worst_period_stats(trips, tp_consumption);
    is_over
}

/// Calculate fuel remaining after each trip.
/// `initial_fuel` is the fuel level at the start of the period (carryover from previous year).
pub(crate) fn calculate_fuel_remaining(
    chronological: &[Trip],
    rates: &HashMap<String, f64>,
    initial_fuel: f64,
    tank_size: f64,
) -> HashMap<String, f64> {
    let mut remaining = HashMap::new();
    let mut fuel = initial_fuel;

    for trip in chronological {
        let trip_id = trip.id.to_string();
        let rate = rates.get(&trip_id).copied().unwrap_or(0.0);
        let fuel_used = if rate > 0.0 {
            (trip.distance_km * rate) / 100.0
        } else {
            0.0
        };

        fuel -= fuel_used;

        if let Some(fuel_added) = trip.fuel_liters {
            if fuel_added > 0.0 {
                if trip.full_tank {
                    fuel = tank_size;
                } else {
                    fuel += fuel_added;
                }
            }
        }

        // Clamp to valid range
        fuel = fuel.max(0.0).min(tank_size);
        remaining.insert(trip_id, fuel);
    }

    remaining
}

/// Calculate energy data for BEV vehicles.
/// Returns (energy_rates, estimated_energy_rates, battery_remaining_kwh, battery_remaining_percent)
fn calculate_energy_grid_data(
    chronological: &[Trip],
    vehicle: &Vehicle,
    initial_battery: f64,
) -> (
    HashMap<String, f64>,
    HashSet<String>,
    HashMap<String, f64>,
    HashMap<String, f64>,
) {
    let mut energy_rates = HashMap::new();
    let mut estimated_energy_rates = HashSet::new();
    let mut battery_kwh = HashMap::new();
    let mut battery_percent = HashMap::new();

    let capacity = vehicle.battery_capacity_kwh.unwrap_or(0.0);
    let baseline_rate = vehicle.baseline_consumption_kwh.unwrap_or(0.0);

    if capacity <= 0.0 {
        return (energy_rates, estimated_energy_rates, battery_kwh, battery_percent);
    }

    // Initial battery state: use year start carryover
    let mut current_battery = initial_battery;

    // Track charge periods for rate calculation (similar to fuel periods)
    let mut period_energy = 0.0;
    let mut period_km = 0.0;
    let mut period_trip_ids: Vec<String> = Vec::new();

    for trip in chronological {
        let trip_id = trip.id.to_string();

        // Check for SoC override - this resets the battery state
        if let Some(override_percent) = trip.soc_override_percent {
            current_battery = capacity * override_percent / 100.0;
        }

        // Calculate energy used for this trip
        let energy_used = calculate_energy_used(trip.distance_km, baseline_rate);

        // Update battery (subtract used, add charged)
        current_battery = calculate_battery_remaining(
            current_battery,
            energy_used,
            trip.energy_kwh,
            capacity,
        );

        // Store battery remaining
        battery_kwh.insert(trip_id.clone(), current_battery);
        battery_percent.insert(trip_id.clone(), kwh_to_percent(current_battery, capacity));

        // Track period for rate calculation
        period_km += trip.distance_km;
        period_trip_ids.push(trip_id.clone());

        // If this trip has a charge and is marked as full charge, close the period
        if trip.energy_kwh.is_some() && trip.full_charge {
            let charged = trip.energy_kwh.unwrap_or(0.0);
            period_energy += charged;

            // Calculate rate for this period
            let rate = if period_km > 0.0 {
                (period_energy / period_km) * 100.0
            } else {
                baseline_rate
            };

            // Apply rate to all trips in period
            for id in &period_trip_ids {
                energy_rates.insert(id.clone(), rate);
            }

            // Reset period
            period_energy = 0.0;
            period_km = 0.0;
            period_trip_ids.clear();
        } else if let Some(charged) = trip.energy_kwh {
            // Partial charge - accumulate but don't close period
            period_energy += charged;
        }
    }

    // Handle remaining trips without a full charge - use baseline rate (estimated)
    for id in &period_trip_ids {
        energy_rates.insert(id.clone(), baseline_rate);
        estimated_energy_rates.insert(id.clone());
    }

    (energy_rates, estimated_energy_rates, battery_kwh, battery_percent)
}

/// PHEV grid data calculation result
struct PhevGridData {
    /// Fuel consumption rates (l/100km) - only for km_on_fuel portion
    fuel_rates: HashMap<String, f64>,
    /// Trip IDs with estimated fuel rates
    estimated_fuel_rates: HashSet<String>,
    /// Fuel remaining after each trip (liters)
    fuel_remaining: HashMap<String, f64>,
    /// Energy consumption rates (kWh/100km)
    energy_rates: HashMap<String, f64>,
    /// Trip IDs with estimated energy rates
    estimated_energy_rates: HashSet<String>,
    /// Battery remaining (kWh)
    battery_remaining_kwh: HashMap<String, f64>,
    /// Battery remaining (%)
    battery_remaining_percent: HashMap<String, f64>,
}

/// Calculate PHEV grid data - tracks both fuel and battery state.
/// Uses electricity first until battery depleted, then fuel.
/// Fuel consumption rate is calculated only for the km driven on fuel.
fn calculate_phev_grid_data(
    chronological: &[Trip],
    vehicle: &Vehicle,
    initial_fuel: f64,
    initial_battery: f64,
) -> PhevGridData {
    let mut fuel_rates = HashMap::new();
    let mut estimated_fuel_rates = HashSet::new();
    let mut fuel_remaining = HashMap::new();
    let mut energy_rates = HashMap::new();
    let mut estimated_energy_rates = HashSet::new();
    let mut battery_kwh = HashMap::new();
    let mut battery_percent = HashMap::new();

    let capacity = vehicle.battery_capacity_kwh.unwrap_or(0.0);
    let baseline_energy = vehicle.baseline_consumption_kwh.unwrap_or(18.0); // kWh/100km
    let tp_consumption = vehicle.tp_consumption.unwrap_or(7.0); // l/100km
    let tank_size = vehicle.tank_size_liters.unwrap_or(50.0);

    // Initial battery state: use year start carryover
    let mut current_battery = initial_battery;
    let mut current_fuel = initial_fuel;

    // Fuel period tracking - only count km_on_fuel for rate calculation
    let mut fuel_period_km = 0.0;
    let mut fuel_period_liters = 0.0;
    let mut fuel_period_trip_ids: Vec<String> = Vec::new();

    // Energy period tracking
    let mut energy_period_km = 0.0;
    let mut energy_period_kwh = 0.0;
    let mut energy_period_trip_ids: Vec<String> = Vec::new();

    for trip in chronological {
        let trip_id = trip.id.to_string();

        // Check for SoC override
        if let Some(override_percent) = trip.soc_override_percent {
            current_battery = capacity * override_percent / 100.0;
        }

        // Use PHEV calculation to split km between electric and fuel
        let phev_result = calculate_phev_trip_consumption(
            trip.distance_km,
            current_battery,
            current_fuel,
            trip.energy_kwh,
            trip.fuel_liters,
            baseline_energy,
            tp_consumption,
            capacity,
            tank_size,
        );

        // Update state
        current_battery = phev_result.battery_remaining_kwh;
        current_fuel = phev_result.fuel_remaining_liters;

        // Store remaining values
        battery_kwh.insert(trip_id.clone(), current_battery);
        battery_percent.insert(trip_id.clone(), kwh_to_percent(current_battery, capacity));
        fuel_remaining.insert(trip_id.clone(), current_fuel);

        // Track fuel period (only km_on_fuel counts)
        if phev_result.km_on_fuel > 0.0 {
            fuel_period_km += phev_result.km_on_fuel;
            fuel_period_trip_ids.push(trip_id.clone());
        }

        // Track energy period (only km_on_electricity counts)
        if phev_result.km_on_electricity > 0.0 {
            energy_period_km += phev_result.km_on_electricity;
            energy_period_trip_ids.push(trip_id.clone());
        }

        // Close fuel period on full tank
        if trip.fuel_liters.is_some() && trip.full_tank {
            fuel_period_liters += trip.fuel_liters.unwrap_or(0.0);
            let rate = if fuel_period_km > 0.0 {
                (fuel_period_liters / fuel_period_km) * 100.0
            } else {
                tp_consumption
            };
            for id in &fuel_period_trip_ids {
                fuel_rates.insert(id.clone(), rate);
            }
            fuel_period_km = 0.0;
            fuel_period_liters = 0.0;
            fuel_period_trip_ids.clear();
        } else if let Some(liters) = trip.fuel_liters {
            fuel_period_liters += liters;
        }

        // Close energy period on full charge
        if trip.energy_kwh.is_some() && trip.full_charge {
            energy_period_kwh += trip.energy_kwh.unwrap_or(0.0);
            let rate = if energy_period_km > 0.0 {
                (energy_period_kwh / energy_period_km) * 100.0
            } else {
                baseline_energy
            };
            for id in &energy_period_trip_ids {
                energy_rates.insert(id.clone(), rate);
            }
            energy_period_km = 0.0;
            energy_period_kwh = 0.0;
            energy_period_trip_ids.clear();
        } else if let Some(kwh) = trip.energy_kwh {
            energy_period_kwh += kwh;
        }
    }

    // Handle remaining trips - use baseline rates (estimated)
    for id in &fuel_period_trip_ids {
        fuel_rates.insert(id.clone(), tp_consumption);
        estimated_fuel_rates.insert(id.clone());
    }
    for id in &energy_period_trip_ids {
        energy_rates.insert(id.clone(), baseline_energy);
        estimated_energy_rates.insert(id.clone());
    }

    PhevGridData {
        fuel_rates,
        estimated_fuel_rates,
        fuel_remaining,
        energy_rates,
        estimated_energy_rates,
        battery_remaining_kwh: battery_kwh,
        battery_remaining_percent: battery_percent,
    }
}

/// Check if each trip's date is out of order relative to neighbors.
/// Trips should be sorted by sort_order (0 = newest at top).
fn calculate_date_warnings(trips_by_sort_order: &[Trip]) -> HashSet<String> {
    let mut warnings = HashSet::new();

    for i in 0..trips_by_sort_order.len() {
        let trip = &trips_by_sort_order[i];
        let prev = if i > 0 {
            Some(&trips_by_sort_order[i - 1])
        } else {
            None
        };
        let next = if i < trips_by_sort_order.len() - 1 {
            Some(&trips_by_sort_order[i + 1])
        } else {
            None
        };

        // sort_order 0 = newest (should have highest date)
        // Check: prev.date >= trip.date >= next.date
        if let Some(p) = prev {
            if trip.date > p.date {
                warnings.insert(trip.id.to_string());
            }
        }
        if let Some(n) = next {
            if trip.date < n.date {
                warnings.insert(trip.id.to_string());
            }
        }
    }

    warnings
}

/// Check if any trip's consumption rate exceeds 120% of TP rate.
fn calculate_consumption_warnings(
    trips: &[Trip],
    rates: &HashMap<String, f64>,
    tp_consumption: f64,
) -> HashSet<String> {
    let mut warnings = HashSet::new();
    let limit = tp_consumption * 1.2;

    for trip in trips {
        let trip_id = trip.id.to_string();
        if let Some(&rate) = rates.get(&trip_id) {
            if rate > limit {
                warnings.insert(trip_id);
            }
        }
    }

    warnings
}

/// Find trips with fuel that don't have a matching receipt.
/// A trip has a matching receipt if date, liters, and price all match exactly.
/// Trips without fuel don't need receipts.
fn calculate_missing_receipts(trips: &[Trip], receipts: &[Receipt]) -> HashSet<String> {
    let mut missing = HashSet::new();

    for trip in trips {
        // Trips without fuel don't need receipts
        if trip.fuel_liters.is_none() {
            continue;
        }

        // Check if any receipt matches this trip exactly
        let has_match = receipts.iter().any(|r| {
            let date_match = r.receipt_date.as_ref() == Some(&trip.date);
            let liters_match = r.liters == trip.fuel_liters;
            let price_match = r.total_price_eur == trip.fuel_cost_eur;
            date_match && liters_match && price_match
        });

        if !has_match {
            missing.insert(trip.id.to_string());
        }
    }

    missing
}

// ============================================================================
// Backup Commands
// ============================================================================

/// Parse backup filename to extract type and version
/// Manual: kniha-jazd-backup-2026-01-24-143022.db
/// Pre-update: kniha-jazd-backup-2026-01-24-143022-pre-v0.20.0.db
fn parse_backup_filename(filename: &str) -> (String, Option<String>) {
    if filename.starts_with("kniha-jazd-backup-") {
        let without_prefix = filename.trim_start_matches("kniha-jazd-backup-");
        if let Some(version_start) = without_prefix.find("-pre-v") {
            let version = without_prefix[version_start + 6..].trim_end_matches(".db");
            return ("pre-update".to_string(), Some(version.to_string()));
        }
    }
    ("manual".to_string(), None)
}

/// Generate backup filename based on type and version
fn generate_backup_filename(backup_type: &str, update_version: Option<&str>) -> String {
    let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
    match (backup_type, update_version) {
        ("pre-update", Some(version)) => format!("kniha-jazd-backup-{}-pre-v{}.db", timestamp, version),
        _ => format!("kniha-jazd-backup-{}.db", timestamp),
    }
}

/// Get pre-update backups that should be deleted based on keep_count
/// Returns the oldest backups beyond the keep limit (manual backups are never included)
fn get_cleanup_candidates(backups: &[BackupInfo], keep_count: u32) -> Vec<BackupInfo> {
    // Filter to pre-update only
    let mut pre_update_backups: Vec<&BackupInfo> = backups
        .iter()
        .filter(|b| b.backup_type == "pre-update")
        .collect();

    // Sort by filename (which includes timestamp) - oldest first
    pre_update_backups.sort_by(|a, b| a.filename.cmp(&b.filename));

    let total = pre_update_backups.len();
    let keep = keep_count as usize;

    if total <= keep {
        return vec![];
    }

    // Return the oldest ones (to delete)
    pre_update_backups[0..(total - keep)]
        .iter()
        .map(|b| (*b).clone())
        .collect()
}

#[tauri::command]
pub fn create_backup(app: tauri::AppHandle, db: State<Database>, app_state: State<AppState>) -> Result<BackupInfo, String> {
    check_read_only!(app_state);
    let db_paths = get_db_paths(&app)?;

    // Create backup directory if it doesn't exist
    fs::create_dir_all(&db_paths.backups_dir).map_err(|e| e.to_string())?;

    // Generate backup filename with timestamp
    let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
    let filename = format!("kniha-jazd-backup-{}.db", timestamp);
    let backup_path = db_paths.backups_dir.join(&filename);

    // Copy current database to backup
    fs::copy(&db_paths.db_file, &backup_path).map_err(|e| e.to_string())?;

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
        backup_type: "manual".to_string(),
        update_version: None,
    })
}

/// Create backup with explicit type and optional version
/// Used for pre-update backups that need to record the target version
#[tauri::command]
pub fn create_backup_with_type(
    app: tauri::AppHandle,
    db: State<Database>,
    app_state: State<AppState>,
    backup_type: String,
    update_version: Option<String>,
) -> Result<BackupInfo, String> {
    check_read_only!(app_state);
    let db_paths = get_db_paths(&app)?;

    // Create backup directory if it doesn't exist
    fs::create_dir_all(&db_paths.backups_dir).map_err(|e| e.to_string())?;

    // Generate backup filename with type and version
    let filename = generate_backup_filename(&backup_type, update_version.as_deref());
    let backup_path = db_paths.backups_dir.join(&filename);

    // Copy current database to backup
    fs::copy(&db_paths.db_file, &backup_path).map_err(|e| e.to_string())?;

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

    // Parse the type back from filename to ensure consistency
    let (parsed_type, parsed_version) = parse_backup_filename(&filename);

    Ok(BackupInfo {
        filename,
        created_at: Local::now().to_rfc3339(),
        size_bytes: metadata.len(),
        vehicle_count,
        trip_count,
        backup_type: parsed_type,
        update_version: parsed_version,
    })
}

/// Get preview of pre-update backups that would be deleted
#[tauri::command]
pub fn get_cleanup_preview(app: tauri::AppHandle, keep_count: u32) -> Result<CleanupPreview, String> {
    let all_backups = list_backups(app)?;
    let to_delete = get_cleanup_candidates(&all_backups, keep_count);
    let total_bytes: u64 = to_delete.iter().map(|b| b.size_bytes).sum();

    Ok(CleanupPreview { to_delete, total_bytes })
}

/// Delete old pre-update backups, keeping the N most recent
#[tauri::command]
pub fn cleanup_pre_update_backups(
    app: tauri::AppHandle,
    app_state: State<AppState>,
    keep_count: u32,
) -> Result<CleanupResult, String> {
    check_read_only!(app_state);
    cleanup_pre_update_backups_internal(&app, keep_count)
}

/// Internal cleanup function for use at startup (no State parameters needed)
pub fn cleanup_pre_update_backups_internal(
    app: &tauri::AppHandle,
    keep_count: u32,
) -> Result<CleanupResult, String> {
    let db_paths = get_db_paths(app)?;

    let all_backups = list_backups(app.clone())?;
    let to_delete = get_cleanup_candidates(&all_backups, keep_count);

    let mut deleted = Vec::new();
    let mut freed_bytes = 0u64;

    for backup in &to_delete {
        let path = db_paths.backups_dir.join(&backup.filename);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
            deleted.push(backup.filename.clone());
            freed_bytes += backup.size_bytes;
        }
    }

    Ok(CleanupResult { deleted, freed_bytes })
}

/// Get backup retention settings
#[tauri::command]
pub fn get_backup_retention(app: tauri::AppHandle) -> Result<Option<BackupRetention>, String> {
    let app_dir = get_app_data_dir(&app)?;
    let settings = LocalSettings::load(&app_dir);
    Ok(settings.backup_retention)
}

/// Set backup retention settings
#[tauri::command]
pub fn set_backup_retention(
    app: tauri::AppHandle,
    app_state: State<AppState>,
    retention: BackupRetention,
) -> Result<(), String> {
    check_read_only!(app_state);
    let app_dir = get_app_data_dir(&app)?;
    let mut settings = LocalSettings::load(&app_dir);
    settings.backup_retention = Some(retention);
    settings.save(&app_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_backups(app: tauri::AppHandle) -> Result<Vec<BackupInfo>, String> {
    let db_paths = get_db_paths(&app)?;

    if !db_paths.backups_dir.exists() {
        return Ok(vec![]);
    }

    let mut backups = Vec::new();

    for entry in fs::read_dir(&db_paths.backups_dir).map_err(|e| e.to_string())? {
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

            // Parse backup type and version from filename
            let (backup_type, update_version) = parse_backup_filename(&filename);

            // We can't easily get counts from backup without opening it
            // So we'll return 0 for now - the get_backup_info command will show actual counts
            backups.push(BackupInfo {
                filename,
                created_at,
                size_bytes: metadata.len(),
                vehicle_count: 0,
                trip_count: 0,
                backup_type,
                update_version,
            });
        }
    }

    // Sort by filename descending (newest first)
    backups.sort_by(|a, b| b.filename.cmp(&a.filename));

    Ok(backups)
}

#[tauri::command]
pub fn get_backup_info(app: tauri::AppHandle, filename: String) -> Result<BackupInfo, String> {
    let db_paths = get_db_paths(&app)?;
    let backup_path = db_paths.backups_dir.join(&filename);

    if !backup_path.exists() {
        return Err(format!("Backup not found: {}", filename));
    }

    let metadata = fs::metadata(&backup_path).map_err(|e| e.to_string())?;

    // Open backup database to get counts using Diesel
    let backup_db = crate::db::Database::from_path(&backup_path).map_err(|e| e.to_string())?;
    let conn = &mut *backup_db.connection();

    // Use raw SQL for simple count queries
    #[derive(diesel::QueryableByName)]
    struct CountRow {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        count: i32,
    }

    let vehicle_count: i32 = diesel::sql_query("SELECT COUNT(*) as count FROM vehicles")
        .get_result::<CountRow>(conn)
        .map(|r| r.count)
        .unwrap_or(0);

    let trip_count: i32 = diesel::sql_query("SELECT COUNT(*) as count FROM trips")
        .get_result::<CountRow>(conn)
        .map(|r| r.count)
        .unwrap_or(0);

    // Parse timestamp from filename
    let created_at = if filename.starts_with("kniha-jazd-backup-") {
        let date_part = filename
            .trim_start_matches("kniha-jazd-backup-")
            .trim_end_matches(".db");
        // Handle pre-update suffix: -pre-vX.X.X
        let date_part = if let Some(pred_pos) = date_part.find("-pre-v") {
            &date_part[..pred_pos]
        } else {
            date_part
        };
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

    // Parse backup type and version from filename
    let (backup_type, update_version) = parse_backup_filename(&filename);

    Ok(BackupInfo {
        filename,
        created_at,
        size_bytes: metadata.len(),
        vehicle_count,
        trip_count,
        backup_type,
        update_version,
    })
}

#[tauri::command]
pub fn restore_backup(app: tauri::AppHandle, app_state: State<AppState>, filename: String) -> Result<(), String> {
    check_read_only!(app_state);
    let db_paths = get_db_paths(&app)?;
    let backup_path = db_paths.backups_dir.join(&filename);

    if !backup_path.exists() {
        return Err(format!("Backup not found: {}", filename));
    }

    // Copy backup over current database
    fs::copy(&backup_path, &db_paths.db_file).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_backup(app: tauri::AppHandle, app_state: State<AppState>, filename: String) -> Result<(), String> {
    check_read_only!(app_state);
    let db_paths = get_db_paths(&app)?;
    let backup_path = db_paths.backups_dir.join(&filename);

    if !backup_path.exists() {
        return Err(format!("Backup not found: {}", filename));
    }

    fs::remove_file(&backup_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_backup_path(app: tauri::AppHandle, filename: String) -> Result<String, String> {
    let db_paths = get_db_paths(&app)?;
    let backup_path = db_paths.backups_dir.join(&filename);

    if !backup_path.exists() {
        return Err(format!("Backup not found: {}", filename));
    }

    backup_path
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid path encoding".to_string())
}

// ============================================================================
// HTML Export Commands
// ============================================================================

#[tauri::command]
pub async fn export_to_browser(
    _app: tauri::AppHandle,
    db: State<'_, Database>,
    vehicle_id: String,
    year: i32,
    license_plate: String,
    sort_column: String,
    sort_direction: String,
    labels: ExportLabels,
) -> Result<(), String> {
    // Get vehicle
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    // Get settings
    let settings = db
        .get_settings()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Settings not found - please configure company info first".to_string())?;

    // Get trips
    let mut trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;

    // Create synthetic "Prvý záznam" (first record) trip
    let first_record_date = NaiveDate::from_ymd_opt(year, 1, 1)
        .ok_or_else(|| "Invalid year".to_string())?;
    let first_record = Trip {
        id: Uuid::nil(), // Special marker for first record
        vehicle_id: vehicle.id,
        date: first_record_date,
        origin: "-".to_string(),
        destination: "-".to_string(),
        distance_km: 0.0,
        odometer: vehicle.initial_odometer,
        purpose: "Prvý záznam".to_string(),
        fuel_liters: None,
        fuel_cost_eur: None,
        full_tank: true,
        // Energy fields (BEV/PHEV) - not applicable to first record
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        sort_order: 999999, // Always last in manual sort
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    trips.push(first_record);

    // Sort chronologically for calculations (excluding first record which has 0 km)
    let mut chronological: Vec<_> = trips.iter()
        .filter(|t| t.id != Uuid::nil())
        .cloned()
        .collect();
    chronological.sort_by(|a, b| {
        a.date.cmp(&b.date).then_with(|| {
            a.odometer
                .partial_cmp(&b.odometer)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    // Apply user's sort settings for display (including first record)
    let is_ascending = sort_direction == "asc";
    if sort_column == "date" {
        trips.sort_by(|a, b| {
            let cmp = a.date.cmp(&b.date);
            if is_ascending { cmp } else { cmp.reverse() }
        });
    } else {
        // manual sort by sort_order
        trips.sort_by(|a, b| {
            let cmp = a.sort_order.cmp(&b.sort_order);
            if is_ascending { cmp } else { cmp.reverse() }
        });
    }

    // Calculate rates and fuel remaining (ICE vehicles only for now)
    // TODO: Phase 2 will add BEV/PHEV handling based on vehicle.vehicle_type
    let tp_consumption = vehicle.tp_consumption.unwrap_or_default();
    let tank_size = vehicle.tank_size_liters.unwrap_or_default();
    let baseline_consumption_kwh = vehicle.baseline_consumption_kwh.unwrap_or_default();

    let (rates, estimated_rates) =
        calculate_period_rates(&chronological, tp_consumption);

    // Get initial fuel (carryover from previous year)
    let initial_fuel = get_year_start_fuel_remaining(
        &db,
        &vehicle_id,
        year,
        tank_size,
        tp_consumption,
    )?;

    // Get year starting odometer (carryover from previous year)
    let year_start_odometer = get_year_start_odometer(
        &db,
        &vehicle_id,
        year,
        vehicle.initial_odometer,
    )?;

    let fuel_remaining =
        calculate_fuel_remaining(&chronological, &rates, initial_fuel, tank_size);

    let grid_data = TripGridData {
        trips,
        rates,
        estimated_rates,
        fuel_remaining,
        consumption_warnings: HashSet::new(),
        energy_rates: HashMap::new(),
        estimated_energy_rates: HashSet::new(),
        battery_remaining_kwh: HashMap::new(),
        battery_remaining_percent: HashMap::new(),
        soc_override_trips: HashSet::new(),
        date_warnings: HashSet::new(),
        missing_receipts: HashSet::new(),
        year_start_odometer,
    };

    let totals = ExportTotals::calculate(&chronological, tp_consumption, baseline_consumption_kwh);

    let export_data = ExportData {
        vehicle,
        settings,
        grid_data,
        year,
        totals,
        labels,
    };

    let html = generate_html(export_data)?;

    // Write to temp file
    let temp_dir = std::env::temp_dir();
    let filename = format!("kniha-jazd-{}-{}.html", license_plate, year);
    let temp_path = temp_dir.join(&filename);

    fs::write(&temp_path, html).map_err(|e| format!("Failed to write temp file: {}", e))?;

    // Open in default browser
    open::that(&temp_path).map_err(|e| format!("Failed to open browser: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn export_html(
    db: State<'_, Database>,
    vehicle_id: String,
    year: i32,
    labels: ExportLabels,
) -> Result<String, String> {
    // Get vehicle
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    // Get settings
    let settings = db
        .get_settings()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Settings not found - please configure company info first".to_string())?;

    // Get trip grid data (reuses existing calculation logic)
    let trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;

    if trips.is_empty() {
        return Err("No trips found for this year".to_string());
    }

    // Sort chronologically for calculations
    let mut chronological = trips.clone();
    chronological.sort_by(|a, b| {
        a.date.cmp(&b.date).then_with(|| {
            a.odometer
                .partial_cmp(&b.odometer)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    // Calculate rates and fuel remaining (ICE vehicles only for now)
    // TODO: Phase 2 will add BEV/PHEV handling based on vehicle.vehicle_type
    let tp_consumption = vehicle.tp_consumption.unwrap_or_default();
    let tank_size = vehicle.tank_size_liters.unwrap_or_default();
    let baseline_consumption_kwh = vehicle.baseline_consumption_kwh.unwrap_or_default();

    let (rates, estimated_rates) =
        calculate_period_rates(&chronological, tp_consumption);

    // Get initial fuel (carryover from previous year)
    let initial_fuel = get_year_start_fuel_remaining(
        &db,
        &vehicle_id,
        year,
        tank_size,
        tp_consumption,
    )?;

    // Get year starting odometer (carryover from previous year)
    let year_start_odometer = get_year_start_odometer(
        &db,
        &vehicle_id,
        year,
        vehicle.initial_odometer,
    )?;

    let fuel_remaining =
        calculate_fuel_remaining(&chronological, &rates, initial_fuel, tank_size);

    let grid_data = TripGridData {
        trips,
        rates,
        estimated_rates,
        fuel_remaining,
        consumption_warnings: HashSet::new(),
        energy_rates: HashMap::new(),
        estimated_energy_rates: HashSet::new(),
        battery_remaining_kwh: HashMap::new(),
        battery_remaining_percent: HashMap::new(),
        soc_override_trips: HashSet::new(),
        date_warnings: HashSet::new(),
        missing_receipts: HashSet::new(),
        year_start_odometer,
    };

    // Calculate totals for footer
    let totals = ExportTotals::calculate(&chronological, tp_consumption, baseline_consumption_kwh);

    // Generate HTML
    let export_data = ExportData {
        vehicle,
        settings,
        grid_data,
        year,
        totals,
        labels,
    };

    generate_html(export_data)
}

// ============================================================================
// Receipt Commands
// ============================================================================

use crate::gemini::is_mock_mode_enabled;
use crate::models::{Receipt, ReceiptStatus, ReceiptVerification, VerificationResult};
use crate::receipts::{detect_folder_structure, process_receipt_with_gemini, scan_folder_for_new_receipts, FolderStructure};
use crate::settings::BackupRetention;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptSettings {
    pub gemini_api_key: Option<String>,
    pub receipts_folder_path: Option<String>,
    pub gemini_api_key_from_override: bool,
    pub receipts_folder_from_override: bool,
}

#[tauri::command]
pub fn get_receipt_settings(app: tauri::AppHandle) -> Result<ReceiptSettings, String> {
    let app_dir = get_app_data_dir(&app)?;
    let local = LocalSettings::load(&app_dir);

    Ok(ReceiptSettings {
        gemini_api_key: local.gemini_api_key.clone(),
        receipts_folder_path: local.receipts_folder_path.clone(),
        gemini_api_key_from_override: local.gemini_api_key.is_some(),
        receipts_folder_from_override: local.receipts_folder_path.is_some(),
    })
}

/// Get receipts, optionally filtered by year.
/// - If year is provided: returns receipts for that year (by receipt_date, or source_year if date is None)
/// - If year is None: returns all receipts (for backward compatibility)
#[tauri::command]
pub fn get_receipts(db: State<Database>, year: Option<i32>) -> Result<Vec<Receipt>, String> {
    match year {
        Some(y) => db.get_receipts_for_year(y).map_err(|e| e.to_string()),
        None => db.get_all_receipts().map_err(|e| e.to_string()),
    }
}

/// Get receipts filtered by vehicle - returns unassigned receipts + receipts for specified vehicle.
/// Optionally filter by year.
#[tauri::command]
pub fn get_receipts_for_vehicle(
    db: State<Database>,
    vehicle_id: String,
    year: Option<i32>,
) -> Result<Vec<Receipt>, String> {
    let vehicle_uuid =
        Uuid::parse_str(&vehicle_id).map_err(|e| format!("Invalid vehicle ID: {}", e))?;
    db.get_receipts_for_vehicle(&vehicle_uuid, year)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_unassigned_receipts(db: State<Database>) -> Result<Vec<Receipt>, String> {
    db.get_unassigned_receipts().map_err(|e| e.to_string())
}

/// Result of sync operation - includes both successes and errors
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub processed: Vec<Receipt>,
    pub errors: Vec<SyncError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncError {
    pub file_name: String,
    pub error: String,
}

/// Result of scanning folder for new receipts (no OCR)
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub new_count: usize,
    pub warning: Option<String>,
}

/// Scan folder for new receipts without OCR processing
/// Returns count of new files found and any folder structure warnings
#[tauri::command]
pub fn scan_receipts(app: tauri::AppHandle, db: State<'_, Database>, app_state: State<'_, AppState>) -> Result<ScanResult, String> {
    check_read_only!(app_state);
    let app_dir = get_app_data_dir(&app)?;
    let settings = LocalSettings::load(&app_dir);

    let folder_path = settings.receipts_folder_path
        .ok_or("Receipts folder not configured")?;

    // Scan for new files (this also inserts them into DB as Pending)
    let new_receipts = scan_folder_for_new_receipts(&folder_path, &db)?;

    // Check folder structure for warnings
    let structure = detect_folder_structure(&folder_path);
    let warning = match structure {
        FolderStructure::Invalid(msg) => Some(msg),
        _ => None,
    };

    Ok(ScanResult {
        new_count: new_receipts.len(),
        warning,
    })
}

#[tauri::command]
pub async fn sync_receipts(app: tauri::AppHandle, db: State<'_, Database>, app_state: State<'_, AppState>) -> Result<SyncResult, String> {
    check_read_only!(app_state);
    let app_dir = get_app_data_dir(&app)?;
    let settings = LocalSettings::load(&app_dir);

    let folder_path = settings.receipts_folder_path
        .ok_or("Receipts folder not configured")?;

    // In mock mode, API key is not required (extract_from_image loads from JSON files)
    let api_key = if is_mock_mode_enabled() {
        String::new()
    } else {
        settings.gemini_api_key
            .ok_or("Gemini API key not configured")?
    };

    // Scan for new files
    let mut new_receipts = scan_folder_for_new_receipts(&folder_path, &db)?;
    let mut errors = Vec::new();

    // Process each new receipt with Gemini (async)
    for receipt in &mut new_receipts {
        if let Err(e) = process_receipt_with_gemini(receipt, &api_key).await {
            log::warn!("Failed to process receipt {}: {}", receipt.file_name, e);
            errors.push(SyncError {
                file_name: receipt.file_name.clone(),
                error: e,
            });
        }
        // Update in DB regardless of success/failure
        db.update_receipt(receipt).map_err(|e| e.to_string())?;
    }

    Ok(SyncResult {
        processed: new_receipts,
        errors,
    })
}

#[derive(Clone, Serialize)]
pub struct ProcessingProgress {
    pub current: usize,
    pub total: usize,
    pub file_name: String,
}

#[tauri::command]
pub async fn process_pending_receipts(
    app: tauri::AppHandle,
    db: State<'_, Database>,
) -> Result<SyncResult, String> {
    let app_dir = get_app_data_dir(&app)?;
    let settings = LocalSettings::load(&app_dir);

    // In mock mode, API key is not required (extract_from_image loads from JSON files)
    let api_key = if is_mock_mode_enabled() {
        String::new()
    } else {
        settings.gemini_api_key
            .ok_or("Gemini API key not configured")?
    };

    // Get all pending receipts
    let mut pending_receipts = db.get_pending_receipts().map_err(|e| e.to_string())?;
    let mut errors = Vec::new();
    let total = pending_receipts.len();

    // Process each pending receipt with Gemini
    for (index, receipt) in pending_receipts.iter_mut().enumerate() {
        // Emit progress event
        let _ = app.emit("receipt-processing-progress", ProcessingProgress {
            current: index + 1,
            total,
            file_name: receipt.file_name.clone(),
        });

        match process_receipt_with_gemini(receipt, &api_key).await {
            Ok(()) => {
                // Only update DB on success
                db.update_receipt(receipt).map_err(|e| e.to_string())?;
            }
            Err(e) => {
                log::warn!("Failed to process receipt {}: {}", receipt.file_name, e);
                errors.push(SyncError {
                    file_name: receipt.file_name.clone(),
                    error: e,
                });
                // Don't update DB - leave receipt in Pending state for retry
            }
        }
    }

    Ok(SyncResult {
        processed: pending_receipts,
        errors,
    })
}

#[tauri::command]
pub fn update_receipt(db: State<Database>, app_state: State<AppState>, receipt: Receipt) -> Result<(), String> {
    check_read_only!(app_state);
    db.update_receipt(&receipt).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_receipt(db: State<Database>, app_state: State<AppState>, id: String) -> Result<(), String> {
    check_read_only!(app_state);
    db.delete_receipt(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reprocess_receipt(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    app_state: State<'_, AppState>,
    id: String,
) -> Result<Receipt, String> {
    check_read_only!(app_state);
    let app_dir = get_app_data_dir(&app)?;
    let settings = LocalSettings::load(&app_dir);

    // In mock mode, API key is not required (extract_from_image loads from JSON files)
    let api_key = if is_mock_mode_enabled() {
        String::new()
    } else {
        settings.gemini_api_key
            .ok_or("Gemini API key not configured")?
    };

    let mut receipt = db.get_receipt_by_id(&id)
        .map_err(|e| e.to_string())?
        .ok_or("Receipt not found")?;

    // Clear previous error and reprocess
    receipt.error_message = None;

    // Process with async Gemini API
    if let Err(e) = process_receipt_with_gemini(&mut receipt, &api_key).await {
        receipt.error_message = Some(e.clone());
        receipt.status = ReceiptStatus::NeedsReview;
    }

    db.update_receipt(&receipt).map_err(|e| e.to_string())?;
    Ok(receipt)
}

/// Internal assign_receipt_to_trip logic (testable without State wrapper)
pub fn assign_receipt_to_trip_internal(
    db: &Database,
    receipt_id: &str,
    trip_id: &str,
    vehicle_id: &str,
) -> Result<Receipt, String> {
    let mut receipts = db.get_all_receipts().map_err(|e| e.to_string())?;
    let receipt = receipts
        .iter_mut()
        .find(|r| r.id.to_string() == receipt_id)
        .ok_or("Receipt not found")?;

    let trip_uuid = Uuid::parse_str(trip_id).map_err(|e| e.to_string())?;
    let vehicle_uuid = Uuid::parse_str(vehicle_id).map_err(|e| e.to_string())?;

    let trip = db
        .get_trip(trip_id)
        .map_err(|e| e.to_string())?
        .ok_or("Trip not found")?;

    // Multi-stage matching: determine if this is FUEL or OTHER COST
    let is_fuel_match = match (receipt.liters, receipt.total_price_eur) {
        (Some(liters), Some(price)) => {
            // Has liters + price → check if it matches trip's fuel entry
            let date_match = receipt.receipt_date == Some(trip.date);
            let liters_match = trip
                .fuel_liters
                .map(|fl| (fl - liters).abs() < 0.01)
                .unwrap_or(false);
            let price_match = trip
                .fuel_cost_eur
                .map(|fc| (fc - price).abs() < 0.01)
                .unwrap_or(false);
            date_match && liters_match && price_match
        }
        _ => false, // No liters or no price → cannot be fuel
    };

    if is_fuel_match {
        // FUEL: existing behavior (just link receipt to trip)
        // Trip fuel fields already populated by user - receipt is verification
    } else {
        // OTHER COST: populate trip.other_costs_* fields
        // (even if receipt has liters - e.g., washer fluid that doesn't match any fuel entry)

        // Check for collision
        if trip.other_costs_eur.is_some() {
            return Err("Jazda už má iné náklady".to_string());
        }

        // Build note from receipt data
        let note = match (&receipt.vendor_name, &receipt.cost_description) {
            (Some(v), Some(d)) => format!("{}: {}", v, d),
            (Some(v), None) => v.clone(),
            (None, Some(d)) => d.clone(),
            (None, None) => "Iné náklady".to_string(),
        };

        // Update trip with other costs
        let mut updated_trip = trip.clone();
        updated_trip.other_costs_eur = receipt.total_price_eur;
        updated_trip.other_costs_note = Some(note);
        db.update_trip(&updated_trip).map_err(|e| e.to_string())?;
    }

    // Mark receipt as assigned (same for both types)
    receipt.trip_id = Some(trip_uuid);
    receipt.vehicle_id = Some(vehicle_uuid);
    receipt.status = ReceiptStatus::Assigned;
    db.update_receipt(receipt).map_err(|e| e.to_string())?;

    Ok(receipt.clone())
}

#[tauri::command]
pub fn assign_receipt_to_trip(
    db: State<Database>,
    app_state: State<AppState>,
    receipt_id: String,
    trip_id: String,
    vehicle_id: String,
) -> Result<Receipt, String> {
    check_read_only!(app_state);
    assign_receipt_to_trip_internal(&db, &receipt_id, &trip_id, &vehicle_id)
}

// ============================================================================
// Trip Selection for Receipt Assignment
// ============================================================================

/// A trip annotated with whether a receipt can be attached to it.
/// Used by the frontend to show which trips are eligible for receipt assignment.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TripForAssignment {
    pub trip: Trip,
    /// Whether this receipt can be attached to this trip
    pub can_attach: bool,
    /// Status explaining why: "empty" (no fuel), "matches" (receipt matches trip fuel), "differs" (data conflicts)
    pub attachment_status: String,
    /// When status is "differs", explains what specifically doesn't match (for UI display)
    /// Values: null, "date", "liters", "price", "liters_and_price", "date_and_liters", "date_and_price", "all"
    pub mismatch_reason: Option<String>,
}

/// Result of checking receipt-trip compatibility
struct CompatibilityResult {
    can_attach: bool,
    status: String,
    mismatch_reason: Option<String>,
}

/// Check if receipt data matches trip's existing fuel data.
/// Returns compatibility result with detailed mismatch reason.
fn check_receipt_trip_compatibility(receipt: &Receipt, trip: &Trip) -> CompatibilityResult {
    // No fuel data on trip → can attach (receipt will populate fuel fields)
    let trip_has_fuel = trip.fuel_liters.map(|l| l > 0.0).unwrap_or(false);
    if !trip_has_fuel {
        return CompatibilityResult {
            can_attach: true,
            status: "empty".to_string(),
            mismatch_reason: None,
        };
    }

    // Trip has fuel data - check if receipt matches
    match (receipt.liters, receipt.total_price_eur) {
        (Some(r_liters), Some(r_price)) => {
            // Receipt has fuel data - compare with trip
            let date_match = receipt.receipt_date == Some(trip.date);
            let liters_match = trip
                .fuel_liters
                .map(|fl| (fl - r_liters).abs() < 0.01)
                .unwrap_or(false);
            let price_match = trip
                .fuel_cost_eur
                .map(|fc| (fc - r_price).abs() < 0.01)
                .unwrap_or(false);

            if date_match && liters_match && price_match {
                CompatibilityResult {
                    can_attach: true,
                    status: "matches".to_string(),
                    mismatch_reason: None,
                }
            } else {
                // Determine what specifically doesn't match
                let mismatch = match (date_match, liters_match, price_match) {
                    (false, false, false) => "all",
                    (false, false, true) => "date_and_liters",
                    (false, true, false) => "date_and_price",
                    (false, true, true) => "date",
                    (true, false, false) => "liters_and_price",
                    (true, false, true) => "liters",
                    (true, true, false) => "price",
                    (true, true, true) => unreachable!(), // Would have matched above
                };
                CompatibilityResult {
                    can_attach: false,
                    status: "differs".to_string(),
                    mismatch_reason: Some(mismatch.to_string()),
                }
            }
        }
        _ => {
            // Receipt has no fuel data (other cost receipt) - can still attach as other cost
            // But wait - trip already has fuel, so this would be "other cost" on a fuel trip
            // Allow it since trips can have both fuel AND other costs
            CompatibilityResult {
                can_attach: true,
                status: "empty".to_string(),
                mismatch_reason: None,
            }
        }
    }
}

/// Internal get_trips_for_receipt_assignment logic (testable without State wrapper)
pub fn get_trips_for_receipt_assignment_internal(
    db: &Database,
    receipt_id: &str,
    vehicle_id: &str,
    year: i32,
) -> Result<Vec<TripForAssignment>, String> {
    // Get the receipt
    let receipt = db
        .get_receipt_by_id(receipt_id)
        .map_err(|e| e.to_string())?
        .ok_or("Receipt not found")?;

    // Get trips for this vehicle and year
    let trips = db
        .get_trips_for_vehicle_in_year(vehicle_id, year)
        .map_err(|e| e.to_string())?;

    // Annotate each trip with attachment eligibility
    let result = trips
        .into_iter()
        .map(|trip| {
            let compat = check_receipt_trip_compatibility(&receipt, &trip);
            TripForAssignment {
                trip,
                can_attach: compat.can_attach,
                attachment_status: compat.status,
                mismatch_reason: compat.mismatch_reason,
            }
        })
        .collect();

    Ok(result)
}

/// Get trips for a vehicle/year annotated with whether a specific receipt can be attached.
/// This allows the frontend to show which trips are eligible for receipt assignment.
#[tauri::command]
pub fn get_trips_for_receipt_assignment(
    db: State<Database>,
    receipt_id: String,
    vehicle_id: String,
    year: i32,
) -> Result<Vec<TripForAssignment>, String> {
    get_trips_for_receipt_assignment_internal(&db, &receipt_id, &vehicle_id, year)
}

/// Internal verify_receipts logic (testable without State wrapper)
pub fn verify_receipts_internal(
    db: &Database,
    vehicle_id: &str,
    year: i32,
) -> Result<VerificationResult, String> {
    let vehicle_uuid =
        Uuid::parse_str(vehicle_id).map_err(|e| format!("Invalid vehicle ID: {}", e))?;

    // Get receipts filtered by vehicle (unassigned + this vehicle's receipts)
    let all_receipts = db
        .get_receipts_for_vehicle(&vehicle_uuid, Some(year))
        .map_err(|e| e.to_string())?;
    let receipts_for_year: Vec<_> = all_receipts
        .into_iter()
        .filter(|r| {
            r.receipt_date
                .map(|d| d.year() == year)
                .unwrap_or(false)
        })
        .collect();

    verify_receipts_with_data(db, vehicle_id, year, receipts_for_year)
}

/// Helper to perform verification with pre-fetched receipts
fn verify_receipts_with_data(
    db: &Database,
    vehicle_id: &str,
    year: i32,
    receipts_for_year: Vec<Receipt>,
) -> Result<VerificationResult, String> {
    use crate::models::MismatchReason;

    // Get all trips for this vehicle/year
    let all_trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;

    // Separate trips with fuel and trips with other costs
    let trips_with_fuel: Vec<_> = all_trips.iter().filter(|t| t.fuel_liters.is_some()).collect();
    let trips_with_other_costs: Vec<_> = all_trips
        .iter()
        .filter(|t| t.other_costs_eur.is_some())
        .collect();

    let mut verifications = Vec::new();
    let mut matched_count = 0;

    for receipt in &receipts_for_year {
        let mut matched = false;
        let mut matched_trip_id = None;
        let mut matched_trip_date = None;
        let mut matched_trip_route = None;
        let mut mismatch_reason = MismatchReason::None;

        // Check if receipt has the necessary data for fuel matching
        let has_fuel_data = receipt.receipt_date.is_some()
            && receipt.liters.is_some()
            && receipt.total_price_eur.is_some();

        // Track closest match for determining specific mismatch reason
        // (date_match, liters_match, price_match, trip_date_str)
        let mut closest_match: Option<(bool, bool, bool, String)> = None;

        // 1. Try to match FUEL receipts (has liters) to fuel trips
        if let (Some(receipt_date), Some(receipt_liters), Some(receipt_price)) =
            (receipt.receipt_date, receipt.liters, receipt.total_price_eur)
        {
            for trip in &trips_with_fuel {
                if let (Some(trip_liters), Some(trip_price)) =
                    (trip.fuel_liters, trip.fuel_cost_eur)
                {
                    // Match by exact date, liters (within small tolerance), and price (within small tolerance)
                    let date_match = trip.date == receipt_date;
                    let liters_match = (trip_liters - receipt_liters).abs() < 0.01;
                    let price_match = (trip_price - receipt_price).abs() < 0.01;

                    if date_match && liters_match && price_match {
                        matched = true;
                        matched_trip_id = Some(trip.id.to_string());
                        matched_trip_date = Some(trip.date.format("%Y-%m-%d").to_string());
                        matched_trip_route =
                            Some(format!("{} - {}", trip.origin, trip.destination));
                        break;
                    }

                    // Track closest match (most fields matching)
                    let match_count =
                        date_match as u8 + liters_match as u8 + price_match as u8;
                    if match_count >= 2 {
                        // At least 2 fields match - this is a close match
                        let trip_date_str = trip.date.format("%-d.%-m.").to_string();
                        closest_match = Some((date_match, liters_match, price_match, trip_date_str));
                    }
                }
            }

            // Determine mismatch reason for fuel receipts
            if !matched {
                if trips_with_fuel.is_empty() {
                    mismatch_reason = MismatchReason::NoFuelTripFound;
                } else if let Some((date_match, liters_match, price_match, ref trip_date)) =
                    closest_match
                {
                    // Prioritize: date > liters > price (most common user error is date)
                    if !date_match && liters_match && price_match {
                        mismatch_reason = MismatchReason::DateMismatch {
                            receipt_date: receipt_date.format("%-d.%-m.").to_string(),
                            closest_trip_date: trip_date.clone(),
                        };
                    } else if date_match && !liters_match && price_match {
                        let trip_liters = trips_with_fuel
                            .iter()
                            .find(|t| t.date == receipt_date)
                            .and_then(|t| t.fuel_liters)
                            .unwrap_or(0.0);
                        mismatch_reason = MismatchReason::LitersMismatch {
                            receipt_liters,
                            trip_liters,
                        };
                    } else if date_match && liters_match && !price_match {
                        let trip_price = trips_with_fuel
                            .iter()
                            .find(|t| t.date == receipt_date)
                            .and_then(|t| t.fuel_cost_eur)
                            .unwrap_or(0.0);
                        mismatch_reason = MismatchReason::PriceMismatch {
                            receipt_price,
                            trip_price,
                        };
                    } else {
                        // Multiple fields don't match - show as no matching trip
                        mismatch_reason = MismatchReason::NoFuelTripFound;
                    }
                } else {
                    mismatch_reason = MismatchReason::NoFuelTripFound;
                }
            }
        } else if !has_fuel_data && receipt.liters.is_some() {
            // Has liters but missing date or price
            mismatch_reason = MismatchReason::MissingReceiptData;
        }

        // 2. If not matched as fuel, try to match "other cost" receipts by price
        if !matched && mismatch_reason == MismatchReason::None {
            if let Some(receipt_price) = receipt.total_price_eur {
                for trip in &trips_with_other_costs {
                    if let Some(trip_other_costs) = trip.other_costs_eur {
                        // Match by price (within small tolerance)
                        let price_match = (trip_other_costs - receipt_price).abs() < 0.01;

                        if price_match {
                            matched = true;
                            matched_trip_id = Some(trip.id.to_string());
                            matched_trip_date = Some(trip.date.format("%Y-%m-%d").to_string());
                            matched_trip_route =
                                Some(format!("{} - {}", trip.origin, trip.destination));
                            break;
                        }
                    }
                }

                // Set mismatch reason for other-cost receipts (non-fuel)
                if !matched && receipt.liters.is_none() {
                    mismatch_reason = MismatchReason::NoOtherCostMatch;
                }
            } else if receipt.liters.is_none() {
                // No price and no liters - missing data
                mismatch_reason = MismatchReason::MissingReceiptData;
            }
        }

        if matched {
            matched_count += 1;
            mismatch_reason = MismatchReason::None;
        }

        verifications.push(ReceiptVerification {
            receipt_id: receipt.id.to_string(),
            matched,
            matched_trip_id,
            matched_trip_date,
            matched_trip_route,
            mismatch_reason,
        });
    }

    let total = verifications.len();
    Ok(VerificationResult {
        total,
        matched: matched_count,
        unmatched: total - matched_count,
        receipts: verifications,
    })
}

/// Verify receipts against trips by matching date, liters, and price.
/// Returns verification status for each receipt in the given year.
/// Only considers receipts that are unassigned or assigned to this vehicle.
#[tauri::command]
pub fn verify_receipts(
    db: State<Database>,
    vehicle_id: String,
    year: i32,
) -> Result<VerificationResult, String> {
    verify_receipts_internal(&db, &vehicle_id, year)
}

// ============================================================================
// Window Commands
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

/// Returns the optimal window size from tauri.conf.json (embedded at compile time)
#[tauri::command]
pub fn get_optimal_window_size() -> WindowSize {
    // Parse tauri.conf.json at compile time
    let config_str = include_str!("../tauri.conf.json");

    // Simple parsing - find width and height values
    let width = config_str
        .find("\"width\":")
        .and_then(|i| {
            let rest = &config_str[i + 8..];
            let end = rest.find(',')?;
            rest[..end].trim().parse().ok()
        })
        .unwrap_or(1980);

    let height = config_str
        .find("\"height\":")
        .and_then(|i| {
            let rest = &config_str[i + 9..];
            let end = rest.find(',')?;
            rest[..end].trim().parse().ok()
        })
        .unwrap_or(1080);

    WindowSize { width, height }
}

// ============================================================================
// Live Preview Commands
// ============================================================================

/// Calculate preview values for a trip being edited/created.
/// Returns consumption rate, fuel remaining, and margin without saving.
#[tauri::command]
pub fn preview_trip_calculation(
    db: State<Database>,
    vehicle_id: String,
    year: i32,
    distance_km: i32,
    fuel_liters: Option<f64>,
    full_tank: bool,
    insert_at_sort_order: Option<i32>,
    editing_trip_id: Option<String>,
) -> Result<PreviewResult, String> {
    // Get vehicle for TP consumption and tank size
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    // Get existing trips
    let mut trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;

    // Create a virtual trip for preview
    let preview_trip_id = Uuid::new_v4();
    let now = Utc::now();

    // Determine the date and odometer for the preview trip to place it correctly
    // in chronological order for rate calculations
    let (preview_date, preview_odometer) = if let Some(sort_order) = insert_at_sort_order {
        // Inserting above a specific trip - use that trip's date and odometer - 0.5
        // (so it sorts just before the target trip on same date)
        trips
            .iter()
            .find(|t| t.sort_order == sort_order)
            .map(|t| (t.date, t.odometer - 0.5))
            .unwrap_or_else(|| (NaiveDate::from_ymd_opt(year, 12, 31).unwrap(), 0.0))
    } else {
        // New row at top - use the most recent trip's date and odometer + 0.5
        trips
            .iter()
            .max_by_key(|t| (t.date, t.odometer as i64))
            .map(|t| (t.date, t.odometer + 0.5))
            .unwrap_or_else(|| (Utc::now().date_naive(), 0.0))
    };

    let virtual_trip = Trip {
        id: preview_trip_id,
        vehicle_id: Uuid::parse_str(&vehicle_id).unwrap_or_else(|_| Uuid::new_v4()),
        date: preview_date,
        origin: "Preview".to_string(),
        destination: "Preview".to_string(),
        distance_km: distance_km as f64,
        odometer: preview_odometer,
        purpose: "Preview".to_string(),
        fuel_liters,
        fuel_cost_eur: None,
        full_tank,
        // Energy fields (BEV/PHEV) - TODO: Phase 2 will add preview support
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        sort_order: insert_at_sort_order.unwrap_or(0),
        created_at: now,
        updated_at: now,
    };

    // Handle editing vs inserting
    if let Some(edit_id) = &editing_trip_id {
        // Replace existing trip with virtual trip (keeping the ID for lookup)
        if let Some(pos) = trips.iter().position(|t| t.id.to_string() == *edit_id) {
            let existing = &trips[pos];
            // Create a modified trip with the new values but same ID
            let modified_trip = Trip {
                id: existing.id,
                vehicle_id: existing.vehicle_id,
                date: existing.date,
                origin: existing.origin.clone(),
                destination: existing.destination.clone(),
                distance_km: distance_km as f64,
                odometer: existing.odometer,
                purpose: existing.purpose.clone(),
                fuel_liters,
                fuel_cost_eur: existing.fuel_cost_eur,
                full_tank,
                // Preserve energy fields from existing trip
                energy_kwh: existing.energy_kwh,
                energy_cost_eur: existing.energy_cost_eur,
                full_charge: existing.full_charge,
                soc_override_percent: existing.soc_override_percent,
                other_costs_eur: existing.other_costs_eur,
                other_costs_note: existing.other_costs_note.clone(),
                sort_order: existing.sort_order,
                created_at: existing.created_at,
                updated_at: now,
            };
            trips[pos] = modified_trip;
        }
    } else {
        // Insert new virtual trip at the specified position
        trips.push(virtual_trip);
    }

    // Sort chronologically for calculations
    trips.sort_by(|a, b| {
        a.date.cmp(&b.date).then_with(|| {
            a.odometer
                .partial_cmp(&b.odometer)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    // Calculate rates and remaining fuel (ICE vehicles only for now)
    // TODO: Phase 2 will add BEV/PHEV preview support
    let tp_consumption = vehicle.tp_consumption.unwrap_or_default();
    let tank_size = vehicle.tank_size_liters.unwrap_or_default();

    let (rates, estimated_rates) = calculate_period_rates(&trips, tp_consumption);

    // Get initial fuel (carryover from previous year)
    let initial_fuel = get_year_start_fuel_remaining(
        &db,
        &vehicle_id,
        year,
        tank_size,
        tp_consumption,
    )?;

    let fuel_remaining = calculate_fuel_remaining(&trips, &rates, initial_fuel, tank_size);

    // Find the preview trip in results
    let target_id = if let Some(edit_id) = editing_trip_id {
        edit_id
    } else {
        preview_trip_id.to_string()
    };

    let consumption_rate = rates.get(&target_id).copied().unwrap_or(tp_consumption);
    let fuel_remaining_value = fuel_remaining.get(&target_id).copied().unwrap_or(tank_size);
    let is_estimated_rate = estimated_rates.contains(&target_id);
    let margin_percent = calculate_margin_percent(consumption_rate, tp_consumption);
    let is_over_limit = !is_within_legal_limit(margin_percent);

    Ok(PreviewResult {
        fuel_remaining: fuel_remaining_value,
        consumption_rate,
        margin_percent,
        is_over_limit,
        is_estimated_rate,
    })
}

// ============================================================================
// Theme Commands
// ============================================================================

#[tauri::command]
pub fn get_theme_preference(app_handle: tauri::AppHandle) -> Result<String, String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_data_dir);
    Ok(settings.theme.unwrap_or_else(|| "system".to_string()))
}

#[tauri::command]
pub fn set_theme_preference(app_handle: tauri::AppHandle, theme: String) -> Result<(), String> {
    // Validate
    if !["system", "light", "dark"].contains(&theme.as_str()) {
        return Err(format!("Invalid theme: {}. Must be system, light, or dark", theme));
    }

    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.theme = Some(theme);

    // Save to file
    let settings_path = app_data_dir.join("local.settings.json");
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, json).map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Auto-Update Settings Commands
// ============================================================================

#[tauri::command]
pub fn get_auto_check_updates(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_data_dir);
    // Default to true if not set
    Ok(settings.auto_check_updates.unwrap_or(true))
}

#[tauri::command]
pub fn set_auto_check_updates(app_handle: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.auto_check_updates = Some(enabled);

    // Save to file
    let settings_path = app_data_dir.join("local.settings.json");
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, json).map_err(|e| e.to_string())?;

    Ok(())
}


// ============================================================================
// Database Location Commands
// ============================================================================

/// Information about the current database location.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbLocationInfo {
    /// Full path to the database file
    pub db_path: String,
    /// Whether using a custom (non-default) database path
    pub is_custom_path: bool,
    /// Path to the backups directory
    pub backups_path: String,
}

/// Information about the current application mode.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppModeInfo {
    /// Current mode: "Normal" or "ReadOnly"
    pub mode: String,
    /// Whether the app is in read-only mode
    pub is_read_only: bool,
    /// Reason for read-only mode (if applicable)
    pub read_only_reason: Option<String>,
}

#[tauri::command]
pub fn get_db_location(app_state: State<AppState>) -> Result<DbLocationInfo, String> {
    let db_path = app_state
        .get_db_path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let is_custom = app_state.is_custom_path();

    // Derive backups path from db path
    let backups_path = app_state
        .get_db_path()
        .map(|p| {
            p.parent()
                .map(|parent| parent.join("backups").to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    Ok(DbLocationInfo {
        db_path,
        is_custom_path: is_custom,
        backups_path,
    })
}

#[tauri::command]
pub fn get_app_mode(app_state: State<AppState>) -> Result<AppModeInfo, String> {
    let mode = app_state.get_mode();
    let is_read_only = app_state.is_read_only();
    let read_only_reason = app_state.get_read_only_reason();

    Ok(AppModeInfo {
        mode: format!("{:?}", mode),
        is_read_only,
        read_only_reason,
    })
}

/// Check if a target directory already contains a database.
#[tauri::command]
pub fn check_target_has_db(target_path: String) -> Result<bool, String> {
    let db_file = PathBuf::from(&target_path).join("kniha-jazd.db");
    Ok(db_file.exists())
}

/// Result of a database move operation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveDbResult {
    pub success: bool,
    pub new_path: String,
    pub files_moved: usize,
}

/// Move the database to a new location (e.g., Google Drive, NAS).
///
/// This command:
/// 1. Copies the database file to the new location
/// 2. Copies the backups directory to the new location
/// 3. Updates local.settings.json with the new custom path
/// 4. Creates a lock file in the new location
/// 5. Releases the old lock and deletes old files
#[tauri::command]
pub fn move_database(
    app_handle: tauri::AppHandle,
    app_state: State<AppState>,
    target_folder: String,
) -> Result<MoveDbResult, String> {
    use crate::db_location::{DbPaths, acquire_lock, release_lock};

    check_read_only!(app_state);

    let target_dir = PathBuf::from(&target_folder);

    // Validate and create target directory
    if !target_dir.exists() {
        std::fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Nepodarilo sa vytvoriť priečinok: {}", e))?;
    }

    // Get current database path from app state
    let current_path = app_state.get_db_path()
        .ok_or("Cesta k databáze nie je nastavená")?;
    let current_dir = current_path.parent()
        .ok_or("Neplatná cesta k databáze")?
        .to_path_buf();

    // Don't allow moving to the same location
    if current_dir == target_dir {
        return Err("Databáza sa už nachádza v tomto priečinku".to_string());
    }

    let source_paths = DbPaths::from_dir(&current_dir);
    let target_paths = DbPaths::from_dir(&target_dir);

    let mut files_moved = 0;

    // Copy database file
    if source_paths.db_file.exists() {
        std::fs::copy(&source_paths.db_file, &target_paths.db_file)
            .map_err(|e| format!("Nepodarilo sa skopírovať databázu: {}", e))?;
        files_moved += 1;
    } else {
        return Err("Zdrojová databáza neexistuje".to_string());
    }

    // Copy backups directory if it exists
    if source_paths.backups_dir.exists() {
        copy_dir_all(&source_paths.backups_dir, &target_paths.backups_dir)
            .map_err(|e| format!("Nepodarilo sa skopírovať zálohy: {}", e))?;
        files_moved += count_files(&target_paths.backups_dir);
    }

    // Update local.settings.json with new custom path
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.custom_db_path = Some(target_folder.clone());
    settings.save(&app_data_dir)
        .map_err(|e| format!("Nepodarilo sa uložiť nastavenia: {}", e))?;

    // Create lock file in new location
    let version = env!("CARGO_PKG_VERSION");
    acquire_lock(&target_paths.lock_file, version)
        .map_err(|e| format!("Nepodarilo sa vytvoriť zámok: {}", e))?;

    // Release old lock (ignore errors - file may not exist)
    let _ = release_lock(&source_paths.lock_file);

    // Delete old files (after successful copy)
    let _ = std::fs::remove_file(&source_paths.db_file);
    if source_paths.backups_dir.exists() {
        let _ = std::fs::remove_dir_all(&source_paths.backups_dir);
    }

    // Update app state with new path
    app_state.set_db_path(target_paths.db_file, true);

    Ok(MoveDbResult {
        success: true,
        new_path: target_folder,
        files_moved,
    })
}

/// Reset database location to default app data directory.
#[tauri::command]
pub fn reset_database_location(
    app_handle: tauri::AppHandle,
    app_state: State<AppState>,
) -> Result<MoveDbResult, String> {
    use crate::db_location::{DbPaths, acquire_lock, release_lock};

    check_read_only!(app_state);

    // Get app data directory (default location)
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;

    // Get current database path
    let current_path = app_state.get_db_path()
        .ok_or("Cesta k databáze nie je nastavená")?;
    let current_dir = current_path.parent()
        .ok_or("Neplatná cesta k databáze")?
        .to_path_buf();

    // Don't reset if already in default location
    if current_dir == app_data_dir {
        return Err("Databáza sa už nachádza v predvolenom umiestnení".to_string());
    }

    let source_paths = DbPaths::from_dir(&current_dir);
    let target_paths = DbPaths::from_dir(&app_data_dir);

    let mut files_moved = 0;

    // Copy database file
    if source_paths.db_file.exists() {
        std::fs::copy(&source_paths.db_file, &target_paths.db_file)
            .map_err(|e| format!("Nepodarilo sa skopírovať databázu: {}", e))?;
        files_moved += 1;
    }

    // Copy backups directory
    if source_paths.backups_dir.exists() {
        copy_dir_all(&source_paths.backups_dir, &target_paths.backups_dir)
            .map_err(|e| format!("Nepodarilo sa skopírovať zálohy: {}", e))?;
        files_moved += count_files(&target_paths.backups_dir);
    }

    // Clear custom_db_path in settings
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.custom_db_path = None;
    settings.save(&app_data_dir)
        .map_err(|e| format!("Nepodarilo sa uložiť nastavenia: {}", e))?;

    // Create lock in new location
    let version = env!("CARGO_PKG_VERSION");
    acquire_lock(&target_paths.lock_file, version)
        .map_err(|e| format!("Nepodarilo sa vytvoriť zámok: {}", e))?;

    // Release old lock
    let _ = release_lock(&source_paths.lock_file);

    // Delete old files
    let _ = std::fs::remove_file(&source_paths.db_file);
    if source_paths.backups_dir.exists() {
        let _ = std::fs::remove_dir_all(&source_paths.backups_dir);
    }

    // Update app state
    app_state.set_db_path(target_paths.db_file, false);

    let new_path = app_data_dir.to_string_lossy().to_string();
    Ok(MoveDbResult {
        success: true,
        new_path,
        files_moved,
    })
}

/// Helper: Recursively copy a directory.
fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Helper: Count files in a directory.
fn count_files(dir: &PathBuf) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| entries.filter_map(|e| e.ok()).filter(|e| e.path().is_file()).count())
        .unwrap_or(0)
}

// ============================================================================
// Receipt Settings Commands
// ============================================================================

#[tauri::command]
pub fn set_gemini_api_key(
    app_handle: tauri::AppHandle,
    app_state: State<AppState>,
    api_key: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    let app_data_dir = get_app_data_dir(&app_handle)?;
    let mut settings = LocalSettings::load(&app_data_dir);

    // Allow empty string to clear the key
    settings.gemini_api_key = if api_key.is_empty() {
        None
    } else {
        Some(api_key)
    };

    settings.save(&app_data_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_receipts_folder_path(
    app_handle: tauri::AppHandle,
    app_state: State<AppState>,
    path: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    let app_data_dir = get_app_data_dir(&app_handle)?;

    // Validate path exists and is a directory (unless clearing)
    if !path.is_empty() {
        let path_buf = std::path::PathBuf::from(&path);
        if !path_buf.exists() {
            return Err(format!("Path does not exist: {}", path));
        }
        if !path_buf.is_dir() {
            return Err(format!("Path is not a directory: {}", path));
        }
    }

    let mut settings = LocalSettings::load(&app_data_dir);

    // Allow empty string to clear the path
    settings.receipts_folder_path = if path.is_empty() {
        None
    } else {
        Some(path)
    };

    settings.save(&app_data_dir).map_err(|e| e.to_string())
}

  #[cfg(test)]
  #[path = "commands_tests.rs"]
  mod tests;