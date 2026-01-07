# Plan Review

**Target:** `_tasks/26-integration-tests/02-plan.md`
**Started:** 2026-01-07
**Status:** In Progress
**Focus:** Completeness, feasibility, clarity

## Iteration 1

### New Findings

#### Critical (Blockers)

1. **[C1] DB Seeding Not Implemented - Plan Assumes It Will Work**
   - **Location:** Task 2 (DB Seeding Utilities)
   - **Issue:** The plan assumes `seedVehicle()`, `seedTrip()`, `seedReceipt()` can be implemented, but `tests/integration/utils/db.ts` currently has only placeholder stubs. The plan doesn't address:
     - Tauri commands cannot be invoked from test utilities directly - requires WebDriver IPC
     - Direct SQLite access requires importing a library, finding DB path, running migrations
     - The DB is created by the app on first launch, not by tests
   - **Suggested Resolution:** Expand Task 2 to specify whether to use `better-sqlite3` npm package or invoke Tauri commands via browser context. Add steps to verify app has created DB before seeding. Consider hybrid approach: seed via `invoke()` from browser context in `beforeEach`.

2. **[C2] Test Count Mismatch - Only 38 Tests Specified, Not 48**
   - **Location:** Tasks 3-13
   - **Issue:** Counting all tests: 7+3+2+2+3+2+3+4+3+2+7 = **38 tests**, not 48 as claimed. This is a 21% shortfall.
   - **Suggested Resolution:** Clarify whether the 10 existing tests are included in the 48 count. If so, state this in Task 1. If not, add 10 more tests.

3. **[C3] Missing Handling of Database Cleanup Between Tests**
   - **Location:** Task 2, wdio.conf.ts
   - **Issue:** The `beforeTest` hook deletes the DB file before each test. This means DB seeding must run AFTER the app starts. The plan's `seedScenario()` doesn't account for app restart timing.
   - **Suggested Resolution:** Clarify whether seeding happens via Tauri IPC after app start (requires browser context) or via direct SQLite with migrations.

#### Important (Should Be Fixed)

4. **[I1] Existing Test Migration Strategy Unclear**
   - **Location:** Task 1
   - **Issue:** Moving existing tests to `specs/existing/` but some overlap with new tests (e.g., `ev-vehicle.spec.ts` vs Task 7 BEV tests). No guidance on keeping, deleting, or merging duplicates.
   - **Suggested Resolution:** Add sub-step to review existing tests for overlap, mark tests to keep vs. merge vs. delete.

5. **[I2] Drag-and-Drop Test May Be Flaky**
   - **Location:** Task 3, test 7
   - **Issue:** Drag-and-drop is notoriously difficult to test reliably with WebDriver. No guidance on actions to use or fallback strategy.
   - **Suggested Resolution:** Verify drag-and-drop exists in app, consider keyboard shortcuts, add flakiness note.

6. **[I3] Export Test Verification Is Vague**
   - **Location:** Task 6
   - **Issue:** "Export opens in browser. Totals match stats panel values." doesn't explain how to handle new window/tab, capture values before export, or verify PDF content.
   - **Suggested Resolution:** Add specific steps for value capture, window switching, and selector verification.

7. **[I4] Receipt Mock Strategy Not Specified**
   - **Location:** Task 10
   - **Issue:** "Seed mock receipts" doesn't explain what fields to populate, whether mock images are needed, or how to handle `source_year`.
   - **Suggested Resolution:** Define mock receipt data in `fixtures/receipts.ts` matching `Receipt` struct from `models.rs`.

8. **[I5] Tier Filtering Implementation Not Specified**
   - **Location:** Task 14
   - **Issue:** Plan mentions `TIER=1` env var but no implementation details.
   - **Suggested Resolution:** Add example: `specs: tier ? [\`./specs/tier${tier}/**/*.spec.ts\`] : ['./specs/**/*.spec.ts']`

9. **[I6] PHEV Tests Incomplete**
   - **Location:** Task 8
   - **Issue:** Only 2 PHEV tests. Missing: fuel-only trips, energy-only trips, mixed usage margin calculations.
   - **Suggested Resolution:** Add 2-3 more PHEV tests for different usage modes.

#### Minor (Nice-to-Have)

10. **[M1] Form Helpers Already Exist**
    - **Location:** Task 1
    - **Issue:** `fixtures/seed-data.ts` already has `createVehicleViaUI()` and `createTripViaUI()`. Should these be extended or replaced?
    - **Suggested Resolution:** Review existing helpers and document what's new vs. extended.

11. **[M2] Screenshots Directory Not in .gitignore Check**
    - **Location:** Task 1
    - **Issue:** No verification that `tests/integration/screenshots/` is gitignored.
    - **Suggested Resolution:** Add verification step.

