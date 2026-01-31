# Implementation Plan: end_time to end_datetime Cleanup

**Date:** 2026-01-29
**Status:** Complete
**Completed:** 2026-01-31
**Reviewed:** 2026-01-29 (see `_plan-review.md`)

## Implementation Summary

- Trip models and DB now use `start_datetime` + `end_datetime` only.
- Legacy columns dropped in migration `2026-01-30-100000_drop_legacy_datetime_columns`.
- Commands, exports, and tests updated to use the new fields.

---

## Approach

**Simplified approach** (confirmed after review):

1. Task 47 already added `start_datetime` and `end_datetime` columns with backfilled data
2. This cleanup switches code to use ONLY these new columns
3. Migration drops obsolete columns (`datetime`, `date`, `end_time`)

No column renaming needed - new columns already exist!

---

## Phase 1: Update Trip Model

### Task 1.1: Update Trip struct (`src-tauri/src/models.rs`)
- [ ] Rename `datetime: NaiveDateTime` → `start_datetime: NaiveDateTime`
- [ ] Replace `end_time: Option<String>` → `end_datetime: Option<NaiveDateTime>`
- [ ] Remove `date: NaiveDate` field
- [ ] Update serde attributes if needed

### Task 1.2: Update TripRow struct (`src-tauri/src/models.rs`)
- [ ] Remove `date: String` field
- [ ] Remove `datetime: String` field
- [ ] Remove `end_time: String` field
- [ ] Add `start_datetime: String` (maps to existing DB column)
- [ ] Add `end_datetime: Option<String>` (maps to existing nullable DB column)

### Task 1.3: Update NewTripRow struct (`src-tauri/src/models.rs`)
- [ ] Same changes as TripRow

### Task 1.4: Update From<TripRow> for Trip (`src-tauri/src/models.rs`)
- [ ] Parse `start_datetime` as NaiveDateTime
- [ ] Parse `end_datetime` as Option<NaiveDateTime>
- [ ] Remove date parsing (derive from start_datetime if needed internally)

---

## Phase 2: Update Database Layer

### Task 2.1: Update schema.rs (`src-tauri/src/schema.rs`)
- [ ] Remove `date -> Text`
- [ ] Remove `datetime -> Text`
- [ ] Remove `end_time -> Text`
- [ ] Keep `start_datetime -> Text` (already exists)
- [ ] Keep `end_datetime -> Nullable<Text>` (already exists)

### Task 2.2: Update db.rs create_trip (`src-tauri/src/db.rs`)
- [ ] Remove `date` field from NewTripRow construction
- [ ] Remove `datetime` field
- [ ] Remove `end_time` field
- [ ] Use `start_datetime` and `end_datetime` directly

### Task 2.3: Update db.rs update_trip (`src-tauri/src/db.rs`)
- [ ] Update diesel update query to use new field names
- [ ] Remove `end_time` handling

---

## Phase 3: Update Commands and Export

### Task 3.1: Update commands/mod.rs (`src-tauri/src/commands/mod.rs`)

**Remove helper functions:**
- [ ] Remove `extract_time_string()` (~L77) - no longer needed
- [ ] Remove `parse_trip_datetime()` (~L43) - no longer needed
- [ ] Remove tests for these in `commands_tests.rs` (~L2388-2434)

**A. Sorting functions (10 locations) - change `.date` → `.start_datetime.date()`:**

All sorting closures follow pattern `a.date.cmp(&b.date).then_with(|| a.datetime.cmp(...))`:
- [ ] `calculate_trip_numbers()` (~L125-127): also update `.datetime` → `.start_datetime`
- [ ] `calculate_odometer_start()` (~L148-150): also update `.datetime` → `.start_datetime`
- [ ] `calculate_period_totals()` (~L203)
- [ ] `get_closed_period_rate()` (~L329)
- [ ] `get_open_period_rate()` (~L486)
- [ ] `get_receipt_with_stats()` (~L542)
- [ ] `match_receipt_to_trip()` (~L598)
- [ ] `calculate_fuel_remaining()` (~L689)
- [ ] `calculate_energy_remaining()` (~L965)
- [ ] `magic_fill_trip()` (~L2514)

**B. Month/period filtering (4 locations):**
- [ ] `calculate_period_totals()` (~L216): `sorted.last().unwrap().date.month()`
- [ ] `calculate_period_totals()` (~L233): `trip.date <= month_end_date`
- [ ] `calculate_period_totals()` (~L252): `t.date.month() == month && t.date <= month_end_date`

