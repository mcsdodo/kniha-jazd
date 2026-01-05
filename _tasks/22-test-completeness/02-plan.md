# Implementation Plan: Test Completeness

**Date:** 2026-01-05
**Status:** Ready for implementation

## Overview

Add targeted tests to verify business rules that are currently implemented but not explicitly tested. Focus on correctness and maintainability, not coverage metrics.

## Phase 1: Unit Tests (commands.rs)

### 1.1 Partial Fill-up Period Handling

**File:** `src-tauri/src/commands.rs` (add to existing `#[cfg(test)]` module)

**Business Rule:** BIZ from Task 06 - Only `full_tank=true` fillups close a consumption period

```rust
#[test]
fn test_period_rates_partial_fillup_doesnt_close_period() {
    // Setup: 4 trips
    // Trip 1: 100km (no fuel)
    // Trip 2: 100km + 20L PARTIAL (full_tank=false)
    // Trip 3: 100km (no fuel)
    // Trip 4: 100km + 30L FULL (full_tank=true)

    // Expected: All 4 trips get rate = 50L/400km*100 = 12.5 l/100km
    // The partial fillup at trip 2 does NOT create a separate period
}

#[test]
fn test_period_rates_full_fillup_closes_period() {
    // Setup: 4 trips with full fillup at trip 2
    // Trip 1: 100km (no fuel) -> period 1
    // Trip 2: 100km + 10L FULL -> closes period 1, rate = 10/200*100 = 5.0
    // Trip 3: 100km (no fuel) -> period 2
    // Trip 4: 100km + 8L FULL -> closes period 2, rate = 8/200*100 = 4.0

    // Expected: Trips 1-2 get 5.0, trips 3-4 get 4.0
}
```

### 1.2 Warning Calculations

**File:** `src-tauri/src/commands.rs`

```rust
#[test]
fn test_date_warnings_detects_out_of_order() {
    // Trips by sort_order: [Jan 15, Jan 10, Jan 20]
    // Trip at index 1 (Jan 10) has earlier date than index 0 (Jan 15)
    // Expected: Jan 10 trip ID in date_warnings set
}

#[test]
fn test_date_warnings_correct_order_no_warnings() {
    // Trips in correct chronological order
    // Expected: empty date_warnings set
}

#[test]
fn test_consumption_warnings_over_120_percent() {
    // Trip with rate 7.2 l/100km, TP rate = 5.0
    // 7.2 > 5.0 * 1.2 (6.0) = over limit
    // Expected: trip ID in consumption_warnings set
}

#[test]
fn test_consumption_warnings_at_limit_not_flagged() {
    // Trip with rate 6.0 l/100km, TP rate = 5.0
    // 6.0 = 5.0 * 1.2 = exactly at limit (not over)
    // Expected: NOT in consumption_warnings set
}
```

### 1.3 Year Carryover

**File:** `src-tauri/src/commands.rs`

```rust
#[test]
fn test_year_start_fuel_no_previous_year() {
    // Vehicle with no 2023 trips
    // Query year_start for 2024
    // Expected: full tank (tank_size)
}

#[test]
fn test_year_start_fuel_with_previous_year() {
    // Vehicle with 2023 trips, last trip ends with 30L remaining
    // Query year_start for 2024
    // Expected: 30L (carryover from 2023)
}
```

## Phase 2: Integration Tests

### 2.1 Trip Flow with Fuel Calculation

**File:** `tests/integration/specs/trip-fuel.spec.ts`

```typescript
describe('Trip with Fuel Calculation', () => {
  it('should calculate consumption rate after full tank fillup', async () => {
    // 1. Create vehicle (tank=50L, TP=6.0)
    // 2. Add trip: 200km, no fuel
    // 3. Add trip: 100km, 18L full tank
    // 4. Verify rate shown: 18/300*100 = 6.0 l/100km
    // 5. Verify no over-limit warning (6.0 = 6.0*1.2 at limit)
  });

  it('should show warning when consumption exceeds 120% of TP', async () => {
    // 1. Create vehicle (TP=5.0)
    // 2. Add trips totaling 200km
    // 3. Add fillup: 15L (rate = 7.5 l/100km)
    // 4. Verify warning indicator visible
  });
});
```

### 2.2 Compensation Suggestion Flow

**File:** `tests/integration/specs/compensation.spec.ts`

```typescript
describe('Compensation Suggestions', () => {
  it('should suggest buffer trip when over margin', async () => {
    // 1. Create vehicle with routes
    // 2. Create trips that result in >20% margin
    // 3. Navigate to suggestion panel
    // 4. Verify suggestion shows with km needed
    // 5. (Optional) Apply suggestion, verify margin decreases
  });
});
```

## Implementation Order

| Step | Task | Effort | Dependency |
|------|------|--------|------------|
| 1 | Add partial fill-up tests | 30 min | None |
| 2 | Add warning calculation tests | 30 min | None |
| 3 | Add year carryover tests | 20 min | None |
| 4 | Run `cargo test`, fix if needed | 15 min | Steps 1-3 |
| 5 | Implement trip-fuel.spec.ts | 1 hour | Integration setup working |
| 6 | Implement compensation.spec.ts | 1 hour | Step 5 |
| 7 | Verify CI passes | 15 min | All above |
| 8 | Update CLAUDE.md test counts | 5 min | All above |

## Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/src/commands.rs` | Add ~8 unit tests |
| `tests/integration/specs/trip-fuel.spec.ts` | New file, 2 tests |
| `tests/integration/specs/compensation.spec.ts` | New file, 1 test |
| `CLAUDE.md` | Update test count (93 → ~101) |

## Test Data Requirements

### Unit Tests
- Use in-memory database (`Database::in_memory()`)
- Create test vehicles/trips inline (existing pattern in `db.rs` tests)

### Integration Tests
- Use sandboxed temp directory (already configured in `wdio.conf.ts`)
- May need to add seed data helpers to `tests/integration/fixtures/seed-data.ts`

## Success Criteria

```bash
# All pass
cd src-tauri && cargo test

# Integration tests pass (requires debug build)
npm run test:integration
```

## Notes

- Keep tests focused on business rules, not implementation details
- Each test should document WHY the behavior matters (comments)
- Integration tests may be flaky on CI - add retries if needed
