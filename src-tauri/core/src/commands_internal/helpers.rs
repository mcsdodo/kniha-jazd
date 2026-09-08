//! Pure helpers shared across `*_internal` command implementations.
//!
//! Nothing here discovers the application directory on its own — the server
//! owns it (`ServerState::app_dir`, from `KNIHA_JAZD_DATA_DIR`) and passes it
//! down, which is what keeps these functions testable against a temp dir.

use crate::db_location::{resolve_db_paths, DbPaths};
use crate::models::{MonthEndRow, Trip};
use crate::settings::LocalSettings;
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use std::collections::HashMap;

/// Parse a full ISO datetime string (from datetime-local input).
/// Accepts "YYYY-MM-DDTHH:MM" or "YYYY-MM-DDTHH:MM:SS" format.
pub fn parse_iso_datetime(datetime: &str) -> Result<NaiveDateTime, String> {
    if datetime.len() == 16 {
        let with_seconds = format!("{}:00", datetime);
        NaiveDateTime::parse_from_str(&with_seconds, "%Y-%m-%dT%H:%M:%S")
            .map_err(|e| format!("Invalid datetime format: {}", e))
    } else {
        NaiveDateTime::parse_from_str(datetime, "%Y-%m-%dT%H:%M:%S")
            .map_err(|e| format!("Invalid datetime format: {}", e))
    }
}

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

/// Get resolved database paths from a directory path.
/// Honours a `custom_db_path` in that directory's `local.settings.json`.
pub fn get_db_paths_for_dir(app_dir: &std::path::Path) -> Result<DbPaths, String> {
    let local_settings = LocalSettings::load(app_dir);
    let (db_paths, _is_custom) =
        resolve_db_paths(app_dir, local_settings.custom_db_path.as_deref());
    Ok(db_paths)
}

/// The one order the whole book uses.
///
/// Each key applies only when the one before it carries no information:
///
/// 1. `start_datetime` -- what the driver recorded.
/// 2. `created_at` -- real evidence when it differs. It is what separates the
///    two 2026-08-19 rows correctly, which the places confirm (task 79).
/// 3. `odometer` -- the only surviving evidence for rows imported under one
///    identical timestamp. The odometer only moves forward, so the lower
///    ending value came first.
/// 4. `id` -- makes the order total, so nothing depends on the DB row order
///    or on a stable-sort accident.
///
/// The odometer sits BELOW `created_at` on purpose. Ordering by the odometer
/// first makes the chain agree with itself by construction, which would
/// silence the span warnings that exist to test those very odometers.
pub fn trip_order(a: &Trip, b: &Trip) -> std::cmp::Ordering {
    a.start_datetime
        .cmp(&b.start_datetime)
        .then_with(|| a.created_at.cmp(&b.created_at))
        .then_with(|| {
            a.odometer
                .partial_cmp(&b.odometer)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| a.id.cmp(&b.id))
}

/// Calculate trip sequence numbers (1-based, chronological order by datetime then created_at)
pub fn calculate_trip_numbers(trips: &[Trip]) -> HashMap<String, i32> {
    let mut sorted: Vec<_> = trips.iter().collect();
    sorted.sort_by(|a, b| trip_order(a, b));

    sorted
        .iter()
        .enumerate()
        .map(|(i, trip)| (trip.id.to_string(), (i + 1) as i32))
        .collect()
}

/// Calculate starting odometer for each trip (previous trip's ending odo)
/// First trip uses initial_odometer from vehicle.
pub fn calculate_odometer_start(
    trips: &[Trip],
    initial_odometer: f64,
) -> HashMap<String, f64> {
    let mut sorted: Vec<_> = trips.iter().collect();
    sorted.sort_by(|a, b| trip_order(a, b));

    let mut result = HashMap::new();
    let mut prev_odo = initial_odometer;

    for trip in sorted {
        result.insert(trip.id.to_string(), prev_odo);
        prev_odo = trip.odometer;
    }

    result
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_year = if month == 12 { year + 1 } else { year };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
        .day()
}

/// Generate synthetic month-end rows for all closed months.
pub fn generate_month_end_rows(
    trips: &[Trip],
    year: i32,
    initial_odometer: f64,
    initial_fuel: f64,
    fuel_remaining: &HashMap<String, f64>,
    trip_numbers: &HashMap<String, i32>,
) -> Vec<MonthEndRow> {
    let mut sorted: Vec<_> = trips.iter().collect();
    sorted.sort_by(|a, b| trip_order(a, b));

    let current_year = chrono::Utc::now().year();
    let last_month = if sorted.is_empty() {
        if year < current_year { 12 } else { 0 }
    } else if year < current_year {
        12
    } else {
        let latest_month = sorted.last().unwrap().start_datetime.date().month();
        if latest_month > 1 { latest_month - 1 } else { 0 }
    };

    let mut current_odo = initial_odometer;
    let mut last_trip_id: Option<String> = None;
    let mut rows = Vec::new();

    for month in 1..=last_month {
        let last_day = last_day_of_month(year, month);
        let month_end_date = NaiveDate::from_ymd_opt(year, month, last_day).unwrap();

        for trip in &sorted {
            if trip.start_datetime.date() <= month_end_date {
                current_odo = trip.odometer;
                last_trip_id = Some(trip.id.to_string());
            } else {
                break;
            }
        }

        let current_fuel = last_trip_id
            .as_ref()
            .and_then(|id| fuel_remaining.get(id))
            .copied()
            .unwrap_or(initial_fuel);

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
