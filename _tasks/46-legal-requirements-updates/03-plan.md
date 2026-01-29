# Legal Requirements Updates (2026) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add Slovak legal compliance features for vehicle logbooks effective 1.1.2026

**Architecture:** Backend-only calculations (ADR-008). New `end_time` field stored in DB; trip numbers, odometer_start, and month-end rows are calculated at runtime in `get_trip_grid_data`. Frontend displays calculated values.

**Tech Stack:** Rust/Tauri backend, SvelteKit frontend, Diesel ORM, SQLite

---

## Task 1: Database Migration for end_time

**Files:**
- Create: `src-tauri/migrations/2026-01-29-100000_add_trip_end_time/up.sql`
- Create: `src-tauri/migrations/2026-01-29-100000_add_trip_end_time/down.sql`

**Step 1: Create migration directory**

```bash
mkdir -p src-tauri/migrations/2026-01-29-100000_add_trip_end_time
```

**Step 2: Write up.sql**

```sql
-- Add end_time column for trip end time (HH:MM format)
-- DEFAULT '' for backward compatibility with older app versions
ALTER TABLE trips ADD COLUMN end_time TEXT NOT NULL DEFAULT '';
```

**Step 3: Write down.sql**

```sql
-- SQLite doesn't support DROP COLUMN directly
-- This is a placeholder - we don't actually support downgrades
SELECT 1;
```

**Step 4: Run migration**

```bash
cd src-tauri && diesel migration run
```

Expected: Migration applies successfully, schema.rs updated

**Step 5: Commit**

```bash
git add src-tauri/migrations/2026-01-29-100000_add_trip_end_time/
git commit -m "feat(db): add end_time column to trips table

Legal requirement 4c: track trip start and end times separately.
Column uses TEXT DEFAULT '' for backward compatibility."
```

---

## Task 2: Update Rust Models for end_time

**Files:**
- Modify: `src-tauri/src/schema.rs` (auto-generated, verify)
- Modify: `src-tauri/src/models.rs`

**Step 1: Verify schema.rs was updated by migration**

Check that `src-tauri/src/schema.rs` contains:
```rust
diesel::table! {
    trips (id) {
        // ... existing columns ...
        end_time -> Text,  // NEW
    }
}
```

**Step 2: Update Trip struct in models.rs**

Add after `datetime` field (~line 183):
```rust
pub end_time: Option<String>,  // Trip end time "HH:MM" or None
```

**Step 3: Update TripRow struct**

Add after `datetime` field (~line 633):
```rust
pub end_time: String,  // "HH:MM" or ""
```

**Step 4: Update NewTripRow struct**

Add after `datetime` field (~line 660):
```rust
pub end_time: &'a str,
```

**Step 5: Update From<TripRow> for Trip**

In the `impl From<TripRow> for Trip` (~line 824), add:
```rust
end_time: if row.end_time.is_empty() { None } else { Some(row.end_time) },
```

**Step 6: Run cargo check**

```bash
cd src-tauri && cargo check
```

Expected: Compiles with errors about db.rs not passing end_time

**Step 7: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/schema.rs
git commit -m "feat(models): add end_time field to Trip

Stores trip end time as 'HH:MM' string. None when not entered."
```

---

## Task 3: Update Database CRUD for end_time

**Files:**
- Modify: `src-tauri/src/db.rs`

**Step 1: Update insert_trip function**

Find `insert_trip` and add `end_time` to NewTripRow:
```rust
end_time: trip.end_time.as_deref().unwrap_or(""),
```

**Step 2: Update update_trip function**

Find the update query and add:
```rust
trips::end_time.eq(trip.end_time.as_deref().unwrap_or("")),
```

**Step 3: Refactor year filtering to avoid raw SQL**

The `get_trips_for_vehicle_in_year()` function uses raw SQL with `strftime()`.
Replace with Diesel query builder using date range filtering:

```rust
pub fn get_trips_for_vehicle_in_year(
    &self,
    vehicle_id: &str,
    year: i32,
) -> QueryResult<Vec<Trip>> {
    use crate::schema::trips::dsl;
    let conn = &mut *self.conn.lock().unwrap();

    // Use date range instead of strftime - works with Diesel query builder
    let start_date = format!("{}-01-01", year);
    let end_date = format!("{}-12-31", year);

    let rows = dsl::trips
        .filter(dsl::vehicle_id.eq(vehicle_id))
        .filter(dsl::date.ge(&start_date))
        .filter(dsl::date.le(&end_date))
        .order(dsl::sort_order.asc())
        .load::<TripRow>(conn)?;

    Ok(rows.into_iter().map(Trip::from).collect())
}
```

**Why this is better:**
- Uses Diesel ORM (type-safe, auto-includes all columns)
- No raw SQL to maintain when schema changes
- Date range filtering is SQLite-efficient (can use index on `date`)

**Step 4: Run tests**

```bash
cd src-tauri && cargo test db_tests
```

Expected: All DB tests pass

**Step 5: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat(db): handle end_time in trip CRUD operations"
```

---

## Task 4: Update Commands for end_time

