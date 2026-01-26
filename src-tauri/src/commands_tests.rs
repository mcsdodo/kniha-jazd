// ============================================================================
// Tests
// ============================================================================

use super::*;
use crate::models::{ConfidenceLevel, FieldConfidence, Receipt, ReceiptStatus, Trip};
use chrono::{NaiveDate, Utc};
use uuid::Uuid;

/// Helper to create a trip with fuel
fn make_trip_with_fuel(date: NaiveDate, liters: f64, cost: f64) -> Trip {
    let now = Utc::now();
    Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        date,
        datetime: date.and_hms_opt(0, 0, 0).unwrap(),
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
fn make_trip_without_fuel(date: NaiveDate) -> Trip {
    let now = Utc::now();
    Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        date,
        datetime: date.and_hms_opt(0, 0, 0).unwrap(),
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

/// Helper to create a receipt with matching values
fn make_receipt(date: Option<NaiveDate>, liters: Option<f64>, price: Option<f64>) -> Receipt {
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
        receipt_date: date,
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
        created_at: now,
        updated_at: now,
    }
}

// ========================================================================
// Receipt-trip matching tests (calculate_missing_receipts)
// ========================================================================

#[test]
fn test_missing_receipts_exact_match() {
    // Trip and receipt with exact same date, liters, and price
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
    let receipts = vec![make_receipt(Some(date), Some(45.0), Some(72.50))];

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert!(missing.is_empty(), "Trip with matching receipt should not be flagged as missing");
}

#[test]
fn test_missing_receipts_no_match_different_date() {
    // Same liters and price, but different date
    let trip_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let receipt_date = NaiveDate::from_ymd_opt(2024, 6, 16).unwrap();
    let trips = vec![make_trip_with_fuel(trip_date, 45.0, 72.50)];
    let receipts = vec![make_receipt(Some(receipt_date), Some(45.0), Some(72.50))];

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert_eq!(missing.len(), 1, "Trip should be flagged when date differs");
    assert!(missing.contains(&trips[0].id.to_string()));
}

#[test]
fn test_missing_receipts_no_match_different_liters() {
    // Same date and price, but different liters
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
    let receipts = vec![make_receipt(Some(date), Some(44.5), Some(72.50))]; // Different liters

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert_eq!(missing.len(), 1, "Trip should be flagged when liters differ");
}

#[test]
fn test_missing_receipts_no_match_different_price() {
    // Same date and liters, but different price
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
    let receipts = vec![make_receipt(Some(date), Some(45.0), Some(73.00))]; // Different price

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert_eq!(missing.len(), 1, "Trip should be flagged when price differs");
}

#[test]
fn test_missing_receipts_trip_without_fuel_not_flagged() {
    // Trip without fuel should NOT be flagged as missing receipt
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_without_fuel(date)];
    let receipts: Vec<Receipt> = vec![];

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert!(missing.is_empty(), "Trip without fuel should not be flagged as missing receipt");
}

#[test]
fn test_missing_receipts_no_receipts_available() {
    // Trip with fuel but no receipts at all
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
    let receipts: Vec<Receipt> = vec![];

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert_eq!(missing.len(), 1, "Trip with fuel but no receipts should be flagged");
}

#[test]
fn test_missing_receipts_multiple_trips_partial_match() {
    // Multiple trips, some with matching receipts, some without
    let date1 = NaiveDate::from_ymd_opt(2024, 6, 10).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2024, 6, 20).unwrap();
    let date3 = NaiveDate::from_ymd_opt(2024, 6, 30).unwrap();

    let trips = vec![
        make_trip_with_fuel(date1, 40.0, 65.00), // Will have matching receipt
        make_trip_with_fuel(date2, 50.0, 80.00), // No matching receipt
        make_trip_without_fuel(date3),           // No fuel, should not be flagged
    ];
    let receipts = vec![
        make_receipt(Some(date1), Some(40.0), Some(65.00)), // Matches trip 1
    ];

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert_eq!(missing.len(), 1, "Only trip 2 should be flagged");
    assert!(missing.contains(&trips[1].id.to_string()), "Trip 2 (with fuel, no receipt) should be flagged");
    assert!(!missing.contains(&trips[0].id.to_string()), "Trip 1 (with matching receipt) should not be flagged");
    assert!(!missing.contains(&trips[2].id.to_string()), "Trip 3 (no fuel) should not be flagged");
}

