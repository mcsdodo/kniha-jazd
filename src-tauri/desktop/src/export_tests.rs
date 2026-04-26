//! Tests for HTML export functionality

use super::*;
use chrono::{NaiveDate, Utc};
use uuid::Uuid;

fn make_trip(km: f64, fuel: Option<f64>, fuel_cost: Option<f64>, other_cost: Option<f64>) -> Trip {
    let start_datetime = NaiveDate::from_ymd_opt(2025, 1, 1)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        start_datetime,
        end_datetime: None,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: km,
        odometer: 10000.0,
        purpose: "test".to_string(),
        fuel_liters: fuel,
        fuel_cost_eur: fuel_cost,
        full_tank: true,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: other_cost,
        other_costs_note: None,
        sort_order: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn test_export_totals_basic() {
    let trips = vec![
        make_trip(100.0, Some(6.0), Some(10.0), None),
        make_trip(200.0, Some(12.0), Some(20.0), Some(5.0)),
    ];

    let totals = ExportTotals::calculate(&trips, 5.0, 0.0);

    assert_eq!(totals.total_km, 300.0);
    assert_eq!(totals.total_fuel_liters, 18.0);
    assert_eq!(totals.total_fuel_cost, 30.0);
    assert_eq!(totals.total_other_costs, 5.0);
    // avg = 18/300*100 = 6.0 l/100km
    assert!((totals.avg_consumption - 6.0).abs() < 0.001);
    // deviation = 6.0/5.0*100 = 120%
    assert!((totals.deviation_percent - 120.0).abs() < 0.001);
}

#[test]
fn test_export_totals_no_trips() {
    let trips: Vec<Trip> = vec![];
    let totals = ExportTotals::calculate(&trips, 5.0, 0.0);

    assert_eq!(totals.total_km, 0.0);
    assert_eq!(totals.total_fuel_liters, 0.0);
    assert_eq!(totals.avg_consumption, 0.0);
    assert_eq!(totals.deviation_percent, 100.0); // No fuel = 100% (at TP)
}

#[test]
fn test_export_totals_no_fuel() {
    let trips = vec![
        make_trip(100.0, None, None, None),
        make_trip(200.0, None, None, None),
    ];

    let totals = ExportTotals::calculate(&trips, 5.0, 0.0);

    assert_eq!(totals.total_km, 300.0);
    assert_eq!(totals.total_fuel_liters, 0.0);
    assert_eq!(totals.avg_consumption, 0.0);
    assert_eq!(totals.deviation_percent, 100.0);
}

#[test]
fn test_export_totals_zero_tp() {
    let trips = vec![make_trip(100.0, Some(6.0), Some(10.0), None)];

    // Edge case: tp_consumption = 0 should not panic
    let totals = ExportTotals::calculate(&trips, 0.0, 0.0);

    assert_eq!(totals.total_km, 100.0);
    assert_eq!(totals.deviation_percent, 100.0); // Defaults to 100% when tp is 0
}

#[test]
fn test_html_escape() {
    assert_eq!(html_escape("a & b"), "a &amp; b");
    assert_eq!(html_escape("<script>"), "&lt;script&gt;");
    assert_eq!(html_escape("\"test\""), "&quot;test&quot;");
}

#[test]
fn test_export_totals_excludes_dummy_rows() {
    // Dummy row (0 km) should be excluded from totals
    let trips = vec![
        make_trip(0.0, None, None, Some(999.0)), // Dummy row - should be excluded
        make_trip(100.0, Some(6.0), Some(10.0), Some(5.0)),
        make_trip(200.0, Some(12.0), Some(20.0), None),
    ];

    let totals = ExportTotals::calculate(&trips, 5.0, 0.0);

    // Should only count trips with km > 0
    assert_eq!(totals.total_km, 300.0); // 100 + 200, not 0 + 100 + 200
    assert_eq!(totals.total_fuel_liters, 18.0);
    assert_eq!(totals.total_fuel_cost, 30.0);
    assert_eq!(totals.total_other_costs, 5.0); // Only from second trip, dummy's 999 excluded
}