**Files:**
- Modify: `src-tauri/src/commands/trips.rs`

**Step 1: Update create_trip command**

Add parameter:
```rust
end_time: Option<String>,
```

Pass to Trip struct:
```rust
end_time,
```

**Step 2: Update update_trip command**

Add parameter:
```rust
end_time: Option<String>,
```

Update the trip:
```rust
trip.end_time = end_time;
```

**Step 3: Run cargo check**

```bash
cd src-tauri && cargo check
```

Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src-tauri/src/commands/trips.rs
git commit -m "feat(commands): accept end_time in trip create/update"
```

---

## Task 5: Add MonthEndRow Model

**Files:**
- Modify: `src-tauri/src/models.rs`

**Step 1: Add MonthEndRow struct**

Add after TripGridData struct (~line 372):
```rust
/// Synthetic row for month-end state display (legal requirement)
/// Generated for months where no trip falls on the last calendar day.
/// Only contains odometer and fuel state — no trip number, no driver (display-only fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthEndRow {
    /// Last day of the month (e.g., 2026-01-31)
    pub date: NaiveDate,
    /// Odometer reading (same for start/end - no travel)
    pub odometer: f64,
    /// Fuel remaining in liters (carried from last trip state)
    pub fuel_remaining: f64,
    /// Month number 1-12 (for identification/sorting)
    pub month: u32,
}
```

**Step 2: Run cargo check**

```bash
cd src-tauri && cargo check
```

Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src-tauri/src/models.rs
git commit -m "feat(models): add MonthEndRow for synthetic month-end display"
```

---

## Task 6: Extend TripGridData with New Fields

**Files:**
- Modify: `src-tauri/src/models.rs`

**Step 1: Add new fields to TripGridData**

Add these fields to TripGridData struct (~line 325):
```rust
    // Legal compliance fields (2026)
    /// Trip sequence number (1-based, per year, chronological order)
    pub trip_numbers: HashMap<String, i32>,
    /// Odometer at trip START (derived from previous trip's ending odo)
    pub odometer_start: HashMap<String, f64>,
    /// Trip IDs that fall on the last day of their month (for highlighting)
    pub month_end_trips: HashSet<String>,
    /// Synthetic rows for months without a trip on the last day
    pub month_end_rows: Vec<MonthEndRow>,
```

**Step 2: Run cargo check**

```bash
cd src-tauri && cargo check
```

Expected: Errors in get_trip_grid_data about missing fields

**Step 3: Commit (partial - will fix in next task)**

```bash
git add src-tauri/src/models.rs
git commit -m "feat(models): add legal compliance fields to TripGridData

- trip_numbers: sequential numbering per year
- odometer_start: derived from previous trip
- month_end_trips: IDs for highlighting
- month_end_rows: synthetic rows for gaps"
```

---

## Task 7: Write Tests for Trip Numbering

**Files:**
- Modify: `src-tauri/src/commands/commands_tests.rs`

**Step 1: Write failing test for trip numbering**

Add to commands_tests.rs:
```rust
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
    let jan10_id = trips.iter().find(|t| t.date.day() == 10).unwrap().id.to_string();
    let jan15_id = trips.iter().find(|t| t.date.day() == 15).unwrap().id.to_string();
    let jan20_id = trips.iter().find(|t| t.date.day() == 20).unwrap().id.to_string();

    assert_eq!(trip_numbers.get(&jan10_id), Some(&1));
    assert_eq!(trip_numbers.get(&jan15_id), Some(&2));
    assert_eq!(trip_numbers.get(&jan20_id), Some(&3));
}

#[test]
fn test_trip_numbers_same_date_by_odometer() {
    // Multiple trips on same day - order by odometer
    let trips = vec![
        make_trip_with_date_odo("2026-01-15", 50.0, 10100.0),  // Should be #2
        make_trip_with_date_odo("2026-01-15", 30.0, 10050.0),  // Should be #1
        make_trip_with_date_odo("2026-01-15", 25.0, 10150.0),  // Should be #3
    ];

    let trip_numbers = calculate_trip_numbers(&trips);

    let first = trips.iter().find(|t| t.odometer == 10050.0).unwrap().id.to_string();
    let second = trips.iter().find(|t| t.odometer == 10100.0).unwrap().id.to_string();
    let third = trips.iter().find(|t| t.odometer == 10150.0).unwrap().id.to_string();

    assert_eq!(trip_numbers.get(&first), Some(&1));
    assert_eq!(trip_numbers.get(&second), Some(&2));
    assert_eq!(trip_numbers.get(&third), Some(&3));
}

/// Helper to create trip with specific date
fn make_trip_with_date(date_str: &str, distance: f64, odo: f64) -> Trip {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
    Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::new_v4(),
        date,
        datetime: date.and_hms_opt(8, 0, 0).unwrap(),
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
        end_time: None,
        sort_order: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn make_trip_with_date_odo(date_str: &str, distance: f64, odo: f64) -> Trip {
    make_trip_with_date(date_str, distance, odo)
}
```

**Step 2: Run test to verify it fails**

```bash
cd src-tauri && cargo test test_trip_numbers --no-run
```