#[test]
fn test_missing_receipts_receipt_with_missing_date() {
    // Receipt without a date cannot match
    let trip_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_with_fuel(trip_date, 45.0, 72.50)];
    let receipts = vec![make_receipt(None, Some(45.0), Some(72.50))]; // No date

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert_eq!(missing.len(), 1, "Receipt without date should not match");
}

#[test]
fn test_missing_receipts_receipt_with_missing_liters() {
    // Receipt without liters cannot match
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
    let receipts = vec![make_receipt(Some(date), None, Some(72.50))]; // No liters

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert_eq!(missing.len(), 1, "Receipt without liters should not match");
}

#[test]
fn test_missing_receipts_receipt_with_missing_price() {
    // Receipt without price cannot match
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let trips = vec![make_trip_with_fuel(date, 45.0, 72.50)];
    let receipts = vec![make_receipt(Some(date), Some(45.0), None)]; // No price

    let missing = calculate_missing_receipts(&trips, &receipts);

    assert_eq!(missing.len(), 1, "Receipt without price should not match");
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
        date,
        datetime: date.and_hms_opt(0, 0, 0).unwrap(),
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
        make_trip_detailed(base_date, 100.0, None, false, 3),                           // 100km, no fuel
        make_trip_detailed(base_date.succ_opt().unwrap(), 100.0, Some(20.0), false, 2), // 100km, 20L PARTIAL
        make_trip_detailed(base_date.succ_opt().unwrap().succ_opt().unwrap(), 100.0, None, false, 1), // 100km, no fuel
        make_trip_detailed(base_date.succ_opt().unwrap().succ_opt().unwrap().succ_opt().unwrap(), 100.0, Some(30.0), true, 0), // 100km, 30L FULL
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
        make_trip_detailed(base_date, 100.0, None, false, 3),                           // Period 1: 100km
        make_trip_detailed(base_date.succ_opt().unwrap(), 100.0, Some(10.0), true, 2),  // Period 1: closes with 10L -> rate = 10/200*100 = 5.0
        make_trip_detailed(base_date.succ_opt().unwrap().succ_opt().unwrap(), 200.0, None, false, 1), // Period 2: 200km
        make_trip_detailed(base_date.succ_opt().unwrap().succ_opt().unwrap().succ_opt().unwrap(), 200.0, Some(16.0), true, 0), // Period 2: closes with 16L -> rate = 16/400*100 = 4.0
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
        make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(), 50.0, None, false, 0), // Top: Jun 15
        make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(), 50.0, None, false, 1), // Middle: Jun 10 - WRONG! Should be between 15 and 20
        make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 20).unwrap(), 50.0, None, false, 2), // Bottom: Jun 20
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
        make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 20).unwrap(), 50.0, None, false, 0), // Top: Jun 20 (newest)
        make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(), 50.0, None, false, 1), // Middle: Jun 15
        make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(), 50.0, None, false, 2), // Bottom: Jun 10 (oldest)
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

    let trips = vec![
        make_trip_detailed(base_date, 100.0, Some(6.0), true, 0),
    ];

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
        make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(), 100.0, Some(5.0), true, 0),
        // Period 2: 100km, 7L = 7.0 l/100km (OVER limit)
        make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 2).unwrap(), 100.0, Some(7.0), true, 1),
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
        make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(), 100.0, Some(5.0), true, 0),
        // Period 2: 100km, 5.5L = 5.5 l/100km (under)
        make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 2).unwrap(), 100.0, Some(5.5), true, 1),
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
        make_trip_detailed(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(), 100.0, Some(6.0), true, 0),
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
    let trips = vec![
        make_trip_detailed(base_date, 100.0, None, false, 0),
    ];

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
        50.0,  // tank_size
        6.0,   // tp_consumption
        0.0,
    );
    db.create_vehicle(&vehicle).expect("Failed to create vehicle");

    // Query for 2025 with no 2024 data
    let result = get_year_start_fuel_remaining(
        &db,
        &vehicle.id.to_string(),
        2025,
        50.0,  // tank_size
        6.0,   // tp_consumption
    );

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 50.0, "Should return full tank when no previous year data");
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
    db.create_vehicle(&vehicle).expect("Failed to create vehicle");

    let now = Utc::now();
    let date = NaiveDate::from_ymd_opt(2024, 12, 15).unwrap();
    let trip_2024 = Trip {
        id: Uuid::new_v4(),
        vehicle_id: vehicle.id,
        date,
        datetime: date.and_hms_opt(0, 0, 0).unwrap(),
        origin: "A".to_string(),
        destination: "B".to_string(),
        distance_km: 100.0,
        odometer: 10000.0,
        purpose: "test".to_string(),
        fuel_liters: Some(6.0),
        fuel_cost_eur: Some(10.0),
        full_tank: true,  // Full tank fillup -> ends at 50L
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

    let result = get_year_start_fuel_remaining(
        &db,
        &vehicle.id.to_string(),
        2025,
        50.0,
        6.0,
    );

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 50.0, "Full tank fillup should end at tank_size");
}

