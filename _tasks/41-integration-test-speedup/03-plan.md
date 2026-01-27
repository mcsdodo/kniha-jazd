# Implementation Plan: Integration Test Speedup

## Phase 1: Backend - Add Test Reset Command

### Step 1.1: Add `reset_test_database` command

**File:** `src-tauri/src/commands.rs`

```rust
/// Reset database for integration tests - clears all data tables.
/// Only available when KNIHA_JAZD_DATA_DIR env var is set (test mode).
#[tauri::command]
pub fn reset_test_database(app_state: State<'_, AppState>) -> Result<(), String> {
    // Guard: only allow in test mode
    if std::env::var("KNIHA_JAZD_DATA_DIR").is_err() {
        return Err("reset_test_database only available in test mode".into());
    }

    let conn = db::get_connection(&app_state).map_err(|e| e.to_string())?;

    // Delete all data (order matters for foreign keys)
    conn.execute("DELETE FROM trips", []).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM receipts", []).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM routes", []).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM vehicles", []).map_err(|e| e.to_string())?;
    // Settings can stay or be reset to defaults

    Ok(())
}
```

**Register in lib.rs:**
Add `commands::reset_test_database` to invoke_handler.

### Step 1.2: Add unit test for command

**File:** `src-tauri/src/commands_tests.rs`

```rust
#[test]
fn test_reset_test_database_requires_env_var() {
    // Without env var, should fail
    std::env::remove_var("KNIHA_JAZD_DATA_DIR");
    // ... test that command returns error
}

#[test]
fn test_reset_test_database_clears_data() {
    std::env::set_var("KNIHA_JAZD_DATA_DIR", temp_dir);
    // Create some data, call reset, verify empty
}
```

---

## Phase 2: Frontend - Add Store Reset Helpers

### Step 2.1: Create test helpers module

**File:** `src/lib/stores/test-helpers.ts` (new)

```typescript
import { vehiclesStore, activeVehicleStore } from './vehicles';
import { tripsStore } from './trips';
import { routesStore } from './routes';
import { selectedYearStore } from './year';
import { receiptRefreshTrigger } from './receipts';
import type { Vehicle } from '$lib/types';

/**
 * Reset all stores to initial state.
 * Only exposed in development builds for testing.
 */
export function resetAllStores(): void {
  vehiclesStore.set([]);
  activeVehicleStore.set(null);
  tripsStore.set([]);
  routesStore.set([]);
  selectedYearStore.set(new Date().getFullYear());
  receiptRefreshTrigger.set(0);
}

/**
 * Set vehicles store directly (for test seeding).
 */
export function setVehiclesStore(vehicles: Vehicle[]): void {
  vehiclesStore.set(vehicles);
}

/**
 * Set active vehicle store directly (for test seeding).
 */
export function setActiveVehicleStore(vehicle: Vehicle | null): void {
  activeVehicleStore.set(vehicle);
}

// Expose globally for WebdriverIO tests
if (typeof window !== 'undefined') {
  (window as any).__TEST_RESET_STORES__ = resetAllStores;
  (window as any).__TEST_SET_VEHICLES__ = setVehiclesStore;
  (window as any).__TEST_SET_ACTIVE_VEHICLE__ = setActiveVehicleStore;
}
```

### Step 2.2: Import in app entry point

**File:** `src/routes/+layout.svelte`

Add import at top (only runs in browser):
```typescript
import '$lib/stores/test-helpers';
```

---

## Phase 3: Update Test Infrastructure

### Step 3.1: Update wdio.conf.ts beforeTest hook

**File:** `tests/integration/wdio.conf.ts`

Replace the current `beforeTest` function:

```typescript
beforeTest: async function () {
  // Set locale to English
  await browser.execute(() => {
    localStorage.setItem('kniha-jazd-locale', 'en');
  });

  // Reset database via Tauri IPC (no file deletion needed)
  await browser.execute(async () => {
    if (window.__TAURI__?.core?.invoke) {
      await window.__TAURI__.core.invoke('reset_test_database');
    }
  });

  // Reset all Svelte stores
  await browser.execute(() => {
    (window as any).__TEST_RESET_STORES__?.();
  });

  // Small pause for reactivity to settle
  await browser.pause(200);
}
```

