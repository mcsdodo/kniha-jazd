# Task: Home Assistant ODO Integration

## Summary

Integrate Home Assistant sensor to display real-time ODO (odometer) status in the app header, enabling users to detect forgotten trips by comparing real ODO with last logged trip.

## User Story

As a user with a Home Assistant instance tracking my car's odometer, I want to see the real ODO in the app header so I can quickly spot if I forgot to log any trips.

## Requirements

1. **Display real ODO in header** with delta showing difference from last logged trip
2. **Warning indicator** when delta ≥ 50 km (suggests forgotten trips)
3. **Global HA config** in Settings page (URL + API token)
4. **Per-vehicle sensor config** in vehicle modal (sensor entity ID)
5. **Periodic refresh** on startup + every 5 minutes
6. **Graceful failure** - show cached value with staleness, or hide if unavailable

## Out of Scope

- Auto-suggesting trips based on ODO gap
- Historical ODO tracking
- Multiple HA instances
- Other HA sensors (fuel level, location, etc.)

## Acceptance Criteria

- [ ] Real ODO shown in header when configured
- [ ] Delta calculated correctly (real_odo - last_trip_ending_odo)
- [ ] Warning color when delta ≥ 50 km
- [ ] Staleness indicator shown (e.g., "5m ago")
- [ ] HA credentials configurable in Settings
- [ ] Sensor entity configurable per vehicle
- [ ] Data cached and survives app restart
- [ ] Graceful handling when HA unavailable
