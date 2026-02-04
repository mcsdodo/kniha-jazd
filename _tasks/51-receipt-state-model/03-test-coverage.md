# Test Coverage: Receipt-Trip State Model

**Date:** 2026-02-04
**Status:** Analysis Complete
**Related:** [Design Doc](../_TECH_DEBT/05-receipt-trip-state-model-design.md)

---

## Summary

| Category | Covered | Partial | Missing |
|----------|---------|---------|---------|
| A. Invoice Scenarios | 5 | 2 | 2 |
| B. Trip Scenarios | 6 | 2 | 3 |
| C. Assignment Scenarios | 7 | 1 | 0 |
| D. Mismatch Detection | 5 | 0 | 0 |

---

## A. Invoice Scenarios

| # | Scenario | Test | Status |
|---|----------|------|--------|
| A1 | Invoice scanned, OCR pending | - | ❌ Missing (needs async mock) |
| A2 | Invoice scanned, OCR failed/low confidence | [receipts_tests.rs:305](../src-tauri/src/receipts_tests.rs#L305) `test_apply_extraction_low_confidence` | ⚠️ Partial |
| A3 | Invoice ready, not assigned | [commands_tests.rs:1709](../src-tauri/src/commands/commands_tests.rs#L1709) (setup) | ✅ Indirect |
| A4 | Invoice assigned as FUEL, data matches | [commands_tests.rs:1804](../src-tauri/src/commands/commands_tests.rs#L1804) `test_assign_fuel_with_matching_data_links_only` | ✅ Full |
| A5 | Invoice assigned as FUEL, data mismatch | [commands_tests.rs:1847](../src-tauri/src/commands/commands_tests.rs#L1847) `test_assign_fuel_with_mismatch_no_override` | ✅ Full |
| A6 | Invoice assigned as FUEL, mismatch + override | [commands_tests.rs:1885](../src-tauri/src/commands/commands_tests.rs#L1885) `test_assign_fuel_with_mismatch_and_override` | ✅ Full |
| A7 | Invoice assigned as OTHER COST | [commands_tests.rs:1755](../src-tauri/src/commands/commands_tests.rs#L1755) `test_assign_other_to_empty_trip_populates_data` | ✅ Full |
| A8 | Invoice assigned as OTHER, data mismatch | [commands_tests.rs:1923](../src-tauri/src/commands/commands_tests.rs#L1923) `test_assign_other_to_trip_with_existing_other_costs_allowed` | ⚠️ Partial |
| A9 | Invoice assigned as OTHER, mismatch + override | - | ❌ Missing |

---

## B. Trip Scenarios (Trip Grid Indicators)

| # | Scenario | Test | Status |
|---|----------|------|--------|
| B1 | Trip with fuel, no invoice | [commands_tests.rs:140](../src-tauri/src/commands/commands_tests.rs#L140) `test_missing_receipts_trip_without_assigned_receipt_flagged` | ✅ Full |
| B2 | Trip with fuel, invoice assigned, matches | [commands_tests.rs:124](../src-tauri/src/commands/commands_tests.rs#L124) `test_missing_receipts_trip_with_assigned_receipt_not_flagged` | ✅ Full |
| B3 | Trip with fuel, invoice assigned, mismatch | [commands_tests.rs:354](../src-tauri/src/commands/commands_tests.rs#L354) `test_receipt_datetime_warning_before_trip_start` | ✅ Full |
| B4 | Trip with fuel, invoice assigned, override | - | ❌ Missing (no indicator test) |
| B5 | Trip with other costs, no invoice | [commands_tests.rs:169](../src-tauri/src/commands/commands_tests.rs#L169) `test_missing_receipts_trip_with_other_costs_no_receipt_flagged` | ✅ Full |
| B6 | Trip with other costs, invoice assigned, matches | [commands_tests.rs:154](../src-tauri/src/commands/commands_tests.rs#L154) `test_missing_receipts_trip_without_costs_not_flagged` | ⚠️ Indirect |
| B6a | Trip with other costs, invoice assigned, mismatch | - | ❌ Missing |
| B6b | Trip with other costs, invoice assigned, override | - | ❌ Missing |
| B7 | Trip with fuel AND other costs, missing one | [commands_tests.rs:200](../src-tauri/src/commands/commands_tests.rs#L200) `test_missing_receipts_multiple_trips_partial_assignment` | ⚠️ Partial |
| B8 | Trip with fuel AND other costs, both assigned | - | ❌ Missing |
| B9 | Trip with NO costs | [commands_tests.rs:154](../src-tauri/src/commands/commands_tests.rs#L154) `test_missing_receipts_trip_without_costs_not_flagged` | ✅ Full |

---

## C. Assignment Scenarios

| # | Scenario | Test | Status |
|---|----------|------|--------|
| C1 | Assign invoice to trip with NO costs, as FUEL | [commands_tests.rs:1709](../src-tauri/src/commands/commands_tests.rs#L1709) `test_assign_fuel_to_empty_trip_populates_data` | ✅ Full |
| C2 | Assign invoice to trip with NO costs, as OTHER | [commands_tests.rs:1755](../src-tauri/src/commands/commands_tests.rs#L1755) `test_assign_other_to_empty_trip_populates_data` | ✅ Full |
| C3 | Assign invoice to trip with matching fuel data | [commands_tests.rs:1804](../src-tauri/src/commands/commands_tests.rs#L1804) `test_assign_fuel_with_matching_data_links_only` | ✅ Full |
| C4 | Assign invoice with mismatch, no override | [commands_tests.rs:1847](../src-tauri/src/commands/commands_tests.rs#L1847) `test_assign_fuel_with_mismatch_no_override` | ✅ Full |
| C5 | Assign invoice with mismatch + override | [commands_tests.rs:1885](../src-tauri/src/commands/commands_tests.rs#L1885) `test_assign_fuel_with_mismatch_and_override` | ✅ Full |
| C6 | Assign OTHER to trip with existing other costs | [commands_tests.rs:1923](../src-tauri/src/commands/commands_tests.rs#L1923) `test_assign_other_to_trip_with_existing_other_costs_allowed` | ✅ Full |
| C6a | Assign OTHER with mismatch | - | ⚠️ Partial (backend logic exists) |
| C7 | Reassign invoice to different trip | [commands_tests.rs:1972](../src-tauri/src/commands/commands_tests.rs#L1972) `test_reassign_invoice_to_different_trip` | ✅ Full |

---

## D. Mismatch Detection (get_trips_for_receipt_assignment)

| # | Scenario | Test | Status |
|---|----------|------|--------|
| D1 | Empty trip → can attach | [commands_tests.rs:2061](../src-tauri/src/commands/commands_tests.rs#L2061) `test_get_trips_for_receipt_assignment_empty_trip_returns_can_attach_true` | ✅ Full |
| D2 | Matching data → can attach | [commands_tests.rs:2100](../src-tauri/src/commands/commands_tests.rs#L2100) `test_get_trips_for_receipt_assignment_matching_fuel_returns_can_attach_true` | ✅ Full |
| D3 | Different liters → mismatch | [commands_tests.rs:2140](../src-tauri/src/commands/commands_tests.rs#L2140) `test_get_trips_for_receipt_assignment_different_liters_shows_mismatch` | ✅ Full |
| D4 | Different price → mismatch | [commands_tests.rs:2190](../src-tauri/src/commands/commands_tests.rs#L2190) `test_get_trips_for_receipt_assignment_different_price_shows_mismatch` | ✅ Full |
| D5 | Different date → mismatch | [commands_tests.rs:2240](../src-tauri/src/commands/commands_tests.rs#L2240) `test_get_trips_for_receipt_assignment_different_date_shows_mismatch` | ✅ Full |

---

## E. Receipt Datetime Validation

| # | Scenario | Test | Status |
|---|----------|------|--------|
| E1 | Receipt within trip range | [commands_tests.rs:327](../src-tauri/src/commands/commands_tests.rs#L327) `test_receipt_datetime_warning_within_range` | ✅ Full |
| E2 | Receipt before trip start | [commands_tests.rs:354](../src-tauri/src/commands/commands_tests.rs#L354) `test_receipt_datetime_warning_before_trip_start` | ✅ Full |
| E3 | Receipt after trip end | [commands_tests.rs:382](../src-tauri/src/commands/commands_tests.rs#L382) `test_receipt_datetime_warning_after_trip_end` | ✅ Full |
| E4 | Receipt exactly at start | [commands_tests.rs:448](../src-tauri/src/commands/commands_tests.rs#L448) `test_receipt_datetime_warning_exactly_at_start` | ✅ Full |
| E5 | Receipt exactly at end | [commands_tests.rs:472](../src-tauri/src/commands/commands_tests.rs#L472) `test_receipt_datetime_warning_exactly_at_end` | ✅ Full |
| E6 | Trip without end_datetime | [commands_tests.rs:496](../src-tauri/src/commands/commands_tests.rs#L496) `test_receipt_datetime_warning_no_end_datetime_uses_start` | ✅ Full |

---

## F. Mismatch Override Calculations

| # | Scenario | Test | Status |
|---|----------|------|--------|
| F1 | Override flag stored on receipt | [commands_tests.rs:1919](../src-tauri/src/commands/commands_tests.rs#L1919) `assert_eq!(assigned_receipt.mismatch_override, true)` | ✅ Full |
| F2 | Override excludes from datetime warnings | - | ❌ Missing |
| F3 | Legend count excludes overrides | - | ❌ Missing (frontend only) |

---

## G. Integration Tests

**File:** [receipts.spec.ts](../tests/integration/specs/tier2/receipts.spec.ts)

| Test | Line | Scenarios Covered |
|------|------|-------------------|
| `should get receipts for a specific vehicle via IPC` | [42](../tests/integration/specs/tier2/receipts.spec.ts#L42) | Vehicle filtering |
| `should return mismatch_reason when trip data differs` | [107](../tests/integration/specs/tier2/receipts.spec.ts#L107) | C4, D3 |
| `should return "matches" when data matches` | [182](../tests/integration/specs/tier2/receipts.spec.ts#L182) | A4, C3, D2 |
| `should return "empty" when trip has no fuel` | [251](../tests/integration/specs/tier2/receipts.spec.ts#L251) | D1 |
| `should create CZK receipt with NeedsReview` | [346](../tests/integration/specs/tier2/receipts.spec.ts#L346) | A2 |
| `should update CZK receipt with EUR conversion` | [391](../tests/integration/specs/tier2/receipts.spec.ts#L391) | A2→A4 |
| `should auto-populate total_price_eur for EUR` | [448](../tests/integration/specs/tier2/receipts.spec.ts#L448) | A4 |

---

## H. Receipt Extraction Tests

**File:** [receipts_tests.rs](../src-tauri/src/receipts_tests.rs)

| Test | Line | What it tests |
|------|------|---------------|
| `test_apply_extraction_high_confidence_eur_full_datetime` | [237](../src-tauri/src/receipts_tests.rs#L237) | Full datetime EUR → Parsed |
| `test_apply_extraction_foreign_currency_needs_review` | [274](../src-tauri/src/receipts_tests.rs#L274) | CZK → NeedsReview |
| `test_apply_extraction_low_confidence` | [305](../src-tauri/src/receipts_tests.rs#L305) | Low confidence → NeedsReview |
| `test_apply_extraction_date_only_triggers_needs_review` | [389](../src-tauri/src/receipts_tests.rs#L389) | Date-only → NeedsReview |
| `test_parse_receipt_datetime_full_datetime` | [348](../src-tauri/src/receipts_tests.rs#L348) | DateTime parsing |
| `test_parse_confidence` | [334](../src-tauri/src/receipts_tests.rs#L334) | Confidence parsing |

---

## Test Gaps to Address

### Priority 1: Missing Backend Tests

1. **A9: OTHER + mismatch + override**
   - Add test to `commands_tests.rs`
   - Verify `mismatch_override=true` works for OTHER type

2. **F2: Override excludes from datetime warnings**
   - Add test to verify `calculate_receipt_datetime_warnings` excludes overrides
   - Currently frontend filters this, backend should too

### Priority 2: Missing Integration Tests

3. **B4/B6b: Override suppresses trip grid indicator**
   - Add Tier 2 test to verify no warning shown when `mismatch_override=true`

4. **F3: Legend count excludes overrides**
   - Add Tier 2 test to verify legend count doesn't include confirmed mismatches

### Priority 3: Nice to Have

5. **A1: OCR pending state**
   - Would require async Gemini mock setup
   - Low priority - manual testing sufficient

6. **B8: Both FUEL and OTHER assigned**
   - Edge case with two receipts on one trip
   - Low priority

---

## Test Commands

```bash
# Run all backend tests (248 tests)
npm run test:backend

# Run receipt-specific tests
cd src-tauri && cargo test receipt --quiet

# Run assignment tests
cd src-tauri && cargo test assign --quiet

# Run integration tests
npm run test:integration

# Run Tier 2 only (includes receipts)
TIER=2 npm run test:integration
```

---

## File References

| File | Purpose | Test Count |
|------|---------|------------|
| [commands_tests.rs](../src-tauri/src/commands/commands_tests.rs) | Assignment, mismatch, missing receipt | ~30 |
| [receipts_tests.rs](../src-tauri/src/receipts_tests.rs) | OCR extraction, parsing | 17 |
| [receipts.spec.ts](../tests/integration/specs/tier2/receipts.spec.ts) | E2E receipt workflow | 7 |
