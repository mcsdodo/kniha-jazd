# Skipped Tests Analysis

## Summary

| Category | Skipped Tests | Root Cause | Fix Complexity |
|----------|---------------|------------|----------------|
| BEV Energy Fields | 4 tests | db.rs missing INSERT columns | Low (add 4 columns) |
| PHEV Energy Fields | 4 tests | db.rs missing INSERT columns | Low (same fix) |
| EV Badge UI | 1 test | Flaky UI timing | Medium (investigate) |
| Receipt File Seeding | 4 tests | No receipt seeding infrastructure | High (new feature) |
| **Total** | **14 tests** | | |

---

## Category 1: BEV Energy Fields (4 tests)

### Root Cause
The `db.rs` `create_trip()` function's INSERT statement is missing energy-related columns:
- `energy_kwh`
- `energy_cost_eur`
- `full_charge`
- `soc_override_percent`

**Evidence:**
```sql
-- Current INSERT (line 450 in db.rs):
INSERT INTO trips (id, vehicle_id, date, origin, destination, distance_km,
  odometer, purpose, fuel_liters, fuel_cost_eur, other_costs_eur,
  other_costs_note, full_tank, sort_order, created_at, updated_at)
-- Missing: energy_kwh, energy_cost_eur, full_charge, soc_override_percent
```

The schema HAS these columns (from migration `005_add_ev_support.sql`), but they're not being written.

### Affected Tests

| Test | File | Purpose |
|------|------|---------|
| `should create BEV trip with charging session (kWh, cost)` | bev-trips.spec.ts | Verify energy fields are persisted |
| `should calculate energy consumption rate (kWh/100km)` | bev-trips.spec.ts | Verify energy rate calculation |
| `should track battery SoC remaining after trips` | bev-trips.spec.ts | Verify battery tracking |
| `should create BEV trip without fuel fields (fuel_liters = null)` | bev-trips.spec.ts | Verify BEV trips don't use fuel |

### Fix Required

**File:** `src-tauri/src/db.rs` (line ~450)

**Change:** Add the 4 missing columns to the INSERT statement:

```rust
// Before:
"INSERT INTO trips (id, vehicle_id, date, origin, destination, distance_km, odometer, purpose, fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note, full_tank, sort_order, created_at, updated_at)
 VALUES (:id, :vehicle_id, :date, :origin, :destination, :distance_km, :odometer, :purpose, :fuel_liters, :fuel_cost_eur, :other_costs_eur, :other_costs_note, :full_tank, :sort_order, :created_at, :updated_at)"

// After:
"INSERT INTO trips (id, vehicle_id, date, origin, destination, distance_km, odometer, purpose, fuel_liters, fuel_cost_eur, energy_kwh, energy_cost_eur, other_costs_eur, other_costs_note, full_tank, full_charge, soc_override_percent, sort_order, created_at, updated_at)
 VALUES (:id, :vehicle_id, :date, :origin, :destination, :distance_km, :odometer, :purpose, :fuel_liters, :fuel_cost_eur, :energy_kwh, :energy_cost_eur, :other_costs_eur, :other_costs_note, :full_tank, :full_charge, :soc_override_percent, :sort_order, :created_at, :updated_at)"
```

**Also update the named_params! block to include:**
```rust
":energy_kwh": trip.energy_kwh,
":energy_cost_eur": trip.energy_cost_eur,
":full_charge": trip.full_charge,
":soc_override_percent": trip.soc_override_percent,
```

**Effort:** ~30 minutes (add columns, test, commit)

---

## Category 2: PHEV Energy Fields (4 tests)

### Root Cause
Same as BEV - the `db.rs` INSERT is missing energy columns.

### Affected Tests

| Test | File | Purpose |
|------|------|---------|
| `should record both fuel and energy on same trip` | phev-trips.spec.ts | PHEV uses both systems |
| `should show both consumption rates in stats` | phev-trips.spec.ts | Dual rate display |
| `should handle energy-only trip on PHEV (fuel_liters = null)` | phev-trips.spec.ts | Electric-only mode |
| `should calculate correct margin for PHEV with mixed usage` | phev-trips.spec.ts | Legal margin calculation |

### Fix Required
**Same fix as BEV** - once energy columns are added to INSERT, all PHEV tests will also work.

**Effort:** Included in BEV fix (0 additional effort)

---

## Category 3: EV Badge UI (1 test)

### Root Cause
The test `should show BEV badge in vehicle list` is flaky due to UI timing issues.

**From the test comment:**
> The badge uses class="badge type-{vehicle.vehicle_type.toLowerCase()}" which produces "badge type-bev".
> Needs investigation into why vehicle list doesn't update reliably after creation.

### Affected Tests

| Test | File | Purpose |
|------|------|---------|
| `should show BEV badge in vehicle list` | ev-vehicle.spec.ts | Verify BEV badge appears |

### Fix Options

1. **Add explicit wait for badge** (Quick)
   ```typescript
   await browser.waitUntil(async () => {
     const badge = await $('.badge.type-bev');
     return badge.isDisplayed();
   }, { timeout: 5000 });
   ```

2. **Refresh after vehicle creation** (Medium)
   ```typescript
   await browser.refresh();
   await waitForAppReady();
   ```

3. **Investigate Svelte reactivity** (Thorough)
   - Check if `vehiclesStore` updates trigger re-render
   - May be a Svelte 5 runes reactivity issue

**Effort:** 15 minutes (option 1), 1-2 hours (option 3)

---

## Category 4: Receipt File Seeding (4 tests)

### Root Cause
Receipt tests require actual receipt files on disk. The app's receipt system works by scanning a folder for files (via `scan_receipts` command), not by creating receipts programmatically.

