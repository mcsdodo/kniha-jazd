//! Vehicle CRUD Tauri command wrappers.

use kniha_jazd_core::app_state::AppState;
use kniha_jazd_core::commands_internal::vehicles as inner;
use kniha_jazd_core::db::Database;
use kniha_jazd_core::models::Vehicle;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_vehicles(db: State<Arc<Database>>) -> Result<Vec<Vehicle>, String> {
    inner::get_vehicles_internal(&db)
}

#[tauri::command]
pub fn get_active_vehicle(db: State<Arc<Database>>) -> Result<Option<Vehicle>, String> {
    inner::get_active_vehicle_internal(&db)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_vehicle(
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    name: String,
    license_plate: String,
    initial_odometer: f64,
    vehicle_type: Option<String>,
    tank_size_liters: Option<f64>,
    tp_consumption: Option<f64>,
    battery_capacity_kwh: Option<f64>,
    baseline_consumption_kwh: Option<f64>,
    initial_battery_percent: Option<f64>,
    vin: Option<String>,
    driver_name: Option<String>,
) -> Result<Vehicle, String> {
    inner::create_vehicle_internal(
        &db,
        &app_state,
        name,
        license_plate,
        initial_odometer,
        vehicle_type,
        tank_size_liters,
        tp_consumption,
        battery_capacity_kwh,
        baseline_consumption_kwh,
        initial_battery_percent,
        vin,
        driver_name,
    )
}

#[tauri::command]
pub fn update_vehicle(
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    vehicle: Vehicle,
) -> Result<(), String> {
    inner::update_vehicle_internal(&db, &app_state, vehicle)
}

#[tauri::command]
pub fn delete_vehicle(
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    inner::delete_vehicle_internal(&db, &app_state, id)
}

#[tauri::command]
pub fn set_active_vehicle(
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    inner::set_active_vehicle_internal(&db, &app_state, id)
}
