# Comprehensive Integration Test Suite - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create 48 integration tests (38 new + 10 existing) covering all user-facing functionality as a release gate.

**Architecture:** Tiered test structure with shared fixtures and utilities. DB seeding via Tauri IPC (browser context), UI interactions for feature testing.

**Test Count Breakdown:** [C2 fix]
- Tier 1 (Critical): 19 new tests
- Tier 2 (Core): 12 new tests
- Tier 3 (Edge): 7 new tests
- Existing: 10 tests (in specs/existing/)
- **Total: 48 tests**

**Performance Budget:** [C5 fix]
- Target: Full suite < 15 minutes
- Individual test timeout: 30s (reduced from 60s)
- Tier 1 only: < 5 minutes (for quick pre-release checks)

---

## Task 0: Fix Existing Test Interdependencies [C4, I7 fix]

**Files:**
- Modify: `tests/integration/specs/ev-vehicle.spec.ts`
- Modify: `tests/integration/specs/vehicle-setup.spec.ts`

**Steps:**
1. Review existing tests for interdependencies (e.g., "assumes BEV was created in previous test")
2. Refactor each test to set up its own preconditions in `beforeEach`
3. Remove any comments about test ordering assumptions
4. Ensure each test can run independently and in any order
5. Run existing tests 3x to verify no flakiness from ordering

**Verification:** Run `npm run test:integration` multiple times - all tests pass consistently.

---

## Task 1: Set Up Directory Structure and Fixtures

**Files:**
- Create: `tests/integration/specs/tier1/` (directory)
- Create: `tests/integration/specs/tier2/` (directory)
- Create: `tests/integration/specs/tier3/` (directory)
- Create: `tests/integration/specs/existing/` (directory)
- Create: `tests/integration/screenshots/` (directory) [I8 fix]
- Move: `tests/integration/specs/vehicle-setup.spec.ts` → `tests/integration/specs/existing/`
- Move: `tests/integration/specs/ev-vehicle.spec.ts` → `tests/integration/specs/existing/`
- Create: `tests/integration/fixtures/types.ts` [M5 fix]
- Create: `tests/integration/fixtures/vehicles.ts`
- Create: `tests/integration/fixtures/trips.ts`
- Create: `tests/integration/fixtures/receipts.ts`
- Create: `tests/integration/fixtures/scenarios.ts`
- Modify: `tests/integration/utils/forms.ts` (extend existing helpers) [M1 fix]
- Create: `tests/integration/utils/assertions.ts`
- Create: `tests/integration/utils/language.ts` [L1 fix]

**Steps:**
1. Create tier subdirectories under specs/
2. Create screenshots directory (for failure screenshots)
3. Verify `tests/integration/screenshots/` is in `.gitignore` [M2 fix]
4. Review existing specs for overlap with new tests: [I1 fix]
   - `vehicle-setup.spec.ts`: Keep as-is (covers ICE vehicle flow)
   - `ev-vehicle.spec.ts`: Keep, but Task 7 adds more BEV tests
5. Move existing specs to specs/existing/ (after Task 0 fixes)
6. Create TypeScript interfaces in `fixtures/types.ts` matching `models.rs`: [M5 fix]
   ```typescript
   interface VehicleFixture { name: string; type: 'Ice' | 'Bev' | 'Phev'; tp_rate: number; ... }
   interface TripFixture { date: string; origin: string; destination: string; ... }
   interface ReceiptFixture { file_path: string; status: string; liters?: number; ... }
   ```
7. Create vehicle factory with ICE, BEV, PHEV presets - **include Slovak diacritics**: [C7 fix]
   ```typescript
   vehicles: {
     iceDefault: { name: 'Škoda Octavia', type: 'Ice', tp_rate: 7.0, ... },
     bevDefault: { name: 'Tesla Model 3', type: 'Bev', tp_rate_energy: 15.0, ... },
     phevDefault: { name: 'Toyota RAV4 PHEV', type: 'Phev', ... }
   }
   ```
