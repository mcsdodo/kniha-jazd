//! Tests for PHEV combined calculations
//!
//! Tests the electricity-first logic where battery is used until depleted,
//! then fuel takes over for remaining distance.

use super::*;

/// Helper to create test scenario with standard vehicle params:
/// - 10 kWh battery, 20 kWh/100km electric consumption
/// - 40 L tank, 6 L/100km fuel consumption
fn test_trip(
    distance_km: f64,
    battery_kwh: f64,
    fuel_liters: f64,
    energy_charged: Option<f64>,
    fuel_added: Option<f64>,
) -> PhevTripConsumption {
    calculate_phev_trip_consumption(
        distance_km,
        battery_kwh,
        fuel_liters,
        energy_charged,
        fuel_added,
        20.0, // 20 kWh/100km
        6.0,  // 6 L/100km
        10.0, // 10 kWh capacity
        40.0, // 40 L tank
    )
}

// ============================================================================
// Core PHEV scenarios
// ============================================================================

#[test]
fn test_phev_all_electric() {
    // Battery has enough for entire trip: 30 km needs 6 kWh, battery has 10
    let result = test_trip(30.0, 10.0, 40.0, None, None);

    // All distance on electricity
    assert!((result.km_on_electricity - 30.0).abs() < 0.01);
    assert!((result.km_on_fuel - 0.0).abs() < 0.01);

    // Energy used: 30 km × 20/100 = 6 kWh
    assert!((result.energy_used_kwh - 6.0).abs() < 0.01);
    assert!((result.fuel_used_liters - 0.0).abs() < 0.01);

    // Final states: 10 - 6 = 4 kWh, fuel unchanged
    assert!((result.battery_remaining_kwh - 4.0).abs() < 0.01);
    assert!((result.fuel_remaining_liters - 40.0).abs() < 0.01);
}

#[test]
fn test_phev_mixed_drive() {
    // Battery runs out mid-trip: 60 km needs 12 kWh, battery has only 10
    let result = test_trip(60.0, 10.0, 40.0, None, None);

    // 10 kWh / 20 × 100 = 50 km on electricity, then 10 km on fuel
    assert!((result.km_on_electricity - 50.0).abs() < 0.01);
    assert!((result.km_on_fuel - 10.0).abs() < 0.01);

    // Energy: all 10 kWh used; Fuel: 10 km × 6/100 = 0.6 L
    assert!((result.energy_used_kwh - 10.0).abs() < 0.01);
    assert!((result.fuel_used_liters - 0.6).abs() < 0.01);

    // Final states: battery depleted, fuel = 40 - 0.6 = 39.4 L
    assert!((result.battery_remaining_kwh - 0.0).abs() < 0.01);
    assert!((result.fuel_remaining_liters - 39.4).abs() < 0.01);
}

#[test]
fn test_phev_all_fuel_depleted_battery() {
    // Battery already empty: entire trip on fuel
    let result = test_trip(50.0, 0.0, 40.0, None, None);

    // No electricity available, all on fuel
    assert!((result.km_on_electricity - 0.0).abs() < 0.01);
    assert!((result.km_on_fuel - 50.0).abs() < 0.01);

    // Fuel: 50 km × 6/100 = 3 L
    assert!((result.energy_used_kwh - 0.0).abs() < 0.01);
    assert!((result.fuel_used_liters - 3.0).abs() < 0.01);

    // Final states: battery still 0, fuel = 40 - 3 = 37 L
    assert!((result.battery_remaining_kwh - 0.0).abs() < 0.01);
    assert!((result.fuel_remaining_liters - 37.0).abs() < 0.01);
}

#[test]
fn test_phev_charge_then_drive() {
    // Charge during trip, then drive
    // Start with 2 kWh, charge 8 kWh → 10 kWh available
    let result = test_trip(60.0, 2.0, 40.0, Some(8.0), None);

    // After charge: 2 + 8 = 10 kWh
    // 60 km × 20/100 = 12 kWh needed
    // 10 kWh available = 50 km electric, 10 km on fuel
    assert!((result.km_on_electricity - 50.0).abs() < 0.01);
    assert!((result.km_on_fuel - 10.0).abs() < 0.01);

    // Final: battery depleted, fuel = 40 - 0.6 = 39.4 L
    assert!((result.battery_remaining_kwh - 0.0).abs() < 0.01);
    assert!((result.fuel_remaining_liters - 39.4).abs() < 0.01);
}

#[test]
fn test_phev_charge_and_refuel() {
    // Both charge and refuel in same trip
    // Start with 5 kWh, 20 L; charge 5 kWh, add 20 L
    let result = test_trip(80.0, 5.0, 20.0, Some(5.0), Some(20.0));

    // After charge/refuel: 10 kWh, 40 L
    // 80 km × 20/100 = 16 kWh needed
    // 10 kWh available = 50 km electric, 30 km on fuel
    assert!((result.km_on_electricity - 50.0).abs() < 0.01);
    assert!((result.km_on_fuel - 30.0).abs() < 0.01);

    // Fuel: 30 km × 6/100 = 1.8 L
    assert!((result.fuel_used_liters - 1.8).abs() < 0.01);

    // Final: battery depleted, fuel = 40 - 1.8 = 38.2 L
    assert!((result.battery_remaining_kwh - 0.0).abs() < 0.01);
    assert!((result.fuel_remaining_liters - 38.2).abs() < 0.01);
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_phev_zero_distance() {
    // No driving, just charge
    let result = test_trip(0.0, 5.0, 40.0, Some(5.0), None);

    assert!((result.km_on_electricity - 0.0).abs() < 0.01);
    assert!((result.km_on_fuel - 0.0).abs() < 0.01);
    assert!((result.energy_used_kwh - 0.0).abs() < 0.01);
    assert!((result.fuel_used_liters - 0.0).abs() < 0.01);

    // Battery increased: 5 + 5 = 10 kWh
    assert!((result.battery_remaining_kwh - 10.0).abs() < 0.01);
    assert!((result.fuel_remaining_liters - 40.0).abs() < 0.01);
}

#[test]
fn test_phev_charge_caps_at_capacity() {
    // Overcharge: 8 kWh + 5 kWh charge, but capacity is 10 kWh
    let result = test_trip(0.0, 8.0, 40.0, Some(5.0), None);

    // Should cap at 10 kWh (capacity), not 13 kWh
    assert!((result.battery_remaining_kwh - 10.0).abs() < 0.01);
}

#[test]
fn test_phev_refuel_caps_at_tank_size() {
    // Overfill: 30 L + 20 L refuel, but tank is 40 L
    let result = test_trip(0.0, 10.0, 30.0, None, Some(20.0));

    // Should cap at 40 L (tank size), not 50 L
    assert!((result.fuel_remaining_liters - 40.0).abs() < 0.01);
}
