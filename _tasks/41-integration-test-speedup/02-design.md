# Design: Integration Test Speedup

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    WebdriverIO Test                         │
│                                                             │
│  beforeTest: {                                              │
│    await invoke('reset_test_database')  ──────┐             │
│    await execute(resetStores)           ──┐   │             │
│    localStorage.set('locale', 'en')       │   │             │
│    pause(200ms)                           │   │             │
│  }                                        │   │             │
└───────────────────────────────────────────┼───┼─────────────┘
                                            │   │
                    ┌───────────────────────┘   │
                    ▼                           │
┌─────────────────────────────────────────────────────────────┐
│                    SvelteKit Frontend                       │
│                                                             │
│  window.__TEST_RESET_STORES__ = () => {                     │
│    vehiclesStore.set([])                                    │
│    activeVehicleStore.set(null)                             │
│    tripsStore.set([])                                       │
│    routesStore.set([])                                      │
│    selectedYearStore.set(currentYear)                       │
│  }                                                          │
└─────────────────────────────────────────────────────────────┘
                                                │
                                                │ Tauri IPC
                                                ▼
┌─────────────────────────────────────────────────────────────┐
│                    Rust Backend                             │
│                                                             │
│  #[tauri::command]                                          │
│  pub fn reset_test_database() -> Result<()> {               │
│    // Guard: only in test mode                              │
│    if env::var("KNIHA_JAZD_DATA_DIR").is_err() {           │
│      return Err("Not in test mode")                         │
│    }                                                        │
│                                                             │
│    let conn = get_connection()?;                            │
│    conn.execute("DELETE FROM trips")?;                      │
│    conn.execute("DELETE FROM vehicles")?;                   │
│    conn.execute("DELETE FROM receipts")?;                   │
│    conn.execute("DELETE FROM routes")?;                     │
│    // Reset settings to defaults                            │
│    Ok(())                                                   │
│  }                                                          │
└─────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. Rust: `reset_test_database` Command

**Location:** `src-tauri/src/commands.rs`

**Behavior:**
- Guard: Only execute if `KNIHA_JAZD_DATA_DIR` env var is set (test mode)
- Truncate all data tables (trips, vehicles, receipts, routes)
- Reset settings to defaults (or delete entirely)
- Keep schema intact (no migrations needed)

**Why DELETE instead of file deletion:**
- SQLite connection stays valid
- No file locking issues
- Faster than recreating DB
- WAL mode handles cleanly

### 2. Frontend: Store Reset Function

**Location:** `src/lib/stores/test-helpers.ts` (new file)

**Exposed as:** `window.__TEST_RESET_STORES__` (only in dev builds)

**Stores to reset:**
| Store | Reset Value | Notes |
|-------|-------------|-------|
| `vehiclesStore` | `[]` | Clear vehicle list |
| `activeVehicleStore` | `null` | No active vehicle |
| `tripsStore` | `[]` | Clear trips |
| `routesStore` | `[]` | Clear routes |
| `selectedYearStore` | `currentYear` | Reset to current year |
| `receiptRefreshTrigger` | `0` | Reset trigger counter |

**Stores NOT reset:**
- `localeStore` - Set via localStorage in beforeTest
- `themeStore` - Not test-relevant
- `toastStore` - Auto-clears, no persistence
- `confirmStore` - UI state, auto-clears

### 3. Test Utility Updates

**Files to update:**
- `tests/integration/wdio.conf.ts` - Replace beforeTest hook
- `tests/integration/utils/db.ts` - Remove refreshes from seed functions

**Key change in seed functions:**
```typescript
// BEFORE
export async function seedVehicle(data: SeedVehicleData): Promise<Vehicle> {
  const vehicle = await invokeTauri<Vehicle>('create_vehicle', args);
  await browser.refresh();  // REMOVE THIS
  await waitForAppReady();  // REMOVE THIS
  return vehicle;
}

// AFTER
export async function seedVehicle(data: SeedVehicleData): Promise<Vehicle> {
  const vehicle = await invokeTauri<Vehicle>('create_vehicle', args);
  // Store updates automatically via reactive subscription
  return vehicle;
}
```

### 4. Store Reactivity Enhancement

**Problem:** After seeding via IPC, stores don't auto-update.

**Solution options:**
1. **Manual store update after seed** - Call `refreshVehicles()` after `create_vehicle`
2. **Event-based refresh** - Backend emits event, frontend listens
3. **Test-specific refresh trigger** - Add `refreshAllStores()` utility

**Recommended:** Option 1 (simplest, test-only change)

```typescript
// In db.ts
export async function seedVehicle(data: SeedVehicleData): Promise<Vehicle> {
  const vehicle = await invokeTauri<Vehicle>('create_vehicle', args);
  // Refresh store by fetching fresh data
  const vehicles = await invokeTauri<Vehicle[]>('get_vehicles');
  await browser.execute((v) => {
    window.__TEST_SET_VEHICLES__?.(v);
  }, vehicles);
  return vehicle;
}
```

## Risk Analysis

| Risk | Mitigation |
|------|------------|
| State leakage between tests | Comprehensive store reset, verify with existing tests |
| Missing store reset | Audit all stores, add to reset function |
| Component local state not reset | Document which components need special handling |
| SQLite transaction isolation | DELETE runs in single transaction |
| Flaky tests from timing | Keep 200ms pause, increase if needed |

## Test Strategy

1. Run existing tests with new beforeTest - all should pass
2. Add test that verifies store reset clears data
3. Measure actual time savings vs baseline
