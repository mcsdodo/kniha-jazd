//! Tauri commands to expose Rust functionality to the frontend
//!
//! This module is being refactored into feature-based submodules (ADR-011).

mod backup;
mod export_cmd;
mod integrations;
mod receipts_cmd;
mod settings_cmd;
mod statistics;
mod trips;
mod vehicles;

// Re-export all commands from submodules
pub use backup::*;
pub use export_cmd::*;
pub use integrations::*;
pub use receipts_cmd::*;
pub use settings_cmd::*;
pub use statistics::*;
pub use trips::*;
pub use vehicles::*;

use crate::constants::env_vars;
#[cfg(test)]
use crate::db::Database;
use crate::db_location::{resolve_db_paths, DbPaths};
use crate::models::{MonthEndRow, Trip};
use crate::settings::LocalSettings;
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::Manager;

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
    match std::env::var(env_vars::DATA_DIR) {
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

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
