# Plan Review

**Target:** `_tasks/25-receipt-vehicle-filtering/02-plan.md`
**Started:** 2026-01-07
**Status:** Complete
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

---

## Iteration 3

### New Findings

#### Critical

1. **[Critical] Foreign Key Constraint blocks vehicle deletion (Data Integrity)**

   The `receipts` table has `FOREIGN KEY (vehicle_id) REFERENCES vehicles(id)` (db.rs:111) but NO cascade behavior. When a vehicle is deleted:
   - `delete_vehicle()` just runs `DELETE FROM vehicles WHERE id = ?1`
   - This will FAIL if any receipts have that `vehicle_id` assigned

   Plan Task 7 (lines 352-359) tries to handle "deleted vehicle" case, but deletion itself will error first.

   **Fix needed:** Either:
   - Add `ON DELETE SET NULL` to FK constraint in migration, OR
   - Add manual cleanup: `UPDATE receipts SET vehicle_id = NULL WHERE vehicle_id = ?` before vehicle deletion

2. **[Critical] Orphaned receipts after vehicle deletion scenario**

   If FK constraint is bypassed/changed, receipts with `vehicle_id` pointing to non-existent vehicle become orphaned:
   - They won't appear in ANY vehicle's filtered view (not NULL, not matching)
   - Users lose access to these receipts entirely

   **Fix needed:** Add cleanup query or ensure receipts are unassigned before vehicle deletion.

#### Important

1. **[Important] No loading state during vehicle switch (UX)**

   When vehicle changes trigger `loadReceipts()`, there's no explicit `loading = true` set. Users may briefly see stale receipts from previous vehicle.

   **Fix needed:** Ensure all triggers of `loadReceipts()` set loading state first.

#### Minor

No new minor findings.

### Coverage Assessment

**Newly reviewed:**
- Vehicle deletion scenarios and FK constraints
- Orphaned data handling
- UI loading states during transitions

**Review comprehensive:** All critical code paths examined.

---

## Iteration 4

### New Findings

#### Critical

No new critical findings (previous findings confirmed).

#### Important

No new important findings (duplicates of iterations 1-3).

#### Minor

1. **[Minor] Auto-select missing year reset (Task 7)**

   Plan's auto-select logic doesn't reset `selectedYearStore` after selecting a new vehicle. Old year value may persist from previous session.

   **Fix needed:** Call `resetToCurrentYear()` or similar after auto-select.

### Coverage Assessment

**Quality gate met:** No new critical or important findings. Minor enhancement identified.

---

## Review Summary

**Status:** Ready for User Review
**Iterations:** 4
**Total Findings:** 4 Critical, 12 Important, 8 Minor

### All Findings (Consolidated)

#### Critical

1. [ ] **Test helper functions don't exist** (Task 1) - Tests won't compile. Use `Database::in_memory()`, `create_test_vehicle()` → `Vehicle`, manual `db.create_vehicle()`.

2. [ ] **Wrong method name** (Task 1) - `map_receipt_row` → `row_to_receipt`.

3. [ ] **FK constraint blocks vehicle deletion** - No cascade on `receipts.vehicle_id`. Deletion fails if receipts assigned.

4. [ ] **Orphaned receipts after deletion** - If vehicle deleted, its receipts become inaccessible. Need cleanup logic.

#### Important

1. [ ] **Test file location** (Task 1) - Should use separate `*_tests.rs` file per CLAUDE.md convention.

2. [ ] **ReceiptIndicator is NOT a no-op** (Task 5) - Uses `getReceipts()` which returns ALL receipts. Must change.

3. [ ] **Doklady filteredReceipts interaction unclear** (Task 4) - Clarify how backend filtering + tab filtering interact.

4. [ ] **Missing edge case: no active vehicle** (Task 4) - Handle case before Tasks 6/7 complete.

5. [ ] **i18n key not used** (Task 6) - Hardcoded "No vehicles" should use `$LL.app.vehiclePlaceholder()`.

