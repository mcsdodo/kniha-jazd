# Implementation Plan: end_time to end_datetime Cleanup

**Date:** 2026-01-29
**Status:** Ready for Implementation
**Reviewed:** 2026-01-29 (see `_plan-review.md`)

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

**Remove helper function:**
- [ ] Remove `extract_time_string()` function (no longer needed)
- [ ] Remove `parse_trip_datetime()` if unused after cleanup

**Update trip.date usages (11 locations):**
| Line | Current | Change to |
|------|---------|-----------|
| 229 | `trip.date <= month_end_date` | `trip.start_datetime.date() <= month_end_date` |
| 1432 | `trip.date > p.date` | `trip.start_datetime.date() > p.start_datetime.date()` |
| 1434 | `trip.date > p.date` | (same block as 1432) |
| 1439 | `trip.date < n.date` | `trip.start_datetime.date() < n.start_datetime.date()` |
| 1483 | `r.receipt_date == Some(&trip.date)` | `r.receipt_date == Some(trip.start_datetime.date())` |
| 2081 | `receipt.receipt_date == Some(trip.date)` | `receipt.receipt_date == Some(trip.start_datetime.date())` |
| 2195 | `receipt.receipt_date == Some(trip.date)` | `receipt.receipt_date == Some(trip.start_datetime.date())` |
| 2364 | `trip.date == receipt_date` | `trip.start_datetime.date() == receipt_date` |
| 2371 | `trip.date.format(...)` | `trip.start_datetime.date().format(...)` |
| 2382 | `trip.date.format(...)` | `trip.start_datetime.date().format(...)` |
| 2445 | `trip.date.format(...)` | `trip.start_datetime.date().format(...)` |

**Update Trip struct construction:**
- [ ] In `create_trip` command: use `end_datetime` directly (not extract_time_string)
- [ ] In `update_trip` command: use `end_datetime` directly

### Task 3.2: Update export.rs (`src-tauri/src/export.rs`)

**Production code:**
| Line | Current | Change to |
|------|---------|-----------|
| 246 | `trip.datetime.format("%d.%m. %H:%M")` | `trip.start_datetime.format("%d.%m. %H:%M")` |
| 254 | `trip.date.format("%d.%m.")` | `trip.start_datetime.date().format("%d.%m.")` |

---

## Phase 4: Update Tests

### Task 4.1: Update test helpers (`src-tauri/src/models.rs`)
- [ ] Update `Trip::test_ice_trip()` helper to use new fields

### Task 4.2: Update commands_tests.rs (`src-tauri/src/commands_tests.rs`)
- [ ] Update `make_trip()` helper
- [ ] Update `make_trip_with_fuel()` helper
- [ ] Update `make_trip_with_date()` helper
- [ ] Update `make_trip_with_date_odo()` helper
- [ ] Update `make_trip_detailed()` helper
- [ ] Update `make_trip_for_magic_fill()` helper
- [ ] Update all inline Trip struct initializations

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
