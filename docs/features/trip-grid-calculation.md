# Feature: Trip Grid Calculation

> Backend engine that computes consumption rates, fuel/battery remaining, and warnings for the trip grid display, with specialized calculation paths for ICE, BEV, and PHEV vehicles.

## User Flow

1. User opens the **Trips** tab for a vehicle
2. Frontend calls `get_trip_grid_data(vehicle_id, year)`
3. Backend returns pre-calculated `TripGridData` with:
   - All trips for the year (sorted by `sort_order` for display)
   - Consumption rates (l/100km or kWh/100km) per trip
   - Fuel/battery remaining after each trip
   - Warnings for consumption limits, date ordering, missing receipts
   - Year-start values (odometer, fuel, battery) carried over from previous year
4. Frontend renders the grid with rates, tank levels, and warning indicators

## Technical Implementation

### Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         get_trip_grid_data()                                │
│                      (Main Orchestrator - commands.rs)                      │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        │                           │                           │
        ▼                           ▼                           ▼
┌───────────────────┐   ┌───────────────────┐   ┌───────────────────────────┐
│  Year Carryover   │   │  Trip Sorting     │   │  Vehicle Type Dispatch    │
│  - Odometer       │   │  - chronological  │   │                           │
│  - Fuel remaining │   │  - by sort_order  │   │   ┌─────────────────────┐ │
│  - Battery (kWh)  │   │                   │   │   │ ICE:                │ │
└───────────────────┘   └───────────────────┘   │   │ calculate_period_   │ │
                                                │   │ rates() +           │ │
                                                │   │ calculate_fuel_     │ │
                                                │   │ remaining()         │ │
                                                │   └─────────────────────┘ │
                                                │   ┌─────────────────────┐ │
                                                │   │ BEV:                │ │
                                                │   │ calculate_energy_   │ │
                                                │   │ grid_data()         │ │
                                                │   └─────────────────────┘ │
                                                │   ┌─────────────────────┐ │
                                                │   │ PHEV:               │ │
                                                │   │ calculate_phev_     │ │
                                                │   │ grid_data()         │ │
                                                │   └─────────────────────┘ │
                                                └───────────────────────────┘
                                                            │
                                                            ▼
                                                ┌───────────────────────────┐
                                                │   TripGridData Response   │
                                                │   - trips[]               │
                                                │   - rates{}               │
                                                │   - fuel_remaining{}      │
                                                │   - energy_rates{}        │
                                                │   - battery_remaining_*{} │
                                                │   - *_warnings{}          │
                                                │   - year_start_odometer   │
                                                └───────────────────────────┘
```

### ICE Vehicles

For Internal Combustion Engine vehicles, calculation uses **period-based fuel consumption**:

**Period Concept:**
- A "period" spans from one **full tank fill-up** to the next
- All trips within a period share the same calculated consumption rate
- Formula: `rate = (liters_in_period / km_in_period) × 100`

**Algorithm (`calculate_period_rates`):**
```
For each trip (chronologically):
  1. Add trip to current period
  2. Accumulate: km_in_period += trip.distance_km
  3. If trip has fuel: fuel_in_period += fuel_liters
  4. If trip.full_tank AND km > 0:
     → Close period: rate = (fuel / km) × 100
     → Assign rate to all trips in period
     → Reset for next period

Open period (no closing full tank yet):
  → Use TP rate (estimated, marked in estimated_rates)
```

**Fuel Remaining (`calculate_fuel_remaining`):**
```
fuel = initial_fuel (from year carryover)
For each trip (chronologically):
  1. fuel_used = (distance × rate) / 100
  2. fuel -= fuel_used
  3. If fuel added:
     - If full_tank: fuel = tank_size
     - Else: fuel += fuel_liters
  4. Clamp to [0, tank_size]
  5. Store fuel_remaining[trip_id] = fuel
```

### BEV Vehicles

For Battery Electric Vehicles, similar period logic applies but for **energy (kWh)**:

**Period Concept:**
- A "period" spans from one **full charge** to the next
- All trips in period share the calculated energy rate
- Formula: `rate = (kWh_in_period / km_in_period) × 100`

**Algorithm (`calculate_energy_grid_data`):**
```
For each trip (chronologically):
  1. Handle SoC override (manual battery reset)
  2. Calculate: energy_used = distance × baseline_rate / 100
  3. Update battery: battery = battery - used + charged
  4. Track period (km + kWh)
  5. If full_charge: close period, calculate rate

Open period:
  → Use baseline_consumption_kwh (estimated)
```

**Key Differences from ICE:**
- No legal consumption limit (no margin warning)
- SoC override allows manual battery state correction
- Battery shown in both kWh and percentage

### PHEV Vehicles

Plug-in Hybrids are the most complex — they track **both fuel AND battery**:

**Critical Behavior: Electricity First**
> PHEV depletes electricity BEFORE fuel. This is counterintuitive but realistic:
> drivers typically use electric mode for short trips, switching to fuel only
> when battery is empty.

**Algorithm (`calculate_phev_grid_data` + `calculate_phev_trip_consumption`):**
```
For each trip:
  1. Add charged energy first (before driving)
  2. Calculate total energy needed for entire distance
  3. Use electricity first (limited by available battery):
     - energy_from_battery = min(energy_needed, battery_available)
     - km_on_electricity = energy_from_battery / rate × 100
  4. Remaining distance uses fuel:
     - km_on_fuel = distance - km_on_electricity
     - fuel_used = km_on_fuel × tp_consumption / 100
  5. Update both tanks

