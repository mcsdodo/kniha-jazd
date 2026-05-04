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

use crate::app_state::AppState;
use crate::db::Database;

#[test]
fn assign_paperless_doc_blocked_when_read_only() {
    let db = Database::in_memory().unwrap();
    let v = crate::db_tests::create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    let trip = crate::db_tests::seed_test_trip(&db, &v.id.to_string());

    let app_state = AppState::new();
    app_state.enable_read_only("test");

    let err = assign_paperless_doc_to_trip_internal(&app_state, &db, 435, &trip)
        .unwrap_err();
    // check_read_only! returns Slovak text: "Aplikácia je v režime len na čítanie"
    assert!(err.to_lowercase().contains("read") || err.to_lowercase().contains("čítanie"));
}

#[test]
fn assign_paperless_doc_persists_link() {
    let db = Database::in_memory().unwrap();
    let v = crate::db_tests::create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    let trip = crate::db_tests::seed_test_trip(&db, &v.id.to_string());

    let app_state = AppState::new();
    assign_paperless_doc_to_trip_internal(&app_state, &db, 435, &trip).unwrap();
    assert_eq!(db.get_paperless_link_for_doc(435).unwrap(), Some(trip));
}

#[test]
fn unassign_paperless_doc_removes_link() {
    let db = Database::in_memory().unwrap();
    let v = crate::db_tests::create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    let trip = crate::db_tests::seed_test_trip(&db, &v.id.to_string());

    let app_state = AppState::new();
    assign_paperless_doc_to_trip_internal(&app_state, &db, 435, &trip).unwrap();
    unassign_paperless_doc_internal(&app_state, &db, 435).unwrap();
    assert_eq!(db.get_paperless_link_for_doc(435).unwrap(), None);
}
