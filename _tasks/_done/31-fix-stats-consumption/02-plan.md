# Fix Stats Consumption Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Calculate average consumption only from closed fill-up periods, matching the table display logic.

**Architecture:** Add helper function `calculate_closed_period_totals` in `calculations.rs`, then use it in `calculate_trip_stats` in `commands.rs`.

**Tech Stack:** Rust, existing Trip model

---

## Task 1: Add `calculate_closed_period_totals` function

**Files:**
- Modify: `src-tauri/src/calculations.rs` (add function at end, before `#[cfg(test)]`)
- Modify: `src-tauri/src/calculations_tests.rs` (add tests)

**Step 1: Write failing tests in `calculations_tests.rs`**

Add at the end of the file:

```rust
// ============================================================================
// calculate_closed_period_totals tests
// ============================================================================

use crate::models::Trip;
use uuid::Uuid;

fn make_trip(distance_km: f64, fuel_liters: Option<f64>, full_tank: bool) -> Trip {
    Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        odometer: 10000.0,
        distance_km,
        route_name: "Test".to_string(),
        fuel_liters,
        fuel_cost_eur: None,
        full_tank,
        notes: None,
        sort_order: 0,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
        energy_kwh: None,
        energy_cost_eur: None,
        soc_percent: None,
        soc_override_percent: None,
    }
}

#[test]
fn test_closed_period_totals_single_period() {
    // 3 trips, last one has full tank fill-up
    let trips = vec![
        make_trip(100.0, None, false),
        make_trip(200.0, None, false),
        make_trip(50.0, Some(35.0), true), // 35L for 350km = 10 L/100km
    ];
    let (fuel, km) = super::calculate_closed_period_totals(&trips);
    assert_eq!(fuel, 35.0);
    assert_eq!(km, 350.0);
}

#[test]
fn test_closed_period_totals_no_closed_periods() {
    // Trips without any full tank fill-up
    let trips = vec![
        make_trip(100.0, None, false),
        make_trip(200.0, Some(20.0), false), // Partial fill-up
    ];
    let (fuel, km) = super::calculate_closed_period_totals(&trips);
    assert_eq!(fuel, 0.0);
    assert_eq!(km, 0.0);
}

#[test]
fn test_closed_period_totals_open_period_excluded() {
    // Closed period + open period after
    let trips = vec![
        make_trip(100.0, None, false),
        make_trip(50.0, Some(15.0), true),  // Closes period: 15L / 150km
        make_trip(200.0, None, false),       // Open period - excluded
        make_trip(100.0, None, false),       // Open period - excluded
    ];
    let (fuel, km) = super::calculate_closed_period_totals(&trips);
    assert_eq!(fuel, 15.0);
    assert_eq!(km, 150.0);
}

#[test]
fn test_closed_period_totals_multiple_periods() {
    // Two closed periods
    let trips = vec![
        make_trip(100.0, None, false),
        make_trip(100.0, Some(20.0), true),  // Period 1: 20L / 200km
        make_trip(150.0, None, false),
        make_trip(50.0, Some(18.0), true),   // Period 2: 18L / 200km
    ];
    let (fuel, km) = super::calculate_closed_period_totals(&trips);
    assert_eq!(fuel, 38.0);  // 20 + 18
    assert_eq!(km, 400.0);   // 200 + 200
}

#[test]
fn test_closed_period_totals_partial_then_full() {
    // Partial fill-ups accumulated into full fill-up
    let trips = vec![
        make_trip(100.0, None, false),
        make_trip(50.0, Some(10.0), false),  // Partial: 10L accumulated
        make_trip(100.0, None, false),
        make_trip(50.0, Some(15.0), true),   // Full: closes with 10+15=25L / 300km
    ];
    let (fuel, km) = super::calculate_closed_period_totals(&trips);
    assert_eq!(fuel, 25.0);
    assert_eq!(km, 300.0);
}
```

**Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test calculate_closed_period_totals`
Expected: FAIL - function doesn't exist

**Step 3: Implement `calculate_closed_period_totals` in `calculations.rs`**

Add before the `#[cfg(test)]` block:

