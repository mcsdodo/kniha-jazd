# Implementation Plan: Receipt Datetime Validation

**Date:** 2026-02-01
**Status:** Planning
**Design:** [01-design.md](./01-design.md)

## Overview

Upgrade receipts from date-only to datetime, validate against trip range, show warnings.

## Implementation Steps

### Phase 1: Database & Schema

#### Task 1.1: Create Migration
**Files:** `src-tauri/migrations/2026-02-01-100000_replace_receipt_date_with_datetime/`

**Note:** `DROP COLUMN` requires SQLite 3.35.0+ (March 2021). Tauri bundles SQLite, verify version is sufficient.

```sql
-- up.sql
ALTER TABLE receipts ADD COLUMN receipt_datetime TEXT DEFAULT NULL;
UPDATE receipts SET receipt_datetime = receipt_date || 'T00:00:00' WHERE receipt_date IS NOT NULL;
ALTER TABLE receipts DROP COLUMN receipt_date;

-- down.sql
ALTER TABLE receipts ADD COLUMN receipt_date TEXT DEFAULT NULL;
UPDATE receipts SET receipt_date = substr(receipt_datetime, 1, 10) WHERE receipt_datetime IS NOT NULL;
ALTER TABLE receipts DROP COLUMN receipt_datetime;
```

#### Task 1.2: Update schema.rs
**File:** `src-tauri/src/schema.rs`

- Replace `receipt_date -> Nullable<Text>` with `receipt_datetime -> Nullable<Text>`

#### Task 1.3: Update models.rs
**File:** `src-tauri/src/models.rs`

- `Receipt`: change `receipt_date: Option<NaiveDate>` → `receipt_datetime: Option<NaiveDateTime>`
- `ReceiptRow`: change `receipt_date: Option<String>` → `receipt_datetime: Option<String>`
- `NewReceiptRow`: change `receipt_date: Option<&'a str>` → `receipt_datetime: Option<&'a str>`
- Update `From<ReceiptRow> for Receipt` conversion (parse as NaiveDateTime)
- `TripGridData`: add `receipt_datetime_warnings: HashSet<String>`

**Test:** Verify migration runs, existing receipts have datetime with T00:00:00

---

### Phase 2: Backend Logic (TDD)

#### Task 2.1: Write validation tests FIRST
**File:** `src-tauri/src/commands/commands_tests.rs`

```rust
#[test]
fn test_receipt_datetime_warning_within_range() {
    // Receipt datetime inside trip [start, end] → no warning
}

#[test]
fn test_receipt_datetime_warning_before_trip_start() {
    // Receipt datetime before trip.start_datetime → warning
}

#[test]
fn test_receipt_datetime_warning_after_trip_end() {
    // Receipt datetime after trip.end_datetime → warning
}

#[test]
fn test_receipt_datetime_warning_no_receipt() {
    // Trip without receipt → no warning (separate concern)
}

#[test]
fn test_receipt_datetime_warning_receipt_no_datetime() {
    // Receipt with None datetime → no warning (can't validate)
}

#[test]
fn test_receipt_datetime_warning_exactly_at_start() {
    // Receipt datetime == trip.start_datetime → no warning (boundary: inclusive)
}

#[test]
fn test_receipt_datetime_warning_exactly_at_end() {
    // Receipt datetime == trip.end_datetime → no warning (boundary: inclusive)
}
```

#### Task 2.2: Implement validation in get_trip_grid_data
**File:** `src-tauri/src/commands/mod.rs`

- Add `receipt_datetime_warnings` calculation
- Populate `TripGridData.receipt_datetime_warnings`

**Verify:** All Task 2.1 tests pass

#### Task 2.3: Update receipt matching logic
**Files:**
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/commands/receipts_cmd.rs`
- `src-tauri/src/commands/statistics.rs`

Update all occurrences of date-only matching:
- `find_missing_receipts_internal` (mod.rs)
- `assign_receipt_to_trip_internal` (receipts_cmd.rs:400)
- `check_receipt_trip_compatibility` (receipts_cmd.rs:499, :514)
- `get_trips_for_receipt_assignment` (receipts_cmd.rs:584)
- `verify_receipts_internal` (receipts_cmd.rs:628, :669, :678-688, :726-745)
- `find_missing_receipts_internal` (statistics.rs:1269)

Change from:
```rust
receipt.receipt_date == Some(trip.start_datetime.date())
```
To:
```rust
receipt.receipt_datetime
    .map(|dt| dt >= trip.start_datetime && dt <= trip.end_datetime)
    .unwrap_or(false)