8. Create trip factory with fuel/energy variants - **use year parameter, Slovak text**: [M6, C7 fix]
   ```typescript
   createTrip(year: number, overrides?: Partial<TripFixture>) => ({
     date: `${year}-01-15`,
     origin: 'Bratislava',
     destination: 'Košice',  // Slovak diacritics
     purpose: 'Služobná cesta',  // Slovak diacritics
     ...overrides
   })
   ```
9. Create receipt factory with complete field definitions: [I4 fix]
   ```typescript
   receipts: {
     processed: {
       file_path: '/mock/receipts/receipt1.jpg',
       status: 'Processed',
       liters: 45.5,
       price_per_liter: 1.65,
       total_price: 75.08,
       date: '2025-01-15',
       source_year: 2025,
       vehicle_id: null  // unassigned
     },
     needsReview: { status: 'NeedsReview', ... }
   }
   ```
10. Create scenario factory with defined specifications: [I9 fix]
    ```typescript
    scenarios: {
      underLimit: {
        description: 'Consumption under TP rate (no warnings)',
        vehicle: { type: 'Ice', tp_rate: 7.0 },
        trips: [
          { km: 500, fuel_liters: 32.5 }  // 6.5 L/100km
        ]
      },
      overLimit: {
        description: 'Consumption over 20% margin (shows warning)',
        vehicle: { type: 'Ice', tp_rate: 7.0 },
        trips: [
          { km: 500, fuel_liters: 45.0 }  // 9.0 L/100km = 128% of TP
        ]
      },
      yearTransition: {
        description: 'Trips spanning two years for carryover testing',
        vehicle: { type: 'Ice', tp_rate: 7.0, initial_fuel: 50 },
        trips: [
          { year: 2024, km: 300, fuel_liters: 20 },
          { year: 2025, km: 400, fuel_liters: 28 }
        ]
      }
    }
    ```
