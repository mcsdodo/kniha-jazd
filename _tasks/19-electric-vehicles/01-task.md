**Date:** 2026-01-01
**Subject:** Add Electric Vehicle (BEV) and Plug-in Hybrid (PHEV) Support
**Status:** Planning

---

# Task: Electric Vehicle Support

## Overview

Add support for Battery Electric Vehicles (BEV) and Plug-in Hybrid Electric Vehicles (PHEV) to the trip log application. This enables users to track electricity consumption alongside or instead of fuel consumption.

## Background

- Slovak legislation does not define TP consumption for EVs (only power in kW)
- The 20% margin rule does not apply to electricity (no baseline exists)
- PHEVs must track both fuel AND electricity, using one documentation method per year
- See [research.md](./research.md) for full legislation analysis

## Requirements

### Functional Requirements

1. **Vehicle Types**
   - Support three vehicle types: ICE, BEV, PHEV
   - ICE behavior unchanged (existing functionality)
   - BEV tracks electricity only (kWh)
   - PHEV tracks both fuel (liters) and electricity (kWh)

2. **Vehicle Settings (New Fields)**
   - `vehicle_type`: Ice | Bev | Phev
   - `battery_capacity_kwh`: Battery size in kWh (BEV, PHEV)
   - `baseline_consumption_kwh`: User-defined kWh/100km (BEV, PHEV)
   - `initial_battery_percent`: Starting battery % for first record (optional, default 100%)

3. **Trip Tracking (New Fields)**
   - `energy_kwh`: Energy charged during trip
   - `energy_cost_eur`: Cost of charging
   - `full_charge`: Whether charged to 100%
   - `soc_override_percent`: Manual battery % override for degradation tracking (optional)

4. **Consumption Logic**
   - BEV: Same formula as ICE, just kWh instead of liters
   - PHEV: Use electricity first until battery depleted, then fuel
   - No margin calculation for electricity (no legal requirement)
   - PHEV fuel portion still has 20% margin rule

5. **Display**
   - Show battery remaining in both kWh and % (derived)
   - Conditional columns based on vehicle type
   - PHEV shows both fuel and energy columns

### Non-Functional Requirements

- All existing ICE tests must pass unchanged
- Parallel implementation - don't break fuel when adding energy
- Database migration must be backward compatible

### Constraints

1. **Vehicle type is immutable** - Once trips exist for a vehicle, `vehicle_type` cannot be changed
2. **Year boundaries with carryover** - Fuel/battery state carries over between years (consistent with existing ICE behavior)
3. **Initial state for first year** - If no previous year data, use `initial_battery_percent` (default 100%) for BEV/PHEV
4. **PHEV compensation out of scope** - Compensation suggestions for PHEVs deferred (see `_TECH_DEBT/02-phev-compensation-suggestions.md`)

## Out of Scope

- Charging receipt scanning (future enhancement)
- Charging station integration/API
- Regular hybrids (HEV) - they behave like ICE

## Acceptance Criteria

- [ ] User can create BEV vehicle with battery capacity and baseline consumption
- [ ] User can create PHEV vehicle with both fuel and battery settings
- [ ] User can set optional initial battery % for BEV/PHEV
- [ ] BEV trips track energy charged and show battery remaining
- [ ] PHEV trips correctly split consumption between electricity and fuel
- [ ] No margin warnings for electricity consumption
- [ ] PHEV shows margin warnings for fuel portion only
- [ ] Vehicle type cannot be changed after trips exist (UI disabled, backend rejects)
- [ ] User can set SoC override on trip (visible only in edit form)
- [ ] Trips with SoC override show indicator in grid
- [ ] Year boundary: first trip uses carryover from previous year (or initial_battery_percent/100% for first year)
- [ ] Export (PDF) correctly shows energy data for BEV/PHEV
- [ ] All existing ICE functionality unchanged
- [ ] All existing tests pass

## Related Documents

- [research.md](./research.md) - Slovak legislation and accounting research
- [technical-analysis.md](./technical-analysis.md) - Implementation design and code structure
- [02-plan.md](./02-plan.md) - Phased implementation plan
- [PHEV Compensation Tech Debt](../_TECH_DEBT/02-phev-compensation-suggestions.md) - Deferred feature

## Estimated Effort

- **Phase 1 (Foundation):** Models + calculations - Medium
- **Phase 2 (BEV):** Full BEV support - Medium
- **Phase 3 (PHEV):** Dual-fuel support - Medium-High
- **Total:** ~3-4 focused sessions
