# Design: Fuel Consumed Column

## Overview

Add a calculated column showing fuel consumed per trip (liters) to the TripGrid.

**Formula**: `fuel_consumed = distance_km × consumption_rate / 100`

## Data Flow

```
┌─────────────────────────────────────────────────────────┐
│  get_trip_grid_data() - Rust Backend                    │
│  ┌─────────────────────────────────────────────────┐    │
│  │ calculate_period_rates() → rates HashMap        │    │
│  │ calculate_fuel_consumed() → fuel_consumed HashMap│   │
│  └─────────────────────────────────────────────────┘    │
│  Returns: TripGridData { ..., fuel_consumed: {...} }    │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│  TripGrid.svelte - Frontend                             │
│  fuelConsumed: Map<string, number>                      │
│  <TripRow fuelConsumed={fuelConsumed.get(trip.id)} />   │
└─────────────────────────────────────────────────────────┘
```

## Backend Changes

### New Function (commands.rs)

```rust
/// Calculate fuel consumed per trip (liters)
/// Formula: distance_km × rate / 100
pub(crate) fn calculate_fuel_consumed(
    trips: &[Trip],
    rates: &HashMap<String, f64>,
) -> HashMap<String, f64> {
    trips
        .iter()
        .map(|trip| {
            let rate = rates.get(&trip.id).copied().unwrap_or(0.0);
            let consumed = (trip.distance_km * rate) / 100.0;
            (trip.id.clone(), consumed)
        })
        .collect()
}
```

### TripGridData Struct Update

```rust
pub struct TripGridData {
    // ... existing fields ...
    pub fuel_consumed: HashMap<String, f64>,
}
```

## Frontend Changes

### Types (types.ts)

```typescript
export interface TripGridData {
    // ... existing ...
    fuelConsumed: Record<string, number>;
}
```

### TripGrid.svelte

- Add `fuelConsumed: Map<string, number>` state
- Load from `gridData.fuelConsumed` in `loadGridData()`
- Pass to TripRow component
- Add column header after "Cena €"

### TripRow.svelte

- Add `fuelConsumed: number` prop
- Display cell with `.number.calculated` styling

### Column Layout

| Column | Current Width | New Width |
|--------|---------------|-----------|
| l/100km | 4% | 3.5% |
| **Spotr. (L)** | - | **4%** |
| Zostatok | 4% | 3.5% |
| Iné pozn. | 10% | 7% |

## i18n

- SK: `fuelConsumed: () => "Spotr. (L)"`
- EN: `fuelConsumed: () => "Cons. (L)"`

## Edge Cases

| Scenario | Result |
|----------|--------|
| 0 km trip | 0.0 L consumed |
| First record row | 0.0 (display only) |
| Open period (no full tank) | Uses TP rate |
| BEV vehicle | Column hidden |
