# Plan Review: Multi-Currency Receipt Support

**Date:** 2026-01-21
**Plan:** `02-plan.md`
**Reviewer:** Claude (automated review)
**Status:** Needs Revisions

---

## Summary

The plan is **mostly complete and well-structured**, with clear phases and good understanding of the codebase. However, there are **2 Critical** and **3 Important** findings that should be addressed before implementation.

---

## Findings

### Critical

- [ ] **C1: Missing `original_amount` and `original_currency` in db.rs CRUD functions**
  - **Location:** Phase 1.3 (models.rs) + Phase 3.2 (db.rs)
  - **Issue:** The plan mentions updating `create_receipt` and `update_receipt` in db.rs, but doesn't specify the exact changes needed. Looking at `db.rs:642-681` and `db.rs:716-747`, these functions already use all fields from the `NewReceiptRow` struct. The plan needs to add these fields to:
    1. `NewReceiptRow` struct in `models.rs` (lines 710-733)
    2. `ReceiptRow` struct in `models.rs` (lines 683-707)
    3. The `From<ReceiptRow> for Receipt` conversion (lines 832-872)
  - **Fix:** Add explicit steps for:
    - Add `original_amount: Option<f64>` and `original_currency: Option<String>` to `Receipt` struct
    - Add `original_amount: Option<f64>` and `original_currency: Option<&'a str>` to `NewReceiptRow`
    - Add `original_amount -> Nullable<Double>` and `original_currency -> Nullable<Text>` to `ReceiptRow`
    - Update `From<ReceiptRow> for Receipt` to include new fields
    - Update `create_receipt` to include new fields in insert
    - Update `update_receipt` to include new fields in update

- [ ] **C2: Missing frontend TypeScript type updates**
  - **Location:** Phase 4/5 (Frontend)
  - **Issue:** The plan mentions UI changes but doesn't explicitly list updating `src/lib/types.ts`. The `Receipt` interface (line 129-150) needs the new currency fields.
  - **Fix:** Add step to Phase 4.1:
    - Update `Receipt` interface in `src/lib/types.ts` to add `originalAmount: number | null` and `originalCurrency: string | null`
    - Update `FieldConfidence` interface to add `currency: ConfidenceLevel` (if confidence is needed for currency)

### Important

- [ ] **I1: Missing receipt edit command**
  - **Location:** Phase 3.3
  - **Issue:** The plan says "Add/update receipt edit command" but there is no existing `update_receipt` Tauri command in the codebase. The current `update_receipt` in `db.rs` is a database function, not a Tauri command. Looking at `commands.rs`, there's no command to edit receipt fields from the frontend.
  - **Fix:** Clarify Phase 3.3:
    - Create new Tauri command `update_receipt_currency` in `commands.rs` that accepts `receipt_id`, `original_amount`, `original_currency`, and `total_price_eur`
    - Register in `lib.rs` invoke_handler
    - Add `check_read_only!` guard (it's a write command)

- [ ] **I2: Frontend receipt edit modal doesn't exist**
  - **Location:** Phase 5.1-5.3
  - **Issue:** The plan mentions updating the "receipt edit form" and "receipt edit modal", but looking at `src/routes/doklady/+page.svelte`, there is no existing receipt edit modal. The current UI only has: open file, reprocess, assign to trip, and delete actions.
  - **Fix:** Phase 5 should be rewritten to:
    - Create new `ReceiptEditModal.svelte` component (NEW file)
    - Add edit button to receipt card actions
    - Wire up modal open/close state
    - This is more work than the plan suggests

- [ ] **I3: Missing API function in frontend**
  - **Location:** Phase 5.3
  - **Issue:** The plan says "Call updated backend command" but doesn't mention adding the API function in `src/lib/api.ts` to call the new Tauri command.
  - **Fix:** Add to Phase 4 or 5:
    - Add `updateReceiptCurrency(receiptId: string, originalAmount: number | null, originalCurrency: string | null, totalPriceEur: number | null)` function to `src/lib/api.ts`

### Minor

- [ ] **M1: Migration file naming convention**
  - **Location:** Phase 1.1
  - **Issue:** The migration filename `2026-01-21-100000-add_receipt_currency` uses a timestamp prefix. Looking at existing migrations, they follow the pattern `YYYY-MM-DD-HHMMSS-description`. Ensure the actual timestamp is used when creating the migration with `diesel migration generate`.
  - **Fix:** Use `diesel migration generate add_receipt_currency` to generate with correct timestamp.

- [ ] **M2: Tests don't specify file locations**
  - **Location:** Phase 1.4, 2.4, 3.4
  - **Issue:** Tests are mentioned but the plan doesn't specify which test files they should go in.
  - **Fix:** Clarify:
    - Phase 1.4 tests: Add to `db_tests.rs` (receipt lifecycle tests)
    - Phase 2.4 tests: Add to `gemini.rs` `mod tests` section (deserialization tests)
    - Phase 3.4 tests: Add to `commands.rs` tests (integration with mocked Gemini)

---

## Checklist Assessment

| Criteria | Status | Notes |
|----------|--------|-------|
| Tasks have specific file paths | Partial | Missing exact locations for some changes |
| Verification steps present | Yes | Phase 6 covers testing |
| Correct task order | Yes | Dependencies are correct |
| No scope creep | Yes | Focused on currency support only |
| TDD approach followed | Partial | Tests mentioned but not as first step |
| Backward-compatible migration | Yes | NULL defaults correctly specified |

---

## Recommendation

**Address Critical and Important findings before implementation.**

The plan is solid conceptually but needs more specificity for implementation, especially around:
1. The full list of Rust files/structs that need the new fields
2. Creating a NEW receipt edit modal (not updating an existing one)
3. Creating a NEW Tauri command for receipt editing

With these fixes, the plan will be implementation-ready.

---

## Resolution

*(To be filled after Phase 2 - applying fixes)*
