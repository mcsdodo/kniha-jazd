**Date:** 2025-12-25
**Subject:** Year picker - standalone yearly logbooks
**Status:** Planning

## Problem

Each year is a standalone "kniha jázd" (vehicle logbook) for legal purposes. Currently the app shows all trips across all years with no separation.

## Requirements

1. Year picker in header next to vehicle dropdown
2. App starts on current calendar year
3. Stats (km, fuel, consumption, margin) are year-scoped
4. Export dropdown only shows years with data
5. ODO carries over from previous year, zostatok starts fresh (full tank)

## Design

### Data Layer

**New backend command:**
```rust
#[tauri::command]
pub fn get_years_with_trips(db: State<Database>, vehicle_id: String) -> Result<Vec<i32>, String>
// Query: SELECT DISTINCT strftime('%Y', date) FROM trips WHERE vehicle_id = ? ORDER BY 1 DESC
```

**Modified commands (add year parameter):**
- `get_trip_grid_data(vehicle_id, year)` - filter trips by year
- `calculate_trip_stats(vehicle_id, year)` - stats scoped to year

**Frontend API additions:**
- `getYearsWithTrips(vehicleId): Promise<number[]>`
- Update existing functions with optional year parameter

### Header UI

```
[Kniha Jázd] [Kniha jázd | Nastavenia]     [Vozidlo: dropdown] [Rok: dropdown]
```

- Year dropdown styled identically to vehicle dropdown
- Shows: current year + years with data (deduplicated, descending)
- Only visible when vehicle is selected

### State Management

**New store:** `selectedYearStore`
- Initialized to `new Date().getFullYear()` on app start
- Persists during session (doesn't reset on page navigation)
- Resets to current year when switching vehicles

### Main Page Changes

- All data fetches pass selected year
- Stats reflect selected year only
- Empty year shows empty TripGrid (can still add trips)
- New trip date defaults to selected year

### Export in Settings

- Fetch years with data dynamically
- Only show years that have trips
- Independent of header year picker (user picks which year to export)
- If no data: show "Žiadne dáta na export"

### Edge Cases

**ODO across years:**
- First trip of year uses last ODO from previous year
- Fallback: `vehicle.initial_odometer` if no previous year data

**Zostatok across years:**
- Each year starts with full tank assumption
- Matches "standalone logbook" concept - no fuel carry-over

## Files to Change

| File | Change |
|------|--------|
| `src-tauri/src/commands.rs` | Add `get_years_with_trips`, update commands with year param |
| `src-tauri/src/lib.rs` | Register new command |
| `src/lib/api.ts` | Add `getYearsWithTrips`, update signatures |
| `src/lib/stores/year.ts` | NEW - `selectedYearStore` |
| `src/routes/+layout.svelte` | Add year dropdown next to vehicle |
| `src/routes/+page.svelte` | Pass year to all fetches |
| `src/routes/settings/+page.svelte` | Dynamic export year list |
