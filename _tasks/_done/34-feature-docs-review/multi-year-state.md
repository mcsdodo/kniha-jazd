# Review: multi-year-state.md

## Convention Compliance

**Overall:** POOR - Significant embedded code that should be references

The document embeds 12+ code blocks (Rust, TypeScript, SQL) that violate the convention "Reference code with `file.rs:L###` pointers, don't embed full implementations."

**What's compliant:**
- User Flow section (prose, no code)
- Type-specific fields table (reference data, not implementation)
- Design Decisions section (prose explanations)
- Key Files table (good format)

**What violates convention:**
- 6 Rust code blocks with full function signatures/implementations
- 4 TypeScript code blocks with store definitions and logic
- 2 SQL query blocks with full query text

## Issues Found

### 1. Embedded Rust Code (6 instances)

| Location | Content | Should Be |
|----------|---------|-----------|
| Lines 23-31 | `get_year_start_fuel_remaining()` full signature | `commands.rs:L###` - Function retrieves previous year's ending fuel state |
| Lines 42-49 | `get_year_start_battery_remaining()` full signature | `commands.rs:L###` - Function retrieves previous year's ending battery state |
| Lines 58-66 | `get_year_start_odometer()` full signature | `commands.rs:L###` - Searches up to 10 years for last odometer |
| Lines 74-79 | `VehicleType` enum definition | `models.rs:L###` - Defines ICE/BEV/PHEV types |
| Lines 87-93 | Vehicle type immutability check | `commands.rs:L###` - Prevents type change after trips exist |
| Lines 119-142 | `reorder_trip()` and insertion logic | `commands.rs:L###` and `db.rs:L###` - Trip reordering implementation |

### 2. Embedded TypeScript Code (4 instances)

| Location | Content | Should Be |
|----------|---------|-----------|
| Lines 147-157 | Store definitions | `stores/vehicles.ts:L###` and `stores/year.ts:L###` |
| Lines 162-167 | Year picker population | `+layout.svelte:L###` - `loadYears()` function |
| Lines 171-175 | Reactive data loading | `+page.svelte:L###` - Reactive `$:` block |
| Lines 179-188 | Vehicle change handler | `+layout.svelte:L###` - `handleVehicleChange()` |

### 3. Embedded SQL (2 instances)

| Location | Content | Should Be |
|----------|---------|-----------|
| Lines 114-116 | ALTER TABLE migration | Reference to migration file in `migrations/` |
| Lines 193-210 | Year filtering queries | `db.rs:L###` - Query implementations |

### 4. Documentation Drift Risk

The embedded code creates maintenance burden:
- Function signatures may change
- Line numbers in actual files may shift
- Copy-pasted code may become stale

## Recommendations

### High Priority (Code Removal)

1. **Replace all Rust function signatures with references:**
   ```markdown
   **Fuel Carryover:** See `commands.rs:L###` `get_year_start_fuel_remaining()`
   - Gets trips from previous year and calculates ending fuel state
   - Recursive: chains through multiple years to find initial state
   ```

2. **Replace TypeScript blocks with file references:**
   ```markdown
   **Stores:** `stores/vehicles.ts:L###` and `stores/year.ts:L###`
   - `activeVehicleStore` - Currently selected vehicle
   - `selectedYearStore` - Currently selected year (resets on vehicle change)
   ```

3. **Replace SQL queries with db.rs references:**
   ```markdown
   **Year Filtering:** `db.rs:L###` `get_trips_for_vehicle_in_year()`
   - Uses strftime to extract year from date
   - Returns trips ordered by sort_order
   ```

### Medium Priority (Structure)

4. **Keep the behavior descriptions** - The prose explanations of what each function does are valuable and should remain.

5. **Keep tables** - The type-specific fields table and Key Files table follow convention.

6. **Consider adding a Data Flow diagram** - The document lacks a visual flow diagram that the template suggests.

### Suggested Refactored Structure

```markdown
### Year Carryover

The system maintains continuity across years by calculating ending states.

**Fuel Carryover (ICE/PHEV):** `commands.rs:L###` `get_year_start_fuel_remaining()`
- Gets trips from previous year (year - 1)
- If no previous year data → returns full tank
- Recursive: chains back through multiple years

**Battery Carryover (BEV/PHEV):** `commands.rs:L###` `get_year_start_battery_remaining()`
- Gets trips from previous year
- If no previous year data → returns initial battery percent × capacity
- Non-recursive: resets if previous year has no data

**Odometer Carryover:** `commands.rs:L###` `get_year_start_odometer()`
- Searches up to 10 years back
- Falls back to vehicle's initial_odometer
```

This preserves the behavioral documentation while eliminating drift-prone code blocks.

## Summary

| Aspect | Status |
|--------|--------|
| User Flow | PASS |
| Technical Implementation | FAIL - too much embedded code |
| Key Files | PASS |
| Design Decisions | PASS |
| Maintenance burden | HIGH - 12+ code blocks to keep synchronized |

**Estimated effort to fix:** Medium (1-2 hours) - Replace code blocks with references, keeping behavioral descriptions.