### Step 3.2: Update seed functions to not refresh

**File:** `tests/integration/utils/db.ts`

Remove `browser.refresh()` and `waitForAppReady()` from:
- `seedVehicle()` - lines 157-158
- `seedScenario()` - lines 376-377
- `setActiveVehicle()` - lines 422-423

Replace with store updates:

```typescript
export async function seedVehicle(data: SeedVehicleData): Promise<Vehicle> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready for seeding');
  }

  const vehicle = await invokeTauri<Vehicle>('create_vehicle', args);

  // Update stores directly (no refresh needed)
  const vehicles = await invokeTauri<Vehicle[]>('get_vehicles');
  await browser.execute((v) => {
    (window as any).__TEST_SET_VEHICLES__?.(v);
  }, vehicles);

  return vehicle;
}
```

### Step 3.3: Update setActiveVehicle

```typescript
export async function setActiveVehicle(vehicleId: string): Promise<void> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }

  await invokeTauri<void>('set_active_vehicle', { id: vehicleId });

  // Update active vehicle store
  const vehicle = await invokeTauri<Vehicle | null>('get_active_vehicle');
  await browser.execute((v) => {
    (window as any).__TEST_SET_ACTIVE_VEHICLE__?.(v);
  }, vehicle);
}
```

---

## Phase 4: Testing & Validation

### Step 4.1: Run Tier 1 tests

```bash
npm run test:integration:tier1
```

All 39 tests should pass without browser.refresh().

### Step 4.2: Measure time improvement

Compare before/after:
- Run Tier 1 with old beforeTest (baseline)
- Run Tier 1 with new beforeTest (optimized)
- Calculate per-test overhead reduction

### Step 4.3: Run all tiers

```bash
npm run test:integration
```

Verify no regressions across all 103 tests.

### Step 4.4: Add isolation verification test

**File:** `tests/integration/specs/tier1/isolation.spec.ts` (new)

```typescript
describe('Test Isolation', () => {
  it('should have empty database at test start', async () => {
    const vehicles = await getVehicles();
    expect(vehicles).toHaveLength(0);
  });

  it('should not see data from previous test', async () => {
    // Previous test created a vehicle, this test should not see it
    const vehicles = await getVehicles();
    expect(vehicles).toHaveLength(0);

    // Create vehicle in this test
    await seedVehicle({ name: 'Test', licensePlate: 'ABC-123', initialOdometer: 1000 });
    const afterSeed = await getVehicles();
    expect(afterSeed).toHaveLength(1);
  });

  it('should again have empty database', async () => {
    // Verify reset worked
    const vehicles = await getVehicles();
    expect(vehicles).toHaveLength(0);
  });
});
```

---

## Phase 5: Cleanup & Documentation

### Step 5.1: Remove old file deletion code

Remove from wdio.conf.ts:
- `getTestDbPath()` function (if unused elsewhere)
- File deletion retry logic in old beforeTest
- WAL/SHM cleanup code

### Step 5.2: Update CLAUDE.md

Add note about test reset mechanism:
```markdown
### Test Database Reset

Integration tests use `reset_test_database` Tauri command + store reset
instead of file deletion. This is faster (~200ms vs ~4s per test).

Test-only code is guarded by `KNIHA_JAZD_DATA_DIR` env var.
```

### Step 5.3: Update tests/CLAUDE.md

Document the new pattern for future test authors.

---

## Rollback Plan

If tests become flaky:
1. Revert to file deletion approach (keep old code commented)
2. Increase pause from 200ms to 500ms
3. Add `browser.refresh()` back for specific problematic tests

---

## Checklist

- [ ] Phase 1: Backend command added and tested
- [ ] Phase 2: Store helpers added and imported
- [ ] Phase 3: wdio.conf.ts and db.ts updated
- [ ] Phase 4: All tests passing, time improvement measured
- [ ] Phase 5: Documentation updated
