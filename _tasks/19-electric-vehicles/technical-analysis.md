# Electric Vehicle Support - Technical Analysis

> **Focus:** App implementation implications for EV/PHEV support.
> For legislation and accounting research, see [research.md](./research.md).

---

## 1. Current Data Model (ICE-focused)

### Vehicle Model

```rust
pub struct Vehicle {
    pub id: Uuid,
    pub name: String,
    pub license_plate: String,
    pub tank_size_liters: f64,      // Fuel tank capacity
    pub tp_consumption: f64,         // l/100km from technical passport
    pub initial_odometer: f64,
    pub is_active: bool,
    // ...
}
```

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
    pub fuel_liters: Option<f64>,      // Fuel added at fill-up
    pub fuel_cost_eur: Option<f64>,    // Cost of fuel
    pub full_tank: bool,               // True = full tank fill-up
    // ...
}
```

### Core Calculations

```rust
// Consumption rate: liters per 100km
fn calculate_consumption_rate(liters: f64, km_since_last_fillup: f64) -> f64

// Fuel used for trip
fn calculate_spotreba(distance_km: f64, consumption_rate: f64) -> f64

// Remaining fuel
fn calculate_zostatok(previous: f64, spotreba: f64, fuel_added: Option<f64>, tank_size: f64) -> f64

// Margin vs TP consumption (the 20% rule)
fn calculate_margin_percent(consumption_rate: f64, tp_rate: f64) -> f64

// Legal limit check
fn is_within_legal_limit(margin_percent: f64) -> bool  // margin <= 20%
```

---

## 2. What Changes for EVs

### Field Mapping

| Current Field | BEV Equivalent | Notes |
|---------------|----------------|-------|
| `tank_size_liters` | `battery_capacity_kwh` | Battery size in kWh |
| `tp_consumption` | `baseline_consumption_kwh` | **User-defined** (no legal value) |
| `fuel_liters` | `energy_kwh` | Energy charged |
| `fuel_cost_eur` | `energy_cost_eur` | Same concept |
| `full_tank` | `target_soc_reached` | E.g., charged to 80% or 100% |
| 20% margin check | **N/A or optional** | No legal requirement |
| `zostatok` | `battery_soc_kwh` | State of charge in kWh |

### Calculations - Same Logic, Different Units

```rust
// EV consumption rate: kWh per 100km
fn calculate_consumption_rate_ev(kwh: f64, km_since_last_charge: f64) -> f64 {
    // Same formula as ICE, different units
    if km_since_last_charge <= 0.0 { return 0.0; }
    (kwh / km_since_last_charge) * 100.0
}

// Energy used for trip
fn calculate_energy_used(distance_km: f64, consumption_rate_kwh: f64) -> f64 {
    distance_km * consumption_rate_kwh / 100.0
}

// Remaining battery
fn calculate_battery_remaining(
    previous_kwh: f64,
    energy_used: f64,
    energy_charged: Option<f64>,
    battery_capacity: f64
) -> f64 {
    let new_soc = previous_kwh - energy_used + energy_charged.unwrap_or(0.0);
    new_soc.min(battery_capacity).max(0.0)
}
```

### No Margin Calculation for EVs

Per legislation research:
- EVs have **no TP consumption value**
- Therefore, **no baseline** to compare against
- The 20% margin rule **does not apply**

**App options:**
1. **Skip margin entirely for EVs** (legally accurate)
2. **Optional user-defined baseline** (for personal tracking, not legal)

---

## 3. What Changes for PHEVs

### Dual Fuel Tracking Required

PHEVs consume **both gasoline AND electricity**. Must track:
- Gasoline (liters, cost) - may have TP value → 20% rule applies
- Electricity (kWh, cost) - no TP value → no margin rule

### Proposed PHEV Fields

```rust
pub struct Vehicle {
    // Common fields...

    // ICE fields (used for ICE and PHEV gasoline)
    pub tank_size_liters: Option<f64>,
    pub tp_consumption: Option<f64>,        // l/100km

    // EV fields (used for BEV and PHEV electricity)
    pub battery_capacity_kwh: Option<f64>,
    pub baseline_consumption_kwh: Option<f64>,  // User-defined

    // Vehicle type discriminator
    pub vehicle_type: VehicleType,  // ICE | BEV | PHEV
}

pub enum VehicleType {
    ICE,   // Internal combustion engine
    BEV,   // Battery electric vehicle
    PHEV,  // Plug-in hybrid
}
```

### PHEV Trip Structure

```rust
pub struct Trip {
    // Common fields...

    // Gasoline (ICE + PHEV)
    pub fuel_liters: Option<f64>,
    pub fuel_cost_eur: Option<f64>,
    pub full_tank: bool,

    // Electricity (BEV + PHEV)
    pub energy_kwh: Option<f64>,
    pub energy_cost_eur: Option<f64>,
    pub charging_source: Option<ChargingSource>,
}

pub enum ChargingSource {
    PublicStation,
    HomeWallbox,
    Workplace,
    Other,
}
```

---

## 4. Business Logic Changes

### Feature Matrix by Vehicle Type

| Feature | ICE | BEV | PHEV |
|---------|-----|-----|------|
| Consumption rate (l/100km) | ✅ | ❌ | ✅ (gasoline) |
| Consumption rate (kWh/100km) | ❌ | ✅ | ✅ (electricity) |
| Zostatok liters | ✅ | ❌ | ✅ |
| Zostatok kWh | ❌ | ✅ | ✅ |
| 20% margin warning | ✅ | ❌ | ⚠️ gasoline only |
| Compensation suggestions | ✅ | ❓ optional | ⚠️ complex |
| Fuel receipt scanning | ✅ | ❌ | ✅ |
| Charging receipt scanning | ❌ | ✅ | ✅ |

### Margin Calculation Logic

```rust
fn should_calculate_margin(vehicle: &Vehicle) -> bool {
    match vehicle.vehicle_type {
        VehicleType::ICE => true,
        VehicleType::BEV => false,  // No TP baseline
        VehicleType::PHEV => true,  // Only for gasoline portion
    }
}