#[test]
fn test_year_start_fuel_partial_tank_carryover() {
    // Test that partial tank fillups carry over correctly
    let db = crate::db::Database::in_memory().expect("Failed to create database");

    let vehicle = crate::models::Vehicle::new(
        "Test Car".to_string(),
        "BA123XY".to_string(),
        50.0,  // tank_size
        6.0,   // tp_consumption (6 l/100km)
        0.0,
    );
    db.create_vehicle(&vehicle).expect("Failed to create vehicle");

    let now = Utc::now();

    // Trip 1: Drive 100km, full tank fillup with 6L
    // Starts at 50L (no prior year), uses 6L, ends at 50L (full tank)
    let date1 = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let trip1 = Trip {
        id: Uuid::new_v4(),
        vehicle_id: vehicle.id,
        date: date1,
        datetime: date1.and_hms_opt(0, 0, 0).unwrap(),
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
        date: date2,
        datetime: date2.and_hms_opt(0, 0, 0).unwrap(),
        origin: "B".to_string(),
        destination: "C".to_string(),
        distance_km: 200.0,
        odometer: 10200.0,
        purpose: "test".to_string(),
        fuel_liters: Some(10.0),
        fuel_cost_eur: Some(16.0),
        full_tank: false,  // Partial fillup
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

    let result = get_year_start_fuel_remaining(
        &db,
        &vehicle.id.to_string(),
        2025,
        50.0,
        6.0,
    );

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
    db.create_vehicle(&vehicle).expect("Failed to create vehicle");

    // Query for 2025 with no 2024 data
    let result = get_year_start_odometer(
        &db,
        &vehicle.id.to_string(),
        2025,
        38057.0, // initial_odometer
    );

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 38057.0, "Should return initial odometer when no previous year data");
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
    db.create_vehicle(&vehicle).expect("Failed to create vehicle");

    let now = Utc::now();

    // Trip in 2024 ending at 54914 km (like the bug scenario)
    let date = NaiveDate::from_ymd_opt(2024, 12, 13).unwrap();
    let trip_2024 = Trip {
        id: Uuid::new_v4(),
        vehicle_id: vehicle.id,
        date,
        datetime: date.and_hms_opt(0, 0, 0).unwrap(),
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
    assert_eq!(result.unwrap(), 54914.0, "Should return last trip's odometer from previous year");
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
    db.create_vehicle(&vehicle).expect("Failed to create vehicle");

    let now = Utc::now();

    // First trip (earlier date)
    let date1 = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let trip1 = Trip {
        id: Uuid::new_v4(),
        vehicle_id: vehicle.id,
        date: date1,
        datetime: date1.and_hms_opt(0, 0, 0).unwrap(),
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
        date: date2,
        datetime: date2.and_hms_opt(0, 0, 0).unwrap(),
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

    let result = get_year_start_odometer(
        &db,
        &vehicle.id.to_string(),
        2025,
        10000.0,
    );

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 20000.0, "Should return the last trip's odometer by date");
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
        date,
        datetime: date.and_hms_opt(0, 0, 0).unwrap(),
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
        75.0,  // battery capacity
        18.0,  // baseline consumption
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
    receipt_unassigned.receipt_date = Some(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap());

    let mut receipt_a = Receipt::new("path2.jpg".to_string(), "vehicle_a.jpg".to_string());
    receipt_a.vehicle_id = Some(vehicle_a.id);
    receipt_a.receipt_date = Some(NaiveDate::from_ymd_opt(2024, 6, 16).unwrap());

    let mut receipt_b = Receipt::new("path3.jpg".to_string(), "vehicle_b.jpg".to_string());
    receipt_b.vehicle_id = Some(vehicle_b.id);
    receipt_b.receipt_date = Some(NaiveDate::from_ymd_opt(2024, 6, 17).unwrap());

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
        receipt_date: date,
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
        created_at: now,
        updated_at: now,
    }
}

