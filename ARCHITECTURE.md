# Architecture

This document describes the technical architecture of Kniha Jazd (Vehicle Logbook).

## System Overview

```
+-------------------------------------------------------------+
|                    SvelteKit Frontend                       |
|              (Display-only, zero calculations)              |
|  +---------+  +----------+  +-----------+                   |
|  | Logbook |  | Receipts |  | Settings  |   3 routes        |
|  +----+----+  +----+-----+  +-----+-----+                   |
|       +------------+--------------+                         |
|                    | invoke()                               |
+--------------------+----------------------------------------+
                     |
              Tauri IPC (~0.1ms local)
                     |
+--------------------v----------------------------------------+
|                  Rust Backend                               |
|  +----------+ +--------------+ +-------------+              |
|  |commands  | |calculations  | | suggestions |              |
|  |(36 cmds) | |(pure funcs)  | |(route match)|              |
|  +----+-----+ +--------------+ +-------------+              |
|       |                                                     |
|  +----v-----+ +----------+ +---------+                      |
|  |  db.rs   | | export   | | gemini  |                      |
|  |(Mutex<C>)| | (HTML)   | | (OCR)   |                      |
|  +----+-----+ +----------+ +---------+                      |
+-------+-----------------------------------------------------+
        |
     SQLite
  (vehicles, trips, routes, receipts, settings)
```

## Core Principle: Backend-Only Calculations (ADR-008)

All business logic lives in the Rust backend. The frontend is display-only.

```rust
// commands.rs - The "aggregator" pattern
pub fn get_trip_grid_data(db: State<Database>, vehicle_id: String, year: i32)
    -> Result<TripGridData, String>
{
    let vehicle = db.get_vehicle(&vehicle_id)?;
    let trips = db.get_trips_for_vehicle_in_year(&vehicle_id, year)?;

    // ALL calculations happen here, in Rust
    let (rates, estimated_rates) = calculate_period_rates(&trips, vehicle.tp_consumption);
    let fuel_remaining = calculate_fuel_remaining(&trips, &rates, vehicle.tank_size_liters);
    let date_warnings = calculate_date_warnings(&trips);
    let consumption_warnings = calculate_consumption_warnings(&trips, &rates, vehicle.tp_consumption);

    Ok(TripGridData {
        trips,
        rates,              // HashMap<trip_id, f64>
        estimated_rates,    // HashSet<trip_id>
        fuel_remaining,     // HashMap<trip_id, f64>
        date_warnings,      // HashSet<trip_id>
        consumption_warnings,
        missing_receipts,
    })
}
```

**Why this pattern?** Tauri IPC is local (microseconds), so computing everything server-side has negligible latency while providing a single source of truth for legally-sensitive calculations.

## Module Responsibilities

| Module | Responsibility | Pattern |
|--------|----------------|---------|
| `commands.rs` | IPC bridge, orchestration | 36 `#[tauri::command]` handlers |
| `calculations.rs` | Pure business logic | Stateless functions, 28 tests |
| `db.rs` | SQLite CRUD | `Mutex<Connection>` singleton |
| `suggestions.rs` | Route matching algorithm | Filter + min_by for best match |
| `export.rs` | HTML generation | Template-based, i18n labels |
| `gemini.rs` | OCR integration | Gemini API for receipt parsing |
| `models.rs` | Data structures | Serde + typed enums |

## Data Model

### Core Entities

```
VEHICLES (1)
  |-- name, license_plate, tank_size_liters
  |-- tp_consumption (l/100km - legal reference)
  |-- initial_odometer
  |
  +--< TRIPS (N per vehicle)
        |-- date, origin, destination, distance_km
        |-- odometer (for validation)
        |-- purpose (business/personal)
        |-- fuel_liters (nullable - fillups only)
        |-- full_tank (1=full, 0=partial)
        |-- sort_order (manual ordering)
        |
        +--< RECEIPTS (0..1 per trip)
              |-- file_path (UNIQUE)
              |-- liters, total_price_eur (OCR)
              |-- receipt_date, station_name
              |-- status (Pending->Parsed->Assigned)
              |-- confidence (typed enum per field)

ROUTES (autocomplete cache, populated from trips)
SETTINGS (singleton: company_name, ico, buffer_trip_purpose)
```

### Key Pattern: Dual-Purpose Trip Records

A single `trips` row can represent a regular trip, a fuel fillup, or both:

