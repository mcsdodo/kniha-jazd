//! Tauri commands to expose Rust functionality to the frontend

use crate::calculations::{
    calculate_consumption_rate, calculate_margin_percent, calculate_spotreba, calculate_zostatok,
    is_within_legal_limit,
};
use crate::db::Database;
use crate::models::{Route, Settings, Trip, TripStats, Vehicle};
use crate::suggestions::{build_compensation_suggestion, CompensationSuggestion};
use chrono::{NaiveDate, Utc};
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
) -> Result<Trip, String> {
    let vehicle_uuid = Uuid::parse_str(&vehicle_id).map_err(|e| e.to_string())?;
    let trip_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|e| e.to_string())?;

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
pub fn update_trip(db: State<Database>, trip: Trip) -> Result<(), String> {
    db.update_trip(&trip).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_trip(db: State<Database>, id: String) -> Result<(), String> {
    db.delete_trip(&id).map_err(|e| e.to_string())
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

    // For filler trip purpose, we'll use "testovanie" as default
    // In a full implementation, this would come from Settings
    let filler_purpose = "testovanie";

    let suggestion = build_compensation_suggestion(&routes, buffer_km, &current_location, filler_purpose);

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
    db: State<Database>,
) -> Result<TripStats, String> {
    // Get vehicle
    let vehicle_uuid = Uuid::parse_str(&vehicle_id).map_err(|e| e.to_string())?;
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    // Get all trips for this vehicle, sorted by date
    let mut trips = db
        .get_trips_for_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?;
    trips.sort_by(|a, b| a.date.cmp(&b.date));

    // If no trips, return default values
    if trips.is_empty() {
        return Ok(TripStats {
            zostatok_liters: vehicle.tank_size_liters,
            consumption_rate: 0.0,
            margin_percent: None,
            is_over_limit: false,
        });
    }

    // Find the last fill-up to calculate current consumption rate
    let mut last_fillup_idx = None;
    for (idx, trip) in trips.iter().enumerate().rev() {
        if trip.is_fillup() {
            last_fillup_idx = Some(idx);
            break;
        }
    }

    // Calculate consumption rate and margin from last fill-up
    let (consumption_rate, margin_percent) = if let Some(idx) = last_fillup_idx {
        let fillup_trip = &trips[idx];
        let fuel_liters = fillup_trip.fuel_liters.unwrap();

        // Calculate total distance since last fill-up
        // We need to look back to the previous fill-up (or start of trips)
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

        let rate = calculate_consumption_rate(fuel_liters, km_since_last_fillup);
        let margin = calculate_margin_percent(rate, vehicle.tp_consumption);

        (rate, Some(margin))
    } else {
        // No fill-up yet, use TP consumption
        (vehicle.tp_consumption, None)
    };

    // Calculate current zostatok by processing all trips sequentially
    let mut current_zostatok = vehicle.tank_size_liters; // Start with full tank

    for trip in &trips {
        // Calculate spotreba for this trip
        let spotreba = calculate_spotreba(trip.distance_km, consumption_rate);

        // Update zostatok
        current_zostatok = calculate_zostatok(
            current_zostatok,
            spotreba,
            trip.fuel_liters,
            vehicle.tank_size_liters,
        );
    }

    // Check if over legal limit
    let is_over_limit = if let Some(margin) = margin_percent {
        !is_within_legal_limit(margin)
    } else {
        false
    };

    Ok(TripStats {
        zostatok_liters: current_zostatok,
        consumption_rate,
        margin_percent,
        is_over_limit,
    })
}
