# Plan Review: Datetime Field Consolidation

**Date:** 2026-01-29
**Plan:** `02-plan.md`
**Status:** Needs Revisions

## Summary

The plan has several issues stemming from being partially implemented already. Phase 1 (migration) is ALREADY DONE but the plan doesn't reflect this. More critically, the migration has a semantic bug, and the existing code for `update_trip` doesn't sync the new fields.

## Findings

### Critical

- [ ] **Migration already exists but sets end_datetime incorrectly**
  - **Issue:** Migration `2026-01-29-193744-0000_add_start_end_datetime/up.sql` exists and sets `end_datetime = date || 'T00:00:00'` for trips without `end_time`. This makes NULL (no end time) indistinguishable from "ended at midnight".
  - **Impact:** Data corruption - cannot tell if a trip has no end time recorded vs ended at midnight. This affects future queries and display logic.
  - **Suggestion:** Create a fix-up migration that sets `end_datetime = NULL` WHERE `end_time IS NULL OR end_time = ''`. The migration is already deployed so we can't change it, but we can fix the data.

- [ ] **update_trip does not sync start_datetime and end_datetime**
  - **Issue:** `db.rs:update_trip()` (lines 380-402) updates `datetime` and `end_time` but does NOT update `start_datetime` and `end_datetime`, causing the new fields to get out of sync when trips are edited.
  - **Impact:** After editing a trip, `start_datetime`/`end_datetime` will contain stale data, causing inconsistencies.
  - **Suggestion:** Add `trips::start_datetime.eq(&datetime_str)` and `trips::end_datetime.eq(...)` to the update query. This is required for correctness.

### Important

- [ ] **Phase 1 says "Create Migration" but migration already exists**
  - **Issue:** The plan's Task 1.1 says `diesel migration generate add_start_end_datetime` but migration `2026-01-29-193744-0000_add_start_end_datetime` already exists. Schema.rs and models.rs already have the fields.
  - **Impact:** Confusion for implementer - Phase 1 is done (partially incorrectly). The plan needs updating to reflect current state.
  - **Suggestion:** Update plan to mark Phase 1 as "DONE with issues" and add Task 1.3 for fix-up migration.

- [ ] **Trip domain struct doesn't have start_datetime/end_datetime fields (as planned)**
  - **Issue:** Plan Task 2.1 says to add `start_datetime: NaiveDateTime` and `end_datetime: Option<NaiveDateTime>` to Trip struct, but the plan's goal is to use these as primary and keep old fields. Current Trip struct uses `datetime` and `end_time` as primary.
  - **Impact:** The consolidation benefit (cleaner code, single source of truth) won't be achieved if domain struct still uses old field names.
  - **Suggestion:** Clarify: Either (A) rename `datetime` to `start_datetime` in Trip struct (breaking change to frontend), or (B) keep `datetime`/`end_time` names in domain model and only use new DB columns internally. The plan should be explicit about which approach.

- [ ] **Plan references "commands/trips.rs" but structure is "commands/mod.rs" + "commands/trips.rs"**
  - **Issue:** Task 3.1/3.2 reference `commands/trips.rs` which exists, but Task 3.3 references `commands/mod.rs`. The file structure is `src-tauri/src/commands/` with separate files.
  - **Impact:** Minor confusion - paths are mostly correct.
  - **Suggestion:** Verify each task references the correct file in `src-tauri/src/commands/`.

- [ ] **"~50 locations" estimate is inaccurate**
  - **Issue:** Plan says "~10 files, ~50 locations". Grep for `Trip {` shows 41 occurrences across 7 files. If adding new fields to Trip struct, each initialization needs updating.
  - **Impact:** Estimate is close but understates test helper functions that create Trip structs (5+ helper functions each called multiple times).
  - **Suggestion:** Count is approximately correct. The key effort is in test helpers (`make_trip_with_fuel`, `make_trip_detailed`, `make_trip_for_magic_fill`, etc.).

### Minor

- [ ] **Phase 6 test count is outdated**
  - **Issue:** Plan says "237 backend tests" but CLAUDE.md says "195 tests".
  - **Impact:** None - just outdated documentation.
  - **Suggestion:** Update count or say "all backend tests pass" without specific number.

- [ ] **Missing verification step for data integrity after migration fix**
  - **Issue:** No verification step to ensure existing data is correctly migrated after the fix-up migration runs.
  - **Impact:** Could miss migration issues.
  - **Suggestion:** Add Task 1.4: "Verify migration - SELECT COUNT(*) of trips where start_datetime != datetime (should be 0)".

- [ ] **Frontend types.ts may need Trip type update if field names change**
  - **Issue:** Task 5.1 says "Likely no changes needed" but if Trip domain struct gets new field names, TypeScript types will need updating.
  - **Impact:** TypeScript compilation errors if frontend expects different field names.
  - **Suggestion:** Add explicit check: "If Trip struct fields renamed, update `src/lib/types.ts`".

## Iteration Notes

**Iteration 1:** Identified migration semantic bug, update_trip missing sync, plan outdated (Phase 1 done).

**Iteration 2:** Verified Trip struct does NOT have new fields yet (still uses datetime/end_time), confirmed test helper count (~41 Trip struct initializations + 5 helper functions).

**Iteration 3:** Confirmed db.rs create_trip populates both old and new fields correctly, but update_trip does not. Migration SQL sets end_datetime incorrectly for NULL end_time.

**Iteration 4:** No new findings. All issues categorized.

## Recommendation

**Revise plan before implementation.** Key actions:

1. Add fix-up migration to correct `end_datetime = NULL` for trips without end_time
2. Add `start_datetime`/`end_datetime` sync to `update_trip` in db.rs
3. Update plan to reflect Phase 1 is partially done
4. Clarify whether Trip domain struct renames fields or just uses new DB columns internally
5. Remove/update Task 1.1-1.2 since migration already exists
