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

**Fuel Carryover (ICE/PHEV):**
```rust
// commands.rs - get_year_start_fuel_remaining()
fn get_year_start_fuel_remaining(
    db: &Database,
    vehicle_id: &str,
    year: i32,
    tank_size: f64,
    tp_consumption: f64,
) -> Result<f64, String>
```
- Gets trips from previous year (year - 1)
- If no previous year data → returns full tank (`tank_size`)
- Sorts previous year's trips chronologically
- Calculates consumption rates using `calculate_period_rates()`
- Computes fuel remaining after each trip via `calculate_fuel_remaining()`
- Returns the last trip's fuel remaining as the year-end state
- **Recursive**: Chains back through multiple years to find initial state

**Battery Carryover (BEV/PHEV):**
```rust
// commands.rs - get_year_start_battery_remaining()
fn get_year_start_battery_remaining(
    db: &Database,
    vehicle_id: &str,
    year: i32,
    vehicle: &Vehicle,
) -> Result<f64, String>
```
- Gets trips from previous year
- If no previous year data → returns `initial_battery_percent × capacity` (default: 100%)
- Processes SoC overrides and energy consumption
- Returns year-end battery level in kWh
- **Non-recursive for battery**: If previous year has no trips, returns initial battery and does not search earlier years

**Odometer Carryover:**
```rust
// commands.rs - get_year_start_odometer()
fn get_year_start_odometer(
    db: &Database,
    vehicle_id: &str,
    year: i32,
    initial_odometer: f64,
) -> Result<f64, String>
```
- Searches up to 10 years back for trips
- Returns the last trip's odometer from the most recent previous year
- Falls back to `vehicle.initial_odometer` if no historical data

### Vehicle Management

**Vehicle Type Enum:**
```rust
// models.rs
pub enum VehicleType {
    Ice,  // Internal combustion engine
    Bev,  // Battery electric vehicle
    Phev, // Plug-in hybrid electric vehicle
}
```

**Immutability Rule:**
Vehicle type cannot be changed once trips exist. This prevents data inconsistency:
```rust
// commands.rs - update_vehicle()
if existing.vehicle_type != vehicle.vehicle_type {
    let trips = db.get_trips_for_vehicle(&vehicle.id.to_string())?;
    if !trips.is_empty() {
        return Err("Cannot change vehicle type after trips have been recorded. \
            Vehicle type is immutable once data exists.".to_string());
    }
}
```

**Type-Specific Fields:**
| Field | ICE | BEV | PHEV |
|-------|-----|-----|------|
| `tank_size_liters` | ✓ | — | ✓ |
| `tp_consumption` | ✓ | — | ✓ |
| `battery_capacity_kwh` | — | ✓ | ✓ |
| `baseline_consumption_kwh` | — | ✓ | ✓ |
| `initial_battery_percent` | — | ✓ | ✓ |

### Trip Ordering

Trips use explicit `sort_order` for manual reordering (drag-and-drop), separate from chronological date ordering.

**Two Orderings:**
1. **Display order** (`sort_order ASC`) — User-controlled, 0 = top/newest
2. **Chronological order** (date + odometer) — Used for calculations

**Database Schema:**
```sql
ALTER TABLE trips ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
```

**Reorder Command:**
```rust
// commands.rs - reorder_trip()
pub fn reorder_trip(
    db: State<Database>,
    trip_id: String,
    new_sort_order: i32,
) -> Result<Vec<Trip>, String>
```
- Uses database transaction for atomicity
- Shifts other trips up/down to make room
- Preserves date (only changes display order)

**Insertion Logic:**
```rust
// create_trip() - insert_at_position parameter
let sort_order = if let Some(position) = insert_at_position {
    db.shift_trips_from_position(&vehicle_id, position)?;
    position
} else {
    // Default: insert at top (sort_order = 0)
    db.shift_trips_from_position(&vehicle_id, 0)?;
    0
};
```

### Frontend State