**C. Date warning logic (3 locations):**
- [ ] `get_trip_grid_data()` (~L1446-1453): `trip.date > p.date`, `trip.date < n.date`

**D. Receipt matching (9 locations):**
- [ ] `get_trip_grid_data()` (~L1497): `r.receipt_date.as_ref() == Some(&trip.date)`
- [ ] `get_receipt_with_stats()` (~L1940): `receipt.receipt_date == Some(trip.date)`
- [ ] `match_receipt_to_trip()` (~L2054): `receipt.receipt_date == Some(trip.date)`
- [ ] `auto_assign_receipts()` (~L2223): `trip.date == receipt_date`
- [ ] `auto_assign_receipts()` (~L2230): `trip.date.format("%Y-%m-%d")`
- [ ] `auto_assign_receipts()` (~L2241): `trip.date.format("%-d.%-m.")`
- [ ] `auto_assign_receipts()` (~L2263): `t.date == receipt_date`
- [ ] `auto_assign_receipts()` (~L2273): `t.date == receipt_date`
- [ ] `auto_assign_receipts()` (~L2304): `trip.date.format("%Y-%m-%d")`

**E. Magic fill function (5 locations):**
- [ ] `magic_fill_trip()` (~L2437): `t.date` in map
- [ ] `magic_fill_trip()` (~L2443): `t.date` in max_by_key
- [ ] `magic_fill_trip()` (~L2444): `t.date` in map
- [ ] `magic_fill_trip()` (~L2483-2485): `existing.date`, `existing.datetime`, `existing.end_time.clone()`
  - Change to: `start_datetime: existing.start_datetime`, `end_datetime: existing.end_datetime`

### Task 3.2: Update commands/trips.rs (`src-tauri/src/commands/trips.rs`)

**Remove extract_time_string usage:**
- [ ] `create_trip()` (~L74): Remove `let end_time = Some(extract_time_string(...))`, use `end_datetime` directly
- [ ] `update_trip()` (~L167): Same change
- [ ] Update import (~L11): Remove `extract_time_string` from use statement

### Task 3.3: Update db.rs (`src-tauri/src/db.rs`)

**create_trip (~L264-276):**
- [ ] Remove: `date_str`, `datetime_str`, `end_time` construction
- [ ] Use: `start_datetime` and `end_datetime` strings directly

**update_trip (~L376-388):**
- [ ] Remove: `date_str`, `datetime_str`, `end_time` handling
- [ ] Update diesel query to use new column names

### Task 3.4: Update export.rs (`src-tauri/src/export.rs`)

**Production code:**
- [ ] ~L232: `trip.end_time.as_deref()` → `trip.end_datetime.map(|dt| dt.format("%H:%M").to_string())`
- [ ] ~L246: `trip.datetime.format(...)` → `trip.start_datetime.format(...)`
- [ ] ~L254: `trip.date.format(...)` → `trip.start_datetime.date().format(...)`
- [ ] ~L435: `month_end.date.format(...)` → `month_end.start_datetime.date().format(...)`

---

## Phase 4: Update Tests

### Task 4.1: Update test helpers (`src-tauri/src/models.rs`)
- [ ] Update `Trip::test_ice_trip()` helper to use new fields

### Task 4.2: Update commands_tests.rs (`src-tauri/src/commands/commands_tests.rs`)

**Test helpers:**
- [ ] Update `make_trip()` helper
- [ ] Update `make_trip_with_fuel()` helper
- [ ] Update `make_trip_with_date()` helper
- [ ] Update `make_trip_with_date_odo()` helper
- [ ] Update `make_trip_detailed()` helper
- [ ] Update `make_trip_for_magic_fill()` helper
- [ ] Update all inline Trip struct initializations

**Remove parse_trip_datetime tests (~L2388-2434):**
- [ ] Delete `test_parse_trip_datetime_with_time`
- [ ] Delete `test_parse_trip_datetime_without_time`
- [ ] Delete `test_parse_trip_datetime_none_time`
- [ ] Delete `test_parse_trip_datetime_invalid_time_format`
- [ ] Delete `test_parse_trip_datetime_invalid_date_format`

