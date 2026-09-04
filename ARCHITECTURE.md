# Architecture

This document describes the technical architecture of Kniha Jazd (Vehicle Logbook).

The app ships as a single Docker image: one Rust process serving a static SvelteKit
SPA and a JSON-RPC endpoint over HTTP, against one SQLite file on a `/data` volume.
There is no desktop build.

## System Overview

```
+-------------------------------------------------------------+
|                    SvelteKit Frontend                       |
|              (Display-only, zero calculations)              |
|  +---------+ +----------+ +----------+ +----------+         |
|  | Logbook | | Receipts | | Map      | | Settings |         |
|  +----+----+ +----+-----+ +----+-----+ +----+-----+         |
|       +-----------+------------+------------+               |
|                   | apiCall()                               |
+-------------------+-----------------------------------------+
                    |
        HTTP: POST /api/rpc { command, args }
                    |
+-------------------v-----------------------------------------+
|  kniha-jazd-web  -  Axum server, static files, /health       |
+-------------------+-----------------------------------------+
                    |
+-------------------v-----------------------------------------+
|  kniha-jazd-core  -  all business logic                     |
|  +------------+ +--------------+ +-------------+            |
|  |  server/   | |calculations/ | | suggestions |            |
|  |dispatcher  | |(pure funcs)  | |(route match)|            |
|  +----+-------+ +--------------+ +-------------+            |
|       |                                                     |
|  +----v-----+ +----------+ +---------+                      |
|  |  db.rs   | | export   | | gemini  |                      |
|  |(Mutex<C>)| | (HTML)   | | (OCR)   |                      |
|  +----+-----+ +----------+ +---------+                      |
+-------+-----------------------------------------------------+
        |
     SQLite  (on the /data volume)
  (vehicles, trips, routes, receipts, settings)
```

## Core Principle: Backend-Only Calculations (ADR-008)

All business logic lives in the Rust backend. The frontend is display-only.

```rust
// commands_internal/statistics.rs - The "aggregator" pattern
pub fn build_trip_grid_data(db: &Database, vehicle_id: &str, year: i32)
    -> Result<TripGridData, String>
{
    let vehicle = db.get_vehicle(vehicle_id)?;
    let trips = db.get_trips_for_vehicle_in_year(vehicle_id, year)?;

    // ALL calculations happen here, in Rust
    let (rates, estimated_rates) = calculate_period_rates(&trips, vehicle.tp_consumption);
    let fuel_remaining = calculate_fuel_remaining(&trips, &rates, vehicle.tank_size_liters);
    let consumption_warnings = calculate_consumption_warnings(&trips, &rates, vehicle.tp_consumption);

    Ok(TripGridData {
        trips,
        rates,              // HashMap<trip_id, f64>
        estimated_rates,    // HashSet<trip_id>
        fuel_remaining,     // HashMap<trip_id, f64>
        consumption_warnings,
        missing_receipts,
    })
}
```

Trip order is fixed by `start_datetime DESC` (with `created_at` ASC as tiebreaker) — see [ADR-022](./DECISIONS.md). There is no separate display order, and no date-warning calculation (removed in [Task 65](./_tasks/_done/65-datetime-is-order/)).

**Why this pattern?** The RPC round-trip is same-host (or one LAN hop), so computing everything server-side has negligible latency while providing a single source of truth for legally-sensitive calculations. It is also what makes the browser the only client the app needs.

## Module Responsibilities

Two crates in the `src-tauri/` workspace: `kniha-jazd-core` (everything below) and
`kniha-jazd-web` (a `main.rs` that reads env vars and starts the server). Paths are
relative to `src-tauri/core/src/`.

