// ============================================================================
// Tests
// ============================================================================

use crate::commands_internal::statistics::{
    calculate_consumption_warnings, calculate_duplicate_datetime_warnings,
    calculate_odometer_span_warnings, calculate_odometer_spans,
    calculate_energy_grid_data, calculate_invoice_datetime_warnings,
    calculate_invoice_override_warnings, calculate_missing_receipts,
    calculate_other_invoice_sums, calculate_other_sum_mismatches,
    calculate_suggested_fillups, get_open_period_km,
    has_any_period_over_limit, period_margin_impact, preview_anchor,
};
use crate::commands_internal::helpers::trip_order;
use super::*;
use crate::db::Database;
use crate::models::{
    AssignmentType, OdometerChange, PaperlessLink, Trip, TripInvoiceCoverage, Vehicle,
};
use chrono::{Datelike, NaiveDate, NaiveDateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// Helper to create a trip with fuel
/// Sets end_datetime to end of day to allow invoice matching for any time during the day
fn make_trip_with_fuel(date: NaiveDate, liters: f64, cost: f64) -> Trip {
    let now = Utc::now();
    Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        start_datetime: date.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: Some(date.and_hms_opt(23, 59, 59).unwrap()),
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: 100.0,
        odometer: 10000.0,
        purpose: "business".to_string(),
        fuel_liters: Some(liters),
        fuel_cost_eur: Some(cost),
        full_tank: true,
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

/// Helper to create a trip without fuel
/// Sets end_datetime to end of day to allow invoice matching for any time during the day
fn make_trip_without_fuel(date: NaiveDate) -> Trip {
    let now = Utc::now();
    Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        start_datetime: date.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: Some(date.and_hms_opt(23, 59, 59).unwrap()),
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: 50.0,
        odometer: 10050.0,
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

// ========================================================================
// Missing-invoice tests (calculate_missing_receipts)
// Source-agnostic and per-type (Task 66): the function takes the per-trip
// invoice coverage map (built from Paperless links) and flags
// trips with a fuel cost but no Fuel invoice / other costs but no Other
// invoice. Zero-value costs never need an invoice.
// ========================================================================

fn cov(
    has_fuel: bool,
    has_other: bool,
    other_sum_cents: i64,
    has_unknown_amount: bool,
) -> TripInvoiceCoverage {
    TripInvoiceCoverage {
        has_fuel,
        has_other,
        other_sum_cents,
        has_unknown_amount,
    }
}

fn coverage_for(trip_id: Uuid, entry: TripInvoiceCoverage) -> HashMap<String, TripInvoiceCoverage> {
    HashMap::from([(trip_id.to_string(), entry)])
}

#[test]
fn test_missing_receipts_trip_with_invoice_not_flagged() {
    // Trip has a Fuel invoice attached (Paperless) -> NOT missing
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
    let coverage = coverage_for(trips[0].id, cov(true, false, 0, false));

    let (missing_fuel, missing_other) = calculate_missing_receipts(&trips, &coverage);

    assert!(
        missing_fuel.is_empty() && missing_other.is_empty(),
        "Trip with attached Fuel invoice should not be flagged as missing"
    );
}

#[test]
fn test_missing_receipts_trip_without_invoice_flagged() {
    // Trip with fuel but no invoice attached → missing fuel invoice
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
    let coverage = HashMap::new();

    let (missing_fuel, missing_other) = calculate_missing_receipts(&trips, &coverage);

    assert_eq!(
        missing_fuel.len(),
        1,
        "Trip without invoice should be flagged"
    );
    assert!(missing_fuel.contains(&trips[0].id.to_string()));
    assert!(missing_other.is_empty(), "No other costs on this trip");
}

#[test]
fn test_missing_receipts_trip_without_costs_not_flagged() {
    // Trip without fuel or other_costs → NOT flagged (doesn't need an invoice)
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_without_fuel(date)];
    let coverage = HashMap::new();

    let (missing_fuel, missing_other) = calculate_missing_receipts(&trips, &coverage);

    assert!(
        missing_fuel.is_empty() && missing_other.is_empty(),
        "Trip without fuel/other_costs should not be flagged"
    );
}

#[test]
fn test_missing_receipts_trip_with_other_costs_and_no_invoice_flagged() {
    // Trip with other_costs but no invoice → missing Other invoice
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let mut trip = make_trip_without_fuel(date);
    trip.other_costs_eur = Some(15.0);
    trip.other_costs_note = Some("Parkovanie".to_string());
    let trips = vec![trip];
    let coverage = HashMap::new();

    let (missing_fuel, missing_other) = calculate_missing_receipts(&trips, &coverage);

    assert_eq!(missing_other.len(), 1);
    assert!(missing_other.contains(&trips[0].id.to_string()));
    assert!(missing_fuel.is_empty(), "No fuel on this trip");
}

#[test]
fn test_missing_receipts_multiple_trips_partial_coverage() {
    // Multiple trips: trip 1 covered, trip 2 not, trip 3 has no costs
    let date1 = NaiveDate::from_ymd_opt(2024, 6, 10).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2024, 6, 20).unwrap();
    let date3 = NaiveDate::from_ymd_opt(2024, 6, 30).unwrap();

    let trips = vec![
        make_trip_with_fuel(date1, 40.0, 65.00), // covered
        make_trip_with_fuel(date2, 50.0, 80.00), // missing
        make_trip_without_fuel(date3),           // no costs, not flagged
    ];
    let coverage = coverage_for(trips[0].id, cov(true, false, 0, false));

    let (missing_fuel, missing_other) = calculate_missing_receipts(&trips, &coverage);

    assert_eq!(missing_fuel.len(), 1);
    assert!(missing_fuel.contains(&trips[1].id.to_string()));
    assert!(!missing_fuel.contains(&trips[0].id.to_string()));
    assert!(!missing_fuel.contains(&trips[2].id.to_string()));
    assert!(missing_other.is_empty());
}

#[test]
fn test_missing_fuel_invoice_flagged_per_type() {
    // Trip has fuel cost + other cost; only Other invoice attached
    // -> in missing_fuel_invoices, NOT in missing_other_invoices
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let mut trip = make_trip_with_fuel(date, 45.0, 72.50);
    trip.other_costs_eur = Some(15.0);
    let trips = vec![trip];
    let coverage = coverage_for(trips[0].id, cov(false, true, 1500, false));

    let (missing_fuel, missing_other) = calculate_missing_receipts(&trips, &coverage);

    assert!(
        missing_fuel.contains(&trips[0].id.to_string()),
        "Fuel cost without Fuel invoice should be flagged"
    );
    assert!(
        missing_other.is_empty(),
        "Other cost IS covered — must not be flagged"
    );
}

#[test]
fn test_missing_other_invoice_flagged_per_type() {
    // Mirror case: fuel cost + other cost; only Fuel invoice attached
    // -> in missing_other_invoices, NOT in missing_fuel_invoices
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let mut trip = make_trip_with_fuel(date, 45.0, 72.50);
    trip.other_costs_eur = Some(15.0);
    let trips = vec![trip];
    let coverage = coverage_for(trips[0].id, cov(true, false, 0, false));

    let (missing_fuel, missing_other) = calculate_missing_receipts(&trips, &coverage);

    assert!(
        missing_other.contains(&trips[0].id.to_string()),
        "Other cost without Other invoice should be flagged"
    );
    assert!(
        missing_fuel.is_empty(),
        "Fuel IS covered — must not be flagged"
    );
}

#[test]
fn test_zero_value_costs_not_flagged_missing() {
    // other_costs_eur = Some(0.0) / fuel_liters = Some(0.0) -> NOT flagged
    // (pins the is_some() -> "> 0" predicate change, test review I9)
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let mut trip = make_trip_with_fuel(date, 0.0, 0.0);
    trip.other_costs_eur = Some(0.0);
    let trips = vec![trip];
    let coverage = HashMap::new();

    let (missing_fuel, missing_other) = calculate_missing_receipts(&trips, &coverage);

    assert!(
        missing_fuel.is_empty() && missing_other.is_empty(),
        "Zero-value costs do not need an invoice"
    );
}

// ========================================================================
// Other sum-mismatch tests (calculate_other_sum_mismatches)
// ========================================================================

#[test]
fn test_other_sum_mismatch_flagged() {
    // other_costs_eur = 20.00, attached Others sum 15.01 -> trip in other_sum_mismatches
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let mut trip = make_trip_without_fuel(date);
    trip.other_costs_eur = Some(20.00);
    let trips = vec![trip];
    let coverage = coverage_for(trips[0].id, cov(false, true, 1501, false));

    let mismatches = calculate_other_sum_mismatches(&trips, &coverage);

    assert!(
        mismatches.contains(&trips[0].id.to_string()),
        "20.00 vs invoice sum 15.01 should be flagged as mismatch"
    );
}

#[test]
fn test_other_sum_match_not_flagged() {
    // 15.01 vs 15.01 -> absent
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let mut trip = make_trip_without_fuel(date);
    trip.other_costs_eur = Some(15.01);
    let trips = vec![trip];
    let coverage = coverage_for(trips[0].id, cov(false, true, 1501, false));

    let mismatches = calculate_other_sum_mismatches(&trips, &coverage);

    assert!(
        mismatches.is_empty(),
        "Matching sum (cent-exact) must not be flagged"
    );
}

#[test]
fn test_other_sum_unknown_amount_skips_check() {
    // One Other link with NULL amount -> trip absent even though partial sum differs
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let mut trip = make_trip_without_fuel(date);
    trip.other_costs_eur = Some(20.00);
    let trips = vec![trip];
    // Partial sum 5.00 differs from 20.00, but an unknown-amount invoice exists
    let coverage = coverage_for(trips[0].id, cov(false, true, 500, true));

    let mismatches = calculate_other_sum_mismatches(&trips, &coverage);

    assert!(
        mismatches.is_empty(),
        "Unknown invoice amount must skip the sum-mismatch check"
    );
}

#[test]
fn test_other_invoice_sums_exposed_for_mismatched_trips() {
    // The sum-mismatch tooltip shows "{total} € vs {sum} €" — the invoice sum
    // must ship with the grid data for flagged trips only (ADR-008: no
    // frontend calculation, and no payload for trips without a warning).
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let mut mismatched = make_trip_without_fuel(date);
    mismatched.other_costs_eur = Some(20.00);
    let mut matching = make_trip_without_fuel(date);
    matching.other_costs_eur = Some(15.01);
    let trips = vec![mismatched, matching];

    let mut coverage = coverage_for(trips[0].id, cov(false, true, 1501, false));
    coverage.extend(coverage_for(trips[1].id, cov(false, true, 1501, false)));

    let mismatches = calculate_other_sum_mismatches(&trips, &coverage);
    let sums = calculate_other_invoice_sums(&mismatches, &coverage);

    assert_eq!(
        sums.get(&trips[0].id.to_string()),
        Some(&15.01),
        "Flagged trip ships its cent-exact invoice sum in EUR"
    );
    assert!(
        !sums.contains_key(&trips[1].id.to_string()),
        "Unflagged trip must not appear in the sums map"
    );
}

// ========================================================================
// Invoice datetime warning tests (calculate_invoice_datetime_warnings)
// ========================================================================

/// Helper to create a trip with specific start and end datetimes
fn make_trip_with_datetime_range(
    start_datetime: NaiveDateTime,
    end_datetime: Option<NaiveDateTime>,
) -> Trip {
    let now = Utc::now();
    Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        start_datetime,
        end_datetime,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: 100.0,
        odometer: 10000.0,
        purpose: "business".to_string(),
        fuel_liters: Some(45.0),
        fuel_cost_eur: Some(72.50),
        full_tank: true,
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

/// Helper to create a Paperless link with a specific datetime snapshot and
/// override flag, assigned to a trip.
fn make_paperless_link(
    trip_id: Uuid,
    assignment_type: AssignmentType,
    receipt_datetime: Option<NaiveDateTime>,
    mismatch_override: bool,
) -> PaperlessLink {
    PaperlessLink {
        paperless_document_id: 1,
        trip_id: trip_id.to_string(),
        assignment_type,
        amount_eur: Some(50.0),
        title: Some("doc".to_string()),
        applied_amount_cents: None,
        receipt_datetime,
        mismatch_override,
    }
}

#[test]
fn datetime_warning_flags_link_outside_trip_range() {
    // Fuel link with a datetime outside [start, end] -> fuel warning
    let trip_start = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let trip_end = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap();
    let link_dt = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(20, 0, 0)
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, Some(trip_end));
    let link = make_paperless_link(trip.id, AssignmentType::Fuel, Some(link_dt), false);

    let (fuel, other) = calculate_invoice_datetime_warnings(&[trip.clone()], &[link]);

    assert!(
        fuel.contains(&trip.id.to_string()),
        "Out-of-range Fuel link must land in fuel_datetime_warnings"
    );
    assert!(
        other.is_empty(),
        "No Other link -- other_datetime_warnings must stay empty"
    );
}

#[test]
fn datetime_warning_ignores_link_inside_range() {
    // Link datetime inside trip [start, end] -> no warning
    let trip_start = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let trip_end = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(17, 0, 0)
        .unwrap();
    let link_dt = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, Some(trip_end));
    let link = make_paperless_link(trip.id, AssignmentType::Fuel, Some(link_dt), false);

    let (fuel, other) = calculate_invoice_datetime_warnings(&[trip], &[link]);

    assert!(fuel.is_empty(), "Link within range must not warn");
    assert!(other.is_empty());
}

#[test]
fn datetime_warning_ignores_link_without_datetime() {
    // Link with None datetime -> no warning (cannot validate)
    let trip_start = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, None);
    let link = make_paperless_link(trip.id, AssignmentType::Fuel, None, false);

    let (fuel, other) = calculate_invoice_datetime_warnings(&[trip], &[link]);

    assert!(fuel.is_empty(), "Link without datetime must not warn");
    assert!(other.is_empty());
}

#[test]
fn datetime_warning_honours_inclusive_boundaries() {
    // Datetime exactly at start or end is inside the range
    let trip_start = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let trip_end = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(17, 0, 0)
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, Some(trip_end));
    let at_start = make_paperless_link(trip.id, AssignmentType::Fuel, Some(trip_start), false);
    let at_end = make_paperless_link(trip.id, AssignmentType::Fuel, Some(trip_end), false);

    let (start_fuel, _) = calculate_invoice_datetime_warnings(&[trip.clone()], &[at_start]);
    let (end_fuel, _) = calculate_invoice_datetime_warnings(&[trip], &[at_end]);

    assert!(start_fuel.is_empty(), "Link at exact start must not warn");
    assert!(end_fuel.is_empty(), "Link at exact end must not warn");
}

#[test]
fn datetime_warning_uses_start_when_no_end() {
    // Trip without end_datetime: the range is just the start instant
    let trip_start = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let later = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, None);
    let later_link = make_paperless_link(trip.id, AssignmentType::Fuel, Some(later), false);
    let exact_link = make_paperless_link(trip.id, AssignmentType::Fuel, Some(trip_start), false);

    let (later_fuel, _) = calculate_invoice_datetime_warnings(&[trip.clone()], &[later_link]);
    let (exact_fuel, _) = calculate_invoice_datetime_warnings(&[trip], &[exact_link]);

    assert_eq!(
        later_fuel.len(),
        1,
        "Later link must warn when no end_datetime"
    );
    assert!(exact_fuel.is_empty(), "Link at exact start must not warn");
}

#[test]
fn datetime_warning_splits_by_assignment_type() {
    // Out-of-range Other link -> other warning only
    let trip_start = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let trip_end = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(14, 0, 0)
        .unwrap();
    let out_of_range = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(18, 0, 0)
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, Some(trip_end));
    let link = make_paperless_link(trip.id, AssignmentType::Other, Some(out_of_range), false);

    let (fuel, other) = calculate_invoice_datetime_warnings(&[trip.clone()], &[link]);

    assert!(
        other.contains(&trip.id.to_string()),
        "Out-of-range Other link must land in other_datetime_warnings"
    );
    assert!(
        fuel.is_empty(),
        "No Fuel link -- fuel_datetime_warnings must stay empty"
    );
}

#[test]
fn datetime_warning_fires_for_second_link() {
    // Trip with in-range Fuel link + out-of-range Other link -> other warning
    // fires even though the mismatched link is not first (test review I8)
    let trip_start = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let trip_end = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(14, 0, 0)
        .unwrap();
    let in_range = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let out_of_range = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(18, 0, 0)
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, Some(trip_end));
    let fuel_link = make_paperless_link(trip.id, AssignmentType::Fuel, Some(in_range), false);
    let other_link = make_paperless_link(trip.id, AssignmentType::Other, Some(out_of_range), false);

    let (fuel, other) =
        calculate_invoice_datetime_warnings(&[trip.clone()], &[fuel_link, other_link]);

    assert!(
        other.contains(&trip.id.to_string()),
        "Second (Other) link out of range must fire other_datetime_warnings"
    );
    assert!(fuel.is_empty(), "Fuel link is in range -- no fuel warning");
}

// ========================================================================
// Invoice mismatch override tests (calculate_invoice_override_warnings)
// ========================================================================

#[test]
fn override_warning_flags_only_true_links() {
    // Other link with mismatch_override = true -> other warning
    let trip_start = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let trip_end = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap();
    let in_range = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(8, 30, 0)
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, Some(trip_end));
    let link = make_paperless_link(trip.id, AssignmentType::Other, Some(in_range), true);

    let (fuel, other) = calculate_invoice_override_warnings(&[trip.clone()], &[link]);

    assert!(
        other.contains(&trip.id.to_string()),
        "Confirmed Other link must land in other_mismatch_overrides"
    );
    assert!(
        fuel.is_empty(),
        "No Fuel link -- fuel_mismatch_overrides must stay empty"
    );
}

#[test]
fn override_warning_ignores_false_links() {
    // mismatch_override = false -> no warning even when the datetime mismatches
    let trip_start = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let trip_end = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap();
    let out_of_range = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(20, 0, 0)
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, Some(trip_end));
    let link = make_paperless_link(trip.id, AssignmentType::Fuel, Some(out_of_range), false);

    let (fuel, other) = calculate_invoice_override_warnings(&[trip], &[link]);

    assert!(fuel.is_empty(), "Unconfirmed link must not warn");
    assert!(other.is_empty());
}

#[test]
fn override_warning_recognized_on_second_link() {
    // I8 mirror: first (Fuel) link has no override, second (Other) link has
    // mismatch_override = true -> other_mismatch_overrides must contain the trip.
    let trip_start = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let trip_end = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(14, 0, 0)
        .unwrap();
    let in_range = NaiveDate::from_ymd_opt(2026, 5, 4)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, Some(trip_end));
    let fuel_link = make_paperless_link(trip.id, AssignmentType::Fuel, Some(in_range), false);
    let other_link = make_paperless_link(trip.id, AssignmentType::Other, Some(in_range), true);

    let (fuel, other) =
        calculate_invoice_override_warnings(&[trip.clone()], &[fuel_link, other_link]);

    assert!(
        other.contains(&trip.id.to_string()),
        "Override on the second (Other) link must be recognized"
    );
    assert!(
        fuel.is_empty(),
        "Fuel link has no override -- fuel_mismatch_overrides must stay empty"
    );
}

// ========================================================================
// Migration: no false warnings after upgrade (Task 66, test review I5)
// ========================================================================

/// Raw-SQL exec against the test DB (legacy schema has no Rust structs).
fn exec_raw(db: &Database, sql: &str) {
    use diesel::RunQueryDsl;
    let conn = &mut *db.connection();
    diesel::sql_query(sql)
        .execute(conn)
        .unwrap_or_else(|e| panic!("SQL failed: {e}\n{sql}"));
}

#[test]
fn test_migrated_db_produces_no_false_warnings() {
    // Legacy DB: trip with other_costs_eur = 12.50 + one legacy paperless
    // link (backfills to Other, amount NULL). After migration + coverage:
    // trip absent from other_sum_mismatches (unknown amount excluded) AND
    // absent from missing_other_invoices (has_other = true). (I5 — "don't
    // scare users after update".)
    let db = crate::db::open_db_legacy();

    let vehicle_id = "00000000-0000-0000-0000-0000000000a1";
    let trip_id = "00000000-0000-0000-0000-0000000000b1";
    exec_raw(
        &db,
        &format!(
            "INSERT INTO vehicles (id, name, license_plate, created_at, updated_at) \
             VALUES ('{vehicle_id}', 'Test Vehicle', 'BA123XY', \
                     '2026-01-01T00:00:00', '2026-01-01T00:00:00')"
        ),
    );
    exec_raw(
        &db,
        &format!(
            "INSERT INTO trips (id, vehicle_id, origin, destination, distance_km, odometer, \
                                purpose, other_costs_eur, start_datetime, end_datetime, \
                                created_at, updated_at) \
             VALUES ('{trip_id}', '{vehicle_id}', 'BA', 'TT', 50.0, 12345.0, 'test', 12.50, \
                     '2026-01-01T08:00:00', '2026-01-01T10:00:00', \
                     '2026-01-01T00:00:00', '2026-01-01T00:00:00')"
        ),
    );
    // Legacy paperless link shape: no assignment_type/amount columns yet
    exec_raw(
        &db,
        &format!(
            "INSERT INTO paperless_trip_links (trip_id, paperless_document_id, \
                                               created_at, updated_at) \
             VALUES ('{trip_id}', 42, '2026-04-01T08:00:00', '2026-04-01T08:00:00')"
        ),
    );

    crate::db::migrate_to_current(&db);

    let trips = db.get_trips_for_vehicle_in_year(&vehicle_id, 2026).unwrap();
    assert_eq!(trips.len(), 1, "Seeded trip must survive the migration");
    let coverage = db.get_trip_invoice_coverage().unwrap();

    let (_, missing_other) = calculate_missing_receipts(&trips, &coverage);
    assert!(
        !missing_other.contains(trip_id),
        "Trip with a backfilled Other link must not be flagged as missing an Other invoice"
    );

    let mismatches = calculate_other_sum_mismatches(&trips, &coverage);
    assert!(
        !mismatches.contains(trip_id),
        "Backfilled link has unknown amount — sum-mismatch check must be skipped"
    );
}

// ========================================================================
// Period rate calculation tests (calculate_period_rates)
// ========================================================================

