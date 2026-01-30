//! Tauri commands to expose Rust functionality to the frontend
//!
//! This module is being refactored into feature-based submodules (ADR-011).

mod backup;
mod trips;
mod vehicles;

// Re-export all commands from submodules
pub use backup::*;
pub use trips::*;
pub use vehicles::*;

use crate::calculations::{
    calculate_buffer_km, calculate_closed_period_totals, calculate_consumption_rate,
    calculate_fuel_level, calculate_fuel_used, calculate_margin_percent, is_within_legal_limit,
};
use crate::calculations_energy::{
    calculate_battery_remaining, calculate_energy_used, kwh_to_percent,
};
use crate::calculations_phev::calculate_phev_trip_consumption;
use crate::db::Database;
use crate::db_location::{resolve_db_paths, DbPaths};
use crate::export::{generate_html, ExportData, ExportLabels, ExportTotals};
use crate::models::{
    MonthEndRow, PreviewResult, Settings, SuggestedFillup, Trip, TripGridData, TripStats, Vehicle,
    VehicleType,
};
use crate::settings::{DatePrefillMode, LocalSettings};
use chrono::{Datelike, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use tauri::{Emitter, Manager, State};
use uuid::Uuid;

// ============================================================================
// Helper Functions
// ============================================================================
use crate::app_state::AppState;

/// Parse a full ISO datetime string (from datetime-local input).
/// Accepts "YYYY-MM-DDTHH:MM" or "YYYY-MM-DDTHH:MM:SS" format.
pub(crate) fn parse_iso_datetime(datetime: &str) -> Result<NaiveDateTime, String> {
    // datetime-local gives us "YYYY-MM-DDTHH:MM", we need to handle both formats
    if datetime.len() == 16 {
        // "YYYY-MM-DDTHH:MM" format
        let with_seconds = format!("{}:00", datetime);
        NaiveDateTime::parse_from_str(&with_seconds, "%Y-%m-%dT%H:%M:%S")
            .map_err(|e| format!("Invalid datetime format: {}", e))
    } else {
        // "YYYY-MM-DDTHH:MM:SS" format
        NaiveDateTime::parse_from_str(datetime, "%Y-%m-%dT%H:%M:%S")
            .map_err(|e| format!("Invalid datetime format: {}", e))
    }
}

// ============================================================================
// Read-Only Guard Macro
// ============================================================================

/// Macro to check if app is in read-only mode before write operations.
/// Returns an error with Slovak message if read-only.
#[macro_export]
macro_rules! check_read_only {
    ($app_state:expr) => {
        if $app_state.is_read_only() {
            let reason = $app_state
                .get_read_only_reason()
                .unwrap_or_else(|| "Neznámy dôvod".to_string());
            return Err(format!("Aplikácia je v režime len na čítanie. {}", reason));
        }
    };
}

/// Get the app data directory, respecting the KNIHA_JAZD_DATA_DIR environment variable.
/// This ensures consistency between database operations and other file operations (backups, settings).
pub(crate) fn get_app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    match std::env::var("KNIHA_JAZD_DATA_DIR") {
        Ok(path) => Ok(PathBuf::from(path)),
        Err(_) => app.path().app_data_dir().map_err(|e| e.to_string()),
    }
}

/// Get resolved database paths (including backups directory), respecting custom_db_path in local.settings.json.
/// This ensures backups are stored alongside the database, even when using a custom location.
pub(crate) fn get_db_paths(app: &tauri::AppHandle) -> Result<DbPaths, String> {
    let app_dir = get_app_data_dir(app)?;
    let local_settings = LocalSettings::load(&app_dir);
    let (db_paths, _is_custom) =
        resolve_db_paths(&app_dir, local_settings.custom_db_path.as_deref());
    Ok(db_paths)
}