Expected: Compile error - `calculate_trip_numbers` doesn't exist

**Step 3: Commit test**

```bash
git add src-tauri/src/commands/commands_tests.rs
git commit -m "test: add failing tests for trip numbering calculation"
```

---

## Task 8: Implement Trip Numbering Calculation

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`

**Step 1: Add calculate_trip_numbers function**

Add after the helper functions section (~line 97):
```rust
/// Calculate trip sequence numbers (1-based, chronological order by date then odometer)
pub(crate) fn calculate_trip_numbers(trips: &[Trip]) -> HashMap<String, i32> {
    // Sort by date, then by odometer for same-day trips
    let mut sorted: Vec<_> = trips.iter().collect();
    sorted.sort_by(|a, b| {
        a.date.cmp(&b.date)
            .then_with(|| a.datetime.cmp(&b.datetime))
            .then_with(|| a.odometer.partial_cmp(&b.odometer).unwrap_or(std::cmp::Ordering::Equal))
    });

    sorted.iter()
        .enumerate()
        .map(|(i, trip)| (trip.id.to_string(), (i + 1) as i32))
        .collect()
}
```

**Step 2: Run tests**

```bash
cd src-tauri && cargo test test_trip_numbers
```

Expected: Both tests pass

**Step 3: Commit**

```bash
git add src-tauri/src/commands/mod.rs
git commit -m "feat: implement trip numbering calculation

Numbers trips 1,2,3... in chronological order (date, time, odometer)."
```

---

## Task 9: Write Tests for Odometer Start Derivation

**Files:**
- Modify: `src-tauri/src/commands/commands_tests.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn test_odometer_start_first_trip_uses_initial() {
    let initial_odo = 10000.0;
    let trips = vec![
        make_trip_with_date_odo("2026-01-10", 50.0, 10050.0),
    ];

    let odo_start = calculate_odometer_start(&trips, initial_odo);

    let trip_id = trips[0].id.to_string();
    assert_eq!(odo_start.get(&trip_id), Some(&10000.0));
}

#[test]
fn test_odometer_start_subsequent_trips() {
    let initial_odo = 10000.0;
    let trips = vec![
        make_trip_with_date_odo("2026-01-10", 50.0, 10050.0),   // start: 10000
        make_trip_with_date_odo("2026-01-15", 100.0, 10150.0),  // start: 10050
        make_trip_with_date_odo("2026-01-20", 50.0, 10200.0),   // start: 10150
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
        make_trip_with_date_odo("2026-01-20", 50.0, 10200.0),   // chronologically 3rd
        make_trip_with_date_odo("2026-01-10", 50.0, 10050.0),   // chronologically 1st
        make_trip_with_date_odo("2026-01-15", 100.0, 10150.0),  // chronologically 2nd
    ];

    let odo_start = calculate_odometer_start(&trips, initial_odo);

    // Trip on Jan 10 is first chronologically, so uses initial_odo
    let jan10 = trips.iter().find(|t| t.date.day() == 10).unwrap();
    assert_eq!(odo_start.get(&jan10.id.to_string()), Some(&10000.0));

    // Trip on Jan 15 uses Jan 10's ending odo
    let jan15 = trips.iter().find(|t| t.date.day() == 15).unwrap();
    assert_eq!(odo_start.get(&jan15.id.to_string()), Some(&10050.0));

    // Trip on Jan 20 uses Jan 15's ending odo
    let jan20 = trips.iter().find(|t| t.date.day() == 20).unwrap();
    assert_eq!(odo_start.get(&jan20.id.to_string()), Some(&10150.0));
}
```

**Step 2: Run to verify failure**

```bash
cd src-tauri && cargo test test_odometer_start --no-run
```

Expected: Compile error - function doesn't exist

**Step 3: Commit tests**

```bash
git add src-tauri/src/commands/commands_tests.rs
git commit -m "test: add failing tests for odometer_start derivation"
```

---

## Task 10: Implement Odometer Start Calculation

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`

**Step 1: Add calculate_odometer_start function**

```rust
/// Calculate starting odometer for each trip (previous trip's ending odo)
/// First trip uses initial_odometer from vehicle.
pub(crate) fn calculate_odometer_start(trips: &[Trip], initial_odometer: f64) -> HashMap<String, f64> {
    // Sort chronologically
    let mut sorted: Vec<_> = trips.iter().collect();
    sorted.sort_by(|a, b| {
        a.date.cmp(&b.date)
            .then_with(|| a.datetime.cmp(&b.datetime))
            .then_with(|| a.odometer.partial_cmp(&b.odometer).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut result = HashMap::new();
    let mut prev_odo = initial_odometer;

    for trip in sorted {
        result.insert(trip.id.to_string(), prev_odo);
        prev_odo = trip.odometer;
    }

    result
}
```

**Step 2: Run tests**

```bash
cd src-tauri && cargo test test_odometer_start
```

Expected: All 3 tests pass

**Step 3: Commit**

```bash
git add src-tauri/src/commands/mod.rs
git commit -m "feat: implement odometer_start calculation

Derives starting odometer from previous trip's ending odometer.
First trip uses vehicle.initial_odometer."
```

