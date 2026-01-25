# Feature: Multi-Year State

> Seamless year-over-year vehicle logbook management with automatic carryover of odometer, fuel, and battery state.

## User Flow

1. **Select Vehicle** — User chooses a vehicle from the dropdown in the header
2. **Select Year** — Year picker appears showing years with existing trips plus current year (defaults to current year on app start and vehicle switch)
3. **View Trips** — Grid displays trips filtered to the selected year
4. **Year Carryover** — Starting odometer, fuel, and battery levels automatically carry over from previous year's ending state
5. **Add/Edit Trips** — Trip year is derived from the trip date (selected year only filters the UI)
6. **Switch Years** — User can navigate between years; data recalculates based on carryover rules
7. **Receipts/Verification** — Receipts list, verification, and header badges are filtered by the selected year
8. **Export/Print** — Export and print use the selected year

## Technical Implementation

### Year Carryover

The system maintains continuity across years by calculating the ending state of the previous year and using it as the starting state for the current year.

**Fuel Carryover (ICE/PHEV):** `commands.rs:L668` `get_year_start_fuel_remaining()`
- Gets trips from previous year (year - 1)
- If no previous year data, returns full tank (`tank_size`)
- Sorts previous year's trips chronologically
- Calculates consumption rates using `calculate_period_rates()`
- Computes fuel remaining after each trip via `calculate_fuel_remaining()`
- Returns the last trip's fuel remaining as the year-end state
- **Recursive**: Chains back through multiple years to find initial state

**Battery Carryover (BEV/PHEV):** `commands.rs:L717` `get_year_start_battery_remaining()`
- Gets trips from previous year
- If no previous year data, returns `initial_battery_percent * capacity` (default: 100%)
- Processes SoC overrides and energy consumption
- Returns year-end battery level in kWh
- **Non-recursive for battery**: If previous year has no trips, returns initial battery and does not search earlier years

**Odometer Carryover:** `commands.rs:L782` `get_year_start_odometer()`
- Searches up to 10 years back for trips
- Returns the last trip's odometer from the most recent previous year
- Falls back to `vehicle.initial_odometer` if no historical data

### Vehicle Management

**Vehicle Type:** `models.rs:L17` `VehicleType` enum
- `Ice` — Internal combustion engine
- `Bev` — Battery electric vehicle
- `Phev` — Plug-in hybrid electric vehicle

**Immutability Rule:** `commands.rs:L186` `update_vehicle()`
- Vehicle type cannot be changed once trips exist
- Prevents data inconsistency between fuel-based and energy-based calculations
- Returns error message if user attempts to change type after trips recorded

**Type-Specific Fields:**
| Field | ICE | BEV | PHEV |
|-------|-----|-----|------|
| `tank_size_liters` | Yes | — | Yes |
| `tp_consumption` | Yes | — | Yes |
| `battery_capacity_kwh` | — | Yes | Yes |
| `baseline_consumption_kwh` | — | Yes | Yes |
| `initial_battery_percent` | — | Yes | Yes |

### Trip Ordering

Trips use explicit `sort_order` for manual reordering (drag-and-drop), separate from chronological date ordering.

**Two Orderings:**
1. **Display order** (`sort_order ASC`) — User-controlled, 0 = top/newest
2. **Chronological order** (date + odometer) — Used for calculations

**Database Schema:** Baseline migration defines `sort_order INTEGER NOT NULL DEFAULT 0` on trips table.

**Reorder Functions:**
- `db.rs:L405` `reorder_trip()` — Database-level reordering with transaction
- `commands.rs:L437` `reorder_trip` — Tauri command wrapper
- `db.rs:L462` `shift_trips_from_position()` — Shifts other trips to make room

**Reorder Behavior:**
- Uses database transaction for atomicity
- Shifts other trips up/down to make room
- Preserves date (only changes display order)
- New trips default to position 0 (top)

### Frontend State

**Stores:**
- `stores/vehicles.ts:L6-7` — `vehiclesStore` and `activeVehicleStore`
- `stores/year.ts:L4` — `selectedYearStore` (defaults to current year)
- `stores/year.ts:L7` — `resetToCurrentYear()` helper function

**Year Picker Population:** `+layout.svelte:L30` `loadYears()`
- Fetches years with existing trips via `getYearsWithTrips()`
- Combines with current year to ensure it's always available
- Sorts descending (newest first)
- Auto-selects most recent year with data if selected year has no trips

**Reactive Data Loading:** `+page.svelte:L61`
- Reactive statement watches `$activeVehicleStore` and `$selectedYearStore`
- Triggers `loadTrips()` when either changes

**Vehicle Change Handling:** `+layout.svelte:L145` `handleVehicleChange()`
- Sets active vehicle and resets to current year
- Prevents stale year selection when switching vehicles
- Reloads available years for the new vehicle

### Year Filtering Queries

**Trip Filtering:** `db.rs:L320` `get_trips_for_vehicle_in_year()`
- Uses `strftime('%Y', date)` to extract year from date
- Returns trips ordered by `sort_order ASC`
- Raw SQL query (Diesel's type-safe query builder doesn't support strftime)

**Years With Trips:** `db.rs:L345` `get_years_with_trips()`
- Returns distinct years that have trips for a vehicle
- Orders by year descending
- Used to populate the year picker dropdown

## Key Files

| File | Purpose |
|------|---------|
| `src-tauri/src/commands.rs` | `get_trip_grid_data()`, carryover functions, CRUD commands |
| `src-tauri/src/db.rs` | Year filtering queries, `reorder_trip()`, `shift_trips_from_position()` |
| `src-tauri/src/models.rs` | `Vehicle`, `Trip`, `TripGridData`, `VehicleType` |
| `src/lib/stores/vehicles.ts` | `activeVehicleStore`, `vehiclesStore` |
| `src/lib/stores/year.ts` | `selectedYearStore`, `resetToCurrentYear()` |
| `src/routes/+layout.svelte` | Header with vehicle/year selectors |
| `src/routes/+page.svelte` | Trip grid with reactive year loading |

## Design Decisions

1. **Partial Carryover** — Odometer searches back up to 10 years; fuel/battery only use previous year and reset when there are gaps.

2. **Vehicle Type Immutability** — Changing ICE/BEV/PHEV would invalidate all historical calculations (fuel vs energy). Enforced at backend level.

3. **Separate Sort Order vs Date** — Allows users to manually reorder trips (drag-drop) without affecting the chronological date used in calculations.

4. **Chronological for Calculations** — All consumption/remaining calculations use date+odometer ordering, not display order, ensuring correct fuel/energy tracking.

5. **Year Picker Auto-Population** — Shows current year plus all years with data, auto-selects most recent year with data when the selected year has no trips (including initial load).

6. **Reset Year on Vehicle Switch** — Prevents confusion when switching vehicles that may have different year ranges of data.

7. **TripGridData as Single Source of Truth** — Backend pre-calculates rates, remaining fuel/battery, and warnings. Frontend renders without duplicating calculation logic.
