# Integration Test Suite Analysis

## Overview

**Total Runtime:** ~1:53 (113 seconds)
**Spec Files:** 17
**Active Tests:** 48
**Skipped Tests:** 14 (BEV/PHEV energy fields, receipt file seeding)

## Timing Breakdown

| Spec File | Tests | Time | Overhead |
|-----------|-------|------|----------|
| ev-vehicle.spec.ts | 5 (1 skip) | 6.4s | ~1.3s/test |
| vehicle-setup.spec.ts | 4 | 4.0s | ~1.0s/test |
| bev-trips.spec.ts | 0 (4 skip) | 0.0s | - |
| consumption-warnings.spec.ts | 2 | 2.7s | ~1.4s/test |
| export.spec.ts | 2 | 6.8s | ~3.4s/test |
| phev-trips.spec.ts | 1 (4 skip) | 1.2s | ~1.2s/test |
| seeding.spec.ts | 7 | 7.3s | ~1.0s/test |
| trip-management.spec.ts | 7 | 9.6s | ~1.4s/test |
| year-handling.spec.ts | 2 | 5.2s | ~2.6s/test |
| backup-restore.spec.ts | 3 | 4.1s | ~1.4s/test |
| receipts.spec.ts | 1 (4 skip) | 2.2s | ~2.2s/test |
| settings.spec.ts | 2 | 4.3s | ~2.2s/test |
| vehicle-management.spec.ts | 3 | 6.0s | ~2.0s/test |
| compensation.spec.ts | 2 | 2.6s | ~1.3s/test |
| empty-states.spec.ts | 2 | 4.0s | ~2.0s/test |
| multi-vehicle.spec.ts | 2 | 5.3s | ~2.7s/test |
| validation.spec.ts | 3 | 6.5s | ~2.2s/test |

**Actual Test Time:** ~78 seconds
**Per-Spec Overhead:** ~35 seconds (~2 sec/spec for worker startup, session, app load)

---

## Use Case Coverage Matrix

### Core Business Logic (Tier 1 - Critical)

| Use Case | Test Location | Status |
|----------|---------------|--------|
| **Trip CRUD** | | |
| Create trip with basic fields | trip-management.spec.ts | ✅ |
| Create trip with full tank refill | trip-management.spec.ts | ✅ |
| Create trip with partial refill | trip-management.spec.ts | ✅ |
| Edit existing trip | trip-management.spec.ts | ✅ |
| Delete trip | trip-management.spec.ts | ✅ |
| Insert trip between existing | trip-management.spec.ts | ✅ |
| Reorder trips via keyboard | trip-management.spec.ts | ✅ |
| **Consumption Calculation** | | |
| Normal consumption (under TP) | consumption-warnings.spec.ts | ✅ |
| Warning when over 20% margin | consumption-warnings.spec.ts | ✅ |
| **Year Handling** | | |
| Filter trips by year | year-handling.spec.ts | ✅ |
| Fuel carryover between years | year-handling.spec.ts | ✅ |
| **Export** | | |
| Open export preview | export.spec.ts | ✅ |
| Correct totals in export | export.spec.ts | ✅ |
| **Data Seeding (IPC)** | | |
| Seed vehicle via IPC | seeding.spec.ts | ✅ |
| Seed trip via IPC | seeding.spec.ts | ✅ |
| Seed complete scenario | seeding.spec.ts | ✅ |

### Vehicle Management (Tier 2)

| Use Case | Test Location | Status |
|----------|---------------|--------|
| Create ICE vehicle | vehicle-setup.spec.ts | ✅ |
| Create BEV vehicle | ev-vehicle.spec.ts | ✅ |
| Create PHEV vehicle | vehicle-management.spec.ts | ✅ |
| Edit vehicle | vehicle-management.spec.ts | ✅ |
| Delete vehicle | vehicle-management.spec.ts | ✅ |
| Show battery fields for BEV | ev-vehicle.spec.ts | ✅ |
| Show both fields for PHEV | ev-vehicle.spec.ts | ✅ |
| Block type change with trips | ev-vehicle.spec.ts | ✅ |

### Settings & Backup (Tier 2)

| Use Case | Test Location | Status |
|----------|---------------|--------|
| Save company settings | settings.spec.ts | ✅ |
| Switch language | settings.spec.ts | ✅ |
| Create backup | backup-restore.spec.ts | ✅ |
| Restore backup | backup-restore.spec.ts | ✅ |
| Delete backup | backup-restore.spec.ts | ✅ |

### Edge Cases (Tier 3)

| Use Case | Test Location | Status |
|----------|---------------|--------|
| No vehicle prompt | empty-states.spec.ts | ✅ |
| Empty trip grid | empty-states.spec.ts | ✅ |
| Multi-vehicle switching | multi-vehicle.spec.ts | ✅ |
| Per-vehicle stats isolation | multi-vehicle.spec.ts | ✅ |
| Compensation banner | compensation.spec.ts | ✅ |
| Add buffer trip | compensation.spec.ts | ✅ |
| Odometer warnings | validation.spec.ts | ✅ |
| Negative input handling | validation.spec.ts | ✅ |
| Leap year dates | validation.spec.ts | ✅ |

