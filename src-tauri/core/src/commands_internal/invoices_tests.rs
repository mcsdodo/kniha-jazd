//! Boundary tests for the Paperless invoice commands (Task 64, collapsed 84).
use super::*;
use crate::app_state::AppState;
use crate::db::Database;
use crate::db_tests;
use crate::models::AssignmentType;
use crate::paperless::PaperlessDoc;
use chrono::NaiveDate;

fn paperless_doc_fuel() -> PaperlessDoc {
    PaperlessDoc {
        id: 435,
        title: "Doc 435".into(),
        tag_ids: vec![],
        created: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        total_amount: Some(58.20),
        litres: Some(40.5),
        receipt_datetime: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(12, 0, 0),
    }
}

fn paperless_doc_other() -> PaperlessDoc {
    PaperlessDoc {
        id: 435,
        title: "Iná cena".into(),
        tag_ids: vec![],
        created: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        total_amount: Some(15.00),
        litres: None,
        receipt_datetime: None,
    }
}

#[test]
fn get_trips_for_paperless_assignment_returns_trips() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    db_tests::seed_test_trip(&db, &v.id.to_string());

    let result = get_trips_for_paperless_assignment_internal(
        &db,
        &paperless_doc_fuel(),
        &v.id.to_string(),
        2026,
    )
    .unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn assign_paperless_populates_trip_fuel_when_empty() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    let doc = paperless_doc_fuel();
    assign_paperless_invoice_internal(
        &db,
        &app_state,
        &doc,
        &trip_id,
        &v.id.to_string(),
        AssignmentType::Fuel,
        false,
    )
    .unwrap();
    let trip = db.get_trip(&trip_id).unwrap().unwrap();
    assert_eq!(trip.fuel_liters, Some(40.5));
    assert_eq!(trip.fuel_cost_eur, Some(58.20));
    let link = db.get_paperless_link(435).unwrap().expect("link created");
    assert_eq!(link.trip_id, trip_id);
    assert_eq!(link.assignment_type, AssignmentType::Fuel);
    // Snapshots taken from the backend-fetched doc at assign time
    assert_eq!(link.amount_eur, Some(58.20));
    assert_eq!(link.title, Some("Doc 435".to_string()));
}

#[test]
fn assign_invoice_blocked_when_read_only() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    app_state.enable_read_only("test");
    let doc = paperless_doc_other();
    let err = assign_paperless_invoice_internal(
        &db,
        &app_state,
        &doc,
        &trip_id,
        &v.id.to_string(),
        AssignmentType::Other,
        false,
    )
    .unwrap_err();
    let lower = err.to_lowercase();
    assert!(
        lower.contains("read") || lower.contains("čítanie"),
        "expected read-only error, got: {}",
        err
    );
}

// Fix 2: vehicle ownership check
#[test]
fn assign_paperless_rejects_trip_from_different_vehicle() {
    let db = Database::in_memory().unwrap();
    let v1 = db_tests::create_test_vehicle("Vehicle1");
    db.create_vehicle(&v1).unwrap();
    let v2 = db_tests::create_test_vehicle("Vehicle2");
    db.create_vehicle(&v2).unwrap();
    // Trip belongs to v1, but we pass v2 as the vehicle_id
    let trip_id = db_tests::seed_test_trip(&db, &v1.id.to_string());
    let app_state = AppState::new();
    let doc = paperless_doc_fuel();
    let err = assign_paperless_invoice_internal(
        &db,
        &app_state,
        &doc,
        &trip_id,
        &v2.id.to_string(),
        AssignmentType::Fuel,
        false,
    )
    .unwrap_err();
    assert!(
        err.to_lowercase().contains("vehicle"),
        "expected vehicle mismatch error, got: {}",
        err
    );
}

#[test]
fn assign_paperless_persists_datetime_and_override() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    let dt = chrono::NaiveDateTime::parse_from_str("2026-05-04T13:24:14", "%Y-%m-%dT%H:%M:%S")
        .unwrap();
    let doc = PaperlessDoc {
        id: 435,
        title: "Fuel".into(),
        tag_ids: vec![],
        created: NaiveDate::from_ymd_opt(2026, 5, 4).unwrap(),
        total_amount: Some(63.34),
        litres: Some(40.0),
        receipt_datetime: Some(dt),
    };
    assign_paperless_invoice_internal(
        &db,
        &app_state,
        &doc,
        &trip_id,
        &v.id.to_string(),
        AssignmentType::Fuel,
        true,
    )
    .unwrap();
    let link = db.get_paperless_link(435).unwrap().unwrap();
    assert_eq!(link.receipt_datetime, Some(dt));
    assert!(link.mismatch_override);
}

#[test]
fn revert_paperless_override_clears_the_flag() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    let dt = chrono::NaiveDateTime::parse_from_str("2026-05-04T13:24:14", "%Y-%m-%dT%H:%M:%S")
        .unwrap();
    let doc = PaperlessDoc {
        id: 435,
        title: "Fuel".into(),
        tag_ids: vec![],
        created: NaiveDate::from_ymd_opt(2026, 5, 4).unwrap(),
        total_amount: Some(63.34),
        litres: Some(40.0),
        receipt_datetime: Some(dt),
    };
    assign_paperless_invoice_internal(
        &db,
        &app_state,
        &doc,
        &trip_id,
        &v.id.to_string(),
        AssignmentType::Fuel,
        true,
    )
    .unwrap();
    revert_paperless_override_internal(&db, &app_state, 435).unwrap();
    let link = db.get_paperless_link(435).unwrap().unwrap();
    assert!(!link.mismatch_override);
}

#[test]
fn unassign_dispatches_paperless_source() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    db.upsert_paperless_link(&crate::models::PaperlessLink {
        paperless_document_id: 435,
        trip_id: trip_id.clone(),
        assignment_type: AssignmentType::Other,
        amount_eur: Some(15.00),
        title: Some("Iná cena".to_string()),
        applied_amount_cents: None,
        receipt_datetime: None,
        mismatch_override: false,
    })
    .unwrap();
    unassign_paperless_invoice_internal(&db, &app_state, 435).unwrap();
    assert!(db.get_paperless_link(435).unwrap().is_none());
}
