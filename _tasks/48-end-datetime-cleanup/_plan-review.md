# Plan Review: end_datetime Cleanup

**Reviewed:** 2026-01-29
**Plan:** 02-plan.md
**Status:** ✅ Approved (after revisions)

## Findings

### Critical

- [x] **Incorrect file path in Phase 3**: Plan references `commands/trips.rs` but should be explicit about the path `src-tauri/src/commands/trips.rs`. The file exists and contains both `create_trip` and `update_trip` functions with `extract_time_string` usage (lines 74, 167). This is the correct file but path should be absolute.

- [x] **Missing file: commands/mod.rs**: The plan omits `src-tauri/src/commands/mod.rs` which contains:
  - `extract_time_string()` helper function (line 76) - must be removed after cleanup
  - `calculate_trip_numbers()` (line 120) - uses `trip.date` for sorting
  - `calculate_odometer_start()` (line 143) - uses `trip.date` for sorting
  - Date warning logic (lines 1432-1439) - uses `trip.date` comparisons
  - Receipt matching logic (lines 1483, 2081, 2195, 2364-2382, 2445) - uses `trip.date`

  **Impact:** If `date` field is removed from Trip, all these usages must change to `start_datetime.date()`.

### Important

- [x] **export.rs date usage**: Plan mentions `export.rs` for test updates but doesn't note that production code at line 213 uses `trip.date.format("%d.%m.")`. This needs updating to `trip.start_datetime.date().format(...)` if the `date` field is removed.

- [x] **Phase 6 decision needed upfront**: The plan defers the DB migration decision ("Option A or B, decide later") but this affects the entire implementation approach:
  - **Option A (map in code)**: TripRow keeps `datetime`, `date`, `end_time` columns; Trip model uses new names internally
  - **Option B (migration)**: Columns get renamed; cleaner but requires careful From<TripRow> impl changes

  **Recommendation:** Option A is safer for initial implementation. The DB columns can be renamed in a follow-up task after the model is stable.

- [x] **Missing: calculations_tests.rs check**: Plan lists `calculations_tests.rs` for updates but doesn't verify what needs changing. I checked: `calculate_closed_period_totals()` in calculations.rs only uses `trip.distance_km` and `trip.fuel_liters` - no date/datetime fields. Test file uses `Trip::test_ice_trip()` helper which will be updated in Task 4.1.

### Minor

- [x] **Task ordering clarification**: Phase 3 (commands) cannot be done before Phase 1 (models) since commands create Trip structs. The plan correctly sequences these but should note the dependency explicitly.

- [x] **Verification steps in Phase 5**: Missing `npm run lint` and `npm run format` checks. Should also verify frontend still works with `npm run tauri dev`.

- [x] **End_time to end_datetime type change**: The plan says replace `end_time: Option<String>` with `end_datetime: NaiveDateTime`. Current code in `commands/trips.rs` has:
  ```rust
  let end_time = Some(extract_time_string(&trip_end_datetime));
  ```
  After cleanup, this becomes:
  ```rust
  let end_datetime = trip_end_datetime; // Just use the parsed value directly
  ```
  The plan should note that `end_datetime` should be `Option<NaiveDateTime>` (not required) to match the nullable DB column `end_datetime -> Nullable<Text>` in schema.rs.

## Summary of Missing Files

Files that need updates but aren't listed in the plan:

| File | Changes Needed |
|------|----------------|
| `src-tauri/src/commands/mod.rs` | Replace `trip.date` usages with `trip.start_datetime.date()`, remove `extract_time_string()` |
| `src-tauri/src/export.rs` | Line 213: update `trip.date.format()` to use start_datetime |

## Phase 6 Assessment

**Initial question:** "Is Phase 6 (DB migration) actually needed or over-engineering?"

**Initial answer (incorrect):** Recommended removing Phase 6.

**User correction:** Phase 6 IS needed, but should DROP columns, not rename:
1. Task 47 already added `start_datetime` and `end_datetime` columns with backfilled data
2. After code cleanup, old columns (`datetime`, `date`, `end_time`) become dead weight
3. Per ADR-012: forward-only migrations allow dropping deprecated columns
4. Leaving unused columns is technical debt

**Revised recommendation:** Keep Phase 6 as DROP migration (not rename).

## Recommendation

The plan is **structurally sound** but needed these revisions before implementation:

1. **Add `commands/mod.rs`** to Phase 3 with explicit list of `trip.date` usages to update
2. **Add `export.rs` production code** to Phase 3 (not just test code in Phase 4)
3. **Keep Phase 6** but rewrite as DROP migration (not rename) - see user feedback
4. **Clarify `end_datetime` type** as `Option<NaiveDateTime>` to match nullable DB column
5. **Add verification steps**: `npm run lint`, `npm run format`, `npm run tauri dev`

After these revisions, the plan is ready for implementation.

---

## Resolution (2026-01-29)

**User feedback:** Phase 6 should DROP columns, not rename. Task 47 already added `start_datetime` and `end_datetime` columns with backfilled data.

**Applied changes:**
1. ✅ Added `commands/mod.rs` to Phase 3 with explicit list of 11 `trip.date` usages
2. ✅ Added `export.rs` production code changes (lines 246, 254)
3. ✅ Rewrote Phase 6 to DROP obsolete columns (not rename)
4. ✅ Clarified `end_datetime` as `Option<NaiveDateTime>`
5. ✅ Added verification steps (`npm run lint`, `npm run format`, `npm run tauri dev`)
6. ✅ Added SQLite table rebuild pattern (SQLite doesn't support DROP COLUMN directly)
7. ✅ Updated "Approach" section to clarify simplified strategy

**Plan status:** Ready for implementation.
