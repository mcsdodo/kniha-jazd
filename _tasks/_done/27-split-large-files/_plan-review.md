# Plan Review

**Target:** `_tasks/27-split-large-files/02-plan.md`
**Started:** 2026-01-07
**Status:** Complete
**Focus:** Completeness, feasibility, clarity

## Iteration 1

### Coverage Assessment

Reviewed:
- Task requirements (`01-task.md`)
- Implementation plan (`02-plan.md`)
- Source files (`commands.rs` - 3,044 lines, `db.rs` - 1,805 lines)
- Command registration (`main.rs` / `lib.rs`)

### New Findings

#### Critical

1. **[Critical] Incorrect Line Estimate for `receipts.rs`**
   - **Location:** Line 21 of `02-plan.md`
   - **Issue:** Plan states `receipts.rs` will have 11 commands at ~1,450 lines, but actual receipt commands total ~260-300 lines
   - **Cause:** The 1,450 figure likely includes the ~870 lines of tests at the end of `commands.rs` which should go to `grid_tests.rs`, not `receipts.rs`
   - **Fix:** Update line estimate to ~300 lines

#### Important

2. **[Important] Struct Definitions Not Mentioned**
   - **Location:** Not explicitly covered in plan
   - **Issue:** Several types defined in `commands.rs` must move to their respective modules (not just re-exported):
     - `BackupInfo` (lines 26-34) → `backup.rs`
     - `ReceiptSettings`, `SyncResult`, `SyncError`, `ScanResult`, `ProcessingProgress` → `receipts.rs`
     - `WindowSize` → `window.rs`
     - `PhevGridData` → `grid.rs` (internal struct)
   - **Fix:** Add a section listing struct definitions that must move with their commands

3. **[Important] Incomplete Helper Function List for `grid.rs`**
   - **Location:** Lines 116-122 of `02-plan.md`
   - **Issue:** Plan lists only 3 `pub(crate)` helpers, but `commands.rs` has more internal functions:
     - `calculate_energy_grid_data` (~90 lines)
     - `calculate_phev_grid_data` (~135 lines)
     - `calculate_date_warnings` (~35 lines)
     - `calculate_consumption_warnings` (~20 lines)
     - `calculate_missing_receipts` (~25 lines)
   - **Fix:** Either list all helpers, or clarify these stay private within `grid.rs` since only used by `get_trip_grid_data`

4. **[Important] Missing Re-export Verification Step**
   - **Location:** Line 189 of `02-plan.md`
   - **Issue:** Plan says "lib.rs NO CHANGES" but doesn't verify all 40 commands are re-exported correctly from `commands/mod.rs`
   - **Fix:** Add verification step to ensure all commands from `lib.rs` invoke_handler are re-exported

5. **[Important] Test Module Pattern Not Shown for db**
   - **Location:** Line 35 of `02-plan.md`
   - **Issue:** Plan shows `db_tests.rs` but doesn't show how `#[path]` directive will be added
   - **Fix:** Add example showing where the test module declaration goes:
     ```rust
     // In db/mod.rs
     #[cfg(test)]
     #[path = "db_tests.rs"]
     mod tests;
     ```

#### Minor

6. **[Minor] Line Count Estimates Slightly Off**
   - **Location:** Lines 9-22 of `02-plan.md`
   - **Issue:** `grid.rs` estimated at ~850 lines but actual content ~695 lines without tests/imports
   - **Impact:** Low - files will still be under target

7. **[Minor] `get_purposes` Placement Implicit**
   - **Location:** Line 13 of `02-plan.md`
   - **Issue:** Plan shows `route.rs` with 2 commands but doesn't explicitly name them (`get_routes`, `get_purposes`)
   - **Impact:** Low - placement is logical

## Review Summary

**Status:** Complete
**Iterations:** 1
**Total Findings:** 1 Critical, 4 Important, 2 Minor

### All Findings (Consolidated)

#### Critical
1. [x] Update `receipts.rs` line estimate from ~1,450 to ~300 lines - `02-plan.md` line 20

#### Important
1. [x] Add section listing struct definitions that must move with their commands - added "Struct Definitions to Move" section
2. [x] Expand helper function list for `grid.rs` or clarify they stay private - updated "Internal Helper Visibility" section
3. [x] Add verification step for all 40 command re-exports - added "Re-export Verification" section
4. [x] Add `#[path]` example for db test module pattern - added to "Test File Pattern" section

#### Minor
1. [x] Refine line count estimates - updated `grid.rs` to ~700 lines
2. [x] Explicitly name route.rs commands - added `get_routes, get_purposes`

### Recommendation

**Ready for implementation.** All findings have been addressed.

### Feasibility Assessment

| Aspect | Assessment |
|--------|------------|
| Technical approach | Sound - standard Rust module patterns |
| Dependency order | Correct - db first, then commands |
| Risk mitigation | Adequate - incremental with verification |
| Effort estimate | Reasonable - 2-3 hours |

---

## Resolution

**Addressed:** 7 findings (all)
**Skipped:** 0 findings
**Status:** Complete

### Applied Changes

| Finding | Resolution |
|---------|------------|
| Critical #1: receipts.rs line estimate | Updated from ~1,450 to ~300 lines |
| Important #1: Struct definitions | Added "Struct Definitions to Move" table listing 8 structs |
| Important #2: Helper function list | Expanded to show all 10 helpers with visibility (pub(crate) vs private) |
| Important #3: Re-export verification | Added complete list of all 40 commands by domain |
| Important #4: db test pattern | Added `#[path = "db_tests.rs"]` example in Test File Pattern section |
| Minor #1: Line estimates | Updated `grid.rs` from ~850 to ~700 lines |
| Minor #2: route.rs commands | Explicitly named `get_routes, get_purposes` |
