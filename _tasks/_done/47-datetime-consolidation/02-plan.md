# Implementation Plan: Datetime Field Consolidation

## Overview

Add `start_datetime` and `end_datetime` columns to trips table, migrate existing data, update all Rust code to use new fields.

**Status:** Complete (Phase 5 cleanup done in Task 48)

---

## Phase 1: Database Migration ✅ DONE

### Task 1.1: Create Migration ✅
- [x] Migration `2026-01-29-193744-0000_add_start_end_datetime` created
- [x] up.sql adds `start_datetime` (NOT NULL, default '') and `end_datetime` (nullable)
- [x] Existing data migrated: `start_datetime = datetime`, `end_datetime = date || 'T' || end_time || ':00'`
- [x] For trips without end_time: `end_datetime = date || 'T00:00:00'` (mandatory field)

### Task 1.2: Update schema.rs ✅
- [x] Added `start_datetime -> Text` to trips table
- [x] Added `end_datetime -> Nullable<Text>` to trips table

---

## Phase 2: Update Rust Models ✅ DONE

### Task 2.1: Update TripRow struct ✅
- [x] Added `start_datetime: String` field (models.rs:683)
- [x] Added `end_datetime: Option<String>` field (models.rs:684)

### Task 2.2: Update NewTripRow struct ✅
- [x] Added `start_datetime: &'a str` field (models.rs:714)
- [x] Added `end_datetime: Option<&'a str>` field (models.rs:715)

### Task 2.3: Update create_trip ✅
- [x] `create_trip` (db.rs:260-304) populates both old and new fields
- [x] `start_datetime = datetime_str` (same as legacy datetime)
- [x] `end_datetime = date + end_time` or `date + "T00:00:00"` if no end_time

### Note: Trip Domain Struct
The domain `Trip` struct keeps using `datetime` and `end_time` field names internally. This is intentional:
- Backend handles mapping to new DB columns
- Frontend API unchanged
- No breaking changes to serialization

---

## Phase 3: Fix update_trip Sync ✅ DONE

### Task 3.1: Add start_datetime/end_datetime to update_trip ✅
**File:** `src-tauri/src/db.rs` (lines 372-410)

- [x] Added `trips::start_datetime.eq(&datetime_str)` to update query
- [x] Added `trips::end_datetime.eq(end_datetime_str.as_deref())` with same logic as create_trip

**Verification:** All 237 backend tests pass

---

## Phase 4: Verification

### Task 4.1: Run all tests
- [ ] `cargo test` - all backend tests pass
- [ ] `npm run check` - frontend compiles

### Task 4.2: Manual testing
- [ ] Create trip - verify both old and new fields populated
- [ ] Edit trip - verify fields stay in sync
- [ ] Export - verify times display correctly

---

## Phase 5: Cleanup Migration (Future - after v1.0 release)

> **Note:** This phase was completed in Task 48. Legacy columns were dropped via
> migration `2026-01-30-100000_drop_legacy_datetime_columns` once forward-only
> migrations were approved.
> - All users have migrated to the new version
> - A major version bump (e.g., v1.0 → v2.0)
> - Sufficient time has passed (3+ months)

### Task 5.1: Mark legacy columns as deprecated
- [ ] Add `// DEPRECATED` comments to schema.rs for `date`, `datetime`, `end_time` (N/A after cleanup)
- [ ] Add deprecation notice to CHANGELOG (N/A after cleanup)

### Task 5.2: Create cleanup migration (v2.0+)
- [x] Create migration to drop redundant columns:
  ```sql
  -- Only run after all users on v1.x+
  ALTER TABLE trips DROP COLUMN date;
  ALTER TABLE trips DROP COLUMN datetime;
  ALTER TABLE trips DROP COLUMN end_time;
  ```
- [x] Update schema.rs - remove deprecated columns
- [x] Update TripRow - remove deprecated fields
- [x] Update NewTripRow - remove deprecated fields
- [x] Update Trip struct - remove legacy fields
- [x] Update all tests

### Task 5.3: Verification
- [x] All tests pass
- [x] Manual testing with fresh DB
- [x] Document breaking change in CHANGELOG

---

## Rollback Plan

If issues arise:
1. New columns can be ignored (old code doesn't read them)
2. Old columns still contain valid data
3. No data loss possible due to additive-only migration
