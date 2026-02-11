//! Vehicle CRUD commands.

use crate::app_state::AppState;
use crate::check_read_only;
use crate::db::Database;
use crate::models::{Vehicle, VehicleType};
use chrono::Utc;
use tauri::State;
use uuid::Uuid;

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
                return Err(
                    "BEV vehicles require battery_capacity_kwh and baseline_consumption_kwh"
                        .to_string(),
                );
            }
        }
        VehicleType::Phev => {
            if tank_size_liters.is_none()
                || tp_consumption.is_none()
                || battery_capacity_kwh.is_none()
                || baseline_consumption_kwh.is_none()
            {
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
        ha_odo_sensor: None,    // Set via vehicle edit modal
        ha_fillup_sensor: None, // Set via vehicle edit modal
        created_at: now,
        updated_at: now,
    };

    db.create_vehicle(&vehicle).map_err(|e| e.to_string())?;
    Ok(vehicle)
}

#[tauri::command]
pub fn update_vehicle(
    db: State<Database>,
    app_state: State<AppState>,
    vehicle: Vehicle,
) -> Result<(), String> {
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
pub fn delete_vehicle(
    db: State<Database>,
    app_state: State<AppState>,
    id: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.delete_vehicle(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_active_vehicle(
    db: State<Database>,
    app_state: State<AppState>,
    id: String,
) -> Result<(), String> {
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
