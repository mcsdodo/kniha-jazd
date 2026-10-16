//! Tests for the xlsx export. Every test reads the written file back with
//! calamine, so they check the file a person opens, not the writer calls.

use super::*;
use crate::export::fixtures::{ice_export_data, phev_export_data};
use crate::export::RouteMapPage;
use crate::export_table::{build_export_table, CellValue};
use base64::Engine;
use calamine::{open_workbook_from_rs, Data, Range, Reader, Xlsx};
use std::io::Cursor;

type Book = Xlsx<Cursor<Vec<u8>>>;

fn open(bytes: Vec<u8>) -> Book {
    open_workbook_from_rs(Cursor::new(bytes)).expect("the file must be a valid xlsx")
}

fn sheet(book: &mut Book, name: &str) -> Range<Data> {
    book.worksheet_range(name).expect("sheet must exist")
}

/// A real PNG, because the writer parses the image header.
fn tiny_png_base64() -> String {
    let png = tiny_skia::Pixmap::new(4, 3).unwrap().encode_png().unwrap();
    base64::engine::general_purpose::STANDARD.encode(png)
}

fn ice_without_maps() -> ExportData {
    let mut data = ice_export_data();
    data.route_maps = vec![];
    data
}

fn ice_with_one_map() -> ExportData {
    let mut data = ice_export_data();
    data.route_maps = vec![RouteMapPage {
        attachment_no: 1,
        row_number: 3,
        png_base64: tiny_png_base64(),
    }];
    data
}

fn text_at(range: &Range<Data>, row: u32, col: u32) -> String {
    match range.get_value((row, col)) {
        Some(Data::String(s)) => s.clone(),
        other => panic!("expected text at ({row},{col}), got {other:?}"),
    }
}

/// Row index of the first row whose column A holds `text`.
fn row_with(range: &Range<Data>, text: &str) -> u32 {
    (0..range.height() as u32)
        .find(|&r| matches!(range.get_value((r, 0)), Some(Data::String(s)) if s == text))
        .unwrap_or_else(|| panic!("no row starts with {text:?}"))
}

#[test]
fn xlsx_sheet_names() {
    let data = ice_without_maps();
    let book = open(generate_xlsx(&data).unwrap());
    assert_eq!(book.sheet_names(), vec![data.labels.sheet_logbook.clone()]);

    let data = ice_with_one_map();
    let book = open(generate_xlsx(&data).unwrap());
    assert_eq!(
        book.sheet_names(),
        vec![
            data.labels.sheet_logbook.clone(),
            data.labels.sheet_attachments.clone()
        ]
    );
}

#[test]
fn sheet_labels_default_when_the_caller_sends_none() {
    let mut value = serde_json::to_value(crate::export::sample_export_labels()).unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("sheet_logbook");
    object.remove("sheet_attachments");
    let labels: crate::export::ExportLabels = serde_json::from_value(value).unwrap();
    assert_eq!(labels.sheet_logbook, "Kniha jázd");
    assert_eq!(labels.sheet_attachments, "Prílohy");
}

#[test]
fn sheet_name_drops_forbidden_characters_and_length() {
    assert_eq!(sheet_name("Kniha jázd"), "Kniha jázd");
    assert_eq!(sheet_name("a/b\\c[d]e:f*g?h"), "a-b-c-d-e-f-g-h");
    assert_eq!(sheet_name(&"x".repeat(40)).chars().count(), 31);
}

#[test]
fn xlsx_header_block() {
    let data = ice_without_maps();
    let mut book = open(generate_xlsx(&data).unwrap());
    let range = sheet(&mut book, &data.labels.sheet_logbook);

    assert_eq!(text_at(&range, 0, 0), data.labels.page_title);
    assert_eq!(text_at(&range, 2, 0), "header_company");
    assert_eq!(text_at(&range, 2, 1), "Firma & Syn s.r.o.");
    let plate = row_with(&range, "header_license_plate");
    assert_eq!(text_at(&range, plate, 1), "BA 123/AB");
    let tp = row_with(&range, "header_tp_consumption");
    assert_eq!(range.get_value((tp, 1)), Some(&Data::Float(6.5)));
    let driver = row_with(&range, "header_driver");
    assert_eq!(text_at(&range, driver, 1), "Ján Novák");
}

#[test]
fn xlsx_table_matches_model() {
    let data = ice_without_maps();
    let table = build_export_table(&data);
    let mut book = open(generate_xlsx(&data).unwrap());
    let range = sheet(&mut book, &data.labels.sheet_logbook);

    let header = row_with(&range, "col_trip_number");
    for (c, column) in table.columns.iter().enumerate() {
        assert_eq!(text_at(&range, header, c as u32), column.label);
    }
    for (r, row) in table.rows.iter().enumerate() {
        let excel_row = header + 1 + r as u32;
        for (c, cell) in row.cells.iter().enumerate() {
            let got = range.get_value((excel_row, c as u32)).cloned().unwrap_or(Data::Empty);
            match cell {
                CellValue::Empty => assert_eq!(got, Data::Empty, "row {r} col {c}"),
                CellValue::Text(t) => assert_eq!(got, Data::String(t.clone()), "row {r} col {c}"),
                CellValue::Number { value, .. } => {
                    assert_eq!(got, Data::Float(*value), "row {r} col {c}")
                }
                CellValue::DateTime(dt) => match got {
                    Data::DateTime(excel) => {
                        assert_eq!(excel.as_datetime(), Some(*dt), "row {r} col {c}")
                    }
                    other => panic!("row {r} col {c}: expected a date cell, got {other:?}"),
                },
            }
        }
    }
}

