# Implementation Plan: Datetime Field Consolidation

## Overview

Add `start_datetime` and `end_datetime` columns to trips table, migrate existing data, update all Rust code to use new fields.

**Estimated effort:** ~2-3 hours
**Files to modify:** ~10 files, ~50 locations

---

## Phase 1: Database Migration

### Task 1.1: Create Migration
- [ ] Run `diesel migration generate add_start_end_datetime`
- [ ] Write up.sql:
  ```sql
  ALTER TABLE trips ADD COLUMN start_datetime TEXT NOT NULL DEFAULT '';
  ALTER TABLE trips ADD COLUMN end_datetime TEXT DEFAULT NULL;
  UPDATE trips SET start_datetime = datetime WHERE start_datetime = '';
  UPDATE trips SET end_datetime = date || 'T' || end_time || ':00'
    WHERE end_time IS NOT NULL AND end_time != '';
  ```
- [ ] Write down.sql (empty per backward-compat policy)
- [ ] Run migration on dev database

### Task 1.2: Update schema.rs
- [ ] Add `start_datetime -> Text` to trips table
- [ ] Add `end_datetime -> Nullable<Text>` to trips table
- [ ] Keep Double types (don't let diesel regenerate with Float)

---

## Phase 2: Update Rust Models

### Task 2.1: Update Trip struct (models.rs)
- [ ] Add `start_datetime: NaiveDateTime` field
- [ ] Add `end_datetime: Option<NaiveDateTime>` field
- [ ] Keep legacy fields (date, datetime, end_time) for compatibility

### Task 2.2: Update TripRow struct (models.rs)
- [ ] Add `start_datetime: String` field at end (matches schema order)
- [ ] Add `end_datetime: Option<String>` field at end

### Task 2.3: Update NewTripRow struct (models.rs)
- [ ] Add `start_datetime: &'a str` field
- [ ] Add `end_datetime: Option<&'a str>` field

### Task 2.4: Update From<TripRow> for Trip (models.rs)
- [ ] Parse `start_datetime` as primary, fallback to `datetime`
- [ ] Parse `end_datetime` if present
- [ ] Set `datetime = start_datetime` for backward compat

---

## Phase 3: Update Commands

### Task 3.1: Update create_trip (commands/trips.rs)
- [ ] Accept start_datetime and end_datetime parameters
- [ ] Populate both new and legacy fields in NewTripRow
- [ ] Keep backward compat: also set date, datetime, end_time

### Task 3.2: Update update_trip (commands/trips.rs)
- [ ] Accept start_datetime and end_datetime parameters
- [ ] Update both new and legacy fields

### Task 3.3: Update export commands (commands/mod.rs)
- [ ] ~3 places create mock Trip structs for export
- [ ] Add start_datetime and end_datetime fields

---

## Phase 4: Update Tests

### Task 4.1: Update test helpers (commands/commands_tests.rs)
- [ ] `make_trip()` - add new fields
- [ ] `make_trip_with_fuel()` - add new fields
- [ ] `make_trip_with_date()` - add new fields
- [ ] `make_trip_with_date_odo()` - add new fields
- [ ] ~15 other Trip struct initializations

### Task 4.2: Update other test files
- [ ] calculations_tests.rs - Trip struct initializations
- [ ] db_tests.rs - Trip struct initializations
- [ ] export.rs - mock Trip for tests

---

## Phase 5: Frontend Updates (if needed)

### Task 5.1: Review frontend types
- [ ] Check if `Trip` interface in types.ts needs updates
- [ ] Likely no changes needed - backend handles mapping

---

## Phase 6: Verification

### Task 6.1: Run all tests
- [ ] `cargo test` - all 237 backend tests pass
- [ ] `npm run check` - frontend compiles

### Task 6.2: Manual testing
- [ ] Create trip - verify both old and new fields populated
- [ ] Edit trip - verify fields stay in sync
- [ ] Export - verify times display correctly

---

## Rollback Plan

If issues arise:
1. New columns can be ignored (old code doesn't read them)
2. Old columns still contain valid data
3. No data loss possible due to additive-only migration