**There is no `create_receipt` command** - receipts are discovered from the filesystem.

### Affected Tests

| Test | File | Purpose |
|------|------|---------|
| `should display pre-seeded receipts in list` | receipts.spec.ts | View receipts |
| `should filter receipts by status` | receipts.spec.ts | Filter UI |
| `should assign receipt to trip` | receipts.spec.ts | Link receipt to trip |
| `should delete receipt from list` | receipts.spec.ts | Remove receipt |

### Fix Options

1. **Create receipt seeding infrastructure** (High effort)
   - Add test helper to copy receipt files to test data dir
   - Run `scan_receipts` to discover them
   - Clean up after tests

   ```typescript
   async function seedReceiptFile(filename: string, content: Buffer): Promise<void> {
     const receiptsDir = path.join(testDataDir, 'receipts', '2024');
     await fs.mkdir(receiptsDir, { recursive: true });
     await fs.writeFile(path.join(receiptsDir, filename), content);
     await invokeTauri('scan_receipts');
   }
   ```

2. **Add `create_receipt_for_testing` command** (Medium effort)
   - Backend command that creates a receipt without a file
   - Only enabled in test builds
   - Cleaner than file manipulation

3. **Mock the receipt API** (Low effort, limited value)
   - Override `get_receipts` to return fake data
   - Doesn't test real functionality

**Effort:** 2-4 hours (option 1), 1-2 hours (option 2)

---

## Fix Priority

### High Priority (Quick Wins)
1. **Fix db.rs INSERT** - Enables 8 tests (BEV + PHEV)
   - Effort: 30 min
   - Impact: 8 tests enabled

### Medium Priority
2. **Fix EV badge test** - 1 flaky test
   - Effort: 15 min
   - Impact: 1 test stabilized

### Lower Priority (Infrastructure)
3. **Receipt seeding** - 4 tests need this
   - Effort: 2-4 hours
   - Impact: 4 tests enabled
   - Consider: Is receipt testing worth the infrastructure?

---

## Recommended Action Plan

### Phase 1: Quick Fix (30 min)
1. Update `db.rs` INSERT to include energy columns
2. Run backend tests to verify no regressions
3. Remove `.skip` from 8 BEV/PHEV tests
4. Run integration tests to verify

### Phase 2: Stabilize (15 min)
1. Fix EV badge test with explicit wait
2. Run full test suite 3 times to verify no flakiness

### Phase 3: Evaluate (Future)
1. Decide if receipt file seeding is worth the effort
2. If yes, implement option 1 or 2
3. If no, document as "requires manual testing"

---

## Appendix: Test Details

### bev-trips.spec.ts (4 skipped)

```typescript
// Test 1: BEV Trip with Charging
it.skip('should create BEV trip with charging session (kWh, cost)')
// Seeds: BEV vehicle, 2 trips (one with energy charge)
// Validates: energy_kwh, energy_cost_eur, full_charge are persisted

// Test 2: BEV Energy Consumption Rate
it.skip('should calculate energy consumption rate (kWh/100km)')
// Seeds: BEV vehicle, 2 trips with full charge
// Validates: energy_rates[tripId] is calculated correctly

// Test 3: BEV Battery SoC Tracking
it.skip('should track battery SoC remaining after trips')
// Seeds: BEV vehicle, 2 trips without charging
// Validates: battery_remaining_kwh, battery_remaining_percent

// Test 4: BEV Trips Without Fuel Fields
it.skip('should create BEV trip without fuel fields (fuel_liters = null)')
// Seeds: BEV vehicle, 1 trip with energy
// Validates: fuel_liters/fuel_cost_eur are null, energy fields populated
```

### phev-trips.spec.ts (4 skipped)

```typescript
// Test 1: PHEV Mixed Fuel and Energy
it.skip('should record both fuel and energy on same trip')
// Seeds: PHEV vehicle, 1 trip with both fuel AND energy
// Validates: Both systems recorded on same trip

// Test 2: PHEV Both Consumption Rates
it.skip('should show both consumption rates in stats')
// Seeds: PHEV vehicle, 2 trips with dual refills
// Validates: rates[tripId] AND energy_rates[tripId] both exist

// Test 3: PHEV Energy-Only Trip
it.skip('should handle energy-only trip on PHEV (fuel_liters = null)')
// Seeds: PHEV vehicle, 2 trips (energy only, no fuel)
// Validates: Energy tracking works without fuel

// Test 4: PHEV Margin Calculation
it.skip('should calculate correct margin for PHEV with mixed usage')
// Seeds: PHEV vehicle, 2 trips with over-consumption
// Validates: consumption_warnings includes the trip (fuel portion only)
```

### ev-vehicle.spec.ts (1 skipped)

```typescript
// Test: BEV Badge Display
it.skip('should show BEV badge in vehicle list')
// Seeds: BEV vehicle via UI
// Validates: .badge.type-bev element is visible
// Issue: UI doesn't reliably update after vehicle creation
```

### receipts.spec.ts (4 skipped)

```typescript
// Test 1: Receipt Display
it.skip('should display pre-seeded receipts in list')
// Requires: Receipt files in filesystem
// Validates: Receipt list shows items

// Test 2: Receipt Filtering
it.skip('should filter receipts by status')
// Requires: Receipt files with different statuses
// Validates: Filter buttons work

// Test 3: Receipt Assignment
it.skip('should assign receipt to trip')
// Requires: Receipt files + seeded trip
// Validates: Receipt→Trip linking works

// Test 4: Receipt Deletion
it.skip('should delete receipt from list')
// Requires: Receipt files
// Validates: Delete removes from list
```