---

## Task 11: Write Tests for Month-End Detection

**Files:**
- Modify: `src-tauri/src/commands/commands_tests.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn test_month_end_trips_detected() {
    let trips = vec![
        make_trip_with_date("2026-01-15", 50.0, 10050.0),   // Not month-end
        make_trip_with_date("2026-01-31", 50.0, 10100.0),   // Month-end!
        make_trip_with_date("2026-02-15", 50.0, 10150.0),   // Not month-end
        make_trip_with_date("2026-02-28", 50.0, 10200.0),   // Month-end!
    ];

    let month_end_trips = detect_month_end_trips(&trips);

    let jan31 = trips.iter().find(|t| t.date.day() == 31).unwrap();
    let feb28 = trips.iter().find(|t| t.date.month() == 2 && t.date.day() == 28).unwrap();

    assert!(month_end_trips.contains(&jan31.id.to_string()));
    assert!(month_end_trips.contains(&feb28.id.to_string()));
    assert_eq!(month_end_trips.len(), 2);
}

#[test]
fn test_month_end_leap_year_february() {
    // 2024 is a leap year - Feb has 29 days
    let trips = vec![
        make_trip_with_date("2024-02-28", 50.0, 10050.0),   // NOT month-end in leap year
        make_trip_with_date("2024-02-29", 50.0, 10100.0),   // Month-end!
    ];

    let month_end_trips = detect_month_end_trips(&trips);

    let feb29 = trips.iter().find(|t| t.date.day() == 29).unwrap();
    let feb28 = trips.iter().find(|t| t.date.day() == 28).unwrap();

    assert!(month_end_trips.contains(&feb29.id.to_string()));
    assert!(!month_end_trips.contains(&feb28.id.to_string()));
}
```

**Step 2: Run to verify failure**

```bash
cd src-tauri && cargo test test_month_end --no-run
```

**Step 3: Commit tests**

```bash
git add src-tauri/src/commands/commands_tests.rs
git commit -m "test: add failing tests for month-end trip detection"
```

---

## Task 12: Implement Month-End Trip Detection

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`

**Step 1: Add helper function for last day of month**

```rust
/// Get the last day of a given month
fn last_day_of_month(year: i32, month: u32) -> u32 {
    // Create first day of next month, subtract one day
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_year = if month == 12 { year + 1 } else { year };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
        .day()
}

/// Detect trips that fall on the last day of their month
pub(crate) fn detect_month_end_trips(trips: &[Trip]) -> HashSet<String> {
    trips.iter()
        .filter(|t| {
            let last_day = last_day_of_month(t.date.year(), t.date.month());
            t.date.day() == last_day
        })
        .map(|t| t.id.to_string())
        .collect()
}
```

**Step 2: Run tests**

```bash
cd src-tauri && cargo test test_month_end
```

Expected: Both tests pass

**Step 3: Commit**

```bash
git add src-tauri/src/commands/mod.rs
git commit -m "feat: implement month-end trip detection

Identifies trips falling on the last calendar day of their month.
Handles leap years correctly."
```

---

## Task 13: Write Tests for Month-End Row Generation

**Files:**
- Modify: `src-tauri/src/commands/commands_tests.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn test_month_end_rows_generated_for_gaps() {
    // Trips only in January and March - need synthetic rows for Jan 31, Feb 28, Mar 31
    let trips = vec![
        make_trip_with_date_odo("2026-01-15", 50.0, 10050.0),
        make_trip_with_date_odo("2026-03-10", 50.0, 10100.0),
    ];
    let year = 2026;
    let initial_odo = 10000.0;
    // Fuel remaining map (keyed by trip ID) - simulates what get_trip_grid_data provides
    let mut fuel_remaining: HashMap<String, f64> = HashMap::new();
    fuel_remaining.insert(trips[0].id.to_string(), 45.0);  // After Jan 15 trip
    fuel_remaining.insert(trips[1].id.to_string(), 40.0);  // After Mar 10 trip
    let initial_fuel = 50.0;

    let rows = generate_month_end_rows(&trips, year, initial_odo, initial_fuel, &fuel_remaining);

    // Should have rows for: Jan 31, Feb 28, Mar 31 (no trip on last day of any month)
    assert_eq!(rows.len(), 3);

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

    // Mar 31 carries Mar 10's values
    let mar = rows.iter().find(|r| r.month == 3).unwrap();
    assert_eq!(mar.date, NaiveDate::from_ymd_opt(2026, 3, 31).unwrap());
    assert_eq!(mar.odometer, 10100.0);
    assert_eq!(mar.fuel_remaining, 40.0);
}

#[test]
fn test_month_end_rows_not_generated_when_trip_exists() {
    // Trip on Jan 31 - no synthetic row needed
    let trips = vec![
        make_trip_with_date_odo("2026-01-31", 50.0, 10050.0),
    ];
    let year = 2026;
    let fuel_remaining: HashMap<String, f64> = HashMap::new();

    let rows = generate_month_end_rows(&trips, year, 10000.0, 50.0, &fuel_remaining);

    // Jan should NOT have synthetic row (trip exists on 31st)
    let jan_row = rows.iter().find(|r| r.month == 1);
    assert!(jan_row.is_none());
}