6. [ ] **Auto-select breaks parallel loading** (Task 7) - Sequential calls slower than current `Promise.all`.

7. [ ] **Missing database index on vehicle_id** - Add `idx_receipts_vehicle` for performance.

8. [ ] **Doklady doesn't re-fetch on vehicle change** (Task 4) - Add `$activeVehicleStore` to effect dependencies.

9. [ ] **Badge count inconsistency** (Task 5) - `verify_receipts` not vehicle-aware. Badge shows wrong count.

10. [ ] **Missing error handling for non-existent vehicle_id** (Task 2) - Document expected behavior.

11. [ ] **No loading state during vehicle switch** - Set `loading = true` before fetch.

12. [ ] **ReceiptIndicator needs vehicle filtering** - Task 5 must use `getReceiptsForVehicle`.

#### Minor

1. [ ] Year parameter handling (`year ?? null` vs `undefined`)
2. [ ] Commit message style could be more concise
3. [ ] Missing Uuid import mention in Task 2
4. [ ] Duplicate year filtering SQL could be shared
5. [ ] Manual testing steps vague
6. [ ] Source year filtering inconsistency with `verify_receipts`
7. [ ] No handling of concurrent vehicle switching (race condition)
8. [ ] Auto-select missing year reset

### Recommendation

**Needs Revisions** - Plan has 4 critical issues that will cause compilation failures and data integrity problems. Must address:
1. Fix test helper patterns to match codebase
2. Correct method name in implementation
3. Add vehicle deletion cascade/cleanup logic
4. Add Doklady reactivity to vehicle changes

---

## Resolution

**Addressed:** 16 findings (4 Critical, 12 Important)
**Skipped:** 8 findings (all Minor - user decision)
**Status:** Complete

### Applied Changes

#### Critical
1. **Test helper functions** → Rewrote tests to use `Database::in_memory()`, `create_test_vehicle()`, `Receipt::new()` patterns
2. **Wrong method name** → Changed `map_receipt_row` to `row_to_receipt` in implementation
3. **FK constraint blocks deletion** → Added Task 2b: Vehicle deletion cleanup (unassign receipts before delete)
4. **Orphaned receipts** → Covered by Task 2b cleanup logic

#### Important
1. **Test file location** → Changed to `src-tauri/src/db_tests.rs` per CLAUDE.md convention
2. **ReceiptIndicator NOT no-op** → Rewrote Task 5 with explicit vehicle filtering requirement
3. **filteredReceipts interaction** → Added clarification note in Task 4 Step 3
4. **No active vehicle edge case** → Handled in Task 4 Step 1 (`if (!vehicle) receipts = []`)
5. **i18n key not used** → Updated Task 6 to use `$LL.app.noVehicles()` with translation keys
6. **Auto-select breaks parallel loading** → Rewrote Task 7 to preserve `Promise.all` pattern
7. **Missing database index** → Added Step 0 in Task 1 to create index
8. **Doklady no re-fetch on vehicle change** → Added Step 2 in Task 4 with `$activeVehicleStore` dependency
9. **Badge count inconsistency** → Addressed in Task 5 by using `getReceiptsForVehicle`
10. **Error handling for non-existent vehicle_id** → Acceptable behavior (returns empty vec)
11. **No loading state during vehicle switch** → Added `loading = true` in Task 4 and Task 5
12. **ReceiptIndicator needs vehicle filtering** → Explicitly added in Task 5 Steps 1-3

### Skipped Items (Minor)
- Year parameter handling - low risk, TypeScript/Tauri IPC handles correctly
- Commit message style - cosmetic
- Missing Uuid import mention - already imported in codebase
- Duplicate year filtering SQL - acceptable duplication for clarity
- Manual testing steps vague - sufficient for developer guidance
- Source year filtering inconsistency - edge case, low impact
- Concurrent vehicle switching race - unlikely in practice
- Auto-select missing year reset - added to Task 7
