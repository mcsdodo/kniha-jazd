**Date:** 2026-01-13
**Subject:** Implementation Status - Electric Vehicle Support
**Status:** 🟡 Partial Implementation

---

# EV Feature Status

## Summary

The EV feature was partially implemented via PR #1, merged to main. Core infrastructure exists but **calculation integration is incomplete** - BEV/PHEV vehicles can be created but trip data doesn't show energy consumption.

## What's Done ✅

### Database Schema
- [x] `vehicles.vehicle_type` (Text: Ice/Bev/Phev)
- [x] `vehicles.battery_capacity_kwh` (Double, nullable)
- [x] `vehicles.baseline_consumption_kwh` (Double, nullable)
- [x] `vehicles.initial_battery_percent` (Double, nullable)
- [x] `trips.energy_kwh` (Double, nullable)
- [x] `trips.energy_cost_eur` (Double, nullable)
- [x] `trips.full_charge` (Integer, nullable)
- [x] `trips.soc_override_percent` (Double, nullable)

### Models (`models.rs`)
- [x] `VehicleType` enum (Ice, Bev, Phev)
- [x] Vehicle struct with EV fields
- [x] Trip struct with energy fields
- [x] Helper methods (exist but unused - cause warnings)

### Calculation Modules
- [x] `calculations_energy.rs` - BEV consumption formulas
- [x] `calculations_energy_tests.rs` - 7 tests
- [x] `calculations_phev.rs` - PHEV split logic
- [x] `calculations_phev_tests.rs` - 4 tests

### Commands (`commands.rs`)
- [x] `create_vehicle` - validates BEV/PHEV fields
- [x] `update_vehicle` - blocks vehicle_type change if trips exist
- [x] Energy calculation imports exist

### Frontend UI
- [x] `VehicleModal.svelte` - vehicle type selector
- [x] Conditional fuel/battery field display
- [x] Battery capacity input
- [x] Baseline consumption input
- [x] i18n translations for EV labels

## What's Missing ❌

### TripGrid Integration (CRITICAL)
- [ ] `get_trip_grid_data` doesn't call energy calculations for BEV/PHEV
- [ ] `TripGridData` missing energy-related HashMaps:
  - `energy_rates: HashMap<String, f64>`
  - `battery_remaining_kwh: HashMap<String, f64>`
  - `battery_remaining_percent: HashMap<String, f64>`
- [ ] BEV trips show empty consumption columns

### Trip Form UI
- [ ] `TripRow.svelte` - no energy input fields for BEV/PHEV
- [ ] No "Full charge" checkbox
- [ ] No SoC override input

### Export
- [ ] `export.rs` doesn't handle BEV/PHEV data
- [ ] No energy columns in HTML/PDF export

### Year Boundary
- [ ] `get_year_start_state` doesn't handle battery carryover
- [ ] BEV vehicles don't carry over battery state between years

## Dead Code Warnings (14 EV-related)

These are **not dead code** - they're unused due to incomplete integration:

| Warning | File | Reason |
|---------|------|--------|
| `calculate_consumption_rate_kwh` | calculations_energy.rs | Not called from get_trip_grid_data |
| `percent_to_kwh` | calculations_energy.rs | Not called |
| `PhevTripConsumption` fields | calculations_phev.rs | PHEV logic not integrated |
| `uses_fuel`, `uses_electricity` | models.rs | Helper methods not used |
| `new_ice`, `new_bev`, `new_phev` | models.rs | Convenience constructors not used |
| `is_charge`, `has_soc_override` | models.rs | Trip helpers not used |

**Action:** These warnings will resolve when integration is completed.

## Recommended Next Steps

1. **Phase 1: BEV TripGrid** (~2-4h)
   - Add energy HashMaps to TripGridData
   - Call energy calculations in get_trip_grid_data for BEV
   - Display energy columns in TripGrid

2. **Phase 2: BEV Trip Input** (~1-2h)
   - Add energy_kwh input to TripRow for BEV vehicles
   - Add full_charge checkbox

3. **Phase 3: PHEV Support** (~3-4h)
   - Implement electricity-first logic
   - Show both fuel and energy columns

4. **Phase 4: Export + Polish** (~2h)
   - Update export for BEV/PHEV
   - Year boundary handling

## Test Coverage

Existing tests (passing):
- `calculations_energy_tests.rs` - 7 tests for BEV formulas
- `calculations_phev_tests.rs` - 4 tests for PHEV split logic

Missing tests:
- Integration tests for BEV vehicle lifecycle
- PHEV margin calculation (fuel portion only)