/// Helper to create a trip with specific km, fuel, and full_tank flag
fn make_trip_detailed(
    date: NaiveDate,
    distance_km: f64,
    fuel_liters: Option<f64>,
    full_tank: bool,
) -> Trip {
    let now = Utc::now();
    Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        start_datetime: date.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: None,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km,
        odometer: 10000.0 + distance_km,
        purpose: "business".to_string(),
        fuel_liters,
        fuel_cost_eur: fuel_liters.map(|l| l * 1.5),
        full_tank,
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

#[test]
fn test_period_rates_partial_fillup_doesnt_close_period() {
    // Business rule: Only full_tank=true closes a consumption period
    // Partial fillups (full_tank=false) accumulate fuel but don't close
    let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let tp_rate = 6.0;

    let trips = vec![
        make_trip_detailed(base_date, 100.0, None, false), // 100km, no fuel
        make_trip_detailed(base_date.succ_opt().unwrap(), 100.0, Some(20.0), false), // 100km, 20L PARTIAL
        make_trip_detailed(
            base_date.succ_opt().unwrap().succ_opt().unwrap(),
            100.0,
            None,
            false,
        ), // 100km, no fuel
        make_trip_detailed(
            base_date
                .succ_opt()
                .unwrap()
                .succ_opt()
                .unwrap()
                .succ_opt()
                .unwrap(),
            100.0,
            Some(30.0),
            true,
        ), // 100km, 30L FULL
    ];

    let (rates, estimated) = calculate_period_rates(&trips, tp_rate);

    // All 4 trips should get same rate: 50L / 400km * 100 = 12.5 l/100km
    // The partial fillup at trip 2 should NOT create a separate period
    let expected_rate = 50.0 / 400.0 * 100.0; // 12.5
    for trip in &trips {
        let rate = rates.get(&trip.id.to_string()).unwrap();
        assert!(
            (rate - expected_rate).abs() < 0.01,
            "All trips should have rate {:.2}, got {:.2}",
            expected_rate,
            rate
        );
        assert!(
            !estimated.contains(&trip.id.to_string()),
            "All trips should have calculated (not estimated) rate"
        );
    }
}

#[test]
fn test_period_rates_full_fillup_closes_period() {
    // Full tank fillups should close periods and create new rate calculations
    let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let tp_rate = 6.0;

    let trips = vec![
        make_trip_detailed(base_date, 100.0, None, false), // Period 1: 100km
        make_trip_detailed(base_date.succ_opt().unwrap(), 100.0, Some(10.0), true), // Period 1: closes with 10L -> rate = 10/200*100 = 5.0
        make_trip_detailed(
            base_date.succ_opt().unwrap().succ_opt().unwrap(),
            200.0,
            None,
            false,
        ), // Period 2: 200km
        make_trip_detailed(
            base_date
                .succ_opt()
                .unwrap()
                .succ_opt()
                .unwrap()
                .succ_opt()
                .unwrap(),
            200.0,
            Some(16.0),
            true,
        ), // Period 2: closes with 16L -> rate = 16/400*100 = 4.0
    ];

    let (rates, _) = calculate_period_rates(&trips, tp_rate);

    // Period 1 (trips 0-1): rate = 10L / 200km * 100 = 5.0
    let rate_period1 = 10.0 / 200.0 * 100.0;
    assert!(
        (rates.get(&trips[0].id.to_string()).unwrap() - rate_period1).abs() < 0.01,
        "Trip 0 should have period 1 rate"
    );
    assert!(
        (rates.get(&trips[1].id.to_string()).unwrap() - rate_period1).abs() < 0.01,
        "Trip 1 should have period 1 rate"
    );

    // Period 2 (trips 2-3): rate = 16L / 400km * 100 = 4.0
    let rate_period2 = 16.0 / 400.0 * 100.0;
    assert!(
        (rates.get(&trips[2].id.to_string()).unwrap() - rate_period2).abs() < 0.01,
        "Trip 2 should have period 2 rate"
    );
    assert!(
        (rates.get(&trips[3].id.to_string()).unwrap() - rate_period2).abs() < 0.01,
        "Trip 3 should have period 2 rate"
    );
}

#[test]
fn test_period_rates_no_fullup_uses_tp_rate() {
    // When no full-tank fillup exists, use TP rate (estimated)
    let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let tp_rate = 6.0;

    let trips = vec![
        make_trip_detailed(base_date, 100.0, None, false),
        make_trip_detailed(base_date.succ_opt().unwrap(), 100.0, Some(15.0), false), // Partial only
    ];

    let (rates, estimated) = calculate_period_rates(&trips, tp_rate);

    // All trips should use TP rate (estimated)
    for trip in &trips {
        let rate = rates.get(&trip.id.to_string()).unwrap();
        assert!(
            (rate - tp_rate).abs() < 0.01,
            "Should use TP rate when no full fillup"
        );
        assert!(
            estimated.contains(&trip.id.to_string()),
            "Trips should be marked as estimated"
        );
    }
}

// ========================================================================
// Consumption warning tests (calculate_consumption_warnings)
// ========================================================================

#[test]
fn test_consumption_warnings_over_120_percent() {
    // Trip with rate > 120% of TP should be flagged
    let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let tp_rate = 5.0; // TP rate
    let limit = tp_rate * 1.2; // 6.0

    let trips = vec![
        make_trip_detailed(base_date, 100.0, Some(7.5), true), // Rate = 7.5 l/100km > 6.0 limit
    ];

    let mut rates = std::collections::HashMap::new();
    rates.insert(trips[0].id.to_string(), 7.5);

    let warnings = calculate_consumption_warnings(&trips, &rates, tp_rate);

    assert!(
        warnings.contains(&trips[0].id.to_string()),
        "Trip with rate {:.1} > limit {:.1} should be flagged",
        7.5,
        limit
    );
}

#[test]
fn test_consumption_warnings_at_limit_not_flagged() {
    // Trip with rate exactly at 120% should NOT be flagged (not OVER)
    let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let tp_rate = 5.0;
    let at_limit_rate = tp_rate * 1.2; // Exactly 6.0

    let trips = vec![make_trip_detailed(base_date, 100.0, Some(6.0), true)];

    let mut rates = std::collections::HashMap::new();
    rates.insert(trips[0].id.to_string(), at_limit_rate);

    let warnings = calculate_consumption_warnings(&trips, &rates, tp_rate);

    assert!(
        warnings.is_empty(),
        "Trip at exactly 120% limit should NOT be flagged (limit is 'greater than', not 'greater or equal')"
    );
}

#[test]
fn test_consumption_warnings_under_limit_not_flagged() {
    // Trip with rate under limit should not be flagged
    let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let tp_rate = 5.0;

    let trips = vec![
        make_trip_detailed(base_date, 100.0, Some(5.0), true), // Rate = 5.0 < 6.0 limit
    ];

    let mut rates = std::collections::HashMap::new();
    rates.insert(trips[0].id.to_string(), 5.0);

    let warnings = calculate_consumption_warnings(&trips, &rates, tp_rate);

    assert!(
        warnings.is_empty(),
        "Trip under limit should not be flagged"
    );
}

// ========================================================================
// Duplicate start-datetime tests (calculate_duplicate_datetime_warnings)
// ========================================================================

/// Build a trip with an exact start datetime (make_trip_detailed pins 08:00).
fn make_trip_at(date: NaiveDate, hour: u32, minute: u32) -> Trip {
    let mut trip = make_trip_detailed(date, 10.0, None, false);
    trip.start_datetime = date.and_hms_opt(hour, minute, 0).unwrap();
    trip
}

#[test]
fn test_duplicate_datetime_flags_both_trips() {
    // Two trips at the identical start datetime: the grid order then falls to
    // created_at, which is data-entry order, not travel order (task 79).
    let date = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
    let trips = vec![make_trip_at(date, 15, 0), make_trip_at(date, 15, 0)];

    let warnings = calculate_duplicate_datetime_warnings(&trips);

    assert_eq!(warnings.len(), 2, "Both trips of the pair must be flagged");
    assert!(warnings.contains(&trips[0].id.to_string()));
    assert!(warnings.contains(&trips[1].id.to_string()));
}

#[test]
fn test_duplicate_datetime_flags_all_members_of_a_group() {
    let date = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
    let trips = vec![
        make_trip_at(date, 15, 0),
        make_trip_at(date, 15, 0),
        make_trip_at(date, 15, 0),
        make_trip_at(date, 16, 0), // alone at 16:00
    ];

    let warnings = calculate_duplicate_datetime_warnings(&trips);

    assert_eq!(warnings.len(), 3, "All three of the 15:00 group must be flagged");
    assert!(!warnings.contains(&trips[3].id.to_string()), "The lone 16:00 trip must stay clean");
}

#[test]
fn test_duplicate_datetime_same_date_different_time_is_clean() {
    // Several trips on one day are normal. Only an identical time is a problem.
    let date = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
    let trips = vec![
        make_trip_at(date, 8, 0),
        make_trip_at(date, 8, 30),
        make_trip_at(date, 15, 0),
    ];

    let warnings = calculate_duplicate_datetime_warnings(&trips);

    assert!(warnings.is_empty(), "Same date with different times must not warn");
}

#[test]
fn test_duplicate_datetime_distinct_dates_is_clean() {
    let date = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
    let trips = vec![
        make_trip_at(date, 15, 0),
        make_trip_at(date.succ_opt().unwrap(), 15, 0),
    ];

    let warnings = calculate_duplicate_datetime_warnings(&trips);

    assert!(warnings.is_empty(), "The same time on different days must not warn");
}

// ========================================================================
// Odometer span tests (calculate_odometer_span_warnings)
// ========================================================================

/// Build the odometer_start map the grid passes to the span check.
fn odo_starts(pairs: &[(&Trip, f64)]) -> HashMap<String, f64> {
    pairs
        .iter()
        .map(|(trip, start)| (trip.id.to_string(), *start))
        .collect()
}

#[test]
fn test_odometer_span_matching_distance_is_clean() {
    let date = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
    let mut trip = make_trip_at(date, 8, 0);
    trip.distance_km = 50.0;
    trip.odometer = 69465.0;

    let starts = odo_starts(&[(&trip, 69415.0)]);
    let warnings = calculate_odometer_span_warnings(&[trip], &starts);

    assert!(warnings.is_empty(), "A span equal to the recorded distance must not warn");
}

#[test]
fn test_odometer_span_longer_than_distance_is_flagged() {
    // Task 79, row 03f46d80: 4 km recorded, 356 km of odometer.
    let date = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
    let mut trip = make_trip_at(date, 15, 0);
    trip.distance_km = 4.0;
    trip.odometer = 69415.0;

    let starts = odo_starts(&[(&trip, 69059.0)]);
    let warnings = calculate_odometer_span_warnings(std::slice::from_ref(&trip), &starts);

    assert!(warnings.contains(&trip.id.to_string()), "A 356 km span on a 4 km trip must warn");
}

#[test]
fn test_odometer_span_negative_is_flagged() {
    // Task 79, row a51cb498: the odometer goes backwards, which is impossible.
    let date = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
    let mut trip = make_trip_at(date, 15, 0);
    trip.distance_km = 352.0;
    trip.odometer = 69411.0;

    let starts = odo_starts(&[(&trip, 69415.0)]);
    let warnings = calculate_odometer_span_warnings(std::slice::from_ref(&trip), &starts);

    assert!(warnings.contains(&trip.id.to_string()), "A negative span must always warn");
}

#[test]
fn test_odometer_span_half_kilometre_stays_quiet() {
    // A half kilometre carried over a year boundary is not an error worth a
    // warning: the grid shows odometers to whole kilometres.
    let date = NaiveDate::from_ymd_opt(2025, 1, 12).unwrap();
    let mut trip = make_trip_at(date, 8, 0);
    trip.distance_km = 88.0;
    trip.odometer = 38145.0;

    let starts = odo_starts(&[(&trip, 38056.5)]);
    let warnings = calculate_odometer_span_warnings(std::slice::from_ref(&trip), &starts);

    assert!(warnings.is_empty(), "A 0.5 km difference is under the tolerance");
}

#[test]
fn test_odometer_span_one_kilometre_is_flagged() {
    // The tolerance boundary: 1.0 km is the first difference that warns.
    let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
    let mut trip = make_trip_at(date, 8, 0);
    trip.distance_km = 50.0;
    trip.odometer = 1051.0;

    let starts = odo_starts(&[(&trip, 1000.0)]);
    let warnings = calculate_odometer_span_warnings(std::slice::from_ref(&trip), &starts);

    assert!(warnings.contains(&trip.id.to_string()), "A full kilometre of difference must warn");
}

#[test]
fn test_odometer_span_first_row_of_year_uses_carryover() {
    // The first row of a year has no previous trip. Its start comes from the
    // year carryover, so a clean first row must not warn.
    let date = NaiveDate::from_ymd_opt(2026, 1, 3).unwrap();
    let mut trip = make_trip_at(date, 8, 0);
    trip.distance_km = 370.0;
    trip.odometer = 55275.0;

    let starts = odo_starts(&[(&trip, 54905.0)]);
    let warnings = calculate_odometer_span_warnings(std::slice::from_ref(&trip), &starts);

    assert!(warnings.is_empty(), "A first row seeded from the carryover must not warn");
}

#[test]
fn test_odometer_span_missing_start_does_not_warn() {
    // No derived start means nothing to compare. Stay silent rather than guess.
    let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
    let mut trip = make_trip_at(date, 8, 0);
    trip.distance_km = 50.0;
    trip.odometer = 1500.0;

    let warnings = calculate_odometer_span_warnings(std::slice::from_ref(&trip), &HashMap::new());

    assert!(warnings.is_empty(), "A trip with no derived start must not warn");
}

#[test]
fn test_odometer_spans_reported_for_flagged_trips_only() {
    // The grid tooltip shows the measured span, so the backend sends it
    // (ADR-008). Only flagged rows carry a value.
    let date = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
    let mut broken = make_trip_at(date, 15, 0);
    broken.distance_km = 4.0;
    broken.odometer = 69415.0;
    let mut clean = make_trip_at(date, 16, 0);
    clean.distance_km = 50.0;
    clean.odometer = 69465.0;

    let starts = odo_starts(&[(&broken, 69059.0), (&clean, 69415.0)]);
    let trips = vec![broken.clone(), clean.clone()];
    let warnings = calculate_odometer_span_warnings(&trips, &starts);
    let spans = calculate_odometer_spans(&warnings, &starts, &trips);

    assert_eq!(spans.len(), 1, "Only the flagged trip carries a span");
    assert_eq!(spans.get(&broken.id.to_string()), Some(&356.0));
}

// ========================================================================
// Trip order tests (trip_order) - task 1
// ========================================================================

#[test]
fn test_trip_order_prefers_created_at_over_odometer() {
    // The 2026-08-19 pair: same datetime, different created_at. The places
    // prove the 4 km hop to the petrol station came first, and created_at
    // agrees. The odometer must not override that.
    let date = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
    let mut first = make_trip_at(date, 15, 0); // 4 km hop, higher odometer
    first.created_at = Utc::now() - chrono::Duration::seconds(60);
    first.odometer = 69415.0;
    let mut second = make_trip_at(date, 15, 0); // 352 km leg, lower odometer
    second.created_at = Utc::now();
    second.odometer = 69411.0;

    assert_eq!(trip_order(&first, &second), std::cmp::Ordering::Less);
}

#[test]
fn test_trip_order_falls_back_to_odometer_when_created_at_ties() {
    // The imported rows: one datetime, one identical import timestamp. The
    // odometer is then the only surviving evidence of order.
    let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
    let stamp = Utc::now();
    let mut low = make_trip_at(date, 0, 0);
    low.created_at = stamp;
    low.odometer = 17416.0;
    let mut high = make_trip_at(date, 0, 0);
    high.created_at = stamp;
    high.odometer = 17618.0;
    // Ids run opposite to the expected odometer order, so a dropped odometer
    // key falls through to id and fails the assertion below every run.
    low.id = Uuid::from_u128(2);
    high.id = Uuid::from_u128(1);

    assert_eq!(trip_order(&low, &high), std::cmp::Ordering::Less);
}

#[test]
fn test_trip_order_is_total() {
    // Same datetime, same created_at, same odometer: id decides, so the
    // result never depends on the DB row order or a stable-sort accident.
    let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
    let stamp = Utc::now();
    let mut a = make_trip_at(date, 0, 0);
    let mut b = make_trip_at(date, 0, 0);
    a.created_at = stamp;
    b.created_at = stamp;
    a.odometer = 100.0;
    b.odometer = 100.0;

    assert_ne!(trip_order(&a, &b), std::cmp::Ordering::Equal);
    assert_eq!(trip_order(&a, &b), a.id.cmp(&b.id));
}

#[test]
fn test_trip_numbers_and_odometer_start_agree_on_a_tied_group() {
    // The bug this task exists for: numbering said one order, the chain said
    // another, so Km pred did not equal the previous row's ODO.
    let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
    let stamp = Utc::now();
    let mut long_leg = make_trip_at(date, 0, 0);
    long_leg.created_at = stamp;
    long_leg.distance_km = 377.0;
    long_leg.odometer = 17416.0;
    let mut short_leg = make_trip_at(date, 0, 0);
    short_leg.created_at = stamp;
    short_leg.distance_km = 202.0;
    short_leg.odometer = 17618.0;
    // Ids run opposite to the expected odometer order, so a dropped odometer
    // key falls through to id and fails the assertions below every run.
    long_leg.id = Uuid::from_u128(2);
    short_leg.id = Uuid::from_u128(1);

    // Hand them in the order that used to break it: short leg first.
    let trips = vec![short_leg.clone(), long_leg.clone()];
    let numbers = calculate_trip_numbers(&trips);
    let starts = calculate_odometer_start(&trips, 17039.0);

    assert_eq!(numbers.get(&long_leg.id.to_string()), Some(&1));
    assert_eq!(numbers.get(&short_leg.id.to_string()), Some(&2));
    assert_eq!(starts.get(&long_leg.id.to_string()), Some(&17039.0));
    assert_eq!(starts.get(&short_leg.id.to_string()), Some(&17416.0));
}

// ========================================================================
// Trip sort routing tests (task 80 task 2)
// ========================================================================

#[test]
fn test_grid_chain_reads_correctly_in_trip_number_order() {
    // Walking the rows in the order the grid displays them, every row's
    // start must equal the previous row's stored odometer. This held only
    // by accident before: numbering and the chain used different sorts.
    let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
    let stamp = Utc::now();
    let mut a = make_trip_at(date, 0, 0);
    a.created_at = stamp;
    a.distance_km = 377.0;
    a.odometer = 17416.0;
    let mut b = make_trip_at(date, 0, 0);
    b.created_at = stamp;
    b.distance_km = 202.0;
    b.odometer = 17618.0;
    let mut c = make_trip_at(date.succ_opt().unwrap(), 8, 0);
    c.created_at = stamp;
    c.distance_km = 168.0;
    c.odometer = 17786.0;

    let trips = vec![b.clone(), c.clone(), a.clone()];
    let numbers = calculate_trip_numbers(&trips);
    let starts = calculate_odometer_start(&trips, 17039.0);

    let mut ordered: Vec<_> = trips.iter().collect();
    ordered.sort_by_key(|t| numbers[&t.id.to_string()]);

    let mut prev = 17039.0;
    for trip in ordered {
        assert_eq!(
            starts[&trip.id.to_string()], prev,
            "row {} starts where the previous row ended",
            numbers[&trip.id.to_string()]
        );
        prev = trip.odometer;
    }
}

// ========================================================================
// Preview anchor tests (preview_anchor) - task 80 task 2
// ========================================================================

/// Three trips at one start_datetime and one created_at, separated only by
/// the odometer. Two details make the tests below discriminate:
///
/// * the group sits at 15:00, not at midnight, so a preview row pinned to
///   midnight sorts ahead of the whole group instead of inside it.
/// * created_at is a day old, so a preview row that keeps `Utc::now()` sorts
///   after the whole group instead of beside its anchor.
///
/// The ids run opposite to the odometers, so a pick that fell through to the
/// id key would return a different trip than the one asserted.
fn tied_preview_group() -> Vec<Trip> {
    let date = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
    let stamp = Utc::now() - chrono::Duration::days(1);
    let mut trips = Vec::new();
    for (index, odometer) in [69400.0, 69411.0, 69415.0].iter().enumerate() {
        let mut trip = make_trip_at(date, 15, 0);
        trip.created_at = stamp;
        trip.odometer = *odometer;
        trip.id = Uuid::from_u128(3 - index as u128);
        trips.push(trip);
    }
    trips
}

/// Build the virtual preview row the way `preview_trip_calculation_internal`
/// does: copy the anchor's start_datetime and created_at, and offset the
/// odometer.
fn virtual_preview_trip(trips: &[Trip], insert_at_trip_id: Option<&str>) -> Trip {
    let (anchor, offset) = preview_anchor(trips, insert_at_trip_id).expect("anchor exists");
    let mut preview = make_trip_at(anchor.start_datetime.date(), 0, 0);
    preview.id = Uuid::from_u128(999);
    preview.start_datetime = anchor.start_datetime;
    preview.created_at = anchor.created_at;
    preview.odometer = anchor.odometer + offset;
    preview
}

#[test]
fn test_preview_anchor_top_path_picks_the_last_trip_in_trip_order() {
    // A new row at the top anchors to the last row of the book in trip_order.
    // The extra row is what separates the two implementations: it is LATER on
    // the same date but carries a LOWER odometer than the tied group.
    //
    //   trip_order                            -> the 18:00 row, odometer 69408
    //   max_by_key((date, odometer as i64))   -> the 15:00 row, odometer 69415
    //
    // The second is what the preview command used before task 80. Without this
    // row both answers are the same trip and the test guards nothing.
    let mut trips = tied_preview_group();
    let mut later_but_lower = make_trip_at(NaiveDate::from_ymd_opt(2026, 8, 19).unwrap(), 18, 0);
    later_but_lower.created_at = trips[0].created_at;
    later_but_lower.odometer = 69408.0;
    later_but_lower.id = Uuid::from_u128(4);
    trips.push(later_but_lower);

    let (anchor, offset) = preview_anchor(&trips, None).expect("the group is not empty");

    assert_eq!(
        anchor.odometer, 69408.0,
        "the last row by datetime, not the highest odometer"
    );
    assert_eq!(offset, 0.5);
}

#[test]
fn test_preview_anchor_insert_path_picks_the_named_target() {
    let trips = tied_preview_group();
    let target_id = trips[1].id.to_string();

    let (anchor, offset) = preview_anchor(&trips, Some(target_id.as_str())).expect("target exists");

    assert_eq!(anchor.id, trips[1].id);
    assert_eq!(offset, -0.5);
}

#[test]
fn test_preview_anchor_has_no_anchor_without_trips_or_target() {
    // The caller keeps its own fallbacks for both of these.
    let trips = tied_preview_group();

    assert!(preview_anchor(&[], None).is_none());
    assert!(preview_anchor(&trips, Some("not-a-trip-id")).is_none());
}

#[test]
fn test_preview_row_lands_after_the_whole_book_on_the_top_path() {
    let mut trips = tied_preview_group();
    let preview = virtual_preview_trip(&trips, None);
    let preview_id = preview.id;
    trips.push(preview);

    trips.sort_by(|a, b| trip_order(a, b));

    let index = trips.iter().position(|t| t.id == preview_id).unwrap();
    assert_eq!(
        index, 3,
        "a new top row belongs after the tied group, not at the top of its day"
    );
}

#[test]
fn test_preview_row_lands_directly_above_its_insert_target() {
    let mut trips = tied_preview_group();
    let target_id = trips[1].id;
    let target = target_id.to_string();
    let preview = virtual_preview_trip(&trips, Some(target.as_str()));
    let preview_id = preview.id;
    trips.push(preview);

    trips.sort_by(|a, b| trip_order(a, b));

    let preview_index = trips.iter().position(|t| t.id == preview_id).unwrap();
    let target_index = trips.iter().position(|t| t.id == target_id).unwrap();
    assert_eq!(
        preview_index + 1,
        target_index,
        "the preview row belongs immediately above the row it is inserted at"
    );
}

/// Seed one trip into a real DB, with every field the preview placement reads.
fn seed_trip(
    db: &Database,
    vehicle_id: Uuid,
    start: NaiveDateTime,
    distance_km: f64,
    odometer: f64,
    fuel_liters: Option<f64>,
    created_at: chrono::DateTime<Utc>,
) -> Trip {
    let mut trip = make_trip_detailed(
        start.date(),
        distance_km,
        fuel_liters,
        fuel_liters.is_some(),
    );
    trip.vehicle_id = vehicle_id;
    trip.start_datetime = start;
    trip.odometer = odometer;
    trip.created_at = created_at;
    db.create_trip(&trip).unwrap();
    trip
}

#[test]
fn test_preview_command_places_the_row_beside_its_insert_target() {
    // This drives preview_trip_calculation_internal itself, not a copy of it.
    //
    // The book has two closed periods on one day and a tied pair between them:
    //
    //   09:00  100 km, 8.0 L full  -- closes period A
    //   12:00  100 km, no fuel     -- tied pair, one created_at
    //   12:00  100 km, 10.0 L full -- tied pair, closes period B
    //
    // Insert a 50 km preview row above the 10.0 L row. Where it lands decides
    // the rate the command reports, and each way of getting it wrong reports a
    // different number:
    //
    //   beside its anchor (correct) -> period B, 10.0 L / 250 km = 4.0
    //   created_at not copied       -> after the pair, open period, TP 5.1
    //   pinned to midnight          -> period A, 8.0 L / 150 km = 5.333
    let (db, vehicle) = setup_db_with_vehicle(); // tank 66.0 l, TP 5.1 l/100km
    let year = 2026;
    let day = NaiveDate::from_ymd_opt(year, 6, 1).unwrap();
    let older = Utc::now() - chrono::Duration::days(2);
    let tied_stamp = Utc::now() - chrono::Duration::days(1);

    // A full tank early in the year, so period A starts from a known state.
    seed_trip(
        &db,
        vehicle.id,
        NaiveDate::from_ymd_opt(year, 1, 5)
            .unwrap()
            .and_hms_opt(8, 0, 0)
            .unwrap(),
        100.0,
        1100.0,
        Some(20.0),
        older,
    );
    seed_trip(
        &db,
        vehicle.id,
        day.and_hms_opt(9, 0, 0).unwrap(),
        100.0,
        1200.0,
        Some(8.0),
        older,
    );
    seed_trip(
        &db,
        vehicle.id,
        day.and_hms_opt(12, 0, 0).unwrap(),
        100.0,
        1300.0,
        None,
        tied_stamp,
    );
    let fillup = seed_trip(
        &db,
        vehicle.id,
        day.and_hms_opt(12, 0, 0).unwrap(),
        100.0,
        1400.0,
        Some(10.0),
        tied_stamp,
    );

    let result = preview_trip_calculation_internal(
        &db,
        vehicle.id.to_string(),
        year,
        50.0,
        None,
        false,
        Some(fillup.id.to_string()),
        None,
    )
    .unwrap();

    assert!(
        !result.is_estimated_rate,
        "the preview row belongs in the period the fill-up below it closes, not in the open period after it"
    );
    assert!(
        (result.consumption_rate - 4.0).abs() < 0.0001,
        "expected 10.0 L over 250 km (100 + 50 preview + 100) = 4.0, got {}",
        result.consumption_rate
    );
}

// ========================================================================
// Per-period over-limit tests (has_any_period_over_limit)
// ========================================================================

#[test]
fn test_has_any_period_over_limit_single_period_over() {
    // Single period with rate > 120% of TP should return true
    let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let tp_rate = 5.0; // TP rate, limit is 6.0 (120%)

    let trips = vec![
        // Period: 100km, 7.5L filled = 7.5 l/100km > 6.0 limit
        make_trip_detailed(base_date, 100.0, Some(7.5), true),
    ];

    assert!(
        has_any_period_over_limit(&trips, tp_rate),
        "Period with rate 7.5 > limit 6.0 should trigger over-limit"
    );
}

#[test]
fn test_has_any_period_over_limit_average_ok_but_one_period_over() {
    // Two periods: one under, one over - average might be OK but should still trigger
    // This is the key test: average can be under 120% but individual period is over
    let tp_rate = 5.0; // limit is 6.0

    let trips = vec![
        // Period 1: 100km, 5L = 5.0 l/100km (under limit)
        make_trip_detailed(
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            100.0,
            Some(5.0),
            true,
        ),
        // Period 2: 100km, 7L = 7.0 l/100km (OVER limit)
        make_trip_detailed(
            NaiveDate::from_ymd_opt(2024, 6, 2).unwrap(),
            100.0,
            Some(7.0),
            true,
        ),
    ];
    // Average: (5+7) / 200 * 100 = 6.0 l/100km (exactly at limit, not over)
    // But Period 2 is 7.0 > 6.0, so should trigger

    assert!(
        has_any_period_over_limit(&trips, tp_rate),
        "Should trigger when ANY period is over limit, even if average is OK"
    );
}

#[test]
fn test_has_any_period_over_limit_all_periods_ok() {
    // All periods under limit should return false
    let tp_rate = 5.0; // limit is 6.0

    let trips = vec![
        // Period 1: 100km, 5L = 5.0 l/100km (under)
        make_trip_detailed(
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            100.0,
            Some(5.0),
            true,
        ),
        // Period 2: 100km, 5.5L = 5.5 l/100km (under)
        make_trip_detailed(
            NaiveDate::from_ymd_opt(2024, 6, 2).unwrap(),
            100.0,
            Some(5.5),
            true,
        ),
    ];

    assert!(
        !has_any_period_over_limit(&trips, tp_rate),
        "Should not trigger when all periods are under limit"
    );
}

#[test]
fn test_has_any_period_over_limit_at_exactly_limit() {
    // Period exactly at 120% should NOT trigger (limit is "greater than", not ">=")
    let tp_rate = 5.0; // limit is 6.0

    let trips = vec![
        // Period: 100km, 6L = 6.0 l/100km (exactly at limit)
        make_trip_detailed(
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            100.0,
            Some(6.0),
            true,
        ),
    ];

    assert!(
        !has_any_period_over_limit(&trips, tp_rate),
        "Period exactly at 120% limit should NOT trigger (not OVER)"
    );
}

// ========================================================================
// Fuel remaining tests (calculate_fuel_remaining)
// ========================================================================

#[test]
fn test_fuel_remaining_basic_trip() {
    // Start with 50L, drive 100km at 6 l/100km = 6L used, end with 44L
    let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let trips = vec![make_trip_detailed(base_date, 100.0, None, false)];

    let mut rates = std::collections::HashMap::new();
    rates.insert(trips[0].id.to_string(), 6.0);

    let remaining = calculate_fuel_remaining(&trips, &rates, 50.0, 66.0);

    let expected = 50.0 - 6.0; // 44L
    let actual = remaining.get(&trips[0].id.to_string()).unwrap();
    assert!(
        (actual - expected).abs() < 0.01,
        "Expected {:.1}L remaining, got {:.1}L",
        expected,
        actual
    );
}

#[test]
fn test_fuel_remaining_with_partial_fillup() {
    // Partial fillup adds fuel but doesn't fill tank
    let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let trips = vec![
        make_trip_detailed(base_date, 100.0, Some(30.0), false), // 100km, add 30L partial
    ];

    let mut rates = std::collections::HashMap::new();
    rates.insert(trips[0].id.to_string(), 6.0);

    let remaining = calculate_fuel_remaining(&trips, &rates, 20.0, 66.0);

    // Start: 20L, use 6L, add 30L = 44L
    let expected = 20.0 - 6.0 + 30.0; // 44L
    let actual = remaining.get(&trips[0].id.to_string()).unwrap();
    assert!(
        (actual - expected).abs() < 0.01,
        "Partial fillup: expected {:.1}L, got {:.1}L",
        expected,
        actual
    );
}

#[test]
fn test_fuel_remaining_with_full_fillup() {
    // Full tank fillup fills to tank_size regardless of fuel added
    let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let tank_size = 66.0;
    let trips = vec![
        make_trip_detailed(base_date, 100.0, Some(30.0), true), // Full tank
    ];

    let mut rates = std::collections::HashMap::new();
    rates.insert(trips[0].id.to_string(), 6.0);

    let remaining = calculate_fuel_remaining(&trips, &rates, 20.0, tank_size);

    // Full tank = always ends at tank_size
    let actual = remaining.get(&trips[0].id.to_string()).unwrap();
    assert!(
        (actual - tank_size).abs() < 0.01,
        "Full fillup should result in full tank ({:.1}L), got {:.1}L",
        tank_size,
        actual
    );
}

#[test]
fn test_fuel_remaining_clamps_to_zero() {
    // Can't go negative - clamps to 0
    let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let trips = vec![
        make_trip_detailed(base_date, 500.0, None, false), // 500km at 6 l/100km = 30L, but only have 10L
    ];

    let mut rates = std::collections::HashMap::new();
    rates.insert(trips[0].id.to_string(), 6.0);

    let remaining = calculate_fuel_remaining(&trips, &rates, 10.0, 66.0);

    let actual = remaining.get(&trips[0].id.to_string()).unwrap();
    assert!(
        *actual >= 0.0,
        "Fuel remaining should not go negative, got {:.1}L",
        actual
    );
    assert!(
        (actual - 0.0).abs() < 0.01,
        "Should clamp to 0, got {:.1}L",
        actual
    );
}

// ========================================================================
// Year carryover tests (get_year_start_fuel_remaining)
// ========================================================================

#[test]
fn test_year_start_fuel_no_previous_year_data() {
    // When no trips exist in the previous year, should return full tank
    let db = crate::db::Database::in_memory().expect("Failed to create database");

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        50.0, // tank_size
        6.0,  // tp_consumption
        0.0,
    );
    db.create_vehicle(&vehicle)
        .expect("Failed to create vehicle");

    // Query for 2025 with no 2024 data
    let result = get_year_start_fuel_remaining(
        &db,
        &vehicle.id.to_string(),
        2025,
        50.0, // tank_size
        6.0,  // tp_consumption
    );

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        50.0,
        "Should return full tank when no previous year data"
    );
}

