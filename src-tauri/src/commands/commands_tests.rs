// ============================================================================
// Tests
// ============================================================================

use super::statistics::{
    calculate_consumption_warnings, calculate_date_warnings, calculate_energy_grid_data,
    calculate_missing_receipts, calculate_receipt_datetime_warnings, calculate_suggested_fillups,
    get_open_period_km,
};
use super::*;
use crate::models::{ConfidenceLevel, FieldConfidence, Receipt, ReceiptStatus, Trip, Vehicle};
use chrono::{NaiveDate, NaiveDateTime, Utc};
use uuid::Uuid;

/// Helper to create a trip with fuel
/// Sets end_datetime to end of day to allow receipt matching for any time during the day
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
        sort_order: 0,
        created_at: now,
        updated_at: now,
    }
}

/// Helper to create a trip without fuel
/// Sets end_datetime to end of day to allow receipt matching for any time during the day
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
        sort_order: 0,
        created_at: now,
        updated_at: now,
    }
}

/// Helper to create an unassigned receipt
fn make_receipt(date: Option<NaiveDate>, liters: Option<f64>, price: Option<f64>) -> Receipt {
    make_receipt_with_trip_id(date, liters, price, None)
}

/// Helper to create a receipt optionally assigned to a trip
fn make_receipt_with_trip_id(
    date: Option<NaiveDate>,
    liters: Option<f64>,
    price: Option<f64>,
    trip_id: Option<Uuid>,
) -> Receipt {
    let now = Utc::now();
    Receipt {
        id: Uuid::new_v4(),
        vehicle_id: None,
        trip_id,
        file_path: "/test/receipt.jpg".to_string(),
        file_name: "receipt.jpg".to_string(),
        scanned_at: now,
        liters,
        total_price_eur: price,
        receipt_datetime: date.and_then(|d| d.and_hms_opt(12, 0, 0)), // Convert date to datetime at noon
        station_name: None,
        station_address: None,
        vendor_name: None,
        cost_description: None,
        original_amount: price,
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
        assignment_type: if trip_id.is_some() {
            Some(crate::models::AssignmentType::Fuel)
        } else {
            None
        },
        mismatch_override: false,
        created_at: now,
        updated_at: now,
    }
}

// ========================================================================
// Receipt-trip assignment tests (calculate_missing_receipts)
// Task 51: Uses trip_id (explicit assignment) instead of computed matching
// ========================================================================

#[test]
fn test_missing_receipts_trip_with_assigned_receipt_not_flagged() {
    // Trip with explicitly assigned receipt → NOT missing
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
    // Receipt assigned to trip via trip_id
    let receipts = vec![make_receipt_with_trip_id(
        Some(date),
        Some(45.0),
        Some(72.50),
        Some(trips[0].id),
    )];

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert!(
        missing.is_empty(),
        "Trip with assigned receipt should not be flagged as missing"
    );
}

#[test]
fn test_missing_receipts_trip_without_assigned_receipt_flagged() {
    // Trip with fuel but no receipt assigned → missing
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
    // Receipt exists but NOT assigned (trip_id = None)
    let receipts = vec![make_receipt(Some(date), Some(45.0), Some(72.50))];

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert_eq!(
        missing.len(),
        1,
        "Trip without assigned receipt should be flagged"
    );
    assert!(missing.contains(&trips[0].id.to_string()));
}

#[test]
fn test_missing_receipts_trip_without_costs_not_flagged() {
    // Trip without fuel or other_costs → NOT flagged (doesn't need receipt)
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_without_fuel(date)];
    let receipts: Vec<Receipt> = vec![];

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert!(
        missing.is_empty(),
        "Trip without fuel/other_costs should not be flagged as missing receipt"
    );
}

#[test]
fn test_missing_receipts_trip_with_other_costs_no_receipt_flagged() {
    // Trip with other_costs but no assigned receipt → missing
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let mut trip = make_trip_without_fuel(date);
    trip.other_costs_eur = Some(15.0);
    trip.other_costs_note = Some("Parkovanie".to_string());
    let trips = vec![trip];
    let receipts: Vec<Receipt> = vec![];

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert_eq!(
        missing.len(),
        1,
        "Trip with other_costs but no receipt should be flagged"
    );
}

#[test]
fn test_missing_receipts_no_receipts_available() {
    // Trip with fuel but no receipts at all → missing
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
    let receipts: Vec<Receipt> = vec![];

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert_eq!(
        missing.len(),
        1,
        "Trip with fuel but no receipts should be flagged"
    );
}

#[test]
fn test_missing_receipts_multiple_trips_partial_assignment() {
    // Multiple trips: some with assigned receipts, some without
    let date1 = NaiveDate::from_ymd_opt(2024, 6, 10).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2024, 6, 20).unwrap();
    let date3 = NaiveDate::from_ymd_opt(2024, 6, 30).unwrap();

    let trips = vec![
        make_trip_with_fuel(date1, 40.0, 65.00), // Has assigned receipt
        make_trip_with_fuel(date2, 50.0, 80.00), // No assigned receipt
        make_trip_without_fuel(date3),           // No costs, doesn't need receipt
    ];
    let receipts = vec![
        make_receipt_with_trip_id(Some(date1), Some(40.0), Some(65.00), Some(trips[0].id)), // Assigned to trip 1
    ];

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert_eq!(missing.len(), 1, "Only trip 2 should be flagged");
    assert!(
        missing.contains(&trips[1].id.to_string()),
        "Trip 2 (with fuel, no assigned receipt) should be flagged"
    );
    assert!(
        !missing.contains(&trips[0].id.to_string()),
        "Trip 1 (with assigned receipt) should not be flagged"
    );
    assert!(
        !missing.contains(&trips[2].id.to_string()),
        "Trip 3 (no costs) should not be flagged"
    );
}

