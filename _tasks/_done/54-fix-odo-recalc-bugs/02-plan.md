# Fix Odometer Recalculation Bugs — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix stale-data recalculation in `handleUpdate` and remove circular sort dependency

**Architecture:** Unify all recalculation paths to use `recalculateAllOdo` (refresh-first pattern). Fix backend sorts for consistency.

---

## Task 1: Write backend tests for sort tiebreaker

**Files:**
- Modify: `src-tauri/src/commands/commands_tests.rs`

**Steps:**
1. Add test `test_calculate_odometer_start_same_datetime_uses_sort_order` — two trips with identical `start_datetime`, verify they're ordered by `sort_order` not `odometer`
2. Add test `test_year_start_odometer_same_day_uses_sort_order` — two trips on same day with different sort_orders, verify the one with lowest sort_order (newest) is returned as year-end value

**Verification:** `cd src-tauri && cargo test` — new tests should FAIL (red phase of TDD)

---

## Task 2: Fix backend sort tiebreakers

**Files:**
- Modify: `src-tauri/src/commands/mod.rs` (line ~124-134)
- Modify: `src-tauri/src/commands/statistics.rs` (line ~338-347)

**Steps:**
1. In `calculate_odometer_start` (`mod.rs`): change final `.then_with()` from `a.odometer.partial_cmp(&b.odometer)` to `b.sort_order.cmp(&a.sort_order)` (higher sort_order = older = first chronologically)
2. In `get_year_start_odometer` (`statistics.rs`): add `.then_with(|| a.start_datetime.cmp(&b.start_datetime))` before the tiebreaker, then change tiebreaker from `a.odometer.partial_cmp(&b.odometer)` to `b.sort_order.cmp(&a.sort_order)`

**Verification:** `cd src-tauri && cargo test` — Task 1 tests should now PASS, all existing tests still pass

---

## Task 3: Fix `handleUpdate` and remove `recalculateNewerTripsOdo`

**Files:**
- Modify: `src/lib/components/TripGrid.svelte`

**Steps:**
1. In `handleUpdate` (line ~253-287): replace the recalculation sequence:
   ```
   // OLD (buggy):
   await recalculateNewerTripsOdo(trip.id, tripData.odometer!);
   onTripsChanged();

   // NEW (same pattern as handleSaveNew):
   await onTripsChanged();
   await tick();
   await recalculateAllOdo();
   ```
2. Remove the entire `recalculateNewerTripsOdo` function (lines ~289-314) — it's now dead code
3. Ensure `tick` import exists (already imported at line 8)

**Verification:**
- App builds: `npm run tauri dev`
- Manual test: edit a trip's date → verify odometer chain stays correct
- Manual test: add trip between existing trips → verify all odometers correct

---

## Task 4: Run full test suite and commit

**Steps:**
1. Run `cd src-tauri && cargo test` — all backend tests pass
2. Verify app launches and trip grid displays correctly
3. Commit with descriptive message

**Verification:** `npm run test:backend` passes