#[test]
fn test_year_start_fuel_with_previous_year_full_tank() {
    // When previous year ends with full tank fillup, should return tank_size
    let db = crate::db::Database::in_memory().expect("Failed to create database");

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        50.0,
        6.0,
        0.0,
    );
    db.create_vehicle(&vehicle)
        .expect("Failed to create vehicle");

    let now = Utc::now();
    let date = NaiveDate::from_ymd_opt(2024, 12, 15).unwrap();
    let trip_2024 = Trip {
        id: Uuid::new_v4(),
        vehicle_id: vehicle.id,
        start_datetime: date.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: None,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: 100.0,
        odometer: 10000.0,
        purpose: "test".to_string(),
        fuel_liters: Some(6.0),
        fuel_cost_eur: Some(10.0),
        full_tank: true, // Full tank fillup -> ends at 50L
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: now,
        updated_at: now,
    };
    db.create_trip(&trip_2024).expect("Failed to create trip");

    let result = get_year_start_fuel_remaining(&db, &vehicle.id.to_string(), 2025, 50.0, 6.0);

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        50.0,
        "Full tank fillup should end at tank_size"
    );
}

#[test]
fn test_year_start_fuel_partial_tank_carryover() {
    // Test that partial tank fillups carry over correctly
    let db = crate::db::Database::in_memory().expect("Failed to create database");

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        50.0, // tank_size
        6.0,  // tp_consumption (6 l/100km)
        0.0,
    );
    db.create_vehicle(&vehicle)
        .expect("Failed to create vehicle");

    let now = Utc::now();

    // Trip 1: Drive 100km, full tank fillup with 6L
    // Starts at 50L (no prior year), uses 6L, ends at 50L (full tank)
    let date1 = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let trip1 = Trip {
        id: Uuid::new_v4(),
        vehicle_id: vehicle.id,
        start_datetime: date1.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: None,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: 100.0,
        odometer: 10000.0,
        purpose: "test".to_string(),
        fuel_liters: Some(6.0),
        fuel_cost_eur: Some(10.0),
        full_tank: true,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: now,
        updated_at: now,
    };

    // Trip 2: Drive 200km, partial fillup with 10L
    // Rate from trip1 is 6%, so uses 12L, starts at 50L, ends at 50-12+10=48L
    let date2 = NaiveDate::from_ymd_opt(2024, 12, 20).unwrap();
    let trip2 = Trip {
        id: Uuid::new_v4(),
        vehicle_id: vehicle.id,
        start_datetime: date2.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: None,
        origin: "B".to_string(),
        destination: "C".to_string(),
        distance_km: 200.0,
        odometer: 10200.0,
        purpose: "test".to_string(),
        fuel_liters: Some(10.0),
        fuel_cost_eur: Some(16.0),
        full_tank: false, // Partial fillup
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: now,
        updated_at: now,
    };

    db.create_trip(&trip1).expect("Failed to create trip1");
    db.create_trip(&trip2).expect("Failed to create trip2");

    let result = get_year_start_fuel_remaining(&db, &vehicle.id.to_string(), 2025, 50.0, 6.0);

    assert!(result.is_ok());
    // After trip1: full tank (50L)
    // Trip2 uses 12L at 6% rate, adds 10L partial = 50 - 12 + 10 = 48L
    let fuel = result.unwrap();
    assert!((fuel - 48.0).abs() < 0.1, "Expected ~48L, got {}", fuel);
}

// ========================================================================
// Year odometer carryover tests (get_year_start_odometer)
// ========================================================================

#[test]
fn test_year_start_odometer_no_previous_year_data() {
    // When no trips exist in the previous year, should return vehicle's initial odometer
    let db = crate::db::Database::in_memory().expect("Failed to create database");

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        50.0,
        6.0,
        38057.0, // initial_odometer
    );
    db.create_vehicle(&vehicle)
        .expect("Failed to create vehicle");

    // Query for 2025 with no 2024 data
    let result = get_year_start_odometer(
        &db,
        &vehicle.id.to_string(),
        2025,
        38057.0, // initial_odometer
    );

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        38057.0,
        "Should return initial odometer when no previous year data"
    );
}

#[test]
fn test_year_start_odometer_with_previous_year_trips() {
    // When previous year has trips, should return the last trip's odometer
    let db = crate::db::Database::in_memory().expect("Failed to create database");

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        50.0,
        6.0,
        38057.0, // initial_odometer
    );
    db.create_vehicle(&vehicle)
        .expect("Failed to create vehicle");

    let now = Utc::now();

    // Trip in 2024 ending at 54914 km (like the bug scenario)
    let date = NaiveDate::from_ymd_opt(2024, 12, 13).unwrap();
    let trip_2024 = Trip {
        id: Uuid::new_v4(),
        vehicle_id: vehicle.id,
        start_datetime: date.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: None,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: 370.0,
        odometer: 54914.0, // This is the ending odometer
        purpose: "test".to_string(),
        fuel_liters: Some(24.0),
        fuel_cost_eur: Some(35.0),
        full_tank: true,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: now,
        updated_at: now,
    };
    db.create_trip(&trip_2024).expect("Failed to create trip");

    // Query for 2025 should return 54914 (last trip's odometer from 2024)
    let result = get_year_start_odometer(
        &db,
        &vehicle.id.to_string(),
        2025,
        38057.0, // initial_odometer (should be ignored)
    );

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        54914.0,
        "Should return last trip's odometer from previous year"
    );
}

#[test]
fn test_year_start_odometer_multiple_trips_returns_last() {
    // With multiple trips in previous year, should return the chronologically last one
    let db = crate::db::Database::in_memory().expect("Failed to create database");

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        50.0,
        6.0,
        10000.0,
    );
    db.create_vehicle(&vehicle)
        .expect("Failed to create vehicle");

    let now = Utc::now();

    // First trip (earlier date)
    let date1 = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let trip1 = Trip {
        id: Uuid::new_v4(),
        vehicle_id: vehicle.id,
        start_datetime: date1.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: None,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: 100.0,
        odometer: 10100.0,
        purpose: "test".to_string(),
        fuel_liters: Some(6.0),
        fuel_cost_eur: Some(10.0),
        full_tank: true,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: now,
        updated_at: now,
    };

    // Last trip (later date, higher odometer)
    let date2 = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    let trip2 = Trip {
        id: Uuid::new_v4(),
        vehicle_id: vehicle.id,
        start_datetime: date2.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: None,
        origin: "B".to_string(),
        destination: "C".to_string(),
        distance_km: 200.0,
        odometer: 20000.0, // This is the last odometer
        purpose: "test".to_string(),
        fuel_liters: Some(12.0),
        fuel_cost_eur: Some(20.0),
        full_tank: true,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: now,
        updated_at: now,
    };

    db.create_trip(&trip1).expect("Failed to create trip1");
    db.create_trip(&trip2).expect("Failed to create trip2");

    let result = get_year_start_odometer(&db, &vehicle.id.to_string(), 2025, 10000.0);

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        20000.0,
        "Should return the last trip's odometer by date"
    );
}

// ========================================================================
// BEV energy calculation tests (calculate_energy_grid_data)
// ========================================================================

