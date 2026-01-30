//! Trip CRUD and route commands.

use crate::app_state::AppState;
use crate::check_read_only;
use crate::db::{normalize_location, Database};
use crate::models::{Route, Trip};
use chrono::Utc;
use tauri::State;
use uuid::Uuid;

use super::parse_iso_datetime;

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
pub fn get_years_with_trips(db: State<Database>, vehicle_id: String) -> Result<Vec<i32>, String> {
    db.get_years_with_trips(&vehicle_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_trip(
    db: State<Database>,
    app_state: State<AppState>,
    vehicle_id: String,
    start_datetime: String, // Full ISO datetime "YYYY-MM-DDTHH:MM"
    end_datetime: String,   // Full ISO datetime "YYYY-MM-DDTHH:MM"
    origin: String,
    destination: String,
    distance_km: f64,
    odometer: f64,
    purpose: String,
    // Fuel fields (ICE + PHEV)
    fuel_liters: Option<f64>,
    fuel_cost: Option<f64>,
    full_tank: Option<bool>,
    // Energy fields (BEV + PHEV)
    energy_kwh: Option<f64>,
    energy_cost_eur: Option<f64>,
    full_charge: Option<bool>,
    soc_override_percent: Option<f64>,
    // Other
    other_costs: Option<f64>,
    other_costs_note: Option<String>,
    insert_at_position: Option<i32>,
) -> Result<Trip, String> {
    check_read_only!(app_state);
    let vehicle_uuid = Uuid::parse_str(&vehicle_id).map_err(|e| e.to_string())?;
    let trip_start_datetime = parse_iso_datetime(&start_datetime)?;
    let trip_end_datetime = parse_iso_datetime(&end_datetime)?;

    // Normalize locations to prevent whitespace-based duplicates
    let origin = normalize_location(&origin);
    let destination = normalize_location(&destination);

    // Validate: SoC override must be 0-100 if provided
    if let Some(soc) = soc_override_percent {
        if !(0.0..=100.0).contains(&soc) {
            return Err("SoC override must be between 0 and 100".to_string());
        }
    }

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
        start_datetime: trip_start_datetime,
        end_datetime: Some(trip_end_datetime),
        origin: origin.clone(),
        destination: destination.clone(),
        distance_km,
        odometer,
        purpose,
        fuel_liters,
        fuel_cost_eur: fuel_cost,
        full_tank: full_tank.unwrap_or(true), // Default to full tank
        energy_kwh,
        energy_cost_eur,
        full_charge: full_charge.unwrap_or(false),
        soc_override_percent,
        other_costs_eur: other_costs,
        other_costs_note,
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
    app_state: State<AppState>,
    id: String,
    start_datetime: String, // Full ISO datetime "YYYY-MM-DDTHH:MM"
    end_datetime: String,   // Full ISO datetime "YYYY-MM-DDTHH:MM"
    origin: String,
    destination: String,
    distance_km: f64,
    odometer: f64,
    purpose: String,
    // Fuel fields (ICE + PHEV)
    fuel_liters: Option<f64>,
    fuel_cost_eur: Option<f64>,
    full_tank: Option<bool>,
    // Energy fields (BEV + PHEV)
    energy_kwh: Option<f64>,
    energy_cost_eur: Option<f64>,
    full_charge: Option<bool>,
    soc_override_percent: Option<f64>,
    // Other
    other_costs_eur: Option<f64>,
    other_costs_note: Option<String>,
) -> Result<Trip, String> {
    check_read_only!(app_state);
    let trip_uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let trip_start_datetime = parse_iso_datetime(&start_datetime)?;
    let trip_end_datetime = parse_iso_datetime(&end_datetime)?;

    // Normalize locations to prevent whitespace-based duplicates
    let origin = normalize_location(&origin);
    let destination = normalize_location(&destination);

    // Validate: SoC override must be 0-100 if provided
    if let Some(soc) = soc_override_percent {
        if !(0.0..=100.0).contains(&soc) {
            return Err("SoC override must be between 0 and 100".to_string());
        }
    }

    // Get the existing trip to preserve vehicle_id and created_at
    let existing = db
        .get_trip(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip not found: {}", id))?;

    let trip = Trip {
        id: trip_uuid,
        vehicle_id: existing.vehicle_id,
        start_datetime: trip_start_datetime,
        end_datetime: Some(trip_end_datetime),
        origin,
        destination,
        distance_km,
        odometer,
        purpose,
        fuel_liters,
        fuel_cost_eur,
        full_tank: full_tank.unwrap_or(existing.full_tank),
        energy_kwh,
        energy_cost_eur,
        full_charge: full_charge.unwrap_or(existing.full_charge),
        soc_override_percent,
        other_costs_eur,
        other_costs_note,
        sort_order: existing.sort_order,
        created_at: existing.created_at,
        updated_at: Utc::now(),
    };

    db.update_trip(&trip).map_err(|e| e.to_string())?;

    // Update or create route for autocomplete (same as create_trip)
    db.find_or_create_route(
        &trip.vehicle_id.to_string(),
        &trip.origin,
        &trip.destination,
        distance_km,
    )
    .map_err(|e| e.to_string())?;

    Ok(trip)
}

#[tauri::command]
pub fn delete_trip(
    db: State<Database>,
    app_state: State<AppState>,
    id: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.delete_trip(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_trip(
    db: State<Database>,
    app_state: State<AppState>,
    trip_id: String,
    new_sort_order: i32,
) -> Result<Vec<Trip>, String> {
    check_read_only!(app_state);
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

#[tauri::command]
pub fn get_purposes(db: State<Database>, vehicle_id: String) -> Result<Vec<String>, String> {
    db.get_purposes_for_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())
}
