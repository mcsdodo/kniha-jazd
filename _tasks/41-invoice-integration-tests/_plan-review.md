# Plan Review: Invoice Integration Tests (Task 41)

**Date:** 2026-01-21
**Reviewer:** Claude
**Recommendation:** **Needs Revisions**

---

## Summary

The plan is well-structured and correctly identifies the core problems (LLM mocking, file seeding). However, several gaps and inconsistencies need addressing before implementation:

1. **Critical:** Test config already sets `KNIHA_JAZD_MOCK_GEMINI=true` but the plan proposes `KNIHA_JAZD_MOCK_GEMINI_DIR` - these are different mechanisms
2. **Critical:** Invoice.json format doesn't match `ExtractedReceipt` struct - will cause deserialization failures
3. **Important:** No `seedReceipt()` function exists in `db.ts` and no `create_receipt` Tauri command exists
4. **Important:** Plan references `process_receipt` command that doesn't exist - actual command is `reprocess_receipt`

---

## Findings by Category

### Critical (Must Fix Before Implementation)

**[C1] Mock Mechanism Mismatch**
- **Location:** Phase 1.1 (Add Mock Gemini Support)
- **Issue:** `wdio.conf.ts` line 166 already sets `KNIHA_JAZD_MOCK_GEMINI = 'true'` but no Rust code checks this. The plan proposes a different env var `KNIHA_JAZD_MOCK_GEMINI_DIR`.
- **Impact:** Need to decide: implement the existing boolean flag OR change the wdio.conf to use the new DIR-based approach.
- **Suggested Fix:** Use the DIR-based approach (Option A) but update `wdio.conf.ts` to replace the boolean flag with the directory path.

**[C2] Invoice.json Format Mismatch**
- **Location:** Phase 1.2, test fixture `tests/integration/data/invoice.json`
- **Issue:** Current file uses:
  ```json
  { "price": 91.32, "date": "2026-01-20", "litres": 63.680, "station": "Slovnaft, a.s." }
  ```
  But `ExtractedReceipt` struct (gemini.rs:8-18) expects:
  ```json
  { "total_price_eur": 91.32, "receipt_date": "2026-01-20", "liters": 63.68,
    "station_name": "Slovnaft, a.s.", "confidence": {...} }
  ```
- **Impact:** Mock loading will fail with deserialization error.
- **Suggested Fix:** Update `invoice.json` to match `ExtractedReceipt` schema including required `confidence` field.

**[C3] Missing Receipt Seeding Mechanism**
- **Location:** Phase 2.3, Phase 3 tests
- **Issue:** The plan assumes `seedReceipt()` utility exists or that `create_receipt` Tauri command exists. Neither exists. The only way to create receipts is via folder scanning (`scan_receipts` command).
- **Impact:** Tests cannot seed receipts without implementing either:
  1. A test-only `seed_receipt` Tauri command (Option B from design), OR
  2. File seeding + mock Gemini + scan workflow (Option A)
- **Suggested Fix:** Plan should explicitly state Phase 1 must complete before any Phase 3 tests can work. Add Phase 1.4: "Add test-only `seed_receipt` command for direct receipt insertion (bypasses scan+OCR)".

### Important (Should Fix)

**[I1] Incorrect Command Name**
- **Location:** Phase 2.3 (`processReceiptWithMock`)
- **Issue:** Plan references `process_receipt` command. Actual commands are:
  - `sync_receipts` - scan + process all pending
  - `reprocess_receipt` - reprocess single receipt by ID
- **Impact:** Test helper will fail.
- **Suggested Fix:** Use `reprocess_receipt` or `sync_receipts` in the helper implementation.

**[I2] Trip Assignment Mismatch Tests May Already Exist**
- **Location:** Phase 3.3.1 (Backend Unit Tests)
- **Issue:** `commands.rs` lines 4656-4803 already contain 4 tests for `get_trips_for_receipt_assignment`:
  - `test_get_trips_for_receipt_assignment_empty_trip_returns_can_attach_true`
  - `test_get_trips_for_receipt_assignment_matching_fuel_returns_can_attach_true`
  - `test_get_trips_for_receipt_assignment_different_liters_returns_can_attach_false`
  - `test_get_trips_for_receipt_assignment_different_price_returns_can_attach_false`
  - `test_get_trips_for_receipt_assignment_different_date_returns_can_attach_false`
- **Impact:** Plan lists 9 tests but 5 may already exist (need to verify).
- **Suggested Fix:** Before Phase 3.3.1, grep for existing tests and only add missing cases. Update checklist to say "Verify existing coverage, add missing".

**[I3] File Structure Reorganization May Break Things**
- **Location:** Phase 2.1 (Reorganize Test Data)
- **Issue:** Plan proposes moving `invoice.pdf` to `invoices/` and `invoice.json` to `mocks/`, but current files are at root of `tests/integration/data/`.
- **Impact:** Moving files changes paths - need to update plan to specify exact source/destination.
- **Suggested Fix:** Add explicit commands:
  ```
  mkdir tests/integration/data/invoices
  mkdir tests/integration/data/mocks
  move tests/integration/data/invoice.pdf tests/integration/data/invoices/
  move tests/integration/data/invoice.json tests/integration/data/mocks/
  ```