#[test]
fn xlsx_keeps_the_exact_value() {
    let data = ice_without_maps();
    let mut book = open(generate_xlsx(&data).unwrap());
    let range = sheet(&mut book, &data.labels.sheet_logbook);
    let found = range
        .used_cells()
        .any(|(_, _, v)| *v == Data::Float(12.3456));
    assert!(found, "the refuel of 12.3456 L must be stored with all decimals");
}

#[test]
fn xlsx_none_values_are_empty_cells() {
    let data = ice_without_maps();
    let table = build_export_table(&data);
    let liters = table
        .columns
        .iter()
        .position(|c| c.label == "col_fuel_liters")
        .unwrap() as u32;
    let mut book = open(generate_xlsx(&data).unwrap());
    let range = sheet(&mut book, &data.labels.sheet_logbook);
    // Table row 3 is trip 1, which has no fuel.
    let row = row_with(&range, "col_trip_number") + 1 + 3;
    assert_eq!(range.get_value((row, 0)), Some(&Data::Float(1.0)));
    assert!(matches!(range.get_value((row, liters)), None | Some(Data::Empty)));
}

#[test]
fn xlsx_text_is_not_html_escaped() {
    let data = ice_without_maps();
    let mut book = open(generate_xlsx(&data).unwrap());
    let range = sheet(&mut book, &data.labels.sheet_logbook);
    let found = range
        .used_cells()
        .any(|(_, _, v)| *v == Data::String("parkovné & <diaľnica>".into()));
    assert!(found);
}

fn footer_rows(data: &ExportData) -> Vec<(String, f64)> {
    let table = build_export_table(data);
    let mut book = open(generate_xlsx(data).unwrap());
    let range = sheet(&mut book, &data.labels.sheet_logbook);
    let first = row_with(&range, "col_trip_number") + 1 + table.rows.len() as u32 + 1;
    (first..range.height() as u32)
        .map(|r| {
            let value = match range.get_value((r, 1)) {
                Some(Data::Float(f)) => *f,
                other => panic!("footer row {r}: expected a number, got {other:?}"),
            };
            (text_at(&range, r, 0), value)
        })
        .collect()
}

#[test]
fn xlsx_ice_footer_rows() {
    let data = ice_without_maps();
    let t = data.totals.clone();
    assert_eq!(
        footer_rows(&data),
        vec![
            ("footer_total_km".to_string(), t.total_km),
            ("footer_total_fuel".to_string(), t.total_fuel_liters),
            ("footer_total_fuel".to_string(), t.total_fuel_cost),
            ("footer_other_costs".to_string(), t.total_other_costs),
            ("footer_avg_consumption".to_string(), t.avg_consumption),
            ("footer_deviation".to_string(), t.deviation_percent),
            ("footer_tp_norm".to_string(), 6.5),
        ]
    );
}

#[test]
fn xlsx_phev_footer_rows() {
    let data = phev_export_data();
    let t = data.totals.clone();
    assert_eq!(
        footer_rows(&data),
        vec![
            ("footer_total_km".to_string(), t.total_km),
            ("footer_total_fuel".to_string(), t.total_fuel_liters),
            ("footer_total_fuel".to_string(), t.total_fuel_cost),
            ("footer_total_energy".to_string(), t.total_energy_kwh),
            ("footer_total_energy".to_string(), t.total_energy_cost),
            ("footer_other_costs".to_string(), t.total_other_costs),
            ("footer_avg_consumption".to_string(), t.avg_consumption),
            ("footer_avg_energy_rate".to_string(), t.avg_energy_rate),
            ("footer_deviation".to_string(), t.deviation_percent),
        ]
    );
}

#[test]
fn xlsx_bev_footer_rows() {
    let mut data = phev_export_data();
    data.vehicle = crate::models::Vehicle::new_bev("B".into(), "X".into(), 60.0, 18.0, 0.0, None);
    let t = data.totals.clone();
    assert_eq!(
        footer_rows(&data),
        vec![
            ("footer_total_km".to_string(), t.total_km),
            ("footer_total_energy".to_string(), t.total_energy_kwh),
            ("footer_total_energy".to_string(), t.total_energy_cost),
            ("footer_other_costs".to_string(), t.total_other_costs),
            ("footer_avg_energy_rate".to_string(), t.avg_energy_rate),
            ("footer_deviation".to_string(), t.energy_deviation_percent),
            ("footer_baseline_norm".to_string(), 18.0),
        ]
    );
}

#[test]
fn xlsx_attachments_sheet() {
    let mut data = ice_with_one_map();
    data.labels.attachment_heading = "Príloha č.".into();
    data.labels.record_reference = "záznam č.".into();
    let mut book = open(generate_xlsx(&data).unwrap());
    let range = sheet(&mut book, &data.labels.sheet_attachments);

    assert_eq!(text_at(&range, 0, 0), "Príloha č. 1 — záznam č. 3");
    let attribution = range
        .used_cells()
        .any(|(_, _, v)| *v == Data::String("© OpenStreetMap contributors".into()));
    assert!(attribution, "the OSM attribution must be on the sheet");
    assert_eq!(book.pictures().map(|p| p.len()), Some(1));
}

#[test]
fn xlsx_number_formats() {
    assert_eq!(num_format(0), "0");
    assert_eq!(num_format(1), "0.0");
    assert_eq!(num_format(2), "0.00");
}