Period tracking:
  - Fuel periods close on full_tank
  - Energy periods close on full_charge
  - Fuel rate only counts km_on_fuel (not total km!)
```

**Split Calculation Example:**
```
Trip: 100 km, battery at 10 kWh, rate 20 kWh/100km
  → Energy needed: 100 × 20 / 100 = 20 kWh
  → Battery can provide: 10 kWh
  → km_on_electricity: 10 / 20 × 100 = 50 km
  → km_on_fuel: 100 - 50 = 50 km
```

## Key Concepts

### Chronological vs Sort Order

The system maintains **two different orderings** for trips:

| Ordering | Purpose | Sorted By |
|----------|---------|-----------|
| **Chronological** | Calculations (fuel/battery flow) | `date`, then `odometer` |
| **Sort Order** | Display (newest first) | `sort_order` field (0 = top) |

**Why both?**
- Fuel/battery must flow in time order (can't use fuel before buying it)
- Users expect newest trips at the top for quick entry
- Date warnings detect when display order doesn't match chronological order

### Year Carryover

Each year inherits state from the previous year:

| Value | Source | Fallback |
|-------|--------|----------|
| **Odometer** | Last trip's odometer from prev year | `vehicle.initial_odometer` |
| **Fuel** | Last trip's fuel_remaining from prev year | `tank_size` (full tank) |
| **Battery** | Last trip's battery_remaining from prev year | `initial_battery_percent × capacity` |

Carryover is **recursive** — if no data in (year-1), checks (year-2), etc.

### 20% Margin Limit (Legal Compliance)

Slovak regulations allow deducting fuel expenses only if consumption stays within **120% of the vehicle's TP (technical passport) rate**.

```
limit = tp_consumption × 1.2
margin_percent = (actual_rate / tp_rate - 1.0) × 100

Example: TP = 7.0 l/100km, Actual = 8.0 l/100km
  → margin = (8.0/7.0 - 1) × 100 = 14.3%  ✓ OK
  → limit = 7.0 × 1.2 = 8.4 l/100km

Example: TP = 7.0 l/100km, Actual = 9.0 l/100km
  → margin = (9.0/7.0 - 1) × 100 = 28.6%  ✗ WARNING
```

Trips exceeding the limit are flagged in `consumption_warnings`.

### Estimated vs Calculated Rates

| Type | Condition | Source |
|------|-----------|--------|
| **Calculated** | Period closed by full tank/charge | Actual consumption |
| **Estimated** | Open period (no closing fill-up yet) | TP rate or baseline |

Estimated trips are tracked in `estimated_rates` / `estimated_energy_rates` sets for UI styling (typically shown in italic or different color).

### SoC Override

For BEV/PHEV, users can manually set battery percentage when:
- Charging at home without tracking kWh
- Starting a new period without prior data
- Correcting accumulated calculation drift

When `soc_override_percent` is set:
```rust
current_battery = capacity × override_percent / 100.0
```

This resets the battery state, breaking the chain of calculations.

## Key Files

| File | Purpose |
|------|---------|
| [commands.rs#L819-985](src-tauri/src/commands.rs) | `get_trip_grid_data()` — main orchestrator |
| [commands.rs#L1076-1230](src-tauri/src/commands.rs) | `calculate_period_rates()`, `calculate_fuel_remaining()` |
| [commands.rs#L1231-1344](src-tauri/src/commands.rs) | `calculate_energy_grid_data()` for BEV |
| [commands.rs#L1345-1479](src-tauri/src/commands.rs) | `calculate_phev_grid_data()` for PHEV |
| [commands.rs#L668-780](src-tauri/src/commands.rs) | Year carryover functions |
| [calculations.rs](src-tauri/src/calculations.rs) | Core fuel math (rates, margins, buffer km) |
| [calculations_energy.rs](src-tauri/src/calculations_energy.rs) | Battery math (kWh ↔ %, remaining) |
| [calculations_phev.rs](src-tauri/src/calculations_phev.rs) | PHEV split calculation (electricity first) |
| [models.rs#L305-340](src-tauri/src/models.rs) | `TripGridData` struct definition |

## Design Decisions

### Why Period-Based Rates Instead of Per-Trip?

**Problem:** You can't know exact fuel consumption for each individual trip.

**Solution:** Use fill-up-to-fill-up periods. When you fill up to full tank, you know exactly how much fuel was used for the distance since last full tank.

**Trade-off:** All trips in a period share the same rate, even if driving conditions varied.

### Why Electricity First for PHEV?

**Real-world behavior:** PHEV drivers typically:
1. Charge overnight (cheap electricity)
2. Use electric mode for commute (no fuel cost)
3. Switch to fuel only when battery depleted
4. Fuel is "backup" for longer trips

**Accounting benefit:** Maximizes electric km (lower cost), fuel rate only applies to fuel-driven km, giving accurate l/100km for the combustion portion.

### Why Recursive Year Carryover?

**Problem:** User might skip a year (vehicle not used), then resume.

**Solution:** Look back up to 10 years to find the last known state.

**Example:** Vehicle last used in 2023, now 2025:
- 2025 start → looks at 2024 → empty → looks at 2023 → found!
- Uses 2023's ending odometer/fuel/battery

### Why Separate Chronological and Display Order?

**Problem:** Users want newest trips at top (quick entry), but calculations need time order.

**Solution:** 
- `sort_order` field for display (0 = top = newest)
- Sort by date+odometer for calculations
- `date_warnings` flag trips where these orders conflict

This allows users to manually reorder trips while the system warns if the order becomes illogical.
