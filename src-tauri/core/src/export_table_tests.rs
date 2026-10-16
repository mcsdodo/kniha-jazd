//! Tests for the shared export table model.

use super::*;
use crate::export::fixtures::{distinct_labels, ice_export_data, phev_export_data};
use crate::models::{Vehicle, VehicleType};
use chrono::NaiveDate;

fn labels_of(table: &ExportTable) -> Vec<&str> {
    table.columns.iter().map(|c| c.label.as_str()).collect()
}

fn with_type(vehicle_type: VehicleType) -> ExportData {
    let mut data = phev_export_data();
    data.vehicle = match vehicle_type {
        VehicleType::Ice => Vehicle::new_ice("I".into(), "X".into(), 50.0, 6.5, 0.0),
        VehicleType::Bev => Vehicle::new_bev("B".into(), "X".into(), 60.0, 18.0, 0.0, None),
        VehicleType::Phev => phev_export_data().vehicle,
    };
    data
}

#[test]
fn table_columns_ice_all_visible() {
    let mut data = with_type(VehicleType::Ice);
    data.hidden_columns = vec![];
    assert_eq!(
        labels_of(&build_export_table(&data)),
        vec![
            "col_trip_number",
            "col_start_datetime",
            "col_end_datetime",
            "col_driver",
            "col_origin",
            "col_destination",
            "col_purpose",
            "col_km",
            "col_odo_start",
            "col_odo",
            "col_fuel_liters",
            "col_fuel_cost",
            "col_fuel_consumed",
            "col_remaining",
            "col_consumption",
            "col_other_costs",
            "col_note",
        ]
    );
}