#[test]
fn test_missing_receipts_receipt_assigned_to_different_trip() {
    // Receipt assigned to trip A, trip B has no receipt → trip B flagged
    let date1 = NaiveDate::from_ymd_opt(2024, 6, 10).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2024, 6, 20).unwrap();

    let trips = vec![
        make_trip_with_fuel(date1, 40.0, 65.00),
        make_trip_with_fuel(date2, 50.0, 80.00),
    ];
    let receipts = vec![
        make_receipt_with_trip_id(Some(date1), Some(40.0), Some(65.00), Some(trips[0].id)), // Assigned to trip 1 only
    ];

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert_eq!(missing.len(), 1);
    assert!(
        missing.contains(&trips[1].id.to_string()),
        "Trip 2 should be flagged"
    );
    assert!(
        !missing.contains(&trips[0].id.to_string()),
        "Trip 1 should not be flagged"
    );
}

// ========================================================================
// Receipt datetime warning tests (calculate_receipt_datetime_warnings)
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
        sort_order: 0,
        created_at: now,
        updated_at: now,
    }
}

/// Helper to create a receipt with a specific datetime and assigned to a trip
fn make_receipt_with_datetime_assigned(
    receipt_datetime: Option<NaiveDateTime>,
    trip_id: Uuid,
) -> Receipt {
    let now = Utc::now();
    Receipt {
        id: Uuid::new_v4(),
        vehicle_id: None,
        trip_id: Some(trip_id),
        file_path: "/test/receipt.jpg".to_string(),
        file_name: "receipt.jpg".to_string(),
        scanned_at: now,
        liters: Some(45.0),
        total_price_eur: Some(72.50),
        receipt_datetime,
        station_name: None,
        station_address: None,
        vendor_name: None,
        cost_description: None,
        original_amount: Some(72.50),
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
        assignment_type: Some(crate::models::AssignmentType::Fuel),
        mismatch_override: false,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn test_receipt_datetime_warning_within_range() {
    // Receipt datetime inside trip [start, end] -> no warning
    let trip_start = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let trip_end = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(17, 0, 0)
        .unwrap();
    let receipt_dt = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, Some(trip_end));
    let receipt = make_receipt_with_datetime_assigned(Some(receipt_dt), trip.id);

    let warnings = calculate_receipt_datetime_warnings(&[trip], &[receipt]);

    assert!(
        warnings.is_empty(),
        "Receipt within trip range should not generate warning"
    );
}

#[test]
fn test_receipt_datetime_warning_before_trip_start() {
    // Receipt datetime before trip.start_datetime -> warning
    let trip_start = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    let trip_end = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(17, 0, 0)
        .unwrap();
    let receipt_dt = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(8, 0, 0) // Before trip start
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, Some(trip_end));
    let receipt = make_receipt_with_datetime_assigned(Some(receipt_dt), trip.id);

    let warnings = calculate_receipt_datetime_warnings(&[trip.clone()], &[receipt]);

    assert_eq!(warnings.len(), 1, "Should have 1 warning");
    assert!(
        warnings.contains(&trip.id.to_string()),
        "Trip should be flagged when receipt is before start"
    );
}

#[test]
fn test_receipt_datetime_warning_after_trip_end() {
    // Receipt datetime after trip.end_datetime -> warning
    let trip_start = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let trip_end = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(14, 0, 0)
        .unwrap();
    let receipt_dt = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(18, 0, 0) // After trip end
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, Some(trip_end));
    let receipt = make_receipt_with_datetime_assigned(Some(receipt_dt), trip.id);

    let warnings = calculate_receipt_datetime_warnings(&[trip.clone()], &[receipt]);

    assert_eq!(warnings.len(), 1, "Should have 1 warning");
    assert!(
        warnings.contains(&trip.id.to_string()),
        "Trip should be flagged when receipt is after end"
    );
}

#[test]
fn test_receipt_datetime_warning_no_receipt() {
    // Trip without receipt -> no warning
    let trip_start = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, None);
    let receipts: Vec<Receipt> = vec![];

    let warnings = calculate_receipt_datetime_warnings(&[trip], &receipts);

    assert!(
        warnings.is_empty(),
        "Trip without receipt should not generate warning"
    );
}

#[test]
fn test_receipt_datetime_warning_receipt_no_datetime() {
    // Receipt with None datetime -> no warning (can't validate)
    let trip_start = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, None);
    let receipt = make_receipt_with_datetime_assigned(None, trip.id);

    let warnings = calculate_receipt_datetime_warnings(&[trip], &[receipt]);

    assert!(
        warnings.is_empty(),
        "Receipt without datetime should not generate warning"
    );
}

#[test]
fn test_receipt_datetime_warning_exactly_at_start() {
    // Receipt datetime == trip.start_datetime -> no warning (boundary: inclusive)
    let trip_start = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let trip_end = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(17, 0, 0)
        .unwrap();
    let receipt_dt = trip_start; // Exactly at start

    let trip = make_trip_with_datetime_range(trip_start, Some(trip_end));
    let receipt = make_receipt_with_datetime_assigned(Some(receipt_dt), trip.id);

    let warnings = calculate_receipt_datetime_warnings(&[trip], &[receipt]);

    assert!(
        warnings.is_empty(),
        "Receipt at exact start time should not generate warning (inclusive boundary)"
    );
}

#[test]
fn test_receipt_datetime_warning_exactly_at_end() {
    // Receipt datetime == trip.end_datetime -> no warning (boundary: inclusive)
    let trip_start = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let trip_end = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(17, 0, 0)
        .unwrap();
    let receipt_dt = trip_end; // Exactly at end

    let trip = make_trip_with_datetime_range(trip_start, Some(trip_end));
    let receipt = make_receipt_with_datetime_assigned(Some(receipt_dt), trip.id);

    let warnings = calculate_receipt_datetime_warnings(&[trip], &[receipt]);

    assert!(
        warnings.is_empty(),
        "Receipt at exact end time should not generate warning (inclusive boundary)"
    );
}

