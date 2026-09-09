//! Trip CRUD and route command implementations (framework-free).

use crate::app_state::AppState;
use crate::calculations::time_inference::{compute_inferred_times, Jitter, ThreadRngJitter};
use crate::calculations::trip_copy::compute_copied_trip_defaults;
use crate::check_read_only;
use crate::commands_internal::{
    calculate_trip_numbers, get_year_start_odometer, parse_iso_datetime, trip_order,
};
use crate::db::{normalize_location, Database};
use crate::models::{
    CascadePlan, CascadeResult, CopiedTripDefaults, InferredTripTime, OdometerChange, Route, Trip,
};
use crate::settings::LocalSettings;
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, Utc};
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

/// Build the `Trip` a create writes, from the submitted fields. Shared by
/// `create_trip_internal` and `create_trip_cascade_internal` so the two save
/// paths can never disagree about validation or normalisation.
#[allow(clippy::too_many_arguments)]
fn build_new_trip(
    vehicle_id: &str,
    start_datetime: &str,
    end_datetime: &str,
    origin: &str,
    destination: &str,
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
) -> Result<Trip, String> {
    let vehicle_uuid = Uuid::parse_str(vehicle_id).map_err(|e| e.to_string())?;
    let trip_start_datetime = parse_iso_datetime(start_datetime)?;
    let trip_end_datetime = parse_iso_datetime(end_datetime)?;

    let origin = normalize_location(origin);
    let destination = normalize_location(destination);

    if let Some(soc) = soc_override_percent {
        if !(0.0..=100.0).contains(&soc) {
            return Err("SoC override must be between 0 and 100".to_string());
        }
    }

    let now = Utc::now();
    Ok(Trip {
        id: Uuid::new_v4(),
        vehicle_id: vehicle_uuid,
        start_datetime: trip_start_datetime,
        end_datetime: Some(trip_end_datetime),
        origin,
        destination,
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
        created_at: now,
        updated_at: now,
    })
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
) -> Result<Trip, String> {
    check_read_only!(app_state);
    let trip = build_new_trip(
        &vehicle_id,
        &start_datetime,
        &end_datetime,
        &origin,
        &destination,
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
    )?;

    db.create_trip(&trip).map_err(|e| e.to_string())?;

    db.find_or_create_route(&vehicle_id, &trip.origin, &trip.destination, distance_km)
        .map_err(|e| e.to_string())?;

    Ok(trip)
}

