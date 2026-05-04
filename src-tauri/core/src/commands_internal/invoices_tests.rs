//! Boundary tests for invoices commands (Task 64).
use super::*;
use crate::app_state::AppState;
use crate::db::Database;
use crate::db_tests;
use crate::invoice::{InvoiceData, InvoiceRef};
use crate::models::{
    AssignmentType, ConfidenceLevel, FieldConfidence, Receipt, ReceiptStatus,
};
use chrono::{NaiveDate, Utc};
use uuid::Uuid;

/// Create a fuel receipt directly in the database, return its ID.
fn seed_fuel_receipt(db: &Database, vehicle_id: Uuid, liters: f64, price: f64) -> Uuid {
    let now = Utc::now();
    let receipt = Receipt {
        id: Uuid::new_v4(),
        vehicle_id: Some(vehicle_id),
        trip_id: None,
        file_path: format!("/x/{}.jpg", Uuid::new_v4()),
        file_name: "fuel.jpg".to_string(),
        scanned_at: now,
        liters: Some(liters),
        total_price_eur: Some(price),
        receipt_datetime: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
            .and_hms_opt(12, 0, 0),
        station_name: None,
        station_address: None,
        vendor_name: None,
        cost_description: None,
        original_amount: Some(price),
        original_currency: Some("EUR".to_string()),
        source_year: Some(2026),
        status: ReceiptStatus::Parsed,
        confidence: FieldConfidence {
            liters: ConfidenceLevel::High,
            total_price: ConfidenceLevel::High,
            date: ConfidenceLevel::High,
        },
        raw_ocr_text: None,
        error_message: None,
        assignment_type: None,
        mismatch_override: false,
        created_at: now,
        updated_at: now,
    };
    let id = receipt.id;
    db.create_receipt(&receipt).expect("seed receipt");
    id
}

fn paperless_data_fuel() -> InvoiceData {
    InvoiceData {
        datetime: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(12, 0, 0),
        liters: Some(40.5),
        total_price_eur: Some(58.20),
        title: "Doc 435".into(),
        assignment_type: AssignmentType::Fuel,
    }
}

#[test]
fn get_trips_dispatches_receipt_path_loads_from_db() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let receipt_id = seed_fuel_receipt(&db, v.id, 40.0, 58.0);

    let result = get_trips_for_invoice_assignment_internal(
        &db,
        &InvoiceRef::Receipt(receipt_id.to_string()),
        None,
        &v.id.to_string(),
        2026,
    ).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].trip.id.to_string(), trip_id);
}

#[test]
fn get_trips_dispatches_paperless_path_uses_inline_data() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    db_tests::seed_test_trip(&db, &v.id.to_string());
    let data = paperless_data_fuel();
    let result = get_trips_for_invoice_assignment_internal(
        &db,
        &InvoiceRef::Paperless(435),
        Some(&data),
        &v.id.to_string(),
        2026,
    ).unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn get_trips_paperless_without_data_errors() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let err = get_trips_for_invoice_assignment_internal(
        &db,
        &InvoiceRef::Paperless(435),
        None,
        &v.id.to_string(),
        2026,
    ).unwrap_err();
    assert!(
        err.to_lowercase().contains("invoicedata required"),
        "expected 'InvoiceData required' in error, got: {}",
        err
    );
}

#[test]
fn assign_paperless_populates_trip_fuel_when_empty() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    let data = paperless_data_fuel();
    assign_invoice_to_trip_internal(
        &db,
        &app_state,
        &InvoiceRef::Paperless(435),
        Some(&data),
        &trip_id,
        &v.id.to_string(),
        AssignmentType::Fuel,
        false,
    ).unwrap();
    let trip = db.get_trip(&trip_id).unwrap().unwrap();
    assert_eq!(trip.fuel_liters, Some(40.5));
    assert_eq!(trip.fuel_cost_eur, Some(58.20));
    assert_eq!(db.get_paperless_link_for_doc(435).unwrap(), Some(trip_id));
}

#[test]
fn assign_invoice_blocked_when_read_only() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    app_state.enable_read_only("test");
    let data = InvoiceData {
        datetime: None,
        liters: None,
        total_price_eur: None,
        title: "X".into(),
        assignment_type: AssignmentType::Other,
    };
    let err = assign_invoice_to_trip_internal(
        &db,
        &app_state,
        &InvoiceRef::Paperless(435),
        Some(&data),
        &trip_id,
        &v.id.to_string(),
        AssignmentType::Other,
        false,
    ).unwrap_err();
    let lower = err.to_lowercase();
    assert!(
        lower.contains("read") || lower.contains("čítanie"),
        "expected read-only error, got: {}",
        err
    );
}

#[test]
fn unassign_dispatches_paperless_source() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    db.upsert_paperless_link(&trip_id, 435).unwrap();
    unassign_invoice_internal(&db, &app_state, &InvoiceRef::Paperless(435)).unwrap();
    assert_eq!(db.get_paperless_link_for_doc(435).unwrap(), None);
}
