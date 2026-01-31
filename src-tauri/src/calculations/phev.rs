//! PHEV combined calculations - uses electricity first, then fuel
//!
//! PHEV vehicles prioritize electricity: battery is depleted before fuel is used.
//! This module handles the split calculation for a single trip.

use super::energy as calculations_energy;
use crate::calculations;

/// Result of PHEV trip consumption calculation
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PhevTripConsumption {
    // Fields provide complete consumption data even though we currently only use remaining values
    /// km driven on electricity
    pub km_on_electricity: f64,
    /// km driven on fuel
    pub km_on_fuel: f64,
    /// Energy used from battery (kWh)
    pub energy_used_kwh: f64,
    /// Fuel used (liters)
    pub fuel_used_liters: f64,
    /// Battery remaining after trip (kWh)
    pub battery_remaining_kwh: f64,
    /// Fuel remaining after trip (liters)
    pub fuel_remaining_liters: f64,
}

/// Calculate PHEV consumption for a single trip
/// Electricity is used first until battery depleted, then fuel takes over
///
/// # Arguments
/// * `distance_km` - Total trip distance
/// * `previous_battery_kwh` - Battery state before trip (kWh)
/// * `previous_fuel_liters` - Fuel state before trip (liters)
/// * `energy_charged` - Energy added during this trip (kWh)
/// * `fuel_added` - Fuel added during this trip (liters)
/// * `baseline_consumption_kwh` - Vehicle's electric consumption rate (kWh/100km)
/// * `tp_consumption` - Vehicle's fuel consumption rate (l/100km)
/// * `battery_capacity` - Max battery capacity (kWh)
/// * `tank_size` - Max tank size (liters)
pub fn calculate_phev_trip_consumption(
    distance_km: f64,
    previous_battery_kwh: f64,
    previous_fuel_liters: f64,
    energy_charged: Option<f64>,
    fuel_added: Option<f64>,
    baseline_consumption_kwh: f64, // kWh/100km
    tp_consumption: f64,           // l/100km
    battery_capacity: f64,
    tank_size: f64,
) -> PhevTripConsumption {
    // Add any charged energy first (before driving)
    let battery_after_charge =
        (previous_battery_kwh + energy_charged.unwrap_or(0.0)).min(battery_capacity);

    // Calculate total energy needed for entire trip
    let energy_needed =
        calculations_energy::calculate_energy_used(distance_km, baseline_consumption_kwh);

    // Use electricity first (limited by available battery)
    let energy_from_battery = energy_needed.min(battery_after_charge);
    let km_on_electricity = if baseline_consumption_kwh > 0.0 {
        energy_from_battery / baseline_consumption_kwh * 100.0
    } else {
        0.0
    };

    // Remaining distance uses fuel
    let km_on_fuel = (distance_km - km_on_electricity).max(0.0);
    let fuel_used = calculations::calculate_fuel_used(km_on_fuel, tp_consumption);

    // Update both tanks
    let battery_remaining = (battery_after_charge - energy_from_battery).max(0.0);
    let fuel_after_fillup = (previous_fuel_liters + fuel_added.unwrap_or(0.0)).min(tank_size);
    let fuel_remaining = (fuel_after_fillup - fuel_used).max(0.0);

    PhevTripConsumption {
        km_on_electricity,
        km_on_fuel,
        energy_used_kwh: energy_from_battery,
        fuel_used_liters: fuel_used,
        battery_remaining_kwh: battery_remaining,
        fuel_remaining_liters: fuel_remaining,
    }
}

// Tests are included in mod.rs as phev_tests
