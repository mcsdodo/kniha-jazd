# Live Preview Fuel Calculations

**Date:** 2026-01-01
**Subject:** Real-time fuel consumption and zostatok preview while editing trips
**Status:** Planning

## Problem

When adding or editing a trip, the user cannot see how the fuel consumption rate and remaining fuel (zostatok) will change until after saving. This makes it difficult to input data with precision when trying to stay within the 20% legal consumption margin.

## Requirements

1. **Live preview on every keystroke** when editing:
   - Zostatok (fuel remaining after this trip)
   - Consumption rate (l/100km) for the relevant fill-up window
   - Margin percentage over TP rate
   - Warning when margin exceeds 20%

2. **Backend calculation** - Use Tauri IPC for every keystroke (desktop app, IPC is microseconds)

3. **Visual distinction** - Preview values marked with `~` prefix and slightly faded to indicate "estimate"

4. **Full recalculation** - Inserting/editing a trip affects all subsequent fill-up rates; preview must account for this

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Where to calculate | Backend (Rust) | Reuse existing calculation logic, single source of truth |
| Debounce | None | Tauri IPC is local, can handle every keystroke |
| Preview styling | `~` prefix + opacity | Consistent with existing estimated rate indicator |
| Margin display | Background color + percentage | Shows exact margin for precision tuning |

## User Experience

```
Before: User types KM → saves → sees result → adjusts → saves again → repeat
After:  User types KM → instantly sees predicted values → adjusts → saves once
```

## Out of Scope

- Frontend-only calculation (would duplicate logic, violates ADR-008)
- Debounced preview (unnecessary for local IPC)
- Preview for other rows (only the row being edited)
