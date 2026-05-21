//! Tests for the Invoice trait (Task 64).
use super::*;
use chrono::{NaiveDate, NaiveDateTime, Utc};
use uuid::Uuid;
use crate::models::{
    AssignmentType, ConfidenceLevel, FieldConfidence, Receipt, ReceiptStatus, Trip,
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

// Fix 3: overnight trip — invoice on end date but outside trip time range → "time" not "date"
#[test]
fn compat_overnight_trip_invoice_on_end_date_outside_range_is_time_mismatch() {
    // Trip: 2024-06-15 23:00 → 2024-06-16 01:00 (spans midnight)
    // Invoice at 2024-06-16 02:00 — on end date, after trip end → time mismatch (not date)
    let start = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap().and_hms_opt(23, 0, 0).unwrap();
    let end   = NaiveDate::from_ymd_opt(2024, 6, 16).unwrap().and_hms_opt(1, 0, 0).unwrap();
    let invoice_dt = NaiveDate::from_ymd_opt(2024, 6, 16).unwrap().and_hms_opt(2, 0, 0).unwrap();
    let trip = fueled_trip(start, end, 45.0, 72.0);
    let doc = fuel_doc(invoice_dt, 45.0, 72.0);
    let result = check_invoice_trip_compatibility(&doc, &trip);
    assert_eq!(
        result.mismatch_reason.as_deref(), Some("time"),
        "Invoice on overnight trip's end date should be 'time' mismatch, not 'date'"
    );
}

// Fix 4: assignment_type() takes precedence over liters heuristic
#[test]
fn compat_paperless_other_invoice_with_liters_routes_to_other_path() {
    // InvoiceData: AssignmentType::Other but has liters set (edge case)
    // Without fix: liters().is_some() = true → fuel path → sees fuel on trip → "differs" (liters mismatch)
    // With fix: assignment_type = Other → other path → trip has no other_costs → "matches"
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let dt = date.and_hms_opt(12, 0, 0).unwrap();
    let data = InvoiceData {
        datetime: Some(dt),
        liters: Some(40.0),
        total_price_eur: Some(25.0),
        title: "Other cost".into(),
        assignment_type: AssignmentType::Other,
    };
    let view = PaperlessInvoiceView { id: 1, data: &data };
    // Trip has fuel but no other_costs — distinguishes the two paths
    let trip = fueled_trip(
        date.and_hms_opt(8, 0, 0).unwrap(),
        date.and_hms_opt(23, 59, 59).unwrap(),
        50.0, // different from invoice liters (40.0) — fuel path would report "liters" mismatch
        30.0,
    );
    let result = check_invoice_trip_compatibility(&view, &trip);
    // Other path: trip has no other_costs → can_attach = true, mismatch_reason = None
    // Fuel path (wrong): trip has fuel, liters differ → mismatch_reason = Some("liters")
    assert!(result.can_attach);
    assert_eq!(result.mismatch_reason, None, "Other invoice should route to other path, not fuel");
}

#[test]
fn compat_paperless_fuel_invoice_with_no_liters_routes_to_fuel_path_not_other() {
    // InvoiceData: AssignmentType::Fuel, liters = None (incomplete OCR)
    // Without fix: liters().is_some() = false → routes to Other path
    // With fix: assignment_type = Fuel → routes to Fuel path
    // Distinguishable: use a trip that has other_costs but NO fuel_liters.
    // Fuel path (empty fuel): can_attach = true, status = "matches"/"empty"
    // Other path (has other_costs): can_attach = true, status = "differs" (price mismatch would show)
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let dt = date.and_hms_opt(12, 0, 0).unwrap();
    let data = InvoiceData {
        datetime: Some(dt),
        liters: None,
        total_price_eur: Some(58.0),
        title: "Fuel".into(),
        assignment_type: AssignmentType::Fuel,
    };
    let view = PaperlessInvoiceView { id: 2, data: &data };
    let mut trip = empty_trip(
        date.and_hms_opt(8, 0, 0).unwrap(),
        date.and_hms_opt(23, 59, 59).unwrap(),
    );
    // Set other_costs on trip — Other path sees this and compares prices → "differs"
    // Fuel path ignores other_costs → sees no fuel_liters → returns early "matches"
    trip.other_costs_eur = Some(99.0);
    trip.other_costs_note = Some("service".into());
    let result = check_invoice_trip_compatibility(&view, &trip);
    // With fix (Fuel path): no fuel_liters → returns early, status "matches", no mismatch
    // Without fix (Other path): has other_costs(99.0) ≠ invoice price(58.0) → "differs"
    assert!(result.can_attach);
    assert_ne!(result.status, "differs", "Fuel invoice should NOT be routed to Other path");
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