#[test]
fn table_columns_honour_hidden_columns() {
    let mut data = with_type(VehicleType::Phev);
    data.hidden_columns = ["time", "fuelConsumed", "fuelRemaining", "otherCosts", "otherCostsNote"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let table = build_export_table(&data);
    let labels = labels_of(&table);
    for hidden in [
        "col_end_datetime",
        "col_fuel_consumed",
        "col_remaining",
        "col_battery_remaining",
        "col_other_costs",
        "col_note",
    ] {
        assert!(!labels.contains(&hidden), "{hidden} must be hidden");
    }
    assert!(labels.contains(&"col_fuel_liters"));
    assert!(labels.contains(&"col_energy_rate"));
}

#[test]
fn table_columns_bev() {
    let mut data = with_type(VehicleType::Bev);
    data.hidden_columns = vec![];
    let table = build_export_table(&data);
    let labels = labels_of(&table);
    assert!(!labels.contains(&"col_fuel_liters"));
    let odo = labels.iter().position(|l| *l == "col_odo").unwrap();
    assert_eq!(
        &labels[odo + 1..odo + 5],
        &[
            "col_energy_kwh",
            "col_energy_cost",
            "col_battery_remaining",
            "col_energy_rate"
        ]
    );
}

#[test]
fn table_rows_kinds_and_order() {
    // desc: trip 3, the month-end row (sort key 2.5), trip 2, trip 1, then the
    // first record (number 0).
    let table = build_export_table(&ice_export_data());
    let kinds: Vec<RowKind> = table.rows.iter().map(|r| r.kind).collect();
    assert_eq!(
        kinds,
        vec![
            RowKind::Trip,
            RowKind::MonthEnd,
            RowKind::Trip,
            RowKind::Trip,
            RowKind::FirstRecord
        ]
    );
    let numbers: Vec<&CellValue> = table.rows.iter().map(|r| &r.cells[0]).collect();
    assert_eq!(
        numbers,
        vec![
            &CellValue::Number { value: 3.0, decimals: 0 },
            &CellValue::Empty,
            &CellValue::Number { value: 2.0, decimals: 0 },
            &CellValue::Number { value: 1.0, decimals: 0 },
            &CellValue::Empty,
        ]
    );
}

#[test]
fn table_cells_typed() {
    let mut data = ice_export_data();
    data.hidden_columns = vec![];
    let table = build_export_table(&data);
    let col = |label: &str| {
        table
            .columns
            .iter()
            .position(|c| c.label == label)
            .unwrap_or_else(|| panic!("no column {label}"))
    };
    // Row 2 is trip 2: refuel with 4 decimals, no end time.
    let trip2 = &table.rows[2].cells;
    assert_eq!(
        trip2[col("col_fuel_liters")],
        CellValue::Number { value: 12.3456, decimals: 2 }
    );
    assert_eq!(
        trip2[col("col_start_datetime")],
        CellValue::DateTime(
            NaiveDate::from_ymd_opt(2026, 1, 20)
                .unwrap()
                .and_hms_opt(7, 15, 0)
                .unwrap()
        )
    );
    assert_eq!(trip2[col("col_end_datetime")], CellValue::Text("20.01. -".into()));
    assert_eq!(
        trip2[col("col_remaining")],
        CellValue::Number { value: 55.5956, decimals: 1 }
    );
    // Row 3 is trip 1: no fuel -> empty, other costs -> number.
    let trip1 = &table.rows[3].cells;
    assert_eq!(trip1[col("col_fuel_liters")], CellValue::Empty);
    assert_eq!(
        trip1[col("col_other_costs")],
        CellValue::Number { value: 3.5, decimals: 2 }
    );
    assert_eq!(trip1[col("col_note")], CellValue::Text("parkovné & <diaľnica>".into()));
    // Row 1 is the month-end row.
    let month_end = &table.rows[1].cells;
    assert_eq!(month_end[col("col_start_datetime")], CellValue::Text("31.01. -".into()));
    assert_eq!(
        month_end[col("col_odo_start")],
        CellValue::Number { value: 10115.0, decimals: 0 }
    );
    assert_eq!(month_end[col("col_km")], CellValue::Empty);
}

#[test]
fn cell_display_is_the_printout_text() {
    assert_eq!(CellValue::Number { value: 12.3456, decimals: 2 }.display(), "12.35");
    assert_eq!(CellValue::Number { value: 10115.0, decimals: 0 }.display(), "10115");
    assert_eq!(
        CellValue::DateTime(
            NaiveDate::from_ymd_opt(2026, 1, 5)
                .unwrap()
                .and_hms_opt(8, 0, 0)
                .unwrap()
        )
        .display(),
        "05.01. 08:00"
    );
    assert_eq!(CellValue::Empty.display(), "");
}

#[test]
fn every_row_has_one_cell_per_column() {
    let hideable = ["time", "fuelConsumed", "fuelRemaining", "otherCosts", "otherCostsNote"];
    for vehicle_type in [VehicleType::Ice, VehicleType::Bev, VehicleType::Phev] {
        for hidden in std::iter::once(None).chain(hideable.iter().map(Some)) {
            let mut data = with_type(vehicle_type);
            data.labels = distinct_labels();
            data.hidden_columns = hidden.map(|h| vec![h.to_string()]).unwrap_or_default();
            data.grid_data.month_end_rows = ice_export_data().grid_data.month_end_rows;
            let table = build_export_table(&data);
            for row in &table.rows {
                assert_eq!(
                    row.cells.len(),
                    table.columns.len(),
                    "{vehicle_type:?} hidden={hidden:?} kind={:?}",
                    row.kind
                );
            }
        }
    }
}

/// The printout rounds the way a spreadsheet shows a number format: first to
/// 15 significant digits, then half away from zero. So a cell of the xlsx
/// file shows the same text as the printout, also at a tie.
#[test]
fn display_rounds_like_a_spreadsheet() {
    let shown = |value: f64, decimals: usize| CellValue::Number { value, decimals }.display();
    assert_eq!(shown(40.25, 1), "40.3");
    assert_eq!(shown(12.5, 0), "13");
    assert_eq!(shown(2.5, 0), "3");
    assert_eq!(shown(0.125, 2), "0.13");
    assert_eq!(shown(1.005, 2), "1.01");
    assert_eq!(shown(2.675, 2), "2.68");
    assert_eq!(shown(9.825, 2), "9.83");
    assert_eq!(shown(150.0 * 6.55 / 100.0, 2), "9.83");
    assert_eq!(shown(-2.5, 0), "-3");
    assert_eq!(shown(-0.0001, 2), "0.00");
    assert_eq!(shown(12.3456, 2), "12.35");
    assert_eq!(shown(10115.0, 0), "10115");
}
