//! Tauri commands to expose Rust functionality to the frontend

use crate::calculations::{
    calculate_consumption_rate, calculate_margin_percent, calculate_fuel_used, calculate_fuel_level,
    is_within_legal_limit,
};
use crate::db::Database;
use crate::export::{generate_html, ExportData, ExportLabels, ExportTotals};
use crate::models::{PreviewResult, Route, Settings, Trip, TripGridData, TripStats, Vehicle};
use std::collections::{HashMap, HashSet};
use crate::suggestions::{build_compensation_suggestion, CompensationSuggestion};
use chrono::{Datelike, NaiveDate, Utc, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{Emitter, Manager, State};
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

    // Update or create route for autocomplete (same as create_trip)
    db.find_or_create_route(&trip.vehicle_id.to_string(), &trip.origin, &trip.destination, distance_km)
        .map_err(|e| e.to_string())?;

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
    year: i32,
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
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;
    trips.sort_by(|a, b| {
        a.date.cmp(&b.date).then_with(|| a.odometer.partial_cmp(&b.odometer).unwrap_or(std::cmp::Ordering::Equal))
    });

    // If no trips, return default values
    if trips.is_empty() {
        return Ok(TripStats {
            fuel_remaining_liters: vehicle.tank_size_liters,
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

    // Calculate current fuel level by processing all trips sequentially
    // Note: For accurate fuel level, we should use per-period rates, but for header display
    // we use the last consumption rate as a reasonable approximation
    // Start with carryover from previous year (or full tank if no previous data)
    let mut current_fuel = get_year_start_fuel_remaining(
        &db,
        &vehicle_id,
        year,
        vehicle.tank_size_liters,
        vehicle.tp_consumption,
    )?;

    for trip in &trips {
        // Calculate fuel used for this trip
        let fuel_used = calculate_fuel_used(trip.distance_km, last_consumption_rate);

        // Update fuel level
        current_fuel = calculate_fuel_level(
            current_fuel,
            fuel_used,
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
        fuel_remaining_liters: current_fuel,
        avg_consumption_rate,
        last_consumption_rate,
        margin_percent: display_margin,
        is_over_limit,
        total_km,
        total_fuel_liters: total_fuel,
        total_fuel_cost_eur: total_fuel_cost,
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

    if trips.is_empty() {
        return Ok(TripGridData {
            trips: vec![],
            rates: HashMap::new(),
            estimated_rates: HashSet::new(),
            fuel_remaining: HashMap::new(),
            date_warnings: HashSet::new(),
            consumption_warnings: HashSet::new(),
            missing_receipts: HashSet::new(),
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

    // Calculate consumption rates per period
    let (rates, estimated_rates) =
        calculate_period_rates(&chronological, vehicle.tp_consumption);

    // Calculate initial fuel (carryover from previous year or full tank)
    let initial_fuel = get_year_start_fuel_remaining(
        &db,
        &vehicle_id,
        year,
        vehicle.tank_size_liters,
        vehicle.tp_consumption,
    )?;

    // Calculate fuel remaining for each trip
    let fuel_remaining =
        calculate_fuel_remaining(&chronological, &rates, initial_fuel, vehicle.tank_size_liters);

    // Calculate date warnings (trips sorted by sort_order)
    let date_warnings = calculate_date_warnings(&trips);

    // Calculate consumption warnings
    let consumption_warnings =
        calculate_consumption_warnings(&trips, &rates, vehicle.tp_consumption);

    // Calculate missing receipts (trips with fuel but no matching receipt)
    let missing_receipts = calculate_missing_receipts(&trips, &receipts);

    Ok(TripGridData {
        trips,
        rates,
        estimated_rates,
        fuel_remaining,
        date_warnings,
        consumption_warnings,
        missing_receipts,
    })
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

    // Copy backup over current database
    fs::copy(&backup_path, &db_path).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_backup(app: tauri::AppHandle, filename: String) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup_path = app_dir.join("backups").join(&filename);

    if !backup_path.exists() {
        return Err(format!("Backup not found: {}", filename));
    }

    fs::remove_file(&backup_path).map_err(|e| e.to_string())?;
    Ok(())
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
        other_costs_eur: None,
        other_costs_note: None,
        full_tank: true,
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

    // Calculate rates and fuel remaining
    let (rates, estimated_rates) =
        calculate_period_rates(&chronological, vehicle.tp_consumption);

    // Get initial fuel (carryover from previous year)
    let initial_fuel = get_year_start_fuel_remaining(
        &db,
        &vehicle_id,
        year,
        vehicle.tank_size_liters,
        vehicle.tp_consumption,
    )?;

    let fuel_remaining =
        calculate_fuel_remaining(&chronological, &rates, initial_fuel, vehicle.tank_size_liters);

    let grid_data = TripGridData {
        trips,
        rates,
        estimated_rates,
        fuel_remaining,
        date_warnings: HashSet::new(),
        consumption_warnings: HashSet::new(),
        missing_receipts: HashSet::new(),
    };

    let totals = ExportTotals::calculate(&chronological, vehicle.tp_consumption);

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

    // Calculate rates and fuel remaining
    let (rates, estimated_rates) =
        calculate_period_rates(&chronological, vehicle.tp_consumption);

    // Get initial fuel (carryover from previous year)
    let initial_fuel = get_year_start_fuel_remaining(
        &db,
        &vehicle_id,
        year,
        vehicle.tank_size_liters,
        vehicle.tp_consumption,
    )?;

    let fuel_remaining =
        calculate_fuel_remaining(&chronological, &rates, initial_fuel, vehicle.tank_size_liters);

    let grid_data = TripGridData {
        trips,
        rates,
        estimated_rates,
        fuel_remaining,
        date_warnings: HashSet::new(),
        consumption_warnings: HashSet::new(),
        missing_receipts: HashSet::new(),
    };

    // Calculate totals for footer
    let totals = ExportTotals::calculate(&chronological, vehicle.tp_consumption);

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

use crate::models::{Receipt, ReceiptStatus, ReceiptVerification, VerificationResult};
use crate::receipts::{detect_folder_structure, process_receipt_with_gemini, scan_folder_for_new_receipts, FolderStructure};
use crate::settings::LocalSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptSettings {
    pub gemini_api_key: Option<String>,
    pub receipts_folder_path: Option<String>,
    pub gemini_api_key_from_override: bool,
    pub receipts_folder_from_override: bool,
}

#[tauri::command]
pub fn get_receipt_settings(app: tauri::AppHandle) -> Result<ReceiptSettings, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
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

#[tauri::command]
pub fn get_unassigned_receipts(db: State<Database>) -> Result<Vec<Receipt>, String> {
    db.get_unassigned_receipts().map_err(|e| e.to_string())
}

/// Result of sync operation - includes both successes and errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub processed: Vec<Receipt>,
    pub errors: Vec<SyncError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncError {
    pub file_name: String,
    pub error: String,
}

/// Result of scanning folder for new receipts (no OCR)
#[derive(Clone, Serialize)]
pub struct ScanResult {
    pub new_count: usize,
    pub warning: Option<String>,
}

/// Scan folder for new receipts without OCR processing
/// Returns count of new files found and any folder structure warnings
#[tauri::command]
pub fn scan_receipts(app: tauri::AppHandle, db: State<'_, Database>) -> Result<ScanResult, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
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
pub async fn sync_receipts(app: tauri::AppHandle, db: State<'_, Database>) -> Result<SyncResult, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_dir);

    let folder_path = settings.receipts_folder_path
        .ok_or("Receipts folder not configured")?;

    let api_key = settings.gemini_api_key
        .ok_or("Gemini API key not configured")?;

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
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_dir);

    let api_key = settings.gemini_api_key
        .ok_or("Gemini API key not configured")?;

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
pub fn update_receipt(db: State<Database>, receipt: Receipt) -> Result<(), String> {
    db.update_receipt(&receipt).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_receipt(db: State<Database>, id: String) -> Result<(), String> {
    db.delete_receipt(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reprocess_receipt(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    id: String,
) -> Result<Receipt, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_dir);

    let api_key = settings.gemini_api_key
        .ok_or("Gemini API key not configured")?;

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

#[tauri::command]
pub fn assign_receipt_to_trip(
    db: State<Database>,
    receipt_id: String,
    trip_id: String,
    vehicle_id: String,
) -> Result<Receipt, String> {
    let mut receipts = db.get_all_receipts().map_err(|e| e.to_string())?;
    let receipt = receipts
        .iter_mut()
        .find(|r| r.id.to_string() == receipt_id)
        .ok_or("Receipt not found")?;

    receipt.trip_id = Some(Uuid::parse_str(&trip_id).map_err(|e| e.to_string())?);
    receipt.vehicle_id = Some(Uuid::parse_str(&vehicle_id).map_err(|e| e.to_string())?);
    receipt.status = ReceiptStatus::Assigned;

    db.update_receipt(receipt).map_err(|e| e.to_string())?;

    Ok(receipt.clone())
}

/// Verify receipts against trips by matching date, liters, and price.
/// Returns verification status for each receipt in the given year.
#[tauri::command]
pub fn verify_receipts(
    db: State<Database>,
    vehicle_id: String,
    year: i32,
) -> Result<VerificationResult, String> {
    // Get all receipts and filter by year
    let all_receipts = db.get_all_receipts().map_err(|e| e.to_string())?;
    let receipts_for_year: Vec<_> = all_receipts
        .into_iter()
        .filter(|r| {
            r.receipt_date
                .map(|d| d.year() == year)
                .unwrap_or(false)
        })
        .collect();

    // Get all trips with fuel for this vehicle/year
    let trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;
    let trips_with_fuel: Vec<_> = trips
        .into_iter()
        .filter(|t| t.fuel_liters.is_some())
        .collect();

    let mut verifications = Vec::new();
    let mut matched_count = 0;

    for receipt in &receipts_for_year {
        let mut matched = false;
        let mut matched_trip_id = None;
        let mut matched_trip_date = None;
        let mut matched_trip_route = None;

        // Try to find a matching trip by exact date, liters, and price
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
                        matched_trip_route = Some(format!("{} - {}", trip.origin, trip.destination));
                        break;
                    }
                }
            }
        }

        if matched {
            matched_count += 1;
        }

        verifications.push(ReceiptVerification {
            receipt_id: receipt.id.to_string(),
            matched,
            matched_trip_id,
            matched_trip_date,
            matched_trip_route,
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

// ============================================================================
// Window Commands
// ============================================================================

#[derive(Debug, Clone, Serialize)]
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
        other_costs_eur: None,
        other_costs_note: None,
        full_tank,
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
                other_costs_eur: existing.other_costs_eur,
                other_costs_note: existing.other_costs_note.clone(),
                full_tank,
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

    // Calculate rates and remaining fuel using existing logic
    let (rates, estimated_rates) = calculate_period_rates(&trips, vehicle.tp_consumption);

    // Get initial fuel (carryover from previous year)
    let initial_fuel = get_year_start_fuel_remaining(
        &db,
        &vehicle_id,
        year,
        vehicle.tank_size_liters,
        vehicle.tp_consumption,
    )?;

    let fuel_remaining = calculate_fuel_remaining(&trips, &rates, initial_fuel, vehicle.tank_size_liters);

    // Find the preview trip in results
    let target_id = if let Some(edit_id) = editing_trip_id {
        edit_id
    } else {
        preview_trip_id.to_string()
    };

    let consumption_rate = rates.get(&target_id).copied().unwrap_or(vehicle.tp_consumption);
    let fuel_remaining_value = fuel_remaining.get(&target_id).copied().unwrap_or(vehicle.tank_size_liters);
    let is_estimated_rate = estimated_rates.contains(&target_id);
    let margin_percent = calculate_margin_percent(consumption_rate, vehicle.tp_consumption);
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
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ConfidenceLevel, FieldConfidence, Receipt, ReceiptStatus, Trip};
    use chrono::{NaiveDate, Utc};
    use uuid::Uuid;

    /// Helper to create a trip with fuel
    fn make_trip_with_fuel(date: NaiveDate, liters: f64, cost: f64) -> Trip {
        let now = Utc::now();
        Trip {
            id: Uuid::new_v4(),
            vehicle_id: Uuid::new_v4(),
            date,
            origin: "A".to_string(),
            destination: "B".to_string(),
            distance_km: 100.0,
            odometer: 10000.0,
            purpose: "business".to_string(),
            fuel_liters: Some(liters),
            fuel_cost_eur: Some(cost),
            other_costs_eur: None,
            other_costs_note: None,
            full_tank: true,
            sort_order: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Helper to create a trip without fuel
    fn make_trip_without_fuel(date: NaiveDate) -> Trip {
        let now = Utc::now();
        Trip {
            id: Uuid::new_v4(),
            vehicle_id: Uuid::new_v4(),
            date,
            origin: "A".to_string(),
            destination: "B".to_string(),
            distance_km: 50.0,
            odometer: 10050.0,
            purpose: "business".to_string(),
            fuel_liters: None,
            fuel_cost_eur: None,
            other_costs_eur: None,
            other_costs_note: None,
            full_tank: false,
            sort_order: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Helper to create a receipt with matching values
    fn make_receipt(date: Option<NaiveDate>, liters: Option<f64>, price: Option<f64>) -> Receipt {
        let now = Utc::now();
        Receipt {
            id: Uuid::new_v4(),
            vehicle_id: None,
            trip_id: None,
            file_path: "/test/receipt.jpg".to_string(),
            file_name: "receipt.jpg".to_string(),
            scanned_at: now,
            liters,
            total_price_eur: price,
            receipt_date: date,
            station_name: None,
            station_address: None,
            source_year: None,
            status: ReceiptStatus::Parsed,
            confidence: FieldConfidence {
                liters: ConfidenceLevel::High,
                total_price: ConfidenceLevel::High,
                date: ConfidenceLevel::High,
            },
            raw_ocr_text: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        }
    }

    // ========================================================================
    // Receipt-trip matching tests (calculate_missing_receipts)
    // ========================================================================

    #[test]
    fn test_missing_receipts_exact_match() {
        // Trip and receipt with exact same date, liters, and price
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
        let receipts = vec![make_receipt(Some(date), Some(45.0), Some(72.50))];

        let missing = calculate_missing_receipts(&trips, &receipts);

        assert!(missing.is_empty(), "Trip with matching receipt should not be flagged as missing");
    }

    #[test]
    fn test_missing_receipts_no_match_different_date() {
        // Same liters and price, but different date
        let trip_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let receipt_date = NaiveDate::from_ymd_opt(2024, 6, 16).unwrap();
        let trips = vec![make_trip_with_fuel(trip_date, 45.0, 72.50)];
        let receipts = vec![make_receipt(Some(receipt_date), Some(45.0), Some(72.50))];

        let missing = calculate_missing_receipts(&trips, &receipts);

        assert_eq!(missing.len(), 1, "Trip should be flagged when date differs");
        assert!(missing.contains(&trips[0].id.to_string()));
    }

    #[test]
    fn test_missing_receipts_no_match_different_liters() {
        // Same date and price, but different liters
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
        let receipts = vec![make_receipt(Some(date), Some(44.5), Some(72.50))]; // Different liters

        let missing = calculate_missing_receipts(&trips, &receipts);

        assert_eq!(missing.len(), 1, "Trip should be flagged when liters differ");
    }

    #[test]
    fn test_missing_receipts_no_match_different_price() {
        // Same date and liters, but different price
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
        let receipts = vec![make_receipt(Some(date), Some(45.0), Some(73.00))]; // Different price

        let missing = calculate_missing_receipts(&trips, &receipts);

        assert_eq!(missing.len(), 1, "Trip should be flagged when price differs");
    }

    #[test]
    fn test_missing_receipts_trip_without_fuel_not_flagged() {
        // Trip without fuel should NOT be flagged as missing receipt
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let trips = vec![make_trip_without_fuel(date)];
        let receipts: Vec<Receipt> = vec![];

        let missing = calculate_missing_receipts(&trips, &receipts);

        assert!(missing.is_empty(), "Trip without fuel should not be flagged as missing receipt");
    }

    #[test]
    fn test_missing_receipts_no_receipts_available() {
        // Trip with fuel but no receipts at all
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
        let receipts: Vec<Receipt> = vec![];

        let missing = calculate_missing_receipts(&trips, &receipts);

        assert_eq!(missing.len(), 1, "Trip with fuel but no receipts should be flagged");
    }

    #[test]
    fn test_missing_receipts_multiple_trips_partial_match() {
        // Multiple trips, some with matching receipts, some without
        let date1 = NaiveDate::from_ymd_opt(2024, 6, 10).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2024, 6, 20).unwrap();
        let date3 = NaiveDate::from_ymd_opt(2024, 6, 30).unwrap();

        let trips = vec![
            make_trip_with_fuel(date1, 40.0, 65.00), // Will have matching receipt
            make_trip_with_fuel(date2, 50.0, 80.00), // No matching receipt
            make_trip_without_fuel(date3),           // No fuel, should not be flagged
        ];
        let receipts = vec![
            make_receipt(Some(date1), Some(40.0), Some(65.00)), // Matches trip 1
        ];

        let missing = calculate_missing_receipts(&trips, &receipts);

        assert_eq!(missing.len(), 1, "Only trip 2 should be flagged");
        assert!(missing.contains(&trips[1].id.to_string()), "Trip 2 (with fuel, no receipt) should be flagged");
        assert!(!missing.contains(&trips[0].id.to_string()), "Trip 1 (with matching receipt) should not be flagged");
        assert!(!missing.contains(&trips[2].id.to_string()), "Trip 3 (no fuel) should not be flagged");
    }

    #[test]
    fn test_missing_receipts_receipt_with_missing_date() {
        // Receipt without a date cannot match
        let trip_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let trips = vec![make_trip_with_fuel(trip_date, 45.0, 72.50)];
        let receipts = vec![make_receipt(None, Some(45.0), Some(72.50))]; // No date

        let missing = calculate_missing_receipts(&trips, &receipts);

        assert_eq!(missing.len(), 1, "Receipt without date should not match");
    }

    #[test]
    fn test_missing_receipts_receipt_with_missing_liters() {
        // Receipt without liters cannot match
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
        let receipts = vec![make_receipt(Some(date), None, Some(72.50))]; // No liters

        let missing = calculate_missing_receipts(&trips, &receipts);

        assert_eq!(missing.len(), 1, "Receipt without liters should not match");
    }

    #[test]
    fn test_missing_receipts_receipt_with_missing_price() {
        // Receipt without price cannot match
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
        let receipts = vec![make_receipt(Some(date), Some(45.0), None)]; // No price

        let missing = calculate_missing_receipts(&trips, &receipts);

        assert_eq!(missing.len(), 1, "Receipt without price should not match");
    }

    // ========================================================================
    // Period rate calculation tests (calculate_period_rates)
    // ========================================================================

    /// Helper to create a trip with specific km, fuel, and full_tank flag
    fn make_trip_detailed(
        date: NaiveDate,
        distance_km: f64,
        fuel_liters: Option<f64>,
        full_tank: bool,
        sort_order: i32,
    ) -> Trip {
        let now = Utc::now();
        Trip {
            id: Uuid::new_v4(),
            vehicle_id: Uuid::new_v4(),
            date,
            origin: "A".to_string(),
            destination: "B".to_string(),
            distance_km,
            odometer: 10000.0 + distance_km,
            purpose: "business".to_string(),
            fuel_liters,
            fuel_cost_eur: fuel_liters.map(|l| l * 1.5),
            other_costs_eur: None,
            other_costs_note: None,
            full_tank,
            sort_order,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_period_rates_partial_fillup_doesnt_close_period() {
        // Business rule: Only full_tank=true closes a consumption period
        // Partial fillups (full_tank=false) accumulate fuel but don't close
        let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let tp_rate = 6.0;

        let trips = vec![
            make_trip_detailed(base_date, 100.0, None, false, 3),                           // 100km, no fuel
            make_trip_detailed(base_date.succ_opt().unwrap(), 100.0, Some(20.0), false, 2), // 100km, 20L PARTIAL
            make_trip_detailed(base_date.succ_opt().unwrap().succ_opt().unwrap(), 100.0, None, false, 1), // 100km, no fuel
            make_trip_detailed(base_date.succ_opt().unwrap().succ_opt().unwrap().succ_opt().unwrap(), 100.0, Some(30.0), true, 0), // 100km, 30L FULL
        ];

        let (rates, estimated) = calculate_period_rates(&trips, tp_rate);

        // All 4 trips should get same rate: 50L / 400km * 100 = 12.5 l/100km
        // The partial fillup at trip 2 should NOT create a separate period
        let expected_rate = 50.0 / 400.0 * 100.0; // 12.5
        for trip in &trips {
            let rate = rates.get(&trip.id.to_string()).unwrap();
            assert!(
                (rate - expected_rate).abs() < 0.01,
                "All trips should have rate {:.2}, got {:.2}",
                expected_rate,
                rate
            );
            assert!(
                !estimated.contains(&trip.id.to_string()),
                "All trips should have calculated (not estimated) rate"
            );
        }
    }

    #[test]
    fn test_period_rates_full_fillup_closes_period() {
        // Full tank fillups should close periods and create new rate calculations
        let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let tp_rate = 6.0;

        let trips = vec![
            make_trip_detailed(base_date, 100.0, None, false, 3),                           // Period 1: 100km
            make_trip_detailed(base_date.succ_opt().unwrap(), 100.0, Some(10.0), true, 2),  // Period 1: closes with 10L -> rate = 10/200*100 = 5.0
            make_trip_detailed(base_date.succ_opt().unwrap().succ_opt().unwrap(), 200.0, None, false, 1), // Period 2: 200km
            make_trip_detailed(base_date.succ_opt().unwrap().succ_opt().unwrap().succ_opt().unwrap(), 200.0, Some(16.0), true, 0), // Period 2: closes with 16L -> rate = 16/400*100 = 4.0
        ];

        let (rates, _) = calculate_period_rates(&trips, tp_rate);

        // Period 1 (trips 0-1): rate = 10L / 200km * 100 = 5.0
        let rate_period1 = 10.0 / 200.0 * 100.0;
        assert!(
            (rates.get(&trips[0].id.to_string()).unwrap() - rate_period1).abs() < 0.01,
            "Trip 0 should have period 1 rate"
        );
        assert!(
            (rates.get(&trips[1].id.to_string()).unwrap() - rate_period1).abs() < 0.01,
            "Trip 1 should have period 1 rate"
        );

        // Period 2 (trips 2-3): rate = 16L / 400km * 100 = 4.0
        let rate_period2 = 16.0 / 400.0 * 100.0;
        assert!(
            (rates.get(&trips[2].id.to_string()).unwrap() - rate_period2).abs() < 0.01,
            "Trip 2 should have period 2 rate"
        );
        assert!(
            (rates.get(&trips[3].id.to_string()).unwrap() - rate_period2).abs() < 0.01,
            "Trip 3 should have period 2 rate"
        );
    }

    #[test]
    fn test_period_rates_no_fullup_uses_tp_rate() {
        // When no full-tank fillup exists, use TP rate (estimated)
        let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let tp_rate = 6.0;

        let trips = vec![
            make_trip_detailed(base_date, 100.0, None, false, 1),
            make_trip_detailed(base_date.succ_opt().unwrap(), 100.0, Some(15.0), false, 0), // Partial only
        ];

        let (rates, estimated) = calculate_period_rates(&trips, tp_rate);

        // All trips should use TP rate (estimated)
        for trip in &trips {
            let rate = rates.get(&trip.id.to_string()).unwrap();
            assert!(
                (rate - tp_rate).abs() < 0.01,
                "Should use TP rate when no full fillup"
            );
            assert!(
                estimated.contains(&trip.id.to_string()),
                "Trips should be marked as estimated"
            );
        }
    }

    // ========================================================================
    // Date warning tests (calculate_date_warnings)
    // ========================================================================

    #[test]
    fn test_date_warnings_detects_out_of_order() {
        // Trips sorted by sort_order (0 = newest/top), but dates out of order
        let trips = vec![
            make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(), 50.0, None, false, 0), // Top: Jun 15
            make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(), 50.0, None, false, 1), // Middle: Jun 10 - WRONG! Should be between 15 and 20
            make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 20).unwrap(), 50.0, None, false, 2), // Bottom: Jun 20
        ];

        let warnings = calculate_date_warnings(&trips);

        // Jun 10 (middle) has earlier date than Jun 15 (top) - that's wrong for sort_order
        // Jun 10 also has earlier date than Jun 20 (bottom) - wrong again
        assert!(
            warnings.contains(&trips[1].id.to_string()),
            "Jun 10 trip should be flagged (out of order)"
        );
    }

    #[test]
    fn test_date_warnings_correct_order_no_warnings() {
        // Trips in correct order: newest (highest date) at sort_order 0
        let trips = vec![
            make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 20).unwrap(), 50.0, None, false, 0), // Top: Jun 20 (newest)
            make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(), 50.0, None, false, 1), // Middle: Jun 15
            make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(), 50.0, None, false, 2), // Bottom: Jun 10 (oldest)
        ];

        let warnings = calculate_date_warnings(&trips);

        assert!(
            warnings.is_empty(),
            "No warnings expected for correctly ordered trips"
        );
    }

    // ========================================================================
    // Consumption warning tests (calculate_consumption_warnings)
    // ========================================================================

    #[test]
    fn test_consumption_warnings_over_120_percent() {
        // Trip with rate > 120% of TP should be flagged
        let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let tp_rate = 5.0; // TP rate
        let limit = tp_rate * 1.2; // 6.0

        let trips = vec![
            make_trip_detailed(base_date, 100.0, Some(7.5), true, 0), // Rate = 7.5 l/100km > 6.0 limit
        ];

        let mut rates = std::collections::HashMap::new();
        rates.insert(trips[0].id.to_string(), 7.5);

        let warnings = calculate_consumption_warnings(&trips, &rates, tp_rate);

        assert!(
            warnings.contains(&trips[0].id.to_string()),
            "Trip with rate {:.1} > limit {:.1} should be flagged",
            7.5,
            limit
        );
    }

    #[test]
    fn test_consumption_warnings_at_limit_not_flagged() {
        // Trip with rate exactly at 120% should NOT be flagged (not OVER)
        let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let tp_rate = 5.0;
        let at_limit_rate = tp_rate * 1.2; // Exactly 6.0

        let trips = vec![
            make_trip_detailed(base_date, 100.0, Some(6.0), true, 0),
        ];

        let mut rates = std::collections::HashMap::new();
        rates.insert(trips[0].id.to_string(), at_limit_rate);

        let warnings = calculate_consumption_warnings(&trips, &rates, tp_rate);

        assert!(
            warnings.is_empty(),
            "Trip at exactly 120% limit should NOT be flagged (limit is 'greater than', not 'greater or equal')"
        );
    }

    #[test]
    fn test_consumption_warnings_under_limit_not_flagged() {
        // Trip with rate under limit should not be flagged
        let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let tp_rate = 5.0;

        let trips = vec![
            make_trip_detailed(base_date, 100.0, Some(5.0), true, 0), // Rate = 5.0 < 6.0 limit
        ];

        let mut rates = std::collections::HashMap::new();
        rates.insert(trips[0].id.to_string(), 5.0);

        let warnings = calculate_consumption_warnings(&trips, &rates, tp_rate);

        assert!(
            warnings.is_empty(),
            "Trip under limit should not be flagged"
        );
    }

    // ========================================================================
    // Fuel remaining tests (calculate_fuel_remaining)
    // ========================================================================

    #[test]
    fn test_fuel_remaining_basic_trip() {
        // Start with 50L, drive 100km at 6 l/100km = 6L used, end with 44L
        let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let trips = vec![
            make_trip_detailed(base_date, 100.0, None, false, 0),
        ];

        let mut rates = std::collections::HashMap::new();
        rates.insert(trips[0].id.to_string(), 6.0);

        let remaining = calculate_fuel_remaining(&trips, &rates, 50.0, 66.0);

        let expected = 50.0 - 6.0; // 44L
        let actual = remaining.get(&trips[0].id.to_string()).unwrap();
        assert!(
            (actual - expected).abs() < 0.01,
            "Expected {:.1}L remaining, got {:.1}L",
            expected,
            actual
        );
    }

    #[test]
    fn test_fuel_remaining_with_partial_fillup() {
        // Partial fillup adds fuel but doesn't fill tank
        let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let trips = vec![
            make_trip_detailed(base_date, 100.0, Some(30.0), false, 0), // 100km, add 30L partial
        ];

        let mut rates = std::collections::HashMap::new();
        rates.insert(trips[0].id.to_string(), 6.0);

        let remaining = calculate_fuel_remaining(&trips, &rates, 20.0, 66.0);

        // Start: 20L, use 6L, add 30L = 44L
        let expected = 20.0 - 6.0 + 30.0; // 44L
        let actual = remaining.get(&trips[0].id.to_string()).unwrap();
        assert!(
            (actual - expected).abs() < 0.01,
            "Partial fillup: expected {:.1}L, got {:.1}L",
            expected,
            actual
        );
    }

    #[test]
    fn test_fuel_remaining_with_full_fillup() {
        // Full tank fillup fills to tank_size regardless of fuel added
        let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let tank_size = 66.0;
        let trips = vec![
            make_trip_detailed(base_date, 100.0, Some(30.0), true, 0), // Full tank
        ];

        let mut rates = std::collections::HashMap::new();
        rates.insert(trips[0].id.to_string(), 6.0);

        let remaining = calculate_fuel_remaining(&trips, &rates, 20.0, tank_size);

        // Full tank = always ends at tank_size
        let actual = remaining.get(&trips[0].id.to_string()).unwrap();
        assert!(
            (actual - tank_size).abs() < 0.01,
            "Full fillup should result in full tank ({:.1}L), got {:.1}L",
            tank_size,
            actual
        );
    }

    #[test]
    fn test_fuel_remaining_clamps_to_zero() {
        // Can't go negative - clamps to 0
        let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let trips = vec![
            make_trip_detailed(base_date, 500.0, None, false, 0), // 500km at 6 l/100km = 30L, but only have 10L
        ];

        let mut rates = std::collections::HashMap::new();
        rates.insert(trips[0].id.to_string(), 6.0);

        let remaining = calculate_fuel_remaining(&trips, &rates, 10.0, 66.0);

        let actual = remaining.get(&trips[0].id.to_string()).unwrap();
        assert!(
            *actual >= 0.0,
            "Fuel remaining should not go negative, got {:.1}L",
            actual
        );
        assert!(
            (actual - 0.0).abs() < 0.01,
            "Should clamp to 0, got {:.1}L",
            actual
        );
    }

    // ========================================================================
    // Year carryover tests (get_year_start_fuel_remaining)
    // ========================================================================

    #[test]
    fn test_year_start_fuel_no_previous_year_data() {
        // When no trips exist in the previous year, should return full tank
        let db = crate::db::Database::in_memory().expect("Failed to create database");

        let vehicle = crate::models::Vehicle::new(
            "Test Car".to_string(),
            "BA123XY".to_string(),
            50.0,  // tank_size
            6.0,   // tp_consumption
            0.0,
        );
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        // Query for 2025 with no 2024 data
        let result = get_year_start_fuel_remaining(
            &db,
            &vehicle.id.to_string(),
            2025,
            50.0,  // tank_size
            6.0,   // tp_consumption
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 50.0, "Should return full tank when no previous year data");
    }

    #[test]
    fn test_year_start_fuel_with_previous_year_full_tank() {
        // When previous year ends with full tank fillup, should return tank_size
        let db = crate::db::Database::in_memory().expect("Failed to create database");

        let vehicle = crate::models::Vehicle::new(
            "Test Car".to_string(),
            "BA123XY".to_string(),
            50.0,
            6.0,
            0.0,
        );
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        let now = Utc::now();
        let trip_2024 = Trip {
            id: Uuid::new_v4(),
            vehicle_id: vehicle.id,
            date: NaiveDate::from_ymd_opt(2024, 12, 15).unwrap(),
            origin: "A".to_string(),
            destination: "B".to_string(),
            distance_km: 100.0,
            odometer: 10000.0,
            purpose: "test".to_string(),
            fuel_liters: Some(6.0),
            fuel_cost_eur: Some(10.0),
            other_costs_eur: None,
            other_costs_note: None,
            full_tank: true,  // Full tank fillup -> ends at 50L
            sort_order: 0,
            created_at: now,
            updated_at: now,
        };
        db.create_trip(&trip_2024).expect("Failed to create trip");

        let result = get_year_start_fuel_remaining(
            &db,
            &vehicle.id.to_string(),
            2025,
            50.0,
            6.0,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 50.0, "Full tank fillup should end at tank_size");
    }

    #[test]
    fn test_year_start_fuel_partial_tank_carryover() {
        // Test that partial tank fillups carry over correctly
        let db = crate::db::Database::in_memory().expect("Failed to create database");

        let vehicle = crate::models::Vehicle::new(
            "Test Car".to_string(),
            "BA123XY".to_string(),
            50.0,  // tank_size
            6.0,   // tp_consumption (6 l/100km)
            0.0,
        );
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        let now = Utc::now();

        // Trip 1: Drive 100km, full tank fillup with 6L
        // Starts at 50L (no prior year), uses 6L, ends at 50L (full tank)
        let trip1 = Trip {
            id: Uuid::new_v4(),
            vehicle_id: vehicle.id,
            date: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            origin: "A".to_string(),
            destination: "B".to_string(),
            distance_km: 100.0,
            odometer: 10000.0,
            purpose: "test".to_string(),
            fuel_liters: Some(6.0),
            fuel_cost_eur: Some(10.0),
            other_costs_eur: None,
            other_costs_note: None,
            full_tank: true,
            sort_order: 1,
            created_at: now,
            updated_at: now,
        };

        // Trip 2: Drive 200km, partial fillup with 10L
        // Rate from trip1 is 6%, so uses 12L, starts at 50L, ends at 50-12+10=48L
        let trip2 = Trip {
            id: Uuid::new_v4(),
            vehicle_id: vehicle.id,
            date: NaiveDate::from_ymd_opt(2024, 12, 20).unwrap(),
            origin: "B".to_string(),
            destination: "C".to_string(),
            distance_km: 200.0,
            odometer: 10200.0,
            purpose: "test".to_string(),
            fuel_liters: Some(10.0),
            fuel_cost_eur: Some(16.0),
            other_costs_eur: None,
            other_costs_note: None,
            full_tank: false,  // Partial fillup
            sort_order: 0,
            created_at: now,
            updated_at: now,
        };

        db.create_trip(&trip1).expect("Failed to create trip1");
        db.create_trip(&trip2).expect("Failed to create trip2");

        let result = get_year_start_fuel_remaining(
            &db,
            &vehicle.id.to_string(),
            2025,
            50.0,
            6.0,
        );

        assert!(result.is_ok());
        // After trip1: full tank (50L)
        // Trip2 uses 12L at 6% rate, adds 10L partial = 50 - 12 + 10 = 48L
        let fuel = result.unwrap();
        assert!((fuel - 48.0).abs() < 0.1, "Expected ~48L, got {}", fuel);
    }
}
