# Code Review: Receipt Datetime Validation

**Target:** Task 50 - Receipt datetime validation implementation
**Reference:** `_tasks/50-receipt-datetime-validation/02-plan.md`
**Started:** 2026-02-01
**Status:** Ready for User Review
**Focus:** Quality, correctness, best practices

**Baseline Test Status:** 248 tests passing ✅

## Files Modified

### Backend (Rust)
- `src-tauri/migrations/2026-02-01-100000_replace_receipt_date_with_datetime/up.sql` (new)
- `src-tauri/migrations/2026-02-01-100000_replace_receipt_date_with_datetime/down.sql` (new)
- `src-tauri/src/schema.rs` - receipt_date → receipt_datetime
- `src-tauri/src/models.rs` - Receipt struct, TripGridData
- `src-tauri/src/gemini.rs` - OCR prompt update
- `src-tauri/src/gemini_tests.rs` - OCR tests
- `src-tauri/src/receipts.rs` - datetime parsing
- `src-tauri/src/receipts_tests.rs` - parsing tests
- `src-tauri/src/db.rs` - CRUD operations
- `src-tauri/src/db_tests.rs` - DB tests
- `src-tauri/src/commands/receipts_cmd.rs` - matching logic
- `src-tauri/src/commands/statistics.rs` - warnings calculation
- `src-tauri/src/commands/commands_tests.rs` - validation tests

### Frontend (Svelte)
- `src/lib/types.ts` - TypeScript types
- `src/lib/components/TripGrid.svelte` - legend, prop passing
- `src/lib/components/TripRow.svelte` - warning indicator
- `src/lib/components/ReceiptEditModal.svelte` - datetime input
- `src/lib/components/TripSelectorModal.svelte` - type updates
- `src/routes/doklady/+page.svelte` - receipt display
- `src/lib/i18n/sk/index.ts` - Slovak strings
- `src/lib/i18n/en/index.ts` - English strings
- `src/lib/i18n/i18n-types.ts` - generated types

### Tests
- `tests/integration/data/mocks/invoice.json` - mock update
- `tests/integration/data/mocks/invoice-czk.json` - mock update
- `tests/integration/specs/tier2/receipts.spec.ts` - test fixes

---

## Review Summary

**Iterations:** 1
**Total Findings:** 0 Critical, 2 Important, 3 Minor
**Test Status:** All 248 backend tests passing ✅

---

## What Was Done Well

1. **Clean migration pattern** - Correctly handles transition with backfill using midnight (`T00:00:00`) for existing records

2. **Comprehensive test coverage** - 8 unit tests cover all edge cases for `calculate_receipt_datetime_warnings()`:
   - Within range, before start, after end
   - No receipt, receipt without datetime
   - Boundary conditions (inclusive)
   - Fallback when no end_datetime

3. **Backend architecture adherence** - Follows ADR-008: all calculation logic in Rust, frontend displays pre-calculated values

4. **Consistent datetime handling** - Gemini prompt correctly requests both formats, parsing handles both gracefully

5. **NeedsReview for date-only** - Receipts with OCR-extracted date (no time) automatically marked for review

6. **i18n compliance** - Slovak and English translations present for all new strings

---

## All Findings (Consolidated)

### Critical
None identified ✅

### Important
1. [ ] **Helper function duplication** - `receipts_cmd.rs:401, :521` + `statistics.rs:1275, :1296`
   - Datetime range validation logic repeated in 4 places
   - Suggested fix: Extract `is_datetime_in_trip_range(receipt_datetime, trip) -> bool` helper

2. [ ] **Missing integration test for UI** - `tests/integration/`
   - Plan Phase 6 specified E2E test for red asterisk display
   - Suggested fix: Add test that seeds trip + receipt with mismatched datetime, verifies warning appears

### Minor
1. [ ] **No visual indicator for "time not extracted"** - `ReceiptEditModal.svelte`
   - Plan mentioned showing warning when time is `00:00:00`
   - Suggested fix: Add hint text when time component appears to be midnight

2. [ ] **Test helper uses noon for backward compat** - `commands_tests.rs:85`
   - `make_test_receipt_with_date()` converts to noon, could be clearer
   - Suggested fix: Consider `make_test_receipt_with_datetime()` for new tests

3. [ ] **Missing feature documentation** - `docs/features/`
   - No feature doc for receipt datetime validation
   - Suggested fix: Create `docs/features/receipt-datetime-validation.md` post-merge

---

## Test Gaps

- [ ] Integration test for datetime warning UI display (Phase 6, Task 6.1 from plan)

---

## Recommendation

**Ready to merge** - The implementation is well-structured, follows the plan, and has comprehensive backend test coverage. The missing integration test can be added as a follow-up item.

---

## Next Steps

After user review, let me know:
- Which findings to fix now
- Which to skip (with reason)
- Any questions about the findings
