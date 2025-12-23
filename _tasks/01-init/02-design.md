**Date:** 2024-12-23
**Subject:** Kniha jázd - Vehicle Logbook Desktop App
**Status:** Complete

---

# Design Document

## Overview

Desktop application to replace Excel-based vehicle logbook (kniha jázd) for Slovak legal compliance. Tracks trips, fuel consumption, and ensures the 20% over-consumption margin is maintained.

## Tech Stack

| Component | Technology |
|-----------|------------|
| Frontend | SvelteKit + TypeScript |
| Backend | Tauri (Rust) |
| Database | SQLite (local file) |
| PDF Export | Rust library (genpdf or printpdf) |
| i18n | svelte-i18n |

## Data Model

```
Vehicle
├── id: UUID
├── name: String
├── license_plate: String
├── tank_size_liters: f64 (e.g., 66)
├── tp_consumption: f64 (e.g., 5.1 l/100km)
├── is_active: bool
├── created_at: DateTime
└── updated_at: DateTime

Trip
├── id: UUID
├── vehicle_id: UUID (FK)
├── date: Date
├── origin: String
├── destination: String
├── distance_km: f64
├── odometer: f64
├── purpose: String
├── fuel_liters: Option<f64> (if set → fill-up)
├── fuel_cost_eur: Option<f64>
├── other_costs_eur: Option<f64>
├── other_costs_note: Option<String>
├── created_at: DateTime
└── updated_at: DateTime

Route (for autocomplete + distance memory)
├── id: UUID
├── vehicle_id: UUID (FK)
├── origin: String
├── destination: String
├── distance_km: f64
├── usage_count: i32
└── last_used: DateTime

Settings
├── id: UUID
├── company_name: String
├── company_ico: String
├── buffer_trip_purpose: String (default: "testovanie")
└── updated_at: DateTime
```

### Calculated Fields (runtime, not stored)

| Field | Formula |
|-------|---------|
| spotreba_l | `km × current_consumption_rate / 100` |
| zostatok_l | `previous_zostatok - spotreba + fuel_liters` |
| l_per_100km | On fill-up: `fuel_liters / km_since_last_fillup × 100` |
| margin_percent | `(l_per_100km / tp_consumption - 1) × 100` |
| buffer_km | When over margin: km needed to reach target (16-19%) |

## Core Calculations

### Consumption Rate (l/100km)

```rust
fn calculate_consumption_rate(liters: f64, km_since_last_fillup: f64) -> f64 {
    (liters / km_since_last_fillup) * 100.0
}
```

### Fuel Used Per Trip (Spotreba)

```rust
fn calculate_spotreba(distance_km: f64, consumption_rate: f64) -> f64 {
    distance_km * consumption_rate / 100.0
}
```

### Remaining Fuel (Zostatok)

```rust
fn calculate_zostatok(
    previous_zostatok: f64,
    spotreba: f64,
    fuel_added: Option<f64>,
    tank_size: f64
) -> f64 {
    let new_zostatok = previous_zostatok - spotreba + fuel_added.unwrap_or(0.0);
    new_zostatok.min(tank_size) // Cap at tank size
}
```

### Margin Check

```rust
fn calculate_margin_percent(consumption_rate: f64, tp_rate: f64) -> f64 {
    (consumption_rate / tp_rate - 1.0) * 100.0
}

fn is_within_legal_limit(margin_percent: f64) -> bool {
    margin_percent <= 20.0
}
```

### Compensation Suggestion

```rust
fn calculate_buffer_km(
    liters_filled: f64,
    km_driven: f64,
    tp_rate: f64,
    target_margin: f64  // Random 0.16-0.19
) -> f64 {
    let target_rate = tp_rate * (1.0 + target_margin);
    let required_km = (liters_filled * 100.0) / target_rate;
    required_km - km_driven
}
```

## UI Design

### Main View (Trip Grid)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ [Škoda Octavia BA123XY ▼]                    Zostatok: 43.5L │ Spotreba: 6.02│
├─────────────────────────────────────────────────────────────────────────────┤
│ [+ Nový záznam]                                              [⚙ Nastavenia] │
├──────────┬─────────────────┬─────────────────┬──────┬────────┬──────┬───────┤
│ Dátum    │ Odkiaľ          │ Kam             │  Km  │ Účel   │ PHM  │ Cena€ │
├──────────┼─────────────────┼─────────────────┼──────┼────────┼──────┼───────┤
│ [inline editing row for new entry]                                          │
├──────────┼─────────────────┼─────────────────┼──────┼────────┼──────┼───────┤
│ 10.10.25 │ Nejaka Ulica 123│ Ina Ulica 456   │  370 │ navrat │65.49 │ 96.20 │
│ 08.10.25 │ Nejaka Ulica 123│ Nejaka Ulica 123│   65 │ test   │   -  │   -   │
│ ...                                                                          │
└──────────┴─────────────────┴─────────────────┴──────┴────────┴──────┴───────┘
```

### Key UI Behaviors

1. **Inline editing** - Click "Nový záznam" → empty row appears at top, edit in place
2. **Autocomplete** - Origin/destination fields suggest from history, ranked by frequency
3. **Auto-fill km** - When known route selected, distance auto-fills
4. **Fill-up detection** - If liters entered, row is treated as fill-up
5. **Margin warning** - On fill-up over 20%, show inline warning with suggestion
6. **Compensation button** - "Pridať" creates pre-filled compensation trip

### Compensation Suggestion UI

When fill-up exceeds 20% margin:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ⚠ Spotreba 6.35 l/100km (+24.5%)                                            │
│                                                                             │
│ Návrh: Nejaka Ulica 123 → Nejaka Ulica 123, 42 km                          │
│        "testovanie"                                          [Pridať]      │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Settings Page

```
Nastavenia
├── Vozidlá
│   ├── List of vehicles (add/edit/delete)
│   └── Set active vehicle
│
├── Firma
│   ├── Názov spoločnosti
│   └── IČO
│
├── Kompenzácia
│   └── Text účelu (default: "testovanie")
│
└── Export
    └── Exportovať PDF (yearly)
