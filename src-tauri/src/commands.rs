//! Tauri commands to expose Rust functionality to the frontend

use crate::calculations::{
    calculate_consumption_rate, calculate_margin_percent, calculate_spotreba, calculate_zostatok,
    is_within_legal_limit,
};
use crate::db::Database;
use crate::export::{generate_html, ExportData, ExportTotals};
use crate::models::{Route, Settings, Trip, TripGridData, TripStats, Vehicle};
use std::collections::{HashMap, HashSet};
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
        });
    }

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

    // Calculate fuel remaining for each trip
    let fuel_remaining =
        calculate_fuel_remaining(&chronological, &rates, vehicle.tank_size_liters);

    // Calculate date warnings (trips sorted by sort_order)
    let date_warnings = calculate_date_warnings(&trips);

    // Calculate consumption warnings
    let consumption_warnings =
        calculate_consumption_warnings(&trips, &rates, vehicle.tp_consumption);

    Ok(TripGridData {
        trips,
        rates,
        estimated_rates,
        fuel_remaining,
        date_warnings,
        consumption_warnings,
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
pub(crate) fn calculate_fuel_remaining(
    chronological: &[Trip],
    rates: &HashMap<String, f64>,
    tank_size: f64,
) -> HashMap<String, f64> {
    let mut remaining = HashMap::new();
    let mut zostatok = tank_size;

    for trip in chronological {
        let trip_id = trip.id.to_string();
        let rate = rates.get(&trip_id).copied().unwrap_or(0.0);
        let spotreba = if rate > 0.0 {
            (trip.distance_km * rate) / 100.0
        } else {
            0.0
        };

        zostatok -= spotreba;

        if let Some(fuel) = trip.fuel_liters {
            if fuel > 0.0 {
                if trip.full_tank {
                    zostatok = tank_size;
                } else {
                    zostatok += fuel;
                }
            }
        }

        // Clamp to valid range
        zostatok = zostatok.max(0.0).min(tank_size);
        remaining.insert(trip_id, zostatok);
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
    let fuel_remaining =
        calculate_fuel_remaining(&chronological, &rates, vehicle.tank_size_liters);

    let grid_data = TripGridData {
        trips,
        rates,
        estimated_rates,
        fuel_remaining,
        date_warnings: HashSet::new(),
        consumption_warnings: HashSet::new(),
    };

    let totals = ExportTotals::calculate(&chronological, vehicle.tp_consumption);

    let export_data = ExportData {
        vehicle,
        settings,
        grid_data,
        year,
        totals,
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
    let fuel_remaining =
        calculate_fuel_remaining(&chronological, &rates, vehicle.tank_size_liters);

    let grid_data = TripGridData {
        trips,
        rates,
        estimated_rates,
        fuel_remaining,
        date_warnings: HashSet::new(),
        consumption_warnings: HashSet::new(),
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
    };

    generate_html(export_data)
}

// ============================================================================
// Receipt Commands
// ============================================================================

use crate::models::{Receipt, ReceiptStatus};
use crate::receipts::{process_receipt_with_gemini, scan_folder_for_new_receipts};
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

#[tauri::command]
pub fn get_receipts(db: State<Database>) -> Result<Vec<Receipt>, String> {
    db.get_all_receipts().map_err(|e| e.to_string())
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

    // Process each pending receipt with Gemini
    for receipt in &mut pending_receipts {
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
