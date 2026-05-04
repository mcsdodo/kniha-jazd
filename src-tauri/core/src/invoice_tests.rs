//! Tests for the Invoice trait (Task 64).
use super::*;
use chrono::{NaiveDate, NaiveDateTime, Utc};
use uuid::Uuid;
use crate::models::{
    ConfidenceLevel, FieldConfidence, Receipt, ReceiptStatus, Trip,
};
use crate::paperless::PaperlessDoc;

fn make_receipt() -> Receipt {
    let now = Utc::now();
    Receipt {
        id: Uuid::nil(),
        vehicle_id: None,
        trip_id: None,
        file_path: "/x/test.jpg".to_string(),
        file_name: "test.jpg".to_string(),
        scanned_at: now,
        liters: Some(40.5),
        total_price_eur: Some(58.20),
        receipt_datetime: NaiveDate::from_ymd_opt(2026, 5, 4).unwrap()
            .and_hms_opt(13, 24, 14),
        station_name: None,
        station_address: None,
        vendor_name: None,
        cost_description: None,
        original_amount: Some(58.20),
        original_currency: Some("EUR".to_string()),
        source_year: None,
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
    }
}

#[test]
fn receipt_implements_invoice_trait_with_correct_field_mapping() {
    let r = make_receipt();
    let inv: &dyn Invoice = &r;
    assert_eq!(inv.datetime(), r.receipt_datetime);
    assert_eq!(inv.liters(), Some(40.5));
    assert_eq!(inv.total_price_eur(), Some(58.20));
    assert_eq!(inv.display_name(), "test.jpg");
    assert_eq!(inv.invoice_ref(), InvoiceRef::Receipt(Uuid::nil().to_string()));
    assert_eq!(inv.assignment_type(), None);
}

#[test]
fn paperless_doc_implements_invoice_trait_with_uk_us_field_bridge() {
    let doc = PaperlessDoc {
        id: 435,
        title: "Tank Mol Bratislava".into(),
        tag_ids: vec![51], // fuel
        created: chrono::NaiveDate::from_ymd_opt(2026, 5, 4).unwrap(),
        total_amount: Some(58.20),  // UK→US bridge
        litres: Some(40.5),         // UK→US bridge
        receipt_datetime: chrono::NaiveDate::from_ymd_opt(2026, 5, 4).unwrap()
            .and_hms_opt(13, 24, 14),
    };
    let inv: &dyn Invoice = &doc;
    assert_eq!(inv.liters(), Some(40.5));
    assert_eq!(inv.total_price_eur(), Some(58.20));
    assert_eq!(inv.display_name(), "Tank Mol Bratislava");
    assert_eq!(inv.invoice_ref(), InvoiceRef::Paperless(435));
}

#[test]
fn invoice_ref_serde_shape_matches_design() {
    let r = InvoiceRef::Receipt("abc-123".into());
    let json = serde_json::to_string(&r).unwrap();
    assert_eq!(json, r#"{"source":"receipt","id":"abc-123"}"#);

    let p = InvoiceRef::Paperless(435);
    let json = serde_json::to_string(&p).unwrap();
    assert_eq!(json, r#"{"source":"paperless","id":435}"#);

    let round: InvoiceRef = serde_json::from_str(&json).unwrap();
    assert_eq!(round, p);
}

// ---------------------------------------------------------------------------
// Paperless-side compat tests (Task 64, Task 5).
//
// These mirror the 8 receipt-side tests in commands_internal/commands_tests.rs
// (lines 2256-2585) but run the compat check directly against PaperlessDoc —
// no DB involved. PaperlessDoc isn't stored in DB, so a direct unit-test of
// `check_invoice_trip_compatibility` is the right shape.
// ---------------------------------------------------------------------------

fn empty_trip(start: NaiveDateTime, end: NaiveDateTime) -> Trip {
    let now = Utc::now();
    Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::nil(),
        start_datetime: start,
        end_datetime: Some(end),
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: 100.0,
        odometer: 10000.0,
        purpose: "business".to_string(),
        fuel_liters: None,
        fuel_cost_eur: None,
        full_tank: false,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        sort_order: 0,
        created_at: now,
        updated_at: now,
    }
}

fn fueled_trip(start: NaiveDateTime, end: NaiveDateTime, liters: f64, cost: f64) -> Trip {
    let mut t = empty_trip(start, end);
    t.fuel_liters = Some(liters);
    t.fuel_cost_eur = Some(cost);
    t.full_tank = true;
    t
}

fn fuel_doc(dt: NaiveDateTime, liters: f64, price: f64) -> PaperlessDoc {
    PaperlessDoc {
        id: 1,
        title: "Test invoice".into(),
        tag_ids: vec![51],
        created: dt.date(),
        total_amount: Some(price),
        litres: Some(liters),
        receipt_datetime: Some(dt),
    }
}

// 1. Empty trip, matching date — status "matches" (datetime at noon, inside 08:00-23:59)
#[test]
fn paperless_compat_empty_trip_same_date_matches() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let dt = date.and_hms_opt(12, 0, 0).unwrap();
    let doc = fuel_doc(dt, 45.0, 72.0);
    let trip = empty_trip(
        date.and_hms_opt(8, 0, 0).unwrap(),
        date.and_hms_opt(23, 59, 59).unwrap(),
    );
    let result = check_invoice_trip_compatibility(&doc, &trip);
    assert!(result.can_attach, "Empty trip should allow attachment");
    assert_eq!(result.status, "matches", "Same date inside time range → matches");
    assert_eq!(result.mismatch_reason, None);
}

