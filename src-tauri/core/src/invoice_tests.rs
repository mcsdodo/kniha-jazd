//! Tests for the Paperless invoice compatibility check (Task 84).
use super::*;
use chrono::{NaiveDate, NaiveDateTime, Utc};
use uuid::Uuid;
use crate::models::{AssignmentType, Trip, TripInvoiceCoverage};
use crate::paperless::PaperlessDoc;

// ---------------------------------------------------------------------------
// Paperless compat tests (Task 64, Task 5).
//
// PaperlessDoc isn't stored in DB, so a direct unit-test of
// `check_paperless_trip_compatibility` is the right shape.
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

// 1. Empty trip, matching date -- status "matches" (datetime at noon, inside 08:00-23:59)
#[test]
fn paperless_compat_empty_trip_same_date_matches() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let dt = date.and_hms_opt(12, 0, 0).unwrap();
    let doc = fuel_doc(dt, 45.0, 72.0);
    let trip = empty_trip(
        date.and_hms_opt(8, 0, 0).unwrap(),
        date.and_hms_opt(23, 59, 59).unwrap(),
    );
    let result = check_paperless_trip_compatibility(&doc, &trip, &TripInvoiceCoverage::default());
    assert!(result.can_attach, "Empty trip should allow attachment");
    assert_eq!(result.status, "matches", "Same date inside time range -> matches");
    assert_eq!(result.mismatch_reason, None);
}

// 2. Empty trip, different date -- "differs", and the reason must be set so the
//    picker offers the override (Task 84 review, I7). The grid warns on this
//    snapshot, so the warning has to be confirmable.
#[test]
fn paperless_compat_empty_trip_different_date_differs() {
    let trip_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let doc_date = NaiveDate::from_ymd_opt(2024, 6, 20).unwrap();
    let doc = fuel_doc(doc_date.and_hms_opt(12, 0, 0).unwrap(), 45.0, 72.0);
    let trip = empty_trip(
        trip_date.and_hms_opt(8, 0, 0).unwrap(),
        trip_date.and_hms_opt(23, 59, 59).unwrap(),
    );
    let result = check_paperless_trip_compatibility(&doc, &trip, &TripInvoiceCoverage::default());
    assert!(result.can_attach);
    assert_eq!(result.status, "differs", "Different date with empty trip -> differs");
    assert_eq!(
        result.mismatch_reason.as_deref(),
        Some("date"),
        "the grid warns on this link, so the picker must offer the override"
    );
}

// 3. Empty trip, same date but outside time range -- stays "matches_date" for the
//    picker list, but carries a reason so the override is reachable (I7).
#[test]
fn paperless_compat_empty_trip_same_date_outside_time_range() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let doc = fuel_doc(date.and_hms_opt(16, 0, 0).unwrap(), 45.0, 72.0);
    let trip = empty_trip(
        date.and_hms_opt(8, 0, 0).unwrap(),
        date.and_hms_opt(12, 0, 0).unwrap(),
    );
    let result = check_paperless_trip_compatibility(&doc, &trip, &TripInvoiceCoverage::default());
    assert!(result.can_attach);
    assert_eq!(result.status, "matches_date", "Same date but outside time range -> matches_date");
    assert_eq!(
        result.mismatch_reason.as_deref(),
        Some("time"),
        "outside the trip range means the grid warns, so it must be confirmable"
    );
}

// 4. Empty trip, datetime inside time range -- status "matches"
#[test]
fn paperless_compat_empty_trip_inside_time_range_matches() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let doc = fuel_doc(date.and_hms_opt(12, 0, 0).unwrap(), 45.0, 72.0);
    let trip = empty_trip(
        date.and_hms_opt(8, 0, 0).unwrap(),
        date.and_hms_opt(18, 0, 0).unwrap(),
    );
    let result = check_paperless_trip_compatibility(&doc, &trip, &TripInvoiceCoverage::default());
    assert!(result.can_attach);
    assert_eq!(result.status, "matches", "Datetime inside trip range -> matches");
    assert_eq!(result.mismatch_reason, None);
}

