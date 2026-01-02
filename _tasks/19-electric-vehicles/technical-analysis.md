# Electric Vehicle Support - Technical Analysis

> **Focus:** App implementation for BEV and PHEV support.
> For legislation and accounting research, see [research.md](./research.md).

---

## 1. Design Decisions Summary

| Aspect | Decision |
|--------|----------|
| **Scope** | BEV + PHEV (no regular hybrids - they behave like ICE) |
| **EV margin tracking** | None - no legal requirement, skip entirely |
| **Battery tracking** | kWh primary, derive % for display |
| **PHEV consumption** | Electricity first (until depleted), then fuel |
| **Implementation** | Parallel systems - don't break fuel when adding energy |
| **Year boundaries** | 100% battery at year start and end (accounting convention) |
| **Vehicle type change** | **Prohibited** after trips exist - immutable once data is recorded |
| **SoC override** | Optional per-trip field for manual correction (battery degradation) |
| **PHEV compensation** | Out of scope - see `_tasks/_TECH_DEBT/` |

---

## 2. Consumption Logic

### Current ICE Logic (Unchanged)

```
For each trip:
  spotreba = distance_km × tp_consumption / 100
  zostatok = previous_zostatok - spotreba + fuel_added
  zostatok = clamp(zostatok, 0, tank_size)
```

### New BEV Logic

```
For each trip:
  energy_used = distance_km × baseline_consumption_kwh / 100
  battery_remaining = previous_battery - energy_used + energy_charged
  battery_remaining = clamp(battery_remaining, 0, battery_capacity)
```

**Same formula, different units.** No margin calculation for BEV.

### New PHEV Logic (Key Innovation)

PHEVs use electricity first, then fuel when battery depleted:

```
For each trip:
  // Calculate total energy needed for trip
  energy_needed = distance_km × baseline_consumption_kwh / 100

  // Use electricity first (from previous battery state)
  energy_from_battery = min(energy_needed, previous_battery)
  km_on_electricity = energy_from_battery / baseline_consumption_kwh × 100

  // Remaining distance uses fuel
  km_on_fuel = distance_km - km_on_electricity
  fuel_used = km_on_fuel × tp_consumption / 100

  // Update both tanks
  battery_remaining = previous_battery - energy_from_battery + energy_charged
  fuel_remaining = previous_fuel - fuel_used + fuel_added

  // Clamp both
  battery_remaining = clamp(battery_remaining, 0, battery_capacity)
  fuel_remaining = clamp(fuel_remaining, 0, tank_size)
```

### PHEV Example Walkthrough

```
Vehicle Settings:
  battery_capacity = 10 kWh
  baseline_consumption_kwh = 20 kWh/100km
  tank_size = 40 L
  tp_consumption = 6.0 L/100km

Trip 1: Charge 10 kWh, drive 60 km
  energy_needed = 60 × 20 / 100 = 12 kWh
  energy_from_battery = min(12, 10) = 10 kWh (all of it)
  km_on_electricity = 10 / 20 × 100 = 50 km
  km_on_fuel = 60 - 50 = 10 km
  fuel_used = 10 × 6.0 / 100 = 0.6 L

  battery_remaining = 10 - 10 + 10 = 10 kWh (charged back to full)
  fuel_remaining = 40 - 0.6 = 39.4 L

Trip 2: No charge, drive 30 km
  energy_needed = 30 × 20 / 100 = 6 kWh
  energy_from_battery = min(6, 10) = 6 kWh
  km_on_electricity = 30 km (all electric)
  km_on_fuel = 0 km

  battery_remaining = 10 - 6 = 4 kWh
  fuel_remaining = 39.4 L (unchanged)

Trip 3: No charge, drive 40 km
  energy_needed = 40 × 20 / 100 = 8 kWh
  energy_from_battery = min(8, 4) = 4 kWh (battery depleted)
  km_on_electricity = 4 / 20 × 100 = 20 km
  km_on_fuel = 40 - 20 = 20 km
  fuel_used = 20 × 6.0 / 100 = 1.2 L

  battery_remaining = 4 - 4 = 0 kWh (depleted)
  fuel_remaining = 39.4 - 1.2 = 38.2 L
```

---

## 3. Data Model Changes