```sql
-- Trip with fillup
INSERT INTO trips (distance_km, fuel_liters, full_tank)
VALUES (150, 45.5, 1);  -- Drove 150km, filled 45.5L

-- Just a trip (fuel_liters IS NULL)
-- Just a fillup (distance_km = 0, has fuel_liters)
```

### Consumption Calculation Spans Multiple Trips

```
Trip 1: 150km (no fuel)     -+
Trip 2: 200km (no fuel)      +- Period: 500km total
Trip 3: 150km + 35L fillup  -+
                              -> Rate: 35L / 500km * 100 = 7.0 l/100km
```

The `full_tank` flag is critical - partial fillups don't close a period.

## Frontend Architecture

### Routes (3 pages, shared layout)

```
src/routes/
  +layout.svelte      # Vehicle selector, year picker, nav
  +page.svelte        # Logbook (trip CRUD)
  doklady/+page.svelte    # Receipts
  settings/+page.svelte   # Config, backups
```

### State Management (Minimal Svelte Stores)

```typescript
// src/lib/stores/
vehiclesStore      // writable<Vehicle[]>
activeVehicleStore // writable<Vehicle|null>
selectedYearStore  // writable<number>
receiptRefreshTrigger // writable<number> - signaling counter
toast, confirmStore   // UI state
```

### IPC Pattern (Single Entry Point)

```typescript
// src/lib/api.ts
export async function getTripGridData(vehicleId: string, year: number): Promise<TripGridData> {
    return await invoke('get_trip_grid_data', { vehicleId, year });
}
```

All 36 backend commands are wrapped here. Snake_case (Rust) -> camelCase (TS) conversion.

## Business Logic

### Core Formulas (`calculations.rs`)

```rust
// Consumption rate: liters per 100km
pub fn calculate_consumption_rate(liters: f64, km: f64) -> f64 {
    (liters / km) * 100.0
}

// Margin: how much over/under the TP (technical passport) rate
pub fn calculate_margin_percent(actual_rate: f64, tp_rate: f64) -> f64 {
    (actual_rate / tp_rate - 1.0) * 100.0  // e.g., 15% over
}

// Legal limit: must stay <= 120% of TP rate (margin <= 20%)
pub fn is_within_legal_limit(margin_percent: f64) -> bool {
    margin_percent <= 20.0 + EPSILON
}

// Fuel remaining after trip
pub fn calculate_zostatok(previous: f64, spotreba: f64, fuel_added: Option<f64>, tank_size: f64) -> f64 {
    let new_zostatok = previous - spotreba + fuel_added.unwrap_or(0.0);
    new_zostatok.min(tank_size).max(0.0)  // Clamp to valid range
}
```

### Compensation Suggestion Algorithm (`suggestions.rs`)

When over the 20% limit, suggest a "buffer trip" to dilute the margin:

```rust
pub fn find_matching_route(routes: &[Route], target_km: f64) -> Option<&Route> {
    let tolerance = 0.10; // +/-10%
    routes.iter()
        .filter(|r| r.distance_km >= target_km * 0.9 && r.distance_km <= target_km * 1.1)
        .min_by(|a, b| (a.distance_km - target_km).abs().partial_cmp(...))
}
```

## Testing Strategy

72 backend tests, zero frontend tests (frontend is display-only):

```bash
cd src-tauri && cargo test

# Distribution:
# calculations.rs - 28 tests (consumption, margin, zostatok)
# suggestions.rs  - 9 tests (route matching)
# db.rs          - 10 tests (CRUD lifecycle)
# commands.rs    - 10 tests (receipt matching)
# export.rs      - 7 tests (HTML escaping, totals)
# receipts.rs    - 3 tests (extraction)
# gemini.rs      - 3 tests (JSON parsing)
# settings.rs    - 3 tests (loading)
```

## Quick Reference: Where to Look

| You want to... | Look at... |
|----------------|------------|
| Add a new calculation | `calculations.rs` -> expose via `commands.rs` |
| Add a new Tauri command | `commands.rs` + register in `lib.rs` |
| Change the grid display | `TripGrid.svelte` + `TripRow.svelte` |
| Modify the data model | `models.rs` + `db.rs` + migrations |
| Add UI text | `src/lib/i18n/sk/index.ts` (Slovak primary) |
| Understand fuel logic | `calculate_period_rates()` in `commands.rs` |
| See architectural decisions | `DECISIONS.md` |
