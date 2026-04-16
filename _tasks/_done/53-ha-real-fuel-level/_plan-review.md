# Plan Review: HA Real Fuel Level

**Plan:** `_tasks/53-ha-real-fuel-level/02-plan.md`
**Reviewed:** 2026-02-12
**Iteration:** 1

---

## Summary

| Category | Count |
|----------|-------|
| Critical | 2 |
| Important | 3 |
| Minor | 2 |

**Recommendation:** Needs Revisions

---

## Findings

### Critical

- [ ] **C1: ADR-008 violation — percentage-to-liters conversion in frontend (Task 4, Step 2)**

  Task 4 Step 2 computes `haFuelLiters = haFuelCache.value * tankSizeLiters / 100` in the Svelte frontend. The task description (01-task.md) notes "No backend calculation needed — conversion (% to L) is simple display math, not business logic." However, ADR-008 is unambiguous: "All business logic and calculations live in Rust backend only." The percentage-to-liters conversion is a calculation involving vehicle data (`tankSizeLiters`). While it is simple, putting it in the frontend sets a precedent for "simple enough" exceptions and contradicts the project's core architectural rule.

  **Recommendation:** Either (a) move the conversion into the Rust backend (e.g., have `fetch_ha_odo` return the raw value and have a new command or extend the existing one to accept tank_size and return liters), or (b) explicitly record an ADR exception justifying why this specific display-only conversion is acceptable in frontend. Given the project's strictness on ADR-008, option (a) is preferred. Alternatively, the haStore `fetchFuelLevel` method could call a backend command that accepts sensor_id + tank_size and returns the liters directly, keeping the frontend truly display-only.

- [ ] **C2: Task 5 (tests) is ordered LAST — violates TDD workflow**

  CLAUDE.md mandates "MANDATORY WORKFLOW: Write failing test first, write minimal code to pass, refactor, repeat." Task 5 writes tests after implementation is done in Tasks 1-4. The persistence test for `ha_fuel_level_sensor` should be part of Task 1 (DB migration), not a separate final task. Task 1 Step 6 mentions "Add test for persistence" but it reads like an afterthought. The null-by-default test in Task 5 is also testing Task 1's migration work.

  **Recommendation:** Merge Task 5's tests into Task 1. The plan's Task 1 Step 6 already mentions following the `test_vehicle_ha_fillup_sensor_persistence` pattern, so make the tests explicit steps at the START of Task 1 (write failing test, then implement migration + model changes to make it pass).

### Important

- [ ] **I1: haStore architecture — dual caching adds complexity without clear benefit (Task 3)**

  Task 3 proposes a "second cache map" with a separate localStorage key (`kniha-jazd-ha-fuel-cache`). The current `haStore` is designed around a single `cache: Map<string, HaOdoCache>` pattern with one periodic refresh for one sensor. Adding a parallel cache doubles the state management surface. The `startPeriodicRefresh` currently takes `(vehicleId, sensorId)` for a single sensor — extending it to optionally accept a second sensor id creates an awkward API.

  **Recommendation:** Consider one of:
  (a) Extend the existing cache entry type to hold multiple sensor values (e.g., `{ odo?: number, fuelLevel?: number, fetchedAt: number }`) with a single cache map.
  (b) Create a separate `haFuelStore` following the same pattern as `haStore` — simpler, more isolated, no coupling. Given the existing store is small (~160 lines), duplicating it as `haFuelStore` would be cleaner than retrofitting.

  Either approach avoids the awkward "optional parameter" API for `startPeriodicRefresh`.

- [ ] **I2: Missing error state handling in haStore for fuel level (Task 3)**

  Task 4 Step 6 says "Handle error state: show (HA: chyba) in red if sensor configured but fetch fails." However, the current `haStore` has a single `error: string | null` field shared across all operations. If the ODO fetch succeeds but the fuel level fetch fails (or vice versa), the single error state cannot represent both independently.

  **Recommendation:** If extending the existing `haStore`, add separate error tracking for fuel level (e.g., `fuelError: string | null`). If creating a separate `haFuelStore` (per I1), this is naturally solved — each store has its own error state.

- [ ] **I3: Missing i18n keys for fuel level sensor in VehicleModal (Task 2)**

  Task 2 Step 2 lists keys: `fuelLevelSensorLabel`, `fuelLevelSensorPlaceholder`, `fuelLevelSensorHint`, `realFuel`. These need to be added under the `homeAssistant` namespace in both `sk/index.ts` and `en/index.ts`. The plan should specify the exact key paths (e.g., `homeAssistant.fuelLevelSensorLabel`) and provide actual Slovak translations, following the existing pattern:

  ```
  fillupSensorLabel: 'Senzor návrhu tankovania',
  fillupSensorPlaceholder: 'sensor.kniha_jazd_fillup',
  ```

  The `realFuel` key also needs context — is it "Reálna hladina paliva" analogous to "Reálne ODO"?

  **Recommendation:** Specify exact translations in the plan to avoid implementer guessing. Also confirm the namespace path (`homeAssistant.*`).

### Minor

- [ ] **M1: Line number references will drift (Tasks 2, 4)**

  Task 2 references "lines 150-159" in VehicleModal.svelte and Task 4 references "line 243" in +page.svelte. These are fragile — any prior change will invalidate them. After the Task 1 migration changes are made and other files are touched, these line numbers may already be wrong.

  **Recommendation:** Use descriptive anchors instead (e.g., "after the `ha-fillup-sensor` input group" or "on the zostatok `.info-item` div"). This is minor because an implementer can find the right location regardless.

- [ ] **M2: Task ordering — Task 2 (UI types + modal) before Task 3 (store) means UI can't be tested end-to-end**

  Task 2 adds the sensor config field to VehicleModal, but without Task 3 (store) and Task 4 (display), you can't verify the full flow. This is acceptable for incremental development but worth noting — Task 2's verification step ("Vehicle modal shows third HA sensor field, saves/loads correctly") only tests persistence, not the actual fuel level display.

  **Recommendation:** No change needed, just note that real verification of the feature happens only after Task 4.

---

## Checklist

| Check | Status |
|-------|--------|
| Tasks have file paths | Pass |
| Tasks have verification steps | Pass |
| Task ordering is correct | Warn (tests last, should be first per TDD) |
| No scope creep | Pass |
| ADR-008 compliance | Fail (frontend calculation in Task 4) |
| i18n keys specified | Partial (keys named but translations missing) |
| Existing patterns followed | Pass (migration, test, VehicleModal patterns) |
| Test coverage plan | Fail (tests deferred to Task 5 instead of TDD) |
