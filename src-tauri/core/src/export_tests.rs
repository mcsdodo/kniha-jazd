//! Tests for HTML export functionality

use super::*;
use chrono::{NaiveDate, Utc};
use std::collections::{HashMap, HashSet};
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
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Labels fixture. Only the strings the tests assert on carry real text -
/// the rest are placeholders so the fixture stays readable.
fn sample_labels() -> ExportLabels {
    ExportLabels {
        lang: "sk".to_string(),
        page_title: "KNIHA JÁZD".to_string(),
        header_company: String::new(),
        header_ico: String::new(),
        header_vehicle: String::new(),
        header_license_plate: String::new(),
        header_tank_size: String::new(),
        header_tp_consumption: String::new(),
        header_year: String::new(),
        header_battery_capacity: String::new(),
        header_baseline_consumption: String::new(),
        header_vin: String::new(),
        header_driver: String::new(),
        col_trip_number: String::new(),
        col_start_datetime: String::new(),
        col_end_datetime: String::new(),
        col_driver: String::new(),
        col_odo_start: String::new(),
        col_time: String::new(),
        col_origin: String::new(),
        col_destination: String::new(),
        col_purpose: String::new(),
        col_km: String::new(),
        col_odo: String::new(),
        col_fuel_liters: String::new(),
        col_fuel_cost: String::new(),
        col_fuel_consumed: String::new(),
        col_other_costs: String::new(),
        col_note: String::new(),
        col_remaining: String::new(),
        col_consumption: String::new(),
        col_energy_kwh: String::new(),
        col_energy_cost: String::new(),
        col_battery_remaining: String::new(),
        col_energy_rate: String::new(),
        footer_total_km: String::new(),
        footer_total_fuel: String::new(),
        footer_other_costs: String::new(),
        footer_avg_consumption: String::new(),
        footer_deviation: String::new(),
        footer_tp_norm: String::new(),
        footer_total_energy: String::new(),
        footer_avg_energy_rate: String::new(),
        footer_baseline_norm: String::new(),
        print_hint: String::new(),
        attachment_heading: "Príloha č.".to_string(),
        record_reference: "záznam č.".to_string(),
    }
}

fn empty_grid_data(trips: Vec<Trip>) -> TripGridData {
    TripGridData {
        trips,
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
        missing_fuel_invoices: HashSet::new(),
        missing_other_invoices: HashSet::new(),
        other_sum_mismatches: HashSet::new(),
        other_invoice_sums: HashMap::new(),
        fuel_datetime_warnings: HashSet::new(),
        other_datetime_warnings: HashSet::new(),
        fuel_mismatch_overrides: HashSet::new(),
        other_mismatch_overrides: HashSet::new(),
        year_start_odometer: 10000.0,
        year_start_fuel: 50.0,
        suggested_fillup: HashMap::new(),
        legend_suggested_fillup: None,
        trip_numbers: HashMap::new(),
        odometer_start: HashMap::new(),
        month_end_rows: vec![],
    }
}

/// Minimal but valid ExportData for an ICE vehicle with a single trip.
fn sample_export_data() -> ExportData {
    let trips = vec![make_trip(100.0, Some(6.0), Some(10.0), None)];
    let totals = ExportTotals::calculate(&trips, 5.0, 0.0);

    ExportData {
        vehicle: Vehicle::new_ice(
            "Test Car".to_string(),
            "BA123AB".to_string(),
            50.0,
            5.0,
            10000.0,
        ),
        settings: Settings::default(),
        grid_data: empty_grid_data(trips),
        year: 2026,
        totals,
        labels: sample_labels(),
        hidden_columns: vec![],
        sort_direction: "asc".to_string(),
        route_maps: vec![],
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

#[test]
fn export_appends_one_page_per_route_map() {
    let mut data = sample_export_data();
    data.route_maps = vec![
        RouteMapPage {
            attachment_no: 1,
            row_number: 3,
            png_base64: "AAAA".into(),
        },
        RouteMapPage {
            attachment_no: 2,
            row_number: 7,
            png_base64: "BBBB".into(),
        },
    ];

    let html = generate_html(data).unwrap();

    assert_eq!(html.matches("class=\"map-page\"").count(), 2);
    assert!(html.contains("data:image/png;base64,AAAA"));
    assert!(html.contains("data:image/png;base64,BBBB"));
    assert!(html.contains("Príloha č. 1"));
    assert!(html.contains("záznam č. 3"));
    assert!(html.contains("Príloha č. 2"));
    assert!(html.contains("záznam č. 7"));
}

#[test]
fn export_without_route_maps_is_unchanged() {
    let html = generate_html(sample_export_data()).unwrap();
    assert!(
        !html.contains("map-page"),
        "no maps must mean no extra markup"
    );
}

#[test]
fn map_pages_carry_osm_attribution() {
    // Licence requirement: the renderer bakes no text into the PNG, so the
    // caption is the only place attribution can live.
    let mut data = sample_export_data();
    data.route_maps = vec![RouteMapPage {
        attachment_no: 1,
        row_number: 1,
        png_base64: "A".into(),
    }];

    assert!(generate_html(data).unwrap().contains("OpenStreetMap"));
}

#[test]
fn the_trip_table_gains_no_column() {
    // Guards the "attachment -> row, one way" decision: the table's header
    // cell count must be identical with and without route maps.
    let without = generate_html(sample_export_data()).unwrap();

    let mut data = sample_export_data();
    data.route_maps = vec![RouteMapPage {
        attachment_no: 1,
        row_number: 1,
        png_base64: "A".into(),
    }];
    let with = generate_html(data).unwrap();

    let headers_without = without.matches("<th").count();
    assert!(headers_without > 0, "sanity: the table must have headers");
    assert_eq!(headers_without, with.matches("<th").count());
}
