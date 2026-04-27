//! Trip CRUD and route command implementations (framework-free).

use crate::app_state::AppState;
use crate::calculations::time_inference::{compute_inferred_times, Jitter, ThreadRngJitter};
use crate::check_read_only;
use crate::commands_internal::parse_iso_datetime;
use crate::db::{normalize_location, Database};
use crate::models::{InferredTripTime, Route, Trip};
use chrono::{NaiveDate, Utc};
use std::path::Path;
use uuid::Uuid;

pub fn get_trips_internal(db: &Database, vehicle_id: String) -> Result<Vec<Trip>, String> {
    db.get_trips_for_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())
}

pub fn get_trips_for_year_internal(
    db: &Database,
    vehicle_id: String,
    year: i32,
) -> Result<Vec<Trip>, String> {
    db.get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())
}

pub fn get_years_with_trips_internal(
    db: &Database,
    vehicle_id: String,
) -> Result<Vec<i32>, String> {
    db.get_years_with_trips(&vehicle_id)
        .map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
pub fn create_trip_internal(
    db: &Database,
    app_state: &AppState,
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
    check_read_only!(app_state);
    let vehicle_uuid = Uuid::parse_str(&vehicle_id).map_err(|e| e.to_string())?;
    let trip_start_datetime = parse_iso_datetime(&start_datetime)?;
    let trip_end_datetime = parse_iso_datetime(&end_datetime)?;

    let origin = normalize_location(&origin);
    let destination = normalize_location(&destination);

    if let Some(soc) = soc_override_percent {
        if !(0.0..=100.0).contains(&soc) {
            return Err("SoC override must be between 0 and 100".to_string());
        }
    }

    let sort_order = if let Some(position) = insert_at_position {
        db.shift_trips_from_position(&vehicle_id, position)
            .map_err(|e| e.to_string())?;
        position
    } else {
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
        full_tank: full_tank.unwrap_or(true),
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

    db.find_or_create_route(&vehicle_id, &origin, &destination, distance_km)
        .map_err(|e| e.to_string())?;

    Ok(trip)
}

#[allow(clippy::too_many_arguments)]
pub fn update_trip_internal(
    db: &Database,
    app_state: &AppState,
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
    check_read_only!(app_state);
    let trip_uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let trip_start_datetime = parse_iso_datetime(&start_datetime)?;
    let trip_end_datetime = parse_iso_datetime(&end_datetime)?;

    let origin = normalize_location(&origin);
    let destination = normalize_location(&destination);

    if let Some(soc) = soc_override_percent {
        if !(0.0..=100.0).contains(&soc) {
            return Err("SoC override must be between 0 and 100".to_string());
        }
    }

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

    db.find_or_create_route(
        &trip.vehicle_id.to_string(),
        &trip.origin,
        &trip.destination,
        distance_km,
    )
    .map_err(|e| e.to_string())?;

    Ok(trip)
}

pub fn delete_trip_internal(
    db: &Database,
    app_state: &AppState,
    id: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.delete_trip(&id).map_err(|e| e.to_string())
}

pub fn reorder_trip_internal(
    db: &Database,
    app_state: &AppState,
    trip_id: String,
    new_sort_order: i32,
) -> Result<Vec<Trip>, String> {
    check_read_only!(app_state);
    let trip = db
        .get_trip(&trip_id)
        .map_err(|e| e.to_string())?
        .ok_or("Trip not found")?;

    db.reorder_trip(&trip_id, new_sort_order)
        .map_err(|e| e.to_string())?;

    db.get_trips_for_vehicle(&trip.vehicle_id.to_string())
        .map_err(|e| e.to_string())
}

pub fn get_routes_internal(db: &Database, vehicle_id: String) -> Result<Vec<Route>, String> {
    db.get_routes_for_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())
}

pub fn get_purposes_internal(db: &Database, vehicle_id: String) -> Result<Vec<String>, String> {
    db.get_purposes_for_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())
}

pub fn get_inferred_trip_time_for_route_internal(
    db: &Database,
    app_dir: &Path,
    vehicle_id: String,
    origin: String,
    destination: String,
    row_date: String,
) -> Result<Option<InferredTripTime>, String> {
    // Gate on `infer_trip_times` setting — default OFF.
    use crate::settings::LocalSettings;
    let settings = LocalSettings::load(app_dir);
    if !settings.infer_trip_times.unwrap_or(false) {
        return Ok(None);
    }

    let row_date = NaiveDate::parse_from_str(&row_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid row_date (expected YYYY-MM-DD): {}", e))?;
    let mut jitter = ThreadRngJitter;
    inferred_trip_time_for_route(db, &mut jitter, &vehicle_id, &origin, &destination, row_date)
}

/// Inner, testable seam: takes any `Jitter` so unit tests can stub randomness.
/// Returns `None` when no completed historical trip matches the route.
pub fn inferred_trip_time_for_route(
    db: &Database,
    jitter: &mut dyn Jitter,
    vehicle_id: &str,
    origin: &str,
    destination: &str,
    row_date: NaiveDate,
) -> Result<Option<InferredTripTime>, String> {
    let origin = normalize_location(origin);
    let destination = normalize_location(destination);

    let times = db
        .find_most_recent_trip_times_for_route(vehicle_id, &origin, &destination)
        .map_err(|e| e.to_string())?;

    let Some((base_start_dt, base_end_dt)) = times else {
        return Ok(None);
    };

    let base_duration_mins = (base_end_dt - base_start_dt).num_minutes();
    let (start, end) =
        compute_inferred_times(row_date, base_start_dt.time(), base_duration_mins, jitter);

    Ok(Some(InferredTripTime {
        start_datetime: start.format("%Y-%m-%dT%H:%M:%S").to_string(),
        end_datetime: end.format("%Y-%m-%dT%H:%M:%S").to_string(),
    }))
}