#[test]
fn test_month_end_rows_all_12_months() {
    // No trips at all - should generate row for every month
    let trips: Vec<Trip> = vec![];
    let year = 2026;
    let fuel_remaining: HashMap<String, f64> = HashMap::new();

    let rows = generate_month_end_rows(&trips, year, 10000.0, 50.0, &fuel_remaining);

    assert_eq!(rows.len(), 12);
    for month in 1..=12 {
        assert!(rows.iter().any(|r| r.month == month), "Missing month {}", month);
    }
    // All rows should have initial fuel (no trips consumed any)
    for row in &rows {
        assert_eq!(row.fuel_remaining, 50.0);
    }
}
```

**Step 2: Run to verify failure**

```bash
cd src-tauri && cargo test test_month_end_rows --no-run
```

**Step 3: Commit tests**

```bash
git add src-tauri/src/commands/commands_tests.rs
git commit -m "test: add failing tests for month-end row generation"
```

---

## Task 14: Implement Month-End Row Generation

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`

**Step 1: Add generate_month_end_rows function**

```rust
use crate::models::MonthEndRow;

/// Generate synthetic month-end rows for months without a trip on the last day.
/// Returns rows for ALL 12 months where no trip exists on the final day.
///
/// # Arguments
/// * `trips` - All trips for the year (will be sorted chronologically)
/// * `year` - The year being processed
/// * `initial_odometer` - Starting odometer (from vehicle or year carryover)
/// * `initial_fuel` - Starting fuel (from vehicle or year carryover)
/// * `fuel_remaining` - Pre-calculated fuel remaining after each trip (from TripGridData)
pub(crate) fn generate_month_end_rows(
    trips: &[Trip],
    year: i32,
    initial_odometer: f64,
    initial_fuel: f64,
    fuel_remaining: &HashMap<String, f64>,
) -> Vec<MonthEndRow> {
    // Sort trips chronologically
    let mut sorted: Vec<_> = trips.iter().collect();
    sorted.sort_by(|a, b| {
        a.date.cmp(&b.date)
            .then_with(|| a.odometer.partial_cmp(&b.odometer).unwrap_or(std::cmp::Ordering::Equal))
    });

    // Track state as we process each month
    let mut current_odo = initial_odometer;
    let mut current_fuel = initial_fuel;
    let mut last_trip_id: Option<String> = None;

    let mut rows = Vec::new();

    for month in 1..=12u32 {
        let last_day = last_day_of_month(year, month);
        let month_end_date = NaiveDate::from_ymd_opt(year, month, last_day).unwrap();

        // Find the last trip on or before this month-end
        for trip in &sorted {
            if trip.date <= month_end_date {
                current_odo = trip.odometer;
                last_trip_id = Some(trip.id.to_string());
            } else {
                break;
            }
        }

        // Get fuel remaining from the last trip, or use initial if no trips yet
        current_fuel = last_trip_id
            .as_ref()
            .and_then(|id| fuel_remaining.get(id))
            .copied()
            .unwrap_or(initial_fuel);

        // Check if there's a trip exactly on the last day
        let has_trip_on_last_day = sorted.iter().any(|t| t.date == month_end_date);

        if !has_trip_on_last_day {
            rows.push(MonthEndRow {
                date: month_end_date,
                odometer: current_odo,
                fuel_remaining: current_fuel,
                month,
            });
        }
    }

    rows
}
```

**Step 2: Run tests**

```bash
cd src-tauri && cargo test test_month_end_rows
```

Expected: All 3 tests pass

**Step 3: Commit**

```bash
git add src-tauri/src/commands/mod.rs
git commit -m "feat: implement month-end row generation

Generates synthetic rows for all 12 months where no trip
falls on the last calendar day. Carries forward odometer
and fuel state from most recent trip."
```

---

## Task 15: Integrate New Calculations into get_trip_grid_data

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`

**Step 1: Update get_trip_grid_data to populate new fields**

In `get_trip_grid_data` function, add after the fuel_remaining calculation (after line ~598):

```rust
    // Legal compliance calculations (2026)
    let trip_numbers = calculate_trip_numbers(&trips);
    let odometer_start = calculate_odometer_start(&chronological, year_start_odometer);
    let month_end_trips = detect_month_end_trips(&trips);

    // Generate month-end rows using already-calculated fuel_remaining
    let month_end_rows = generate_month_end_rows(
        &chronological,
        year,
        year_start_odometer,
        year_start_fuel,
        &fuel_remaining,
    );
```

**Step 2: Add new fields to TripGridData return**

Update the `Ok(TripGridData { ... })` block to include:
```rust
        trip_numbers,
        odometer_start,
        month_end_trips,
        month_end_rows,
```

**Step 3: Update empty trips case**

In the early return for empty trips (~line 478), add:
```rust
            trip_numbers: HashMap::new(),
            odometer_start: HashMap::new(),
            month_end_trips: HashSet::new(),
            month_end_rows: generate_month_end_rows(
                &[],
                year,
                year_start_odometer,
                year_start_fuel,
                &HashMap::new(),  // No trips = no fuel_remaining entries
            ),