fn calculate_margin_for_vehicle(vehicle: &Vehicle, consumption_rate: f64) -> Option<f64> {
    if !should_calculate_margin(vehicle) {
        return None;
    }

    match vehicle.tp_consumption {
        Some(tp) if tp > 0.0 => Some(calculate_margin_percent(consumption_rate, tp)),
        _ => None,
    }
}
```

---

## 5. Database Schema Changes

### Option A: Extend Existing Tables

```sql
-- Add to vehicles table
ALTER TABLE vehicles ADD COLUMN vehicle_type TEXT DEFAULT 'ICE';
ALTER TABLE vehicles ADD COLUMN battery_capacity_kwh REAL;
ALTER TABLE vehicles ADD COLUMN baseline_consumption_kwh REAL;

-- Add to trips table
ALTER TABLE trips ADD COLUMN energy_kwh REAL;
ALTER TABLE trips ADD COLUMN energy_cost_eur REAL;
ALTER TABLE trips ADD COLUMN charging_source TEXT;
```

### Option B: Separate Energy Table

```sql
-- New table for charging sessions
CREATE TABLE charging_sessions (
    id TEXT PRIMARY KEY,
    trip_id TEXT REFERENCES trips(id),
    energy_kwh REAL NOT NULL,
    energy_cost_eur REAL,
    charging_source TEXT,
    soc_before_percent REAL,
    soc_after_percent REAL,
    created_at TEXT NOT NULL
);
```

---

## 6. UI Considerations

### Vehicle Form

New/Edit vehicle needs:
- **Vehicle type selector**: ICE / BEV / PHEV
- **Conditional fields** based on type:
  - ICE: tank size, TP consumption
  - BEV: battery capacity, baseline consumption (optional)
  - PHEV: all of the above

### Trip Form

- **ICE**: fuel liters + cost (current behavior)
- **BEV**: energy kWh + cost + charging source
- **PHEV**: both sections, all optional

### Trip Grid Display

- **ICE**: spotreba (l), zostatok (l), margin %
- **BEV**: energy used (kWh), SoC remaining (kWh or %), no margin
- **PHEV**: both rows or combined display

### Export

- PDF/HTML export needs vehicle-type-aware formatting
- Different column headers for kWh vs liters
- PHEV may need dual columns

---

## 7. Key Design Decisions

### Q1: Should EVs have margin tracking at all?

**Option A: No margin for EVs** (Recommended)
- Legally accurate
- Simpler implementation
- Users can still see consumption rate for awareness

**Option B: User-defined baseline for EVs**
- User sets expected consumption (WLTP or own measurement)
- App tracks deviation from baseline
- Warning is informational, not legal
- More complex, questionable value

### Q2: How to handle zostatok for EVs?

**Option A: Track in kWh** (Recommended)
- Matches current liters logic
- More precise for calculations
- Display can show both kWh and % (derived)

**Option B: Track in SoC%**
- What users see in their car
- Less precise for calculations
- Requires knowing battery capacity anyway

### Q3: PHEV dual tracking approach?

**Option A: Full dual-fuel tracking** (Recommended)
- Single vehicle entry with both fuel types
- More accurate for actual PHEV usage
- Required by Slovak law (single documentation method per vehicle)

**Option B: Treat as two vehicle entries**
- User manages two "vehicles" (gasoline and electric)
- Simpler implementation
- Violates single-vehicle-single-method principle

---

## 8. Implementation Phases

### Phase 1: BEV Support (Minimum Viable)
1. Add `vehicle_type` field (ICE/BEV)
2. Add battery fields to Vehicle
3. Add energy fields to Trip
4. Adapt calculations for kWh
5. Skip margin for BEV
6. Update UI for vehicle type

### Phase 2: PHEV Support
1. Add PHEV vehicle type
2. Enable dual fuel tracking in trips
3. Implement gasoline-only margin for PHEVs
4. Update export for dual-fuel

### Phase 3: Charging Features (Optional)
1. Charging source tracking
2. Home vs public charging cost differentiation
3. Charging receipt scanning (different format than fuel)

---

## 9. Test Cases to Add

### BEV Consumption Tests

```rust
#[test]
fn test_ev_consumption_rate() {
    // 45 kWh / 250 km = 18.0 kWh/100km
    let rate = calculate_consumption_rate_ev(45.0, 250.0);
    assert!((rate - 18.0).abs() < 0.01);
}

#[test]
fn test_ev_battery_remaining() {
    // Start with 60 kWh, use 15 kWh, no charge = 45 kWh
    let remaining = calculate_battery_remaining(60.0, 15.0, None, 75.0);
    assert!((remaining - 45.0).abs() < 0.01);
}

#[test]
fn test_ev_no_margin() {
    let vehicle = Vehicle { vehicle_type: VehicleType::BEV, .. };
    assert!(!should_calculate_margin(&vehicle));
}
```

### PHEV Tests

```rust
#[test]
fn test_phev_gasoline_margin_applies() {
    let vehicle = Vehicle {
        vehicle_type: VehicleType::PHEV,
        tp_consumption: Some(5.5),
        ..
    };
    // Only gasoline margin should apply
    assert!(should_calculate_margin(&vehicle));
}
```

---

*Analysis conducted: 2026-01-01*
*Based on: [research.md](./research.md)*
*Status: Ready for design decisions*
