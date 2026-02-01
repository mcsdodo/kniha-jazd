//! Trip statistics and grid data calculation commands.
//!
//! This module contains commands for:
//! - `get_trip_grid_data` - Pre-calculated grid data for frontend display
//! - `calculate_trip_stats` - Aggregated trip statistics (totals, averages)
//! - `calculate_magic_fill_liters` - Suggested fuel for magic fill feature
//! - `preview_trip_calculation` - Live preview of trip calculations

use crate::calculations::{
    calculate_buffer_km, calculate_closed_period_totals, calculate_consumption_rate,
    calculate_fuel_level, calculate_fuel_used, calculate_margin_percent, is_within_legal_limit,
};
use crate::calculations::energy::{
    calculate_battery_remaining, calculate_energy_used, kwh_to_percent,
};
use crate::calculations::phev::calculate_phev_trip_consumption;
use crate::constants::defaults;
use crate::db::Database;
use crate::models::{
    PreviewResult, Receipt, SuggestedFillup, Trip, TripGridData, TripStats, Vehicle,
    VehicleType,
};
use chrono::{NaiveDate, Utc};
use std::collections::{HashMap, HashSet};
use tauri::State;
use uuid::Uuid;

use super::{calculate_odometer_start, calculate_trip_numbers, generate_month_end_rows};

// ============================================================================
// Trip Statistics Commands
// ============================================================================