12. **[M3] Compensation Banner Test Duplicated**
    - **Location:** Tasks 4 and 13
    - **Issue:** Task 4 test 3 and Task 13 compensation.spec.ts test 1 appear to be the same test.
    - **Suggested Resolution:** Remove duplicate or clarify difference.

13. **[M4] No Mention of CI Update**
    - **Location:** Task 15
    - **Issue:** Plan doesn't mention updating `.github/workflows/test.yml` for tiered approach.
    - **Suggested Resolution:** Consider if CI should run Tier 1 only for PRs (fast) and all tiers for main.

14. **[M5] Fixture Type Definitions Missing**
    - **Location:** Task 1
    - **Issue:** No TypeScript interfaces specified for fixtures.
    - **Suggested Resolution:** Reference `models.rs` and create matching TypeScript interfaces.

### Completeness Check

| Requirement (01-task.md) | Addressed? | Notes |
|---|---|---|
| All 3 routes (/, /settings, /doklady) | ⚠️ Partial | No explicit /doklady navigation test |
| All vehicle types (ICE, BEV, PHEV) | ✅ Yes | Tasks 7, 8, 9 |
| CRUD operations | ⚠️ Partial | Delete exists, no explicit "update" for trips |
| Consumption calculation | ✅ Yes | Task 4 |
| Margin warnings | ✅ Yes | Task 4 |
| Compensation suggestions | ✅ Yes | Task 13 |
| Year handling | ✅ Yes | Task 5 |
| Deterministic tests | ⚠️ Implicit | No flakiness mitigation strategy |
| Mock external services | ⚠️ Partial | Gemini mentioned but not how |
| Tiered execution | ✅ Yes | Task 14 |
| Release integration | ✅ Yes | Task 15 |

### Coverage Assessment

**Areas Reviewed:**
- Requirements document (01-task.md)
- Implementation plan (02-plan.md)
- Existing test structure (2 spec files in specs/)
- Test utilities (db.ts, app.ts)
- Fixtures (seed-data.ts)
- Configuration (wdio.conf.ts)
- Data models (models.rs)
- Release workflow (SKILL.md)
- CI pipeline (test.yml)

**Areas Remaining:**
- UI selectors in Svelte components (verification during implementation)
- Compensation suggestion behavior details
- Export preview component structure

---

## Iteration 2

### New Findings

#### Critical (Blockers)

15. **[C4] Test Interdependency Between Spec Files Causes Fragility**
    - **Location:** `ev-vehicle.spec.ts` line 143-155
    - **Issue:** Test "should show BEV badge in vehicle list" has comment "assumes BEV was created in previous test". This creates hidden interdependencies. With `beforeTest` hook cleaning DB, dependent tests will fail for wrong reasons if test order changes.
    - **Suggested Resolution:** Each test must set up its own preconditions. Refactor existing tests before migration.

