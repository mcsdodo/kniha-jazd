//! Tests for energy calculations module (BEV/PHEV electricity tracking)

use super::energy::*;

// ============================================================================
// calculate_consumption_rate_kwh tests
// ============================================================================

#[test]
fn test_consumption_rate_kwh_normal() {
    // 45 kWh / 250 km = 18.0 kWh/100km
    let rate = calculate_consumption_rate_kwh(45.0, 250.0);
    assert!((rate - 18.0).abs() < 0.01);
}

#[test]
fn test_consumption_rate_kwh_zero_km() {
    // Edge case: no distance driven
    let rate = calculate_consumption_rate_kwh(45.0, 0.0);
    assert_eq!(rate, 0.0);
}

#[test]
fn test_consumption_rate_kwh_negative_km() {
    // Edge case: negative distance (invalid input)
    let rate = calculate_consumption_rate_kwh(45.0, -10.0);
    assert_eq!(rate, 0.0);
}

// ============================================================================
// calculate_energy_used tests
// ============================================================================

#[test]
fn test_energy_used_normal() {
    // 100 km at 18 kWh/100km = 18 kWh
    let used = calculate_energy_used(100.0, 18.0);
    assert!((used - 18.0).abs() < 0.01);
}

#[test]
fn test_energy_used_short_trip() {
    // 30 km at 20 kWh/100km = 6 kWh
    let used = calculate_energy_used(30.0, 20.0);
    assert!((used - 6.0).abs() < 0.01);
}

// ============================================================================
// calculate_battery_remaining tests
// ============================================================================

#[test]
fn test_battery_remaining_normal() {
    // Start 60 kWh, use 15 kWh, no charge = 45 kWh
    let remaining = calculate_battery_remaining(60.0, 15.0, None, 75.0);
    assert!((remaining - 45.0).abs() < 0.01);
}

#[test]
fn test_battery_remaining_with_charge() {
    // Start 20 kWh, use 10 kWh, charge 50 kWh = 60 kWh
    let remaining = calculate_battery_remaining(20.0, 10.0, Some(50.0), 75.0);
    assert!((remaining - 60.0).abs() < 0.01);
}

#[test]
fn test_battery_remaining_caps_at_capacity() {
    // Start 60 kWh, use 5 kWh, charge 30 kWh = would be 85, capped at 75
    let remaining = calculate_battery_remaining(60.0, 5.0, Some(30.0), 75.0);
    assert!((remaining - 75.0).abs() < 0.01);
}

#[test]
fn test_battery_remaining_floors_at_zero() {
    // Start 10 kWh, use 20 kWh = would be -10, capped at 0
    let remaining = calculate_battery_remaining(10.0, 20.0, None, 75.0);
    assert_eq!(remaining, 0.0);
}

// ============================================================================
// kwh_to_percent tests
// ============================================================================

#[test]
fn test_kwh_to_percent_normal() {
    // 45 kWh of 75 kWh capacity = 60%
    let percent = kwh_to_percent(45.0, 75.0);
    assert!((percent - 60.0).abs() < 0.01);
}

#[test]
fn test_kwh_to_percent_full() {
    // 75 kWh of 75 kWh = 100%
    let percent = kwh_to_percent(75.0, 75.0);
    assert!((percent - 100.0).abs() < 0.01);
}

#[test]
fn test_kwh_to_percent_zero_capacity() {
    // Edge case: zero capacity returns 0%
    let percent = kwh_to_percent(45.0, 0.0);
    assert_eq!(percent, 0.0);
}

// ============================================================================
// percent_to_kwh tests
// ============================================================================

#[test]
fn test_percent_to_kwh_normal() {
    // 60% of 75 kWh = 45 kWh
    let kwh = percent_to_kwh(60.0, 75.0);
    assert!((kwh - 45.0).abs() < 0.01);
}

#[test]
fn test_percent_to_kwh_full() {
    // 100% of 75 kWh = 75 kWh
    let kwh = percent_to_kwh(100.0, 75.0);
    assert!((kwh - 75.0).abs() < 0.01);
}

#[test]
fn test_percent_to_kwh_zero_capacity() {
    // Edge case: zero capacity returns 0
    let kwh = percent_to_kwh(60.0, 0.0);
    assert_eq!(kwh, 0.0);
}
