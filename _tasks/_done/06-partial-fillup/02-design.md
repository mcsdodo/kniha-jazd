# Design: Partial Fillup Support

**Date:** 2025-12-25

## Problem

The current consumption calculation assumes every fillup is a FULL tank. When a partial fillup is recorded (e.g., 19.67L on 25.09), the system calculates an impossibly low consumption rate and shows incorrect "zostatok" values.

## Solution Summary

- Add `full_tank` boolean field to Trip (default: true)
- Only calculate consumption rate on full tank fillups
- Sum all fillups (partial + full) in a span for rate calculation
- Partial fillups add fuel without resetting to tank size
- Show TP rate with visual indicator for incomplete spans

---

## Data Model

**Add `full_tank` field to Trip:**

```rust
// models.rs
pub struct Trip {
    // ... existing fields ...
    pub fuel_liters: Option<f64>,
    pub fuel_cost_eur: Option<f64>,
    pub full_tank: bool,  // true = full tank, false = partial
    // ...
}
```

**Database migration:**
```sql
ALTER TABLE trips ADD COLUMN full_tank INTEGER NOT NULL DEFAULT 1;
```

**TypeScript type:**
```typescript
interface Trip {
    // ... existing ...
    full_tank: boolean;
}
```

---

## Consumption Rate Calculation

```typescript
function calculateConsumptionRates(trips: Trip[]): Map<string, number> {
    const rates = new Map<string, number>();
    const chronological = sortByDateAndOdo(trips);

    let periodTrips: string[] = [];
    let periodKm = 0;
    let periodFuel = 0;  // Sum of ALL fillups in period

    for (const trip of chronological) {
        periodTrips.push(trip.id);
        periodKm += trip.distance_km;

        if (trip.fuel_liters) {
            periodFuel += trip.fuel_liters;
        }

        // Only calculate rate on FULL TANK fillup
        if (trip.fuel_liters && trip.full_tank && periodKm > 0) {
            const rate = (periodFuel / periodKm) * 100;
            for (const id of periodTrips) {
                rates.set(id, rate);
            }
            // Reset for next period
            periodTrips = [];
            periodKm = 0;
            periodFuel = 0;
        }
    }

    // Incomplete period (no full tank yet) → use TP rate
    for (const id of periodTrips) {
        rates.set(id, tpConsumption);
    }

    return rates;
}
```

---

## Zostatok Calculation

```typescript
function calculateFuelRemaining(trips: Trip[], rates: Map<string, number>): Map<string, number> {
    const remaining = new Map<string, number>();
    const chronological = sortByDateAndOdo(trips);

    let zostatok = tankSize;

    for (const trip of chronological) {
        const rate = rates.get(trip.id) || tpConsumption;
        const consumed = (trip.distance_km * rate) / 100;
        zostatok -= consumed;

        if (trip.fuel_liters && trip.fuel_liters > 0) {
            if (trip.full_tank) {
                zostatok = tankSize;  // Full tank → reset
            } else {
                zostatok += trip.fuel_liters;  // Partial → just add
            }
        }

        if (zostatok < 0) zostatok = 0;
        if (zostatok > tankSize) zostatok = tankSize;

        remaining.set(trip.id, zostatok);
    }

    return remaining;
}
```

---

## UI Changes

**TripRow - Checkbox in edit mode:**
- Show "Plná" checkbox when fuel_liters has value
- Default checked (full tank)
- Uncheck for partial fillups

**Display - Visual indicator:**
- Estimated rates (TP fallback) shown in italic gray
- CSS class `.estimated` for styling

**Column:** Add narrow "Plná" column after "Cena €"

---

## Implementation Order

1. Database migration - Add `full_tank` column
2. Backend models - Update `Trip` struct
3. Backend commands - Update `create_trip`, `update_trip`
4. Frontend types - Update `Trip` interface
5. Frontend API - Update `createTrip`, `updateTrip`
6. TripRow component - Add checkbox, update formData
7. TripGrid calculations - Update rate and zostatok functions
8. Styling - Add `.estimated` class
9. Import script - Set `full_tank=true` for imports

---

## Testing

- Existing data: same behavior (all full tank)
- Partial fillup: zostatok adds fuel, doesn't reset
- Rate calculation: TP rate until next full tank
- Multiple partials: rate from sum of all fillups in span