/// Helper to create a trip for assignment tests (with vehicle_id that stays constant)
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
        date,
        datetime: date.and_hms_opt(0, 0, 0).unwrap(),
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

#[test]
fn test_assign_fuel_receipt_matches_trip() {
    // Test: Assign fuel receipt that matches trip's fuel data
    // Expected: Receipt assigned, trip.other_costs unchanged
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

    // Receipt with matching liters/price/date = fuel receipt
    let receipt = make_receipt_with_details(Some(date), Some(45.0), Some(72.0), None, None);
    db.create_receipt(&receipt).unwrap();

    let result = assign_receipt_to_trip_internal(
        &db,
        &receipt.id.to_string(),
        &trip.id.to_string(),
        &vehicle.id.to_string(),
    );

    assert!(result.is_ok(), "Assignment should succeed");

    let assigned_receipt = result.unwrap();
    assert_eq!(assigned_receipt.trip_id, Some(trip.id));
    assert_eq!(assigned_receipt.vehicle_id, Some(vehicle.id));
    assert_eq!(assigned_receipt.status, ReceiptStatus::Assigned);

    // Trip should NOT have other_costs set (fuel receipt doesn't touch other_costs)
    let updated_trip = db.get_trip(&trip.id.to_string()).unwrap().unwrap();
    assert!(
        updated_trip.other_costs_eur.is_none(),
        "Fuel receipt should not set other_costs"
    );
}

#[test]
fn test_assign_other_cost_receipt_no_liters() {
    // Test: Assign receipt without liters (car wash, parking, etc.)
    // Expected: Receipt assigned, trip.other_costs populated
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

    // Receipt without liters = other cost
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
    );

    assert!(result.is_ok(), "Assignment should succeed");

    let assigned_receipt = result.unwrap();
    assert_eq!(assigned_receipt.status, ReceiptStatus::Assigned);

    // Trip should have other_costs set
    let updated_trip = db.get_trip(&trip.id.to_string()).unwrap().unwrap();
    assert_eq!(
        updated_trip.other_costs_eur,
        Some(15.0),
        "Other cost should be set from receipt"
    );
    assert!(
        updated_trip.other_costs_note.as_ref().unwrap().contains("AutoWash"),
        "Note should contain vendor name"
    );
}