```

**Step 4: Run all backend tests**

```bash
cd src-tauri && cargo test
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add src-tauri/src/commands/mod.rs
git commit -m "feat: integrate legal compliance calculations into get_trip_grid_data

Populates trip_numbers, odometer_start, month_end_trips, and
month_end_rows in TripGridData response."
```

---

## Task 16: Update ExportLabels and Export Function

**Files:**
- Modify: `src-tauri/src/export.rs`

**Step 1: Add new labels to ExportLabels**

Add these fields to `ExportLabels` struct:
```rust
    // Legal compliance columns (2026)
    pub col_trip_number: String,
    pub col_start_time: String,
    pub col_end_time: String,
    pub col_driver: String,
    pub col_odo_start: String,
```

**Step 2: Update generate_html column headers**

Add trip number column (first column):
```rust
    // Trip number (always first)
    let mut col_headers = format!(
        r#"        <th>{}</th>"#,
        html_escape(&l.col_trip_number),
    );
```

**Step 3: Add start/end time columns after date**

After date header, add:
```rust
    // Start time (hideable)
    if is_visible("startTime") {
        col_headers.push_str(&format!(
            r#"
        <th>{}</th>"#,
            html_escape(&l.col_start_time),
        ));
    }

    // End time (hideable)
    if is_visible("endTime") {
        col_headers.push_str(&format!(
            r#"
        <th>{}</th>"#,
            html_escape(&l.col_end_time),
        ));
    }
```

**Step 4: Add driver column after purpose**

```rust
    // Driver (hideable)
    if is_visible("driver") {
        col_headers.push_str(&format!(
            r#"
        <th>{}</th>"#,
            html_escape(&l.col_driver),
        ));
    }
```

**Step 5: Add odo start column before odo end**

```rust
    // Odometer start (hideable)
    if is_visible("odoStart") {
        col_headers.push_str(&format!(
            r#"
        <th>{}</th>"#,
            html_escape(&l.col_odo_start),
        ));
    }
