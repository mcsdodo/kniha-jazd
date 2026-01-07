# Comprehensive Integration Test Suite - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create 48 integration tests covering all user-facing functionality as a release gate.

**Architecture:** Tiered test structure with shared fixtures and utilities. DB seeding for preconditions, UI interactions for feature testing.

---

## Task 1: Set Up Directory Structure and Fixtures

**Files:**
- Create: `tests/integration/specs/tier1/` (directory)
- Create: `tests/integration/specs/tier2/` (directory)
- Create: `tests/integration/specs/tier3/` (directory)
- Move: `tests/integration/specs/vehicle-setup.spec.ts` → `tests/integration/specs/existing/`
- Move: `tests/integration/specs/ev-vehicle.spec.ts` → `tests/integration/specs/existing/`
- Create: `tests/integration/fixtures/vehicles.ts`
- Create: `tests/integration/fixtures/trips.ts`
- Create: `tests/integration/fixtures/receipts.ts`
- Create: `tests/integration/fixtures/scenarios.ts`
- Create: `tests/integration/utils/forms.ts`
- Create: `tests/integration/utils/assertions.ts`

**Steps:**
1. Create tier subdirectories under specs/
2. Move existing specs to specs/existing/
3. Create vehicle factory with ICE, BEV, PHEV presets
4. Create trip factory with fuel/energy variants
5. Create receipt factory with mock processed data
6. Create scenario factory for complex setups (underLimit, overLimit, yearTransition)
7. Create form helpers (createTripViaUI, fillFuelFields, selectYear)
8. Create custom assertions (toHaveConsumptionRate, toShowWarning)

**Verification:** `npm run test:integration` still runs existing tests successfully.

---

## Task 2: Implement DB Seeding Utilities

**Files:**
- Modify: `tests/integration/utils/db.ts`

**Steps:**
1. Add `seedVehicle(data)` - creates vehicle via Tauri command or direct SQL
2. Add `seedTrip(vehicleId, data)` - creates trip with optional fuel/energy
3. Add `seedReceipt(data)` - creates pre-processed receipt
4. Add `seedScenario(name)` - creates full scenario from fixtures
5. Add `clearDatabase()` - wipe for fresh state
6. Ensure functions wait for DB operations to complete

**Verification:** Create a test that seeds data and verifies it appears in UI.

---

## Task 3: Implement Tier 1 - Trip Management (7 tests)

**Files:**
- Create: `tests/integration/specs/tier1/trip-management.spec.ts`

**Tests:**
1. `should create a new trip with basic fields`
2. `should create a trip with full tank refill and see consumption calculated`
3. `should create a trip with partial refill (no consumption until next full)`
4. `should edit an existing trip and see stats update`
5. `should delete a trip and see stats update`
6. `should insert a trip between existing trips and see stats recalculate`
7. `should reorder trips via drag-and-drop`

**Verification:** All 7 tests pass. Stats panel updates correctly after each operation.

---

## Task 4: Implement Tier 1 - Consumption & Margin Warnings (3 tests)

**Files:**
- Create: `tests/integration/specs/tier1/consumption-warnings.spec.ts`

**Tests:**
1. `should show normal state when consumption under TP rate` (seed: 6.5 L/100km, TP: 7.0)
2. `should show warning when consumption exceeds 20% margin` (seed: 8.5 L/100km, TP: 7.0)
3. `should show compensation banner when over limit`

**Verification:** Warning styling appears/disappears based on margin. Banner shows suggested km.

---

## Task 5: Implement Tier 1 - Year Handling (2 tests)

**Files:**
- Create: `tests/integration/specs/tier1/year-handling.spec.ts`

**Tests:**
1. `should filter trips by selected year` (seed trips in 2024 and 2025, verify picker works)
2. `should carry over fuel remaining from previous year`

**Verification:** Year picker changes displayed trips. Zostatok carries across years.

---

## Task 6: Implement Tier 1 - Export (2 tests)

**Files:**
- Create: `tests/integration/specs/tier1/export.spec.ts`

**Tests:**
1. `should open export preview with trip data`
2. `should show correct totals in export footer`

**Verification:** Export opens in browser. Totals match stats panel values.

---

## Task 7: Implement Tier 1 - BEV Trips (3 tests)

**Files:**
- Create: `tests/integration/specs/tier1/bev-trips.spec.ts`

**Tests:**
1. `should create BEV trip with charging session (kWh, cost)`
2. `should calculate energy consumption rate (kWh/100km)`
3. `should track battery SoC remaining after trips`

