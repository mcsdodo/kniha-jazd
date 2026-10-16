//! Deterministic `ExportData` fixtures shared by the HTML golden tests, the
//! table model tests and the xlsx tests. Every id and value is fixed, so a
//! rendered document is byte-stable between runs.

use super::*;
use crate::models::MonthEndRow;
use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};
use std::collections::{HashMap, HashSet};

fn at(month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, month, day)
        .unwrap()
        .and_hms_opt(hour, minute, 0)
        .unwrap()
}

fn fixed_uuid(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

/// A trip with only the fields every fixture sets; the rest are empty.
pub(crate) fn trip(
    n: u128,
    start: NaiveDateTime,
    end: Option<NaiveDateTime>,
    route: (&str, &str),
    distance_km: f64,
    odometer: f64,
) -> Trip {
    let stamp = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    Trip {
        id: fixed_uuid(n),
        vehicle_id: fixed_uuid(900),
        start_datetime: start,
        end_datetime: end,
        origin_place_id: Uuid::nil(),
        destination_place_id: Uuid::nil(),
        origin: route.0.to_string(),
        destination: route.1.to_string(),
        distance_km,
        odometer,
        purpose: "služobná cesta".to_string(),
        fuel_liters: None,
        fuel_cost_eur: None,
        full_tank: true,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: stamp,
        updated_at: stamp,
    }
}

pub(crate) fn empty_grid(trips: Vec<Trip>) -> TripGridData {
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
        duplicate_datetime_warnings: HashSet::new(),
        odometer_span_warnings: HashSet::new(),
        odometer_spans: HashMap::new(),
        other_datetime_warnings: HashSet::new(),
        fuel_receipt_datetimes: HashMap::new(),
        other_receipt_datetimes: HashMap::new(),
        fuel_mismatch_overrides: HashSet::new(),
        other_mismatch_overrides: HashSet::new(),
        year_start_odometer: 10000.0,
        year_start_fuel: 50.0,
        suggested_fillup: HashMap::new(),
        legend_suggested_fillup: None,
        trip_numbers: HashMap::new(),
        odometer_start: HashMap::new(),
        month_end_rows: vec![],
        route_map_trip_ids: HashSet::new(),
    }
}

/// Labels where every text field carries its own field name ("col_km" ...),
/// so a test can tell each label apart.
pub(crate) fn distinct_labels() -> ExportLabels {
    let mut value = serde_json::to_value(sample_export_labels()).unwrap();
    for (key, field) in value.as_object_mut().unwrap().iter_mut() {
        if key != "lang" && key != "attachment_heading" && key != "record_reference" {
            *field = serde_json::Value::String(key.clone());
        }
    }
    serde_json::from_value(value).unwrap()
}

fn settings() -> Settings {
    Settings {
        id: fixed_uuid(800),
        company_name: "Firma & Syn s.r.o.".to_string(),
        company_ico: "12345678".to_string(),
        buffer_trip_purpose: "služobná cesta".to_string(),
        updated_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
    }
}

fn first_record() -> Trip {
    let mut t = trip(0, at(1, 1, 0, 0), None, ("-", "-"), 0.0, 10000.0);
    t.id = Uuid::nil();
    t.purpose = "Prvý záznam".to_string();
    t
}

fn insert_first_record_maps(grid: &mut TripGridData) {
    let nil = Uuid::nil().to_string();
    grid.fuel_remaining.insert(nil.clone(), grid.year_start_fuel);
    grid.trip_numbers.insert(nil.clone(), 0);
    grid.odometer_start.insert(nil, grid.year_start_odometer);
}

