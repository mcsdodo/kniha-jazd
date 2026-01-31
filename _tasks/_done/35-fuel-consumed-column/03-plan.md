# Implementation Plan: Fuel Consumed Column

**Status:** Complete
**Completed:** 2026-01-31

## Step 1: Backend - Add Calculation Function & Tests

**Files**: `src-tauri/src/commands.rs`, `src-tauri/src/commands_tests.rs`, `src-tauri/src/models.rs`

### 1.1 Write Tests First (TDD)

Add to `commands_tests.rs`:

```rust
#[test]
fn test_fuel_consumed_basic() {
    // 100 km at 6.0 l/100km = 6.0 L consumed
}

#[test]
fn test_fuel_consumed_uses_period_rate() {
    // Verify closed period rate is used
}

#[test]
fn test_fuel_consumed_uses_tp_rate_for_open_period() {
    // Verify TP rate fallback for open periods
}

#[test]
fn test_fuel_consumed_zero_distance() {
    // 0 km = 0 L consumed
}
```

### 1.2 Implement Function

Add `calculate_fuel_consumed()` function to `commands.rs`:

```rust
pub(crate) fn calculate_fuel_consumed(
    trips: &[Trip],
    rates: &HashMap<String, f64>,
) -> HashMap<String, f64> {
    trips.iter().map(|trip| {
        let rate = rates.get(&trip.id).copied().unwrap_or(0.0);
        let consumed = (trip.distance_km * rate) / 100.0;
        (trip.id.clone(), consumed)
    }).collect()
}
```

### 1.3 Update TripGridData Struct

**File**: `src-tauri/src/models.rs` (around line 305)

Add field: `pub fuel_consumed: HashMap<String, f64>`

### 1.4 Update get_trip_grid_data Command

Call `calculate_fuel_consumed()` after `calculate_period_rates()` and include in return value.

### 1.5 Verify Tests Pass

```bash
cd src-tauri && cargo test fuel_consumed
```

---

## Step 2: Frontend - Types & State

**Files**: `src/lib/types.ts`, `src/lib/components/TripGrid.svelte`

### 2.1 Update TripGridData Type

Add `fuelConsumed: Record<string, number>` to interface.

### 2.2 Add State Variable

Add `fuelConsumed: Map<string, number>` in TripGrid.svelte (near line 35).

### 2.3 Load Data

Update `loadGridData()` to populate the new Map:
```typescript
fuelConsumed = new Map(Object.entries(gridData.fuelConsumed));
```

---

## Step 3: Frontend - Column Display

**Files**: `src/lib/components/TripGrid.svelte`, `src/lib/components/TripRow.svelte`

### 3.1 Update Column Widths (CSS nth-child renumbering)

**File**: `TripGrid.svelte` (lines 691-703)

Insert new column at position 9, renumber all subsequent columns:

```css
th:nth-child(8) { width: 4%; text-align: right; }   /* Cena € - unchanged */
th:nth-child(9) { width: 4%; text-align: right; }   /* NEW: Spotr. (L) */
th:nth-child(10) { width: 4%; text-align: right; }  /* l/100km - was 9 */
th:nth-child(11) { width: 4%; text-align: right; }  /* Zostatok - was 10 */
th:nth-child(12) { width: 4%; text-align: right; }  /* Iné € - was 11 */
th:nth-child(13) { width: 10%; }                    /* Iné pozn. - was 12 */
th:nth-child(14) { width: 8%; text-align: center; } /* Akcie - was 13 */
```

### 3.2 Add Column Header

Insert after "Cena €" (line 468), before "l/100km" (line 469):
```svelte
<th>{$LL.trips.columns.fuelConsumed()}</th>
```

### 3.3 Add TripRow Prop

**File**: `TripRow.svelte`

Add export: `export let fuelConsumed: number = 0;`

### 3.4 Add Cell Rendering with Preview Support

**File**: `TripRow.svelte`

Display cell after Cena €, before l/100km. Include preview mode support:

```svelte
<td class="number calculated" class:preview={previewData}>
    {#if previewData}
        ~{((previewData.distanceKm || 0) * (previewData.consumptionRate || 0) / 100).toFixed(1)}
    {:else}
        {fuelConsumed.toFixed(1)}
    {/if}
</td>
```

### 3.5 Update First Record Row

**File**: `TripGrid.svelte` (line 547, after tpConsumption cell)

Add: `<td class="number calculated">0.0</td>`

### 3.6 Pass Prop from TripGrid to ALL TripRow Instances

**File**: `TripGrid.svelte`

Three instances need the new prop:

1. **New row at top** (line 488-507): Add `fuelConsumed={0}`
2. **Insert-above row** (line 513-532): Add `fuelConsumed={0}`
3. **Existing trip rows** (line 560-591): Add `fuelConsumed={fuelConsumed.get(trip.id) || 0}`

### 3.7 Update Empty State Colspan

**File**: `TripGrid.svelte` (line 597)

Change from:
```svelte
colspan={9 + (showFuelColumns ? 4 : 0) + (showEnergyColumns ? 4 : 0)}
```
To:
```svelte
colspan={9 + (showFuelColumns ? 5 : 0) + (showEnergyColumns ? 4 : 0)}
```

---

## Step 4: i18n

**Files**: `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

### 4.1 Add Translations

In `trips.columns` section:
- SK: `fuelConsumed: () => "Spotr. (L)"`
- EN: `fuelConsumed: () => "Cons. (L)"`

---

## Step 5: Verification & Cleanup

### 5.1 Run Backend Tests

```bash
cd src-tauri && cargo test fuel_consumed
cd src-tauri && cargo test  # Full suite
```

### 5.2 Manual Testing

- Add trip with known km and fuel → verify consumed value
- Edit trip km → verify preview updates with `~` prefix
- Check PHEV vehicle shows column
- Check BEV vehicle hides column
- Verify first record shows "0.0"

### 5.3 Update CHANGELOG

Add entry under [Unreleased]:
```markdown
### Added
- Trip grid now shows fuel consumed per trip in "Spotr. (L)" column
```

---

## Files Changed

| File | Change |
|------|--------|
| `src-tauri/src/commands.rs` | Add `calculate_fuel_consumed()`, update command |
| `src-tauri/src/commands_tests.rs` | Add 4 tests |
| `src-tauri/src/models.rs` | Add `fuel_consumed` field to TripGridData |
| `src/lib/types.ts` | Add `fuelConsumed` to TripGridData interface |
| `src/lib/components/TripGrid.svelte` | Add state, header, pass prop, CSS, colspan |
| `src/lib/components/TripRow.svelte` | Add prop, cell with preview support |
| `src/lib/i18n/sk/index.ts` | Add translation |
| `src/lib/i18n/en/index.ts` | Add translation |
| `CHANGELOG.md` | Add entry |

---

## Review Findings Addressed

- [x] C1: Preview mode support added (Step 3.4)
- [x] I1: CSS nth-child renumbering explicit (Step 3.1)
- [x] I2: All 3 TripRow instances specified (Step 3.6)
- [x] I3: First record row location specified (Step 3.5)
- [x] I4: Colspan update added (Step 3.7)
- [x] M3: TripGridData struct location corrected (Step 1.3)
- [~] M1: Integration test removed (YAGNI per CLAUDE.md)