**Stores:**
```typescript
// src/lib/stores/vehicles.ts
export const vehiclesStore = writable<Vehicle[]>([]);
export const activeVehicleStore = writable<Vehicle | null>(null);

// src/lib/stores/year.ts
export const selectedYearStore = writable<number>(new Date().getFullYear());

export function resetToCurrentYear(): void {
    selectedYearStore.set(new Date().getFullYear());
}
```

**Year Picker Population:**
```typescript
// +layout.svelte - loadYears()
const yearsWithData = await getYearsWithTrips(activeVehicle.id);
const currentYear = new Date().getFullYear();
const allYears = new Set([currentYear, ...yearsWithData]);
availableYears = [...allYears].sort((a, b) => b - a);
```

**Reactive Data Loading:**
```svelte
<!-- +page.svelte -->
$: if ($activeVehicleStore && $selectedYearStore) {
    loadTrips(true);
}
```

**Vehicle Change Handling:**
```typescript
async function handleVehicleChange(event: Event) {
    await setActiveVehicle(vehicleId);
    activeVehicleStore.set(activeVehicle);
    resetToCurrentYear();  // Prevent stale year selection
    await loadYears();
}

**Initial Load Handling:**
- If the selected year has no data, the UI switches to the most recent year with trips.
```

### Year Filtering Queries

**Database Query (Raw SQL for strftime):**
```rust
// db.rs - get_trips_for_vehicle_in_year()
diesel::sql_query(
    "SELECT ... FROM trips
     WHERE vehicle_id = ? AND strftime('%Y', date) = ?
     ORDER BY sort_order ASC"
)
.bind::<Text, _>(vehicle_id)
.bind::<Text, _>(year.to_string())
```

**Years With Trips:**
```rust
// db.rs - get_years_with_trips()
diesel::sql_query(
    "SELECT DISTINCT CAST(strftime('%Y', date) AS INTEGER) as year
     FROM trips WHERE vehicle_id = ? ORDER BY year DESC"
)
```

## Key Files

| File | Purpose |
|------|---------|
| [src-tauri/src/commands.rs](src-tauri/src/commands.rs) | `get_trip_grid_data()`, carryover functions, CRUD commands |
| [src-tauri/src/db.rs](src-tauri/src/db.rs) | Year filtering queries, `reorder_trip()`, `shift_trips_from_position()` |
| [src-tauri/src/models.rs](src-tauri/src/models.rs) | `Vehicle`, `Trip`, `TripGridData`, `VehicleType` |
| [src/lib/stores/vehicles.ts](src/lib/stores/vehicles.ts) | `activeVehicleStore`, `vehiclesStore` |
| [src/lib/stores/year.ts](src/lib/stores/year.ts) | `selectedYearStore`, `resetToCurrentYear()` |
| [src/routes/+layout.svelte](src/routes/+layout.svelte) | Header with vehicle/year selectors |
| [src/routes/+page.svelte](src/routes/+page.svelte) | Trip grid with reactive year loading |

## Design Decisions

1. **Partial Carryover** — Odometer searches back up to 10 years; fuel/battery only use previous year and reset when there are gaps.

2. **Vehicle Type Immutability** — Changing ICE↔BEV↔PHEV would invalidate all historical calculations (fuel vs energy). Enforced at backend level.

3. **Separate Sort Order vs Date** — Allows users to manually reorder trips (drag-drop) without affecting the chronological date used in calculations.

4. **Chronological for Calculations** — All consumption/remaining calculations use date+odometer ordering, not display order, ensuring correct fuel/energy tracking.

5. **Year Picker Auto-Population** — Shows current year plus all years with data, auto-selects most recent year with data when the selected year has no trips (including initial load).

6. **Reset Year on Vehicle Switch** — Prevents confusion when switching vehicles that may have different year ranges of data.

7. **TripGridData as Single Source of Truth** — Backend pre-calculates rates, remaining fuel/battery, and warnings. Frontend renders without duplicating calculation logic.