```

**Step 6: Update row generation with explicit cell code**

In the trip row loop, add cells for each new column. Update the row building code:

```rust
    for trip in &data.grid_data.trips {
        let trip_id = trip.id.to_string();
        let is_month_end = data.grid_data.month_end_trips.contains(&trip_id);
        let row_class = if is_month_end { " class=\"month-end-trip\"" } else { "" };

        // Trip number (always first, hideable)
        let mut row = format!(r#"        <tr{row_class}>
"#);

        if is_visible("tripNumber") {
            let trip_num = data.grid_data.trip_numbers.get(&trip_id).unwrap_or(&0);
            row.push_str(&format!(r#"          <td class="num">{}</td>
"#, trip_num));
        }

        // Date (always shown)
        row.push_str(&format!(r#"          <td>{}</td>
"#, trip.date.format("%d.%m.%Y")));

        // Start time (hideable)
        if is_visible("startTime") {
            row.push_str(&format!(r#"          <td>{}</td>
"#, trip.datetime.format("%H:%M")));
        }

        // End time (hideable)
        if is_visible("endTime") {
            let end_time = trip.end_time.as_deref().unwrap_or("");
            row.push_str(&format!(r#"          <td>{}</td>
"#, html_escape(end_time)));
        }

        // ... (origin, destination, purpose - existing code)

        // Driver (hideable) - after purpose
        if is_visible("driver") {
            let driver = data.vehicle.driver_name.as_deref().unwrap_or("");
            row.push_str(&format!(r#"          <td>{}</td>
"#, html_escape(driver)));
        }

        // ... (km - existing code)

        // Odo start (hideable)
        if is_visible("odoStart") {
            let odo_start = data.grid_data.odometer_start.get(&trip_id).unwrap_or(&0.0);
            row.push_str(&format!(r#"          <td class="num">{:.0}</td>
"#, odo_start));
        }

        // Odo end (existing odometer column)
        row.push_str(&format!(r#"          <td class="num">{:.0}</td>
"#, trip.odometer));

        // ... (rest of existing columns)
    }

    // Render synthetic month-end rows
    for month_row in &data.grid_data.month_end_rows {
        let mut row = String::from(r#"        <tr class="month-end-synthetic">
"#);

        if is_visible("tripNumber") {
            row.push_str(r#"          <td class="num">—</td>
"#);
        }

        // Date
        row.push_str(&format!(r#"          <td>{}</td>
"#, month_row.date.format("%d.%m.%Y")));

        // Start/end time - empty for synthetic rows
        if is_visible("startTime") {
            row.push_str(r#"          <td></td>
"#);
        }
        if is_visible("endTime") {
            row.push_str(r#"          <td></td>
"#);
        }

        // Driver (hideable)
        if is_visible("driver") {
            let driver = data.vehicle.driver_name.as_deref().unwrap_or("");
            row.push_str(&format!(r#"          <td>{}</td>
"#, html_escape(driver)));
        }

        // Origin/destination/purpose/km - empty for synthetic
        row.push_str(r#"          <td></td>
          <td></td>
          <td></td>
          <td class="num"></td>
"#);

        // Odo start and end are the same (no travel)
        if is_visible("odoStart") {
            row.push_str(&format!(r#"          <td class="num">{:.0}</td>
"#, month_row.odometer));
        }
        row.push_str(&format!(r#"          <td class="num">{:.0}</td>
"#, month_row.odometer));

        // Fuel columns show remaining (no fillup)
        // ... continue with fuel_remaining display

        row.push_str(r#"        </tr>
"#);
        rows.push_str(&row);
    }
```

**Step 7: Add month-end styling CSS**

Add to the `<style>` section:
```css
    tr.month-end-synthetic {{
      background: #f0f0f0;
      font-style: italic;
    }}

    tr.month-end-trip {{
      background: #e8f4fc;
      border-bottom: 2px solid #4a90d9;
    }}
```

**Step 8: Run cargo check**

```bash
cd src-tauri && cargo check
```

**Step 9: Commit**

```bash
git add src-tauri/src/export.rs
git commit -m "feat(export): add legal compliance columns to HTML export

Adds trip number, start/end time, driver, and odo start columns.
Includes month-end row styling."
```

---

## Task 17: Add Slovak i18n Translations

**Files:**
- Modify: `src/lib/i18n/sk/index.ts`
- Modify: `src/lib/i18n/en/index.ts`

**Step 1: Add Slovak translations**

Add to `src/lib/i18n/sk/index.ts` in the appropriate sections:

```typescript
// Column headers (in trips/grid section)
tripNumber: 'Č.',
startTime: 'Čas od',
endTime: 'Čas do',
driver: 'Vodič',
odoStart: 'Km pred',
odoEnd: 'Km po',

// Export labels
colTripNumber: 'Č.',
colStartTime: 'Čas od',
colEndTime: 'Čas do',
colDriver: 'Vodič',
colOdoStart: 'Km pred',
colOdoEnd: 'Km po',

// Trip form
endTimeLabel: 'Čas ukončenia',
endTimePlaceholder: 'HH:MM',
```

**Step 2: Add English translations**

Add to `src/lib/i18n/en/index.ts`:

```typescript
tripNumber: '#',
startTime: 'Start',
endTime: 'End',
driver: 'Driver',
odoStart: 'Odo Start',
odoEnd: 'Odo End',

colTripNumber: '#',
colStartTime: 'Start Time',
colEndTime: 'End Time',
colDriver: 'Driver',
colOdoStart: 'Odo Start',
colOdoEnd: 'Odo End',

endTimeLabel: 'End Time',
endTimePlaceholder: 'HH:MM',
```

**Step 3: Commit**

```bash
git add src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts
git commit -m "feat(i18n): add translations for legal compliance columns"
```

---

## Task 18: Update Frontend Trip Form

**Files:**
- Modify: `src/lib/components/TripForm.svelte` (or equivalent)

**Step 1: Add end_time field to form state**

```typescript
let endTime = trip?.endTime ?? '';
```

**Step 2: Add end time input field**

Add after the existing time input:
```svelte
<div class="form-group">
  <label for="endTime">{LL.endTimeLabel()}</label>
  <input
    type="time"
    id="endTime"
    bind:value={endTime}
    placeholder={LL.endTimePlaceholder()}
  />
</div>
```

**Step 3: Include in save payload**

Update the invoke call to include:
```typescript
endTime: endTime || null,
```

**Step 4: Test manually**

Start dev server and verify:
- End time input appears in trip form
- Value saves and loads correctly

**Step 5: Commit**

```bash
git add src/lib/components/TripForm.svelte
git commit -m "feat(ui): add end time input to trip form"
```

---

## Task 19: Update Frontend Trip Grid

**Files:**
- Modify: `src/lib/components/TripGrid.svelte`
- Modify: `src/lib/components/TripRow.svelte`

**Step 1: Add new columns to TripGrid header**

Add columns in order: trip#, date, start time, end time, driver, origin, destination, purpose, km, odo start, odo end, ...

**Step 2: Update TripRow to display new fields**

```svelte
<!-- Trip number -->
{#if isVisible('tripNumber')}
  <td class="num">{gridData.tripNumbers[trip.id] ?? '—'}</td>
{/if}

<!-- Start time (existing datetime) -->
{#if isVisible('startTime')}
  <td>{formatTime(trip.datetime)}</td>
{/if}

<!-- End time -->
{#if isVisible('endTime')}
  <td>{trip.endTime ?? ''}</td>
{/if}

<!-- Driver -->
{#if isVisible('driver')}
  <td>{vehicle.driverName ?? ''}</td>
{/if}

<!-- Odo Start -->
{#if isVisible('odoStart')}
  <td class="num">{gridData.odometerStart[trip.id]?.toFixed(0) ?? ''}</td>
{/if}
```

**Step 3: Add month-end row styling**

```svelte
<tr class:month-end-trip={gridData.monthEndTrips.has(trip.id)}>
```

**Step 4: Render synthetic month-end rows**

After real trip rows, render synthetic rows from `gridData.monthEndRows`:
```svelte
{#each gridData.monthEndRows as row}
  <tr class="month-end-synthetic">
    <td>—</td>
    <td>{formatDate(row.date)}</td>
    <!-- ... other columns as empty or with carried values -->
  </tr>
{/each}
```

**Step 5: Commit**

```bash
git add src/lib/components/TripGrid.svelte src/lib/components/TripRow.svelte
git commit -m "feat(ui): add legal compliance columns to trip grid

Displays trip number, start/end times, driver, odo start.
Highlights month-end trips and shows synthetic rows."
```

---

## Task 20: Add Column Visibility Settings

**Files:**
- Modify: `src/lib/components/ColumnSettings.svelte` (or equivalent)

**Step 1: Add new columns to visibility options**

```typescript
const newColumns = [
  { key: 'tripNumber', label: LL.tripNumber() },
  { key: 'startTime', label: LL.startTime() },
  { key: 'endTime', label: LL.endTime() },
  { key: 'driver', label: LL.driver() },
  { key: 'odoStart', label: LL.odoStart() },
];
```

**Step 2: Ensure defaults are visible**

New columns should default to visible.

**Step 3: Commit**

```bash
git add src/lib/components/ColumnSettings.svelte
git commit -m "feat(ui): add visibility toggles for legal compliance columns"
```

---

## Task 21: Write Integration Tests

**Files:**
- Modify: `tests/integration/trips.spec.ts`

**Note:** Integration tests verify UI displays correctly. Do NOT re-test calculation logic
(that's covered by backend unit tests in Tasks 7-14).

**Step 1: Add tests for new column visibility (UI flow only)**

```typescript
it('displays new legal compliance columns', async () => {
  // Create a trip
  await createTrip({
    date: '2026-01-15',
    time: '08:30',
    origin: 'Bratislava',
    destination: 'Košice',
    distance: 400,
    odometer: 10400,
  });

  // Verify new columns appear (not testing calculation values)
  const tripRow = await $('.trip-row');

  // Trip number column exists and has content
  const tripNumber = await tripRow.$('[data-col="tripNumber"]');
  expect(await tripNumber.isDisplayed()).toBe(true);

  // Start time column shows the entered time
  const startTime = await tripRow.$('[data-col="startTime"]');
  expect(await startTime.getText()).toContain('08:30');

  // End time column exists (may be empty)
  const endTime = await tripRow.$('[data-col="endTime"]');
  expect(await endTime.isDisplayed()).toBe(true);

  // Odo start column exists and has a value
  const odoStart = await tripRow.$('[data-col="odoStart"]');
  expect(await odoStart.isDisplayed()).toBe(true);
});

it('end time can be entered and saved', async () => {
  // Create trip with end time
  await createTrip({
    date: '2026-01-15',
    time: '08:30',
    endTime: '09:45',  // New field
    // ... other fields
  });

  // Verify end time displays
  const endTime = await $('.trip-row [data-col="endTime"]');
  expect(await endTime.getText()).toContain('09:45');
});

it('month-end trips are visually highlighted', async () => {
  // Create trip on last day of month
  await createTrip({
    date: '2026-01-31',
    // ... other fields
  });

  // Verify row has month-end styling
  const tripRow = await $('.trip-row');
  const classes = await tripRow.getAttribute('class');
  expect(classes).toContain('month-end');
});
```

**Step 2: Run integration tests**

```bash
npm run test:integration:tier1
```

**Step 3: Commit**

```bash
git add tests/integration/trips.spec.ts
git commit -m "test(integration): add tests for legal compliance columns"
```

---

## Task 22: Final Verification and Cleanup

**Step 1: Run all tests**

```bash
npm run test:all
```

Expected: All backend and integration tests pass

**Step 2: Manual testing checklist**

- [ ] Trip form shows end time input
- [ ] Trip grid shows all new columns
- [ ] Trip numbers are sequential (1, 2, 3...)
- [ ] Odo start shows previous trip's odo
- [ ] Month-end trips are highlighted
- [ ] Synthetic month-end rows appear for gaps
- [ ] Export HTML includes all new columns
- [ ] Column visibility toggles work

**Step 3: Update CHANGELOG**

Run `/changelog` to document the changes.

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete legal requirements updates for 2026

Implements Slovak legal compliance for vehicle logbooks:
- Trip sequence numbering (§4a)
- Driver name display (§4b)
- Start and end times (§4c)
- Odometer before/after (§4f)
- Month-end state display

Closes #46"
```

---

## Summary

| Task | Description | Est. Time |
|------|-------------|-----------|
| 1-4 | Database migration + models for end_time | Backend foundation |
| 5-6 | MonthEndRow model + TripGridData extension | Data structures |
| 7-14 | Calculation functions with TDD | Core logic |
| 15 | Integration into get_trip_grid_data | Wire it together |
| 16 | Export HTML updates | Report generation |
| 17 | i18n translations | Localization |
| 18-20 | Frontend form + grid + settings | UI display |
| 21-22 | Integration tests + verification | Quality assurance |

**Total: 22 tasks**
