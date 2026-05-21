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

Trip order is derived purely from `start_datetime` (see [ADR-022](../../DECISIONS.md)). There is no separate display ordering — the chronological order *is* the display order.

**Ordering rule:**
1. Primary: `start_datetime DESC` (newest at top in the UI)
2. Tiebreaker for same datetime: `created_at` ASC, then `id` for full determinism

**Database Schema:** The `sort_order` column was dropped in migration [2026-05-21-100000_drop_sort_order](../../src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/). The baseline migration still defines it for historical reasons; the new migration removes it from the live table.

**No reorder API:** The `reorder_trip` Tauri command and `shift_trips_from_position` helper were removed (Task 65). The only way to change a trip's position is to change its datetime.

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

**Trip Filtering:** [`get_trips_for_vehicle_in_year()`](../../src-tauri/core/src/db.rs)
- Uses `strftime('%Y', start_datetime)` to extract year from the trip start
- Returns trips ordered by `start_datetime DESC`, with `created_at ASC` then `id` as tiebreakers
- Raw SQL query (Diesel's type-safe query builder doesn't support strftime)

**Years With Trips:** `db.rs:L345` `get_years_with_trips()`
- Returns distinct years that have trips for a vehicle
- Orders by year descending
- Used to populate the year picker dropdown

## Key Files

| File | Purpose |
|------|---------|
| [commands.rs](../../src-tauri/core/src/commands_internal/) | `get_trip_grid_data()`, carryover functions, CRUD commands |
| [db.rs](../../src-tauri/core/src/db.rs) | Year filtering queries (ordered by `start_datetime DESC`) |
| [models.rs](../../src-tauri/core/src/models.rs) | `Vehicle`, `Trip`, `TripGridData`, `VehicleType` |
| [vehicles.ts](../../src/lib/stores/vehicles.ts) | `activeVehicleStore`, `vehiclesStore` |
| [year.ts](../../src/lib/stores/year.ts) | `selectedYearStore`, `resetToCurrentYear()` |
| [+layout.svelte](../../src/routes/+layout.svelte) | Header with vehicle/year selectors |
| [+page.svelte](../../src/routes/+page.svelte) | Trip grid with reactive year loading |

## Design Decisions

1. **Partial Carryover** — Odometer searches back up to 10 years; fuel/battery only use previous year and reset when there are gaps.

2. **Vehicle Type Immutability** — Changing ICE/BEV/PHEV would invalidate all historical calculations (fuel vs energy). Enforced at backend level.

3. **Datetime Is The Only Order (see [ADR-022](../../DECISIONS.md))** — `start_datetime` drives both display and calculation order. Manual reordering was removed (Task 65) to make drift between "what the user sees" and "what the math uses" structurally impossible.

4. **Chronological for Calculations** — All consumption/remaining calculations use the same `start_datetime` order as the UI, so out-of-order red rows can no longer happen.

5. **Year Picker Auto-Population** — Shows current year plus all years with data, auto-selects most recent year with data when the selected year has no trips (including initial load).

6. **Reset Year on Vehicle Switch** — Prevents confusion when switching vehicles that may have different year ranges of data.

7. **TripGridData as Single Source of Truth** — Backend pre-calculates rates, remaining fuel/battery, and warnings. Frontend renders without duplicating calculation logic.
