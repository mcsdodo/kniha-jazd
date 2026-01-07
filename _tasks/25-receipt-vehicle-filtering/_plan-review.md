# Plan Review

**Target:** `_tasks/25-receipt-vehicle-filtering/02-plan.md`
**Started:** 2026-01-07
**Status:** In Progress
**Focus:** Completeness, feasibility, clarity

## Iteration 1

### New Findings

#### Critical

1. **[Critical] Test helper functions don't exist (Task 1, lines 26-70)**

   The plan assumes test helpers that don't exist in the codebase:
   - `setup_test_db()` - Does not exist. Codebase uses `Database::in_memory()` directly.
   - `create_test_vehicle(&db, "Car A")` - Does not exist in this form. Actual helper is `create_test_vehicle(name: &str) -> Vehicle` (db.rs:1197) which returns a Vehicle struct, requiring separate `db.create_vehicle(&vehicle)` call.
   - `create_test_receipt(&db, Some(&vehicle_a.id), None)` - Does not exist. Codebase uses `Receipt::new()` or `Receipt::new_with_source_year()` constructors directly.
   - `create_test_receipt_with_date(&db, None, "2024-06-15")` - Does not exist.

   **Impact:** All three tests in Task 1 will fail to compile.

2. **[Critical] Wrong method name referenced (Task 1, line 109)**

   Plan references `Self::map_receipt_row(row)` but actual method is `Self::row_to_receipt(row)` (db.rs:969).

   **Impact:** Implementation code will fail to compile.

#### Important

1. **[Important] Test file location mismatch (Task 1)**

   Task file `01-task.md` specifies tests in `src-tauri/src/receipts_tests.rs`, but plan puts tests in `src-tauri/src/db.rs` inline tests. This contradicts CLAUDE.md:
   > "Tests are split into separate `*_tests.rs` files using the `#[path]` attribute pattern"

2. **[Important] ReceiptIndicator is NOT a no-op (Task 5)**

   `ReceiptIndicator.svelte` (lines 37-47) calls BOTH:
   - `api.getReceipts()` - NOT vehicle-filtered (returns all receipts)
   - `api.verifyReceipts(vehicle.id, ...)` - IS vehicle-aware

   Plan claims Task 5 "may be a no-op" but this is **incorrect**. Badge currently counts ALL receipts, not just active vehicle's receipts. This needs change.

3. **[Important] Doklady `filteredReceipts` interaction unclear (Task 4)**

   Current `filteredReceipts` derivation (lines 303-309) uses `isReceiptVerified(r.id)` from `verifyReceipts` which is vehicle-aware. The "Unassigned" filter checks `!isReceiptVerified(r.id)` - this logic may produce unexpected results when combined with new vehicle filtering.

   Plan should clarify interaction between backend vehicle filtering and tab filtering.

4. **[Important] Missing edge case: no active vehicle (Task 4)**

   Plan shows `if (!vehicle)` returning empty receipts, but doesn't address order dependency. During transition before Task 6/7 complete, or if no vehicles exist, edge case needs explicit handling.

5. **[Important] i18n key not used (Task 6)**

   Plan's "After" code uses hardcoded "No vehicles" string, but current code uses `$LL.app.vehiclePlaceholder()`. Plan should preserve i18n pattern.

6. **[Important] Auto-select breaks parallel loading (Task 7)**

   Current `onMount` uses `Promise.all` for parallel loading. Plan's sequential auto-select logic would increase load time.

#### Minor

1. **[Minor] Year parameter handling (Task 3)** - `year ?? null` vs `undefined` in Tauri IPC may differ.

2. **[Minor] Commit message style** - Could be more concise.

3. **[Minor] Missing Uuid import mention (Task 2)** - Should verify import exists.

4. **[Minor] Duplicate year filtering SQL (Task 1)** - Could share logic with `get_receipts_for_year`.

5. **[Minor] Manual testing steps vague (Task 8)** - Doesn't specify test data setup.

### Coverage Assessment

**Reviewed thoroughly:**
- Database layer (`db.rs`) - test helpers, existing patterns, method names
- Commands layer (`commands.rs`) - existing command patterns
- Frontend API (`api.ts`) - wrapper patterns
- Doklady page (`+page.svelte`) - receipt loading, filtering logic
- ReceiptIndicator (`ReceiptIndicator.svelte`) - badge count logic
- Layout (`+layout.svelte`) - vehicle selector, onMount initialization

**Areas needing more analysis:**
- Integration between `verifyReceipts` and vehicle-filtered receipts
- Interaction between year filtering and vehicle filtering
- Potential Tauri IPC serialization issues

---

## Iteration 2

### New Findings

#### Critical

No new critical findings.

#### Important

1. **[Important] Missing database index on `vehicle_id` (Performance)**

   The schema in `db.rs` (lines 114-116) creates indexes on `status`, `trip_id`, and `receipt_date`, but NOT on `vehicle_id`. The new query `WHERE (vehicle_id IS NULL OR vehicle_id = ?1)` will cause full table scans.

   **Fix needed:** Add `CREATE INDEX IF NOT EXISTS idx_receipts_vehicle ON receipts(vehicle_id);`

2. **[Important] Doklady page does NOT re-fetch on vehicle change (Functionality)**

   In `doklady/+page.svelte` (lines 58-67), the `$effect` only watches `$selectedYearStore`, NOT `$activeVehicleStore`. When user switches vehicles, receipts won't reload automatically.

   **Fix needed:** Add `$activeVehicleStore` to the effect dependencies in Task 4.

3. **[Important] Badge count inconsistency with `verify_receipts` (UX)**

   `verify_receipts` in `commands.rs` (lines 1897-1906) fetches ALL receipts and filters only by year, not vehicle. After plan's changes:
   - Doklady shows filtered receipts (e.g., 2)
   - Badge shows ALL receipts count (e.g., 5)

   **Fix needed:** Update `ReceiptIndicator` to use new `getReceiptsForVehicle` or update `verify_receipts` command.

4. **[Important] Missing error handling for non-existent vehicle_id**

   Task 2's command handles invalid UUID format, but not the case where vehicle_id is valid but doesn't exist. Behavior is acceptable (returns empty vec) but should be documented.

#### Minor

1. **[Minor] Source year filtering inconsistency**

   Plan's SQL handles `source_year` for year filtering, but `verify_receipts` only filters by `receipt_date.year()`, ignoring `source_year`. Could cause inconsistent counts.

2. **[Minor] No handling of concurrent vehicle switching**

   Rapid vehicle switches could cause race conditions in `loadReceipts()` calls. Low priority as unlikely in practice.

### Coverage Assessment

**Newly reviewed:**
- Database schema and indexes
- Svelte reactivity patterns in Doklady page
- Store synchronization between components
- `verify_receipts` command behavior

**Remaining areas:**
- Tauri IPC edge cases (low priority)
- Concurrent request handling (low priority)