// 5. Fueled trip, matching liters + price + date -- status "matches"
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
    let result = check_paperless_trip_compatibility(&doc, &trip, &TripInvoiceCoverage::default());
    assert!(result.can_attach);
    assert_eq!(result.status, "matches", "All values matching -> matches");
    assert_eq!(result.mismatch_reason, None);
}

// 6. Fueled trip, different liters -- status "differs", reason "liters"
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
    let result = check_paperless_trip_compatibility(&doc, &trip, &TripInvoiceCoverage::default());
    assert!(result.can_attach);
    assert_eq!(result.status, "differs");
    assert_eq!(result.mismatch_reason, Some("liters".to_string()));
}

// 7. Fueled trip, different price -- status "differs", reason "price"
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
    let result = check_paperless_trip_compatibility(&doc, &trip, &TripInvoiceCoverage::default());
    assert!(result.can_attach);
    assert_eq!(result.status, "differs");
    assert_eq!(result.mismatch_reason, Some("price".to_string()));
}

// Fix 3: overnight trip -- invoice on end date but outside trip time range -> "time" not "date"
#[test]
fn compat_overnight_trip_invoice_on_end_date_outside_range_is_time_mismatch() {
    // Trip: 2024-06-15 23:00 -> 2024-06-16 01:00 (spans midnight)
    // Invoice at 2024-06-16 02:00 -- on end date, after trip end -> time mismatch (not date)
    let start = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap().and_hms_opt(23, 0, 0).unwrap();
    let end   = NaiveDate::from_ymd_opt(2024, 6, 16).unwrap().and_hms_opt(1, 0, 0).unwrap();
    let invoice_dt = NaiveDate::from_ymd_opt(2024, 6, 16).unwrap().and_hms_opt(2, 0, 0).unwrap();
    let trip = fueled_trip(start, end, 45.0, 72.0);
    let doc = fuel_doc(invoice_dt, 45.0, 72.0);
    let result = check_paperless_trip_compatibility(&doc, &trip, &TripInvoiceCoverage::default());
    assert_eq!(
        result.mismatch_reason.as_deref(), Some("time"),
        "Invoice on overnight trip's end date should be 'time' mismatch, not 'date'"
    );
}

// ---------------------------------------------------------------------------
// Multi-invoice compatibility (Task 66, test review C8/I1).
//
// The Other branch takes the trip's TripInvoiceCoverage: >=1 Other invoice
// already attached -> amount comparison skipped entirely (Matches); zero
// Others -> cent-exact compare via to_cents, so the picker verdict always
// agrees with the assign-time double-count guard. Fuel-covered trips return
// can_attach = false so the picker greys them out.
// ---------------------------------------------------------------------------

fn other_doc(dt: Option<NaiveDateTime>, price: Option<f64>) -> PaperlessDoc {
    PaperlessDoc {
        id: 7,
        title: "Parkovanie".into(),
        tag_ids: vec![],
        created: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
        total_amount: price,
        litres: None,
        receipt_datetime: dt,
    }
}

#[test]
fn test_compatibility_second_other_invoice_not_flagged_as_price_mismatch() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let dt = date.and_hms_opt(12, 0, 0).unwrap();
    // Invoice amount (20.0) wildly differs from the trip total (50.0) -- the
    // old whole-total comparison would flag this as a price mismatch.
    let doc = other_doc(Some(dt), Some(20.0));
    let mut trip = empty_trip(
        date.and_hms_opt(8, 0, 0).unwrap(),
        date.and_hms_opt(23, 59, 59).unwrap(),
    );
    trip.other_costs_eur = Some(50.0);
    let coverage = TripInvoiceCoverage { has_other: true, ..Default::default() };

    let result = check_paperless_trip_compatibility(&doc, &trip, &coverage);
    assert!(result.can_attach);
    assert_eq!(
        result.status, "matches",
        "amount comparison must be skipped when the trip already has an Other invoice"
    );
    assert_eq!(result.mismatch_reason, None);
}

