**Date:** 2026-02-12
**Subject:** Show real fuel level from Home Assistant alongside computed zostatok
**Status:** Planning

## Goal

Display real fuel level from a Home Assistant sensor inline on the zostatok stats line in the vehicle info section. The HA value appears in yellow after the computed value: `45.3 L (HA: 42.0 L)`.

## Requirements

1. New vehicle field `ha_fuel_level_sensor` for configuring the HA entity ID (e.g., `sensor.car_fuel_level`)
2. HA sensor returns percentage (0-100%) — convert to liters: `percentage x tankSize / 100`
3. Display inline on the existing zostatok line in yellow (same accent color as Realne ODO)
4. Reuse existing `fetch_ha_odo` backend command (it's a generic HA sensor fetcher)
5. Extend `haStore` to cache fuel level alongside ODO on the same 5-min refresh interval
6. Show error state if sensor is configured but fetch fails (follow ODO error pattern)
7. Only show when HA is configured AND vehicle has `haFuelLevelSensor` set

## Technical Notes

- Existing pattern: `haOdoCache` in `+page.svelte` (lines 255-272) shows Realne ODO in yellow
- `fetch_ha_odo` command in `integrations.rs` is generic — accepts any `sensor_id`, returns `Option<f64>`
- `haStore` in `src/lib/stores/homeAssistant.ts` has caching + 5-min periodic refresh
- Vehicle Modal already has HA sensor section (lines 138-160 in VehicleModal.svelte)
- Migration follows pattern from `2026-02-11-100000_add_vehicle_ha_fillup_sensor`
- No backend calculation needed — conversion (% to L) is simple display math, not business logic
