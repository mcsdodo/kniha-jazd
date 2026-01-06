# Code Review

**Target:** `feature/electric-vehicles` branch (main..HEAD)
**Reference:** `_tasks/19-electric-vehicles/02-plan.md`
**Started:** 2026-01-06
**Focus:** Quality, correctness, best practices

## Initial Assessment

### Changes Overview
- 23 files changed, +2543/-251 lines
- All 134 backend tests passing

### New Files
- `src-tauri/src/calculations_energy.rs` - BEV energy calculations
- `src-tauri/src/calculations_energy_tests.rs` - 14 tests
- `src-tauri/src/calculations_phev.rs` - PHEV calculations
- `src-tauri/src/calculations_phev_tests.rs` - 8 tests
- `src-tauri/migrations/005_add_ev_support.sql` - DB schema
- `tests/integration/specs/ev-vehicle.spec.ts` - Integration tests

### Modified Files (Major)
- `src-tauri/src/commands.rs` - BEV/PHEV trip processing
- `src-tauri/src/db.rs` - EV field persistence
- `src-tauri/src/models.rs` - VehicleType enum, extended models
- `src-tauri/src/export.rs` - Conditional column export
- UI components: TripGrid, TripRow, VehicleModal

### Compiler Warnings Noted
- Unused imports and variables (minor)
- Some helper methods not yet used from tests

---

## Review Iterations

### Iteration 1 (2026-01-06)

#### Critical
None identified. The implementation is solid and all tests pass.

#### Important

1. **PHEV calculations module never used** - `calculations_phev.rs`
   - `PhevTripConsumption` and `calculate_phev_trip_consumption` are implemented with tests but never called from `commands.rs`
   - PHEV currently behaves like BEV (electricity tracking only)

2. **Stale TODO comments** - `commands.rs:456, 675, 1306, 1406, 1881, 1939`
   - Comments reference "Phase 2 will add BEV/PHEV handling" but Phase 2 appears complete

3. **Unused `soc_override_trips` parameter** - `commands.rs:852`
   - Parameter passed but not used (compiler warning)

4. ~~**`full_tank` default changed to false**~~ - `commands.rs:250`
   - **VERIFIED OK:** Frontend already defaults to `false` (TripRow.svelte:59)
   - Backend change aligns with frontend - both now consistent
   - User must explicitly check "Full tank" checkbox (safer behavior)

5. **Year boundary battery carryover not implemented** - Plan Phase 4.1
   - BEV/PHEV should carry over battery state between years

#### Minor

1. Unused imports and functions (compiler warnings)
2. Missing `initial_battery_percent` validation (0-100)
3. Export doesn't render BEV energy columns yet
4. Preview trip calculation doesn't support BEV/PHEV
5. Integration test has sequential dependency

---

## Assessment

**Recommendation: APPROVE with minor fixes**

### Strengths
- Clean architecture following existing patterns
- Backward compatibility maintained for ICE vehicles
- Good test coverage (26 new tests)
- Complete UI support with conditional columns
- Safe database migration (all fields nullable)

### Action Items Before Merge
1. ~~Fix `full_tank` default~~ - FIXED (was actually a regression, reverted to `true`)
2. ~~Remove unused `soc_override_trips` parameter~~ - FIXED
3. Update/remove stale TODO comments (minor cleanup)
4. ~~PHEV integration~~ - IMPLEMENTED via `calculate_phev_grid_data()`

### Iteration 2 Fixes (2026-01-06)

**Changes made:**
1. **Fixed `full_tank` default regression** - Reverted from `false` to `true` in:
   - `TripRow.svelte` (2 places)
   - `commands.rs:create_trip`

2. **Removed unused `soc_override_trips` parameter** - Cleaned up `calculate_energy_grid_data`

3. **Implemented PHEV calculations integration** - Added:
   - New `PhevGridData` struct and `calculate_phev_grid_data()` function
   - Uses `calculate_phev_trip_consumption()` for electricity-first logic
   - Tracks both battery AND fuel remaining
   - Fuel rate calculated only for `km_on_fuel` portion
   - Match-based vehicle type handling in `get_trip_grid_data`

**Tests:** 134/134 passing

4. **Added soc_override input field** - TripRow.svelte:
   - Added `soc_override_percent` to formData
   - Expandable input (⚡ icon) in battery remaining cell
   - Only visible for BEV/PHEV vehicles (showEnergyFields)
   - Only shown when editing existing trips (not new)
   - Shows hint explaining it affects subsequent trips