16. **[C5] No Timeout Budget or Performance Estimation**
    - **Location:** wdio.conf.ts, Task 14
    - **Issue:** Current timeout: 60s per test. 48 tests × 60s = 48 minutes worst case. `maxInstances: 1` means no parallelization. CI has no explicit timeout (uses GitHub's 6-hour default).
    - **Suggested Resolution:** Add expected runtime budget to plan. Consider reducing individual test timeout. Document acceptable total suite duration.

#### Important (Should Be Fixed)

17. **[I7] Existing Tests Will Break When Moved**
    - **Location:** Task 1
    - **Issue:** Moving `ev-vehicle.spec.ts` to `specs/existing/` without fixing interdependencies (C4) will cause immediate failures.
    - **Suggested Resolution:** Fix existing test interdependencies in Task 1 before moving.

18. **[I8] Screenshots Directory Not Created Automatically**
    - **Location:** `utils/app.ts` takeScreenshot function
    - **Issue:** `takeScreenshot()` writes to `./tests/integration/screenshots/` but doesn't create the directory. No `fs.mkdirSync` with `recursive: true`. Will fail on first screenshot attempt.
    - **Suggested Resolution:** Add directory creation to Task 1 setup or fix in app.ts utility.

19. **[I9] Seed Scenario Factory Has No Definition**
    - **Location:** Tasks 1 and 2
    - **Issue:** Task 1 mentions "Create scenario factory for complex setups (underLimit, overLimit, yearTransition)" but no definition of what each scenario contains (vehicle type, trip count, consumption values).
    - **Suggested Resolution:** Define scenario specifications in plan - what data each scenario seeds.

20. **[I10] Race Condition in beforeTest Hook**
    - **Location:** `wdio.conf.ts:169-176`
    - **Issue:** `beforeTest` hook deletes DB file, but Tauri app may still be running from previous test. If app holds file lock, `unlinkSync` will throw on Windows. No retry logic or proper app shutdown synchronization.
    - **Suggested Resolution:** Add proper app shutdown verification before DB deletion, or add retry logic with delay.

21. **[I11] No Test for Receipt Vehicle Filtering**
    - **Location:** Task 10, `_tasks/27-receipt-vehicle-filtering/`
    - **Issue:** A planned feature (receipt vehicle filtering) exists in task folder but plan doesn't mention testing it. Receipts should be filtered by active vehicle.
    - **Suggested Resolution:** Add test for receipt vehicle filtering if feature is implemented, or note as future work.

#### Minor (Nice-to-Have)

22. **[M6] Hardcoded Test Data Dates in 2024**
    - **Location:** `seed-data.ts` line 25
    - **Issue:** Fixture uses `date: '2024-01-15'`. Year handling tests (Task 5) need multiple years but fixtures are single-year.
    - **Suggested Resolution:** Add year parameter to fixtures or create year-aware data generators.

23. **[M7] Form Helpers Use Inconsistent Selector Strategies**
    - **Location:** `seed-data.ts:48`, `vehicle-setup.spec.ts:56`
    - **Issue:** Different selector patterns used inconsistently (`input[name="name"], #name` vs `input[name="name"], #name, input[placeholder*="názov"]`). Tests will break differently if selectors change.
    - **Suggested Resolution:** Standardize selector strategy in Task 1 utils/assertions.ts.

24. **[M8] CI May Run Wrong Script**
    - **Location:** `.github/workflows/test.yml`, `package.json:21`
    - **Issue:** CI runs `npm run tauri build -- --debug` then tests, but `npm run test:integration:build` also builds. If someone runs `test:integration` directly (not `test:integration:build`), it will fail without debug build.
    - **Suggested Resolution:** Document required build step in README, or make test script check for debug build.

### Refined Analysis

- **C1 is worse than noted:** The seeding functions don't just fail - they return fake IDs without creating data. Tests calling `seedVehicle()` pass at call site but fail later when data doesn't exist. This is a data integrity landmine.
- **C3 clarified:** The race condition (I10) is the root cause of potential cleanup issues.

### Coverage Assessment

**Areas Newly Reviewed:**
- Test interdependencies in existing specs
- Timeout and performance characteristics
- File system operations (screenshots, DB cleanup)
- CI script dependencies

**Areas Remaining:**
- UI selectors (implementation-time verification)
- Accessibility testing (not in scope per plan)

---

## Iteration 3

### New Findings

#### Security (Should Be Critical)

25. **[S1] Gemini API Key Exposed in Local Settings**
    - **Location:** `src-tauri/src/settings.rs`, test infrastructure
    - **Issue:** Tests load `local.settings.json` which may contain `gemini_api_key`. Plan doesn't specify how to mock Gemini API calls or prevent credential exposure in test logs/CI.
    - **Suggested Resolution:** Add step to mock Gemini responses. Use `.env.test` file. Never commit real credentials.

26. **[S2] Database May Contain Sensitive Sample Data**
    - **Location:** Integration test fixtures
    - **Issue:** Tests create entries with company names, IČO, vehicle data. If CI is public, sample data could be visible.
    - **Suggested Resolution:** Ensure test data uses obviously fake values (e.g., "Test Company s.r.o.", IČO "12345678").

#### Localization (Important)

27. **[L1] Tests Hardcoded to Slovak UI - Will Break If Language Changes**
    - **Location:** All test specs and fixtures
    - **Evidence:** `clickButton('Uložiť')` in seed-data.ts, `button*=Uložiť` in vehicle-setup.spec.ts
    - **Issue:** Plan assumes Slovak UI. If app loads in English, all Slovak text selectors fail.
    - **Suggested Resolution:** Add language handling strategy: either set language to Slovak in `beforeEach`, or use `data-testid` attributes instead of text selectors.

28. **[L2] Error Message Assertions Not Specified**
    - **Location:** Task 13 (validation tests)
    - **Issue:** "should prevent invalid odometer" test doesn't specify expected error message or how to verify in Slovak/English.
    - **Suggested Resolution:** Use data attributes or error class for assertions instead of text content.

#### Browser/WebView (Minor)

29. **[B1] Tauri WebView2 Version Not Documented**
    - **Location:** `tauri.conf.json`
    - **Issue:** CSP is set to `null`. Plan doesn't mention WebView2 version compatibility.
    - **Suggested Resolution:** Add note about WebView2 baseline. Document that CSP is disabled.

30. **[B2] Edge Driver Setup Complexity Not Mentioned**
    - **Location:** `wdio.conf.ts:106-127`, `.github/workflows/test.yml:84-104`
    - **Issue:** Test setup requires `msedgedriver.exe` with complex cross-platform fallbacks. Not mentioned in plan.
    - **Suggested Resolution:** Reference driver setup in Task 1 or documentation task.

### Reclassification Notes

- **C4** (test interdependency) → reclassified as **Important** - can be addressed during implementation
- **C5** (timeout budget) → remains **Critical** for planning, but blocking only if tests exceed CI limits

### Coverage Assessment

**Review is comprehensive.** All major areas examined:
- ✅ Requirements completeness
- ✅ Implementation feasibility
- ✅ Existing code compatibility
- ✅ CI/CD integration
- ✅ Security considerations
- ✅ Localization handling
- ✅ Performance expectations

---

## Review Summary

**Status:** Ready for User Review
**Iterations:** 3
**Total Findings:** 5 Critical, 11 Important, 8 Minor + 6 New (Security/Localization/Browser)

### All Findings (Consolidated)

#### Critical (5)

1. [ ] **[C1] DB Seeding Returns Fake IDs** - Task 2: Seeding functions are stubs that return fake IDs without creating data. Must implement via Tauri IPC or direct SQLite.

2. [ ] **[C2] Test Count Mismatch** - Tasks 3-13: Only 38 tests specified, not 48. Clarify if 10 existing tests are included.

3. [ ] **[C3] Database Cleanup Timing** - Task 2: `beforeTest` deletes DB, but seeding must happen AFTER app starts. Timing not addressed.

4. [ ] **[C4] Test Interdependency** - Existing tests: `ev-vehicle.spec.ts` assumes prior test state. Must fix before migration.

5. [ ] **[C5] No Timeout Budget** - Task 14: 48 tests × 60s = 48 min worst case. No parallelization possible. Add performance expectations.

#### Important (13)

6. [ ] **[I1] Migration Strategy Unclear** - Task 1: No guidance on handling duplicate tests when moving existing specs.

7. [ ] **[I2] Drag-and-Drop Flaky** - Task 3: Test 7 may be unreliable. Add fallback strategy.

8. [ ] **[I3] Export Verification Vague** - Task 6: No steps for window switching or value capture.

9. [ ] **[I4] Receipt Mock Not Specified** - Task 10: Missing field definitions for mock receipts.

10. [ ] **[I5] Tier Filtering Unspecified** - Task 14: No implementation example for `TIER=1` env var.

11. [ ] **[I6] PHEV Tests Incomplete** - Task 8: Only 2 tests. Missing fuel-only, energy-only modes.

12. [ ] **[I7] Existing Tests Will Break** - Task 1: Moving tests without fixing interdependencies causes failures.

13. [ ] **[I8] Screenshots Dir Not Created** - Task 1: `takeScreenshot()` fails if directory doesn't exist.

14. [ ] **[I9] Scenario Factory Undefined** - Tasks 1-2: No specification of what underLimit/overLimit scenarios contain.

15. [ ] **[I10] Race Condition in Cleanup** - wdio.conf: DB deletion while app holds file lock fails on Windows.

16. [ ] **[I11] Receipt Vehicle Filtering** - Task 10: Recent feature (`_tasks/27-`) not covered in tests.

17. [ ] **[L1] Slovak UI Hardcoded** - All specs: Selectors use Slovak text. Will fail if app loads in English.

18. [ ] **[L2] Error Messages Not Specified** - Task 13: Validation tests don't specify expected error text.

#### Minor (10)

19. [ ] **[M1] Form Helpers Exist** - Task 1: `seed-data.ts` already has helpers. Document extend vs replace.

20. [ ] **[M2] Screenshots .gitignore** - Task 1: Verify directory is gitignored.

21. [ ] **[M3] Duplicate Compensation Test** - Tasks 4 & 13: Same test appears twice.

22. [ ] **[M4] CI Update Missing** - Task 15: No mention of updating test.yml for tiers.

23. [ ] **[M5] Missing TypeScript Interfaces** - Task 1: No fixture types defined.

24. [ ] **[M6] Hardcoded 2024 Dates** - Fixtures: `date: '2024-01-15'` may break year tests.

25. [ ] **[M7] Inconsistent Selectors** - Fixtures: Different selector patterns across files.

26. [ ] **[M8] CI Script Confusion** - Scripts: `test:integration` vs `test:integration:build` unclear.

27. [ ] **[S1] API Key Exposure Risk** - Settings: Tests may load real Gemini API key.

28. [ ] **[S2] Sensitive Test Data** - Fixtures: Use obviously fake company/IČO values.

### Recommendation

**Needs Revisions** - Plan is architecturally sound but has critical gaps in:

1. **DB Seeding Strategy** (C1, C3) - Must specify implementation approach before Tasks 3-13
2. **Test Count Clarification** (C2) - Resolve 38 vs 48 discrepancy
3. **Language Handling** (L1) - Add strategy to prevent locale-dependent failures
4. **Existing Test Fixes** (C4, I7) - Address interdependencies before migration

**After addressing Critical + Important issues, plan is ready for implementation.**