/// Helper to create a BEV trip with energy data
fn make_bev_trip(
    date: NaiveDate,
    distance_km: f64,
    energy_kwh: Option<f64>,
    full_charge: bool,
) -> Trip {
    let now = Utc::now();
    Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        start_datetime: date.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: None,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km,
        odometer: 10000.0,
        purpose: "business".to_string(),
        fuel_liters: None,
        fuel_cost_eur: None,
        full_tank: false,
        energy_kwh,
        energy_cost_eur: energy_kwh.map(|e| e * 0.30), // ~0.30€/kWh
        full_charge,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn test_bev_energy_calculation_single_trip() {
    // BEV with 75 kWh battery, 18 kWh/100km baseline
    let vehicle = Vehicle::new_bev(
        "Test BEV".to_string(),
        "BEV-001".to_string(),
        75.0, // battery capacity
        18.0, // baseline consumption
        10000.0,
        Some(100.0), // Start at 100% = 75 kWh
    );

    let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let trips = vec![make_bev_trip(date, 100.0, None, false)];

    // Initial battery: 100% of 75 kWh = 75 kWh
    let initial_battery = 75.0;
    let (_energy_rates, estimated_rates, battery_kwh, _battery_percent) =
        calculate_energy_grid_data(&trips, &vehicle, initial_battery);

    // Trip 100km at 18 kWh/100km = 18 kWh used
    // Start at 75 kWh, end at 75 - 18 = 57 kWh
    let remaining = battery_kwh.get(&trips[0].id.to_string()).unwrap();
    assert!(
        (remaining - 57.0).abs() < 0.1,
        "Expected 57 kWh remaining, got {}",
        remaining
    );

    // Should use baseline rate (estimated)
    assert!(estimated_rates.contains(&trips[0].id.to_string()));
}

#[test]
fn test_bev_energy_with_charge() {
    let vehicle = Vehicle::new_bev(
        "Test BEV".to_string(),
        "BEV-001".to_string(),
        75.0,
        18.0,
        10000.0,
        Some(50.0), // Start at 50% = 37.5 kWh
    );

    let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let trips = vec![
        make_bev_trip(date, 100.0, Some(40.0), true), // Drive 100km, charge 40 kWh full
    ];

    // Initial battery: 50% of 75 kWh = 37.5 kWh
    let initial_battery = 37.5;
    let (_energy_rates, estimated_rates, battery_kwh, _) =
        calculate_energy_grid_data(&trips, &vehicle, initial_battery);

    // Start: 37.5 kWh (50%)
    // Drive 100km at 18 kWh/100km = 18 kWh used
    // Add charge: 37.5 - 18 + 40 = 59.5 kWh
    // (Charge happens during trip via calculate_battery_remaining)
    let remaining = battery_kwh.get(&trips[0].id.to_string()).unwrap();
    assert!(
        (remaining - 59.5).abs() < 0.1,
        "Expected ~59.5 kWh remaining, got {}",
        remaining
    );

    // With full charge, should have calculated rate (not estimated)
    assert!(!estimated_rates.contains(&trips[0].id.to_string()));
}

#[test]
fn test_bev_battery_clamps_to_capacity() {
    let vehicle = Vehicle::new_bev(
        "Test BEV".to_string(),
        "BEV-001".to_string(),
        75.0,
        18.0,
        10000.0,
        Some(90.0), // Start at 90% = 67.5 kWh
    );

    let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let trips = vec![
        make_bev_trip(date, 10.0, Some(50.0), true), // Short drive, big charge
    ];

    // Initial battery: 90% of 75 kWh = 67.5 kWh
    let initial_battery = 67.5;
    let (_, _, battery_kwh, _) = calculate_energy_grid_data(&trips, &vehicle, initial_battery);

    // Should be capped at capacity (75 kWh)
    let remaining = battery_kwh.get(&trips[0].id.to_string()).unwrap();
    assert!(
        *remaining <= 75.0,
        "Battery should not exceed capacity, got {}",
        remaining
    );
}

// ========================================================================
/// Helper to create a trip for assignment tests (with vehicle_id that stays constant)
/// Sets end_datetime to end of day to allow invoice matching for any time during the day
fn make_trip_for_assignment(
    vehicle_id: Uuid,
    date: NaiveDate,
    fuel_liters: Option<f64>,
    fuel_cost_eur: Option<f64>,
    other_costs_eur: Option<f64>,
) -> Trip {
    let now = Utc::now();
    Trip {
        id: Uuid::new_v4(),
        vehicle_id,
        start_datetime: date.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: Some(date.and_hms_opt(23, 59, 59).unwrap()),
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: 100.0,
        odometer: 10000.0,
        purpose: "business".to_string(),
        fuel_liters,
        fuel_cost_eur,
        full_tank: fuel_liters.is_some(),
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur,
        other_costs_note: None,
        created_at: now,
        updated_at: now,
    }
}

// ========================================================================
// Task 66: multi-invoice assignment semantics
// (1 Fuel + N Other per trip, cent-exact sum-on-assign/unassign)
// ========================================================================

/// In-memory DB with one vehicle (shared setup for Task 66 tests).
fn setup_db_with_vehicle() -> (Database, Vehicle) {
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Test Car".to_string(), "BA123XY".to_string(), 66.0, 5.1, 0.0);
    db.create_vehicle(&vehicle).unwrap();
    (db, vehicle)
}

fn make_paperless_doc(
    id: i64,
    title: &str,
    total_amount: Option<f64>,
    litres: Option<f64>,
) -> crate::paperless::PaperlessDoc {
    crate::paperless::PaperlessDoc {
        id,
        title: title.to_string(),
        tag_ids: vec![],
        created: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
        total_amount,
        litres,
        receipt_datetime: None,
    }
}

fn assign_paperless(
    db: &Database,
    doc: &crate::paperless::PaperlessDoc,
    trip_id: &str,
    vehicle_id: &str,
    assignment_type: crate::models::AssignmentType,
) -> Result<(), String> {
    let app_state = crate::app_state::AppState::new();
    assign_paperless_invoice_internal(
        db,
        &app_state,
        doc,
        trip_id,
        vehicle_id,
        assignment_type,
        false,
    )
}

fn unassign_paperless(db: &Database, doc_id: i64) -> Result<(), String> {
    let app_state = crate::app_state::AppState::new();
    unassign_paperless_invoice_internal(db, &app_state, doc_id)
}

fn get_trip_costs(db: &Database, trip_id: &Uuid) -> Option<f64> {
    db.get_trip(&trip_id.to_string())
        .unwrap()
        .unwrap()
        .other_costs_eur
}

#[test]
fn test_second_fuel_link_same_trip_rejected() {
    // Paperless-only coverage: a trip can hold at most ONE Fuel invoice.
    let (db, vehicle) = setup_db_with_vehicle();
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trip = make_trip_for_assignment(vehicle.id, date, Some(45.0), Some(72.0), None);
    db.create_trip(&trip).unwrap();

    let first = make_paperless_doc(801, "Tankovanie", Some(72.0), Some(45.0));
    assign_paperless(
        &db,
        &first,
        &trip.id.to_string(),
        &vehicle.id.to_string(),
        AssignmentType::Fuel,
    )
    .unwrap();

    let second = make_paperless_doc(802, "Tankovanie 2", Some(60.0), Some(40.0));
    let err = assign_paperless(
        &db,
        &second,
        &trip.id.to_string(),
        &vehicle.id.to_string(),
        AssignmentType::Fuel,
    )
    .unwrap_err();
    assert!(
        err.contains("fuel invoice"),
        "expected clear fuel-uniqueness error, got: {}",
        err
    );

    assert!(db.get_paperless_link(801).unwrap().is_some(), "first link intact");
    assert!(db.get_paperless_link(802).unwrap().is_none(), "second link not created");
}

#[test]
fn test_paperless_reupsert_same_trip_does_not_double_add() {
    // I12, paperless path
    let (db, vehicle) = setup_db_with_vehicle();
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trip = make_trip_for_assignment(vehicle.id, date, None, None, Some(10.0));
    db.create_trip(&trip).unwrap();

    let doc = make_paperless_doc(701, "Parkovné", Some(5.01), None);
    let trip_id = trip.id.to_string();
    let vehicle_id = vehicle.id.to_string();
    assign_paperless(&db, &doc, &trip_id, &vehicle_id, crate::models::AssignmentType::Other)
        .unwrap();
    assert_eq!(get_trip_costs(&db, &trip.id), Some(15.01));

    assign_paperless(&db, &doc, &trip_id, &vehicle_id, crate::models::AssignmentType::Other)
        .unwrap();
    assert_eq!(get_trip_costs(&db, &trip.id), Some(15.01), "no double add");
    let link = db.get_paperless_link(701).unwrap().unwrap();
    assert_eq!(link.applied_amount_cents, Some(501), "snapshot unchanged");
}

#[test]
fn test_paperless_assign_stores_snapshots() {
    // assign paperless Other -> link row has assignment_type/amount_eur/title
    // from backend-fetched doc + applied_amount_cents set when applied
    let (db, vehicle) = setup_db_with_vehicle();
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trip = make_trip_for_assignment(vehicle.id, date, None, None, None);
    db.create_trip(&trip).unwrap();

    let doc = make_paperless_doc(702, "Parkovné", Some(12.50), None);
    assign_paperless(
        &db,
        &doc,
        &trip.id.to_string(),
        &vehicle.id.to_string(),
        crate::models::AssignmentType::Other,
    )
    .unwrap();

    let link = db.get_paperless_link(702).unwrap().unwrap();
    assert_eq!(link.trip_id, trip.id.to_string());
    assert_eq!(link.assignment_type, crate::models::AssignmentType::Other);
    assert_eq!(link.amount_eur, Some(12.50));
    assert_eq!(link.title, Some("Parkovné".to_string()));
    assert_eq!(link.applied_amount_cents, Some(1250), "amount was applied");
    assert_eq!(get_trip_costs(&db, &trip.id), Some(12.50));
}

#[test]
fn test_paperless_reassign_reverses_old_trip_sum() {
    // Other doc applied to trip A, reassign to trip B -> A restored exactly, B increased
    let (db, vehicle) = setup_db_with_vehicle();
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trip_a = make_trip_for_assignment(vehicle.id, date, None, None, Some(10.0));
    let trip_b = make_trip_for_assignment(vehicle.id, date, None, None, Some(20.0));
    db.create_trip(&trip_a).unwrap();
    db.create_trip(&trip_b).unwrap();

    let doc = make_paperless_doc(703, "Parkovné", Some(5.01), None);
    let vehicle_id = vehicle.id.to_string();
    assign_paperless(&db, &doc, &trip_a.id.to_string(), &vehicle_id, crate::models::AssignmentType::Other)
        .unwrap();
    assert_eq!(get_trip_costs(&db, &trip_a.id), Some(15.01));

    assign_paperless(&db, &doc, &trip_b.id.to_string(), &vehicle_id, crate::models::AssignmentType::Other)
        .unwrap();
    assert_eq!(get_trip_costs(&db, &trip_a.id), Some(10.0), "A restored bit-exact");
    assert_eq!(get_trip_costs(&db, &trip_b.id), Some(25.01), "B increased");
    let link = db.get_paperless_link(703).unwrap().unwrap();
    assert_eq!(link.trip_id, trip_b.id.to_string());
    assert_eq!(link.applied_amount_cents, Some(501));
}

#[test]
fn test_paperless_reassign_link_only_does_not_touch_old_trip() {
    // doc linked via double-count guard (applied None), reassign -> A's total untouched (I4)
    let (db, vehicle) = setup_db_with_vehicle();
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trip_a = make_trip_for_assignment(vehicle.id, date, None, None, Some(15.0));
    let trip_b = make_trip_for_assignment(vehicle.id, date, None, None, None);
    db.create_trip(&trip_a).unwrap();
    db.create_trip(&trip_b).unwrap();

    let doc = make_paperless_doc(704, "Parkovné", Some(15.0), None);
    let vehicle_id = vehicle.id.to_string();
    assign_paperless(&db, &doc, &trip_a.id.to_string(), &vehicle_id, crate::models::AssignmentType::Other)
        .unwrap();
    let link = db.get_paperless_link(704).unwrap().unwrap();
    assert_eq!(link.applied_amount_cents, None, "guard -> link-only");
    assert_eq!(get_trip_costs(&db, &trip_a.id), Some(15.0));

    assign_paperless(&db, &doc, &trip_b.id.to_string(), &vehicle_id, crate::models::AssignmentType::Other)
        .unwrap();
    assert_eq!(
        get_trip_costs(&db, &trip_a.id),
        Some(15.0),
        "link-only reassign must not mutate the old trip"
    );
    assert_eq!(get_trip_costs(&db, &trip_b.id), Some(15.0), "populated on B");
}

#[test]
fn test_paperless_unassign_subtracts_applied_amount() {
    // Rule 5 on the paperless path: unassign subtracts the applied snapshot
    // and deletes the link; the trip total is restored bit-exact.
    let (db, vehicle) = setup_db_with_vehicle();
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trip = make_trip_for_assignment(vehicle.id, date, None, None, Some(10.0));
    db.create_trip(&trip).unwrap();

    let doc = make_paperless_doc(707, "Parkovné", Some(5.01), None);
    assign_paperless(
        &db,
        &doc,
        &trip.id.to_string(),
        &vehicle.id.to_string(),
        crate::models::AssignmentType::Other,
    )
    .unwrap();
    assert_eq!(get_trip_costs(&db, &trip.id), Some(15.01));

    unassign_paperless(&db, 707).unwrap();
    assert_eq!(get_trip_costs(&db, &trip.id), Some(10.0), "restored bit-exact");
    assert!(db.get_paperless_link(707).unwrap().is_none(), "link deleted");
}

#[test]
fn test_assign_rejects_non_finite_or_negative_amount() {
    // invoice amount NaN / -5.0 -> clear error, nothing linked, nothing added
    // (to_cents(NAN) == 0 silently — validate at the boundary instead)
    let (db, vehicle) = setup_db_with_vehicle();
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trip = make_trip_for_assignment(vehicle.id, date, None, None, Some(10.0));
    db.create_trip(&trip).unwrap();

    // Paperless docs come from the server: validate NaN and negative too.
    let vehicle_id = vehicle.id.to_string();
    let trip_id = trip.id.to_string();
    let nan_doc = make_paperless_doc(705, "NaN", Some(f64::NAN), None);
    let err = assign_paperless(&db, &nan_doc, &trip_id, &vehicle_id, crate::models::AssignmentType::Other)
        .unwrap_err();
    assert!(err.to_lowercase().contains("amount"), "got: {}", err);
    assert!(db.get_paperless_link(705).unwrap().is_none());

    let neg_doc = make_paperless_doc(706, "Neg", Some(-1.0), None);
    let err = assign_paperless(&db, &neg_doc, &trip_id, &vehicle_id, crate::models::AssignmentType::Other)
        .unwrap_err();
    assert!(err.to_lowercase().contains("amount"), "got: {}", err);
    assert!(db.get_paperless_link(706).unwrap().is_none());
    assert_eq!(get_trip_costs(&db, &trip.id), Some(10.0), "nothing added");
}

// ========================================================================
// ========================================================================
// Magic fill tests (get_open_period_km)
// ========================================================================

/// Helper to create a trip with specific km and fuel
fn make_trip_for_magic_fill(
    date: NaiveDate,
    distance_km: f64,
    fuel_liters: Option<f64>,
    full_tank: bool,
) -> Trip {
    let now = Utc::now();
    Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        start_datetime: date.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: None,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km,
        odometer: 10000.0,
        purpose: "business".to_string(),
        fuel_liters,
        fuel_cost_eur: fuel_liters.map(|l| l * 1.5),
        full_tank,
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

#[test]
fn test_open_period_km_empty_trips() {
    let trips: Vec<Trip> = vec![];
    assert_eq!(get_open_period_km(&trips, None), 0.0);
}

#[test]
fn test_open_period_km_single_trip_no_fuel() {
    let trips = vec![make_trip_for_magic_fill(
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        100.0,
        None,
        false,
    )];
    assert_eq!(get_open_period_km(&trips, None), 100.0);
}

#[test]
fn test_open_period_km_single_trip_with_full_tank() {
    // Full tank fillup closes the period - open km should be 0
    let trips = vec![make_trip_for_magic_fill(
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        100.0,
        Some(5.0),
        true, // full tank
    )];
    assert_eq!(get_open_period_km(&trips, None), 0.0);
}

#[test]
fn test_open_period_km_multiple_trips_last_full_tank() {
    // Two trips, last one is full tank - should return 0
    let trips = vec![
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            50.0,
            None,
            false,
        ),
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            100.0,
            Some(8.0),
            true, // full tank - closes period
        ),
    ];
    assert_eq!(get_open_period_km(&trips, None), 0.0);
}

#[test]
fn test_open_period_km_multiple_trips_open_period() {
    // Three trips: full tank, then two without - open period = last two
    let trips = vec![
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            100.0,
            Some(6.0),
            true, // full tank - closes period
        ),
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            50.0,
            None,
            false,
        ),
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
            75.0,
            None,
            false,
        ),
    ];
    // Open period: 50 + 75 = 125 km
    assert_eq!(get_open_period_km(&trips, None), 125.0);
}

#[test]
fn test_open_period_km_partial_fillup_doesnt_close() {
    // Partial fillup (full_tank=false) should NOT close the period
    let trips = vec![
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            100.0,
            Some(6.0),
            true, // full tank - closes period
        ),
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            50.0,
            Some(3.0), // partial fillup
            false,     // NOT full tank
        ),
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
            75.0,
            None,
            false,
        ),
    ];
    // Open period: 50 + 75 = 125 km (partial fillup doesn't close)
    assert_eq!(get_open_period_km(&trips, None), 125.0);
}

#[test]
fn test_magic_fill_calculation() {
    // Verify the formula: liters = total_km * target_rate / 100
    // For 100 km at 5.5 l/100km = 5.5 liters
    let tp_rate: f64 = 5.0;
    let total_km: f64 = 100.0;
    let multiplier: f64 = 1.10; // 110% of TP
    let target_rate = tp_rate * multiplier;
    let expected_liters = total_km * target_rate / 100.0;
    assert!((expected_liters - 5.5).abs() < 0.01);
}

#[test]
fn test_magic_fill_existing_trip_no_double_count() {
    // Scenario: User has trips in open period, edits an existing trip
    // The existing trip's km should NOT be double-counted
    //
    // Trips: [full_tank 100km] -> [50km] -> [75km] -> [370km editing]
    // Open period after full tank: 50 + 75 + 370 = 495 km
    // For existing trip: total_km = 495 (NOT 495 + 370 = 865)
    // For new trip with 370km: total_km = 495 + 370 = 865
    //
    // With TP=5.1, target=110% (5.61 l/100km):
    // - Existing trip: 495 * 5.61 / 100 = 27.77 L
    // - New trip: 865 * 5.61 / 100 = 48.53 L (much higher - wrong if used for existing!)

    let trips = vec![
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            100.0,
            Some(6.0),
            true, // full tank - closes period
        ),
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            50.0,
            None,
            false,
        ),
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
            75.0,
            None,
            false,
        ),
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 4).unwrap(),
            370.0,
            None,
            false,
        ),
    ];

    let open_km = get_open_period_km(&trips, None);
    // Open period: 50 + 75 + 370 = 495 km
    assert_eq!(open_km, 495.0);

    // For existing trip (editing_trip_id = Some), total = open_km = 495
    // For new trip (editing_trip_id = None), total = open_km + current_km = 495 + 370 = 865
    // The command handles this distinction via editing_trip_id parameter
}

#[test]
fn test_open_period_km_editing_trip_in_middle() {
    // BUG FIX: When editing a trip in the MIDDLE of an open period,
    // we should only count km up to that trip, not trips that come after.
    //
    // Scenario:
    // Trip A: 100km (full tank) - closes previous period
    // Trip B: 50km (no fuel)
    // Trip C: 75km (no fuel) <- EDITING THIS ONE
    // Trip D: 200km (no fuel)
    // Trip E: 150km (no fuel)
    //
    // When editing Trip C, open period should be: B + C = 50 + 75 = 125 km
    // NOT: B + C + D + E = 50 + 75 + 200 + 150 = 475 km

    let trip_c_id = Uuid::new_v4();
    let trips = vec![
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            100.0,
            Some(50.0),
            true, // full tank - closes period
        ),
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
            50.0,
            None,
            false,
        ),
        {
            // Trip C - the one being edited
            let mut trip = make_trip_for_magic_fill(
                NaiveDate::from_ymd_opt(2026, 1, 10).unwrap(),
                75.0,
                None,
                false,
            );
            trip.id = trip_c_id;
            trip
        },
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            200.0,
            None,
            false,
        ),
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            150.0,
            None,
            false,
        ),
    ];

    // Without stop_at: returns all km in open period = 50 + 75 + 200 + 150 = 475
    let all_open_km = get_open_period_km(&trips, None);
    assert_eq!(all_open_km, 475.0);

    // With stop_at Trip C: should return only 50 + 75 = 125
    let km_up_to_c = get_open_period_km(&trips, Some(&trip_c_id));
    assert_eq!(
        km_up_to_c, 125.0,
        "Should only count km up to the edited trip"
    );
}

// ============================================================================
// Fuel Consumed Tests
// ============================================================================

#[test]
fn test_fuel_consumed_basic() {
    // Trip: 100 km at 6.0 l/100km = 6.0 L consumed
    let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    let trip = Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        start_datetime: date.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: None,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: 100.0,
        odometer: 10100.0,
        purpose: "business".to_string(),
        fuel_liters: Some(6.0),
        fuel_cost_eur: Some(10.0),
        full_tank: true,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let trips = vec![trip.clone()];
    let mut rates = HashMap::new();
    rates.insert(trip.id.to_string(), 6.0); // 6.0 l/100km

    let consumed = calculate_fuel_consumed(&trips, &rates);

    assert_eq!(consumed.len(), 1);
    let fuel = consumed.get(&trip.id.to_string()).unwrap();
    assert!((fuel - 6.0).abs() < 0.01, "100 km at 6.0 l/100km = 6.0 L");
}

