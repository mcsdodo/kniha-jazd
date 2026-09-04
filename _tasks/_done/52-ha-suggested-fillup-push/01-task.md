# Task 52: Push Suggested Fillup to Home Assistant Sensor

## Summary

When the trip grid computes a suggested fillup (legend_suggested_fillup), push its text to a configurable Home Assistant sensor entity. This enables HA automations/dashboards to show the current fillup recommendation.

## Requirements

1. **New per-vehicle field**: `ha_fillup_sensor` — configurable HA entity ID (e.g., `sensor.kniha_jazd_fillup`)
2. **Sensor state format**: `"20.39 L → 5.66 l/100km"` when suggestion exists, `""` when no suggestion
3. **Push timing**: On every `get_trip_grid_data` call, as a background task (non-blocking)
4. **Error handling**: Silent fire-and-forget (log errors, don't block UI)
5. **Uses existing HA credentials**: `ha_url` and `ha_api_token` from `local.settings.json`

## HA REST API

```
POST {ha_url}/api/states/{entity_id}
Authorization: Bearer {token}
Content-Type: application/json

{
  "state": "20.39 L → 5.66 l/100km",
  "attributes": {
    "friendly_name": "Kniha jázd - Návrh tankovania",
    "icon": "mdi:gas-station"
  }
}
```

## Changes

### Backend (Rust)
- Migration: `ALTER TABLE vehicles ADD COLUMN ha_fillup_sensor TEXT DEFAULT NULL`
- Model: Add `ha_fillup_sensor` to Vehicle, VehicleRow, NewVehicleRow
- Schema: Add `ha_fillup_sensor` to vehicles table
- Integrations: Add `push_ha_sensor_state()` helper
- Statistics: Modify `get_trip_grid_data` to spawn background push after computing grid data

### Frontend (Svelte)
- types.ts: Add `haFillupSensor` to Vehicle interface
- VehicleModal: Add fillup sensor input field (next to ODO sensor)
- Settings page: Pass `haFillupSensor` through vehicle save flow
- i18n: Add SK/EN labels for new field