```rust
use crate::models::Trip;

/// Calculate total fuel and km from closed fill-up periods only.
/// A period closes when there's a full tank fill-up.
/// Returns (total_fuel, total_km) from closed periods.
/// Open period (trips after last full fill-up) is excluded.
pub fn calculate_closed_period_totals(trips: &[Trip]) -> (f64, f64) {
    let mut total_fuel = 0.0;
    let mut total_km = 0.0;
    let mut period_km = 0.0;
    let mut period_fuel = 0.0;

    for trip in trips {
        period_km += trip.distance_km;

        if let Some(fuel) = trip.fuel_liters {
            period_fuel += fuel;

            // Full tank closes the period
            if trip.full_tank && period_km > 0.0 {
                total_fuel += period_fuel;
                total_km += period_km;
                period_km = 0.0;
                period_fuel = 0.0;
            }
        }
    }
    // Open period is NOT included in totals
    (total_fuel, total_km)
}
```

**Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test calculate_closed_period_totals`
Expected: All 5 tests PASS

**Step 5: Commit**

```bash
git add src-tauri/src/calculations.rs src-tauri/src/calculations_tests.rs
git commit -m "feat(calculations): add calculate_closed_period_totals function

Calculates average consumption only from closed fill-up periods,
excluding the open period after the last full tank fill-up.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Update `calculate_trip_stats` to use closed periods

**Files:**
- Modify: `src-tauri/src/commands.rs:504-512` (change avg calculation)
- Modify: `src-tauri/src/commands.rs:4` (add import)

**Step 1: Update import at top of `commands.rs`**

Change line 4 from:
```rust
    calculate_consumption_rate, calculate_margin_percent, calculate_fuel_used, calculate_fuel_level,
```
to:
```rust
    calculate_consumption_rate, calculate_margin_percent, calculate_fuel_used, calculate_fuel_level,
    calculate_closed_period_totals,
```

**Step 2: Replace avg_consumption_rate calculation in `calculate_trip_stats`**

Replace lines 504-512:
```rust
    // Calculate totals
    let total_fuel: f64 = trips.iter().filter_map(|t| t.fuel_liters).sum();
    let total_fuel_cost: f64 = trips.iter().filter_map(|t| t.fuel_cost_eur).sum();
    let total_km: f64 = trips.iter().map(|t| t.distance_km).sum();
    let avg_consumption_rate = if total_km > 0.0 {
        (total_fuel / total_km) * 100.0
    } else {
        0.0
    };
```

With:
```rust
    // Calculate totals (all trips for display)
    let total_fuel: f64 = trips.iter().filter_map(|t| t.fuel_liters).sum();
    let total_fuel_cost: f64 = trips.iter().filter_map(|t| t.fuel_cost_eur).sum();
    let total_km: f64 = trips.iter().map(|t| t.distance_km).sum();

    // Calculate average consumption from CLOSED periods only (for accurate margin)
    let (closed_fuel, closed_km) = calculate_closed_period_totals(&trips);
    let avg_consumption_rate = if closed_km > 0.0 {
        (closed_fuel / closed_km) * 100.0
    } else {
        0.0
    };
```

**Step 3: Update margin display logic**

Replace lines 583-584:
```rust
    // Use average margin for legal compliance display
    let display_margin = if total_fuel > 0.0 { Some(avg_margin) } else { None };
```

With:
```rust
    // Only show margin if we have closed periods (accurate data)
    let display_margin = if closed_km > 0.0 { Some(avg_margin) } else { None };
```

**Step 4: Run all backend tests**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS (including existing command tests)

**Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "fix(stats): calculate average consumption from closed periods only

Previously, stats showed average consumption including the open period
after the last fill-up, which diluted the rate and showed a different
number than the per-row rates in the table.

Now avg_consumption_rate and margin_percent are calculated only from
closed fill-up periods, matching the table display logic.

Fixes: Mercedes showing -2.6% deviation despite all table rates >= TP

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Verify with manual testing

**Step 1: Run the app**

Run: `npm run tauri dev`

**Step 2: Check Mercedes vehicle stats**

1. Select Mercedes (BT014IN)
2. Check stats header - should show consumption matching closed periods only
3. If only open period exists, margin should show "—" or similar

**Step 3: Verify edge cases**

- Vehicle with no fill-ups: margin should be None
- Vehicle with only partial fill-ups: margin should be None
- Vehicle with closed periods: margin should be calculated

---

## Task 4: Update changelog

**Step 1: Run changelog skill**

Run: `/changelog`

Add entry:
```
- Opravený výpočet priemernej spotreby - počíta len z uzavretých období plných tankov
```

**Step 2: Commit changelog**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): add stats consumption fix"
```