/// Calculate trip sequence numbers (1-based, chronological order by date then odometer)
pub(crate) fn calculate_trip_numbers(trips: &[Trip]) -> HashMap<String, i32> {
    // Sort by date, then by odometer for same-day trips
    let mut sorted: Vec<_> = trips.iter().collect();
    sorted.sort_by(|a, b| {
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| a.start_datetime.cmp(&b.start_datetime))
            .then_with(|| {
                a.odometer
                    .partial_cmp(&b.odometer)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    sorted
        .iter()
        .enumerate()
        .map(|(i, trip)| (trip.id.to_string(), (i + 1) as i32))
        .collect()
}

/// Calculate starting odometer for each trip (previous trip's ending odo)
/// First trip uses initial_odometer from vehicle.
pub(crate) fn calculate_odometer_start(
    trips: &[Trip],
    initial_odometer: f64,
) -> HashMap<String, f64> {
    // Sort chronologically
    let mut sorted: Vec<_> = trips.iter().collect();
    sorted.sort_by(|a, b| {
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| a.start_datetime.cmp(&b.start_datetime))
            .then_with(|| {
                a.odometer
                    .partial_cmp(&b.odometer)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let mut result = HashMap::new();
    let mut prev_odo = initial_odometer;

    for trip in sorted {
        result.insert(trip.id.to_string(), prev_odo);
        prev_odo = trip.odometer;
    }

    result
}

/// Get the last day of a given month
fn last_day_of_month(year: i32, month: u32) -> u32 {
    // Create first day of next month, subtract one day
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_year = if month == 12 { year + 1 } else { year };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
        .day()
}

/// Generate synthetic month-end rows for all closed months.
/// Returns rows only for months from January through the month of the last trip.
/// If no trips exist, returns rows for all 12 months.
///
/// # Arguments
/// * `trips` - All trips for the year (will be sorted chronologically)
/// * `year` - The year being processed
/// * `initial_odometer` - Starting odometer (from vehicle or year carryover)
/// * `initial_fuel` - Starting fuel (from vehicle or year carryover)
/// * `fuel_remaining` - Pre-calculated fuel remaining after each trip (from TripGridData)
/// * `trip_numbers` - Trip sequence numbers (for calculating sort_key)
pub(crate) fn generate_month_end_rows(
    trips: &[Trip],
    year: i32,
    initial_odometer: f64,
    initial_fuel: f64,
    fuel_remaining: &HashMap<String, f64>,
    trip_numbers: &HashMap<String, i32>,
) -> Vec<MonthEndRow> {
    // Sort trips chronologically
    let mut sorted: Vec<_> = trips.iter().collect();
    sorted.sort_by(|a, b| {
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| {
                a.odometer
                    .partial_cmp(&b.odometer)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // Only generate month-end rows for "closed" months:
    // - Past years: All 12 months are closed
    // - Current year: Only months before the latest trip's month
    let current_year = chrono::Utc::now().year();
    let last_month = if sorted.is_empty() {
        if year < current_year {
            12
        } else {
            0
        } // Past year with no trips: show all 12
    } else if year < current_year {
        12 // Past year: all months are closed
    } else {
        let latest_month = sorted.last().unwrap().start_datetime.date().month();
        // Current year: generate for months BEFORE the latest (those are closed)
        if latest_month > 1 {
            latest_month - 1
        } else {
            0
        }
    };

    // Track state as we process each month
    let mut current_odo = initial_odometer;
    let mut last_trip_id: Option<String> = None;

    let mut rows = Vec::new();

    for month in 1..=last_month {
        let last_day = last_day_of_month(year, month);
        let month_end_date = NaiveDate::from_ymd_opt(year, month, last_day).unwrap();

        // Find the last trip on or before this month-end
        for trip in &sorted {
            if trip.start_datetime.date() <= month_end_date {
                current_odo = trip.odometer;
                last_trip_id = Some(trip.id.to_string());
            } else {
                break;
            }
        }

        // Get fuel remaining from the last trip, or use initial if no trips yet
        let current_fuel = last_trip_id
            .as_ref()
            .and_then(|id| fuel_remaining.get(id))
            .copied()
            .unwrap_or(initial_fuel);

        // Calculate sort_key: last trip number in this month + 0.5
        // This ensures month-end rows sort after the last trip of their month
        let max_trip_num_in_month = sorted
            .iter()
            .filter(|t| {
                t.start_datetime.date().month() == month
                    && t.start_datetime.date() <= month_end_date
            })
            .filter_map(|t| trip_numbers.get(&t.id.to_string()))
            .max()
            .copied()
            .unwrap_or(0);
        let sort_key = max_trip_num_in_month as f64 + 0.5;

        // Always create month-end row (synthetic row shows period-end state)
        rows.push(MonthEndRow {
            date: month_end_date,
            odometer: current_odo,
            fuel_remaining: current_fuel,
            month,
            sort_key,
        });
    }

    rows
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
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| {
                a.odometer
                    .partial_cmp(&b.odometer)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
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
    let mut current_fuel =
        get_year_start_fuel_remaining(&db, &vehicle_id, year, tank_size, tp_consumption)?;

    for trip in &trips {
        // Calculate fuel used for this trip
        let fuel_used = calculate_fuel_used(trip.distance_km, last_consumption_rate);

        // Update fuel level
        current_fuel = calculate_fuel_level(current_fuel, fuel_used, trip.fuel_liters, tank_size);
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
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| {
                a.odometer
                    .partial_cmp(&b.odometer)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // Calculate rates for previous year
    let (rates, _) = calculate_period_rates(&chronological, tp_consumption);

    // Get the starting fuel for the previous year (recursive carryover)
    let prev_year_start =
        get_year_start_fuel_remaining(db, vehicle_id, prev_year, tank_size, tp_consumption)?;

    // Calculate fuel remaining for each trip, then get the last one (year-end state)
    let fuel_remaining =
        calculate_fuel_remaining(&chronological, &rates, prev_year_start, tank_size);

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
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| {
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
        current_battery =
            calculate_battery_remaining(current_battery, energy_used, trip.energy_kwh, capacity);
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
                a.start_datetime
                    .date()
                    .cmp(&b.start_datetime.date())
                    .then_with(|| {
                        a.odometer
                            .partial_cmp(&b.odometer)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            });
            return Ok(chronological
                .last()
                .map(|t| t.odometer)
                .unwrap_or(initial_odometer));
        }
        check_year -= 1;
    }

    // No data found in reasonable range - use vehicle's initial odometer
    Ok(initial_odometer)
}

/// Internal function to build trip grid data - single source of truth.
/// Used by both get_trip_grid_data command and export functions.
pub(crate) fn build_trip_grid_data(
    db: &Database,
    vehicle_id: &str,
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

    // Get vehicle specs needed for year-start calculations
    let tp_consumption = vehicle.tp_consumption.unwrap_or_default();
    let tank_size = vehicle.tank_size_liters.unwrap_or_default();

    // Calculate year starting values (carryover from previous year)
    let year_start_odometer =
        get_year_start_odometer(db, vehicle_id, year, vehicle.initial_odometer)?;

    let year_start_fuel =
        get_year_start_fuel_remaining(db, vehicle_id, year, tank_size, tp_consumption)?;

    if trips.is_empty() {
        return Ok(TripGridData {
            trips: vec![],
            rates: HashMap::new(),
            estimated_rates: HashSet::new(),
            fuel_consumed: HashMap::new(),
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
            year_start_fuel,
            suggested_fillup: HashMap::new(),
            legend_suggested_fillup: None,
            trip_numbers: HashMap::new(),
            odometer_start: HashMap::new(),
            month_end_rows: generate_month_end_rows(
                &[],
                year,
                year_start_odometer,
                year_start_fuel,
                &HashMap::new(),
                &HashMap::new(), // No trips = no trip numbers
            ),
        });
    }

    // Get all receipts for matching
    let receipts = db.get_all_receipts().map_err(|e| e.to_string())?;

    // Sort chronologically for calculations (by date, then odometer)
    let mut chronological = trips.clone();
    chronological.sort_by(|a, b| {
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| {
                a.odometer
                    .partial_cmp(&b.odometer)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

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
        get_year_start_battery_remaining(db, vehicle_id, year, &vehicle)?
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
            let (rates, estimated_rates) = calculate_period_rates(&chronological, tp_consumption);
            let fuel_remaining =
                calculate_fuel_remaining(&chronological, &rates, year_start_fuel, tank_size);
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
            let phev_data = calculate_phev_grid_data(
                &chronological,
                &vehicle,
                year_start_fuel,
                initial_battery,
            );
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

    // Calculate fuel consumed per trip (uses the same rates already calculated)
    let fuel_consumed = calculate_fuel_consumed(&chronological, &rates);

    // Calculate suggested fillup for trips in open period (ICE + PHEV only)
    let (suggested_fillup, legend_suggested_fillup) = if vehicle.vehicle_type.uses_fuel() {
        calculate_suggested_fillups(&chronological, tp_consumption)
    } else {
        (HashMap::new(), None)
    };

    // Legal compliance calculations (2026)
    let trip_numbers = calculate_trip_numbers(&trips);
    let odometer_start = calculate_odometer_start(&chronological, year_start_odometer);

    // Generate month-end rows using already-calculated fuel_remaining and trip_numbers
    let month_end_rows = generate_month_end_rows(
        &chronological,
        year,
        year_start_odometer,
        year_start_fuel,
        &fuel_remaining,
        &trip_numbers,
    );

    Ok(TripGridData {
        trips,
        rates,
        estimated_rates,
        fuel_consumed,
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
        year_start_fuel,
        suggested_fillup,
        legend_suggested_fillup,
        trip_numbers,
        odometer_start,
        month_end_rows,
    })
}

/// Get pre-calculated trip grid data for frontend display.
/// Thin wrapper around build_trip_grid_data for Tauri command.
#[tauri::command]
pub fn get_trip_grid_data(
    db: State<Database>,
    vehicle_id: String,
    year: i32,
) -> Result<TripGridData, String> {
    build_trip_grid_data(&db, &vehicle_id, year)
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

/// Calculate suggested fillup for all trips in open periods.
/// Returns:
/// - HashMap from trip ID to SuggestedFillup (for magic button per-trip)
/// - Option<SuggestedFillup> for the legend (most recent trip's suggestion)
/// Uses random multiplier 1.05-1.20 (same as magic fill).
fn calculate_suggested_fillups(
    chronological: &[Trip],
    tp_consumption: f64,
) -> (HashMap<String, SuggestedFillup>, Option<SuggestedFillup>) {
    use rand::Rng;

    let mut result = HashMap::new();
    let mut rng = rand::thread_rng();

    // Generate one random multiplier for this calculation batch
    // (provides consistency within a single data load)
    let target_multiplier = rng.gen_range(1.05..=1.20);
    let target_rate = tp_consumption * target_multiplier;

    // First pass: find the index where the open period starts
    // (after the last full tank, or from the beginning if no full tanks)
    let mut open_period_start_idx = 0;
    for (idx, trip) in chronological.iter().enumerate() {
        if let Some(fuel) = trip.fuel_liters {
            if fuel > 0.0 && trip.full_tank {
                // Period closed by full tank - next trip starts a new open period
                open_period_start_idx = idx + 1;
            }
        }
    }

    // Second pass: calculate suggested fillup for each trip in the open period
    let mut cumulative_km = 0.0;
    for trip in chronological.iter().skip(open_period_start_idx) {
        cumulative_km += trip.distance_km;

        if cumulative_km > 0.0 {
            // Calculate liters: liters = km * rate / 100
            let suggested_liters = (cumulative_km * target_rate) / 100.0;
            let rounded_liters = (suggested_liters * 100.0).round() / 100.0;

            // Calculate resulting consumption rate
            let consumption_rate = (rounded_liters / cumulative_km) * 100.0;
            let rounded_rate = (consumption_rate * 100.0).round() / 100.0;

            result.insert(
                trip.id.to_string(),
                SuggestedFillup {
                    liters: rounded_liters,
                    consumption_rate: rounded_rate,
                },
            );
        }
    }

    // Find the legend suggestion: most recent trip (lowest sort_order) that has a suggestion
    let legend = chronological
        .iter()
        .filter(|t| result.contains_key(&t.id.to_string()))
        .min_by_key(|t| t.sort_order)
        .and_then(|t| result.get(&t.id.to_string()).cloned());

    (result, legend)
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
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| {
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

/// Calculate fuel consumed per trip (liters).
/// Formula: distance_km × rate / 100
pub(crate) fn calculate_fuel_consumed(
    trips: &[Trip],
    rates: &HashMap<String, f64>,
) -> HashMap<String, f64> {
    trips
        .iter()
        .map(|trip| {
            let rate = rates.get(&trip.id.to_string()).copied().unwrap_or(0.0);
            let consumed = (trip.distance_km * rate) / 100.0;
            (trip.id.to_string(), consumed)
        })
        .collect()
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
        return (
            energy_rates,
            estimated_energy_rates,
            battery_kwh,
            battery_percent,
        );
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
        current_battery =
            calculate_battery_remaining(current_battery, energy_used, trip.energy_kwh, capacity);

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

    (
        energy_rates,
        estimated_energy_rates,
        battery_kwh,
        battery_percent,
    )
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
        // Check: prev.start_datetime.date() >= trip.start_datetime.date() >= next.start_datetime.date()
        if let Some(p) = prev {
            if trip.start_datetime.date() > p.start_datetime.date() {
                warnings.insert(trip.id.to_string());
            }
        }
        if let Some(n) = next {
            if trip.start_datetime.date() < n.start_datetime.date() {
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
            let date_match = r.receipt_date == Some(trip.start_datetime.date());
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
// HTML Export Commands
// ============================================================================

#[tauri::command]
pub async fn export_to_browser(
    _app: tauri::AppHandle,
    db: State<'_, Database>,
    vehicle_id: String,
    year: i32,
    license_plate: String,
    _sort_column: String,
    _sort_direction: String,
    labels: ExportLabels,
    hidden_columns: Vec<String>,
) -> Result<(), String> {
    // Get vehicle and settings
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    let settings = db
        .get_settings()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Settings not found - please configure company info first".to_string())?;

    // REUSE: Get all grid data from single source of truth
    let mut grid_data = build_trip_grid_data(&db, &vehicle_id, year)?;

    // Add synthetic "Prvý záznam" (first record) for export display
    let first_record_date =
        NaiveDate::from_ymd_opt(year, 1, 1).ok_or_else(|| "Invalid year".to_string())?;
    let first_record = Trip {
        id: Uuid::nil(),
        vehicle_id: vehicle.id,
        start_datetime: first_record_date.and_hms_opt(0, 0, 0).unwrap(),
        end_datetime: None,
        origin: "-".to_string(),
        destination: "-".to_string(),
        distance_km: 0.0,
        odometer: grid_data.year_start_odometer,
        purpose: "Prvý záznam".to_string(),
        fuel_liters: None,
        fuel_cost_eur: None,
        full_tank: true,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        sort_order: 999999,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    grid_data.trips.push(first_record);
    grid_data
        .fuel_remaining
        .insert(Uuid::nil().to_string(), grid_data.year_start_fuel);
    grid_data.trip_numbers.insert(Uuid::nil().to_string(), 0);
    grid_data
        .odometer_start
        .insert(Uuid::nil().to_string(), grid_data.year_start_odometer);

    // Calculate totals (reuses grid_data.trips, excludes 0km trips)
    let tp_consumption = vehicle.tp_consumption.unwrap_or_default();
    let baseline_consumption_kwh = vehicle.baseline_consumption_kwh.unwrap_or_default();
    let totals =
        ExportTotals::calculate(&grid_data.trips, tp_consumption, baseline_consumption_kwh);

    let export_data = ExportData {
        vehicle,
        settings,
        grid_data,
        year,
        totals,
        labels,
        hidden_columns,
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
    // Get vehicle and settings
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    let settings = db
        .get_settings()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Settings not found - please configure company info first".to_string())?;

    // REUSE: Get all grid data from single source of truth
    let grid_data = build_trip_grid_data(&db, &vehicle_id, year)?;

    if grid_data.trips.is_empty() {
        return Err("No trips found for this year".to_string());
    }

    // Calculate totals
    let tp_consumption = vehicle.tp_consumption.unwrap_or_default();
    let baseline_consumption_kwh = vehicle.baseline_consumption_kwh.unwrap_or_default();
    let totals =
        ExportTotals::calculate(&grid_data.trips, tp_consumption, baseline_consumption_kwh);

    // Generate HTML (export_html API doesn't support hidden columns, show all)
    let export_data = ExportData {
        vehicle,
        settings,
        grid_data,
        year,
        totals,
        labels,
        hidden_columns: Vec::new(),
    };

    generate_html(export_data)
}

// ============================================================================
// Receipt Commands
// ============================================================================

use crate::gemini::is_mock_mode_enabled;
use crate::models::{Receipt, ReceiptStatus, ReceiptVerification, VerificationResult};
use crate::receipts::{
    detect_folder_structure, process_receipt_with_gemini, scan_folder_for_new_receipts,
    FolderStructure,
};

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
pub fn scan_receipts(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    app_state: State<'_, AppState>,
) -> Result<ScanResult, String> {
    check_read_only!(app_state);
    let app_dir = get_app_data_dir(&app)?;
    let settings = LocalSettings::load(&app_dir);

    let folder_path = settings
        .receipts_folder_path
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
pub async fn sync_receipts(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    app_state: State<'_, AppState>,
) -> Result<SyncResult, String> {
    check_read_only!(app_state);
    let app_dir = get_app_data_dir(&app)?;
    let settings = LocalSettings::load(&app_dir);

    let folder_path = settings
        .receipts_folder_path
        .ok_or("Receipts folder not configured")?;

    // In mock mode, API key is not required (extract_from_image loads from JSON files)
    let api_key = if is_mock_mode_enabled() {
        String::new()
    } else {
        settings
            .gemini_api_key
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
        settings
            .gemini_api_key
            .ok_or("Gemini API key not configured")?
    };

    // Get all pending receipts
    let mut pending_receipts = db.get_pending_receipts().map_err(|e| e.to_string())?;
    let mut errors = Vec::new();
    let total = pending_receipts.len();

    // Process each pending receipt with Gemini
    for (index, receipt) in pending_receipts.iter_mut().enumerate() {
        // Emit progress event
        let _ = app.emit(
            "receipt-processing-progress",
            ProcessingProgress {
                current: index + 1,
                total,
                file_name: receipt.file_name.clone(),
            },
        );

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
pub fn update_receipt(
    db: State<Database>,
    app_state: State<AppState>,
    receipt: Receipt,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.update_receipt(&receipt).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_receipt(
    db: State<Database>,
    app_state: State<AppState>,
    id: String,
) -> Result<(), String> {
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
        settings
            .gemini_api_key
            .ok_or("Gemini API key not configured")?
    };

    let mut receipt = db
        .get_receipt_by_id(&id)
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
    // Receipt is FUEL if:
    //   1. Receipt has liters + price, AND
    //   2. Trip has NO fuel data (empty trip) OR trip fuel data matches receipt
    // Otherwise it's OTHER COST

    let trip_has_fuel = trip.fuel_liters.map(|l| l > 0.0).unwrap_or(false);

    let is_fuel_receipt = match (receipt.liters, receipt.total_price_eur) {
        (Some(liters), Some(price)) if liters > 0.0 => {
            if !trip_has_fuel {
                // Trip has no fuel → receipt will populate fuel fields
                true
            } else {
                // Trip has fuel → check if receipt matches (verification)
                let date_match = receipt.receipt_date == Some(trip.start_datetime.date());
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
        }
        _ => false, // No liters or no price → cannot be fuel
    };

    if is_fuel_receipt {
        // FUEL: populate or verify fuel fields
        if !trip_has_fuel {
            // Trip has no fuel → populate from receipt
            let mut updated_trip = trip.clone();
            updated_trip.fuel_liters = receipt.liters;
            updated_trip.fuel_cost_eur = receipt.total_price_eur;
            updated_trip.full_tank = true; // Assume full tank when populating from receipt
            db.update_trip(&updated_trip).map_err(|e| e.to_string())?;
        }
        // If trip already has matching fuel data, nothing to update (just link receipt)
    } else {
        // OTHER COST: populate trip.other_costs_* fields
        // (receipt without liters, or liters that don't match existing trip fuel)

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
            let date_match = receipt.receipt_date == Some(trip.start_datetime.date());
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
        .filter(|r| r.receipt_date.map(|d| d.year() == year).unwrap_or(false))
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
    let trips_with_fuel: Vec<_> = all_trips
        .iter()
        .filter(|t| t.fuel_liters.is_some())
        .collect();
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
        if let (Some(receipt_date), Some(receipt_liters), Some(receipt_price)) = (
            receipt.receipt_date,
            receipt.liters,
            receipt.total_price_eur,
        ) {
            for trip in &trips_with_fuel {
                if let (Some(trip_liters), Some(trip_price)) =
                    (trip.fuel_liters, trip.fuel_cost_eur)
                {
                    // Match by exact date, liters (within small tolerance), and price (within small tolerance)
                    let date_match = trip.start_datetime.date() == receipt_date;
                    let liters_match = (trip_liters - receipt_liters).abs() < 0.01;
                    let price_match = (trip_price - receipt_price).abs() < 0.01;

                    if date_match && liters_match && price_match {
                        matched = true;
                        matched_trip_id = Some(trip.id.to_string());
                        matched_trip_date =
                            Some(trip.start_datetime.date().format("%Y-%m-%d").to_string());
                        matched_trip_route =
                            Some(format!("{} - {}", trip.origin, trip.destination));
                        break;
                    }

                    // Track closest match (most fields matching)
                    let match_count = date_match as u8 + liters_match as u8 + price_match as u8;
                    if match_count >= 2 {
                        // At least 2 fields match - this is a close match
                        let trip_date_str =
                            trip.start_datetime.date().format("%-d.%-m.").to_string();
                        closest_match =
                            Some((date_match, liters_match, price_match, trip_date_str));
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
                            .find(|t| t.start_datetime.date() == receipt_date)
                            .and_then(|t| t.fuel_liters)
                            .unwrap_or(0.0);
                        mismatch_reason = MismatchReason::LitersMismatch {
                            receipt_liters,
                            trip_liters,
                        };
                    } else if date_match && liters_match && !price_match {
                        let trip_price = trips_with_fuel
                            .iter()
                            .find(|t| t.start_datetime.date() == receipt_date)
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
                            matched_trip_date =
                                Some(trip.start_datetime.date().format("%Y-%m-%d").to_string());
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
    let config_str = include_str!("../../tauri.conf.json");

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
            .map(|t| (t.start_datetime.date(), t.odometer - 0.5))
            .unwrap_or_else(|| (NaiveDate::from_ymd_opt(year, 12, 31).unwrap(), 0.0))
    } else {
        // New row at top - use the most recent trip's date and odometer + 0.5
        trips
            .iter()
            .max_by_key(|t| (t.start_datetime.date(), t.odometer as i64))
            .map(|t| (t.start_datetime.date(), t.odometer + 0.5))
            .unwrap_or_else(|| (Utc::now().date_naive(), 0.0))
    };

    let virtual_trip = Trip {
        id: preview_trip_id,
        vehicle_id: Uuid::parse_str(&vehicle_id).unwrap_or_else(|_| Uuid::new_v4()),
        start_datetime: preview_date.and_hms_opt(0, 0, 0).unwrap(),
        end_datetime: None,
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
                start_datetime: existing.start_datetime,
                end_datetime: existing.end_datetime,
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
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| {
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
    let initial_fuel =
        get_year_start_fuel_remaining(&db, &vehicle_id, year, tank_size, tp_consumption)?;

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
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_data_dir);
    Ok(settings.theme.unwrap_or_else(|| "system".to_string()))
}

#[tauri::command]
pub fn set_theme_preference(app_handle: tauri::AppHandle, theme: String) -> Result<(), String> {
    // Validate
    if !["system", "light", "dark"].contains(&theme.as_str()) {
        return Err(format!(
            "Invalid theme: {}. Must be system, light, or dark",
            theme
        ));
    }

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
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
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_data_dir);
    // Default to true if not set
    Ok(settings.auto_check_updates.unwrap_or(true))
}

#[tauri::command]
pub fn set_auto_check_updates(app_handle: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.auto_check_updates = Some(enabled);

    // Save to file
    let settings_path = app_data_dir.join("local.settings.json");
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, json).map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Date Prefill Mode Commands
// ============================================================================

#[tauri::command]
pub fn get_date_prefill_mode(app_handle: tauri::AppHandle) -> Result<DatePrefillMode, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_data_dir);
    // Default to Previous if not set
    Ok(settings.date_prefill_mode.unwrap_or_default())
}

#[tauri::command]
pub fn set_date_prefill_mode(
    app_handle: tauri::AppHandle,
    mode: DatePrefillMode,
) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.date_prefill_mode = Some(mode);

    // Save to file
    let settings_path = app_data_dir.join("local.settings.json");
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, json).map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Hidden Columns Commands
// ============================================================================

#[tauri::command]
pub fn get_hidden_columns(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_data_dir);
    // Default to empty array (all columns visible) if not set
    Ok(settings.hidden_columns.unwrap_or_default())
}

#[tauri::command]
pub fn set_hidden_columns(
    app_handle: tauri::AppHandle,
    columns: Vec<String>,
) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.hidden_columns = Some(columns);
    settings.save(&app_data_dir).map_err(|e| e.to_string())
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
    use crate::db_location::{acquire_lock, release_lock, DbPaths};

    check_read_only!(app_state);

    let target_dir = PathBuf::from(&target_folder);

    // Validate and create target directory
    if !target_dir.exists() {
        std::fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Nepodarilo sa vytvoriť priečinok: {}", e))?;
    }

    // Get current database path from app state
    let current_path = app_state
        .get_db_path()
        .ok_or("Cesta k databáze nie je nastavená")?;
    let current_dir = current_path
        .parent()
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
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.custom_db_path = Some(target_folder.clone());
    settings
        .save(&app_data_dir)
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
    use crate::db_location::{acquire_lock, release_lock, DbPaths};

    check_read_only!(app_state);

    // Get app data directory (default location)
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;

    // Get current database path
    let current_path = app_state
        .get_db_path()
        .ok_or("Cesta k databáze nie je nastavená")?;
    let current_dir = current_path
        .parent()
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
    settings
        .save(&app_data_dir)
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
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .count()
        })
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
    settings.receipts_folder_path = if path.is_empty() { None } else { Some(path) };

    settings.save(&app_data_dir).map_err(|e| e.to_string())
}

// ============================================================================
// Home Assistant Settings Commands
// ============================================================================

/// Response for get_ha_settings - hides token for security
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HaSettingsResponse {
    pub url: Option<String>,
    pub has_token: bool,
}

/// Response for get_local_settings_for_ha - includes token for frontend API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HaLocalSettingsResponse {
    pub ha_url: Option<String>,
    pub ha_api_token: Option<String>,
}

#[tauri::command]
pub fn get_ha_settings(app_handle: tauri::AppHandle) -> Result<HaSettingsResponse, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    let settings = LocalSettings::load(&app_data_dir);
    Ok(HaSettingsResponse {
        url: settings.ha_url,
        has_token: settings.ha_api_token.is_some(),
    })
}

/// Get HA settings including token for frontend to make API calls.
/// This is needed because the frontend needs the token to call HA directly.
#[tauri::command]
pub fn get_local_settings_for_ha(
    app_handle: tauri::AppHandle,
) -> Result<HaLocalSettingsResponse, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    let settings = LocalSettings::load(&app_data_dir);
    Ok(HaLocalSettingsResponse {
        ha_url: settings.ha_url,
        ha_api_token: settings.ha_api_token,
    })
}

/// Test HA connection from backend (avoids CORS issues in dev mode)
#[tauri::command]
pub async fn test_ha_connection(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    println!("[HA test] Loading settings from: {:?}", app_data_dir);
    let settings = LocalSettings::load(&app_data_dir);
    println!(
        "[HA test] ha_url: {:?}, has_token: {}",
        settings.ha_url,
        settings.ha_api_token.is_some()
    );

    let url = settings.ha_url.ok_or("HA URL not configured")?;
    let token = settings.ha_api_token.ok_or("HA token not configured")?;

    let api_url = format!("{}/api/", url.trim_end_matches('/'));
    println!("[HA test] Testing: {}", api_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(&api_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| {
            println!("[HA test] Error: {}", e);
            e.to_string()
        })?;

    let is_ok = response.status().is_success();
    println!(
        "[HA test] Response: {} ({})",
        response.status(),
        if is_ok { "OK" } else { "FAILED" }
    );
    Ok(is_ok)
}

/// Fetch ODO value from Home Assistant for a specific sensor
#[tauri::command]
pub async fn fetch_ha_odo(
    app_handle: tauri::AppHandle,
    sensor_id: String,
) -> Result<Option<f64>, String> {
    println!("[HA ODO] Fetching sensor: {}", sensor_id);
    let app_data_dir = get_app_data_dir(&app_handle)?;
    let settings = LocalSettings::load(&app_data_dir);

    let url = match settings.ha_url {
        Some(u) => u,
        None => {
            println!("[HA ODO] No URL configured");
            return Ok(None);
        }
    };
    let token = match settings.ha_api_token {
        Some(t) => t,
        None => {
            println!("[HA ODO] No token configured");
            return Ok(None);
        }
    };

    let api_url = format!("{}/api/states/{}", url.trim_end_matches('/'), sensor_id);
    println!("[HA ODO] Calling: {}", api_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(&api_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| {
            println!("[HA ODO] Request error: {}", e);
            e.to_string()
        })?;

    println!("[HA ODO] Response status: {}", response.status());
    if !response.status().is_success() {
        return Ok(None);
    }

    let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    // HA returns { state: "12345.6", ... }
    let state = data.get("state").and_then(|s| s.as_str());
    println!("[HA ODO] State value: {:?}", state);

    match state {
        Some(s) if s != "unavailable" && s != "unknown" => {
            let value = s.parse::<f64>().ok();
            println!("[HA ODO] Parsed value: {:?}", value);
            Ok(value)
        }
        _ => Ok(None),
    }
}

#[tauri::command]
pub fn save_ha_settings(
    app_handle: tauri::AppHandle,
    app_state: State<AppState>,
    url: Option<String>,
    token: Option<String>,
) -> Result<(), String> {
    check_read_only!(app_state);
    let app_data_dir = get_app_data_dir(&app_handle)?;

    // Validate URL if provided
    if let Some(ref url_str) = url {
        if !url_str.is_empty() {
            // Must start with http:// or https://
            if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
                return Err("URL must start with http:// or https://".to_string());
            }
            // Basic URL validation
            if url::Url::parse(url_str).is_err() {
                return Err("Invalid URL format".to_string());
            }
        }
    }

    let mut settings = LocalSettings::load(&app_data_dir);

    // Update URL (allow clearing with empty string, keep existing if None)
    if let Some(u) = url {
        settings.ha_url = if u.is_empty() { None } else { Some(u) };
    }

    // Update token only if explicitly provided (None = keep existing)
    // Empty string = clear token, Some(value) = set new token
    if let Some(t) = token {
        settings.ha_api_token = if t.is_empty() { None } else { Some(t) };
    }

    settings.save(&app_data_dir).map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