11. Extend existing form helpers in `utils/forms.ts` (don't replace): [M1 fix]
    - Keep existing `createVehicleViaUI()`, `createTripViaUI()`
    - Add: `fillFuelFields()`, `fillEnergyFields()`, `selectYear()`
12. Standardize selector strategy in `utils/assertions.ts`: [M7 fix]
    ```typescript
    // Use data-testid where available, fall back to semantic selectors
    const selectors = {
      saveButton: '[data-testid="save-btn"], button[type="submit"]',
      tripRow: '[data-testid="trip-row"], tr.trip-row',
      // Avoid language-specific text in selectors
    }
    ```
13. Create language helper in `utils/language.ts`: [L1 fix]
    ```typescript
    export async function ensureLanguage(lang: 'sk' | 'en') {
      // Set language via Tauri invoke to ensure consistent UI
      await browser.execute(async (l) => {
        await (window as any).__TAURI__.invoke('set_language', { language: l });
      }, lang);
      await browser.refresh();
      await waitForAppReady();
    }
    ```
14. Create custom assertions (toHaveConsumptionRate, toShowWarning, toShowError) [L2 fix]
    ```typescript
    // Use CSS classes for error detection, not text
    export async function toShowError(selector: string) {
      const element = await $(selector);
      const classes = await element.getAttribute('class');
      return classes.includes('error') || classes.includes('invalid');
    }
    ```

**Verification:** `npm run test:integration` still runs existing tests successfully.

---

## Task 2: Implement DB Seeding Utilities [C1, C3, I10 fix]

**Files:**
- Modify: `tests/integration/utils/db.ts`
- Modify: `tests/integration/wdio.conf.ts`

**Implementation Approach:** [C1 fix]
DB seeding uses **Tauri IPC via browser context** (not direct SQLite). This ensures:
- App handles migrations automatically
- No need for external SQLite library
- Seeding happens after app initialization

**Steps:**
1. Update `beforeTest` hook to handle race conditions: [I10 fix]
   ```typescript
   // In wdio.conf.ts beforeTest hook
   beforeTest: async function() {
     // Ensure app is fully stopped before DB operations
     await browser.pause(500);  // Allow file handles to release

     const dbPath = getTestDbPath();
     const maxRetries = 3;
     for (let i = 0; i < maxRetries; i++) {
       try {
         if (fs.existsSync(dbPath)) {
           fs.unlinkSync(dbPath);
         }
         break;
       } catch (e) {
         if (i === maxRetries - 1) throw e;
         await new Promise(r => setTimeout(r, 200));
       }
     }
   }
   ```

2. Implement seeding via Tauri IPC (runs in browser context): [C1, C3 fix]
   ```typescript
   // In utils/db.ts
   export async function seedVehicle(data: VehicleFixture): Promise<string> {
     // Wait for app to be ready (DB initialized)
     await waitForAppReady();

     // Call Tauri command via browser context
     const vehicleId = await browser.execute(async (vehicleData) => {
       return await (window as any).__TAURI__.invoke('create_vehicle', vehicleData);
     }, data);

     // Refresh UI to show seeded data
     await browser.refresh();
     await waitForAppReady();

     return vehicleId;
   }

   export async function seedTrip(vehicleId: string, data: TripFixture): Promise<string> {
     await waitForAppReady();
     const tripId = await browser.execute(async (vId, tripData) => {
       return await (window as any).__TAURI__.invoke('create_trip', {
         vehicleId: vId,
         ...tripData
       });
     }, vehicleId, data);
     await browser.refresh();
     await waitForAppReady();
     return tripId;
   }

   export async function seedReceipt(data: ReceiptFixture): Promise<string> {
     await waitForAppReady();
     const receiptId = await browser.execute(async (receiptData) => {
       return await (window as any).__TAURI__.invoke('create_receipt', receiptData);
     }, data);
     await browser.refresh();
     await waitForAppReady();
     return receiptId;
   }

   export async function seedScenario(name: keyof typeof scenarios): Promise<void> {
     const scenario = scenarios[name];
     const vehicleId = await seedVehicle(scenario.vehicle);
     for (const trip of scenario.trips) {
       await seedTrip(vehicleId, trip);
     }
   }
   ```

3. Mock Gemini API to prevent credential exposure: [S1 fix]
   ```typescript
   // In wdio.conf.ts onPrepare or beforeSession
   // Ensure tests don't use real Gemini API
   process.env.KNIHA_JAZD_MOCK_GEMINI = 'true';
   ```

4. Use obviously fake test data: [S2 fix]
   - Company name: "Test Company s.r.o."
   - IČO: "12345678"
   - Vehicle names: "Test Škoda", "Test Tesla"

**Verification:** Create a test that seeds data and verifies it appears in UI:
```typescript
it('should seed vehicle and see it in UI', async () => {
  const vehicleId = await seedVehicle(fixtures.vehicles.iceDefault);
  const vehicleName = await $('[data-testid="vehicle-name"]').getText();
  expect(vehicleName).toContain('Škoda');
});
```

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
7. `should reorder trips via keyboard shortcuts` [I2 fix - avoid flaky drag-and-drop]

**Note on test 7:** [I2 fix]
- Primary: Test via keyboard shortcuts (Up/Down arrows with modifier key) if available
- Fallback: Use WebDriver `performActions` with explicit coordinates
- Mark as `@flaky` if unreliable after 3 attempts

**Setup:** Each test calls `ensureLanguage('sk')` in `beforeEach` [L1 fix]

**Verification:** All 7 tests pass. Stats panel updates correctly after each operation.

---

## Task 4: Implement Tier 1 - Consumption & Margin Warnings (2 tests) [M3 fix - removed duplicate]

**Files:**
- Create: `tests/integration/specs/tier1/consumption-warnings.spec.ts`

**Tests:**
1. `should show normal state when consumption under TP rate` (seed: 6.5 L/100km, TP: 7.0)
2. `should show warning when consumption exceeds 20% margin` (seed: 8.5 L/100km, TP: 7.0)

**Note:** Compensation banner test moved to Task 13 (Tier 3) to avoid duplication [M3 fix]

**Verification:** Warning styling appears/disappears based on margin.

---

## Task 5: Implement Tier 1 - Year Handling (2 tests)

**Files:**
- Create: `tests/integration/specs/tier1/year-handling.spec.ts`

**Tests:**
1. `should filter trips by selected year` (seed trips in 2024 and 2025, verify picker works)
2. `should carry over fuel remaining from previous year`

**Verification:** Year picker changes displayed trips. Zostatok carries across years.

---

## Task 6: Implement Tier 1 - Export (2 tests) [I3 fix]

**Files:**
- Create: `tests/integration/specs/tier1/export.spec.ts`

**Tests:**
1. `should open export preview with trip data`
2. `should show correct totals in export footer`

**Implementation Details:** [I3 fix]
```typescript
it('should show correct totals in export footer', async () => {
  // 1. Capture stats panel values BEFORE export
  const totalKm = await $('[data-testid="total-km"]').getText();
  const totalFuel = await $('[data-testid="total-fuel"]').getText();

  // 2. Click export button
  await $('[data-testid="export-btn"]').click();

  // 3. Handle new window/tab
  const handles = await browser.getWindowHandles();
  await browser.switchToWindow(handles[handles.length - 1]);

  // 4. Verify export totals match
  const exportTotalKm = await $('[data-testid="export-total-km"]').getText();
  expect(exportTotalKm).toEqual(totalKm);

  // 5. Close export window and return
  await browser.closeWindow();
  await browser.switchToWindow(handles[0]);
});
```

**Verification:** Export opens in browser. Totals match stats panel values.

---

## Task 7: Implement Tier 1 - BEV Trips (4 tests) [C6 fix - added nullable field test]

**Files:**
- Create: `tests/integration/specs/tier1/bev-trips.spec.ts`

**Tests:**
1. `should create BEV trip with charging session (kWh, cost)`
2. `should calculate energy consumption rate (kWh/100km)`
3. `should track battery SoC remaining after trips`
4. `should create BEV trip without fuel fields (fuel_liters = null)` [C6 fix]

**Verification:** Energy fields visible. Rate calculates. SoC updates in stats. Fuel fields hidden for BEV.

---

## Task 8: Implement Tier 1 - PHEV Trips (5 tests) [I6 fix - expanded]

**Files:**
- Create: `tests/integration/specs/tier1/phev-trips.spec.ts`

**Tests:**
1. `should record both fuel and energy on same trip`
2. `should show both consumption rates in stats`
3. `should handle fuel-only trip on PHEV (energy_kwh = null)` [I6, C6 fix]
4. `should handle energy-only trip on PHEV (fuel_liters = null)` [I6, C6 fix]
5. `should calculate correct margin for PHEV with mixed usage` [I6 fix]

**Verification:** Both fuel and energy columns editable. Both rates shown. Null fields handled correctly.

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

## Task 10: Implement Tier 2 - Receipts Workflow (5 tests) [I11 fix - added vehicle filtering]

**Files:**
- Create: `tests/integration/specs/tier2/receipts.spec.ts`

**Tests:**
1. `should display pre-seeded receipts in list`
2. `should filter receipts by status (all, unassigned, needs review)`
3. `should assign receipt to trip and see "verified" badge`
4. `should delete receipt from list`
5. `should filter receipts by active vehicle` [I11 fix]

**Setup:** Seed mock receipts with complete field data per `fixtures/receipts.ts` [I4 fix]

**Verification:** Receipts appear. Filters work. Assignment updates badge. Delete removes card. Vehicle filter works.

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

## Task 13: Implement Tier 3 - Edge Cases & Validation (9 tests) [M11, M13 fix - expanded]

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

*validation.spec.ts:* [L2, M11, M13 fix]
1. `should prevent invalid odometer (lower than previous trip)` - verify error class appears
2. `should prevent negative distance input` [M11 fix]
3. `should handle leap year date (February 29)` [M13 fix]

**Verification:** Each edge case handled gracefully with appropriate UI feedback. Errors shown via CSS class, not text.

---

## Task 14: Update wdio.conf.ts for Tiered Execution [I5 fix]

**Files:**
- Modify: `tests/integration/wdio.conf.ts`

**Steps:**
1. Update specs glob to include all tier subdirectories
2. Add environment variable support for tier filtering: [I5 fix]
   ```typescript
   // In wdio.conf.ts
   const tier = process.env.TIER;
   export const config: Options.Testrunner = {
     specs: tier
       ? [`./specs/tier${tier}/**/*.spec.ts`, './specs/existing/**/*.spec.ts']
       : ['./specs/**/*.spec.ts'],
     // ...
   }
   ```
3. Reduce test timeout from 60s to 30s [C5 fix]
4. Ensure `maxInstances: 1` (Tauri limitation - no parallelization)
5. Add screenshot directory creation in `onPrepare`: [I8 fix]
   ```typescript
   onPrepare: function() {
     const screenshotsDir = './tests/integration/screenshots';
     if (!fs.existsSync(screenshotsDir)) {
       fs.mkdirSync(screenshotsDir, { recursive: true });
     }
   }
   ```

**Verification:** `npm run test:integration` runs all 48 tests. `TIER=1 npm run test:integration` runs only Tier 1.

---

## Task 15: Integrate with Release Workflow & CI [M4 fix]

**Files:**
- Modify: `.claude/skills/release-skill/SKILL.md`
- Modify: `.github/workflows/test.yml` [M4 fix]
- Modify: `package.json`

**Steps:**
1. Add `npm run test:integration` to release checklist
2. Add `npm run test:integration:tier1` script for faster pre-release check
3. Update CI workflow: [M4 fix]
   ```yaml
   # For PRs: run Tier 1 only (fast feedback)
   - name: Run Tier 1 Integration Tests
     if: github.event_name == 'pull_request'
     run: TIER=1 npm run test:integration

   # For main: run all tiers
   - name: Run All Integration Tests
     if: github.ref == 'refs/heads/main'
     run: npm run test:integration
   ```
4. Document script usage: [M8 fix]
   ```json
   {
     "scripts": {
       "test:integration": "wdio run tests/integration/wdio.conf.ts",
       "test:integration:tier1": "TIER=1 npm run test:integration",
       "test:integration:build": "npm run tauri build -- --debug && npm run test:integration"
     }
   }
   ```
   Note: `test:integration` requires debug build. Use `test:integration:build` if unsure.

**Verification:** `/release` skill mentions running integration tests. CI uses tiered approach.

---

## Task 16: Update Documentation

**Files:**
- Modify: `tests/integration/README.md`
- Modify: `CLAUDE.md` (update test counts)

**Steps:**
1. Document new directory structure
2. Document fixture usage and TypeScript interfaces
3. Document tier system and performance expectations
4. Document seeding approach (Tauri IPC, not direct SQL)
5. Update test count in CLAUDE.md (10 → 48)
6. Document required debug build for integration tests

**Verification:** README explains how to run/write integration tests.

---

## Task 17: Run /changelog

**Files:**
- Modify: `CHANGELOG.md`

**Steps:**
1. Run `/changelog` skill to document the new integration test suite
2. Commit changelog update

**Verification:** CHANGELOG.md has entry under [Unreleased] for integration tests.

---

## Appendix: Test Count Summary

| Tier | Spec File | Tests | Cumulative |
|------|-----------|-------|------------|
| 0 | existing/* | 10 | 10 |
| 1 | trip-management | 7 | 17 |
| 1 | consumption-warnings | 2 | 19 |
| 1 | year-handling | 2 | 21 |
| 1 | export | 2 | 23 |
| 1 | bev-trips | 4 | 27 |
| 1 | phev-trips | 5 | 32 |
| 2 | vehicle-management | 3 | 35 |
| 2 | receipts | 5 | 40 |
| 2 | backup-restore | 3 | 43 |
| 2 | settings | 2 | 45 |
| 3 | empty-states | 2 | 47 |
| 3 | compensation | 2 | 49 |
| 3 | multi-vehicle | 2 | 51 |
| 3 | validation | 3 | 54 |

**Note:** Final count is 54 tests (6 more than originally planned due to edge case additions).

---

## Appendix: Out of Scope (Future Enhancements)

The following were identified during plan review but deferred: [I14, I15, M12 noted]

- **Offline/network testing** - Test behavior with no network [M12]
- **Form recovery testing** - Test state after browser focus loss [I14]
- **Stress testing** - Rapid UI operations [I15]
- **Accessibility testing** - Screen readers, keyboard navigation
- **Upgrade path testing** - Database migration across versions
