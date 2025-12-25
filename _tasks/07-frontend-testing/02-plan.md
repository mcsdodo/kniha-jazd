# Testing Strategy: Backend Analysis & Frontend Test Plan

**Date:** 2025-12-25
**Status:** In Progress

## 1. Backend Tests Overview (76 tests)

### 1.1 calculations.rs (41 tests) - CORE BUSINESS LOGIC

| Test Group | Count | What It Tests | Use-Case Fit |
|------------|-------|---------------|--------------|
| Consumption Rate | 3 | `liters / km * 100` formula | Direct: l/100km shown in grid |
| Spotreba (Fuel Used) | 3 | `distance * rate / 100` | Direct: deducted from zostatok |
| Zostatok (Remaining) | 4 | Fuel tracking with tank cap | Direct: shown in grid column |
| Margin Percentage | 5 | `(rate - tp) / tp * 100` | Direct: legal compliance check |
| Buffer KM | 4 | KM needed to reach target margin | Direct: compensation suggestions |
| Excel Verification | 22 | Real-world data from Excel | Critical: validates all calculations match reference |

**Key Test: `test_excel_integration_full_flow`** (lines 419-568)
- Simulates 7 trips with 2 fill-ups
- Validates entire calculation chain matches Excel reference
- This is the "golden standard" test

### 1.2 suggestions.rs (11 tests) - COMPENSATION LOGIC

| Test Group | Count | What It Tests |
|------------|-------|---------------|
| Random Target Margin | 2 | 16-19% target range generation |
| Route Matching | 5 | Find route within ±10% of target |
| Suggestion Building | 4 | Create buffer trip when no match |

### 1.3 db.rs (24 tests) - DATA PERSISTENCE

| Test Group | Count | What It Tests |
|------------|-------|---------------|
| Database Init | 1 | Tables created correctly |
| Vehicle CRUD | 6 | Create/read/update/delete vehicles |
| Trip CRUD | 8 | Trip operations + sort_order |
| Route CRUD | 9 | Route matching + usage tracking |

**⚠️ Issue:** 2 test helpers missing `full_tank` field - tests currently fail

---

## 2. Frontend vs Backend: Calculation Duplication

**Critical Finding:** Frontend duplicates Rust calculations:

| Calculation | Backend (Rust) | Frontend (TS) |
|-------------|----------------|---------------|
| Consumption rate | `calculate_consumption_rate()` | `calculateConsumptionRates()` |
| Fuel remaining | `calculate_zostatok()` | `calculateFuelRemaining()` |
| Margin check | `calculate_margin_percent()` | `calculateConsumptionWarnings()` |

**Risk:** Logic divergence between frontend and backend. Frontend is untested.

---

## 3. Frontend Logic Requiring Tests

### 3.1 `calculateConsumptionRates(trips)` - MOST COMPLEX

**Test Cases Needed:**
- [ ] Single full tank fillup → correct rate
- [ ] Multiple trips same period → all get same rate
- [ ] Partial fillup → doesn't close period
- [ ] Partial + full fillup → sums fuel, then closes
- [ ] No fillups → all use TP rate (estimated)
- [ ] Empty trips array → empty map
- [ ] Zero km in period → handle division

### 3.2 `calculateFuelRemaining(trips, rates)` - FUEL TRACKING

**Test Cases Needed:**
- [ ] Basic trip → correct deduction
- [ ] Full tank fillup → resets to tankSize
- [ ] Partial fillup → adds fuel directly
- [ ] Overfill with partial → capped at tankSize
- [ ] Zostatok goes negative → clamped to 0

### 3.3 `calculateDateWarnings(trips)` - DATE VALIDATION

**Test Cases Needed:**
- [ ] Correct order → no warnings
- [ ] Date out of order → warning set
- [ ] Same dates → no warning
- [ ] Single trip → no warning

### 3.4 `calculateConsumptionWarnings(trips, rates)` - LEGAL LIMIT

**Test Cases Needed:**
- [ ] Rate under 120% TP → no warning
- [ ] Rate at 120% TP → no warning (edge)
- [ ] Rate over 120% TP → warning

---

## 4. Implementation Priority

1. **Fix backend tests** - Add `full_tank` to test helpers (2 fixes)
2. **Extract frontend calculations** - Move to `src/lib/calculations.ts`
3. **Setup Vitest** - Add dependencies and config
4. **Port key tests** - Start with Excel verification tests from Rust

## 5. Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/db.rs` | Fix test helpers (add `full_tank`) |
| `src/lib/calculations.ts` | NEW - extracted functions |
| `src/lib/calculations.test.ts` | NEW - unit tests |
| `src/lib/components/TripGrid.svelte` | Import from calculations.ts |
| `package.json` | Add vitest dependencies |
| `vitest.config.ts` | NEW - test configuration |
