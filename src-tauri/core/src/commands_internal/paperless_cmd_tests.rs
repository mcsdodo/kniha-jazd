//! Tests for paperless commands.
use super::*;

#[test]
fn get_paperless_invoices_maps_fuel_only_to_fuel() {
    let row = test_helpers::make_doc(&[51]);
    let assigned = map_assignment(&row.tag_ids, 51, 59);
    assert_eq!(assigned, crate::models::AssignmentType::Fuel);
}

#[test]
fn get_paperless_invoices_maps_car_only_to_other() {
    let row = test_helpers::make_doc(&[59]);
    let assigned = map_assignment(&row.tag_ids, 51, 59);
    assert_eq!(assigned, crate::models::AssignmentType::Other);
}

#[test]
fn get_paperless_invoices_both_tags_priority_fuel() {
    let assigned = map_assignment(&[51, 59], 51, 59);
    assert_eq!(assigned, crate::models::AssignmentType::Fuel);
}

#[test]
fn map_assignment_logs_warning_and_returns_other_when_neither_tag_present() {
    let assigned = map_assignment(&[1234], 51, 59);
    assert_eq!(assigned, crate::models::AssignmentType::Other);
}

#[test]
fn year_filter_uses_receipt_datetime_when_present() {
    let dt = chrono::NaiveDateTime::parse_from_str("2026-04-27T13:24:14", "%Y-%m-%dT%H:%M:%S").unwrap();
    let created = chrono::NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
    assert_eq!(doc_year(&Some(dt), &created), 2026);
}

#[test]
fn year_filter_falls_back_to_created_when_no_datetime() {
    let created = chrono::NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();
    assert_eq!(doc_year(&None, &created), 2025);
}

fn row(assignment_type: AssignmentType, trip_id: Option<&str>) -> PaperlessInvoiceRow {
    PaperlessInvoiceRow {
        paperless_document_id: 0,
        title: "t".into(),
        paperless_url: "http://localhost/documents/0/".into(),
        total_price_eur: None,
        liters: None,
        receipt_datetime: None,
        created_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        assignment_type,
        trip_id: trip_id.map(|s| s.to_string()),
    }
}

#[test]
fn count_unlinked_fuel_counts_only_unlinked_fuel_rows() {
    let rows = vec![
        row(AssignmentType::Fuel, None),
        row(AssignmentType::Fuel, Some("trip-1")),
        row(AssignmentType::Other, None),
    ];
    assert_eq!(count_unlinked_fuel(&rows), 1);
}
