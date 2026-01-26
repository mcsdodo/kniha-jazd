# Task: Trip Time Field + Hideable Columns

## Summary

Add optional departure time to trips and implement hideable columns UI pattern for the trip grid.

## User Stories

1. **As a user**, I want to record the departure time of my trips for legal/audit compliance
2. **As a user**, I want to hide columns I don't use to reduce visual clutter
3. **As a user**, I want to see clearly when columns are hidden so I don't forget about them
4. **As a user**, I want all columns exported regardless of visibility settings

## Requirements

### Time Field
- Add departure time to trips (stored as datetime with date)
- Display as separate column next to date
- Always show HH:MM format (including 00:00)
- HTML5 time picker for input

### Hideable Columns
- Candidates: time, fuel consumed (l), fuel left (zostatok), other costs (€), other costs note
- Toggle via dropdown in header bar
- Eye icon indicates state (open = all visible, crossed = some hidden)
- Badge shows count when columns are hidden
- Persist preference in local.settings.json
- All columns always included in export

## Out of Scope
- Arrival time (only departure time)
- Per-vehicle column visibility (global setting)
- Reordering columns