### Skipped (Need Backend/Infrastructure)

| Use Case | Reason | Status |
|----------|--------|--------|
| BEV trip with charging | db.rs doesn't persist energy_kwh | ⏭️ |
| BEV consumption rate | Needs energy fields | ⏭️ |
| BEV SoC tracking | Needs energy fields | ⏭️ |
| PHEV mixed fuel+energy | Needs energy fields | ⏭️ |
| Receipt display | Needs file seeding | ⏭️ |
| Receipt filtering | Needs file seeding | ⏭️ |
| Receipt assignment | Needs file seeding | ⏭️ |
| Receipt deletion | Needs file seeding | ⏭️ |

---

## Optimization Opportunities

### Current Overhead Analysis

Each spec file incurs ~2 seconds overhead:
1. Worker process spawn (~0.5s)
2. WebDriver session creation (~0.5s)
3. App load + Tauri IPC ready (~0.5s)
4. Database cleanup (~0.5s)

With 17 spec files: **~34 seconds of pure overhead**

### Proposed Consolidation

#### Option A: Consolidate by Feature Domain (Recommended)

Reduce from 17 → 8 spec files:

| New Spec File | Merged From | Est. Time |
|---------------|-------------|-----------|
| **vehicle.spec.ts** | ev-vehicle, vehicle-setup, vehicle-management | ~12s |
| **trip-crud.spec.ts** | trip-management, seeding (trip parts) | ~12s |
| **calculations.spec.ts** | consumption-warnings, year-handling, compensation | ~8s |
| **export.spec.ts** | export | ~7s |
| **settings.spec.ts** | settings, backup-restore | ~7s |
| **multi-vehicle.spec.ts** | multi-vehicle, empty-states | ~7s |
| **validation.spec.ts** | validation | ~6s |
| **receipts.spec.ts** | receipts | ~2s |

**Estimated New Runtime:** ~61s + 16s overhead = **~77s** (vs current 113s)
**Savings:** ~36 seconds (32% faster)

#### Option B: Aggressive Consolidation

Reduce to 4 mega-specs:

| New Spec File | Contents | Tests |
|---------------|----------|-------|
| **core.spec.ts** | Vehicle CRUD, Trip CRUD, Calculations | ~25 |
| **workflow.spec.ts** | Export, Backup, Settings, Receipts | ~10 |
| **edge-cases.spec.ts** | Validation, Empty States, Multi-vehicle | ~10 |
| **ev-support.spec.ts** | BEV, PHEV (mostly skipped) | ~3 active |

**Estimated New Runtime:** ~70s + 8s overhead = **~78s** (vs current 113s)
**Savings:** ~35 seconds (31% faster)

### More Assertions Per Test

Instead of separate tests for related scenarios, combine into workflow tests:

**Before (3 tests, 3 setups):**
```typescript
it('should create trip', ...)
it('should edit trip', ...)
it('should delete trip', ...)
```

**After (1 test, 1 setup):**
```typescript
it('should handle complete trip lifecycle (create → edit → delete)', async () => {
  // Create
  const trip = await seedTrip({...});
  expect(trip.id).toBeDefined();

  // Edit
  await updateTrip(trip.id, {...});
  const updated = await getTripGridData(...);
  expect(updated.trips[0].destination).toBe('New Destination');

  // Delete
  await deleteTrip(trip.id);
  const afterDelete = await getTripGridData(...);
  expect(afterDelete.trips.length).toBe(0);
});
```

**Pros:**
- Single DB setup/teardown
- Tests real workflow sequence
- Faster execution

**Cons:**
- Harder to pinpoint which step failed
- All-or-nothing (one failure skips remaining assertions)
- Less granular CI feedback

### Recommended Approach

1. **Keep Tier 1 tests granular** - These are critical, need clear failure identification
2. **Consolidate Tier 2/3 into workflow tests** - These test less critical paths
3. **Merge related spec files** - Reduce per-file overhead

---

## Implementation Priority

### Quick Wins (Low Effort, High Impact)

1. **Merge ev-vehicle + vehicle-management** → Save ~2s overhead
2. **Merge empty-states + multi-vehicle** → Save ~2s overhead
3. **Merge settings + backup-restore** → Save ~2s overhead

### Medium Effort

4. **Combine seeding tests into single workflow test** → Save ~4s
5. **Combine trip-management into 2-3 workflow tests** → Save ~3s

### Future Consideration

6. **Run specs in parallel** (requires separate DB per worker)
7. **Keep app running between tests** (requires stateless test design)

---

## Summary

| Metric | Current | After Quick Wins | After Full Optimization |
|--------|---------|------------------|------------------------|
| Spec Files | 17 | 12 | 8 |
| Total Time | ~113s | ~100s | ~77s |
| Overhead | ~34s | ~24s | ~16s |
| Savings | - | 13s (12%) | 36s (32%) |

The biggest opportunity is **reducing the number of spec files** to minimize per-file overhead. The actual test execution is already reasonably efficient.
