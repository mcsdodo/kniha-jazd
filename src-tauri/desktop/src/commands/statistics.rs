//! Trip statistics and grid data Tauri command wrappers.
//!
//! All `_internal` implementations and helpers live in
//! [`kniha_jazd_core::commands_internal::statistics`].
//!
//! Re-exports here keep `super::statistics::*` working from
//! `commands_tests.rs` until that test file moves to core (Task 22a).

pub use kniha_jazd_core::commands_internal::statistics::*;

use kniha_jazd_core::commands_internal::statistics as inner;
use kniha_jazd_core::db::Database;
use kniha_jazd_core::models::{PreviewResult, TripGridData, TripStats};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn calculate_trip_stats(
    vehicle_id: String,
    year: i32,
    db: State<Arc<Database>>,
) -> Result<TripStats, String> {
    inner::calculate_trip_stats_internal(&db, vehicle_id, year)
}

/// Get pre-calculated trip grid data for frontend display.
/// Also pushes suggested fillup to HA sensor in the background (if configured).
#[tauri::command]
pub fn get_trip_grid_data(
    app_handle: tauri::AppHandle,
    db: State<Arc<Database>>,
    vehicle_id: String,
    year: i32,
) -> Result<TripGridData, String> {
    let grid_data = inner::build_trip_grid_data(&db, &vehicle_id, year)?;

    // Push suggested fillup to HA sensor in background (fire-and-forget)
    if let Ok(Some(vehicle)) = db.get_vehicle(&vehicle_id) {
        if let Some(sensor_id) = vehicle.ha_fillup_sensor {
            if let Ok(app_data_dir) = super::get_app_data_dir(&app_handle) {
                let state_text = super::integrations::format_suggested_fillup_text(
                    grid_data.legend_suggested_fillup.as_ref(),
                );
                tauri::async_runtime::spawn(super::integrations::push_ha_input_text(
                    app_data_dir,
                    sensor_id,
                    state_text,
                ));
            }
        }
    }

    Ok(grid_data)
}

#[tauri::command]
pub fn calculate_magic_fill_liters(
    db: State<Arc<Database>>,
    vehicle_id: String,
    year: i32,
    current_trip_km: f64,
    editing_trip_id: Option<String>,
) -> Result<f64, String> {
    inner::calculate_magic_fill_liters_internal(&db, vehicle_id, year, current_trip_km, editing_trip_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn preview_trip_calculation(
    db: State<Arc<Database>>,
    vehicle_id: String,
    year: i32,
    distance_km: i32,
    fuel_liters: Option<f64>,
    full_tank: bool,
    insert_at_sort_order: Option<i32>,
    editing_trip_id: Option<String>,
) -> Result<PreviewResult, String> {
    inner::preview_trip_calculation_internal(
        &db,
        vehicle_id,
        year,
        distance_km,
        fuel_liters,
        full_tank,
        insert_at_sort_order,
        editing_trip_id,
    )
}