#[test]
fn test_fuel_consumed_uses_period_rate() {
    // Two trips in a closed period: 150km + 100km = 250km total, 15L fuel
    // Period rate = 15/250*100 = 6.0 l/100km
    // Trip 1 (150km): consumes 150 * 6.0 / 100 = 9.0 L
    // Trip 2 (100km): consumes 100 * 6.0 / 100 = 6.0 L
    let trip1_id = Uuid::new_v4();
    let trip2_id = Uuid::new_v4();
    let vehicle_id = Uuid::new_v4();
    let date1 = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

    let trips = vec![
        Trip {
            id: trip1_id,
            vehicle_id,
            start_datetime: date1.and_hms_opt(8, 0, 0).unwrap(),
            end_datetime: None,
            origin: "A".to_string(),
            destination: "B".to_string(),
            distance_km: 150.0,
            odometer: 10150.0,
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        Trip {
            id: trip2_id,
            vehicle_id,
            start_datetime: date2.and_hms_opt(8, 0, 0).unwrap(),
            end_datetime: None,
            origin: "B".to_string(),
            destination: "C".to_string(),
            distance_km: 100.0,
            odometer: 10250.0,
            purpose: "business".to_string(),
            fuel_liters: Some(15.0),
            fuel_cost_eur: Some(25.0),
            full_tank: true, // Closes the period
            energy_kwh: None,
            energy_cost_eur: None,
            full_charge: false,
            soc_override_percent: None,
            other_costs_eur: None,
            other_costs_note: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    ];

    // Both trips get period rate of 6.0 l/100km
    let mut rates = HashMap::new();
    rates.insert(trip1_id.to_string(), 6.0);
    rates.insert(trip2_id.to_string(), 6.0);

    let consumed = calculate_fuel_consumed(&trips, &rates);

    let trip1_consumed = consumed.get(&trip1_id.to_string()).unwrap();
    let trip2_consumed = consumed.get(&trip2_id.to_string()).unwrap();

    assert!(
        (trip1_consumed - 9.0).abs() < 0.01,
        "150 km at 6.0 l/100km = 9.0 L"
    );
    assert!(
        (trip2_consumed - 6.0).abs() < 0.01,
        "100 km at 6.0 l/100km = 6.0 L"
    );
}

#[test]
fn test_fuel_consumed_uses_tp_rate_for_open_period() {
    // Trip in open period uses TP rate (e.g., 5.5 l/100km)
    // Trip: 200 km at 5.5 l/100km = 11.0 L consumed
    let trip_id = Uuid::new_v4();
    let date = NaiveDate::from_ymd_opt(2024, 1, 20).unwrap();

    let trip = Trip {
        id: trip_id,
        vehicle_id: Uuid::new_v4(),
        start_datetime: date.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: None,
        origin: "X".to_string(),
        destination: "Y".to_string(),
        distance_km: 200.0,
        odometer: 10200.0,
        purpose: "business".to_string(),
        fuel_liters: None, // No fill-up, open period
        fuel_cost_eur: None,
        full_tank: false,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let trips = vec![trip];
    let mut rates = HashMap::new();
    rates.insert(trip_id.to_string(), 5.5); // TP rate (estimated)

    let consumed = calculate_fuel_consumed(&trips, &rates);

    let fuel = consumed.get(&trip_id.to_string()).unwrap();
    assert!((fuel - 11.0).abs() < 0.01, "200 km at 5.5 l/100km = 11.0 L");
}

#[test]
fn test_fuel_consumed_zero_distance() {
    // Trip with 0 km = 0 L consumed (edge case)
    let trip_id = Uuid::new_v4();
    let date = NaiveDate::from_ymd_opt(2024, 1, 25).unwrap();

    let trip = Trip {
        id: trip_id,
        vehicle_id: Uuid::new_v4(),
        start_datetime: date.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: None,
        origin: "Home".to_string(),
        destination: "Home".to_string(),
        distance_km: 0.0, // Zero distance
        odometer: 10000.0,
        purpose: "refuel only".to_string(),
        fuel_liters: Some(50.0),
        fuel_cost_eur: Some(80.0),
        full_tank: true,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let trips = vec![trip];
    let mut rates = HashMap::new();
    rates.insert(trip_id.to_string(), 6.0); // Rate doesn't matter for 0 km

    let consumed = calculate_fuel_consumed(&trips, &rates);

    let fuel = consumed.get(&trip_id.to_string()).unwrap();
    assert!((fuel - 0.0).abs() < 0.01, "0 km = 0 L consumed");
}

// ============================================================================
// Suggested Fillup Tests
// ============================================================================

#[test]
fn test_suggested_fillup_open_period() {
    // Trips in an open period (no full tank) should get suggestions
    let trips = vec![
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            100.0,
            None, // no fuel
            false,
        ),
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            150.0,
            None, // no fuel
            false,
        ),
    ];

    let tp_consumption = 6.0; // 6 l/100km
    let (suggestions, _legend) = calculate_suggested_fillups(&trips, tp_consumption);

    // Both trips should have suggestions
    assert_eq!(suggestions.len(), 2);

    // First trip: 100 km
    let first = suggestions.get(&trips[0].id.to_string()).unwrap();
    assert!(first.liters > 0.0);
    // Liters should be in range: 100km * 6.0 * 1.05/100 = 6.3 to 100 * 6.0 * 1.20/100 = 7.2
    assert!(first.liters >= 6.3 && first.liters <= 7.2);

    // Second trip: 100 + 150 = 250 km cumulative
    let second = suggestions.get(&trips[1].id.to_string()).unwrap();
    assert!(second.liters > first.liters); // Cumulative, so more liters
                                           // Liters: 250km * 6.0 * 1.05/100 = 15.75 to 250 * 6.0 * 1.20/100 = 18.0
    assert!(second.liters >= 15.75 && second.liters <= 18.0);
}

#[test]
fn test_suggested_fillup_closed_period_no_suggestions() {
    // Trip with full tank closes the period - only trip after gets suggestion
    let trips = vec![
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            100.0,
            Some(8.0), // full tank
            true,
        ),
        make_trip_for_magic_fill(
            NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            150.0,
            None, // no fuel - open period starts here
            false,
        ),
    ];

    let tp_consumption = 6.0;
    let (suggestions, _legend) = calculate_suggested_fillups(&trips, tp_consumption);

    // Only second trip should have suggestion (first closed the period)
    assert_eq!(suggestions.len(), 1);
    assert!(suggestions.contains_key(&trips[1].id.to_string()));
    assert!(!suggestions.contains_key(&trips[0].id.to_string()));
}

#[test]
fn test_suggested_fillup_consumption_rate_calculation() {
    // Verify the consumption rate is calculated correctly
    let trips = vec![make_trip_for_magic_fill(
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        200.0, // 200 km
        None,
        false,
    )];

    let tp_consumption = 5.0; // 5 l/100km
    let (suggestions, _legend) = calculate_suggested_fillups(&trips, tp_consumption);

    let suggestion = suggestions.get(&trips[0].id.to_string()).unwrap();
    // consumption_rate = liters / km * 100
    let expected_rate = (suggestion.liters / 200.0) * 100.0;
    let expected_rate_rounded = (expected_rate * 100.0).round() / 100.0;
    assert!((suggestion.consumption_rate - expected_rate_rounded).abs() < 0.01);
}

#[test]
fn test_suggested_fillup_empty_trips() {
    let trips: Vec<Trip> = vec![];
    let (suggestions, legend) = calculate_suggested_fillups(&trips, 6.0);
    assert!(suggestions.is_empty());
    assert!(legend.is_none());
}

#[test]
fn test_legend_suggested_fillup_returns_most_recent() {
    // Legend should return the suggestion for the MOST RECENT trip (latest start_datetime).
    // This is the trip that would close the open period.
    let trip1 = make_trip_for_magic_fill(
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        100.0, // older trip
        None,
        false,
    );

    let trip2 = make_trip_for_magic_fill(
        NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        150.0, // newer trip
        None,
        false,
    );

    let trip3 = make_trip_for_magic_fill(
        NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
        200.0, // most recent trip
        None,
        false,
    );

    // Chronological order for calculation (by date)
    let trips = vec![trip1.clone(), trip2.clone(), trip3.clone()];

    let tp_consumption = 6.0;
    let (suggestions, legend) = calculate_suggested_fillups(&trips, tp_consumption);

    // All 3 trips should have suggestions
    assert_eq!(suggestions.len(), 3);

    // Legend should be the MOST RECENT trip's suggestion (trip3, latest date).
    // Cumulative km for trip3: 100 + 150 + 200 = 450 km
    let legend = legend.expect("Legend should exist");
    let trip3_suggestion = suggestions.get(&trip3.id.to_string()).unwrap();
    assert_eq!(legend.liters, trip3_suggestion.liters);
    assert_eq!(legend.consumption_rate, trip3_suggestion.consumption_rate);

    // Verify it's NOT the first trip's suggestion
    let trip1_suggestion = suggestions.get(&trip1.id.to_string()).unwrap();
    assert!(legend.liters > trip1_suggestion.liters);
}

#[test]
fn test_legend_suggested_fillup_none_when_closed() {
    // When all periods are closed, legend should be None
    let trip = make_trip_for_magic_fill(
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        100.0,
        Some(8.0), // full tank - closes period
        true,
    );

    let (suggestions, legend) = calculate_suggested_fillups(&[trip], 6.0);

    assert!(suggestions.is_empty()); // No open period
    assert!(legend.is_none());
}

// ========================================================================
// Home Assistant Settings Tests
// ========================================================================

#[test]
fn test_vehicle_with_ha_sensor_persists() {
    use crate::models::{Vehicle, VehicleType};

    let db = Database::in_memory().unwrap();
    let now = Utc::now();

    let vehicle = Vehicle {
        id: Uuid::new_v4(),
        name: "Test Car".to_string(),
        license_plate: "HA-001-HA".to_string(),
        vehicle_type: VehicleType::Ice,
        tank_size_liters: Some(50.0),
        tp_consumption: Some(6.0),
        initial_odometer: 10000.0,
        battery_capacity_kwh: None,
        baseline_consumption_kwh: None,
        initial_battery_percent: None,
        is_active: true,
        vin: None,
        driver_name: None,
        ha_odo_sensor: Some("sensor.car_odometer".to_string()),
        ha_fillup_sensor: None,
        ha_fuel_level_sensor: None,
        created_at: now,
        updated_at: now,
    };

    db.create_vehicle(&vehicle).unwrap();

    // Retrieve and verify sensor is persisted
    let loaded = db.get_vehicle(&vehicle.id.to_string()).unwrap().unwrap();
    assert_eq!(
        loaded.ha_odo_sensor,
        Some("sensor.car_odometer".to_string())
    );
}

#[test]
fn test_vehicle_ha_sensor_update() {
    use crate::models::{Vehicle, VehicleType};

    let db = Database::in_memory().unwrap();
    let now = Utc::now();

    let mut vehicle = Vehicle {
        id: Uuid::new_v4(),
        name: "Test Car".to_string(),
        license_plate: "HA-002-HA".to_string(),
        vehicle_type: VehicleType::Ice,
        tank_size_liters: Some(50.0),
        tp_consumption: Some(6.0),
        initial_odometer: 10000.0,
        battery_capacity_kwh: None,
        baseline_consumption_kwh: None,
        initial_battery_percent: None,
        is_active: true,
        vin: None,
        driver_name: None,
        ha_odo_sensor: None,
        ha_fillup_sensor: None,
        ha_fuel_level_sensor: None,
        created_at: now,
        updated_at: now,
    };

    db.create_vehicle(&vehicle).unwrap();

    // Update sensor
    vehicle.ha_odo_sensor = Some("sensor.new_odometer".to_string());
    db.update_vehicle(&vehicle).unwrap();

    // Verify update
    let loaded = db.get_vehicle(&vehicle.id.to_string()).unwrap().unwrap();
    assert_eq!(
        loaded.ha_odo_sensor,
        Some("sensor.new_odometer".to_string())
    );

    // Clear sensor
    vehicle.ha_odo_sensor = None;
    db.update_vehicle(&vehicle).unwrap();

    let loaded = db.get_vehicle(&vehicle.id.to_string()).unwrap().unwrap();
    assert_eq!(loaded.ha_odo_sensor, None);
}

#[test]
fn test_vehicle_ha_sensor_null_by_default() {
    use crate::models::Vehicle;

    let db = Database::in_memory().unwrap();

    // Create vehicle using constructor (no ha_odo_sensor parameter)
    let vehicle = Vehicle::new_ice(
        "Test Car".to_string(),
        "HA-003-HA".to_string(),
        50.0,
        6.0,
        10000.0,
    );

    db.create_vehicle(&vehicle).unwrap();

    // Verify sensor is null by default
    let loaded = db.get_vehicle(&vehicle.id.to_string()).unwrap().unwrap();
    assert_eq!(loaded.ha_odo_sensor, None);
}

// ============================================================================
// Home Assistant Fillup Sensor Tests
// ============================================================================

#[test]
fn test_format_suggested_fillup_text_with_suggestion() {
    use crate::commands_internal::integrations::format_suggested_fillup_text;
    use crate::models::SuggestedFillup;

    let suggestion = SuggestedFillup {
        liters: 20.39,
        consumption_rate: 5.66,
    };

    assert_eq!(
        format_suggested_fillup_text(Some(&suggestion)),
        "20.39 L → 5.66 l/100km"
    );
}

#[test]
fn test_format_suggested_fillup_text_none() {
    use crate::commands_internal::integrations::format_suggested_fillup_text;

    assert_eq!(format_suggested_fillup_text(None), "Plná nádrž");
}

#[test]
fn test_format_suggested_fillup_text_rounding() {
    use crate::commands_internal::integrations::format_suggested_fillup_text;
    use crate::models::SuggestedFillup;

    let suggestion = SuggestedFillup {
        liters: 38.123456,
        consumption_rate: 5.789012,
    };

    assert_eq!(
        format_suggested_fillup_text(Some(&suggestion)),
        "38.12 L → 5.79 l/100km"
    );
}

#[test]
fn test_vehicle_ha_fillup_sensor_persistence() {
    use crate::models::{Vehicle, VehicleType};

    let db = Database::in_memory().unwrap();
    let now = Utc::now();

    let vehicle = Vehicle {
        id: Uuid::new_v4(),
        name: "Test Car".to_string(),
        license_plate: "HA-004-HA".to_string(),
        vehicle_type: VehicleType::Ice,
        tank_size_liters: Some(50.0),
        tp_consumption: Some(6.0),
        initial_odometer: 10000.0,
        battery_capacity_kwh: None,
        baseline_consumption_kwh: None,
        initial_battery_percent: None,
        is_active: true,
        vin: None,
        driver_name: None,
        ha_odo_sensor: None,
        ha_fillup_sensor: Some("sensor.kniha_jazd_fillup".to_string()),
        ha_fuel_level_sensor: None,
        created_at: now,
        updated_at: now,
    };

    db.create_vehicle(&vehicle).unwrap();

    let loaded = db.get_vehicle(&vehicle.id.to_string()).unwrap().unwrap();
    assert_eq!(
        loaded.ha_fillup_sensor,
        Some("sensor.kniha_jazd_fillup".to_string())
    );
}

#[test]
fn test_vehicle_ha_fillup_sensor_null_by_default() {
    use crate::models::Vehicle;

    let db = Database::in_memory().unwrap();

    let vehicle = Vehicle::new_ice(
        "Test Car".to_string(),
        "HA-005-HA".to_string(),
        50.0,
        6.0,
        10000.0,
    );

    db.create_vehicle(&vehicle).unwrap();

    let loaded = db.get_vehicle(&vehicle.id.to_string()).unwrap().unwrap();
    assert_eq!(loaded.ha_fillup_sensor, None);
}

// ============================================================================
// Vehicle HA Fuel Level Sensor Tests
// ============================================================================

#[test]
fn test_vehicle_ha_fuel_level_sensor_persistence() {
    use crate::models::{Vehicle, VehicleType};

    let db = Database::in_memory().unwrap();
    let now = Utc::now();

    let vehicle = Vehicle {
        id: Uuid::new_v4(),
        name: "Test Car".to_string(),
        license_plate: "HA-006-HA".to_string(),
        vehicle_type: VehicleType::Ice,
        tank_size_liters: Some(50.0),
        tp_consumption: Some(6.0),
        initial_odometer: 10000.0,
        battery_capacity_kwh: None,
        baseline_consumption_kwh: None,
        initial_battery_percent: None,
        is_active: true,
        vin: None,
        driver_name: None,
        ha_odo_sensor: None,
        ha_fillup_sensor: None,
        ha_fuel_level_sensor: Some("sensor.car_fuel_level".to_string()),
        created_at: now,
        updated_at: now,
    };

    db.create_vehicle(&vehicle).unwrap();

    let loaded = db.get_vehicle(&vehicle.id.to_string()).unwrap().unwrap();
    assert_eq!(
        loaded.ha_fuel_level_sensor,
        Some("sensor.car_fuel_level".to_string())
    );
}

#[test]
fn test_vehicle_ha_fuel_level_sensor_null_by_default() {
    use crate::models::Vehicle;

    let db = Database::in_memory().unwrap();

    let vehicle = Vehicle::new_ice(
        "Test Car".to_string(),
        "HA-007-HA".to_string(),
        50.0,
        6.0,
        10000.0,
    );

    db.create_vehicle(&vehicle).unwrap();

    let loaded = db.get_vehicle(&vehicle.id.to_string()).unwrap().unwrap();
    assert_eq!(loaded.ha_fuel_level_sensor, None);
}

// ============================================================================
// Synthetic First Record Tests (Export)
// ============================================================================

/// Test that synthetic first record's fuel_remaining should be the year-start fuel (initial_fuel).
/// The year-start fuel is either carryover from previous year or full tank if no previous data.
#[test]
fn test_synthetic_first_record_fuel_remaining_is_initial_fuel() {
    // Given: initial_fuel = 40.0 (e.g., carryover from previous year)
    let initial_fuel = 40.0;
    let tank_size = 50.0;

    // And: a trip that uses some fuel
    let mut trip = make_trip_with_fuel(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(), 30.0, 45.0);
    trip.distance_km = 100.0;
    let trips = vec![trip.clone()];

    // When: we calculate fuel_remaining with initial_fuel
    let rates = std::collections::HashMap::from([(trip.id.to_string(), 6.0)]); // 6 l/100km
    let mut fuel_remaining = calculate_fuel_remaining(&trips, &rates, initial_fuel, tank_size);

    // Then: the first trip's fuel_remaining is calculated from initial_fuel
    // fuel = 40 - (100 * 6 / 100) + 30 = 40 - 6 + 30 = 64, clamped to 50
    assert_eq!(*fuel_remaining.get(&trip.id.to_string()).unwrap(), 50.0);

    // And: when we add the synthetic first record entry (as export_to_browser does)
    fuel_remaining.insert(Uuid::nil().to_string(), initial_fuel);

    // Then: the synthetic record has the year-start fuel (BEFORE any trips)
    assert_eq!(*fuel_remaining.get(&Uuid::nil().to_string()).unwrap(), 40.0);
}

/// Test that when there's no previous year data, the synthetic first record
/// should show full tank (tank_size) as the zostatok.
#[test]
fn test_synthetic_first_record_fuel_remaining_full_tank_default() {
    // Given: no previous year data, so initial_fuel = tank_size (full tank)
    let tank_size = 50.0;
    let initial_fuel = tank_size; // Full tank assumption

    // When: we add the synthetic first record entry
    let mut fuel_remaining = std::collections::HashMap::new();
    fuel_remaining.insert(Uuid::nil().to_string(), initial_fuel);

    // Then: the synthetic record shows full tank
    assert_eq!(*fuel_remaining.get(&Uuid::nil().to_string()).unwrap(), 50.0);
}

// ============================================================================
// Legal Compliance Tests (2026)
// ============================================================================

#[test]
fn test_trip_numbers_chronological_order() {
    // Given trips in various orders, trip_numbers should be 1,2,3... by date
    let trips = vec![
        make_trip_with_date("2026-01-15", 100.0, 10100.0), // Should be #2
        make_trip_with_date("2026-01-10", 50.0, 10050.0),  // Should be #1
        make_trip_with_date("2026-01-20", 75.0, 10175.0),  // Should be #3
    ];

    let trip_numbers = calculate_trip_numbers(&trips);

    // Find by date to verify numbering
    let jan10_id = trips
        .iter()
        .find(|t| t.start_datetime.date().day() == 10)
        .unwrap()
        .id
        .to_string();
    let jan15_id = trips
        .iter()
        .find(|t| t.start_datetime.date().day() == 15)
        .unwrap()
        .id
        .to_string();
    let jan20_id = trips
        .iter()
        .find(|t| t.start_datetime.date().day() == 20)
        .unwrap()
        .id
        .to_string();

    assert_eq!(trip_numbers.get(&jan10_id), Some(&1));
    assert_eq!(trip_numbers.get(&jan15_id), Some(&2));
    assert_eq!(trip_numbers.get(&jan20_id), Some(&3));
}

// Same-datetime tiebreaker is now exercised by
// `test_trip_numbers_same_datetime_tiebroken_by_created_at` below.

// =============================================================================
// Odometer Start Derivation Tests
// =============================================================================

#[test]
fn test_odometer_start_first_trip_uses_initial() {
    let initial_odo = 10000.0;
    let trips = vec![make_trip_with_date_odo("2026-01-10", 50.0, 10050.0)];

    let odo_start = calculate_odometer_start(&trips, initial_odo);

    let trip_id = trips[0].id.to_string();
    assert_eq!(odo_start.get(&trip_id), Some(&10000.0));
}

#[test]
fn test_odometer_start_subsequent_trips() {
    let initial_odo = 10000.0;
    let trips = vec![
        make_trip_with_date_odo("2026-01-10", 50.0, 10050.0), // start: 10000
        make_trip_with_date_odo("2026-01-15", 100.0, 10150.0), // start: 10050
        make_trip_with_date_odo("2026-01-20", 50.0, 10200.0), // start: 10150
    ];

    let odo_start = calculate_odometer_start(&trips, initial_odo);

    assert_eq!(odo_start.get(&trips[0].id.to_string()), Some(&10000.0));
    assert_eq!(odo_start.get(&trips[1].id.to_string()), Some(&10050.0));
    assert_eq!(odo_start.get(&trips[2].id.to_string()), Some(&10150.0));
}

#[test]
fn test_odometer_start_respects_chronological_order() {
    // Trips not in date order in the vec - should still derive correctly
    let initial_odo = 10000.0;
    let trips = vec![
        make_trip_with_date_odo("2026-01-20", 50.0, 10200.0), // chronologically 3rd
        make_trip_with_date_odo("2026-01-10", 50.0, 10050.0), // chronologically 1st
        make_trip_with_date_odo("2026-01-15", 100.0, 10150.0), // chronologically 2nd
    ];

    let odo_start = calculate_odometer_start(&trips, initial_odo);

    // Trip on Jan 10 is first chronologically, so uses initial_odo
    let jan10 = trips
        .iter()
        .find(|t| t.start_datetime.date().day() == 10)
        .unwrap();
    assert_eq!(odo_start.get(&jan10.id.to_string()), Some(&10000.0));

    // Trip on Jan 15 uses Jan 10's ending odo
    let jan15 = trips
        .iter()
        .find(|t| t.start_datetime.date().day() == 15)
        .unwrap();
    assert_eq!(odo_start.get(&jan15.id.to_string()), Some(&10050.0));

    // Trip on Jan 20 uses Jan 15's ending odo
    let jan20 = trips
        .iter()
        .find(|t| t.start_datetime.date().day() == 20)
        .unwrap();
    assert_eq!(odo_start.get(&jan20.id.to_string()), Some(&10150.0));
}

/// Helper to create trip with specific date
fn make_trip_with_date(date_str: &str, distance: f64, odo: f64) -> Trip {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
    Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        start_datetime: date.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: None,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: distance,
        odometer: odo,
        purpose: "test".to_string(),
        fuel_liters: None,
        fuel_cost_eur: None,
        full_tank: false,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn make_trip_with_date_odo(date_str: &str, distance: f64, odo: f64) -> Trip {
    make_trip_with_date(date_str, distance, odo)
}

/// Helper to create a trip with specific datetime, created_at, and odometer.
/// `created_at_offset_secs` shifts the trip's `created_at` from `now` —
/// negative values make it "earlier", positive "later".
fn make_trip_with_datetime_created(
    date_str: &str,
    hour: u32,
    minute: u32,
    distance: f64,
    odo: f64,
    created_at_offset_secs: i64,
) -> Trip {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
    let created_at = Utc::now() + chrono::Duration::seconds(created_at_offset_secs);
    Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        start_datetime: date.and_hms_opt(hour, minute, 0).unwrap(),
        end_datetime: None,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: distance,
        odometer: odo,
        purpose: "test".to_string(),
        fuel_liters: None,
        fuel_cost_eur: None,
        full_tank: false,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at,
        updated_at: created_at,
    }
}

#[test]
fn test_calculate_odometer_start_same_datetime_uses_created_at() {
    // Two trips with identical start_datetime but different created_at timestamps.
    // After Task 7, created_at ASC is the tiebreaker (earlier created_at = earlier in order).
    // If we sort by odometer instead (bug), wrong trip gets wrong start odo.
    let initial_odo = 10000.0;

    // Trip A: created earlier (so it sorts first), has "wrong" high odometer
    let trip_a = make_trip_with_datetime_created("2026-01-10", 8, 0, 50.0, 10200.0, -10);
    // Trip B: created later (so it sorts second), has "wrong" low odometer
    let trip_b = make_trip_with_datetime_created("2026-01-10", 8, 0, 50.0, 10100.0, 0);

    let trips = vec![trip_a.clone(), trip_b.clone()];
    let odo_start = calculate_odometer_start(&trips, initial_odo);

    // Trip A (earlier created_at = first) should use initial_odo as start
    assert_eq!(
        odo_start.get(&trip_a.id.to_string()),
        Some(&initial_odo),
        "First trip (earlier created_at) should start from initial odometer"
    );
    // Trip B (later created_at = second) should use trip A's ending odo as start
    assert_eq!(
        odo_start.get(&trip_b.id.to_string()),
        Some(&10200.0),
        "Second trip (later created_at) should start from previous trip's odometer"
    );
}

#[test]
fn test_year_start_odometer_same_day_uses_time_and_created_at() {
    // Two trips on same day with different times. The later trip's odometer
    // should be returned as year-end value.
    // Bug: get_year_start_odometer sorts by date only (strips time), then
    // uses odometer as tiebreaker — if odometers are "wrong", returns wrong value.
    let db = crate::db::Database::in_memory().expect("Failed to create database");

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        50.0,
        6.0,
        10000.0,
    );
    db.create_vehicle(&vehicle).expect("Failed to create vehicle");

    let now = Utc::now();
    let date = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

    // Morning trip: earlier start time, higher odo (data corruption scenario)
    let trip_morning = Trip {
        id: Uuid::new_v4(),
        vehicle_id: vehicle.id,
        start_datetime: date.and_hms_opt(8, 0, 0).unwrap(),
        end_datetime: None,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: 300.0,
        odometer: 50300.0, // "wrong" — higher than afternoon trip
        purpose: "test".to_string(),
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
    };

    // Afternoon trip: later start time, lower odo (data corruption scenario)
    let trip_afternoon = Trip {
        id: Uuid::new_v4(),
        vehicle_id: vehicle.id,
        start_datetime: date.and_hms_opt(16, 0, 0).unwrap(),
        end_datetime: None,
        origin: "B".to_string(),
        destination: "C".to_string(),
        distance_km: 200.0,
        odometer: 50200.0, // "wrong" — lower than morning trip
        purpose: "test".to_string(),
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
    };

    db.create_trip(&trip_morning).expect("Failed to create trip");
    db.create_trip(&trip_afternoon).expect("Failed to create trip");

    let result = get_year_start_odometer(&db, &vehicle.id.to_string(), 2025, 10000.0);

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        50200.0,
        "Should return afternoon trip's odometer (later time), not morning trip's (higher odo)"
    );
}