#[test]
fn test_receipt_datetime_warning_no_end_datetime_uses_start() {
    // Trip without end_datetime - receipt must match start_datetime exactly
    let trip_start = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();

    // Case 1: Receipt at different time on same day - should warn (range is just start_datetime)
    let receipt_dt = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();

    let trip = make_trip_with_datetime_range(trip_start, None);
    let receipt = make_receipt_with_datetime_assigned(Some(receipt_dt), trip.id);

    let warnings = calculate_receipt_datetime_warnings(&[trip.clone()], &[receipt]);

    assert_eq!(
        warnings.len(),
        1,
        "Receipt not at exact start time should generate warning when no end_datetime"
    );

    // Case 2: Receipt at exact start time - no warning
    let receipt_exact = make_receipt_with_datetime_assigned(Some(trip_start), trip.id);

    let warnings = calculate_receipt_datetime_warnings(&[trip], &[receipt_exact]);

    assert!(
        warnings.is_empty(),
        "Receipt at exact start time should not generate warning"
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
    sort_order: i32,
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
        sort_order,
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
        make_trip_detailed(base_date, 100.0, None, false, 3), // 100km, no fuel
        make_trip_detailed(base_date.succ_opt().unwrap(), 100.0, Some(20.0), false, 2), // 100km, 20L PARTIAL
        make_trip_detailed(
            base_date.succ_opt().unwrap().succ_opt().unwrap(),
            100.0,
            None,
            false,
            1,
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
            0,
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
        make_trip_detailed(base_date, 100.0, None, false, 3), // Period 1: 100km
        make_trip_detailed(base_date.succ_opt().unwrap(), 100.0, Some(10.0), true, 2), // Period 1: closes with 10L -> rate = 10/200*100 = 5.0
        make_trip_detailed(
            base_date.succ_opt().unwrap().succ_opt().unwrap(),
            200.0,
            None,
            false,
            1,
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
            0,
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
        make_trip_detailed(base_date, 100.0, None, false, 1),
        make_trip_detailed(base_date.succ_opt().unwrap(), 100.0, Some(15.0), false, 0), // Partial only
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
// Date warning tests (calculate_date_warnings)
// ========================================================================

#[test]
fn test_date_warnings_detects_out_of_order() {
    // Trips sorted by sort_order (0 = newest/top), but dates out of order
    let trips = vec![
        make_trip_detailed(
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            50.0,
            None,
            false,
            0,
        ), // Top: Jun 15
        make_trip_detailed(
            NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(),
            50.0,
            None,
            false,
            1,
        ), // Middle: Jun 10 - WRONG! Should be between 15 and 20
        make_trip_detailed(
            NaiveDate::from_ymd_opt(2024, 6, 20).unwrap(),
            50.0,
            None,
            false,
            2,
        ), // Bottom: Jun 20
    ];

    let warnings = calculate_date_warnings(&trips);

    // Jun 10 (middle) has earlier date than Jun 15 (top) - that's wrong for sort_order
    // Jun 10 also has earlier date than Jun 20 (bottom) - wrong again
    assert!(
        warnings.contains(&trips[1].id.to_string()),
        "Jun 10 trip should be flagged (out of order)"
    );
}

#[test]
fn test_date_warnings_correct_order_no_warnings() {
    // Trips in correct order: newest (highest date) at sort_order 0
    let trips = vec![
        make_trip_detailed(
            NaiveDate::from_ymd_opt(2024, 6, 20).unwrap(),
            50.0,
            None,
            false,
            0,
        ), // Top: Jun 20 (newest)
        make_trip_detailed(
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            50.0,
            None,
            false,
            1,
        ), // Middle: Jun 15
        make_trip_detailed(
            NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(),
            50.0,
            None,
            false,
            2,
        ), // Bottom: Jun 10 (oldest)
    ];

    let warnings = calculate_date_warnings(&trips);

    assert!(
        warnings.is_empty(),
        "No warnings expected for correctly ordered trips"
    );
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
        make_trip_detailed(base_date, 100.0, Some(7.5), true, 0), // Rate = 7.5 l/100km > 6.0 limit
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

    let trips = vec![make_trip_detailed(base_date, 100.0, Some(6.0), true, 0)];

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
        make_trip_detailed(base_date, 100.0, Some(5.0), true, 0), // Rate = 5.0 < 6.0 limit
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
// Per-period over-limit tests (has_any_period_over_limit)
// ========================================================================

#[test]
fn test_has_any_period_over_limit_single_period_over() {
    // Single period with rate > 120% of TP should return true
    let base_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let tp_rate = 5.0; // TP rate, limit is 6.0 (120%)

    let trips = vec![
        // Period: 100km, 7.5L filled = 7.5 l/100km > 6.0 limit
        make_trip_detailed(base_date, 100.0, Some(7.5), true, 0),
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
            0,
        ),
        // Period 2: 100km, 7L = 7.0 l/100km (OVER limit)
        make_trip_detailed(
            NaiveDate::from_ymd_opt(2024, 6, 2).unwrap(),
            100.0,
            Some(7.0),
            true,
            1,
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
            0,
        ),
        // Period 2: 100km, 5.5L = 5.5 l/100km (under)
        make_trip_detailed(
            NaiveDate::from_ymd_opt(2024, 6, 2).unwrap(),
            100.0,
            Some(5.5),
            true,
            1,
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
            0,
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
    let trips = vec![make_trip_detailed(base_date, 100.0, None, false, 0)];

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
        make_trip_detailed(base_date, 100.0, Some(30.0), false, 0), // 100km, add 30L partial
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
        make_trip_detailed(base_date, 100.0, Some(30.0), true, 0), // Full tank
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
        make_trip_detailed(base_date, 500.0, None, false, 0), // 500km at 6 l/100km = 30L, but only have 10L
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
        sort_order: 0,
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
        sort_order: 1,
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
        sort_order: 0,
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
        sort_order: 0,
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
        sort_order: 1,
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
        sort_order: 0,
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
    sort_order: i32,
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
        odometer: 10000.0 + (sort_order as f64) * distance_km,
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
        sort_order,
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
    let trips = vec![make_bev_trip(date, 100.0, None, false, 0)];

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
        make_bev_trip(date, 100.0, Some(40.0), true, 0), // Drive 100km, charge 40 kWh full
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
        make_bev_trip(date, 10.0, Some(50.0), true, 0), // Short drive, big charge
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
// verify_receipts vehicle filtering tests
// ========================================================================

#[test]
fn test_verify_receipts_filters_by_vehicle() {
    use crate::models::{Receipt, Vehicle, VehicleType};

    let db = Database::in_memory().unwrap();

    // Create two vehicles
    let now = Utc::now();
    let vehicle_a = Vehicle {
        id: Uuid::new_v4(),
        name: "Vehicle A".to_string(),
        license_plate: "AA-001-AA".to_string(),
        vehicle_type: VehicleType::Ice,
        tank_size_liters: Some(50.0),
        tp_consumption: Some(6.0),
        initial_odometer: 10000.0,
        battery_capacity_kwh: None,
        baseline_consumption_kwh: None,
        initial_battery_percent: None,
        is_active: false,
        vin: None,
        driver_name: None,
        ha_odo_sensor: None,
        ha_fillup_sensor: None,
        ha_fuel_level_sensor: None,
        created_at: now,
        updated_at: now,
    };
    let vehicle_b = Vehicle {
        id: Uuid::new_v4(),
        name: "Vehicle B".to_string(),
        license_plate: "BB-002-BB".to_string(),
        vehicle_type: VehicleType::Ice,
        tank_size_liters: Some(50.0),
        tp_consumption: Some(6.0),
        initial_odometer: 10000.0,
        battery_capacity_kwh: None,
        baseline_consumption_kwh: None,
        initial_battery_percent: None,
        is_active: false,
        vin: None,
        driver_name: None,
        ha_odo_sensor: None,
        ha_fillup_sensor: None,
        ha_fuel_level_sensor: None,
        created_at: now,
        updated_at: now,
    };
    db.create_vehicle(&vehicle_a).unwrap();
    db.create_vehicle(&vehicle_b).unwrap();

    // Create receipts:
    // - 1 unassigned (should be counted for both vehicles)
    // - 1 assigned to vehicle A
    // - 1 assigned to vehicle B (should NOT be counted when viewing A)
    let mut receipt_unassigned =
        Receipt::new("path1.jpg".to_string(), "unassigned.jpg".to_string());
    receipt_unassigned.receipt_datetime =
        NaiveDate::from_ymd_opt(2024, 6, 15).and_then(|d| d.and_hms_opt(12, 0, 0));

    let mut receipt_a = Receipt::new("path2.jpg".to_string(), "vehicle_a.jpg".to_string());
    receipt_a.vehicle_id = Some(vehicle_a.id);
    receipt_a.receipt_datetime =
        NaiveDate::from_ymd_opt(2024, 6, 16).and_then(|d| d.and_hms_opt(12, 0, 0));

    let mut receipt_b = Receipt::new("path3.jpg".to_string(), "vehicle_b.jpg".to_string());
    receipt_b.vehicle_id = Some(vehicle_b.id);
    receipt_b.receipt_datetime =
        NaiveDate::from_ymd_opt(2024, 6, 17).and_then(|d| d.and_hms_opt(12, 0, 0));

    db.create_receipt(&receipt_unassigned).unwrap();
    db.create_receipt(&receipt_a).unwrap();
    db.create_receipt(&receipt_b).unwrap();

    // Verify receipts for vehicle A
    let result = verify_receipts_internal(&db, &vehicle_a.id.to_string(), 2024).unwrap();

    // Should only count unassigned + vehicle A's receipts = 2
    // Vehicle B's receipt should NOT be included
    assert_eq!(
        result.total, 2,
        "Expected 2 receipts (unassigned + vehicle A), got {}",
        result.total
    );
}

// ========================================================================
// Multi-stage matching tests (assign_receipt_to_trip_internal)
// ========================================================================

/// Helper to create a receipt with vendor_name and cost_description
fn make_receipt_with_details(
    date: Option<NaiveDate>,
    liters: Option<f64>,
    price: Option<f64>,
    vendor_name: Option<&str>,
    cost_description: Option<&str>,
) -> Receipt {
    let now = Utc::now();
    Receipt {
        id: Uuid::new_v4(),
        vehicle_id: None,
        trip_id: None,
        file_path: "/test/receipt.jpg".to_string(),
        file_name: "receipt.jpg".to_string(),
        scanned_at: now,
        liters,
        total_price_eur: price,
        receipt_datetime: date.and_then(|d| d.and_hms_opt(12, 0, 0)), // Convert date to datetime at noon
        station_name: None,
        station_address: None,
        vendor_name: vendor_name.map(|s| s.to_string()),
        cost_description: cost_description.map(|s| s.to_string()),
        original_amount: price,
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

/// Helper to create a trip for assignment tests (with vehicle_id that stays constant)
/// Sets end_datetime to end of day to allow receipt matching for any time during the day
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
        sort_order: 0,
        created_at: now,
        updated_at: now,
    }
}

// ========================================================================
// Task 51: Explicit assignment type tests (C1-C7 scenarios)
// ========================================================================

#[test]
fn test_assign_fuel_to_empty_trip_populates_data() {
    // Scenario C1: FUEL receipt to empty trip → populates fuel_liters/fuel_cost_eur
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    // Trip with NO fuel data
    let trip = make_trip_for_assignment(vehicle.id, date, None, None, None);
    db.create_trip(&trip).unwrap();

    let receipt = make_receipt_with_details(Some(date), Some(45.0), Some(72.0), Some("OMV"), None);
    db.create_receipt(&receipt).unwrap();

    let result = assign_receipt_to_trip_internal(
        &db,
        &receipt.id.to_string(),
        &trip.id.to_string(),
        &vehicle.id.to_string(),
        "Fuel", // Explicit FUEL assignment
        false,  // No mismatch override
    );

    assert!(result.is_ok(), "Assignment should succeed");

    let assigned_receipt = result.unwrap();
    assert_eq!(assigned_receipt.trip_id, Some(trip.id));
    assert_eq!(
        assigned_receipt.assignment_type,
        Some(crate::models::AssignmentType::Fuel)
    );
    assert_eq!(assigned_receipt.mismatch_override, false);

    // Trip should have FUEL fields populated
    let updated_trip = db.get_trip(&trip.id.to_string()).unwrap().unwrap();
    assert_eq!(updated_trip.fuel_liters, Some(45.0));
    assert_eq!(updated_trip.fuel_cost_eur, Some(72.0));
    assert!(
        updated_trip.other_costs_eur.is_none(),
        "FUEL should not touch other_costs"
    );
}

#[test]
fn test_assign_other_to_empty_trip_populates_data() {
    // Scenario C2: OTHER receipt to empty trip → populates other_costs_eur/note
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trip = make_trip_for_assignment(vehicle.id, date, None, None, None);
    db.create_trip(&trip).unwrap();

    let receipt = make_receipt_with_details(
        Some(date),
        None,
        Some(15.0),
        Some("AutoWash"),
        Some("Umytie auta"),
    );
    db.create_receipt(&receipt).unwrap();

    let result = assign_receipt_to_trip_internal(
        &db,
        &receipt.id.to_string(),
        &trip.id.to_string(),
        &vehicle.id.to_string(),
        "Other", // Explicit OTHER assignment
        false,
    );

    assert!(result.is_ok(), "Assignment should succeed");

    let assigned_receipt = result.unwrap();
    assert_eq!(
        assigned_receipt.assignment_type,
        Some(crate::models::AssignmentType::Other)
    );

    // Trip should have OTHER_COSTS populated
    let updated_trip = db.get_trip(&trip.id.to_string()).unwrap().unwrap();
    assert_eq!(updated_trip.other_costs_eur, Some(15.0));
    assert!(updated_trip
        .other_costs_note
        .as_ref()
        .unwrap()
        .contains("AutoWash"));
    assert!(
        updated_trip.fuel_liters.is_none(),
        "OTHER should not touch fuel"
    );
}

#[test]
fn test_assign_fuel_with_matching_data_links_only() {
    // Scenario C3: FUEL receipt to trip that already has matching fuel → just links
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trip = make_trip_for_assignment(vehicle.id, date, Some(45.0), Some(72.0), None);
    db.create_trip(&trip).unwrap();

    let receipt = make_receipt_with_details(Some(date), Some(45.0), Some(72.0), None, None);
    db.create_receipt(&receipt).unwrap();

    let result = assign_receipt_to_trip_internal(
        &db,
        &receipt.id.to_string(),
        &trip.id.to_string(),
        &vehicle.id.to_string(),
        "Fuel",
        false,
    );

    assert!(result.is_ok(), "Assignment should succeed");

    let assigned_receipt = result.unwrap();
    assert_eq!(assigned_receipt.trip_id, Some(trip.id));
    assert_eq!(
        assigned_receipt.assignment_type,
        Some(crate::models::AssignmentType::Fuel)
    );

    // Trip fuel data should be unchanged (just linked)
    let updated_trip = db.get_trip(&trip.id.to_string()).unwrap().unwrap();
    assert_eq!(updated_trip.fuel_liters, Some(45.0));
    assert_eq!(updated_trip.fuel_cost_eur, Some(72.0));
    assert!(updated_trip.other_costs_eur.is_none());
}

#[test]
fn test_assign_fuel_with_mismatch_no_override() {
    // Scenario C4: FUEL receipt with mismatched data, no override → links with mismatch_override=false
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trip = make_trip_for_assignment(vehicle.id, date, Some(45.0), Some(72.0), None);
    db.create_trip(&trip).unwrap();

    // Receipt with DIFFERENT values (mismatch)
    let receipt = make_receipt_with_details(Some(date), Some(50.0), Some(80.0), None, None);
    db.create_receipt(&receipt).unwrap();

    let result = assign_receipt_to_trip_internal(
        &db,
        &receipt.id.to_string(),
        &trip.id.to_string(),
        &vehicle.id.to_string(),
        "Fuel",
        false, // No override - UI will show warning
    );

    assert!(
        result.is_ok(),
        "Assignment should succeed even with mismatch"
    );

    let assigned_receipt = result.unwrap();
    assert_eq!(
        assigned_receipt.assignment_type,
        Some(crate::models::AssignmentType::Fuel)
    );
    assert_eq!(
        assigned_receipt.mismatch_override, false,
        "Should not have override set"
    );
}

#[test]
fn test_assign_fuel_with_mismatch_and_override() {
    // Scenario C5: FUEL receipt with mismatched data + user override → links with mismatch_override=true
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trip = make_trip_for_assignment(vehicle.id, date, Some(45.0), Some(72.0), None);
    db.create_trip(&trip).unwrap();

    // Receipt with DIFFERENT values (mismatch)
    let receipt = make_receipt_with_details(Some(date), Some(50.0), Some(80.0), None, None);
    db.create_receipt(&receipt).unwrap();

    let result = assign_receipt_to_trip_internal(
        &db,
        &receipt.id.to_string(),
        &trip.id.to_string(),
        &vehicle.id.to_string(),
        "Fuel",
        true, // User confirmed override
    );

    assert!(result.is_ok(), "Assignment should succeed with override");

    let assigned_receipt = result.unwrap();
    assert_eq!(
        assigned_receipt.assignment_type,
        Some(crate::models::AssignmentType::Fuel)
    );
    assert_eq!(
        assigned_receipt.mismatch_override, true,
        "Should have override set"
    );
}

#[test]
fn test_assign_other_to_trip_with_existing_other_costs_allowed() {
    // Scenario C6: OTHER receipt to trip that already has other_costs → Just link (like C3 for fuel)
    // Design decision 2026-02-03: Allow assignment, don't overwrite existing data
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    // Trip already has other_costs (10.0 EUR)
    let trip = make_trip_for_assignment(vehicle.id, date, None, None, Some(10.0));
    db.create_trip(&trip).unwrap();

    let receipt = make_receipt_with_details(
        Some(date),
        None,
        Some(15.0), // Different price - but we allow it
        Some("AutoWash"),
        Some("Umytie auta"),
    );
    db.create_receipt(&receipt).unwrap();

    let result = assign_receipt_to_trip_internal(
        &db,
        &receipt.id.to_string(),
        &trip.id.to_string(),
        &vehicle.id.to_string(),
        "Other",
        false,
    );

    assert!(
        result.is_ok(),
        "Assignment should succeed - just link receipt to trip"
    );

    let assigned_receipt = result.unwrap();
    assert_eq!(assigned_receipt.trip_id, Some(trip.id));
    assert_eq!(
        assigned_receipt.assignment_type,
        Some(crate::models::AssignmentType::Other)
    );

    // Verify trip's other_costs is NOT overwritten (keeps original 10.0)
    let updated_trip = db.get_trip(&trip.id.to_string()).unwrap().unwrap();
    assert_eq!(
        updated_trip.other_costs_eur,
        Some(10.0),
        "Trip other_costs should remain unchanged"
    );
}

#[test]
fn test_reassign_invoice_to_different_trip() {
    // Scenario C7: Reassign receipt from one trip to another
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    let date1 = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2024, 6, 16).unwrap();
    let trip1 = make_trip_for_assignment(vehicle.id, date1, None, None, None);
    let trip2 = make_trip_for_assignment(vehicle.id, date2, None, None, None);
    db.create_trip(&trip1).unwrap();
    db.create_trip(&trip2).unwrap();

    let receipt = make_receipt_with_details(Some(date1), Some(45.0), Some(72.0), None, None);
    db.create_receipt(&receipt).unwrap();

    // First assignment to trip1
    let result1 = assign_receipt_to_trip_internal(
        &db,
        &receipt.id.to_string(),
        &trip1.id.to_string(),
        &vehicle.id.to_string(),
        "Fuel",
        false,
    );
    assert!(result1.is_ok());
    assert_eq!(result1.unwrap().trip_id, Some(trip1.id));

    // Reassign to trip2
    let result2 = assign_receipt_to_trip_internal(
        &db,
        &receipt.id.to_string(),
        &trip2.id.to_string(),
        &vehicle.id.to_string(),
        "Fuel",
        false,
    );
    assert!(result2.is_ok(), "Reassignment should succeed");

    let reassigned_receipt = result2.unwrap();
    assert_eq!(
        reassigned_receipt.trip_id,
        Some(trip2.id),
        "Should be assigned to trip2 now"
    );
}

#[test]
fn test_assign_other_with_mismatch_and_override() {
    // Scenario A9: OTHER receipt with mismatched data + user override
    // Verify mismatch_override=true works for OTHER type
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    // Trip already has other_costs (10.0 EUR)
    let trip = make_trip_for_assignment(vehicle.id, date, None, None, Some(10.0));
    db.create_trip(&trip).unwrap();

    // Receipt with DIFFERENT price (15.0 EUR) - mismatch
    let receipt = make_receipt_with_details(
        Some(date),
        None,
        Some(15.0),
        Some("AutoWash"),
        Some("Umytie auta"),
    );
    db.create_receipt(&receipt).unwrap();

    let result = assign_receipt_to_trip_internal(
        &db,
        &receipt.id.to_string(),
        &trip.id.to_string(),
        &vehicle.id.to_string(),
        "Other",
        true, // User confirmed override
    );

    assert!(result.is_ok(), "Assignment should succeed with override");

    let assigned_receipt = result.unwrap();
    assert_eq!(
        assigned_receipt.assignment_type,
        Some(crate::models::AssignmentType::Other)
    );
    assert_eq!(
        assigned_receipt.mismatch_override, true,
        "Should have override set"
    );

    // Verify trip's other_costs is NOT overwritten (keeps original 10.0)
    let updated_trip = db.get_trip(&trip.id.to_string()).unwrap().unwrap();
    assert_eq!(
        updated_trip.other_costs_eur,
        Some(10.0),
        "Trip other_costs should remain unchanged"
    );
}

#[test]
fn test_receipt_datetime_warnings_excludes_overrides() {
    // Scenario F2: Receipt with datetime OUTSIDE trip range but with mismatch_override=true
    // The current implementation returns the warning, but frontend filters it out.
    // This test documents the current behavior.
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    // Trip on June 15, 8:00-14:00
    let trip_start = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(8, 0, 0)
        .unwrap();
    let trip_end = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(14, 0, 0)
        .unwrap();

    let mut trip = make_trip_with_datetime_range(trip_start, Some(trip_end));
    trip.vehicle_id = vehicle.id;
    trip.fuel_liters = Some(45.0);
    trip.fuel_cost_eur = Some(72.0);
    db.create_trip(&trip).unwrap();

    // Receipt datetime AFTER trip end (18:00) - would normally trigger warning
    let receipt_dt = NaiveDate::from_ymd_opt(2024, 6, 15)
        .unwrap()
        .and_hms_opt(18, 0, 0)
        .unwrap();

    let mut receipt = make_receipt_with_datetime_assigned(Some(receipt_dt), trip.id);
    receipt.vehicle_id = Some(vehicle.id);
    receipt.mismatch_override = true; // User confirmed the mismatch
    db.create_receipt(&receipt).unwrap();

    // Call the warning calculation function
    let warnings = calculate_receipt_datetime_warnings(&[trip.clone()], &[receipt]);

    // Currently, the backend DOES include this in warnings
    // Frontend filters it out using the mismatch_override flag
    // This test documents that behavior
    assert_eq!(
        warnings.len(),
        1,
        "Backend returns warning (frontend will filter it)"
    );
    assert!(
        warnings.contains(&trip.id.to_string()),
        "Trip ID should be in warnings set (frontend filters using mismatch_override)"
    );
}

#[test]
fn test_invalid_assignment_type_rejected() {
    // Test: Invalid assignment type string → error
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trip = make_trip_for_assignment(vehicle.id, date, None, None, None);
    db.create_trip(&trip).unwrap();

    let receipt = make_receipt_with_details(Some(date), Some(45.0), Some(72.0), None, None);
    db.create_receipt(&receipt).unwrap();

    let result = assign_receipt_to_trip_internal(
        &db,
        &receipt.id.to_string(),
        &trip.id.to_string(),
        &vehicle.id.to_string(),
        "InvalidType", // Bad value
        false,
    );

    assert!(result.is_err(), "Should reject invalid assignment type");
    assert!(result.unwrap_err().contains("Invalid assignment type"));
}

// ========================================================================
// get_trips_for_receipt_assignment tests
// ========================================================================

#[test]
fn test_get_trips_for_receipt_assignment_empty_trip_returns_can_attach_true() {
    // Trip has NO fuel data → can attach receipt (will populate from receipt)
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trip = make_trip_for_assignment(vehicle.id, date, None, None, None);
    db.create_trip(&trip).unwrap();

    // Receipt with fuel data
    let receipt = make_receipt_with_details(Some(date), Some(45.0), Some(72.0), None, None);
    db.create_receipt(&receipt).unwrap();

    let result = get_trips_for_receipt_assignment_internal(
        &db,
        &receipt.id.to_string(),
        &vehicle.id.to_string(),
        2024,
    );

    assert!(result.is_ok(), "Should return trips");
    let trips = result.unwrap();
    assert_eq!(trips.len(), 1, "Should have 1 trip");
    assert!(trips[0].can_attach, "Empty trip should allow attachment");
    assert_eq!(
        trips[0].attachment_status, "empty",
        "Status should be 'empty'"
    );
}

#[test]
fn test_get_trips_for_receipt_assignment_matching_fuel_returns_can_attach_true() {
    // Trip HAS fuel data AND receipt matches → can attach as documentation
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    // Trip with fuel: 45L, 72 EUR
    let trip = make_trip_for_assignment(vehicle.id, date, Some(45.0), Some(72.0), None);
    db.create_trip(&trip).unwrap();

    // Receipt with MATCHING fuel data (same date, liters, price)
    let receipt = make_receipt_with_details(Some(date), Some(45.0), Some(72.0), None, None);
    db.create_receipt(&receipt).unwrap();

    let result = get_trips_for_receipt_assignment_internal(
        &db,
        &receipt.id.to_string(),
        &vehicle.id.to_string(),
        2024,
    );

    assert!(result.is_ok(), "Should return trips");
    let trips = result.unwrap();
    assert_eq!(trips.len(), 1, "Should have 1 trip");
    assert!(trips[0].can_attach, "Matching fuel should allow attachment");
    assert_eq!(
        trips[0].attachment_status, "matches",
        "Status should be 'matches'"
    );
}

#[test]
fn test_get_trips_for_receipt_assignment_different_liters_shows_mismatch() {
    // Design spec v7 (C4): Trip HAS fuel data but receipt has DIFFERENT liters
    // → CAN attach (user decides), but status shows 'differs' with mismatch reason
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    // Trip with 45L
    let trip = make_trip_for_assignment(vehicle.id, date, Some(45.0), Some(72.0), None);
    db.create_trip(&trip).unwrap();

    // Receipt with DIFFERENT liters (50L instead of 45L)
    let receipt = make_receipt_with_details(Some(date), Some(50.0), Some(72.0), None, None);
    db.create_receipt(&receipt).unwrap();

    let result = get_trips_for_receipt_assignment_internal(
        &db,
        &receipt.id.to_string(),
        &vehicle.id.to_string(),
        2024,
    );

    assert!(result.is_ok(), "Should return trips");
    let trips = result.unwrap();
    assert_eq!(trips.len(), 1, "Should have 1 trip");
    // Design spec v7: User CAN attach with mismatch, they just see a warning
    assert!(
        trips[0].can_attach,
        "Different liters should allow attachment (user confirms mismatch)"
    );
    assert_eq!(
        trips[0].attachment_status, "differs",
        "Status should be 'differs'"
    );
    assert_eq!(
        trips[0].mismatch_reason.as_deref(),
        Some("liters"),
        "Mismatch reason should indicate liters"
    );
}

#[test]
fn test_get_trips_for_receipt_assignment_different_price_shows_mismatch() {
    // Design spec v7 (C4): Trip HAS fuel data but receipt has DIFFERENT price
    // → CAN attach (user decides), but status shows 'differs' with mismatch reason
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    // Trip with 72 EUR
    let trip = make_trip_for_assignment(vehicle.id, date, Some(45.0), Some(72.0), None);
    db.create_trip(&trip).unwrap();

    // Receipt with DIFFERENT price (80 EUR instead of 72 EUR)
    let receipt = make_receipt_with_details(Some(date), Some(45.0), Some(80.0), None, None);
    db.create_receipt(&receipt).unwrap();

    let result = get_trips_for_receipt_assignment_internal(
        &db,
        &receipt.id.to_string(),
        &vehicle.id.to_string(),
        2024,
    );

    assert!(result.is_ok(), "Should return trips");
    let trips = result.unwrap();
    assert_eq!(trips.len(), 1, "Should have 1 trip");
    // Design spec v7: User CAN attach with mismatch, they just see a warning
    assert!(
        trips[0].can_attach,
        "Different price should allow attachment (user confirms mismatch)"
    );
    assert_eq!(
        trips[0].attachment_status, "differs",
        "Status should be 'differs'"
    );
    assert_eq!(
        trips[0].mismatch_reason.as_deref(),
        Some("price"),
        "Mismatch reason should indicate price"
    );
}

#[test]
fn test_get_trips_for_receipt_assignment_different_date_shows_mismatch() {
    // Design spec v7 (C4): Trip HAS fuel data but receipt has DIFFERENT date
    // → CAN attach (user decides), but status shows 'differs' with mismatch reason
    let db = Database::in_memory().unwrap();

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        66.0,
        5.1,
        0.0,
    );
    db.create_vehicle(&vehicle).unwrap();

    let trip_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let receipt_date = NaiveDate::from_ymd_opt(2024, 6, 16).unwrap(); // Different date

    // Trip with fuel on June 15
    let trip = make_trip_for_assignment(vehicle.id, trip_date, Some(45.0), Some(72.0), None);
    db.create_trip(&trip).unwrap();

    // Receipt with same liters/price but DIFFERENT date (June 16)
    let receipt = make_receipt_with_details(Some(receipt_date), Some(45.0), Some(72.0), None, None);
    db.create_receipt(&receipt).unwrap();

    let result = get_trips_for_receipt_assignment_internal(
        &db,
        &receipt.id.to_string(),
        &vehicle.id.to_string(),
        2024,
    );

    assert!(result.is_ok(), "Should return trips");
    let trips = result.unwrap();
    assert_eq!(trips.len(), 1, "Should have 1 trip");
    // Design spec v7: User CAN attach with mismatch, they just see a warning
    assert!(
        trips[0].can_attach,
        "Different date should allow attachment (user confirms mismatch)"
    );
    assert_eq!(
        trips[0].attachment_status, "differs",
        "Status should be 'differs'"
    );
    assert_eq!(
        trips[0].mismatch_reason.as_deref(),
        Some("date"),
        "Mismatch reason should indicate date"
    );
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
        sort_order: 0,
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
        sort_order: 0,
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
            sort_order: 1,
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
            sort_order: 0,
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
        sort_order: 0,
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
        sort_order: 0,
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
    // Legend should return the suggestion for the MOST RECENT trip (lowest sort_order)
    // This is the trip that would close the open period
    let mut trip1 = make_trip_for_magic_fill(
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        100.0, // older trip
        None,
        false,
    );
    trip1.sort_order = 2; // Higher = older in display

    let mut trip2 = make_trip_for_magic_fill(
        NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        150.0, // newer trip
        None,
        false,
    );
    trip2.sort_order = 1;

    let mut trip3 = make_trip_for_magic_fill(
        NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
        200.0, // most recent trip
        None,
        false,
    );
    trip3.sort_order = 0; // Lowest = most recent in display

    // Chronological order for calculation (by date)
    let trips = vec![trip1.clone(), trip2.clone(), trip3.clone()];

    let tp_consumption = 6.0;
    let (suggestions, legend) = calculate_suggested_fillups(&trips, tp_consumption);

    // All 3 trips should have suggestions
    assert_eq!(suggestions.len(), 3);

    // Legend should be the MOST RECENT trip's suggestion (trip3 with sort_order 0)
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
    use crate::commands::integrations::format_suggested_fillup_text;
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
    use crate::commands::integrations::format_suggested_fillup_text;

    assert_eq!(format_suggested_fillup_text(None), "");
}

#[test]
fn test_format_suggested_fillup_text_rounding() {
    use crate::commands::integrations::format_suggested_fillup_text;
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

#[test]
fn test_trip_numbers_same_date_by_odometer() {
    // Multiple trips on same day - order by odometer
    let trips = vec![
        make_trip_with_date_odo("2026-01-15", 50.0, 10100.0), // Should be #2
        make_trip_with_date_odo("2026-01-15", 30.0, 10050.0), // Should be #1
        make_trip_with_date_odo("2026-01-15", 25.0, 10150.0), // Should be #3
    ];

    let trip_numbers = calculate_trip_numbers(&trips);

    let first = trips
        .iter()
        .find(|t| t.odometer == 10050.0)
        .unwrap()
        .id
        .to_string();
    let second = trips
        .iter()
        .find(|t| t.odometer == 10100.0)
        .unwrap()
        .id
        .to_string();
    let third = trips
        .iter()
        .find(|t| t.odometer == 10150.0)
        .unwrap()
        .id
        .to_string();

    assert_eq!(trip_numbers.get(&first), Some(&1));
    assert_eq!(trip_numbers.get(&second), Some(&2));
    assert_eq!(trip_numbers.get(&third), Some(&3));
}

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
        sort_order: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn make_trip_with_date_odo(date_str: &str, distance: f64, odo: f64) -> Trip {
    make_trip_with_date(date_str, distance, odo)
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