/// Build the `Trip` a save writes, from the submitted fields and the stored row.
/// Shared by `update_trip_internal` and `update_trip_cascade_internal` so the
/// two save paths can never disagree about validation or normalisation.
#[allow(clippy::too_many_arguments)]
fn build_updated_trip(
    db: &Database,
    id: &str,
    start_datetime: &str,
    end_datetime: &str,
    origin: &str,
    destination: &str,
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
    let trip_uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
    let trip_start_datetime = parse_iso_datetime(start_datetime)?;
    let trip_end_datetime = parse_iso_datetime(end_datetime)?;

    let origin = normalize_location(origin);
    let destination = normalize_location(destination);

    if let Some(soc) = soc_override_percent {
        if !(0.0..=100.0).contains(&soc) {
            return Err("SoC override must be between 0 and 100".to_string());
        }
    }

    let existing = db
        .get_trip(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip not found: {}", id))?;

    Ok(Trip {
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
        created_at: existing.created_at,
        updated_at: Utc::now(),
    })
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
    let trip = build_updated_trip(
        db,
        &id,
        &start_datetime,
        &end_datetime,
        &origin,
        &destination,
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
    )?;

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

/// Rewrite the year's odometers so each row starts where the previous one
/// ended, walking `trip_order`. Reports every row it did (or, on a dry run,
/// would) change instead of a bare count, so a correction to a legal record
/// can be reviewed before it is written.
///
/// This replaces a loop that used to run in the browser after every create
/// and every update, rewrote rows the user never touched, and walked the DB
/// order rather than the canonical one (task 80). Nothing calls this
/// automatically now -- it runs only when invoked directly, deliberately.
///
/// `dry_run == true` writes nothing and is always allowed, even in
/// read-only mode, because reading is always allowed. `dry_run == false`
/// writes and is subject to `check_read_only!`.
pub fn recalculate_odometers_internal(
    db: &Database,
    app_state: &AppState,
    vehicle_id: String,
    year: i32,
    dry_run: bool,
) -> Result<Vec<OdometerChange>, String> {
    if !dry_run {
        check_read_only!(app_state);
    }

    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    let mut trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;
    trips.sort_by(|a, b| trip_order(a, b));

    let trip_numbers = calculate_trip_numbers(&trips);

    let mut running = get_year_start_odometer(db, &vehicle_id, year, vehicle.initial_odometer)?;
    let mut changes = Vec::new();

    for trip in trips.iter_mut() {
        running += trip.distance_km;
        if (trip.odometer - running).abs() > 0.001 {
            let trip_number = *trip_numbers.get(&trip.id.to_string()).unwrap_or(&0);
            changes.push(OdometerChange {
                trip_id: trip.id.to_string(),
                trip_number,
                old_odometer: trip.odometer,
                new_odometer: running,
            });

            if !dry_run {
                trip.odometer = running;
                trip.updated_at = Utc::now();
                db.update_trip(trip).map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(changes)
}

/// Largest float difference the cascade treats as "the same number". Matches
/// the tolerance `recalculate_odometers_internal` uses.
const CASCADE_EPSILON: f64 = 0.001;

/// Plan the odometer cascade for one edited row, without touching the database.
///
/// `trips` is every trip of the vehicle in that year, in any order; this
/// function sorts them by `trip_order` itself. `year_start_odometer` is what
/// `get_year_start_odometer` returns, so the first row of a year is anchored
/// on the previous year and not on nothing.
///
/// Which number gives way depends on which one the user changed. A km edit
/// makes the odometer follow (`anchor + km`); an odometer edit makes the km
/// follow (`odometer - anchor`); an edit to neither moves nothing at all, so
/// a save that only fixed a typo in the purpose leaves a deliberately broken
/// row exactly as it was (task 79).
///
/// The walk stops at the end of the year. That is on purpose and it is
/// visible: `year_end_odometer_moved` tells the caller the boundary to the
/// next year has opened by `delta`, and the span warning marks the first row
/// of that year.
///
/// The reported trip numbers come from the book BEFORE the shift, and they
/// stay correct after it. `trip_order` falls through to the odometer only when
/// `start_datetime` and `created_at` both tie, and every member of such a group
/// moves by the same `delta`, so the group keeps its internal order. A shift
/// can therefore never renumber the book.
pub fn plan_odometer_cascade(
    trips: &[Trip],
    year_start_odometer: f64,
    trip_id: &str,
    submitted_distance_km: f64,
    submitted_odometer: f64,
) -> Result<CascadePlan, String> {
    let mut sorted: Vec<&Trip> = trips.iter().collect();
    sorted.sort_by(|a, b| trip_order(a, b));

    let idx = sorted
        .iter()
        .position(|t| t.id.to_string() == trip_id)
        .ok_or_else(|| format!("Trip not found in this year: {}", trip_id))?;

    let stored = sorted[idx];
    let anchor = if idx == 0 {
        year_start_odometer
    } else {
        sorted[idx - 1].odometer
    };

    let km_changed = (submitted_distance_km - stored.distance_km).abs() > CASCADE_EPSILON;
    let odo_changed = (submitted_odometer - stored.odometer).abs() > CASCADE_EPSILON;

    let (new_distance_km, new_odometer) = if km_changed {
        (submitted_distance_km, anchor + submitted_distance_km)
    } else if odo_changed {
        (submitted_odometer - anchor, submitted_odometer)
    } else {
        // Nothing the user did moves the chain. Report a no-op rather than a
        // repair: the row keeps whatever the book records for it.
        return Ok(CascadePlan {
            new_odometer: stored.odometer,
            new_distance_km: stored.distance_km,
            delta: 0.0,
            delta_from_distance: 0.0,
            delta_from_repair: 0.0,
            repair_crosses_year: false,
            year_end_odometer_moved: false,
            next_year_chain_breaks: false,
            changes: Vec::new(),
        });
    };

    // A negative span is never valid: it means the odometer went backwards
    // (task 79). Zero is left alone -- an odometer-correction row can
    // legitimately record no distance, and the HTML input's own `min="0"`
    // (TripRow.svelte) already treats zero as the floor, not below it.
    if new_distance_km < -CASCADE_EPSILON {
        return Err(format!(
            "Odometer {:.3} is below the anchor {:.3}: distance would be {:.3} km",
            new_odometer, anchor, new_distance_km
        ));
    }

    let delta = new_odometer - stored.odometer;
    let delta_from_distance = new_distance_km - stored.distance_km;
    let delta_from_repair = delta - delta_from_distance;

    let trip_numbers = calculate_trip_numbers(trips);
    let changes = sorted[idx + 1..]
        .iter()
        .map(|t| OdometerChange {
            trip_id: t.id.to_string(),
            trip_number: *trip_numbers.get(&t.id.to_string()).unwrap_or(&0),
            old_odometer: t.odometer,
            new_odometer: t.odometer + delta,
        })
        .collect::<Vec<_>>();

    Ok(CascadePlan {
        new_odometer,
        new_distance_km,
        delta,
        delta_from_distance,
        delta_from_repair,
        repair_crosses_year: idx == 0 && delta_from_repair.abs() > CASCADE_EPSILON,
        year_end_odometer_moved: delta.abs() > CASCADE_EPSILON,
        next_year_chain_breaks: false, // the command knows, this function does not
        changes,
    })
}

/// Plan the cascade for a row that does not exist yet.
///
/// The new row lands where `trip_order` puts it, and its odometer is
/// `anchor + km`. Everything after it then starts `km` later, so the whole
/// tail shifts by exactly the distance of the new row (task 81, R8).
///
/// The position is decided on `start_datetime` and then `created_at` alone. A
/// new row's `created_at` is the moment it is written, so it sorts last inside
/// any group it ties with, and the odometer key of `trip_order` never decides
/// an insert. That is what stops the position and the odometer -- which is
/// derived from the position -- from depending on each other.
///
/// A new row carries no span error, so `delta_from_repair` is always 0.
///
/// Rejects a negative `new_distance_km` (task 9b): the JSON-RPC layer has no
/// `min="0"` the way the HTML input does, so a negative value can arrive
/// directly. Zero is allowed -- see `plan_odometer_cascade`.
pub fn plan_insert_cascade(
    trips: &[Trip],
    year_start_odometer: f64,
    new_start_datetime: NaiveDateTime,
    new_distance_km: f64,
) -> Result<CascadePlan, String> {
    if new_distance_km < -CASCADE_EPSILON {
        return Err(format!(
            "Distance {:.3} km cannot be negative",
            new_distance_km
        ));
    }

    let mut sorted: Vec<&Trip> = trips.iter().collect();
    sorted.sort_by(|a, b| trip_order(a, b));

    // The new row goes after every row whose start_datetime is not later. Its
    // created_at is now, so it also goes after every row it ties with.
    let idx = sorted
        .iter()
        .position(|t| t.start_datetime > new_start_datetime)
        .unwrap_or(sorted.len());

    let anchor = if idx == 0 {
        year_start_odometer
    } else {
        sorted[idx - 1].odometer
    };

    let trip_numbers = calculate_trip_numbers(trips);
    let changes = sorted[idx..]
        .iter()
        .map(|t| OdometerChange {
            trip_id: t.id.to_string(),
            trip_number: *trip_numbers.get(&t.id.to_string()).unwrap_or(&0),
            old_odometer: t.odometer,
            new_odometer: t.odometer + new_distance_km,
        })
        .collect::<Vec<_>>();

    Ok(CascadePlan {
        new_odometer: anchor + new_distance_km,
        new_distance_km,
        delta: new_distance_km,
        delta_from_distance: new_distance_km,
        delta_from_repair: 0.0,
        repair_crosses_year: false,
        year_end_odometer_moved: new_distance_km.abs() > CASCADE_EPSILON,
        next_year_chain_breaks: false,
        changes,
    })
}

/// Plan the cascade for removing a row.
///
/// The row after the deleted one inherits the deleted row's start, so the
/// chain loses exactly the deleted row's SPAN -- `odometer - anchor` -- and
/// not its recorded distance (task 81, R9). The two are the same number when
/// the row was consistent. When it was not, only the span leaves the chain
/// continuous.
pub fn plan_delete_cascade(
    trips: &[Trip],
    year_start_odometer: f64,
    trip_id: &str,
) -> Result<CascadePlan, String> {
    let mut sorted: Vec<&Trip> = trips.iter().collect();
    sorted.sort_by(|a, b| trip_order(a, b));

    let idx = sorted
        .iter()
        .position(|t| t.id.to_string() == trip_id)
        .ok_or_else(|| format!("Trip not found in this year: {}", trip_id))?;

    let removed = sorted[idx];
    let anchor = if idx == 0 {
        year_start_odometer
    } else {
        sorted[idx - 1].odometer
    };

    let span = removed.odometer - anchor;
    let delta = -span;

    let trip_numbers = calculate_trip_numbers(trips);
    let changes = sorted[idx + 1..]
        .iter()
        .map(|t| OdometerChange {
            trip_id: t.id.to_string(),
            trip_number: *trip_numbers.get(&t.id.to_string()).unwrap_or(&0),
            old_odometer: t.odometer,
            new_odometer: t.odometer + delta,
        })
        .collect::<Vec<_>>();

    Ok(CascadePlan {
        new_odometer: removed.odometer,
        new_distance_km: removed.distance_km,
        delta,
        delta_from_distance: -removed.distance_km,
        delta_from_repair: delta + removed.distance_km,
        repair_crosses_year: idx == 0 && (span - removed.distance_km).abs() > CASCADE_EPSILON,
        year_end_odometer_moved: delta.abs() > CASCADE_EPSILON,
        next_year_chain_breaks: false,
        changes,
    })
}

/// A moved year end only matters when a later year has rows to break. Appending
/// to the newest year moves its year end every time and breaks nothing, and a
/// warning on that would fire on the most common action in the app (task 81, R4).
fn mark_next_year_chain_breaks(
    db: &Database,
    vehicle_id: &str,
    year: i32,
    plan: &mut CascadePlan,
) -> Result<(), String> {
    if !plan.year_end_odometer_moved {
        return Ok(());
    }
    let years = db.get_years_with_trips(vehicle_id).map_err(|e| e.to_string())?;
    plan.next_year_chain_breaks = years.iter().any(|y| *y > year);
    Ok(())
}

/// Save one row and move every later row of the same year by the same amount.
///
/// This is the save path the grid uses. `update_trip_internal` stays beside it
/// and writes one row with no cascade: the task 79 correction procedure needs
/// a command that writes exactly what it is given.
///
/// `dry_run == true` writes nothing and is always allowed, even in read-only
/// mode, because reading is always allowed. It is what fills the confirmation
/// modal. The apply call plans again from the stored book rather than
/// replaying the dry run's numbers, so a book that moved in between is
/// corrected against as it is now.
///
/// The row and its shift go to the database in one transaction. They are one
/// correction to a legal record, so a partial write is never acceptable.
#[allow(clippy::too_many_arguments)]
pub fn update_trip_cascade_internal(
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
    dry_run: bool,
) -> Result<CascadeResult, String> {
    if !dry_run {
        check_read_only!(app_state);
    }

    let existing = db
        .get_trip(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip not found: {}", id))?;

    let submitted_start = parse_iso_datetime(&start_datetime)?;
    let year = existing.start_datetime.year();
    let re_dated = submitted_start != existing.start_datetime;

    let vehicle_id = existing.vehicle_id.to_string();
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    let trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;
    let year_start =
        get_year_start_odometer(db, &vehicle_id, year, vehicle.initial_odometer)?;

    // A re-dated row moves in trip_order, so its old and its new position both
    // shift. That is not modelled here: write the row and cascade nothing.
    let mut plan = if re_dated {
        CascadePlan {
            new_odometer: odometer,
            new_distance_km: distance_km,
            delta: 0.0,
            delta_from_distance: 0.0,
            delta_from_repair: 0.0,
            repair_crosses_year: false,
            year_end_odometer_moved: false,
            next_year_chain_breaks: false,
            changes: Vec::new(),
        }
    } else {
        plan_odometer_cascade(&trips, year_start, &id, distance_km, odometer)?
    };
    mark_next_year_chain_breaks(db, &vehicle_id, year, &mut plan)?;

    if dry_run {
        return Ok(CascadeResult { trip: None, plan });
    }

    let trip = build_updated_trip(
        db,
        &id,
        &start_datetime,
        &end_datetime,
        &origin,
        &destination,
        plan.new_distance_km,
        plan.new_odometer,
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
    )?;

    let shifts: Vec<(String, f64)> = plan
        .changes
        .iter()
        .map(|c| (c.trip_id.clone(), c.new_odometer))
        .collect();
    db.update_trip_with_odometer_shift(&trip, &shifts)
        .map_err(|e| e.to_string())?;

    db.find_or_create_route(
        &trip.vehicle_id.to_string(),
        &trip.origin,
        &trip.destination,
        plan.new_distance_km,
    )
    .map_err(|e| e.to_string())?;

    Ok(CascadeResult { trip: Some(trip), plan })
}

/// Insert a trip and move every later row of the same year by its distance.
///
/// It takes no `odometer` argument. The row's odometer is `anchor + km`, and
/// the anchor comes from the book, so the browser has nothing left to guess
/// (ADR-008, task 81 R8).
#[allow(clippy::too_many_arguments)]
pub fn create_trip_cascade_internal(
    db: &Database,
    app_state: &AppState,
    vehicle_id: String,
    start_datetime: String,
    end_datetime: String,
    origin: String,
    destination: String,
    distance_km: f64,
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
    dry_run: bool,
) -> Result<CascadeResult, String> {
    if !dry_run {
        check_read_only!(app_state);
    }

    let trip_start_datetime = parse_iso_datetime(&start_datetime)?;
    let year = trip_start_datetime.year();

    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;
    let trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;
    let year_start =
        get_year_start_odometer(db, &vehicle_id, year, vehicle.initial_odometer)?;

    let mut plan =
        plan_insert_cascade(&trips, year_start, trip_start_datetime, distance_km)?;
    mark_next_year_chain_breaks(db, &vehicle_id, year, &mut plan)?;

    if dry_run {
        return Ok(CascadeResult { trip: None, plan });
    }

    let trip = build_new_trip(
        &vehicle_id,
        &start_datetime,
        &end_datetime,
        &origin,
        &destination,
        distance_km,
        plan.new_odometer,
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
    )?;

    let shifts: Vec<(String, f64)> = plan
        .changes
        .iter()
        .map(|c| (c.trip_id.clone(), c.new_odometer))
        .collect();
    db.create_trip_with_odometer_shift(&trip, &shifts)
        .map_err(|e| e.to_string())?;

    db.find_or_create_route(&vehicle_id, &trip.origin, &trip.destination, distance_km)
        .map_err(|e| e.to_string())?;

    Ok(CascadeResult { trip: Some(trip), plan })
}

/// Remove a trip and close the gap it leaves in the odometer chain.
///
/// The rows after it move by the removed row's SPAN, not by its recorded
/// distance -- the span is what the chain loses (task 81, R9).
pub fn delete_trip_cascade_internal(
    db: &Database,
    app_state: &AppState,
    id: String,
    dry_run: bool,
) -> Result<CascadePlan, String> {
    if !dry_run {
        check_read_only!(app_state);
    }

    let existing = db
        .get_trip(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip not found: {}", id))?;

    let vehicle_id = existing.vehicle_id.to_string();
    let year = existing.start_datetime.year();
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;
    let trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;
    let year_start =
        get_year_start_odometer(db, &vehicle_id, year, vehicle.initial_odometer)?;

    let mut plan = plan_delete_cascade(&trips, year_start, &id)?;
    mark_next_year_chain_breaks(db, &vehicle_id, year, &mut plan)?;

    if dry_run {
        return Ok(plan);
    }

    let shifts: Vec<(String, f64)> = plan
        .changes
        .iter()
        .map(|c| (c.trip_id.clone(), c.new_odometer))
        .collect();
    db.delete_trip_with_odometer_shift(&id, &shifts)
        .map_err(|e| e.to_string())?;

    Ok(plan)
}

pub fn delete_trip_internal(
    db: &Database,
    app_state: &AppState,
    id: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.delete_trip(&id).map_err(|e| e.to_string())
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
    // Default OFF — None and Some(false) both disable inference (opt-in).
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

/// Seed values for a row copied from an existing trip.
///
/// Thin wrapper around [`compute_copied_trip_defaults`]: supplies the DB read
/// and the clock so the rule itself stays pure and unit-testable.
///
/// `Local` is the *host's* timezone. On the desktop that is the user's calendar
/// day, which is what "today" should mean. In server mode it is the server's —
/// a container on `TZ=UTC` serving a browser on UTC+2 will date a 01:00 copy to
/// the previous day. That is a known divergence: the grid's own `defaultNewDate`
/// derives its date in UTC, so the two new-row paths can disagree by a day near
/// midnight. Fixing it properly means taking the client's date as a parameter.
pub fn get_copied_trip_defaults_internal(
    db: &Database,
    trip_id: String,
    year: i32,
) -> Result<CopiedTripDefaults, String> {
    let source = db
        .get_trip(&trip_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip not found: {}", trip_id))?;

    compute_copied_trip_defaults(&source, year, Local::now().date_naive())
}