| Module | Responsibility | Pattern |
|--------|----------------|---------|
| `server/mod.rs` | Axum router, `/api/rpc`, `/health`, CORS, static SPA | One RPC endpoint, not 80 REST routes |
| `server/dispatcher.rs` | Command name -> `*_internal` fn | 68 sync commands, via `spawn_blocking` |
| `server/dispatcher_async.rs` | Async commands | 12 (OCR, HA, export, grid data) |
| `commands_internal/` | Orchestration per domain | Plain fns taking `&Database` / `&AppState` |
| `calculations/` | Pure business logic | Stateless functions |
| `db.rs` | SQLite CRUD | `Mutex<Connection>` singleton |
| `suggestions.rs` | Route matching algorithm | Filter + min_by for best match |
| `export.rs` | HTML generation | Template-based, i18n labels |
| `gemini.rs` | OCR integration | Gemini API for receipt parsing |
| `paperless.rs` | Paperless-ngx client | `impl Invoice for PaperlessDoc` |
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
        |-- start_datetime, end_datetime (drive ordering)
        |-- origin, destination, distance_km
        |-- odometer (for validation)
        |-- purpose (business/personal)
        |-- fuel_liters (nullable - fillups only)
        |-- full_tank (1=full, 0=partial)
        |-- created_at (same-datetime tiebreaker)
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

### Routes (4 pages, shared layout)

```
src/routes/
  +layout.svelte          # Vehicle selector, year picker, nav
  +page.svelte            # Logbook (trip CRUD)
  doklady/+page.svelte    # Receipts
  mapa/+page.svelte       # Route maps
  settings/+page.svelte   # Config, backups
```

Built with `adapter-static` into `build/`, which the Rust server serves from
`STATIC_DIR`. In local dev `STATIC_DIR` is left unset and vite serves the SPA
instead, proxying `/api` to the backend on port 3456.

### State Management (Minimal Svelte Stores)

```typescript
// src/lib/stores/
vehiclesStore      // writable<Vehicle[]>
activeVehicleStore // writable<Vehicle|null>
selectedYearStore  // writable<number>
receiptRefreshTrigger // writable<number> - signaling counter
toast, confirmStore   // UI state
```

### RPC Pattern (Single Entry Point)

```typescript
// src/lib/api-adapter.ts - the only place that talks to the network
export async function apiCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const response = await fetch('/api/rpc', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-KJ-Client': '1' },
        body: JSON.stringify({ command, args: args ?? {} }),
    });
    if (!response.ok) throw new Error(await response.text());
    return response.json();
}

// src/lib/api.ts - one typed wrapper per command
export async function getTripGridData(vehicleId: string, year: number): Promise<TripGridData> {
    return apiCall('get_trip_grid_data', { vehicleId, year });
}
```

Every backend command is wrapped in `api.ts`. Snake_case (Rust) -> camelCase (TS) conversion.

## Business Logic

### Core Formulas (`calculations/mod.rs`)

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

Two layers, no frontend unit tests (the frontend is display-only):

```bash
# Backend: every business rule, all in kniha-jazd-core's *_tests.rs companions
npm run test:backend
# = cargo test --manifest-path src-tauri/Cargo.toml --workspace

# Integration: Chrome against the real HTTP server, verifying UI flows only
npm run test:integration
```

Integration tests run in two shapes from the same
[wdio.server.conf.ts](./tests/integration/wdio.server.conf.ts): WebdriverIO either
spawns `kniha-jazd-web` itself (default, port 3457) or drives an already-running
container (`WDIO_EXTERNAL_SERVER=1`, port 3456 - what CI uses).

## Quick Reference: Where to Look

| You want to... | Look at... |
|----------------|------------|
| Add a new calculation | `calculations/mod.rs` -> expose via `commands_internal/` |
| Add a new command | `commands_internal/` + register in `server/dispatcher.rs` |
| Change the grid display | `TripGrid.svelte` + `TripRow.svelte` |
| Modify the data model | `models.rs` + `db.rs` + migrations |
| Add UI text | `src/lib/i18n/sk/index.ts` (Slovak primary) |
| Understand fuel logic | `calculate_period_rates()` in `commands_internal/statistics.rs` |
| Change deployment / env vars | `Dockerfile.web`, `docker-compose.web.yml`, `src-tauri/web/src/main.rs` |
| See architectural decisions | `DECISIONS.md` |