```

**Tests:** Update existing receipt matching tests to use datetime

---

### Phase 3: OCR Changes

#### Task 3.1: Update Gemini prompt
**File:** `src-tauri/src/gemini.rs`

- Change prompt: `receipt_date` → `receipt_datetime`
- Update format description: `YYYY-MM-DDTHH:MM:SS` or `YYYY-MM-DD`
- Update response schema

#### Task 3.2: Write OCR parsing tests FIRST
**File:** `src-tauri/src/gemini_tests.rs`

```rust
#[test]
fn test_parse_receipt_with_full_datetime() {
    // "2026-01-15T14:32:00" → NaiveDateTime
}

#[test]
fn test_parse_receipt_date_only_triggers_needs_review() {
    // "2026-01-15" → NaiveDateTime at midnight + NeedsReview status
}
```

#### Task 3.3: Update receipt processing
**File:** `src-tauri/src/receipts.rs`

- Parse `receipt_datetime` from extracted data
- Set `NeedsReview` if only date (no time) extracted

**Verify:** Task 3.2 tests pass

---

### Phase 4: Database Operations

#### Task 4.1: Update db.rs queries
**File:** `src-tauri/src/db.rs`

- `insert_receipt`: use `receipt_datetime`
- `update_receipt`: use `receipt_datetime`
- `get_receipts_for_year`: filter by year from `receipt_datetime`
- All raw SQL queries referencing `receipt_date`

#### Task 4.2: Update db_tests.rs
**File:** `src-tauri/src/db_tests.rs`

- Update test helpers to use `receipt_datetime`
- Update `test_get_receipts_for_year_filters_by_receipt_date` → `..._by_receipt_datetime`

---

### Phase 5: Frontend

#### Task 5.1: Update TypeScript types
**File:** `src/lib/types.ts` (or wherever Trip/Receipt types are)

- `Receipt.receiptDatetime: string | null` (replaces `receiptDate`)
- `TripGridData.receiptDatetimeWarnings: string[]`

#### Task 5.2: Update TripGrid.svelte
**File:** `src/lib/components/TripGrid.svelte`

- Compute `receiptDatetimeWarningCount`
- Pass `hasReceiptDatetimeWarning` prop to TripRow
- Add legend item for datetime warnings

#### Task 5.3: Update TripRow.svelte
**File:** `src/lib/components/TripRow.svelte`

- Add `hasReceiptDatetimeWarning` prop
- Display red asterisk with combined tooltip
- Add CSS for `.datetime-warning-indicator`

#### Task 5.4: Update Receipt editing
**File:** `src/lib/components/ReceiptEditModal.svelte`

- Change date input → datetime-local input
- Show "time not extracted" warning when applicable

#### Task 5.5: Add i18n strings
**Files:** `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

- `receiptDatetimeMismatch`
- `legend.receiptDatetimeMismatch`
- `timeNotExtracted`

---

### Phase 6: Integration Tests

#### Task 6.1: Add E2E test for warning display
**File:** `tests/integration/specs/trip-grid.spec.ts` (or new file)

```typescript
it('shows red asterisk when receipt datetime outside trip range', async () => {
    // Seed trip with datetime range
    // Seed receipt with datetime outside range
    // Assign receipt to trip
    // Verify red asterisk appears
    // Verify tooltip contains warning text
});
```

---

### Phase 7: Finalization

#### Task 7.1: Run full test suite
```bash
npm run test:all
```

#### Task 7.2: Manual testing
- Create trip with specific datetime range
- OCR a receipt (verify time extraction)
- Assign receipt outside range → verify red asterisk
- Edit receipt datetime → verify warning clears

#### Task 7.3: Update CHANGELOG
```markdown
### Added
- Receipt datetime validation: warns when receipt datetime falls outside trip range
- Red asterisk indicator on trip grid for datetime mismatches
- Time extraction from receipts during OCR

### Changed
- Receipts now store full datetime instead of date-only
```

---

## Commit Strategy

1. **Phase 1 complete:** `feat(db): add receipt_datetime column, replace receipt_date`
2. **Phase 2 complete:** `feat(backend): validate receipt datetime against trip range`
3. **Phase 3 complete:** `feat(ocr): extract datetime from receipts`
4. **Phase 4 complete:** `refactor(db): update queries for receipt_datetime`
5. **Phase 5 complete:** `feat(ui): show receipt datetime warning on trip grid`
6. **Phase 6 complete:** `test(e2e): add receipt datetime warning tests`
7. **Final:** `docs: update changelog for receipt datetime validation`

## Estimated Scope

- **Backend changes:** ~8 files
- **Frontend changes:** ~5 files
- **New tests:** ~10-15 test cases
- **Migration:** 1 migration (2 files)