### Vehicle Model

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VehicleType {
    Ice,   // Internal combustion engine (default, existing)
    Bev,   // Battery electric vehicle
    Phev,  // Plug-in hybrid electric vehicle
}

pub struct Vehicle {
    pub id: Uuid,
    pub name: String,
    pub license_plate: String,

    // Vehicle type (new)
    pub vehicle_type: VehicleType,

    // Fuel system (ICE + PHEV) - existing fields
    pub tank_size_liters: Option<f64>,    // None for BEV
    pub tp_consumption: Option<f64>,       // l/100km, None for BEV

    // Energy system (BEV + PHEV) - new fields
    pub battery_capacity_kwh: Option<f64>,      // None for ICE
    pub baseline_consumption_kwh: Option<f64>,  // kWh/100km, user-defined, None for ICE
    pub initial_battery_percent: Option<f64>,   // Initial SoC % for first record (BEV/PHEV)

    pub initial_odometer: f64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Field Requirements by Type:**

| Field | ICE | BEV | PHEV |
|-------|-----|-----|------|
| `tank_size_liters` | Required | None | Required |
| `tp_consumption` | Required | None | Required |
| `battery_capacity_kwh` | None | Required | Required |
| `baseline_consumption_kwh` | None | Required | Required |
| `initial_battery_percent` | None | Optional (default 100%) | Optional (default 100%) |

**Validation Rules:**
- `vehicle_type` is **immutable** once any trip exists for the vehicle
- `initial_battery_percent` must be 0-100 if provided

### Trip Model

```rust
pub struct Trip {
    pub id: Uuid,
    pub vehicle_id: Uuid,
    pub date: NaiveDate,
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub odometer: f64,
    pub purpose: String,

    // Fuel system (ICE + PHEV) - existing fields
    pub fuel_liters: Option<f64>,
    pub fuel_cost_eur: Option<f64>,
    pub full_tank: bool,

    // Energy system (BEV + PHEV) - new fields
    pub energy_kwh: Option<f64>,           // Energy charged
    pub energy_cost_eur: Option<f64>,      // Cost of charging
    pub full_charge: bool,                  // Charged to 100% (or target SoC)

    // SoC override for battery degradation tracking (BEV + PHEV)
    // When set, this value is used as the battery SoC "from now on" instead of calculated
    // Allows user to correct for battery degradation or measurement errors
    pub soc_override_percent: Option<f64>, // Manual SoC override (0-100)

    // Existing
    pub other_costs_eur: Option<f64>,
    pub other_costs_note: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Trip {
    pub fn is_fillup(&self) -> bool {
        self.fuel_liters.is_some()
    }

    pub fn is_charge(&self) -> bool {
        self.energy_kwh.is_some()
    }

    pub fn has_soc_override(&self) -> bool {
        self.soc_override_percent.is_some()
    }
}
```

**SoC Override Behavior:**
- When `soc_override_percent` is set, the trip is marked with an indicator in the UI
- The override value replaces calculated battery remaining for this trip and all subsequent trips
- Only visible in trip edit form (not in trip grid)
- Used for: battery degradation, measurement correction, mid-year battery replacement
- Grid shows small icon (e.g., 🔧) next to battery % when override is active

### Year Boundary Logic

Battery state handling at year boundaries:

```
Year Start (first trip of year):
  if vehicle.initial_battery_percent is set AND this is first year:
    battery_remaining = capacity × initial_battery_percent / 100
  else:
    battery_remaining = capacity (assume 100%)

Year End:
  Accounting convention: battery ends at 100%
  (No carry-over to next year - each year starts fresh at 100%)

When viewing previous year:
  Calculate from that year's first trip forward
  No dependency on current year's data
```

**Rationale:** Slovak accounting convention - similar to how fuel tank is assumed full at year end for tax purposes. Simplifies year-over-year tracking and avoids complex carry-over logic.

### TripStats Model

```rust
pub struct TripStats {
    // Fuel stats (ICE + PHEV)
    pub fuel_remaining_liters: Option<f64>,      // zostatok for fuel
    pub avg_consumption_rate_liters: Option<f64>,
    pub last_consumption_rate_liters: Option<f64>,
    pub margin_percent: Option<f64>,              // Only for ICE/PHEV fuel
    pub is_over_limit: bool,
    pub total_km: f64,
    pub total_fuel_liters: Option<f64>,
    pub total_fuel_cost_eur: Option<f64>,

    // Energy stats (BEV + PHEV) - new
    pub battery_remaining_kwh: Option<f64>,       // zostatok for battery
    pub battery_remaining_percent: Option<f64>,   // Derived: remaining / capacity × 100
    pub avg_consumption_rate_kwh: Option<f64>,
    pub total_energy_kwh: Option<f64>,
    pub total_energy_cost_eur: Option<f64>,
}
```

### TripGridData Model

```rust
pub struct TripGridData {
    pub trips: Vec<Trip>,

    // Fuel data (ICE + PHEV)
    pub fuel_rates: HashMap<String, f64>,           // l/100km per trip
    pub fuel_remaining: HashMap<String, f64>,       // Liters remaining per trip
    pub estimated_fuel_rates: HashSet<String>,      // Trips using TP rate
    pub consumption_warnings: HashSet<String>,      // Over 20% margin

    // Energy data (BEV + PHEV) - new
    pub energy_rates: HashMap<String, f64>,         // kWh/100km per trip
    pub battery_remaining_kwh: HashMap<String, f64>,
    pub battery_remaining_percent: HashMap<String, f64>,
    pub estimated_energy_rates: HashSet<String>,    // Trips using baseline rate
    pub soc_override_trips: HashSet<String>,        // Trips with manual SoC override

    // Shared
    pub date_warnings: HashSet<String>,
    pub missing_receipts: HashSet<String>,
}
```

---

## 4. Calculation Module Structure

### Parallel Systems Approach

```
src-tauri/src/
├── calculations.rs              # EXISTING - fuel/liters (untouched)
│   ├── calculate_consumption_rate()
│   ├── calculate_spotreba()
│   ├── calculate_zostatok()
│   ├── calculate_margin_percent()
│   └── is_within_legal_limit()
│
├── calculations_energy.rs       # NEW - energy/kWh
│   ├── calculate_consumption_rate_kwh()
│   ├── calculate_energy_used()
│   ├── calculate_battery_remaining()
│   └── // NO margin functions - not applicable
│
├── calculations_phev.rs         # NEW - PHEV combined logic
│   ├── calculate_phev_trip_consumption()
│   └── // Orchestrates fuel + energy based on battery state
│
└── trip_processor.rs            # NEW - routes to correct calculator
    └── process_trips_for_vehicle()
```

### calculations_energy.rs

```rust
//! Energy calculations for BEV and PHEV electricity tracking

/// Calculate energy consumption rate (kWh/100km) from a charge
/// Formula: (kwh / km_since_last_charge) * 100.0
pub fn calculate_consumption_rate_kwh(kwh: f64, km_since_last_charge: f64) -> f64 {
    if km_since_last_charge <= 0.0 {
        return 0.0;
    }
    (kwh / km_since_last_charge) * 100.0
}

/// Calculate energy used for a trip
/// Formula: distance_km * consumption_rate_kwh / 100.0
pub fn calculate_energy_used(distance_km: f64, consumption_rate_kwh: f64) -> f64 {
    distance_km * consumption_rate_kwh / 100.0
}

/// Calculate remaining battery (kWh)
/// Formula: previous - energy_used + energy_charged (capped at capacity)
pub fn calculate_battery_remaining(
    previous_kwh: f64,
    energy_used: f64,
    energy_charged: Option<f64>,
    battery_capacity: f64,
) -> f64 {
    let new_level = previous_kwh - energy_used + energy_charged.unwrap_or(0.0);
    new_level.min(battery_capacity).max(0.0)
}

/// Convert kWh to percentage of battery capacity
pub fn kwh_to_percent(kwh: f64, battery_capacity: f64) -> f64 {
    if battery_capacity <= 0.0 {
        return 0.0;
    }
    (kwh / battery_capacity) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consumption_rate_kwh_normal() {
        // 45 kWh / 250 km = 18.0 kWh/100km
        let rate = calculate_consumption_rate_kwh(45.0, 250.0);
        assert!((rate - 18.0).abs() < 0.01);
    }

    #[test]
    fn test_consumption_rate_kwh_zero_km() {
        let rate = calculate_consumption_rate_kwh(45.0, 0.0);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_energy_used_normal() {
        // 100 km at 18 kWh/100km = 18 kWh
        let used = calculate_energy_used(100.0, 18.0);
        assert!((used - 18.0).abs() < 0.01);
    }

    #[test]
    fn test_battery_remaining_normal() {
        // Start 60 kWh, use 15 kWh, no charge = 45 kWh
        let remaining = calculate_battery_remaining(60.0, 15.0, None, 75.0);
        assert!((remaining - 45.0).abs() < 0.01);
    }

    #[test]
    fn test_battery_remaining_with_charge() {
        // Start 20 kWh, use 10 kWh, charge 50 kWh = 60 kWh
        let remaining = calculate_battery_remaining(20.0, 10.0, Some(50.0), 75.0);
        assert!((remaining - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_battery_remaining_caps_at_capacity() {
        // Start 60 kWh, use 5 kWh, charge 30 kWh = would be 85, capped at 75
        let remaining = calculate_battery_remaining(60.0, 5.0, Some(30.0), 75.0);
        assert!((remaining - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_battery_remaining_floors_at_zero() {
        // Start 10 kWh, use 20 kWh = would be -10, capped at 0
        let remaining = calculate_battery_remaining(10.0, 20.0, None, 75.0);
        assert_eq!(remaining, 0.0);
    }

    #[test]
    fn test_kwh_to_percent() {
        // 45 kWh of 75 kWh capacity = 60%
        let percent = kwh_to_percent(45.0, 75.0);
        assert!((percent - 60.0).abs() < 0.01);
    }
}
```

### calculations_phev.rs

```rust
//! PHEV combined calculations - uses electricity first, then fuel

use crate::calculations;
use crate::calculations_energy;

/// Result of PHEV trip consumption calculation
pub struct PhevTripConsumption {
    /// km driven on electricity
    pub km_on_electricity: f64,
    /// km driven on fuel
    pub km_on_fuel: f64,
    /// Energy used from battery (kWh)
    pub energy_used_kwh: f64,
    /// Fuel used (liters)
    pub fuel_used_liters: f64,
    /// Battery remaining after trip (kWh)
    pub battery_remaining_kwh: f64,
    /// Fuel remaining after trip (liters)
    pub fuel_remaining_liters: f64,
}

/// Calculate PHEV consumption for a single trip
/// Electricity is used first until battery depleted, then fuel takes over
pub fn calculate_phev_trip_consumption(
    distance_km: f64,
    previous_battery_kwh: f64,
    previous_fuel_liters: f64,
    energy_charged: Option<f64>,
    fuel_added: Option<f64>,
    baseline_consumption_kwh: f64,  // kWh/100km
    tp_consumption: f64,             // l/100km
    battery_capacity: f64,
    tank_size: f64,
) -> PhevTripConsumption {
    // Add any charged energy first
    let battery_after_charge = (previous_battery_kwh + energy_charged.unwrap_or(0.0))
        .min(battery_capacity);

    // Calculate total energy needed for entire trip
    let energy_needed = calculations_energy::calculate_energy_used(
        distance_km,
        baseline_consumption_kwh
    );

    // Use electricity first (limited by available battery)
    let energy_from_battery = energy_needed.min(battery_after_charge);
    let km_on_electricity = if baseline_consumption_kwh > 0.0 {
        energy_from_battery / baseline_consumption_kwh * 100.0
    } else {
        0.0
    };

    // Remaining distance uses fuel
    let km_on_fuel = (distance_km - km_on_electricity).max(0.0);
    let fuel_used = calculations::calculate_spotreba(km_on_fuel, tp_consumption);

    // Update both tanks
    let battery_remaining = (battery_after_charge - energy_from_battery).max(0.0);
    let fuel_after_fillup = (previous_fuel_liters + fuel_added.unwrap_or(0.0))
        .min(tank_size);
    let fuel_remaining = (fuel_after_fillup - fuel_used).max(0.0);

    PhevTripConsumption {
        km_on_electricity,
        km_on_fuel,
        energy_used_kwh: energy_from_battery,
        fuel_used_liters: fuel_used,
        battery_remaining_kwh: battery_remaining,
        fuel_remaining_liters: fuel_remaining,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phev_all_electric() {
        // Battery has enough for entire trip
        let result = calculate_phev_trip_consumption(
            30.0,   // 30 km trip
            10.0,   // 10 kWh battery
            40.0,   // 40 L fuel
            None,   // no charge
            None,   // no refuel
            20.0,   // 20 kWh/100km
            6.0,    // 6 L/100km
            10.0,   // 10 kWh capacity
            40.0,   // 40 L tank
        );

        // 30 km × 20/100 = 6 kWh needed, battery has 10
        assert!((result.km_on_electricity - 30.0).abs() < 0.01);
        assert!((result.km_on_fuel - 0.0).abs() < 0.01);
        assert!((result.energy_used_kwh - 6.0).abs() < 0.01);
        assert!((result.fuel_used_liters - 0.0).abs() < 0.01);
        assert!((result.battery_remaining_kwh - 4.0).abs() < 0.01);
        assert!((result.fuel_remaining_liters - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_phev_mixed_drive() {
        // Battery runs out mid-trip
        let result = calculate_phev_trip_consumption(
            60.0,   // 60 km trip
            10.0,   // 10 kWh battery
            40.0,   // 40 L fuel
            None,   // no charge
            None,   // no refuel
            20.0,   // 20 kWh/100km
            6.0,    // 6 L/100km
            10.0,   // 10 kWh capacity
            40.0,   // 40 L tank
        );

        // 60 km × 20/100 = 12 kWh needed, battery has only 10
        // 10 kWh / 20 × 100 = 50 km on electricity
        // 60 - 50 = 10 km on fuel
        // 10 km × 6/100 = 0.6 L fuel
        assert!((result.km_on_electricity - 50.0).abs() < 0.01);
        assert!((result.km_on_fuel - 10.0).abs() < 0.01);
        assert!((result.energy_used_kwh - 10.0).abs() < 0.01);
        assert!((result.fuel_used_liters - 0.6).abs() < 0.01);
        assert!((result.battery_remaining_kwh - 0.0).abs() < 0.01);
        assert!((result.fuel_remaining_liters - 39.4).abs() < 0.01);
    }

    #[test]
    fn test_phev_all_fuel_depleted_battery() {
        // Battery already empty
        let result = calculate_phev_trip_consumption(
            50.0,   // 50 km trip
            0.0,    // 0 kWh battery (depleted)
            40.0,   // 40 L fuel
            None,   // no charge
            None,   // no refuel
            20.0,   // 20 kWh/100km
            6.0,    // 6 L/100km
            10.0,   // 10 kWh capacity
            40.0,   // 40 L tank
        );

        // No electricity available, all on fuel
        // 50 km × 6/100 = 3 L
        assert!((result.km_on_electricity - 0.0).abs() < 0.01);
        assert!((result.km_on_fuel - 50.0).abs() < 0.01);
        assert!((result.energy_used_kwh - 0.0).abs() < 0.01);
        assert!((result.fuel_used_liters - 3.0).abs() < 0.01);
        assert!((result.battery_remaining_kwh - 0.0).abs() < 0.01);
        assert!((result.fuel_remaining_liters - 37.0).abs() < 0.01);
    }

    #[test]
    fn test_phev_charge_then_drive() {
        // Charge during trip, then drive
        let result = calculate_phev_trip_consumption(
            60.0,   // 60 km trip
            2.0,    // 2 kWh battery remaining
            40.0,   // 40 L fuel
            Some(8.0), // charge 8 kWh (total 10)
            None,   // no refuel
            20.0,   // 20 kWh/100km
            6.0,    // 6 L/100km
            10.0,   // 10 kWh capacity
            40.0,   // 40 L tank
        );

        // After charge: 2 + 8 = 10 kWh
        // 60 km × 20/100 = 12 kWh needed
        // 10 kWh available = 50 km electric
        // 10 km on fuel = 0.6 L
        assert!((result.km_on_electricity - 50.0).abs() < 0.01);
        assert!((result.km_on_fuel - 10.0).abs() < 0.01);
        assert!((result.battery_remaining_kwh - 0.0).abs() < 0.01);
        assert!((result.fuel_remaining_liters - 39.4).abs() < 0.01);
    }
}
```

---

## 5. Database Migration

```sql
-- Migration: Add EV support fields

-- Vehicle type and energy fields
ALTER TABLE vehicles ADD COLUMN vehicle_type TEXT NOT NULL DEFAULT 'Ice';
ALTER TABLE vehicles ADD COLUMN battery_capacity_kwh REAL;
ALTER TABLE vehicles ADD COLUMN baseline_consumption_kwh REAL;
ALTER TABLE vehicles ADD COLUMN initial_battery_percent REAL;  -- Initial SoC for first record

-- Make existing fuel fields nullable (for BEV)
-- Note: SQLite doesn't support ALTER COLUMN, need to recreate table or leave as-is
-- Existing vehicles will have fuel fields populated (ICE default)

-- Trip energy fields
ALTER TABLE trips ADD COLUMN energy_kwh REAL;
ALTER TABLE trips ADD COLUMN energy_cost_eur REAL;
ALTER TABLE trips ADD COLUMN full_charge INTEGER DEFAULT 0;
ALTER TABLE trips ADD COLUMN soc_override_percent REAL;  -- Manual SoC override for battery degradation

-- Index for efficient queries
CREATE INDEX idx_vehicles_type ON vehicles(vehicle_type);
```

---

## 6. Implementation Phases

### Phase 1: Foundation (Models + Energy Calculations)
- [ ] Add `VehicleType` enum to models
- [ ] Add energy fields to Vehicle and Trip models
- [ ] Create `calculations_energy.rs` module with tests
- [ ] Create `calculations_phev.rs` module with tests
- [ ] Database migration for new fields
- [ ] **All existing ICE tests must pass unchanged**

### Phase 2: BEV Support
- [ ] Update vehicle form UI for BEV fields
- [ ] Update trip form UI for charging fields
- [ ] Integrate energy calculations into `get_trip_grid_data`
- [ ] Update trip grid display for BEV vehicles
- [ ] Update export for BEV

### Phase 3: PHEV Support
- [ ] Enable dual fields in vehicle/trip forms
- [ ] Integrate PHEV calculations (electricity-first logic)
- [ ] Margin calculation for fuel only
- [ ] Update trip grid for dual display
- [ ] Update export for PHEV

### Phase 4: Receipts (Optional)
- [ ] Charging receipt scanning (different format)
- [ ] Receipt type detection (fuel vs charging)

---

## 7. UI Field Visibility

### Vehicle Form

| Field | ICE | BEV | PHEV |
|-------|-----|-----|------|
| Tank size (L) | ✅ | ❌ | ✅ |
| TP consumption (l/100km) | ✅ | ❌ | ✅ |
| Battery capacity (kWh) | ❌ | ✅ | ✅ |
| Baseline consumption (kWh/100km) | ❌ | ✅ | ✅ |
| Initial battery % | ❌ | ✅ (optional) | ✅ (optional) |

**Note:** Vehicle type selector is disabled/hidden once trips exist for the vehicle.

### Trip Form

| Field | ICE | BEV | PHEV | Notes |
|-------|-----|-----|------|-------|
| Fuel (L) | ✅ | ❌ | ✅ | |
| Fuel cost (€) | ✅ | ❌ | ✅ | |
| Full tank | ✅ | ❌ | ✅ | |
| Energy (kWh) | ❌ | ✅ | ✅ | |
| Energy cost (€) | ❌ | ✅ | ✅ | |
| Full charge | ❌ | ✅ | ✅ | |
| SoC override (%) | ❌ | ✅ | ✅ | Hidden in grid, only in edit form |

**SoC override field:**
- Not visible in trip grid (only when editing trip)
- When set, a small indicator (🔧) appears next to battery % in grid
- Used for battery degradation correction - value applies "from this trip onwards"

### Trip Grid Columns

| Column | ICE | BEV | PHEV | Notes |
|--------|-----|-----|------|-------|
| Spotreba (L) | ✅ | ❌ | ✅ | |
| Zostatok (L) | ✅ | ❌ | ✅ | |
| l/100km | ✅ | ❌ | ✅ | |
| Marža % | ✅ | ❌ | ✅ (fuel only) | |
| Energy used (kWh) | ❌ | ✅ | ✅ | |
| Battery (kWh / %) | ❌ | ✅ | ✅ | Shows 🔧 if SoC override |
| kWh/100km | ❌ | ✅ | ✅ | |

---

*Analysis conducted: 2026-01-01*
*Based on: [research.md](./research.md)*
*Status: Design complete, ready for implementation planning*