#[tauri::command]
pub fn calculate_trip_stats(
    vehicle_id: String,
    year: i32,
    db: State<Database>,
) -> Result<TripStats, String> {
    // Get vehicle (validate UUID format first)
    let _vehicle_uuid = Uuid::parse_str(&vehicle_id).map_err(|e| e.to_string())?;
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    // Extract fuel fields (ICE vehicles only for now)
    // TODO: Phase 2 will add BEV/PHEV handling based on vehicle.vehicle_type
    let tank_size = vehicle.tank_size_liters.unwrap_or_default();
    let tp_consumption = vehicle.tp_consumption.unwrap_or_default();

    // Get all trips for this vehicle, sorted by date + odometer (for same-day trips)
    let mut trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;
    trips.sort_by(|a, b| {
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| {
                a.odometer
                    .partial_cmp(&b.odometer)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // If no trips, return default values
    if trips.is_empty() {
        return Ok(TripStats {
            fuel_remaining_liters: tank_size,
            avg_consumption_rate: 0.0,
            last_consumption_rate: 0.0,
            margin_percent: None,
            is_over_limit: false,
            total_km: 0.0,
            total_fuel_liters: 0.0,
            total_fuel_cost_eur: 0.0,
            buffer_km: 0.0,
        });
    }

    // Calculate totals (all trips for display)
    let total_fuel: f64 = trips.iter().filter_map(|t| t.fuel_liters).sum();
    let total_fuel_cost: f64 = trips.iter().filter_map(|t| t.fuel_cost_eur).sum();
    let total_km: f64 = trips.iter().map(|t| t.distance_km).sum();

    // Calculate average consumption from CLOSED periods only (for accurate margin)
    let (closed_fuel, closed_km) = calculate_closed_period_totals(&trips);
    let avg_consumption_rate = if closed_km > 0.0 {
        (closed_fuel / closed_km) * 100.0
    } else {
        0.0
    };

    // Find the last fill-up to calculate current consumption rate
    let mut last_fillup_idx = None;
    for (idx, trip) in trips.iter().enumerate().rev() {
        if trip.is_fillup() {
            last_fillup_idx = Some(idx);
            break;
        }
    }

    // Calculate last consumption rate from last fill-up (for current period tracking)
    let last_consumption_rate = if let Some(idx) = last_fillup_idx {
        let fillup_trip = &trips[idx];
        let fuel_liters = fillup_trip.fuel_liters.unwrap();

        // Calculate total distance since previous fill-up
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

        calculate_consumption_rate(fuel_liters, km_since_last_fillup)
    } else {
        // No fill-up yet, use TP consumption
        tp_consumption
    };

    // Calculate current fuel level by processing all trips sequentially
    // Note: For accurate fuel level, we should use per-period rates, but for header display
    // we use the last consumption rate as a reasonable approximation
    // Start with carryover from previous year (or full tank if no previous data)
    let mut current_fuel =
        get_year_start_fuel_remaining(&db, &vehicle_id, year, tank_size, tp_consumption)?;

    for trip in &trips {
        // Calculate fuel used for this trip
        let fuel_used = calculate_fuel_used(trip.distance_km, last_consumption_rate);

        // Update fuel level
        current_fuel = calculate_fuel_level(current_fuel, fuel_used, trip.fuel_liters, tank_size);
    }

    // Check if over legal limit - ANY fill-up window must be within 120% of TP
    // (not just the average, since each window is separately auditable)
    // Use the WORST window's margin for display (that's what triggers the warning)
    let (worst_rate, worst_margin, is_over_limit) = if total_fuel > 0.0 {
        get_worst_period_stats(&trips, tp_consumption)
    } else {
        (0.0, 0.0, false)
    };

    // Calculate buffer km needed to reach 18% target margin for the worst period
    const TARGET_MARGIN: f64 = 0.18; // 18% - safe buffer below 20% legal limit
    let buffer_km = if is_over_limit {
        // Use worst period's fuel/km for buffer calculation
        // Since we track the worst rate, we can derive the needed buffer
        // Buffer = how much more km needed to bring worst_rate down to target
        // For simplicity, use closed totals (conservative estimate)
        calculate_buffer_km(closed_fuel, closed_km, tp_consumption, TARGET_MARGIN)
    } else {
        0.0
    };

    // Show the WORST window's margin (not average) - that's what triggers warnings
    let display_margin = if closed_km > 0.0 && worst_rate > 0.0 {
        Some(worst_margin)
    } else {
        None
    };

    Ok(TripStats {
        fuel_remaining_liters: current_fuel,
        avg_consumption_rate,
        last_consumption_rate,
        margin_percent: display_margin,
        is_over_limit,
        total_km,
        total_fuel_liters: total_fuel,
        total_fuel_cost_eur: total_fuel_cost,
        buffer_km,
    })
}

/// Get the starting fuel remaining for a year (carryover from previous year).
/// If there are trips in the previous year, returns the ending fuel state of that year.
/// Otherwise, returns full tank (initial state for the vehicle).
pub(crate) fn get_year_start_fuel_remaining(
    db: &Database,
    vehicle_id: &str,
    year: i32,
    tank_size: f64,
    tp_consumption: f64,
) -> Result<f64, String> {
    // Try to get trips from previous year
    let prev_year = year - 1;
    let prev_trips = db
        .get_trips_for_vehicle_in_year(vehicle_id, prev_year)
        .map_err(|e| e.to_string())?;

    if prev_trips.is_empty() {
        // No previous year data - start with full tank
        return Ok(tank_size);
    }

    // Sort previous year's trips chronologically
    let mut chronological = prev_trips;
    chronological.sort_by(|a, b| {
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| {
                a.odometer
                    .partial_cmp(&b.odometer)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // Calculate rates for previous year
    let (rates, _) = calculate_period_rates(&chronological, tp_consumption);

    // Get the starting fuel for the previous year (recursive carryover)
    let prev_year_start =
        get_year_start_fuel_remaining(db, vehicle_id, prev_year, tank_size, tp_consumption)?;

    // Calculate fuel remaining for each trip, then get the last one (year-end state)
    let fuel_remaining =
        calculate_fuel_remaining(&chronological, &rates, prev_year_start, tank_size);

    // Get the last trip's fuel remaining (year-end state)
    let last_trip_id = chronological.last().map(|t| t.id.to_string());
    let year_end_fuel = last_trip_id
        .and_then(|id| fuel_remaining.get(&id).copied())
        .unwrap_or(tank_size);

    Ok(year_end_fuel)
}

/// Get the starting battery (kWh) for a year (carryover from previous year).
/// If there are trips in the previous year, returns the ending battery of that year.
/// Otherwise, returns the vehicle's initial battery (initial_battery_percent × capacity).
pub(crate) fn get_year_start_battery_remaining(
    db: &Database,
    vehicle_id: &str,
    year: i32,
    vehicle: &Vehicle,
) -> Result<f64, String> {
    let capacity = vehicle.battery_capacity_kwh.unwrap_or(0.0);
    let baseline_rate = vehicle.baseline_consumption_kwh.unwrap_or(0.0);

    if capacity <= 0.0 {
        return Ok(0.0);
    }

    // Try to get trips from previous year
    let prev_year = year - 1;
    let prev_trips = db
        .get_trips_for_vehicle_in_year(vehicle_id, prev_year)
        .map_err(|e| e.to_string())?;

    if prev_trips.is_empty() {
        // No previous year data - start with initial battery
        let initial_percent = vehicle.initial_battery_percent.unwrap_or(100.0);
        return Ok(capacity * initial_percent / 100.0);
    }

    // Sort previous year's trips chronologically
    let mut chronological = prev_trips;
    chronological.sort_by(|a, b| {
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| {
                a.odometer
                    .partial_cmp(&b.odometer)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // Get the starting battery for the previous year (recursive carryover)
    let prev_year_start = get_year_start_battery_remaining(db, vehicle_id, prev_year, vehicle)?;

    // Calculate battery remaining for each trip, then get the last one (year-end state)
    let mut current_battery = prev_year_start;

    for trip in &chronological {
        // Check for SoC override
        if let Some(override_percent) = trip.soc_override_percent {
            current_battery = capacity * override_percent / 100.0;
        }

        // Calculate energy used
        let energy_used = calculate_energy_used(trip.distance_km, baseline_rate);

        // Update battery
        current_battery =
            calculate_battery_remaining(current_battery, energy_used, trip.energy_kwh, capacity);
    }

    Ok(current_battery)
}

/// Get the starting odometer for a year (carryover from previous year).
/// If there are trips in the previous year, returns the ending odometer of that year.
/// Otherwise, returns the vehicle's initial odometer.
pub(crate) fn get_year_start_odometer(
    db: &Database,
    vehicle_id: &str,
    year: i32,
    initial_odometer: f64,
) -> Result<f64, String> {
    // Try to find trips from previous years (up to 10 years back)
    let min_year = year - 10;
    let mut check_year = year - 1;

    while check_year >= min_year {
        let trips = db
            .get_trips_for_vehicle_in_year(vehicle_id, check_year)
            .map_err(|e| e.to_string())?;

        if !trips.is_empty() {
            // Found trips - sort and get the last one's odometer
            let mut chronological = trips;
            chronological.sort_by(|a, b| {
                a.start_datetime
                    .date()
                    .cmp(&b.start_datetime.date())
                    .then_with(|| {
                        a.odometer
                            .partial_cmp(&b.odometer)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            });
            return Ok(chronological
                .last()
                .map(|t| t.odometer)
                .unwrap_or(initial_odometer));
        }
        check_year -= 1;
    }

    // No data found in reasonable range - use vehicle's initial odometer
    Ok(initial_odometer)
}

// ============================================================================
// Trip Grid Data
// ============================================================================

/// Internal function to build trip grid data - single source of truth.
/// Used by both get_trip_grid_data command and export functions.
pub(crate) fn build_trip_grid_data(
    db: &Database,
    vehicle_id: &str,
    year: i32,
) -> Result<TripGridData, String> {
    // Get vehicle for TP consumption and tank size
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    // Get trips sorted by sort_order (for display)
    let trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;

    // Get vehicle specs needed for year-start calculations
    let tp_consumption = vehicle.tp_consumption.unwrap_or_default();
    let tank_size = vehicle.tank_size_liters.unwrap_or_default();

    // Calculate year starting values (carryover from previous year)
    let year_start_odometer =
        get_year_start_odometer(db, vehicle_id, year, vehicle.initial_odometer)?;

    let year_start_fuel =
        get_year_start_fuel_remaining(db, vehicle_id, year, tank_size, tp_consumption)?;

    if trips.is_empty() {
        return Ok(TripGridData {
            trips: vec![],
            rates: HashMap::new(),
            estimated_rates: HashSet::new(),
            fuel_consumed: HashMap::new(),
            fuel_remaining: HashMap::new(),
            consumption_warnings: HashSet::new(),
            energy_rates: HashMap::new(),
            estimated_energy_rates: HashSet::new(),
            battery_remaining_kwh: HashMap::new(),
            battery_remaining_percent: HashMap::new(),
            soc_override_trips: HashSet::new(),
            date_warnings: HashSet::new(),
            missing_receipts: HashSet::new(),
            year_start_odometer,
            year_start_fuel,
            suggested_fillup: HashMap::new(),
            legend_suggested_fillup: None,
            trip_numbers: HashMap::new(),
            odometer_start: HashMap::new(),
            month_end_rows: generate_month_end_rows(
                &[],
                year,
                year_start_odometer,
                year_start_fuel,
                &HashMap::new(),
                &HashMap::new(), // No trips = no trip numbers
            ),
        });
    }

    // Get all receipts for matching
    let receipts = db.get_all_receipts().map_err(|e| e.to_string())?;

    // Sort chronologically for calculations (by date, then odometer)
    let mut chronological = trips.clone();
    chronological.sort_by(|a, b| {
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| {
                a.odometer
                    .partial_cmp(&b.odometer)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // Calculate date warnings (trips sorted by sort_order)
    let date_warnings = calculate_date_warnings(&trips);

    // Populate SoC override trips (works for all vehicle types)
    let soc_override_trips: HashSet<String> = trips
        .iter()
        .filter(|t| t.soc_override_percent.is_some())
        .map(|t| t.id.to_string())
        .collect();

    // Calculate missing receipts (trips with fuel but no matching receipt)
    let missing_receipts = calculate_missing_receipts(&trips, &receipts);

    // Calculate initial battery for BEV/PHEV (carryover from previous year)
    let initial_battery = if vehicle.vehicle_type.uses_electricity() {
        get_year_start_battery_remaining(db, vehicle_id, year, &vehicle)?
    } else {
        0.0
    };

    // Calculate rates, fuel, and energy based on vehicle type
    let (
        rates,
        estimated_rates,
        fuel_remaining,
        consumption_warnings,
        energy_rates,
        estimated_energy_rates,
        battery_remaining_kwh,
        battery_remaining_percent,
    ) = match vehicle.vehicle_type {
        VehicleType::Ice => {
            // ICE: Fuel calculations only
            let (rates, estimated_rates) = calculate_period_rates(&chronological, tp_consumption);
            let fuel_remaining =
                calculate_fuel_remaining(&chronological, &rates, year_start_fuel, tank_size);
            let consumption_warnings =
                calculate_consumption_warnings(&trips, &rates, tp_consumption);
            (
                rates,
                estimated_rates,
                fuel_remaining,
                consumption_warnings,
                HashMap::new(),
                HashSet::new(),
                HashMap::new(),
                HashMap::new(),
            )
        }
        VehicleType::Bev => {
            // BEV: Energy calculations only, no fuel
            let (energy_rates, estimated_energy_rates, battery_kwh, battery_percent) =
                calculate_energy_grid_data(&chronological, &vehicle, initial_battery);
            (
                HashMap::new(),
                HashSet::new(),
                HashMap::new(),
                HashSet::new(), // No consumption warnings for BEV
                energy_rates,
                estimated_energy_rates,
                battery_kwh,
                battery_percent,
            )
        }
        VehicleType::Phev => {
            // PHEV: Both fuel and energy, using PHEV-specific calculations
            let phev_data = calculate_phev_grid_data(
                &chronological,
                &vehicle,
                year_start_fuel,
                initial_battery,
            );
            // Calculate consumption warnings for fuel portion only
            let consumption_warnings =
                calculate_consumption_warnings(&trips, &phev_data.fuel_rates, tp_consumption);
            (
                phev_data.fuel_rates,
                phev_data.estimated_fuel_rates,
                phev_data.fuel_remaining,
                consumption_warnings,
                phev_data.energy_rates,
                phev_data.estimated_energy_rates,
                phev_data.battery_remaining_kwh,
                phev_data.battery_remaining_percent,
            )
        }
    };

    // Calculate fuel consumed per trip (uses the same rates already calculated)
    let fuel_consumed = calculate_fuel_consumed(&chronological, &rates);

    // Calculate suggested fillup for trips in open period (ICE + PHEV only)
    let (suggested_fillup, legend_suggested_fillup) = if vehicle.vehicle_type.uses_fuel() {
        calculate_suggested_fillups(&chronological, tp_consumption)
    } else {
        (HashMap::new(), None)
    };

    // Legal compliance calculations (2026)
    let trip_numbers = calculate_trip_numbers(&trips);
    let odometer_start = calculate_odometer_start(&chronological, year_start_odometer);

    // Generate month-end rows using already-calculated fuel_remaining and trip_numbers
    let month_end_rows = generate_month_end_rows(
        &chronological,
        year,
        year_start_odometer,
        year_start_fuel,
        &fuel_remaining,
        &trip_numbers,
    );

    Ok(TripGridData {
        trips,
        rates,
        estimated_rates,
        fuel_consumed,
        fuel_remaining,
        consumption_warnings,
        energy_rates,
        estimated_energy_rates,
        battery_remaining_kwh,
        battery_remaining_percent,
        soc_override_trips,
        date_warnings,
        missing_receipts,
        year_start_odometer,
        year_start_fuel,
        suggested_fillup,
        legend_suggested_fillup,
        trip_numbers,
        odometer_start,
        month_end_rows,
    })
}

/// Get pre-calculated trip grid data for frontend display.
/// Thin wrapper around build_trip_grid_data for Tauri command.
#[tauri::command]
pub fn get_trip_grid_data(
    db: State<Database>,
    vehicle_id: String,
    year: i32,
) -> Result<TripGridData, String> {
    build_trip_grid_data(&db, &vehicle_id, year)
}

// ============================================================================
// Magic Fill / Suggested Fillup
// ============================================================================

/// Get accumulated km in the current (open) fillup period.
/// Reuses the same logic as calculate_period_rates.
///
/// If `stop_at_trip_id` is provided, only count km up to and including that trip.
/// This is needed when editing a trip in the middle of a period.
pub(crate) fn get_open_period_km(chronological: &[Trip], stop_at_trip_id: Option<&Uuid>) -> f64 {
    let mut km_in_period = 0.0;

    // Same logic as calculate_period_rates - accumulate km until we find a full tank
    for trip in chronological {
        km_in_period += trip.distance_km;

        // If editing a specific trip, stop after we've counted it
        if let Some(stop_id) = stop_at_trip_id {
            if &trip.id == stop_id {
                break;
            }
        }

        if let Some(fuel) = trip.fuel_liters {
            if fuel > 0.0 && trip.full_tank {
                // Period closed by full tank - reset counter
                km_in_period = 0.0;
            }
        }
    }

    km_in_period
}

/// Calculate suggested fillup for all trips in open periods.
/// Returns:
/// - HashMap from trip ID to SuggestedFillup (for magic button per-trip)
/// - Option<SuggestedFillup> for the legend (most recent trip's suggestion)
/// Uses random multiplier 1.05-1.20 (same as magic fill).
pub(crate) fn calculate_suggested_fillups(
    chronological: &[Trip],
    tp_consumption: f64,
) -> (HashMap<String, SuggestedFillup>, Option<SuggestedFillup>) {
    use rand::Rng;

    let mut result = HashMap::new();
    let mut rng = rand::thread_rng();

    // Generate one random multiplier for this calculation batch
    // (provides consistency within a single data load)
    let target_multiplier = rng.gen_range(1.05..=1.20);
    let target_rate = tp_consumption * target_multiplier;

    // First pass: find the index where the open period starts
    // (after the last full tank, or from the beginning if no full tanks)
    let mut open_period_start_idx = 0;
    for (idx, trip) in chronological.iter().enumerate() {
        if let Some(fuel) = trip.fuel_liters {
            if fuel > 0.0 && trip.full_tank {
                // Period closed by full tank - next trip starts a new open period
                open_period_start_idx = idx + 1;
            }
        }
    }

    // Second pass: calculate suggested fillup for each trip in the open period
    let mut cumulative_km = 0.0;
    for trip in chronological.iter().skip(open_period_start_idx) {
        cumulative_km += trip.distance_km;

        if cumulative_km > 0.0 {
            // Calculate liters: liters = km * rate / 100
            let suggested_liters = (cumulative_km * target_rate) / 100.0;
            let rounded_liters = (suggested_liters * 100.0).round() / 100.0;

            // Calculate resulting consumption rate
            let consumption_rate = (rounded_liters / cumulative_km) * 100.0;
            let rounded_rate = (consumption_rate * 100.0).round() / 100.0;

            result.insert(
                trip.id.to_string(),
                SuggestedFillup {
                    liters: rounded_liters,
                    consumption_rate: rounded_rate,
                },
            );
        }
    }

    // Find the legend suggestion: most recent trip (lowest sort_order) that has a suggestion
    let legend = chronological
        .iter()
        .filter(|t| result.contains_key(&t.id.to_string()))
        .min_by_key(|t| t.sort_order)
        .and_then(|t| result.get(&t.id.to_string()).cloned());

    (result, legend)
}

/// Calculate suggested fuel liters for magic fill feature.
/// Returns liters that would result in 105-120% of TP consumption rate.
///
/// Parameters:
/// - `current_trip_km`: The km value from the form (for new trips only)
/// - `editing_trip_id`: If editing an existing trip, pass its ID to avoid double-counting
#[tauri::command]
pub fn calculate_magic_fill_liters(
    db: State<Database>,
    vehicle_id: String,
    year: i32,
    current_trip_km: f64,
    editing_trip_id: Option<String>,
) -> Result<f64, String> {
    use rand::Rng;

    // Get vehicle for TP consumption
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    let tp_consumption = vehicle.tp_consumption.unwrap_or(5.0);

    // Get trips sorted chronologically (same as calculate_period_rates)
    let trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;

    let mut chronological = trips;
    chronological.sort_by(|a, b| {
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| {
                a.odometer
                    .partial_cmp(&b.odometer)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // Parse editing_trip_id to Uuid if provided
    let editing_uuid = editing_trip_id
        .as_ref()
        .and_then(|id| Uuid::parse_str(id).ok());

    // Get accumulated km in current open period (reuses period logic)
    // When editing, only count km up to the edited trip (not trips after it)
    let open_period_km = get_open_period_km(&chronological, editing_uuid.as_ref());

    // For existing trips: their km is already in open_period_km, don't add again
    // For new trips: add current_trip_km to the open period
    let total_km = if editing_uuid.is_some() {
        // Existing trip - km already counted in open_period_km
        open_period_km
    } else {
        // New trip - add its km
        open_period_km + current_trip_km
    };

    if total_km <= 0.0 {
        return Ok(0.0);
    }

    // Random target rate between 105-120% of TP consumption
    let mut rng = rand::thread_rng();
    let target_multiplier = rng.gen_range(1.05..=1.20);
    let target_rate = tp_consumption * target_multiplier;

    // Calculate liters: liters = km * rate / 100
    let suggested_liters = (total_km * target_rate) / 100.0;

    // Round to 2 decimal places
    Ok((suggested_liters * 100.0).round() / 100.0)
}

// ============================================================================
// Period Rate Calculations
// ============================================================================

/// Calculate consumption rates for each trip based on fill-up periods.
/// Only calculates actual rate when a period ends with a full tank fillup.
pub(crate) fn calculate_period_rates(
    chronological: &[Trip],
    tp_consumption: f64,
) -> (HashMap<String, f64>, HashSet<String>) {
    let mut rates = HashMap::new();
    let mut estimated = HashSet::new();

    struct Period {
        trip_ids: Vec<String>,
        rate: f64,
        is_estimated: bool,
    }

    let mut periods: Vec<Period> = vec![];
    let mut current_trip_ids: Vec<String> = vec![];
    let mut km_in_period = 0.0;
    let mut fuel_in_period = 0.0;

    for trip in chronological {
        current_trip_ids.push(trip.id.to_string());
        km_in_period += trip.distance_km;

        if let Some(fuel) = trip.fuel_liters {
            if fuel > 0.0 {
                fuel_in_period += fuel;

                // Only close period on full tank fillup
                if trip.full_tank && km_in_period > 0.0 {
                    let rate = (fuel_in_period / km_in_period) * 100.0;
                    periods.push(Period {
                        trip_ids: current_trip_ids.clone(),
                        rate,
                        is_estimated: false,
                    });
                    current_trip_ids.clear();
                    km_in_period = 0.0;
                    fuel_in_period = 0.0;
                }
            }
        }
    }

    // Remaining trips use TP rate (estimated)
    if !current_trip_ids.is_empty() {
        periods.push(Period {
            trip_ids: current_trip_ids,
            rate: tp_consumption,
            is_estimated: true,
        });
    }

    // Assign rates to trips
    for period in periods {
        for trip_id in period.trip_ids {
            rates.insert(trip_id.clone(), period.rate);
            if period.is_estimated {
                estimated.insert(trip_id);
            }
        }
    }

    (rates, estimated)
}

/// Find the worst (highest) fill-up period's consumption rate and margin.
/// Returns (worst_rate, worst_margin, is_over_limit) for the period with highest consumption.
/// This is stricter than checking the average - for legal compliance,
/// each fill-up window must be within the limit, not just the total average.
fn get_worst_period_stats(trips: &[Trip], tp_consumption: f64) -> (f64, f64, bool) {
    if tp_consumption <= 0.0 {
        return (0.0, 0.0, false);
    }

    let limit = tp_consumption * 1.2; // 120% of TP
    let mut worst_rate = 0.0;
    let mut km_in_period = 0.0;
    let mut fuel_in_period = 0.0;

    for trip in trips {
        km_in_period += trip.distance_km;

        if let Some(fuel) = trip.fuel_liters {
            if fuel > 0.0 {
                fuel_in_period += fuel;

                // Calculate rate when period closes (full tank fillup)
                if trip.full_tank && km_in_period > 0.0 {
                    let rate = (fuel_in_period / km_in_period) * 100.0;
                    if rate > worst_rate {
                        worst_rate = rate;
                    }
                    // Reset for next period
                    km_in_period = 0.0;
                    fuel_in_period = 0.0;
                }
            }
        }
    }

    let worst_margin = calculate_margin_percent(worst_rate, tp_consumption);
    let is_over_limit = worst_rate > limit;

    (worst_rate, worst_margin, is_over_limit)
}

/// Check if any closed fill-up period exceeds the legal consumption limit.
/// Returns true if ANY period's consumption rate is > 120% of TP.
#[cfg(test)]
pub(crate) fn has_any_period_over_limit(trips: &[Trip], tp_consumption: f64) -> bool {
    let (_, _, is_over) = get_worst_period_stats(trips, tp_consumption);
    is_over
}

// ============================================================================
// Fuel Calculations
// ============================================================================

/// Calculate fuel consumed per trip (liters).
/// Formula: distance_km × rate / 100
pub(crate) fn calculate_fuel_consumed(
    trips: &[Trip],
    rates: &HashMap<String, f64>,
) -> HashMap<String, f64> {
    trips
        .iter()
        .map(|trip| {
            let rate = rates.get(&trip.id.to_string()).copied().unwrap_or(0.0);
            let consumed = (trip.distance_km * rate) / 100.0;
            (trip.id.to_string(), consumed)
        })
        .collect()
}

/// Calculate fuel remaining after each trip.
/// `initial_fuel` is the fuel level at the start of the period (carryover from previous year).
pub(crate) fn calculate_fuel_remaining(
    chronological: &[Trip],
    rates: &HashMap<String, f64>,
    initial_fuel: f64,
    tank_size: f64,
) -> HashMap<String, f64> {
    let mut remaining = HashMap::new();
    let mut fuel = initial_fuel;

    for trip in chronological {
        let trip_id = trip.id.to_string();
        let rate = rates.get(&trip_id).copied().unwrap_or(0.0);
        let fuel_used = if rate > 0.0 {
            (trip.distance_km * rate) / 100.0
        } else {
            0.0
        };

        fuel -= fuel_used;

        if let Some(fuel_added) = trip.fuel_liters {
            if fuel_added > 0.0 {
                if trip.full_tank {
                    fuel = tank_size;
                } else {
                    fuel += fuel_added;
                }
            }
        }

        // Clamp to valid range
        fuel = fuel.max(0.0).min(tank_size);
        remaining.insert(trip_id, fuel);
    }

    remaining
}

// ============================================================================
// Energy Calculations (BEV)
// ============================================================================

/// Calculate energy data for BEV vehicles.
/// Returns (energy_rates, estimated_energy_rates, battery_remaining_kwh, battery_remaining_percent)
pub(crate) fn calculate_energy_grid_data(
    chronological: &[Trip],
    vehicle: &Vehicle,
    initial_battery: f64,
) -> (
    HashMap<String, f64>,
    HashSet<String>,
    HashMap<String, f64>,
    HashMap<String, f64>,
) {
    let mut energy_rates = HashMap::new();
    let mut estimated_energy_rates = HashSet::new();
    let mut battery_kwh = HashMap::new();
    let mut battery_percent = HashMap::new();

    let capacity = vehicle.battery_capacity_kwh.unwrap_or(0.0);
    let baseline_rate = vehicle.baseline_consumption_kwh.unwrap_or(0.0);

    if capacity <= 0.0 {
        return (
            energy_rates,
            estimated_energy_rates,
            battery_kwh,
            battery_percent,
        );
    }

    // Initial battery state: use year start carryover
    let mut current_battery = initial_battery;

    // Track charge periods for rate calculation (similar to fuel periods)
    let mut period_energy = 0.0;
    let mut period_km = 0.0;
    let mut period_trip_ids: Vec<String> = Vec::new();

    for trip in chronological {
        let trip_id = trip.id.to_string();

        // Check for SoC override - this resets the battery state
        if let Some(override_percent) = trip.soc_override_percent {
            current_battery = capacity * override_percent / 100.0;
        }

        // Calculate energy used for this trip
        let energy_used = calculate_energy_used(trip.distance_km, baseline_rate);

        // Update battery (subtract used, add charged)
        current_battery =
            calculate_battery_remaining(current_battery, energy_used, trip.energy_kwh, capacity);

        // Store battery remaining
        battery_kwh.insert(trip_id.clone(), current_battery);
        battery_percent.insert(trip_id.clone(), kwh_to_percent(current_battery, capacity));

        // Track period for rate calculation
        period_km += trip.distance_km;
        period_trip_ids.push(trip_id.clone());

        // If this trip has a charge and is marked as full charge, close the period
        if trip.energy_kwh.is_some() && trip.full_charge {
            let charged = trip.energy_kwh.unwrap_or(0.0);
            period_energy += charged;

            // Calculate rate for this period
            let rate = if period_km > 0.0 {
                (period_energy / period_km) * 100.0
            } else {
                baseline_rate
            };

            // Apply rate to all trips in period
            for id in &period_trip_ids {
                energy_rates.insert(id.clone(), rate);
            }

            // Reset period
            period_energy = 0.0;
            period_km = 0.0;
            period_trip_ids.clear();
        } else if let Some(charged) = trip.energy_kwh {
            // Partial charge - accumulate but don't close period
            period_energy += charged;
        }
    }

    // Handle remaining trips without a full charge - use baseline rate (estimated)
    for id in &period_trip_ids {
        energy_rates.insert(id.clone(), baseline_rate);
        estimated_energy_rates.insert(id.clone());
    }

    (
        energy_rates,
        estimated_energy_rates,
        battery_kwh,
        battery_percent,
    )
}

// ============================================================================
// PHEV Calculations
// ============================================================================

/// PHEV grid data calculation result
struct PhevGridData {
    /// Fuel consumption rates (l/100km) - only for km_on_fuel portion
    fuel_rates: HashMap<String, f64>,
    /// Trip IDs with estimated fuel rates
    estimated_fuel_rates: HashSet<String>,
    /// Fuel remaining after each trip (liters)
    fuel_remaining: HashMap<String, f64>,
    /// Energy consumption rates (kWh/100km)
    energy_rates: HashMap<String, f64>,
    /// Trip IDs with estimated energy rates
    estimated_energy_rates: HashSet<String>,
    /// Battery remaining (kWh)
    battery_remaining_kwh: HashMap<String, f64>,
    /// Battery remaining (%)
    battery_remaining_percent: HashMap<String, f64>,
}

/// Calculate PHEV grid data - tracks both fuel and battery state.
/// Uses electricity first until battery depleted, then fuel.
/// Fuel consumption rate is calculated only for the km driven on fuel.
fn calculate_phev_grid_data(
    chronological: &[Trip],
    vehicle: &Vehicle,
    initial_fuel: f64,
    initial_battery: f64,
) -> PhevGridData {
    let mut fuel_rates = HashMap::new();
    let mut estimated_fuel_rates = HashSet::new();
    let mut fuel_remaining = HashMap::new();
    let mut energy_rates = HashMap::new();
    let mut estimated_energy_rates = HashSet::new();
    let mut battery_kwh = HashMap::new();
    let mut battery_percent = HashMap::new();

    let capacity = vehicle.battery_capacity_kwh.unwrap_or(0.0);
    let baseline_energy = vehicle.baseline_consumption_kwh.unwrap_or(18.0); // kWh/100km
    let tp_consumption = vehicle.tp_consumption.unwrap_or(7.0); // l/100km
    let tank_size = vehicle
        .tank_size_liters
        .unwrap_or(defaults::TANK_SIZE_LITERS);

    // Initial battery state: use year start carryover
    let mut current_battery = initial_battery;
    let mut current_fuel = initial_fuel;

    // Fuel period tracking - only count km_on_fuel for rate calculation
    let mut fuel_period_km = 0.0;
    let mut fuel_period_liters = 0.0;
    let mut fuel_period_trip_ids: Vec<String> = Vec::new();

    // Energy period tracking
    let mut energy_period_km = 0.0;
    let mut energy_period_kwh = 0.0;
    let mut energy_period_trip_ids: Vec<String> = Vec::new();

    for trip in chronological {
        let trip_id = trip.id.to_string();

        // Check for SoC override
        if let Some(override_percent) = trip.soc_override_percent {
            current_battery = capacity * override_percent / 100.0;
        }

        // Use PHEV calculation to split km between electric and fuel
        let phev_result = calculate_phev_trip_consumption(
            trip.distance_km,
            current_battery,
            current_fuel,
            trip.energy_kwh,
            trip.fuel_liters,
            baseline_energy,
            tp_consumption,
            capacity,
            tank_size,
        );

        // Update state
        current_battery = phev_result.battery_remaining_kwh;
        current_fuel = phev_result.fuel_remaining_liters;

        // Store remaining values
        battery_kwh.insert(trip_id.clone(), current_battery);
        battery_percent.insert(trip_id.clone(), kwh_to_percent(current_battery, capacity));
        fuel_remaining.insert(trip_id.clone(), current_fuel);

        // Track fuel period (only km_on_fuel counts)
        if phev_result.km_on_fuel > 0.0 {
            fuel_period_km += phev_result.km_on_fuel;
            fuel_period_trip_ids.push(trip_id.clone());
        }

        // Track energy period (only km_on_electricity counts)
        if phev_result.km_on_electricity > 0.0 {
            energy_period_km += phev_result.km_on_electricity;
            energy_period_trip_ids.push(trip_id.clone());
        }

        // Close fuel period on full tank
        if trip.fuel_liters.is_some() && trip.full_tank {
            fuel_period_liters += trip.fuel_liters.unwrap_or(0.0);
            let rate = if fuel_period_km > 0.0 {
                (fuel_period_liters / fuel_period_km) * 100.0
            } else {
                tp_consumption
            };
            for id in &fuel_period_trip_ids {
                fuel_rates.insert(id.clone(), rate);
            }
            fuel_period_km = 0.0;
            fuel_period_liters = 0.0;
            fuel_period_trip_ids.clear();
        } else if let Some(liters) = trip.fuel_liters {
            fuel_period_liters += liters;
        }

        // Close energy period on full charge
        if trip.energy_kwh.is_some() && trip.full_charge {
            energy_period_kwh += trip.energy_kwh.unwrap_or(0.0);
            let rate = if energy_period_km > 0.0 {
                (energy_period_kwh / energy_period_km) * 100.0
            } else {
                baseline_energy
            };
            for id in &energy_period_trip_ids {
                energy_rates.insert(id.clone(), rate);
            }
            energy_period_km = 0.0;
            energy_period_kwh = 0.0;
            energy_period_trip_ids.clear();
        } else if let Some(kwh) = trip.energy_kwh {
            energy_period_kwh += kwh;
        }
    }

    // Handle remaining trips - use baseline rates (estimated)
    for id in &fuel_period_trip_ids {
        fuel_rates.insert(id.clone(), tp_consumption);
        estimated_fuel_rates.insert(id.clone());
    }
    for id in &energy_period_trip_ids {
        energy_rates.insert(id.clone(), baseline_energy);
        estimated_energy_rates.insert(id.clone());
    }

    PhevGridData {
        fuel_rates,
        estimated_fuel_rates,
        fuel_remaining,
        energy_rates,
        estimated_energy_rates,
        battery_remaining_kwh: battery_kwh,
        battery_remaining_percent: battery_percent,
    }
}

// ============================================================================
// Warning Calculations
// ============================================================================

/// Check if each trip's date is out of order relative to neighbors.
/// Trips should be sorted by sort_order (0 = newest at top).
pub(crate) fn calculate_date_warnings(trips_by_sort_order: &[Trip]) -> HashSet<String> {
    let mut warnings = HashSet::new();

    for i in 0..trips_by_sort_order.len() {
        let trip = &trips_by_sort_order[i];
        let prev = if i > 0 {
            Some(&trips_by_sort_order[i - 1])
        } else {
            None
        };
        let next = if i < trips_by_sort_order.len() - 1 {
            Some(&trips_by_sort_order[i + 1])
        } else {
            None
        };

        // sort_order 0 = newest (should have highest date)
        // Check: prev.start_datetime.date() >= trip.start_datetime.date() >= next.start_datetime.date()
        if let Some(p) = prev {
            if trip.start_datetime.date() > p.start_datetime.date() {
                warnings.insert(trip.id.to_string());
            }
        }
        if let Some(n) = next {
            if trip.start_datetime.date() < n.start_datetime.date() {
                warnings.insert(trip.id.to_string());
            }
        }
    }

    warnings
}

/// Check if any trip's consumption rate exceeds 120% of TP rate.
pub(crate) fn calculate_consumption_warnings(
    trips: &[Trip],
    rates: &HashMap<String, f64>,
    tp_consumption: f64,
) -> HashSet<String> {
    let mut warnings = HashSet::new();
    let limit = tp_consumption * 1.2;

    for trip in trips {
        let trip_id = trip.id.to_string();
        if let Some(&rate) = rates.get(&trip_id) {
            if rate > limit {
                warnings.insert(trip_id);
            }
        }
    }

    warnings
}

/// Find trips with fuel that don't have a matching receipt.
/// A trip has a matching receipt if date, liters, and price all match exactly.
/// Trips without fuel don't need receipts.
pub(crate) fn calculate_missing_receipts(trips: &[Trip], receipts: &[Receipt]) -> HashSet<String> {
    let mut missing = HashSet::new();

    for trip in trips {
        // Trips without fuel don't need receipts
        if trip.fuel_liters.is_none() {
            continue;
        }

        // Check if any receipt matches this trip exactly
        let has_match = receipts.iter().any(|r| {
            let date_match = r.receipt_date == Some(trip.start_datetime.date());
            let liters_match = r.liters == trip.fuel_liters;
            let price_match = r.total_price_eur == trip.fuel_cost_eur;
            date_match && liters_match && price_match
        });

        if !has_match {
            missing.insert(trip.id.to_string());
        }
    }

    missing
}

// ============================================================================
// Live Preview
// ============================================================================

/// Calculate preview values for a trip being edited/created.
/// Returns consumption rate, fuel remaining, and margin without saving.
#[tauri::command]
pub fn preview_trip_calculation(
    db: State<Database>,
    vehicle_id: String,
    year: i32,
    distance_km: i32,
    fuel_liters: Option<f64>,
    full_tank: bool,
    insert_at_sort_order: Option<i32>,
    editing_trip_id: Option<String>,
) -> Result<PreviewResult, String> {
    // Get vehicle for TP consumption and tank size
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    // Get existing trips
    let mut trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;

    // Create a virtual trip for preview
    let preview_trip_id = Uuid::new_v4();
    let now = Utc::now();

    // Determine the date and odometer for the preview trip to place it correctly
    // in chronological order for rate calculations
    let (preview_date, preview_odometer) = if let Some(sort_order) = insert_at_sort_order {
        // Inserting above a specific trip - use that trip's date and odometer - 0.5
        // (so it sorts just before the target trip on same date)
        trips
            .iter()
            .find(|t| t.sort_order == sort_order)
            .map(|t| (t.start_datetime.date(), t.odometer - 0.5))
            .unwrap_or_else(|| (NaiveDate::from_ymd_opt(year, 12, 31).unwrap(), 0.0))
    } else {
        // New row at top - use the most recent trip's date and odometer + 0.5
        trips
            .iter()
            .max_by_key(|t| (t.start_datetime.date(), t.odometer as i64))
            .map(|t| (t.start_datetime.date(), t.odometer + 0.5))
            .unwrap_or_else(|| (Utc::now().date_naive(), 0.0))
    };

    let virtual_trip = Trip {
        id: preview_trip_id,
        vehicle_id: Uuid::parse_str(&vehicle_id).unwrap_or_else(|_| Uuid::new_v4()),
        start_datetime: preview_date.and_hms_opt(0, 0, 0).unwrap(),
        end_datetime: None,
        origin: "Preview".to_string(),
        destination: "Preview".to_string(),
        distance_km: distance_km as f64,
        odometer: preview_odometer,
        purpose: "Preview".to_string(),
        fuel_liters,
        fuel_cost_eur: None,
        full_tank,
        // Energy fields (BEV/PHEV) - TODO: Phase 2 will add preview support
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        sort_order: insert_at_sort_order.unwrap_or(0),
        created_at: now,
        updated_at: now,
    };

    // Handle editing vs inserting
    if let Some(edit_id) = &editing_trip_id {
        // Replace existing trip with virtual trip (keeping the ID for lookup)
        if let Some(pos) = trips.iter().position(|t| t.id.to_string() == *edit_id) {
            let existing = &trips[pos];
            // Create a modified trip with the new values but same ID
            let modified_trip = Trip {
                id: existing.id,
                vehicle_id: existing.vehicle_id,
                start_datetime: existing.start_datetime,
                end_datetime: existing.end_datetime,
                origin: existing.origin.clone(),
                destination: existing.destination.clone(),
                distance_km: distance_km as f64,
                odometer: existing.odometer,
                purpose: existing.purpose.clone(),
                fuel_liters,
                fuel_cost_eur: existing.fuel_cost_eur,
                full_tank,
                // Preserve energy fields from existing trip
                energy_kwh: existing.energy_kwh,
                energy_cost_eur: existing.energy_cost_eur,
                full_charge: existing.full_charge,
                soc_override_percent: existing.soc_override_percent,
                other_costs_eur: existing.other_costs_eur,
                other_costs_note: existing.other_costs_note.clone(),
                sort_order: existing.sort_order,
                created_at: existing.created_at,
                updated_at: now,
            };
            trips[pos] = modified_trip;
        }
    } else {
        // Insert new virtual trip at the specified position
        trips.push(virtual_trip);
    }

    // Sort chronologically for calculations
    trips.sort_by(|a, b| {
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| {
                a.odometer
                    .partial_cmp(&b.odometer)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // Calculate rates and remaining fuel (ICE vehicles only for now)
    // TODO: Phase 2 will add BEV/PHEV preview support
    let tp_consumption = vehicle.tp_consumption.unwrap_or_default();
    let tank_size = vehicle.tank_size_liters.unwrap_or_default();

    let (rates, estimated_rates) = calculate_period_rates(&trips, tp_consumption);

    // Get initial fuel (carryover from previous year)
    let initial_fuel =
        get_year_start_fuel_remaining(&db, &vehicle_id, year, tank_size, tp_consumption)?;

    let fuel_remaining = calculate_fuel_remaining(&trips, &rates, initial_fuel, tank_size);

    // Find the preview trip in results
    let target_id = if let Some(edit_id) = editing_trip_id {
        edit_id
    } else {
        preview_trip_id.to_string()
    };

    let consumption_rate = rates.get(&target_id).copied().unwrap_or(tp_consumption);
    let fuel_remaining_value = fuel_remaining.get(&target_id).copied().unwrap_or(tank_size);
    let is_estimated_rate = estimated_rates.contains(&target_id);
    let margin_percent = calculate_margin_percent(consumption_rate, tp_consumption);
    let is_over_limit = !is_within_legal_limit(margin_percent);

    Ok(PreviewResult {
        fuel_remaining: fuel_remaining_value,
        consumption_rate,
        margin_percent,
        is_over_limit,
        is_estimated_rate,
    })
}