// =============================================================================
// Month-End Row Generation Tests
// =============================================================================

#[test]
fn test_month_end_rows_generated_for_gaps() {
    // Trips only in January and March
    // Only generate rows for CLOSED months (months before the latest trip month)
    // March is the latest month, so Jan and Feb are closed
    let trips = vec![
        make_trip_with_date_odo("2026-01-15", 50.0, 10050.0),
        make_trip_with_date_odo("2026-03-10", 50.0, 10100.0),
    ];
    let year = 2026;
    let initial_odo = 10000.0;
    let mut fuel_remaining: HashMap<String, f64> = HashMap::new();
    fuel_remaining.insert(trips[0].id.to_string(), 45.0); // After Jan 15 trip
    fuel_remaining.insert(trips[1].id.to_string(), 40.0); // After Mar 10 trip
    let initial_fuel = 50.0;

    let trip_numbers = calculate_trip_numbers(&trips);
    let rows = generate_month_end_rows(
        &trips,
        year,
        initial_odo,
        initial_fuel,
        &fuel_remaining,
        &trip_numbers,
    );

    // Should have rows for: Jan 31, Feb 28 (closed months before March)
    // Mar 31 NOT generated (March is the latest month, not yet closed)
    assert_eq!(rows.len(), 2);

    // Jan 31 carries Jan 15's values
    let jan = rows.iter().find(|r| r.month == 1).unwrap();
    assert_eq!(jan.date, NaiveDate::from_ymd_opt(2026, 1, 31).unwrap());
    assert_eq!(jan.odometer, 10050.0);
    assert_eq!(jan.fuel_remaining, 45.0);

    // Feb 28 carries Jan's state (no trips in Feb)
    let feb = rows.iter().find(|r| r.month == 2).unwrap();
    assert_eq!(feb.date, NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
    assert_eq!(feb.odometer, 10050.0);
    assert_eq!(feb.fuel_remaining, 45.0);

    // Mar should NOT have a row (latest month, not closed)
    let mar = rows.iter().find(|r| r.month == 3);
    assert!(mar.is_none());
}

#[test]
fn test_month_end_rows_always_generated_for_closed_months() {
    // Trip on Jan 31 AND a trip in February (so January is a "closed" month)
    // Month-end rows are ALWAYS generated for closed months (even if trip exists on last day)
    let trips = vec![
        make_trip_with_date_odo("2026-01-31", 50.0, 10050.0),
        make_trip_with_date_odo("2026-02-15", 50.0, 10100.0), // Makes January "closed"
    ];
    let year = 2026;
    let mut fuel_remaining: HashMap<String, f64> = HashMap::new();
    fuel_remaining.insert(trips[0].id.to_string(), 45.0);
    fuel_remaining.insert(trips[1].id.to_string(), 40.0);

    let trip_numbers = calculate_trip_numbers(&trips);
    let rows = generate_month_end_rows(&trips, year, 10000.0, 50.0, &fuel_remaining, &trip_numbers);

    // Jan SHOULD have synthetic row (always generated for closed months)
    let jan_row = rows.iter().find(|r| r.month == 1);
    assert!(
        jan_row.is_some(),
        "Should create synthetic row for all closed months"
    );

    // Total rows should be 1 (Jan is closed, Feb is not)
    assert_eq!(rows.len(), 1);

    // Verify Jan row has correct values from the Jan 31 trip
    let jan = jan_row.unwrap();
    assert_eq!(jan.odometer, 10050.0);
    assert_eq!(jan.fuel_remaining, 45.0);
}

#[test]
fn test_month_end_rows_none_when_no_trips() {
    // No trips at all - no months are "closed" so no rows generated
    let trips: Vec<Trip> = vec![];
    let year = 2026;
    let fuel_remaining: HashMap<String, f64> = HashMap::new();

    let trip_numbers: HashMap<String, i32> = HashMap::new();
    let rows = generate_month_end_rows(&trips, year, 10000.0, 50.0, &fuel_remaining, &trip_numbers);

    // No trips = no closed months = no month-end rows
    assert_eq!(rows.len(), 0);
}

// ============================================================================
// Chronological ordering contract (datetime-is-order, Task 65)
// ============================================================================

#[test]
fn test_create_trip_orders_by_date_regardless_of_creation_order() {
    // After datetime-is-order ships, the order in which trips are CREATED
    // must not influence the final retrieval order — only start_datetime does.
    // get_trips_for_vehicle orders by start_datetime DESC, with created_at ASC as tiebreaker.
    let db = crate::db::Database::in_memory().expect("Failed to create database");
    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        50.0,
        6.0,
        10000.0,
    );
    db.create_vehicle(&vehicle).expect("Failed to create vehicle");
    let app_state = crate::app_state::AppState::new();
    let v = vehicle.id.to_string();

    // Create in non-chronological order — exactly the user's repro
    crate::commands_internal::trips::create_trip_internal(
        &db,
        &app_state,
        v.clone(),
        "2026-05-21T09:00:00".into(),
        "2026-05-21T09:30:00".into(),
        "A".into(),
        "B".into(),
        10.0,
        10000.0,
        "test".into(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    crate::commands_internal::trips::create_trip_internal(
        &db,
        &app_state,
        v.clone(),
        "2026-05-18T04:30:00".into(),
        "2026-05-18T08:30:00".into(),
        "A".into(),
        "B".into(),
        370.0,
        10370.0,
        "test".into(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    crate::commands_internal::trips::create_trip_internal(
        &db,
        &app_state,
        v.clone(),
        "2026-05-20T16:00:00".into(),
        "2026-05-20T19:00:00".into(),
        "A".into(),
        "B".into(),
        370.0,
        10740.0,
        "test".into(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();

    let trips = db.get_trips_for_vehicle(&v).unwrap();
    assert_eq!(trips.len(), 3);
    assert_eq!(
        trips[0].start_datetime.date(),
        NaiveDate::from_ymd_opt(2026, 5, 21).unwrap(),
        "newest first"
    );
    assert_eq!(
        trips[1].start_datetime.date(),
        NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
        "middle second"
    );
    assert_eq!(
        trips[2].start_datetime.date(),
        NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
        "oldest last"
    );
}

#[test]
fn test_trip_numbers_same_datetime_tiebroken_by_created_at() {
    // When two trips share the same start_datetime, the tiebreaker MUST be
    // created_at (earlier creation = earlier number). trip_a is created
    // earlier than trip_b, so trip_a must get #1 and trip_b #2.
    use chrono::Duration;
    let dt = NaiveDate::from_ymd_opt(2026, 5, 1)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let earlier = Utc::now() - Duration::seconds(60);
    let later = Utc::now();

    let trip_a = Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        start_datetime: dt,
        end_datetime: None,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: 10.0,
        odometer: 10000.0,
        purpose: "test".to_string(),
        fuel_liters: None,
        fuel_cost_eur: None,
        full_tank: false,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: earlier,
        updated_at: earlier,
    };
    let trip_b = Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        start_datetime: dt,
        end_datetime: None,
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: 10.0,
        odometer: 10010.0,
        purpose: "test".to_string(),
        fuel_liters: None,
        fuel_cost_eur: None,
        full_tank: false,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: later,
        updated_at: later,
    };

    let nums = calculate_trip_numbers(&[trip_a.clone(), trip_b.clone()]);
    assert_eq!(
        nums.get(&trip_a.id.to_string()),
        Some(&1),
        "earlier created_at gets #1"
    );
    assert_eq!(nums.get(&trip_b.id.to_string()), Some(&2));
}

// ============================================================================
// Odometer rewrite as an explicit Rust command, not an automatic browser
// loop (Task 3 / task 80)
// ============================================================================

/// A vehicle whose odometer starts at a known value. `setup_db_with_vehicle`
/// hardcodes 0.0, and this task needs a real start. `Vehicle::new` already
/// sets `initial_odometer` from its fifth argument, so there is nothing left
/// to assign after construction.
fn setup_db_with_start_odometer(initial: f64) -> (Database, Vehicle) {
    let db = Database::in_memory().unwrap();
    let vehicle =
        Vehicle::new("Order Car".to_string(), "BA999XY".to_string(), 66.0, 5.1, initial);
    db.create_vehicle(&vehicle).unwrap();
    (db, vehicle)
}

fn seed_chain_trip(db: &Database, vehicle_id: Uuid, day: u32, km: f64, odo: f64) -> Uuid {
    let date = NaiveDate::from_ymd_opt(2026, 3, day).unwrap();
    let mut trip = make_trip_detailed(date, km, None, false);
    trip.vehicle_id = vehicle_id;
    trip.odometer = odo;
    db.create_trip(&trip).unwrap();
    trip.id
}

#[test]
fn test_recalculate_odometers_rewrites_only_rows_that_move() {
    // The frontend loop rewrote a whole year on every save. The command
    // writes a row only when its value actually changes, and it walks the
    // canonical order rather than the DB order.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50999.0); // wrong
    let c = seed_chain_trip(&db, vehicle.id, 3, 30.0, 50150.0);

    let changes =
        recalculate_odometers_internal(&db, &app_state, vehicle.id.to_string(), 2026, false)
            .unwrap();

    // Running total: a=50050 (matches, no write), b=50120 (differs, writes),
    // c=50150 (matches, no write). Only one row actually moves.
    assert_eq!(changes.len(), 1, "only b moves");
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().odometer, 50050.0);
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().odometer, 50120.0);
    assert_eq!(db.get_trip(&c.to_string()).unwrap().unwrap().odometer, 50150.0);

    // The one change reported must name the row that moved and the values
    // it moved between -- this is the review record the old silent
    // frontend loop never left behind.
    assert_eq!(changes[0].trip_id, b.to_string());
    assert_eq!(changes[0].old_odometer, 50999.0);
    assert_eq!(changes[0].new_odometer, 50120.0);
}

#[test]
fn test_recalculate_odometers_is_read_only_guarded() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    // Seeded with a mismatch so the dry-run assertion below is checking a
    // real proposed change, not vacuously passing over an empty walk.
    seed_chain_trip(&db, vehicle.id, 1, 50.0, 99999.0);
    app_state.enable_read_only("newer migrations");

    let result =
        recalculate_odometers_internal(&db, &app_state, vehicle.id.to_string(), 2026, false);
    assert!(result.is_err(), "a write command must respect read-only mode");

    // Reading is always allowed: a dry run proposes changes but writes
    // nothing, so read-only mode must not block it.
    let dry_run =
        recalculate_odometers_internal(&db, &app_state, vehicle.id.to_string(), 2026, true)
            .unwrap();
    assert_eq!(
        dry_run.len(),
        1,
        "the dry run must still propose the seeded trip's correction, got: {dry_run:?}"
    );
}

#[test]
fn test_recalculate_odometers_is_idempotent_with_tied_start_datetimes() {
    // trip_order's final tiebreaker is the odometer, and this command
    // writes odometers -- so it can change the order it just walked. The
    // three trips below share one start_datetime and one created_at, so the
    // tiebreak (ascending odometer) alone decides the first pass's walk
    // order; the stored odometers are scrambled on purpose. One pass must
    // be a fixed point: the second call has nothing left to change.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();

    let tied_date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
    let tied_start = tied_date.and_hms_opt(9, 0, 0).unwrap();
    let tied_created = Utc::now();

    let seed_tied = |km: f64, odo: f64| -> Uuid {
        let mut trip = make_trip_detailed(tied_date, km, None, false);
        trip.vehicle_id = vehicle.id;
        trip.start_datetime = tied_start;
        trip.created_at = tied_created;
        trip.updated_at = tied_created;
        trip.odometer = odo;
        db.create_trip(&trip).unwrap();
        trip.id
    };

    // Seeded (inserted) in the opposite order from their correct odometer
    // order, so a version that trusted insertion order instead of sorting
    // would walk them backwards.
    seed_tied(70.0, 50999.0);
    seed_tied(30.0, 50010.0);
    seed_tied(50.0, 50020.0);

    let first =
        recalculate_odometers_internal(&db, &app_state, vehicle.id.to_string(), 2026, false)
            .unwrap();
    assert_eq!(
        first.len(),
        3,
        "all three tied rows must need correcting or this test proves nothing"
    );

    let second =
        recalculate_odometers_internal(&db, &app_state, vehicle.id.to_string(), 2026, false)
            .unwrap();
    assert!(second.is_empty(), "one pass must be a fixed point");
}

#[test]
fn test_recalculate_odometers_closes_a_carryover_gap() {
    // Pins the shape task 80 found in production: the year opens with a
    // stored odometer that already sits above the true carryover by a real,
    // unexplained gap (391.5 km in the 2026 book). Every row after the gap
    // is internally consistent with its neighbour, so nothing local looks
    // wrong -- but every one of them is still wrong against the carryover,
    // and the command reports and closes the gap for the whole chain.
    //
    // This is acceptable ONLY because the command runs on demand and never
    // automatically: an automatic rewrite would silently erase the exact
    // discontinuity the odometer-span warning exists to raise on a legal
    // column.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();

    let a = seed_chain_trip(&db, vehicle.id, 1, 370.0, 50761.5); // gap opens the year
    let b = seed_chain_trip(&db, vehicle.id, 2, 50.0, 50811.5); // consistent with a
    let c = seed_chain_trip(&db, vehicle.id, 3, 30.0, 50841.5); // consistent with b

    let changes = recalculate_odometers_internal(
        &db,
        &app_state,
        vehicle.id.to_string(),
        2026,
        true, // dry run -- this is the review step before anything is written
    )
    .unwrap();

    assert_eq!(changes.len(), 3, "the gap displaces every row from the first one on");

    let by_trip: HashMap<String, &OdometerChange> =
        changes.iter().map(|c| (c.trip_id.clone(), c)).collect();

    assert_eq!(by_trip[&a.to_string()].new_odometer, 50370.0);
    assert_eq!(by_trip[&b.to_string()].new_odometer, 50420.0);
    assert_eq!(by_trip[&c.to_string()].new_odometer, 50450.0);

    // Every proposed value closes exactly the same 391.5 km gap: the
    // chain's internal shape is kept, only its anchor moves.
    for change in &changes {
        assert_eq!(change.old_odometer - change.new_odometer, 391.5);
    }

    // A dry run must not have written anything.
    assert_eq!(
        db.get_trip(&a.to_string()).unwrap().unwrap().odometer,
        50761.5,
        "dry run must not write"
    );
}

// ============================================================================
// Task 81: the cascade planner. Pure arithmetic that plans what a cascading
// edit, insert, or delete would do to the odometer chain of a year, without
// touching the database. Nothing calls these yet.
// ============================================================================

/// Three consecutive rows of 2026, chain intact from a year start of 50000.
/// Returns them in `trip_order`.
fn make_cascade_chain() -> Vec<Trip> {
    let mut trips = Vec::new();
    let mut odo = 50000.0;
    for (day, km) in [(1u32, 50.0), (2, 70.0), (3, 30.0)] {
        let date = NaiveDate::from_ymd_opt(2026, 3, day).unwrap();
        let mut trip = make_trip_detailed(date, km, None, false);
        odo += km;
        trip.odometer = odo;
        trips.push(trip);
    }
    trips
}

#[test]
fn test_cascade_km_edit_shifts_every_later_row() {
    // The whole point of task 81: change the middle row's km by +10 and the
    // rows after it move +10. The row before it does not move.
    let trips = make_cascade_chain();
    let target = trips[1].id.to_string();

    let plan = plan_odometer_cascade(&trips, 50000.0, &target, 80.0, trips[1].odometer)
        .unwrap();

    assert_eq!(plan.new_distance_km, 80.0);
    assert_eq!(plan.new_odometer, 50130.0, "anchor 50050 + 80 km");
    assert_eq!(plan.delta, 10.0);
    assert_eq!(plan.changes.len(), 1, "only the row after the target moves");
    assert_eq!(plan.changes[0].trip_id, trips[2].id.to_string());
    assert_eq!(plan.changes[0].old_odometer, 50150.0);
    assert_eq!(plan.changes[0].new_odometer, 50160.0);
}

#[test]
fn test_cascade_odo_edit_derives_the_km() {
    // The mirrored direction: type an odometer, and the km becomes the gap to
    // the anchor. The shift is the same.
    let trips = make_cascade_chain();
    let target = trips[1].id.to_string();

    let plan = plan_odometer_cascade(&trips, 50000.0, &target, 70.0, 50130.0).unwrap();

    assert_eq!(plan.new_odometer, 50130.0);
    assert_eq!(plan.new_distance_km, 80.0, "50130 - anchor 50050");
    assert_eq!(plan.delta, 10.0);
    assert_eq!(plan.changes.len(), 1);
}

#[test]
fn test_cascade_km_wins_when_both_fields_change() {
    // Save can beat the preview, so both fields can arrive changed. The km is
    // what the user types; the odometer is derived. The km wins.
    let trips = make_cascade_chain();
    let target = trips[1].id.to_string();

    let plan = plan_odometer_cascade(&trips, 50000.0, &target, 80.0, 99999.0).unwrap();

    assert_eq!(plan.new_odometer, 50130.0, "anchor + km, not the stale 99999");
    assert_eq!(plan.new_distance_km, 80.0);
}

#[test]
fn test_cascade_does_nothing_when_neither_field_changed() {
    // Editing a purpose or a time must not move a single odometer. A row that
    // sits below its anchor is a record the book keeps on purpose (task 79).
    let mut trips = make_cascade_chain();
    trips[1].odometer = 50999.0; // a broken row, left alone on an unrelated save
    let target = trips[1].id.to_string();

    let plan = plan_odometer_cascade(&trips, 50000.0, &target, 70.0, 50999.0).unwrap();

    assert_eq!(plan.delta, 0.0);
    assert_eq!(plan.new_odometer, 50999.0, "unchanged");
    assert!(plan.changes.is_empty());
    assert_eq!(plan.delta_from_repair, 0.0, "no repair on an untouched row");
}

#[test]
fn test_cascade_decomposes_the_repair_from_the_edit() {
    // Row 1 of 2025 in the production book: anchor 38056.5 carried from
    // 2024-12-31, stored odometer 38145, recorded 88 km, so the span is 88.5.
    // Change the km 88 -> 90 and the delta is +1.5, not +2. The modal must be
    // able to say why, so the plan reports both parts.
    let date = NaiveDate::from_ymd_opt(2025, 1, 12).unwrap();
    let mut first = make_trip_detailed(date, 88.0, None, false);
    first.odometer = 38145.0;
    let second_date = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();
    let mut second = make_trip_detailed(second_date, 10.0, None, false);
    second.odometer = 38155.0;
    let trips = vec![first.clone(), second.clone()];

    let plan =
        plan_odometer_cascade(&trips, 38056.5, &first.id.to_string(), 90.0, 38145.0).unwrap();

    assert_eq!(plan.new_odometer, 38146.5);
    assert_eq!(plan.delta, 1.5);
    assert_eq!(plan.delta_from_distance, 2.0, "what the user typed");
    assert_eq!(plan.delta_from_repair, -0.5, "the half kilometre from 2024");
    assert!(plan.repair_crosses_year, "the anchor is the previous year's");
    assert_eq!(plan.changes.len(), 1);
    assert_eq!(plan.changes[0].new_odometer, 38156.5);
}

#[test]
fn test_cascade_reports_that_the_year_end_moved() {
    // A shift confined to one year opens the boundary to the next. The modal
    // must warn, so the plan says whether the year's last odometer moved.
    let trips = make_cascade_chain();
    let target = trips[2].id.to_string(); // the last row of the year

    let plan = plan_odometer_cascade(&trips, 50000.0, &target, 40.0, trips[2].odometer)
        .unwrap();

    assert!(plan.changes.is_empty(), "no row follows the last one");
    assert_eq!(plan.delta, 10.0);
    assert!(plan.year_end_odometer_moved);
}

#[test]
fn test_cascade_keeps_the_order_of_a_tied_group() {
    // The imported years tie on start_datetime AND created_at, so trip_order
    // falls through to the odometer. Measured on the production copy: 2025 has
    // 6 such groups, 14 rows. A shift moves every member by the same delta, so
    // the group must keep its order and its trip numbers. This is the one place
    // a cascade could reorder a legal book.
    let date = NaiveDate::from_ymd_opt(2025, 5, 5).unwrap();
    let stamp = chrono::Utc::now();
    let mut edited = make_trip_detailed(
        NaiveDate::from_ymd_opt(2025, 5, 1).unwrap(), 20.0, None, false,
    );
    edited.created_at = stamp;
    edited.odometer = 43000.0;
    let mut low = make_trip_detailed(date, 91.0, None, false);
    low.created_at = stamp;
    low.odometer = 43091.0;
    let mut high = make_trip_detailed(date, 120.0, None, false);
    high.created_at = stamp;
    high.odometer = 43211.0;
    let trips = vec![edited.clone(), low.clone(), high.clone()];

    let plan =
        plan_odometer_cascade(&trips, 42980.0, &edited.id.to_string(), 30.0, 43000.0)
            .unwrap();

    assert_eq!(plan.delta, 10.0);
    assert_eq!(plan.changes.len(), 2);
    assert_eq!(plan.changes[0].trip_id, low.id.to_string(), "the lower odometer stays first");
    assert_eq!(plan.changes[0].new_odometer, 43101.0);
    assert_eq!(plan.changes[1].trip_id, high.id.to_string());
    assert_eq!(plan.changes[1].new_odometer, 43221.0);
    assert!(
        plan.changes[0].trip_number < plan.changes[1].trip_number,
        "the shift must not renumber the group"
    );
}

#[test]
fn test_cascade_rejects_a_trip_that_is_not_in_the_year() {
    let trips = make_cascade_chain();

    let result = plan_odometer_cascade(&trips, 50000.0, "not-a-trip-id", 10.0, 1.0);

    assert!(result.is_err());
}

// ============================================================================
// Task 9b: a negative distance_km is an impossible value on a single row --
// the odometer went backwards. Reject it in the backend, on every branch that
// can derive or receive one. Zero stays allowed: an odometer-correction row
// can legitimately record no distance (TripRow.svelte's own `min="0"` treats
// zero as the floor, not below it), and `calculate_consumption_rate` already
// treats a zero span as a no-op rather than a division.
// ============================================================================

#[test]
fn test_cascade_odo_edit_below_the_anchor_is_rejected() {
    // The odo-wins branch: typing an odometer below the row's anchor derives
    // a negative distance_km. That must never reach the database silently.
    let trips = make_cascade_chain();
    let target = trips[1].id.to_string(); // anchor is trips[0].odometer, 50050.0

    let result = plan_odometer_cascade(&trips, 50000.0, &target, 70.0, 50040.0);

    let err = result.unwrap_err();
    assert!(err.contains("50040"), "names the submitted odometer: {}", err);
    assert!(err.contains("50050"), "names the anchor: {}", err);
}

#[test]
fn test_cascade_km_edit_negative_is_rejected() {
    // The km-wins branch: the HTML input has `min="0"`, but the JSON-RPC
    // layer does not, so a negative distance_km can arrive directly.
    let trips = make_cascade_chain();
    let target = trips[1].id.to_string();

    let result = plan_odometer_cascade(&trips, 50000.0, &target, -5.0, trips[1].odometer);

    assert!(result.is_err());
}

#[test]
fn test_cascade_zero_km_edit_is_allowed() {
    // Only NEGATIVE is impossible. A zero-km row (an odometer correction) is
    // odd but valid, and must not be trapped by the guard.
    let trips = make_cascade_chain();
    let target = trips[1].id.to_string(); // anchor 50050.0

    let plan =
        plan_odometer_cascade(&trips, 50000.0, &target, 0.0, trips[1].odometer).unwrap();

    assert_eq!(plan.new_distance_km, 0.0);
    assert_eq!(plan.new_odometer, 50050.0, "anchor + 0");
}

#[test]
fn test_cascade_no_change_on_a_row_already_below_its_anchor_still_succeeds() {
    // Task 79: three production rows already sit below their anchor from a
    // save the guard did not exist to stop. A save that touches neither field
    // must keep succeeding for them -- the guard must not retroactively block
    // a row it did not create, and it must cascade nothing.
    let mut trips = make_cascade_chain();
    trips[1].odometer = 50010.0; // below the 50050.0 anchor: already broken
    trips[1].distance_km = -40.0; // what task 79's rows actually look like
    let target = trips[1].id.to_string();

    let plan = plan_odometer_cascade(&trips, 50000.0, &target, -40.0, 50010.0).unwrap();

    assert_eq!(plan.delta, 0.0);
    assert_eq!(plan.new_odometer, 50010.0, "unchanged");
    assert_eq!(plan.new_distance_km, -40.0, "unchanged, no repair attempted");
    assert!(plan.changes.is_empty());
}

#[test]
fn test_cascade_insert_in_the_middle_shifts_every_row_after_it() {
    // The chain is 1 March 50 km, 2 March 70 km, 3 March 30 km. Put a 25 km
    // row on 2 March at 06:00 and everything from the 70 km row onward moves
    // +25.
    let trips = make_cascade_chain();
    let when = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap().and_hms_opt(6, 0, 0).unwrap();

    let plan = plan_insert_cascade(&trips, 50000.0, when, 25.0).unwrap();

    assert_eq!(plan.new_odometer, 50075.0, "anchor 50050 + 25 km");
    assert_eq!(plan.delta, 25.0);
    assert_eq!(plan.delta_from_repair, 0.0, "a new row carries no span error");
    assert_eq!(plan.changes.len(), 2);
    assert_eq!(plan.changes[0].new_odometer, 50145.0);
    assert_eq!(plan.changes[1].new_odometer, 50175.0);
}

#[test]
fn test_cascade_insert_at_the_end_moves_no_other_row() {
    // Appending is the daily action. It must stay a silent write: no row moves,
    // so the grid opens no modal.
    let trips = make_cascade_chain();
    let when = NaiveDate::from_ymd_opt(2026, 3, 9).unwrap().and_hms_opt(8, 0, 0).unwrap();

    let plan = plan_insert_cascade(&trips, 50000.0, when, 25.0).unwrap();

    assert_eq!(plan.new_odometer, 50175.0, "anchor is the last row, 50150");
    assert!(plan.changes.is_empty());
    assert!(plan.year_end_odometer_moved);
    assert!(!plan.next_year_chain_breaks, "the planner never sets this");
}

#[test]
fn test_cascade_insert_before_every_row_anchors_on_the_year_start() {
    let trips = make_cascade_chain();
    let when = NaiveDate::from_ymd_opt(2026, 1, 4).unwrap().and_hms_opt(8, 0, 0).unwrap();

    let plan = plan_insert_cascade(&trips, 50000.0, when, 12.0).unwrap();

    assert_eq!(plan.new_odometer, 50012.0);
    assert_eq!(plan.changes.len(), 3, "the whole year moves");
    assert_eq!(plan.changes[0].new_odometer, 50062.0);
}

#[test]
fn test_cascade_insert_with_negative_distance_is_rejected() {
    // Task 9b: an insert has no odometer field to derive from, so the only
    // guard is on the distance itself.
    let trips = make_cascade_chain();
    let when = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap().and_hms_opt(6, 0, 0).unwrap();

    let result = plan_insert_cascade(&trips, 50000.0, when, -5.0);

    assert!(result.is_err());
}

#[test]
fn test_cascade_insert_with_zero_distance_is_allowed() {
    let trips = make_cascade_chain();
    let when = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap().and_hms_opt(6, 0, 0).unwrap();

    let plan = plan_insert_cascade(&trips, 50000.0, when, 0.0).unwrap();

    assert_eq!(plan.new_distance_km, 0.0);
}

#[test]
fn test_cascade_delete_shifts_by_the_removed_span_not_its_distance() {
    // The row records 70 km but spans 80. Deleting it takes 80 out of the
    // chain, because 80 is what the chain actually loses. Shifting by 70 would
    // leave the next row 10 km adrift.
    let mut trips = make_cascade_chain();
    trips[1].odometer = 50130.0; // 70 km recorded, 80 km span from 50050
    trips[2].odometer = 50160.0;
    let target = trips[1].id.to_string();

    let plan = plan_delete_cascade(&trips, 50000.0, &target).unwrap();

    assert_eq!(plan.delta, -80.0);
    assert_eq!(plan.delta_from_distance, -70.0, "what the row records");
    assert_eq!(plan.delta_from_repair, -10.0, "the span error it also removes");
    assert_eq!(plan.changes.len(), 1);
    assert_eq!(plan.changes[0].old_odometer, 50160.0);
    assert_eq!(plan.changes[0].new_odometer, 50080.0, "50050 + 30 km");
}

#[test]
fn test_cascade_delete_of_a_consistent_row_shifts_by_its_distance() {
    let trips = make_cascade_chain();
    let target = trips[1].id.to_string();

    let plan = plan_delete_cascade(&trips, 50000.0, &target).unwrap();

    assert_eq!(plan.delta, -70.0);
    assert_eq!(plan.delta_from_repair, 0.0);
    assert_eq!(plan.changes[0].new_odometer, 50080.0);
}

#[test]
fn test_cascade_delete_of_the_last_row_moves_nothing_but_the_year_end() {
    let trips = make_cascade_chain();
    let target = trips[2].id.to_string();

    let plan = plan_delete_cascade(&trips, 50000.0, &target).unwrap();

    assert!(plan.changes.is_empty());
    assert!(plan.year_end_odometer_moved);
}

// ============================================================================
// Task 80: the preview returns the row's odometer, the editor does not compute
// it. Both fields come from the canonical order, so the editor and the grid
// read one chain (ADR-008).
// ============================================================================

#[test]
fn test_preview_returns_the_row_odometer() {
    // The editor stops computing odo = previous + km. The preview command
    // already runs on every km change, so it returns the answer instead.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);

    let preview = preview_trip_calculation_internal(
        &db,
        vehicle.id.to_string(),
        2026,
        70.0,
        None,  // fuel_liters
        false, // full_tank
        None,  // insert_at_trip_id
        None,  // editing_trip_id
    )
    .unwrap();

    assert_eq!(preview.odometer_start, 50050.0);
    assert_eq!(preview.odometer, 50120.0);
}

#[test]
fn test_preview_answers_a_fractional_distance() {
    // The book holds odometers that end in .5, so the editor sends a
    // fractional km. The command took an i32, so `parse_args` rejected the
    // request, the preview came back null, and a new row saved odometer 0.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);

    let preview = preview_trip_calculation_internal(
        &db,
        vehicle.id.to_string(),
        2026,
        100.5,
        None,  // fuel_liters
        false, // full_tank
        None,  // insert_at_trip_id
        None,  // editing_trip_id
    )
    .unwrap();

    assert_eq!(preview.odometer_start, 50050.0);
    assert_eq!(preview.odometer, 50150.5);
}

#[test]
fn test_preview_of_an_edited_row_anchors_on_its_canonical_predecessor() {
    // Two of the three rows are tied on start_datetime and created_at, so
    // only trip_order decides which of them comes first. The edited row is
    // the second one; its anchor must be the row BEFORE it in that order.
    //
    // Each way of getting it wrong reports a different number:
    //   the canonical predecessor (correct) -> 50050
    //   the tied row below it               -> 50250
    //   the year start                      -> 50000
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let day = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
    let older = Utc::now() - chrono::Duration::days(2);
    let tied_stamp = Utc::now() - chrono::Duration::days(1);

    seed_trip(
        &db,
        vehicle.id,
        day.and_hms_opt(8, 0, 0).unwrap(),
        50.0,
        50050.0,
        None,
        older,
    );
    let edited = seed_trip(
        &db,
        vehicle.id,
        day.and_hms_opt(12, 0, 0).unwrap(),
        100.0,
        50150.0,
        None,
        tied_stamp,
    );
    seed_trip(
        &db,
        vehicle.id,
        day.and_hms_opt(12, 0, 0).unwrap(),
        100.0,
        50250.0,
        None,
        tied_stamp,
    );

    let preview = preview_trip_calculation_internal(
        &db,
        vehicle.id.to_string(),
        2026,
        70.0,
        None,
        false,
        None,
        Some(edited.id.to_string()),
    )
    .unwrap();

    assert_eq!(
        preview.odometer_start, 50050.0,
        "the anchor is the odometer of the row before this one in trip_order"
    );
    assert_eq!(
        preview.odometer, 50120.0,
        "the row ends at anchor + the previewed distance"
    );
}

// ============================================================================
// Task 81: the three cascade commands -- load, plan, apply.
// ============================================================================

/// Build the argument list `update_trip_cascade_internal` takes for a row that
/// changes only its distance. Keeps the tests below to the numbers that matter.
fn cascade_args(trip: &Trip, km: f64, odo: f64) -> (String, String, String, f64, f64) {
    (
        trip.start_datetime.format("%Y-%m-%dT%H:%M:%S").to_string(),
        trip.origin.clone(),
        trip.destination.clone(),
        km,
        odo,
    )
}

#[test]
fn test_update_trip_cascade_dry_run_writes_nothing() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);

    let trip_a = db.get_trip(&a.to_string()).unwrap().unwrap();
    let (start, origin, destination, km, odo) = cascade_args(&trip_a, 60.0, 50050.0);
    let result = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), start.clone(), start, origin, destination,
        km, odo, trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        true,
    )
    .unwrap();

    assert!(result.trip.is_none(), "a dry run returns no saved trip");
    assert_eq!(result.plan.delta, 10.0);
    assert_eq!(result.plan.changes.len(), 1);
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().odometer, 50050.0);
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().odometer, 50120.0);
}