**Verification:** Energy fields visible. Rate calculates. SoC updates in stats.

---

## Task 8: Implement Tier 1 - PHEV Trips (2 tests)

**Files:**
- Create: `tests/integration/specs/tier1/phev-trips.spec.ts`

**Tests:**
1. `should record both fuel and energy on same trip`
2. `should show both consumption rates in stats`

**Verification:** Both fuel and energy columns editable. Both rates shown.

---

## Task 9: Implement Tier 2 - Vehicle Management (3 tests)

**Files:**
- Create: `tests/integration/specs/tier2/vehicle-management.spec.ts`

**Tests:**
1. `should create PHEV vehicle with both tank and battery fields`
2. `should edit existing vehicle and see changes reflected`
3. `should delete vehicle and redirect to empty state`

**Verification:** PHEV shows both field sets. Edits persist. Delete clears active vehicle.

---

## Task 10: Implement Tier 2 - Receipts Workflow (4 tests)

**Files:**
- Create: `tests/integration/specs/tier2/receipts.spec.ts`

**Tests:**
1. `should display pre-seeded receipts in list`
2. `should filter receipts by status (all, unassigned, needs review)`
3. `should assign receipt to trip and see "verified" badge`
4. `should delete receipt from list`

**Setup:** Seed mock receipts with status Processed/NeedsReview (no OCR).

**Verification:** Receipts appear. Filters work. Assignment updates badge. Delete removes card.

---

## Task 11: Implement Tier 2 - Backup & Restore (3 tests)

**Files:**
- Create: `tests/integration/specs/tier2/backup-restore.spec.ts`

**Tests:**
1. `should create backup and see it in list`
2. `should restore backup and see data reloaded`
3. `should delete backup from list`

**Verification:** Backup appears with timestamp. Restore shows confirmation. Delete removes entry.

---

## Task 12: Implement Tier 2 - Settings (2 tests)

**Files:**
- Create: `tests/integration/specs/tier2/settings.spec.ts`

**Tests:**
1. `should save company name and IČO`
2. `should switch language and see UI update`

**Verification:** Settings persist after page reload. Language toggle changes visible text.

---

## Task 13: Implement Tier 3 - Edge Cases (7 tests)

**Files:**
- Create: `tests/integration/specs/tier3/empty-states.spec.ts`
- Create: `tests/integration/specs/tier3/compensation.spec.ts`
- Create: `tests/integration/specs/tier3/multi-vehicle.spec.ts`
- Create: `tests/integration/specs/tier3/validation.spec.ts`

**Tests:**

*empty-states.spec.ts:*
1. `should show "no vehicle" prompt on fresh install`
2. `should show "no trips" state for new vehicle`

*compensation.spec.ts:*
1. `should show compensation banner when over 20% margin`
2. `should add suggested buffer trip and see margin decrease`

*multi-vehicle.spec.ts:*
1. `should switch active vehicle and see different trips`
2. `should maintain separate stats per vehicle`

*validation.spec.ts:*
1. `should prevent invalid odometer (lower than previous trip)`

**Verification:** Each edge case handled gracefully with appropriate UI feedback.

---

## Task 14: Update wdio.conf.ts for Tiered Execution

**Files:**
- Modify: `tests/integration/wdio.conf.ts`

**Steps:**
1. Update specs glob to include all tier subdirectories
2. Add environment variable support for tier filtering: `TIER=1 npm run test:integration`
3. Configure parallel execution if beneficial

**Verification:** `npm run test:integration` runs all 48 tests. `TIER=1` runs only Tier 1.

---

## Task 15: Integrate with Release Workflow

**Files:**
- Modify: `.claude/skills/release-skill/SKILL.md` (or equivalent)
- Modify: `package.json` (add script if needed)

**Steps:**
1. Add `npm run test:integration` to release checklist
2. Consider adding `npm run test:integration:tier1` for faster pre-release check
3. Document in README that integration tests are required for release

**Verification:** `/release` skill mentions running integration tests.

---

## Task 16: Update Documentation

**Files:**
- Modify: `tests/integration/README.md`
- Modify: `CLAUDE.md` (update test counts)

**Steps:**
1. Document new directory structure
2. Document fixture usage
3. Document tier system
4. Update test count in CLAUDE.md (10 → 48)

**Verification:** README explains how to run/write integration tests.

---

## Task 17: Run /changelog

**Files:**
- Modify: `CHANGELOG.md`

**Steps:**
1. Run `/changelog` skill to document the new integration test suite
2. Commit changelog update

**Verification:** CHANGELOG.md has entry under [Unreleased] for integration tests.