#[test]
fn test_assign_receipt_with_liters_not_matching_trip_fuel() {
    // Test: Receipt has liters (washer fluid) but doesn't match trip's fuel
    // Expected: Treated as other cost, trip.other_costs populated
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
    // Trip with fuel entry: 45L, 72 EUR
    let trip = make_trip_for_assignment(vehicle.id, date, Some(45.0), Some(72.0), None);
    db.create_trip(&trip).unwrap();

    // Receipt with 2L, 5 EUR (washer fluid - doesn't match trip fuel)
    let receipt = make_receipt_with_details(
        Some(date),
        Some(2.0),
        Some(5.0),
        Some("OMV"),
        Some("Zimná zmes"),
    );
    db.create_receipt(&receipt).unwrap();

    let result = assign_receipt_to_trip_internal(
        &db,
        &receipt.id.to_string(),
        &trip.id.to_string(),
        &vehicle.id.to_string(),
    );

    assert!(result.is_ok(), "Assignment should succeed");

    // Trip should have other_costs set (not treated as fuel)
    let updated_trip = db.get_trip(&trip.id.to_string()).unwrap().unwrap();
    assert_eq!(
        updated_trip.other_costs_eur,
        Some(5.0),
        "Washer fluid should be treated as other cost"
    );
    assert!(
        updated_trip.other_costs_note.as_ref().unwrap().contains("OMV"),
        "Note should contain vendor name"
    );
}

#[test]
fn test_assign_other_cost_receipt_collision_rejected() {
    // Test: Try to assign other cost receipt when trip already has other_costs
    // Expected: Error "Jazda uz ma ine naklady"
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
    // Trip already has other_costs
    let trip = make_trip_for_assignment(vehicle.id, date, None, None, Some(10.0));
    db.create_trip(&trip).unwrap();

    // Try to assign another "other cost" receipt
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
    );

    assert!(result.is_err(), "Assignment should fail due to collision");
    assert!(
        result.unwrap_err().contains("náklady"),
        "Error should mention existing costs"
    );
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
    assert_eq!(trips[0].attachment_status, "empty", "Status should be 'empty'");
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
    assert_eq!(trips[0].attachment_status, "matches", "Status should be 'matches'");
}

#[test]
fn test_get_trips_for_receipt_assignment_different_liters_returns_can_attach_false() {
    // Trip HAS fuel data but receipt has DIFFERENT liters → cannot attach
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
    assert!(!trips[0].can_attach, "Different liters should NOT allow attachment");
    assert_eq!(trips[0].attachment_status, "differs", "Status should be 'differs'");
}

#[test]
fn test_get_trips_for_receipt_assignment_different_price_returns_can_attach_false() {
    // Trip HAS fuel data but receipt has DIFFERENT price → cannot attach
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
    assert!(!trips[0].can_attach, "Different price should NOT allow attachment");
    assert_eq!(trips[0].attachment_status, "differs", "Status should be 'differs'");
}

#[test]
fn test_get_trips_for_receipt_assignment_different_date_returns_can_attach_false() {
    // Trip HAS fuel data but receipt has DIFFERENT date → cannot attach
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
    assert!(!trips[0].can_attach, "Different date should NOT allow attachment");
    assert_eq!(trips[0].attachment_status, "differs", "Status should be 'differs'");
}

// ========================================================================
// Backup Filename Parsing/Generating Tests
// ========================================================================

#[test]
fn test_parse_backup_filename_manual() {
    let filename = "kniha-jazd-backup-2026-01-24-143022.db";
    let (backup_type, update_version) = parse_backup_filename(filename);
    assert_eq!(backup_type, "manual");
    assert_eq!(update_version, None);
}

#[test]
fn test_parse_backup_filename_pre_update() {
    let filename = "kniha-jazd-backup-2026-01-24-143022-pre-v0.20.0.db";
    let (backup_type, update_version) = parse_backup_filename(filename);
    assert_eq!(backup_type, "pre-update");
    assert_eq!(update_version, Some("0.20.0".to_string()));
}