```

## PDF Export

Yearly export matching Excel structure:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Príklad s.r.o., IČO: 12 345 678                                             │
│ Škoda Octavia Combi BA123XY                                                 │
│ Veľkosť nádrže: 66L, Spotreba TP: 5.1 l/100km                              │
├──────────┬─────────────┬─────────────┬──────┬────────┬──────┬───────┬──────┤
│ Dátum    │ Štart       │ Cieľ        │ ODO  │ Účel   │ PHM  │ Cena  │ l/km │
├──────────┼─────────────┼─────────────┼──────┼────────┼──────┼───────┼──────┤
│ 01.01.25 │ -           │ -           │38057 │ počiat.│  -   │   -   │ 5.10 │
│ 13.01.25 │ Elektraren. │ Mlynske N.  │38427 │ SC     │  -   │   -   │  -   │
│ ...                                                                          │
├──────────┼─────────────┼─────────────┼──────┼────────┼──────┼───────┼──────┤
│ SPOLU    │             │             │16857 │        │1011L │1550€  │ 6.00 │
└──────────┴─────────────┴─────────────┴──────┴────────┴──────┴───────┴──────┘
```

- Chronological order (oldest first)
- Summary row with totals
- Matches legal requirements

## Project Structure

```
kniha-jazd/
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs               # Tauri entry, command handlers
│   │   ├── db.rs                 # SQLite connection, migrations
│   │   ├── models.rs             # Vehicle, Trip, Route, Settings
│   │   ├── calculations.rs       # Core business logic
│   │   ├── suggestions.rs        # Compensation trip logic
│   │   └── export.rs             # PDF generation
│   ├── migrations/
│   │   └── 001_initial.sql       # Schema creation
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── src/                          # SvelteKit frontend
│   ├── lib/
│   │   ├── components/
│   │   │   ├── TripGrid.svelte
│   │   │   ├── TripRow.svelte
│   │   │   ├── Autocomplete.svelte
│   │   │   ├── VehicleSelector.svelte
│   │   │   └── CompensationSuggestion.svelte
│   │   ├── stores/
│   │   │   ├── trips.ts
│   │   │   ├── vehicles.ts
│   │   │   └── settings.ts
│   │   ├── i18n/
│   │   │   ├── sk.json
│   │   │   └── en.json
│   │   └── api.ts                # Tauri invoke wrappers
│   ├── routes/
│   │   ├── +page.svelte          # Main grid view
│   │   ├── +layout.svelte        # App shell
│   │   └── settings/
│   │       └── +page.svelte      # Settings page
│   └── app.html
│
├── static/                       # Static assets
├── tests/                        # E2E tests (Playwright)
├── package.json
├── svelte.config.js
├── vite.config.ts
├── CLAUDE.md
├── DECISIONS.md
└── _tasks/                       # Planning docs
```

## Testing Strategy

### Focus: Business Logic (Rust)

```rust
// calculations.rs tests
#[cfg(test)]
mod tests {
    // Consumption rate
    - normal case: 50L / 820km = 6.097 l/100km
    - edge: very short distance
    - edge: first trip (no previous data)

    // Spotreba
    - uses previous fillup's rate
    - edge: no fillups yet → uses TP rate

    // Zostatok
    - decreases with trips
    - increases on fillup (capped at tank_size)
    - edge: near zero

    // Margin
    - under limit: 6.0 / 5.1 = +17.6% ✓
    - at limit: 6.12 / 5.1 = +20.0% ✓
    - over limit: 6.5 / 5.1 = +27.5% ✗
}

// suggestions.rs tests
#[cfg(test)]
mod tests {
    // Buffer km calculation
    - calculates correct km for target margin

    // Route matching
    - finds existing route within ±10%
    - falls back to filler when no match

    // Target variation
    - produces values in 16-19% range
}
```

### Integration Tests

```rust
- full cycle: 5 trips → fillup → verify all fields
- compensation accepted → recalculates → margin OK
- PDF export structure verification
```

### E2E Tests (Playwright)

```
- add trip flow
- fillup shows margin warning
- accept compensation suggestion
- vehicle switch filters trips
```

### What We Don't Test

- Trivial CRUD (SQLite handles it)
- UI rendering (unless behavior-critical)
- Getters/setters

## Implementation Order

1. **Rust core** - Models, DB schema, calculation logic with tests
2. **Tauri commands** - Expose CRUD + calculations to frontend
3. **SvelteKit shell** - Basic app structure, routing
4. **Trip grid** - Main view with inline editing
5. **Autocomplete** - Location suggestions + route memory
6. **Fill-up logic** - Detection, margin warning, suggestions
7. **Settings** - Vehicle management, company info
8. **PDF export** - Yearly report generation
9. **Polish** - i18n, error handling, edge cases
