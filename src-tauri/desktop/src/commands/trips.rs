//! Trip CRUD and route Tauri command wrappers.

use kniha_jazd_core::app_state::AppState;
use kniha_jazd_core::commands_internal::trips as inner;
use kniha_jazd_core::db::Database;
use kniha_jazd_core::models::{InferredTripTime, Route, Trip};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_trips(db: State<Arc<Database>>, vehicle_id: String) -> Result<Vec<Trip>, String> {
    inner::get_trips_internal(&db, vehicle_id)
}

#[tauri::command]
pub fn get_trips_for_year(
    db: State<Arc<Database>>,
    vehicle_id: String,
    year: i32,
) -> Result<Vec<Trip>, String> {
    inner::get_trips_for_year_internal(&db, vehicle_id, year)
}

#[tauri::command]
pub fn get_years_with_trips(db: State<Arc<Database>>, vehicle_id: String) -> Result<Vec<i32>, String> {
    inner::get_years_with_trips_internal(&db, vehicle_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_trip(
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    vehicle_id: String,
    start_datetime: String,
    end_datetime: String,
    origin: String,
    destination: String,
    distance_km: f64,
    odometer: f64,
    purpose: String,
    fuel_liters: Option<f64>,
    fuel_cost: Option<f64>,
    full_tank: Option<bool>,
    energy_kwh: Option<f64>,
    energy_cost_eur: Option<f64>,
    full_charge: Option<bool>,
    soc_override_percent: Option<f64>,
    other_costs: Option<f64>,
    other_costs_note: Option<String>,
    insert_at_position: Option<i32>,
) -> Result<Trip, String> {
    inner::create_trip_internal(
        &db,
        &app_state,
        vehicle_id,
        start_datetime,
        end_datetime,
        origin,
        destination,
        distance_km,
        odometer,
        purpose,
        fuel_liters,
        fuel_cost,
        full_tank,
        energy_kwh,
        energy_cost_eur,
        full_charge,
        soc_override_percent,
        other_costs,
        other_costs_note,
        insert_at_position,
    )
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_trip(
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    id: String,
    start_datetime: String,
    end_datetime: String,
    origin: String,
    destination: String,
    distance_km: f64,
    odometer: f64,
    purpose: String,
    fuel_liters: Option<f64>,
    fuel_cost_eur: Option<f64>,
    full_tank: Option<bool>,
    energy_kwh: Option<f64>,
    energy_cost_eur: Option<f64>,
    full_charge: Option<bool>,
    soc_override_percent: Option<f64>,
    other_costs_eur: Option<f64>,
    other_costs_note: Option<String>,
) -> Result<Trip, String> {
    inner::update_trip_internal(
        &db,
        &app_state,
        id,
        start_datetime,
        end_datetime,
        origin,
        destination,
        distance_km,
        odometer,
        purpose,
        fuel_liters,
        fuel_cost_eur,
        full_tank,
        energy_kwh,
        energy_cost_eur,
        full_charge,
        soc_override_percent,
        other_costs_eur,
        other_costs_note,
    )
}

#[tauri::command]
pub fn delete_trip(
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    inner::delete_trip_internal(&db, &app_state, id)
}

#[tauri::command]
pub fn reorder_trip(
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    trip_id: String,
    new_sort_order: i32,
) -> Result<Vec<Trip>, String> {
    inner::reorder_trip_internal(&db, &app_state, trip_id, new_sort_order)
}

#[tauri::command]
pub fn get_routes(db: State<Arc<Database>>, vehicle_id: String) -> Result<Vec<Route>, String> {
    inner::get_routes_internal(&db, vehicle_id)
}

#[tauri::command]
pub fn get_purposes(db: State<Arc<Database>>, vehicle_id: String) -> Result<Vec<String>, String> {
    inner::get_purposes_internal(&db, vehicle_id)
}

#[tauri::command]
pub fn get_inferred_trip_time_for_route(
    app: tauri::AppHandle,
    db: State<Arc<Database>>,
    vehicle_id: String,
    origin: String,
    destination: String,
    row_date: String,
) -> Result<Option<InferredTripTime>, String> {
    let app_dir = super::get_app_data_dir(&app)?;
    inner::get_inferred_trip_time_for_route_internal(
        &db,
        &app_dir,
        vehicle_id,
        origin,
        destination,
        row_date,
    )
}