**[I4] Missing Dependency: receipts_folder_path Setting**
- **Location:** Phase 2.2 (env vars for receipt folder)
- **Issue:** Receipt scanning reads folder from `LocalSettings.receipts_folder_path`. The plan sets `KNIHA_JAZD_RECEIPTS_FOLDER` env var but there's no code checking this env var in `receipts.rs` or `commands.rs`.
- **Impact:** Even if env var is set, app won't use it unless code is added.
- **Suggested Fix:** Either:
  1. Add env var check in `scan_receipts` command, OR
  2. Use `seedSettings()` to set `receipts_folder_path` before tests

### Minor (Nice to Have)

**[M1] Tolerance Value Not Specified in Plan**
- **Location:** Design doc mentions "Tolerance: +/-0.01 for liters and price"
- **Issue:** Plan doesn't specify where this tolerance is defined or tested.
- **Suggested Fix:** Reference existing tolerance logic in `check_receipt_trip_compatibility()` (commands.rs:2296-2303).

**[M2] wdio.conf.ts Redundant Mock Flag**
- **Location:** `wdio.conf.ts` line 166
- **Issue:** Line `process.env.KNIHA_JAZD_MOCK_GEMINI = 'true'` does nothing since no code checks it.
- **Suggested Fix:** Either implement the check or remove the dead code.

**[M3] Plan Phase Numbering**
- **Location:** 03-plan.md
- **Issue:** Phase 4.3 lists "Add more test fixtures" but this is optional/stretch goal, not required for MVP.
- **Suggested Fix:** Mark 4.3 as "Optional" or move to separate "Future Enhancements" section.

---

## Checklist Verification

| Criterion | Status | Notes |
|-----------|--------|-------|
| Tasks have specific file paths | Partial | Most paths specified, but some helpers missing exact location |
| Verification steps are defined | Yes | Each phase has verification items |
| Steps are in correct order | Partial | Phase 1 must complete before Phase 3 - not explicit |
| No scope creep beyond testing | Yes | Plan stays focused on testing |
| References task 42 correctly | Yes | Correctly notes verification mismatch is done |

---

## Recommended Revisions

### Before Implementation

1. **Update `invoice.json`** to match `ExtractedReceipt` struct format (add confidence field, fix field names)

2. **Clarify mock approach:** Either:
   - Remove `KNIHA_JAZD_MOCK_GEMINI=true` from wdio.conf.ts AND implement `KNIHA_JAZD_MOCK_GEMINI_DIR` in Rust, OR
   - Keep boolean flag but implement it in Rust to return hardcoded mock data

3. **Add Phase 1.4:** Add `seed_receipt` test-only command (or clarify that tests MUST use scan workflow)

4. **Fix command names:** `process_receipt` -> `reprocess_receipt` or `sync_receipts`

5. **Check existing test coverage** before adding Phase 3.3.1 unit tests

### During Implementation

1. Update env var in wdio.conf.ts when Phase 1 is complete
2. Run `cargo test` after each Phase 1 change to verify
3. Run single integration test after Phase 2 setup before enabling all skipped tests

---

## Iteration Notes

**Round 1:** Initial review identified C1-C3, I1-I4, M1-M3
**Round 2:** Verified findings against source code - no new issues found
**Round 3:** Cross-checked with existing test infrastructure - confirmed no seedReceipt exists
**Round 4:** Final check - no additional findings

Review complete.

---

## Resolution (2026-01-21)

**Addressed:** Critical (C1-C3) + Important (I1-I4)
**Skipped:** Minor (M1-M3)

| Finding | Status | Resolution |
|---------|--------|------------|
| [C1] Mock mechanism mismatch | [x] Fixed | Added note about dead code in wdio.conf.ts, documented DIR-based approach |
| [C2] Invoice.json format | [x] Fixed | Added field mapping table, showed correct vs current format |
| [C3] Missing seed mechanism | [x] Fixed | Added Phase 1.4 documenting Option A (scan workflow) vs Option B (test command) |
| [I1] Wrong command name | [x] Fixed | Changed `process_receipt` to `reprocess_receipt` and `sync_receipts` |
| [I2] Existing tests | [x] Fixed | Added list of 5 existing tests, "check first" note |
| [I3] File reorganization | [x] Fixed | Added explicit mkdir/mv commands with before/after structure |
| [I4] Receipts folder path | [x] Fixed | Clarified env var not implemented, added settings-based Option A |
| [M1] Tolerance value | Skipped | Minor - existing code has tolerance |
| [M2] wdio dead code | Skipped | Will be fixed when implementing C1 |
| [M3] Phase numbering | Skipped | Minor clarity issue |

**Plan status:** Ready for implementation