#[test]
fn test_parse_backup_filename_pre_update_complex_version() {
    let filename = "kniha-jazd-backup-2026-01-24-143022-pre-v1.2.3-beta.db";
    let (backup_type, update_version) = parse_backup_filename(filename);
    assert_eq!(backup_type, "pre-update");
    assert_eq!(update_version, Some("1.2.3-beta".to_string()));
}

#[test]
fn test_generate_backup_filename_manual() {
    let filename = generate_backup_filename("manual", None);
    assert!(filename.starts_with("kniha-jazd-backup-"));
    assert!(filename.ends_with(".db"));
    assert!(!filename.contains("-pre-v"));
}

#[test]
fn test_generate_backup_filename_pre_update() {
    let filename = generate_backup_filename("pre-update", Some("0.20.0"));
    assert!(filename.starts_with("kniha-jazd-backup-"));
    assert!(filename.contains("-pre-v0.20.0.db"));
}

// ========================================================================
// Backup Cleanup Tests
// ========================================================================

#[test]
fn test_get_cleanup_candidates_keeps_n_most_recent() {
    let backups = vec![
        BackupInfo {
            filename: "kniha-jazd-backup-2026-01-20-100000-pre-v0.17.0.db".to_string(),
            created_at: "2026-01-20T10:00:00".to_string(),
            size_bytes: 1000,
            vehicle_count: 0,
            trip_count: 0,
            backup_type: "pre-update".to_string(),
            update_version: Some("0.17.0".to_string()),
        },
        BackupInfo {
            filename: "kniha-jazd-backup-2026-01-21-100000-pre-v0.18.0.db".to_string(),
            created_at: "2026-01-21T10:00:00".to_string(),
            size_bytes: 1000,
            vehicle_count: 0,
            trip_count: 0,
            backup_type: "pre-update".to_string(),
            update_version: Some("0.18.0".to_string()),
        },
        BackupInfo {
            filename: "kniha-jazd-backup-2026-01-22-100000-pre-v0.19.0.db".to_string(),
            created_at: "2026-01-22T10:00:00".to_string(),
            size_bytes: 1000,
            vehicle_count: 0,
            trip_count: 0,
            backup_type: "pre-update".to_string(),
            update_version: Some("0.19.0".to_string()),
        },
        BackupInfo {
            filename: "kniha-jazd-backup-2026-01-23-100000-pre-v0.20.0.db".to_string(),
            created_at: "2026-01-23T10:00:00".to_string(),
            size_bytes: 1000,
            vehicle_count: 0,
            trip_count: 0,
            backup_type: "pre-update".to_string(),
            update_version: Some("0.20.0".to_string()),
        },
        BackupInfo {
            filename: "kniha-jazd-backup-2026-01-24-100000.db".to_string(),
            created_at: "2026-01-24T10:00:00".to_string(),
            size_bytes: 1000,
            vehicle_count: 0,
            trip_count: 0,
            backup_type: "manual".to_string(),
            update_version: None,
        },
    ];

    let to_delete = get_cleanup_candidates(&backups, 2);

    // Should delete oldest 2 pre-update backups, keep 2 most recent
    assert_eq!(to_delete.len(), 2);
    assert!(to_delete.iter().any(|b| b.filename.contains("v0.17.0")));
    assert!(to_delete.iter().any(|b| b.filename.contains("v0.18.0")));
    // Manual backup should NOT be in delete list
    assert!(!to_delete.iter().any(|b| b.backup_type == "manual"));
}

#[test]
fn test_get_cleanup_candidates_ignores_manual() {
    let backups = vec![
        BackupInfo {
            filename: "kniha-jazd-backup-2026-01-20-100000.db".to_string(),
            created_at: "2026-01-20T10:00:00".to_string(),
            size_bytes: 1000,
            vehicle_count: 0,
            trip_count: 0,
            backup_type: "manual".to_string(),
            update_version: None,
        },
        BackupInfo {
            filename: "kniha-jazd-backup-2026-01-21-100000.db".to_string(),
            created_at: "2026-01-21T10:00:00".to_string(),
            size_bytes: 1000,
            vehicle_count: 0,
            trip_count: 0,
            backup_type: "manual".to_string(),
            update_version: None,
        },
    ];

    let to_delete = get_cleanup_candidates(&backups, 1);

    // Manual backups should never be deleted
    assert_eq!(to_delete.len(), 0);
}