**Test assertions using trip.date (~L2608-2763):**
- [ ] ~L2608-2610: `t.date.day() == 10/15/20` → `t.start_datetime.date().day()`
- [ ] ~L2683-2691: Same pattern
- [ ] ~L2757, L2763: `assert_eq!(jan.date, ...)` → `assert_eq!(jan.start_datetime.date(), ...)`

### Task 4.3: Update calculations_tests.rs (`src-tauri/src/calculations_tests.rs`)
- [ ] Update Trip struct initializations (uses `Trip::test_ice_trip()` helper)

### Task 4.4: Update db_tests.rs (`src-tauri/src/db_tests.rs`)
- [ ] Update Trip struct initializations

### Task 4.5: Update export.rs tests (`src-tauri/src/export.rs`)
- [ ] Update mock Trip structs in test module

---

## Phase 5: Verification

### Task 5.1: Run all backend tests
```bash
cd src-tauri && cargo test
```
- [ ] All 195+ tests pass

### Task 5.2: Run linting and formatting
```bash
npm run lint
npm run format
```
- [ ] No lint errors
- [ ] Code formatted

### Task 5.3: Manual testing
```bash
npm run tauri dev
```
- [ ] Create trip - verify datetime storage
- [ ] Edit trip - verify datetime updates
- [ ] Export - verify times display correctly

---

## Phase 6: Database Migration (Drop Old Columns)

### Task 6.1: Create migration to drop obsolete columns

Create migration: `migrations/YYYY-MM-DD-HHMMSS_drop_legacy_datetime_columns/up.sql`

```sql
-- Drop legacy datetime columns (data already migrated to start_datetime/end_datetime)
-- Per ADR-012: forward-only migrations, no backward compatibility needed

-- Create new table without legacy columns
CREATE TABLE trips_new (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT NOT NULL,
    origin TEXT NOT NULL,
    destination TEXT NOT NULL,
    distance_km REAL NOT NULL,
    odometer REAL NOT NULL,
    purpose TEXT NOT NULL,
    fuel_liters REAL,
    fuel_cost_eur REAL,
    other_costs_eur REAL,
    other_costs_note TEXT,
    full_tank INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    energy_kwh REAL,
    energy_cost_eur REAL,
    full_charge INTEGER,
    soc_override_percent REAL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    start_datetime TEXT NOT NULL,
    end_datetime TEXT,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id)
);

-- Copy data (excluding datetime, date, end_time)
INSERT INTO trips_new SELECT
    id, vehicle_id, origin, destination, distance_km, odometer, purpose,
    fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note,
    full_tank, sort_order, energy_kwh, energy_cost_eur, full_charge,
    soc_override_percent, created_at, updated_at, start_datetime, end_datetime
FROM trips;

-- Replace old table
DROP TABLE trips;
ALTER TABLE trips_new RENAME TO trips;

-- Recreate index if any existed
CREATE INDEX IF NOT EXISTS idx_trips_vehicle_id ON trips(vehicle_id);
```

**Note:** SQLite doesn't support `DROP COLUMN` directly, so we use the table rebuild pattern.

### Task 6.2: Verify migration
- [ ] Run `cargo test` after migration
- [ ] Test with fresh database
- [ ] Test with migrated database

---

## Rollback Plan

If issues arise:
1. Revert commits before Phase 6 migration
2. DB migration is irreversible - test thoroughly in Phase 5 before applying Phase 6
3. Consider splitting: Phases 1-5 in one PR, Phase 6 in follow-up PR

---

## Notes

- Per ADR-012: forward-only migrations, no backward compatibility needed
- Frontend already uses startDatetime/endDatetime API (Task 47)
- This is purely a backend cleanup task
- `end_datetime` is `Option<NaiveDateTime>` to match nullable DB column

## Scope Summary

| File | Changes |
|------|---------|
| `models.rs` | Trip, TripRow, NewTripRow struct updates |
| `schema.rs` | Remove 3 columns |
| `db.rs` | create_trip, update_trip updates |
| `commands/mod.rs` | ~31 locations (sorting, filtering, receipts, magic fill) |
| `commands/trips.rs` | Remove extract_time_string usage |
| `export.rs` | 4 production code locations |
| `commands_tests.rs` | Test helpers + ~11 test assertions + remove 5 tests |
| Other test files | Trip struct initializations |
| Migration | Drop 3 columns (datetime, date, end_time) |

**Total estimated changes:** ~50+ code locations across 8 files