#[test]
fn test_compatibility_other_uses_cent_exact_not_epsilon() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let dt = date.and_hms_opt(12, 0, 0).unwrap();
    let mut trip = empty_trip(
        date.and_hms_opt(8, 0, 0).unwrap(),
        date.and_hms_opt(23, 59, 59).unwrap(),
    );
    trip.other_costs_eur = Some(12.34);
    // Zero Other invoices attached -> comparison happens, cent-exact.
    let coverage = TripInvoiceCoverage::default();

    // 12.34 vs 12.3345: old +/-0.01 epsilon says "equal" (diff 0.0055) but the
    // Task 5 double-count guard says 1234 != 1233 and WOULD add the amount --
    // the picker must agree and report a price mismatch.
    let doc = other_doc(Some(dt), Some(12.3345));
    let result = check_paperless_trip_compatibility(&doc, &trip, &coverage);
    assert!(result.can_attach);
    assert_eq!(result.status, "differs");
    assert_eq!(result.mismatch_reason, Some("price".to_string()));

    // Exact cent equality still matches (the guard would link-only).
    let doc = other_doc(Some(dt), Some(12.34));
    let result = check_paperless_trip_compatibility(&doc, &trip, &coverage);
    assert!(result.can_attach);
    assert_eq!(result.status, "matches");
    assert_eq!(result.mismatch_reason, None);
}

#[test]
fn test_trips_for_fuel_invoice_assignment_excludes_covered_trip() {
    use crate::commands_internal::invoices::{
        get_trips_for_paperless_assignment_internal, TripForAssignment,
    };
    use crate::db::Database;
    use crate::db_tests;
    use crate::models::PaperlessLink;

    let db = Database::in_memory().unwrap();
    let vehicle = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&vehicle).unwrap();

    // Trip A: Fuel covered via a paperless link.
    let trip_a = db_tests::seed_test_trip(&db, &vehicle.id.to_string());
    db.upsert_paperless_link(&PaperlessLink {
        paperless_document_id: 900,
        trip_id: trip_a.clone(),
        assignment_type: AssignmentType::Fuel,
        amount_eur: Some(58.20),
        title: Some("Tankovanie".into()),
        applied_amount_cents: None,
        receipt_datetime: None,
        mismatch_override: false,
    })
    .unwrap();

    // Trip B: no invoice at all.
    let trip_b = db_tests::seed_test_trip(&db, &vehicle.id.to_string());

    fn can_attach(trips: &[TripForAssignment], id: &str) -> bool {
        trips
            .iter()
            .find(|t| t.trip.id.to_string() == id)
            .expect("trip present in picker list")
            .can_attach
    }

    // Paperless picker entry point.
    let doc = PaperlessDoc {
        id: 901,
        title: "Fuel doc".into(),
        tag_ids: vec![],
        created: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        total_amount: Some(58.20),
        litres: Some(40.5),
        receipt_datetime: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(12, 0, 0),
    };
    let trips = get_trips_for_paperless_assignment_internal(
        &db,
        &doc,
        &vehicle.id.to_string(),
        2026,
    )
    .unwrap();
    assert!(
        !can_attach(&trips, &trip_a),
        "paperless picker: paperless-Fuel-covered trip excluded"
    );
    assert!(
        can_attach(&trips, &trip_b),
        "paperless picker: uncovered trip stays attachable"
    );
}

// 8. Fueled trip, different date (liters + price match) -- status "differs", reason "date"
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
    let result = check_paperless_trip_compatibility(&doc, &trip, &TripInvoiceCoverage::default());
    assert!(result.can_attach);
    assert_eq!(result.status, "differs");
    assert_eq!(result.mismatch_reason, Some("date".to_string()));
}