/// ICE, two months, one month-end row, one trip without an end time, one
/// refuel with more decimals than the printout shows, one hidden column,
/// newest first, one route map.
pub(crate) fn ice_export_data() -> ExportData {
    let t1 = {
        let mut t = trip(
            1,
            at(1, 5, 8, 0),
            Some(at(1, 5, 9, 30)),
            ("Bratislava", "Trnava"),
            60.0,
            10060.0,
        );
        t.other_costs_eur = Some(3.5);
        t.other_costs_note = Some("parkovné & <diaľnica>".to_string());
        t
    };
    let t2 = {
        let mut t = trip(2, at(1, 20, 7, 15), None, ("Trnava", "Nitra"), 55.0, 10115.0);
        t.fuel_liters = Some(12.3456);
        t.fuel_cost_eur = Some(20.987);
        t
    };
    let t3 = trip(
        3,
        at(2, 3, 10, 0),
        Some(at(2, 3, 11, 5)),
        ("Nitra", "Bratislava"),
        92.0,
        10207.0,
    );

    let trips = vec![t1, t2, t3, first_record()];
    let mut grid = empty_grid(trips);
    for (n, number, odo_start, rate, consumed, remaining) in [
        (1u128, 1, 10000.0, 5.8765, 3.52, 46.48),
        (2, 2, 10060.0, 5.8765, 3.23, 55.5956),
        (3, 3, 10115.0, 6.1, 5.61, 49.98),
    ] {
        let id = fixed_uuid(n).to_string();
        grid.trip_numbers.insert(id.clone(), number);
        grid.odometer_start.insert(id.clone(), odo_start);
        grid.rates.insert(id.clone(), rate);
        grid.fuel_consumed.insert(id.clone(), consumed);
        grid.fuel_remaining.insert(id, remaining);
    }
    insert_first_record_maps(&mut grid);
    grid.month_end_rows = vec![MonthEndRow {
        date: NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
        odometer: 10115.0,
        fuel_remaining: 55.5956,
        month: 1,
        sort_key: 2.5,
    }];

    let mut vehicle = Vehicle::new_ice(
        "Škoda Octavia".to_string(),
        "BA 123/AB".to_string(),
        50.0,
        6.5,
        10000.0,
    );
    vehicle.vin = Some("TMBJJ7NE0K0123456".to_string());
    vehicle.driver_name = Some("Ján Novák".to_string());

    let totals = ExportTotals::calculate(&grid.trips, 6.5, 0.0);
    ExportData {
        vehicle,
        settings: settings(),
        grid_data: grid,
        year: 2026,
        totals,
        labels: distinct_labels(),
        hidden_columns: vec!["fuelConsumed".to_string()],
        sort_direction: "desc".to_string(),
        route_maps: vec![RouteMapPage {
            attachment_no: 1,
            row_number: 2,
            png_base64: "iVBORw0KGgo=".to_string(),
        }],
    }
}

/// PHEV with fuel and energy on the same year, no hidden columns, oldest
/// first, no route maps.
pub(crate) fn phev_export_data() -> ExportData {
    let t1 = {
        let mut t = trip(
            1,
            at(3, 2, 6, 45),
            Some(at(3, 2, 8, 0)),
            ("Žilina", "Martin"),
            31.0,
            20031.0,
        );
        t.energy_kwh = Some(9.876);
        t.energy_cost_eur = Some(2.1);
        t
    };
    let t2 = {
        let mut t = trip(
            2,
            at(3, 9, 16, 30),
            Some(at(3, 9, 17, 10)),
            ("Martin", "Žilina"),
            31.0,
            20062.0,
        );
        t.fuel_liters = Some(30.0);
        t.fuel_cost_eur = Some(48.6);
        t.other_costs_eur = Some(1.2);
        t
    };

    let mut first = first_record();
    first.odometer = 20000.0;
    let mut grid = empty_grid(vec![t1, t2, first]);
    grid.year_start_odometer = 20000.0;
    grid.year_start_fuel = 20.0;
    for (n, number, odo_start, rate, remaining, e_rate, battery) in [
        (1u128, 1, 20000.0, 0.0, 20.0, 31.86, 8.1),
        (2, 2, 20031.0, 5.4321, 48.3, 0.0, 8.1),
    ] {
        let id = fixed_uuid(n).to_string();
        grid.trip_numbers.insert(id.clone(), number);
        grid.odometer_start.insert(id.clone(), odo_start);
        grid.rates.insert(id.clone(), rate);
        grid.fuel_remaining.insert(id.clone(), remaining);
        grid.energy_rates.insert(id.clone(), e_rate);
        grid.battery_remaining_kwh.insert(id, battery);
    }
    insert_first_record_maps(&mut grid);

    let vehicle = Vehicle::new_phev(
        "Outlander".to_string(),
        "ZA-456CD".to_string(),
        45.0,
        2.0,
        13.8,
        20.5,
        20000.0,
        Some(60.0),
    );
    let totals = ExportTotals::calculate(&grid.trips, 2.0, 20.5);
    ExportData {
        vehicle,
        settings: settings(),
        grid_data: grid,
        year: 2026,
        totals,
        labels: distinct_labels(),
        hidden_columns: vec![],
        sort_direction: "asc".to_string(),
        route_maps: vec![],
    }
}
