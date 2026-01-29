# Implementation Plan: end_time to end_datetime Cleanup

**Date:** 2026-01-29
**Status:** Planning

---

## Phase 1: Update Trip Model

### Task 1.1: Update Trip struct (models.rs)
- [ ] Rename `datetime` to `start_datetime`
- [ ] Replace `end_time: Option<String>` with `end_datetime: NaiveDateTime`
- [ ] Remove `date: NaiveDate` field (can derive from start_datetime)
- [ ] Update `#[serde(rename_all = "camelCase")]` attributes if needed

### Task 1.2: Update TripRow struct (models.rs)
- [ ] Rename `datetime` to `start_datetime`
- [ ] Replace `end_time: String` with `end_datetime: String`
- [ ] Remove `date: String` field
- [ ] Ensure field order matches schema.rs column order

### Task 1.3: Update NewTripRow struct (models.rs)
- [ ] Same changes as TripRow

### Task 1.4: Update From<TripRow> for Trip (models.rs)
- [ ] Parse `start_datetime` and `end_datetime` as NaiveDateTime
- [ ] Remove date parsing

---

## Phase 2: Update Database Layer

### Task 2.1: Update db.rs create_trip
- [ ] Remove `date` field from NewTripRow construction
- [ ] Use `start_datetime` and `end_datetime` directly
- [ ] Remove `end_time` field

### Task 2.2: Update db.rs update_trip
- [ ] Update field names in diesel update query
- [ ] Remove `end_time` handling

### Task 2.3: Consider schema changes
- [ ] Option A: Keep DB columns as-is, map in code (safer)
- [ ] Option B: Create migration to rename columns (cleaner but riskier)
- [ ] Decision: Start with Option A, migrate later if needed

---

## Phase 3: Update Commands

### Task 3.1: Update commands/trips.rs
- [ ] Remove `extract_time_string` usage
- [ ] Store `end_datetime` directly in Trip struct
- [ ] Update Trip struct construction in create_trip
- [ ] Update Trip struct construction in update_trip

---

## Phase 4: Update Tests

### Task 4.1: Update test helpers (models.rs)
- [ ] Update `Trip::test_ice_trip()` helper
- [ ] Update any other test constructors

### Task 4.2: Update commands_tests.rs
- [ ] Update `make_trip()` helper
- [ ] Update `make_trip_with_fuel()` helper
- [ ] Update `make_trip_with_date()` helper
- [ ] Update `make_trip_with_date_odo()` helper
- [ ] Update `make_trip_detailed()` helper
- [ ] Update `make_trip_for_magic_fill()` helper
- [ ] Update all inline Trip struct initializations (~30+)

### Task 4.3: Update calculations_tests.rs
- [ ] Update Trip struct initializations

### Task 4.4: Update db_tests.rs
- [ ] Update Trip struct initializations

### Task 4.5: Update export.rs
- [ ] Update mock Trip structs in tests

---

## Phase 5: Verification

### Task 5.1: Run all tests
- [ ] `cargo test` - all backend tests pass
- [ ] `npm run check` - frontend compiles (should be unchanged)

### Task 5.2: Manual testing
- [ ] Create trip - verify datetime storage
- [ ] Edit trip - verify datetime updates
- [ ] Export - verify times display correctly

---

## Phase 6: Optional DB Migration

### Task 6.1: Create column rename migration
Only if needed for cleanliness:
```sql
-- Rename columns to match new field names
ALTER TABLE trips RENAME COLUMN datetime TO start_datetime;
-- end_datetime column already exists from Task 47 migration
-- Drop legacy columns
ALTER TABLE trips DROP COLUMN date;
ALTER TABLE trips DROP COLUMN end_time;
```

### Task 6.2: Update schema.rs
- [ ] Remove `date` column
- [ ] Rename `datetime` to `start_datetime`
- [ ] Remove `end_time` column

---

## Rollback Plan

If issues arise:
1. Revert to previous commit
2. DB schema changes are irreversible - only apply after thorough testing
3. Consider keeping Option A (map in code) for initial release

---

## Notes

- Per ADR-012, we don't need backward compatibility
- Frontend already uses startDatetime/endDatetime API (Task 47)
- This is purely a backend cleanup task
