# Tech Debt: PHEV Compensation Suggestions

**Date:** 2026-01-01
**Priority:** Medium
**Effort:** High (1-3d)
**Component:** `src-tauri/src/suggestions.rs`
**Status:** Open

## Problem

The compensation suggestion system (buffer trips to reduce consumption margin) does not work correctly for PHEV vehicles due to the electricity-first consumption logic.

When a PHEV is over the 20% fuel margin and the system suggests adding buffer kilometers, the suggested trips may be driven on electricity (if battery is charged) rather than fuel, defeating the purpose of reducing the fuel consumption rate.

## Impact

- PHEV users cannot use the automatic compensation suggestion feature
- Must be handled manually (understanding electricity-first logic)
- Feature gap compared to ICE vehicles

## Root Cause

The existing `suggestions.rs` module was designed for ICE vehicles only. PHEVs have complex dual-fuel logic where:
1. Battery state determines whether a trip uses electricity or fuel
2. Compensation trips only help fuel margin if driven when battery is depleted
3. The order of charging vs driving matters

## Recommended Solution

Implement PHEV-aware compensation suggestions:

1. **Check battery state** - Only suggest buffer trips when battery is depleted (or will be depleted after planned trips)

2. **Calculate effective fuel distance** - Account for any electricity usage in the suggested trips

3. **Consider charging behavior** - If user typically charges daily, buffer trips may never use fuel

### Implementation Approach

```rust
// In suggestions.rs or new phev_suggestions.rs

pub fn calculate_phev_buffer_km(
    fuel_liters: f64,
    fuel_km_driven: f64,  // Only fuel portion of km
    tp_rate: f64,
    target_margin: f64,
    battery_state_kwh: f64,
    baseline_consumption_kwh: f64,
) -> Option<BufferSuggestion> {
    // 1. Calculate current fuel-only margin
    let fuel_rate = (fuel_liters / fuel_km_driven) * 100.0;
    let current_margin = (fuel_rate / tp_rate - 1.0) * 100.0;

    if current_margin <= target_margin * 100.0 {
        return None; // Already within limit
    }

    // 2. Calculate buffer km needed (fuel only)
    let target_rate = tp_rate * (1.0 + target_margin);
    let required_km = (fuel_liters * 100.0) / target_rate;
    let buffer_fuel_km = required_km - fuel_km_driven;

    // 3. Account for battery state
    if battery_state_kwh > 0.0 {
        // Battery would be used first
        let electric_range = battery_state_kwh / baseline_consumption_kwh * 100.0;
        // Total trip must exceed electric range for fuel portion to count
        return Some(BufferSuggestion {
            fuel_km_needed: buffer_fuel_km,
            electric_km_first: electric_range,
            total_km_needed: buffer_fuel_km + electric_range,
            note: "Batéria musí byť vybitá pred jazdou na palivo",
        });
    }

    Some(BufferSuggestion {
        fuel_km_needed: buffer_fuel_km,
        electric_km_first: 0.0,
        total_km_needed: buffer_fuel_km,
        note: None,
    })
}
```

### Files Affected

- `src-tauri/src/suggestions.rs` - Add PHEV-specific logic
- `src-tauri/src/commands.rs` - Extend `get_suggestions` for PHEV
- `src/lib/components/SuggestionPanel.svelte` - Show PHEV-specific guidance

## Alternative Options

### Option A: Disable for PHEV (Current Approach)
Simply don't show compensation suggestions for PHEV vehicles. Users must manage manually.

**Pros:** Simple, no development effort
**Cons:** Feature gap, poor UX for PHEV users

### Option B: Show with Warning
Show ICE-style suggestions with a warning that results depend on battery state.

**Pros:** Some guidance is better than none
**Cons:** May mislead users, inaccurate calculations

## Related

- Task: `_tasks/19-electric-vehicles/`
- Current suggestions: `src-tauri/src/suggestions.rs`
- PHEV calculations: `src-tauri/src/calculations_phev.rs` (to be created)

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-01 | Defer to future work | EV feature scope already large, compensation for PHEV is complex edge case |