// 2. Empty trip, different date — status "empty"
#[test]
fn paperless_compat_empty_trip_different_date_empty() {
    let trip_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let doc_date = NaiveDate::from_ymd_opt(2024, 6, 20).unwrap();
    let doc = fuel_doc(doc_date.and_hms_opt(12, 0, 0).unwrap(), 45.0, 72.0);
    let trip = empty_trip(
        trip_date.and_hms_opt(8, 0, 0).unwrap(),
        trip_date.and_hms_opt(23, 59, 59).unwrap(),
    );
    let result = check_invoice_trip_compatibility(&doc, &trip);
    assert!(result.can_attach);
    assert_eq!(result.status, "empty", "Different date with empty trip → empty");
    assert_eq!(result.mismatch_reason, None);
}

// 3. Empty trip, same date but outside time range — status "matches_date"
#[test]
fn paperless_compat_empty_trip_same_date_outside_time_range() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let doc = fuel_doc(date.and_hms_opt(16, 0, 0).unwrap(), 45.0, 72.0);
    let trip = empty_trip(
        date.and_hms_opt(8, 0, 0).unwrap(),
        date.and_hms_opt(12, 0, 0).unwrap(),
    );
    let result = check_invoice_trip_compatibility(&doc, &trip);
    assert!(result.can_attach);
    assert_eq!(result.status, "matches_date", "Same date but outside time range → matches_date");
    assert_eq!(result.mismatch_reason, None);
}

// 4. Empty trip, datetime inside time range — status "matches"
#[test]
fn paperless_compat_empty_trip_inside_time_range_matches() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let doc = fuel_doc(date.and_hms_opt(12, 0, 0).unwrap(), 45.0, 72.0);
    let trip = empty_trip(
        date.and_hms_opt(8, 0, 0).unwrap(),
        date.and_hms_opt(18, 0, 0).unwrap(),
    );
    let result = check_invoice_trip_compatibility(&doc, &trip);
    assert!(result.can_attach);
    assert_eq!(result.status, "matches", "Datetime inside trip range → matches");
    assert_eq!(result.mismatch_reason, None);
}

// 5. Fueled trip, matching liters + price + date — status "matches"
#[test]
fn paperless_compat_matching_fuel_matches() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let doc = fuel_doc(date.and_hms_opt(12, 0, 0).unwrap(), 45.0, 72.0);
    let trip = fueled_trip(
        date.and_hms_opt(8, 0, 0).unwrap(),
        date.and_hms_opt(23, 59, 59).unwrap(),
        45.0,
        72.0,
    );
    let result = check_invoice_trip_compatibility(&doc, &trip);
    assert!(result.can_attach);
    assert_eq!(result.status, "matches", "All values matching → matches");
    assert_eq!(result.mismatch_reason, None);
}

// 6. Fueled trip, different liters — status "differs", reason "liters"
#[test]
fn paperless_compat_different_liters_differs() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let doc = fuel_doc(date.and_hms_opt(12, 0, 0).unwrap(), 50.0, 72.0);
    let trip = fueled_trip(
        date.and_hms_opt(8, 0, 0).unwrap(),
        date.and_hms_opt(23, 59, 59).unwrap(),
        45.0,
        72.0,
    );
    let result = check_invoice_trip_compatibility(&doc, &trip);
    assert!(result.can_attach);
    assert_eq!(result.status, "differs");
    assert_eq!(result.mismatch_reason, Some("liters".to_string()));
}

// 7. Fueled trip, different price — status "differs", reason "price"
#[test]
fn paperless_compat_different_price_differs() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let doc = fuel_doc(date.and_hms_opt(12, 0, 0).unwrap(), 45.0, 80.0);
    let trip = fueled_trip(
        date.and_hms_opt(8, 0, 0).unwrap(),
        date.and_hms_opt(23, 59, 59).unwrap(),
        45.0,
        72.0,
    );
    let result = check_invoice_trip_compatibility(&doc, &trip);
    assert!(result.can_attach);
    assert_eq!(result.status, "differs");
    assert_eq!(result.mismatch_reason, Some("price".to_string()));
}

// 8. Fueled trip, different date (liters + price match) — status "differs", reason "date"
#[test]
fn paperless_compat_different_date_differs() {
    let trip_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let doc_date = NaiveDate::from_ymd_opt(2024, 6, 16).unwrap();
    let doc = fuel_doc(doc_date.and_hms_opt(12, 0, 0).unwrap(), 45.0, 72.0);
    let trip = fueled_trip(
        trip_date.and_hms_opt(8, 0, 0).unwrap(),
        trip_date.and_hms_opt(23, 59, 59).unwrap(),
        45.0,
        72.0,
    );
    let result = check_invoice_trip_compatibility(&doc, &trip);
    assert!(result.can_attach);
    assert_eq!(result.status, "differs");
    assert_eq!(result.mismatch_reason, Some("date".to_string()));
}
