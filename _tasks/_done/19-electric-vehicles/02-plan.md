**Date:** 2026-01-01
**Subject:** Implementation Plan - Electric Vehicle Support
**Status:** Planning

---

# Implementation Plan

## Overview

Implement BEV and PHEV support in 3 phases, with each phase delivering working functionality. The key principle is **parallel implementation** - energy calculations are separate from fuel calculations to avoid breaking existing ICE functionality.

## Pre-Implementation

- [ ] Create feature branch: `feature/electric-vehicles`
- [ ] Verify all existing tests pass: `cargo test`

---

## Phase 1: Foundation (Models + Calculations)

**Goal:** Add data structures and calculation logic without changing any existing code paths.

### 1.1 Add VehicleType Enum

**File:** `src-tauri/src/models.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum VehicleType {
    #[default]
    Ice,
    Bev,
    Phev,
}
```

- [ ] Add enum definition
- [ ] Implement Display trait for UI
- [ ] Add to Vehicle struct as `vehicle_type: VehicleType`

### 1.2 Extend Vehicle Model

**File:** `src-tauri/src/models.rs`

Add optional fields (None for vehicle types that don't use them):

- [ ] `battery_capacity_kwh: Option<f64>`
- [ ] `baseline_consumption_kwh: Option<f64>`
- [ ] `initial_battery_percent: Option<f64>` - Initial SoC for first record (default: 100%)
- [ ] Make `tank_size_liters` and `tp_consumption` optional (will be None for BEV)

### 1.3 Extend Trip Model

**File:** `src-tauri/src/models.rs`

- [ ] `energy_kwh: Option<f64>` - Energy charged
- [ ] `energy_cost_eur: Option<f64>` - Charging cost
- [ ] `full_charge: bool` - Charged to target SoC
- [ ] `soc_override_percent: Option<f64>` - Manual SoC override for battery degradation
- [ ] Add `is_charge()` helper method
- [ ] Add `has_soc_override()` helper method

### 1.4 Create Energy Calculations Module

**File:** `src-tauri/src/calculations_energy.rs` (NEW)

- [ ] `calculate_consumption_rate_kwh(kwh, km) -> f64`
- [ ] `calculate_energy_used(distance_km, rate) -> f64`
- [ ] `calculate_battery_remaining(previous, used, charged, capacity) -> f64`
- [ ] `kwh_to_percent(kwh, capacity) -> f64`
- [ ] Add unit tests (minimum 7 tests from technical-analysis.md)

### 1.5 Create PHEV Calculations Module

**File:** `src-tauri/src/calculations_phev.rs` (NEW)

- [ ] Define `PhevTripConsumption` struct
- [ ] `calculate_phev_trip_consumption(...)` - electricity first, then fuel
- [ ] Add unit tests (minimum 4 tests from technical-analysis.md)

### 1.6 Database Migration

**File:** `src-tauri/migrations/YYYYMMDD_add_ev_support.sql` (NEW)

```sql
ALTER TABLE vehicles ADD COLUMN vehicle_type TEXT NOT NULL DEFAULT 'Ice';
ALTER TABLE vehicles ADD COLUMN battery_capacity_kwh REAL;
ALTER TABLE vehicles ADD COLUMN baseline_consumption_kwh REAL;
ALTER TABLE vehicles ADD COLUMN initial_battery_percent REAL;

ALTER TABLE trips ADD COLUMN energy_kwh REAL;
ALTER TABLE trips ADD COLUMN energy_cost_eur REAL;
ALTER TABLE trips ADD COLUMN full_charge INTEGER DEFAULT 0;
ALTER TABLE trips ADD COLUMN soc_override_percent REAL;

CREATE INDEX idx_vehicles_type ON vehicles(vehicle_type);
```

- [ ] Create migration file
- [ ] Update db.rs to read/write new fields
- [ ] Test migration on fresh and existing databases

### 1.7 Register New Modules

**File:** `src-tauri/src/main.rs`

- [ ] Add `mod calculations_energy;`
- [ ] Add `mod calculations_phev;`

### Phase 1 Verification

- [ ] `cargo test` - all existing tests pass
- [ ] New calculation tests pass
- [ ] App starts without errors
- [ ] Existing ICE vehicles work unchanged

---

## Phase 2: BEV Support

**Goal:** Full BEV functionality - create vehicle, track trips, see battery state.

### 2.1 Update Vehicle Commands

**File:** `src-tauri/src/commands.rs`

- [ ] `create_vehicle` - accept vehicle_type and battery fields
- [ ] `update_vehicle` - handle battery fields
- [ ] `update_vehicle` - **BLOCK vehicle_type change if trips exist** (immutable after first trip)
- [ ] `get_vehicle` - return new fields
- [ ] Add validation: BEV requires battery fields, ICE requires fuel fields

### 2.2 Update Trip Commands

**File:** `src-tauri/src/commands.rs`

- [ ] `create_trip` - accept energy fields
- [ ] `update_trip` - handle energy fields
- [ ] Validate: BEV trips should not have fuel_liters

### 2.3 Extend TripGridData

**File:** `src-tauri/src/models.rs`

Add to `TripGridData`:
- [ ] `energy_rates: HashMap<String, f64>`
- [ ] `battery_remaining_kwh: HashMap<String, f64>`
- [ ] `battery_remaining_percent: HashMap<String, f64>`
- [ ] `estimated_energy_rates: HashSet<String>`
- [ ] `soc_override_trips: HashSet<String>` - Trips with manual SoC override

### 2.4 Update get_trip_grid_data

**File:** `src-tauri/src/commands.rs`

- [ ] Check vehicle type before processing
- [ ] For BEV: calculate energy consumption instead of fuel
- [ ] For BEV: populate energy-related HashMaps
- [ ] For BEV: skip margin calculation

### 2.5 Add i18n Translations

**Files:** `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

Add Slovak (primary) and English translations for new UI elements:

- [ ] Vehicle type labels: "Typ vozidla", "Elektrické vozidlo (BEV)", "Plug-in hybrid (PHEV)"
- [ ] Battery fields: "Kapacita batérie (kWh)", "Základná spotreba (kWh/100km)", "Počiatočný stav batérie (%)"
- [ ] Trip fields: "Energia (kWh)", "Náklady na nabíjanie (€)", "Plné nabitie"
- [ ] Grid columns: "Batéria", "Spotreba (kWh/100km)"
- [ ] SoC override: "Korekcia stavu batérie (%)"

### 2.6 Vehicle Form UI

**File:** `src/lib/components/VehicleForm.svelte` (or similar)

- [ ] Add vehicle type selector (dropdown)
- [ ] **Disable vehicle type selector when editing vehicle with existing trips** (see test in Testing Strategy)
- [ ] Show/hide fields based on type:
  - ICE: tank + TP consumption
  - BEV: battery + baseline consumption + initial battery %
  - PHEV: all fields
- [ ] Validation: required fields per type
- [ ] Add optional "Počiatočný stav batérie %" field (default 100%)

### 2.7 Trip Form UI

**File:** `src/lib/components/TripForm.svelte` (or similar)

- [ ] For BEV: show energy fields instead of fuel fields
- [ ] Energy (kWh) input
- [ ] Energy cost (€) input
- [ ] Full charge checkbox
- [ ] SoC override (%) input - collapsible/hidden by default, only in edit form
- [ ] When SoC override is set, show info that this affects all subsequent trips

### 2.8 Trip Grid UI

**File:** `src/routes/+page.svelte` (or trip grid component)

- [ ] Conditional columns based on vehicle type
- [ ] For BEV: show Energy used, Battery remaining (kWh / %)
- [ ] For BEV: hide fuel columns
- [ ] For BEV: hide margin column
- [ ] Show indicator (🔧) next to battery % when trip has SoC override

### 2.9 Update Export

**File:** `src-tauri/src/export.rs`

- [ ] Check vehicle type in export
- [ ] BEV: show energy columns instead of fuel
- [ ] Update column headers for Slovak (kWh, Batéria, etc.)

### Phase 2 Verification

- [ ] Create BEV vehicle with battery settings
- [ ] Add trips with charging data
- [ ] Battery remaining calculates correctly
- [ ] Export shows energy data correctly
- [ ] ICE vehicles still work unchanged

---

## Phase 3: PHEV Support

**Goal:** Full PHEV functionality - dual fuel tracking, electricity-first logic.

### 3.1 PHEV Trip Processing

**File:** `src-tauri/src/commands.rs`

- [ ] For PHEV: use `calculate_phev_trip_consumption`
- [ ] Track both battery_remaining and fuel_remaining
- [ ] Calculate km_on_electricity and km_on_fuel

### 3.2 PHEV Margin Calculation

**File:** `src-tauri/src/commands.rs`

- [ ] Calculate margin for fuel portion only
- [ ] Use km_on_fuel (not total km) for fuel consumption rate
- [ ] Add consumption warning if fuel margin > 20%

### 3.3 Vehicle Form UI - PHEV

- [ ] When PHEV selected: show ALL fields
- [ ] Both tank + TP consumption
- [ ] Both battery + baseline consumption

### 3.4 Trip Form UI - PHEV

- [ ] Show both fuel AND energy sections
- [ ] All fields optional (trip may have charge, refuel, both, or neither)
- [ ] Clear visual separation between sections

### 3.5 Trip Grid UI - PHEV

- [ ] Show both fuel and energy columns
- [ ] Consider column grouping or tabs
- [ ] Show margin only for fuel (with clear indication)

### 3.6 PHEV Export

- [ ] Show both fuel and energy data
- [ ] Separate sections or columns for each fuel type
- [ ] Show km on electricity vs km on fuel

### Phase 3 Verification

- [ ] Create PHEV vehicle with all settings
- [ ] Add trip with charge only - uses electricity
- [ ] Add trip with no charge - continues on battery until depleted
- [ ] Add trip after battery depleted - uses fuel only
- [ ] Refuel and charge in same trip works
- [ ] Margin shows for fuel only
- [ ] Export shows both fuel types

---

## Phase 4: Polish & Edge Cases

### 4.1 Year Boundary Handling (CARRYOVER)

**Logic:** Fuel/battery carries over between years. Ideally end year with full tank/charge, but if not - carry over the remaining amount.

> **Note:** ICE carryover is already implemented via `get_year_start_fuel_remaining()` in `commands.rs`. BEV/PHEV follow the same pattern.

**BEV vehicles:**
- [ ] First trip of year: use previous year's ending battery kWh
- [ ] If no previous year data: use `initial_battery_percent × capacity` (default 100%)
- [ ] When switching year in year picker: recalculate from carryover state

**PHEV vehicles:**
- [ ] Carry over BOTH fuel AND battery from previous year
- [ ] Same logic as ICE for fuel, same as BEV for battery

**Implementation:**
```rust
fn get_year_start_state(vehicle_id, year) -> (Option<f64>, Option<f64>) {
    // (fuel_liters, battery_kwh)
    if year == first_year_with_trips {
        return (full_tank, full_battery);
    }
    // Calculate ending state of previous year
    let prev_year_trips = get_trips_for_vehicle_in_year(vehicle_id, year - 1);
    // Process all trips to get final zostatok
    // Note: If any trip has soc_override_percent, year-end must reflect that override
    return calculate_year_end_state(prev_year_trips);
}
```

### 4.2 SoC Override Processing

> **Note:** Core SoC override logic should be integrated into Phase 2.4 (`get_trip_grid_data`) to ensure feature works when UI is available. This section covers edge cases and verification.

- [ ] When trip has `soc_override_percent`, use it instead of calculated value
- [ ] Override applies to current trip AND all subsequent trips
- [ ] Add to `TripGridData.soc_override_trips` HashSet for UI indicator
- [ ] Verify override persists across year boundaries

### 4.3 Suggestions for PHEV

**OUT OF SCOPE** - See `_tasks/_TECH_DEBT/` for future implementation

Compensation suggestions for PHEV are complex due to electricity-first logic and deferred to future work.

### 4.4 Statistics

- [ ] Update TripStats for energy totals
- [ ] Cost comparison: fuel vs electricity

### 4.5 Edge Cases

- [ ] Zero battery capacity edge case
- [ ] Zero baseline consumption edge case
- [ ] Negative battery remaining (should clamp to 0)
- [ ] Battery > capacity (should clamp)
- [ ] SoC override validation (must be 0-100)

---

## Testing Strategy

### Unit Tests (Rust)

| Module | Tests |
|--------|-------|
| `calculations_energy.rs` | 7+ tests |
| `calculations_phev.rs` | 4+ tests |
| `db.rs` | CRUD for new fields |
| `commands.rs` | BEV/PHEV trip processing, vehicle type immutability |

**Required test for vehicle type immutability:**
- [ ] `test_update_vehicle_blocks_type_change_when_trips_exist` - Verify that `update_vehicle` returns error when attempting to change `vehicle_type` for a vehicle that has existing trips

### Validation Tests

- [ ] Vehicle type change blocked when trips exist
- [ ] SoC override must be 0-100
- [ ] initial_battery_percent must be 0-100
- [ ] BEV requires battery fields, rejects fuel fields
- [ ] Year boundary: first trip of year uses correct initial state

### Integration Tests

- [ ] BEV vehicle lifecycle (create → trips → export)
- [ ] PHEV vehicle lifecycle
- [ ] Migration on existing database
- [ ] ICE regression test

### Manual Testing

- [ ] Create each vehicle type
- [ ] Full trip sequence for each type
- [ ] Export verification
- [ ] Year picker works with EV vehicles

---

## Rollback Plan

If issues found after deployment:
1. Database migration is additive (no data loss)
2. New fields are all nullable/optional
3. Existing ICE vehicles continue to work
4. Can disable EV UI without backend changes

---

## Post-Implementation

- [ ] Run `/changelog` to update CHANGELOG.md
- [ ] Update README.md with EV feature description
- [ ] Consider `/decision` for key architectural choices made

---

*Plan created: 2026-01-01*
*Based on: [technical-analysis.md](./technical-analysis.md)*
