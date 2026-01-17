**Date:** 2025-12-25
**Subject:** Remove frontend calculation duplication
**Status:** Complete

## Problem

Frontend duplicates Rust backend calculations unnecessarily:
- `src/lib/calculations.ts` mirrors `src-tauri/src/calculations.rs`
- ~500 lines of duplicate code
- 21 frontend tests duplicating 41 backend tests
- Risk of logic divergence

## Why Duplication Was Wrong

1. **Tauri IPC is fast** - local process communication, not network latency
2. **No other clients** - single desktop app, no API consumers
3. **Double maintenance** - every change needs updating in two places
4. **Divergence risk** - frontend/backend could calculate differently

## Solution

1. Delete `src/lib/calculations.ts` and `src/lib/calculations.test.ts`
2. Add Tauri command returning pre-calculated grid data
3. Frontend becomes pure display logic

## New Tauri Command

```rust
#[tauri::command]
pub fn get_trip_grid_data(
    db: State<Database>,
    vehicle_id: String,
) -> Result<TripGridData, String> {
    // Returns trips + all calculated values
}

struct TripGridData {
    trips: Vec<Trip>,
    rates: HashMap<String, f64>,           // tripId -> l/100km
    estimated_rates: HashSet<String>,       // tripIds using TP rate
    fuel_remaining: HashMap<String, f64>,   // tripId -> zostatok
    date_warnings: HashSet<String>,         // tripIds with date issues
    consumption_warnings: HashSet<String>,  // tripIds over 120% TP
}
```

## Files to Change

| File | Action |
|------|--------|
| `src/lib/calculations.ts` | DELETE |
| `src/lib/calculations.test.ts` | DELETE |
| `src/lib/components/TripGrid.svelte` | Remove imports, call Tauri command |
| `src-tauri/src/commands.rs` | Add `get_trip_grid_data` command |
| `src-tauri/src/models.rs` | Add `TripGridData` struct |
| `src/lib/api.ts` | Add `getTripGridData()` function |

## Benefits

- Single source of truth (Rust)
- 41 existing Rust tests cover all logic
- Frontend simplified to display-only
- No divergence risk