#[test]
fn test_update_trip_cascade_applies_the_shift() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);
    let c = seed_chain_trip(&db, vehicle.id, 3, 30.0, 50150.0);

    let trip_a = db.get_trip(&a.to_string()).unwrap().unwrap();
    let (start, origin, destination, km, odo) = cascade_args(&trip_a, 60.0, 50050.0);
    let result = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), start.clone(), start, origin, destination,
        km, odo, trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        false,
    )
    .unwrap();

    assert!(result.trip.is_some());
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().odometer, 50060.0);
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().odometer, 50130.0);
    assert_eq!(db.get_trip(&c.to_string()).unwrap().unwrap().odometer, 50160.0);
    // The shifted rows keep their own distances.
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().distance_km, 70.0);
}

#[test]
fn test_update_trip_cascade_does_not_cross_the_year_boundary() {
    // The user's explicit call: the shift stops at 31 December. Pin it, so the
    // boundary break stays a visible span warning rather than a silent rewrite
    // of the next year.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let mut next_year = make_trip_detailed(
        NaiveDate::from_ymd_opt(2027, 1, 4).unwrap(), 20.0, None, false,
    );
    next_year.vehicle_id = vehicle.id;
    next_year.odometer = 50070.0;
    db.create_trip(&next_year).unwrap();

    let trip_a = db.get_trip(&a.to_string()).unwrap().unwrap();
    let (start, origin, destination, km, odo) = cascade_args(&trip_a, 60.0, 50050.0);
    let result = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), start.clone(), start, origin, destination,
        km, odo, trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        false,
    )
    .unwrap();

    assert!(result.plan.year_end_odometer_moved, "the caller must be told");
    // The flag that actually opens the modal on a last-row-of-year edit
    // (needsApproval in TripGrid.svelte). This state -- the year end moves and
    // 2027 has trips -- is the only thing mark_next_year_chain_breaks reports,
    // so without this assertion the call can be deleted and no test fails.
    assert!(
        result.plan.next_year_chain_breaks,
        "2027 has trips and the 2026 year end moved, so the chain break must be reported"
    );
    assert_eq!(
        db.get_trip(&next_year.id.to_string()).unwrap().unwrap().odometer,
        50070.0,
        "2027 is untouched"
    );
}

#[test]
fn test_update_trip_cascade_does_not_cascade_a_re_dated_row() {
    // Moving the datetime moves the row in trip_order, so two positions shift,
    // not one. That is not modelled: write the row, cascade nothing.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);

    let trip_a = db.get_trip(&a.to_string()).unwrap().unwrap();
    let moved = "2026-03-05T08:00:00".to_string();
    let result = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), moved.clone(), moved,
        trip_a.origin.clone(), trip_a.destination.clone(), 60.0, 50050.0,
        trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        false,
    )
    .unwrap();

    assert_eq!(result.plan.delta, 0.0);
    assert!(result.plan.changes.is_empty());
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().odometer, 50120.0);
}

#[test]
fn test_update_trip_cascade_re_dated_negative_distance_is_rejected() {
    // Fix round 1 (task 9b): the re_dated branch takes distance_km as
    // submitted, not derived through plan_odometer_cascade, so it bypassed
    // the earlier guard entirely. It needs its own check, on both dry_run and
    // a real write, and the row must stay unchanged when a write is rejected.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);

    let trip_a = db.get_trip(&a.to_string()).unwrap().unwrap();
    let moved = "2026-03-05T08:00:00".to_string();

    let dry = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), moved.clone(), moved.clone(),
        trip_a.origin.clone(), trip_a.destination.clone(), -10.0, 50040.0,
        trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        true,
    );
    assert!(dry.is_err(), "dry run must reject it before any modal opens");

    let applied = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), moved.clone(), moved,
        trip_a.origin.clone(), trip_a.destination.clone(), -10.0, 50040.0,
        trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        false,
    );
    assert!(applied.is_err(), "a real write must reject it too");

    assert_eq!(
        db.get_trip(&a.to_string()).unwrap().unwrap().odometer,
        50050.0,
        "the row must be unchanged after the rejected write"
    );
}

#[test]
fn test_update_trip_cascade_re_dated_unchanged_negative_distance_still_succeeds() {
    // Fix round 2 (task 9b): task 79's already-broken rows must stay
    // re-datable when the save resubmits the SAME (already negative)
    // distance. The guard fires only on a value the caller is actually
    // INTRODUCING -- mirroring plan_odometer_cascade's own no-op carve-out
    // for a row that "sits below its anchor... on purpose" (task 79).
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, -40.0, 50010.0); // already broken

    let trip_a = db.get_trip(&a.to_string()).unwrap().unwrap();
    let moved = "2026-03-05T08:00:00".to_string();
    let result = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), moved.clone(), moved,
        trip_a.origin.clone(), trip_a.destination.clone(), -40.0, 50010.0,
        trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        false,
    )
    .unwrap();

    assert_eq!(result.plan.delta, 0.0);
    assert!(result.plan.changes.is_empty());
    assert_eq!(
        db.get_trip(&a.to_string()).unwrap().unwrap().distance_km,
        -40.0,
        "written as it was, not repaired and not refused"
    );
}

#[test]
fn test_update_trip_cascade_re_dated_zero_distance_is_allowed() {
    // Fix round 2: zero is not negative, so a re-dated save introducing it
    // must still succeed -- the same rule already pinned for the other two
    // branches (test_cascade_zero_km_edit_is_allowed,
    // test_cascade_insert_with_zero_distance_is_allowed).
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);

    let trip_a = db.get_trip(&a.to_string()).unwrap().unwrap();
    let moved = "2026-03-05T08:00:00".to_string();
    let result = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), moved.clone(), moved,
        trip_a.origin.clone(), trip_a.destination.clone(), 0.0, 50000.0,
        trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        false,
    )
    .unwrap();

    assert_eq!(result.plan.new_distance_km, 0.0);
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().distance_km, 0.0);
}

#[test]
fn test_update_trip_cascade_is_read_only_guarded() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let trip_a = db.get_trip(&a.to_string()).unwrap().unwrap();
    app_state.enable_read_only("newer migrations");
    let (start, origin, destination, km, odo) = cascade_args(&trip_a, 60.0, 50050.0);

    let applied = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), start.clone(), start.clone(), origin.clone(),
        destination.clone(), km, odo, trip_a.purpose.clone(),
        None, None, None, None, None, None, None, None, None, false,
    );
    assert!(applied.is_err(), "a write must respect read-only mode");

    let dry = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), start.clone(), start, origin, destination,
        km, odo, trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        true,
    );
    assert!(dry.is_ok(), "a dry run reads only, so it is always allowed");
}

#[test]
fn test_create_trip_cascade_inserts_and_shifts() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 3, 30.0, 50080.0);

    let result = create_trip_cascade_internal(
        &db, &app_state, vehicle.id.to_string(),
        "2026-03-02T08:00:00".to_string(), "2026-03-02T09:00:00".to_string(),
        "A".to_string(), "B".to_string(), 25.0, "work".to_string(),
        None, None, None, None, None, None, None, None, None, false,
    )
    .unwrap();

    let created = result.trip.unwrap();
    assert_eq!(created.odometer, 50075.0, "the backend derived it, 50050 + 25");
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().odometer, 50105.0);
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().odometer, 50050.0, "untouched");
}

#[test]
fn test_create_trip_cascade_appending_moves_no_row_and_raises_no_warning() {
    // The daily action. It must stay a single silent write: no other row moves,
    // and there is no later year to break.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);

    let result = create_trip_cascade_internal(
        &db, &app_state, vehicle.id.to_string(),
        "2026-03-09T08:00:00".to_string(), "2026-03-09T09:00:00".to_string(),
        "A".to_string(), "B".to_string(), 25.0, "work".to_string(),
        None, None, None, None, None, None, None, None, None, true,
    )
    .unwrap();

    assert!(result.plan.changes.is_empty());
    assert!(result.plan.year_end_odometer_moved);
    assert!(
        !result.plan.next_year_chain_breaks,
        "no later year, so nothing to warn about"
    );
}

#[test]
fn test_create_trip_cascade_warns_when_a_later_year_exists() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let mut next_year = make_trip_detailed(
        NaiveDate::from_ymd_opt(2027, 1, 4).unwrap(), 20.0, None, false,
    );
    next_year.vehicle_id = vehicle.id;
    next_year.odometer = 50070.0;
    db.create_trip(&next_year).unwrap();

    let result = create_trip_cascade_internal(
        &db, &app_state, vehicle.id.to_string(),
        "2026-03-09T08:00:00".to_string(), "2026-03-09T09:00:00".to_string(),
        "A".to_string(), "B".to_string(), 25.0, "work".to_string(),
        None, None, None, None, None, None, None, None, None, true,
    )
    .unwrap();

    assert!(result.plan.next_year_chain_breaks);
}