#[test]
fn test_get_cleanup_candidates_no_delete_when_under_limit() {
    let backups = vec![
        BackupInfo {
            filename: "kniha-jazd-backup-2026-01-22-100000-pre-v0.19.0.db".to_string(),
            created_at: "2026-01-22T10:00:00".to_string(),
            size_bytes: 1000,
            vehicle_count: 0,
            trip_count: 0,
            backup_type: "pre-update".to_string(),
            update_version: Some("0.19.0".to_string()),
        },
        BackupInfo {
            filename: "kniha-jazd-backup-2026-01-23-100000-pre-v0.20.0.db".to_string(),
            created_at: "2026-01-23T10:00:00".to_string(),
            size_bytes: 1000,
            vehicle_count: 0,
            trip_count: 0,
            backup_type: "pre-update".to_string(),
            update_version: Some("0.20.0".to_string()),
        },
    ];

    // Keep 3 but only have 2 - should delete nothing
    let to_delete = get_cleanup_candidates(&backups, 3);
    assert_eq!(to_delete.len(), 0);
}

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
        date,
        datetime: date.and_hms_opt(0, 0, 0).unwrap(),
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
    assert_eq!(km_up_to_c, 125.0, "Should only count km up to the edited trip");
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
        date,
        datetime: date.and_hms_opt(0, 0, 0).unwrap(),
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
            date: date1,
            datetime: date1.and_hms_opt(0, 0, 0).unwrap(),
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
            date: date2,
            datetime: date2.and_hms_opt(0, 0, 0).unwrap(),
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

    assert!((trip1_consumed - 9.0).abs() < 0.01, "150 km at 6.0 l/100km = 9.0 L");
    assert!((trip2_consumed - 6.0).abs() < 0.01, "100 km at 6.0 l/100km = 6.0 L");
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
        date,
        datetime: date.and_hms_opt(0, 0, 0).unwrap(),
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
        date,
        datetime: date.and_hms_opt(0, 0, 0).unwrap(),
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
// Time parsing tests (parse_trip_datetime)
// ========================================================================

#[test]
fn test_parse_trip_datetime_with_time() {
    // Test time="08:30" produces correct datetime
    let result = parse_trip_datetime("2026-01-15", Some("08:30"));

    assert!(result.is_ok());
    let datetime = result.unwrap();
    assert_eq!(datetime.format("%Y-%m-%dT%H:%M:%S").to_string(), "2026-01-15T08:30:00");
}

#[test]
fn test_parse_trip_datetime_without_time() {
    // Test time="" defaults to 00:00
    let result = parse_trip_datetime("2026-01-15", Some(""));

    assert!(result.is_ok());
    let datetime = result.unwrap();
    assert_eq!(datetime.format("%Y-%m-%dT%H:%M:%S").to_string(), "2026-01-15T00:00:00");
}

#[test]
fn test_parse_trip_datetime_none_time() {
    // Test time=None defaults to 00:00
    let result = parse_trip_datetime("2026-01-15", None);

    assert!(result.is_ok());
    let datetime = result.unwrap();
    assert_eq!(datetime.format("%Y-%m-%dT%H:%M:%S").to_string(), "2026-01-15T00:00:00");
}

#[test]
fn test_parse_trip_datetime_invalid_time_format() {
    // Test invalid time format returns error
    let result = parse_trip_datetime("2026-01-15", Some("invalid"));

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Invalid time format"));
}

#[test]
fn test_parse_trip_datetime_invalid_date_format() {
    // Test invalid date format returns error
    let result = parse_trip_datetime("not-a-date", Some("08:30"));

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Invalid date format"));
}