#[test]
fn test_delete_trip_cascade_closes_the_gap() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);
    let c = seed_chain_trip(&db, vehicle.id, 3, 30.0, 50150.0);

    let plan = delete_trip_cascade_internal(&db, &app_state, b.to_string(), false).unwrap();

    assert_eq!(plan.delta, -70.0);
    assert!(db.get_trip(&b.to_string()).unwrap().is_none());
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().odometer, 50050.0, "untouched");
    assert_eq!(db.get_trip(&c.to_string()).unwrap().unwrap().odometer, 50080.0);
}

#[test]
fn test_delete_trip_cascade_dry_run_writes_nothing() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50070.0);

    let plan = delete_trip_cascade_internal(&db, &app_state, b.to_string(), true).unwrap();

    assert_eq!(plan.delta, -70.0);
    assert!(db.get_trip(&b.to_string()).unwrap().is_some(), "still there");
}

#[test]
fn test_update_trip_cascade_agrees_with_the_preview_command() {
    // preview_trip_calculation answers the open editor and the cascade answers
    // the save. If they ever disagree the ODO jumps on save (task 80).
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);
    let trip_b = db.get_trip(&b.to_string()).unwrap().unwrap();

    let preview = preview_trip_calculation_internal(
        &db, vehicle.id.to_string(), 2026, 80.0, None, true, None, Some(b.to_string()),
    )
    .unwrap();

    let (start, origin, destination, km, odo) = cascade_args(&trip_b, 80.0, trip_b.odometer);
    let plan = update_trip_cascade_internal(
        &db, &app_state, b.to_string(), start.clone(), start, origin, destination,
        km, odo, trip_b.purpose.clone(), None, None, None, None, None, None, None, None, None,
        true,
    )
    .unwrap()
    .plan;

    assert_eq!(preview.odometer, plan.new_odometer, "one arithmetic, two callers");
    let _ = a;
}

#[test]
fn test_update_trip_cascade_dry_run_rejects_a_negative_resulting_distance() {
    // Task 9b: the dry run fills the confirmation modal, so it must reject a
    // value that cannot be saved before any modal opens on it.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0); // anchor: year start 50000.0
    seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);

    let trip_a = db.get_trip(&a.to_string()).unwrap().unwrap();
    // km unchanged (50.0), odometer dropped below the year-start anchor.
    let (start, origin, destination, km, odo) = cascade_args(&trip_a, 50.0, 49900.0);
    let result = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), start.clone(), start, origin, destination,
        km, odo, trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        true,
    );

    assert!(result.is_err(), "a value that cannot be saved must not reach the modal");
}

// ============================================================================
// Task 78: what a distance write-back does to the consumption period
// ============================================================================

/// Two trips in one closed period: 100 km with no fill-up, then 100 km closing
/// on 12 litres. 200 km on 12 l is 6.0 l/100km, which against a 5.0 l/100km TP
/// rate is exactly the 20 % legal limit.
fn seed_closed_period(db: &Database, vehicle_id: Uuid) -> (Uuid, Uuid) {
    let a = seed_chain_trip(db, vehicle_id, 1, 100.0, 50100.0);
    let date = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
    let mut b = make_trip_detailed(date, 100.0, Some(12.0), true);
    b.vehicle_id = vehicle_id;
    b.odometer = 50200.0;
    db.create_trip(&b).unwrap();
    (a, b.id)
}

#[test]
fn test_period_margin_impact_moves_the_period_rate_and_the_margin() {
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Impact".to_string(), "BA1".to_string(), 60.0, 5.0, 50000.0);
    db.create_vehicle(&vehicle).unwrap();
    let (a, _b) = seed_closed_period(&db, vehicle.id);

    let trips = db.get_trips_for_vehicle_in_year(&vehicle.id.to_string(), 2026).unwrap();

    // Shorten the first trip from 100 km to 90 km: the period keeps its 12
    // litres but loses 10 km, so its rate goes up.
    let impact = period_margin_impact(&trips, 5.0, &a.to_string(), 90.0);

    assert!(impact.period_closed, "a full-tank fill-up closed this period");
    assert!((impact.rate_before - 6.0).abs() < 1e-9);
    assert!((impact.margin_before - 20.0).abs() < 1e-9);
    assert!(!impact.over_limit_before, "exactly 20 % is still legal");

    // 12 l over 190 km is 6.3158 l/100km, 26.3 % over a 5.0 TP rate.
    assert!((impact.rate_after - (1200.0 / 190.0)).abs() < 1e-9);
    assert!((impact.margin_after - 26.315_789_473_684_2).abs() < 1e-6);
    assert!(impact.over_limit_after, "the write crosses the 20 % legal limit");
}

#[test]
fn test_period_margin_impact_can_bring_a_period_back_under_the_limit() {
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Impact".to_string(), "BA1".to_string(), 60.0, 5.0, 50000.0);
    db.create_vehicle(&vehicle).unwrap();
    let (a, _b) = seed_closed_period(&db, vehicle.id);
    let trips = db.get_trips_for_vehicle_in_year(&vehicle.id.to_string(), 2026).unwrap();

    // Lengthen the first trip to 140 km: 240 km on 12 l is 5.0 l/100km, the TP
    // rate exactly, so the margin falls to zero.
    let impact = period_margin_impact(&trips, 5.0, &a.to_string(), 140.0);
    assert!((impact.rate_after - 5.0).abs() < 1e-9);
    assert!((impact.margin_after).abs() < 1e-9);
    assert!(!impact.over_limit_after);
}

#[test]
fn test_period_margin_impact_reports_an_open_period_as_open() {
    // No full-tank fill-up, so nothing closed. The period's rate is the TP
    // rate -- an estimate, not a measurement -- and the modal has to say so
    // rather than present it as a legal number.
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Impact".to_string(), "BA1".to_string(), 60.0, 5.0, 50000.0);
    db.create_vehicle(&vehicle).unwrap();
    let a = seed_chain_trip(&db, vehicle.id, 1, 100.0, 50100.0);
    let trips = db.get_trips_for_vehicle_in_year(&vehicle.id.to_string(), 2026).unwrap();

    let impact = period_margin_impact(&trips, 5.0, &a.to_string(), 90.0);
    assert!(!impact.period_closed);
    assert!((impact.rate_before - 5.0).abs() < 1e-9);
    assert!((impact.rate_after - 5.0).abs() < 1e-9);
}

#[test]
fn test_period_margin_impact_leaves_other_periods_alone() {
    // Period membership is decided by order and by the full_tank flag, never
    // by distance, so exactly one period's rate can move.
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Impact".to_string(), "BA1".to_string(), 60.0, 5.0, 50000.0);
    db.create_vehicle(&vehicle).unwrap();
    let (a, b) = seed_closed_period(&db, vehicle.id);

    let date = NaiveDate::from_ymd_opt(2026, 3, 3).unwrap();
    let mut c = make_trip_detailed(date, 100.0, Some(5.0), true);
    c.vehicle_id = vehicle.id;
    c.odometer = 50300.0;
    db.create_trip(&c).unwrap();

    let trips = db.get_trips_for_vehicle_in_year(&vehicle.id.to_string(), 2026).unwrap();

    let second = period_margin_impact(&trips, 5.0, &c.id.to_string(), 90.0);
    assert!((second.rate_before - 5.0).abs() < 1e-9, "100 km on 5 l");

    let first = period_margin_impact(&trips, 5.0, &a.to_string(), 90.0);
    assert!((first.rate_before - 6.0).abs() < 1e-9, "the first period is untouched");

    // b sits in the first period, so it reports the first period's numbers.
    let same_period = period_margin_impact(&trips, 5.0, &b.to_string(), 100.0);
    assert!((same_period.rate_before - 6.0).abs() < 1e-9);
}

#[test]
fn test_apply_route_distance_dry_run_writes_nothing() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);

    let result =
        apply_route_distance_internal(&db, &app_state, a.to_string(), 61.5, true).unwrap();

    assert!(result.trip.is_none(), "a dry run returns no saved trip");
    assert!((result.distance_before - 50.0).abs() < 1e-9);
    assert!((result.distance_after - 61.5).abs() < 1e-9);
    assert!((result.plan.delta - 11.5).abs() < 1e-9);
    assert_eq!(result.plan.changes.len(), 1, "the one row after it would move");

    // Nothing moved.
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().distance_km, 50.0);
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().odometer, 50120.0);
}

#[test]
fn test_apply_route_distance_writes_the_row_and_shifts_the_rest() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);
    let c = seed_chain_trip(&db, vehicle.id, 3, 30.0, 50150.0);

    apply_route_distance_internal(&db, &app_state, a.to_string(), 61.5, false).unwrap();

    let written = db.get_trip(&a.to_string()).unwrap().unwrap();
    assert!((written.distance_km - 61.5).abs() < 1e-9);
    // The odometer follows the distance: anchor (the year start) + km.
    assert!((written.odometer - 50061.5).abs() < 1e-9);
    // The invariant holds for the edited row and for every row after it.
    assert!((db.get_trip(&b.to_string()).unwrap().unwrap().odometer - 50131.5).abs() < 1e-9);
    assert!((db.get_trip(&c.to_string()).unwrap().unwrap().odometer - 50161.5).abs() < 1e-9);
}

#[test]
fn test_apply_route_distance_does_not_invent_an_end_time() {
    // `update_trip_cascade_internal` rebuilds a row from submitted strings and
    // writes `end_datetime: Some(...)` unconditionally. Routing that path here
    // would stamp an end time onto a trip that stored none, as a side effect
    // of applying a distance. This write touches three fields and no others.
    //
    // No stored row is affected TODAY: `create_trip`, `update_trip` and
    // `update_trip_cascade` all take `end_datetime: String`, so no caller can
    // write a NULL, and the production copy holds none (329 rows, 0 null,
    // measured 2026-09-09). This is a guard on the write-back path, not a
    // repair -- it keeps the property true before something else can create
    // such a row.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    assert!(db.get_trip(&a.to_string()).unwrap().unwrap().end_datetime.is_none());

    let before = db.get_trip(&a.to_string()).unwrap().unwrap();
    apply_route_distance_internal(&db, &app_state, a.to_string(), 61.5, false).unwrap();
    let after = db.get_trip(&a.to_string()).unwrap().unwrap();

    assert!(after.end_datetime.is_none(), "a trip with no end time keeps none");
    assert_eq!(after.start_datetime, before.start_datetime);
    assert_eq!(after.origin, before.origin);
    assert_eq!(after.destination, before.destination);
    assert_eq!(after.purpose, before.purpose);
    assert_eq!(after.fuel_liters, before.fuel_liters);
    assert_eq!(after.full_tank, before.full_tank);
    assert_eq!(after.created_at, before.created_at);
}

#[test]
fn test_apply_route_distance_reports_the_margin_it_would_cause() {
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Writeback".to_string(), "BA2".to_string(), 60.0, 5.0, 50000.0);
    db.create_vehicle(&vehicle).unwrap();
    let app_state = crate::app_state::AppState::new();
    let (a, _b) = seed_closed_period(&db, vehicle.id);

    let result =
        apply_route_distance_internal(&db, &app_state, a.to_string(), 90.0, true).unwrap();

    assert!(result.margin.period_closed);
    assert!(!result.margin.over_limit_before);
    assert!(
        result.margin.over_limit_after,
        "the warning must say the write crosses the 20 % limit"
    );
}

#[test]
fn test_apply_route_distance_is_read_only_guarded() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    app_state.enable_read_only("newer migrations");

    // Reading is always allowed, so the dry run still answers.
    assert!(apply_route_distance_internal(&db, &app_state, a.to_string(), 61.5, true).is_ok());
    // Writing is not.
    assert!(apply_route_distance_internal(&db, &app_state, a.to_string(), 61.5, false).is_err());
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().distance_km, 50.0);
}

#[test]
fn test_apply_route_distance_refuses_a_distance_that_is_not_one() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);

    assert!(apply_route_distance_internal(&db, &app_state, a.to_string(), -1.0, true).is_err());
    assert!(apply_route_distance_internal(&db, &app_state, a.to_string(), f64::NAN, true).is_err());
}

// ============================================================================
// Time inference (smart defaults for new trip rows) — Task 56
// ============================================================================

mod time_inference_tests {
    use super::*;
    use crate::calculations::time_inference::Jitter;
    use crate::commands_internal::trips::inferred_trip_time_for_route;

    /// Deterministic `Jitter` for assertion-friendly tests.
    struct StubJitter {
        minutes: i64,
        factor: f64,
    }
    impl Jitter for StubJitter {
        fn minutes(&mut self) -> i64 {
            self.minutes
        }
        fn duration_factor(&mut self) -> f64 {
            self.factor
        }
    }

    fn make_completed_trip(
        vehicle_id: Uuid,
        origin: &str,
        destination: &str,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> Trip {
        let now = Utc::now();
        Trip {
            id: Uuid::new_v4(),
            vehicle_id,
            start_datetime: start,
            end_datetime: Some(end),
            origin: origin.to_string(),
            destination: destination.to_string(),
            distance_km: 25.0,
            odometer: 10000.0,
            purpose: "test".to_string(),
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

    #[test]
    fn returns_none_when_no_match() {
        let db = Database::in_memory().unwrap();
        let vehicle = crate::models::Vehicle::new(
            "Test".into(),
            "BA1".into(),
            50.0,
            6.0,
            0.0,
        );
        db.create_vehicle(&vehicle).unwrap();

        let mut j = StubJitter { minutes: 0, factor: 1.0 };
        let row_date = NaiveDate::from_ymd_opt(2026, 4, 15).unwrap();

        let result = inferred_trip_time_for_route(
            &db,
            &mut j,
            &vehicle.id.to_string(),
            "Bratislava",
            "Trnava",
            row_date,
        )
        .unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn uses_most_recent_match() {
        let db = Database::in_memory().unwrap();
        let vehicle = crate::models::Vehicle::new(
            "Test".into(),
            "BA1".into(),
            50.0,
            6.0,
            0.0,
        );
        db.create_vehicle(&vehicle).unwrap();

        // Older trip: 6:00–7:00
        let older = make_completed_trip(
            vehicle.id,
            "Bratislava",
            "Trnava",
            NaiveDate::from_ymd_opt(2026, 1, 10).unwrap().and_hms_opt(6, 0, 0).unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 10).unwrap().and_hms_opt(7, 0, 0).unwrap(),
        );
        db.create_trip(&older).unwrap();

        // Newer trip: 8:30–9:15 (45 min duration)
        let newer = make_completed_trip(
            vehicle.id,
            "Bratislava",
            "Trnava",
            NaiveDate::from_ymd_opt(2026, 3, 20).unwrap().and_hms_opt(8, 30, 0).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 20).unwrap().and_hms_opt(9, 15, 0).unwrap(),
        );
        db.create_trip(&newer).unwrap();

        // Zero jitter → result equals newer trip's HH:MM applied to row_date.
        let mut j = StubJitter { minutes: 0, factor: 1.0 };
        let row_date = NaiveDate::from_ymd_opt(2026, 4, 15).unwrap();

        let result = inferred_trip_time_for_route(
            &db,
            &mut j,
            &vehicle.id.to_string(),
            "Bratislava",
            "Trnava",
            row_date,
        )
        .unwrap()
        .expect("should match newer trip");

        assert_eq!(result.start_datetime, "2026-04-15T08:30:00");
        assert_eq!(result.end_datetime, "2026-04-15T09:15:00");
    }

    #[test]
    fn ignores_trips_with_null_end_datetime() {
        let db = Database::in_memory().unwrap();
        let vehicle = crate::models::Vehicle::new(
            "Test".into(),
            "BA1".into(),
            50.0,
            6.0,
            0.0,
        );
        db.create_vehicle(&vehicle).unwrap();

        // Only trip on this route has no end_datetime.
        let now = Utc::now();
        let open_trip = Trip {
            id: Uuid::new_v4(),
            vehicle_id: vehicle.id,
            start_datetime: NaiveDate::from_ymd_opt(2026, 3, 20).unwrap().and_hms_opt(8, 0, 0).unwrap(),
            end_datetime: None,
            origin: "Bratislava".into(),
            destination: "Trnava".into(),
            distance_km: 25.0,
            odometer: 10000.0,
            purpose: "test".into(),
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
        };
        db.create_trip(&open_trip).unwrap();

        let mut j = StubJitter { minutes: 0, factor: 1.0 };
        let row_date = NaiveDate::from_ymd_opt(2026, 4, 15).unwrap();

        let result = inferred_trip_time_for_route(
            &db,
            &mut j,
            &vehicle.id.to_string(),
            "Bratislava",
            "Trnava",
            row_date,
        )
        .unwrap();

        assert!(result.is_none(), "open trips must not be considered");
    }

    #[test]
    fn scoped_to_vehicle() {
        let db = Database::in_memory().unwrap();
        let v1 = crate::models::Vehicle::new("V1".into(), "BA1".into(), 50.0, 6.0, 0.0);
        let v2 = crate::models::Vehicle::new("V2".into(), "BA2".into(), 50.0, 6.0, 0.0);
        db.create_vehicle(&v1).unwrap();
        db.create_vehicle(&v2).unwrap();

        // Match exists on v2's history but not v1's.
        let trip = make_completed_trip(
            v2.id,
            "Bratislava",
            "Trnava",
            NaiveDate::from_ymd_opt(2026, 3, 20).unwrap().and_hms_opt(8, 30, 0).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 20).unwrap().and_hms_opt(9, 15, 0).unwrap(),
        );
        db.create_trip(&trip).unwrap();

        let mut j = StubJitter { minutes: 0, factor: 1.0 };
        let row_date = NaiveDate::from_ymd_opt(2026, 4, 15).unwrap();

        let result = inferred_trip_time_for_route(
            &db,
            &mut j,
            &v1.id.to_string(),
            "Bratislava",
            "Trnava",
            row_date,
        )
        .unwrap();

        assert!(result.is_none(), "v2's trips must not leak to v1");
    }

    // ------------------------------------------------------------------------
    // Task 5: Public command gates on `infer_trip_times` setting.
    // The pure seam `inferred_trip_time_for_route` is unaffected; these tests
    // exercise the gated `_internal` boundary which now takes `&Path`.
    // ------------------------------------------------------------------------

    /// Seeds an in-memory DB with one completed trip on Bratislava→Trnava
    /// (matching the existing time-inference fixtures in this module).
    fn test_db_with_completed_trip() -> (Database, String) {
        let db = Database::in_memory().unwrap();
        let vehicle = crate::models::Vehicle::new(
            "Test".into(),
            "BA1".into(),
            50.0,
            6.0,
            0.0,
        );
        db.create_vehicle(&vehicle).unwrap();

        let trip = make_completed_trip(
            vehicle.id,
            "Bratislava",
            "Trnava",
            NaiveDate::from_ymd_opt(2026, 3, 20).unwrap().and_hms_opt(8, 30, 0).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 20).unwrap().and_hms_opt(9, 15, 0).unwrap(),
        );
        db.create_trip(&trip).unwrap();
        (db, vehicle.id.to_string())
    }

    #[test]
    fn inference_command_returns_none_when_setting_unset() {
        use crate::commands_internal::trips::get_inferred_trip_time_for_route_internal;
        let (db, vehicle_id) = test_db_with_completed_trip();
        let dir = tempfile::tempdir().unwrap();
        let app_dir = dir.path().to_path_buf();
        // No infer_trip_times in local.settings.json → default OFF.

        let result = get_inferred_trip_time_for_route_internal(
            &db,
            &app_dir,
            vehicle_id,
            "Bratislava".to_string(),
            "Trnava".to_string(),
            "2026-04-27".to_string(),
        )
        .unwrap();

        assert!(result.is_none(), "default-OFF must short-circuit to None");
    }

    #[test]
    fn inference_command_returns_none_when_setting_disabled() {
        use crate::commands_internal::settings_cmd::set_infer_trip_times_internal;
        use crate::commands_internal::trips::get_inferred_trip_time_for_route_internal;
        let (db, vehicle_id) = test_db_with_completed_trip();
        let dir = tempfile::tempdir().unwrap();
        let app_dir = dir.path().to_path_buf();
        set_infer_trip_times_internal(&app_dir, false).unwrap();

        let result = get_inferred_trip_time_for_route_internal(
            &db,
            &app_dir,
            vehicle_id,
            "Bratislava".to_string(),
            "Trnava".to_string(),
            "2026-04-27".to_string(),
        )
        .unwrap();

        assert!(result.is_none(), "Some(false) must short-circuit to None");
    }

    #[test]
    fn inference_command_returns_some_when_setting_enabled() {
        use crate::commands_internal::settings_cmd::set_infer_trip_times_internal;
        use crate::commands_internal::trips::get_inferred_trip_time_for_route_internal;
        let (db, vehicle_id) = test_db_with_completed_trip();
        let dir = tempfile::tempdir().unwrap();
        let app_dir = dir.path().to_path_buf();
        set_infer_trip_times_internal(&app_dir, true).unwrap();

        let result = get_inferred_trip_time_for_route_internal(
            &db,
            &app_dir,
            vehicle_id,
            "Bratislava".to_string(),
            "Trnava".to_string(),
            "2026-04-27".to_string(),
        )
        .unwrap();

        assert!(result.is_some(), "Some(true) must allow inference to run");
    }

    // ------------------------------------------------------------------------
    // Task 71: copy-a-trip defaults. The rule itself is covered exhaustively in
    // calculations::trip_copy; only the wrapper's DB error branch lives here.
    // ------------------------------------------------------------------------

    #[test]
    fn copy_defaults_errors_when_trip_is_missing() {
        use crate::commands_internal::trips::get_copied_trip_defaults_internal;
        let (db, _vehicle_id) = test_db_with_completed_trip();

        let result = get_copied_trip_defaults_internal(&db, Uuid::new_v4().to_string(), 2026);

        assert!(result.is_err(), "an unknown trip id must not silently succeed");
    }
}
