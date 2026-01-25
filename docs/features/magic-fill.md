# Feature: Magic Fill

> Automatically calculates fuel liters needed to achieve a target consumption rate, simplifying trip entry by suggesting realistic fuel values.

## User Flow

1. User enters a new trip or edits an existing trip in the trip grid
2. User fills in the distance (km) field
3. User clicks the **Magic Fill** button (sparkle/star icon) in the actions column
4. System calculates suggested fuel liters based on:
   - Accumulated kilometers in the current open period
   - Vehicle's TP (technical passport) consumption rate
   - Random target multiplier between 105-120% of TP rate
5. The fuel liters field is populated with the calculated value
6. The "Full Tank" checkbox is automatically checked
7. Live preview updates to show the resulting consumption rate

**Scope note**: Magic Fill uses only trips in the selected year. When editing an existing trip, it calculates based on the open period **up to that trip** (not future trips), allowing accurate suggestions for mid-period edits.

## Technical Implementation

### Core Calculation

The magic fill feature calculates fuel liters that would result in a realistic consumption rate (105-120% of TP consumption):

```
target_rate = tp_consumption × random(1.05, 1.20)
suggested_liters = (total_km × target_rate) / 100
```

Results are rounded to 2 decimals.

**Example:**
- TP consumption: 5.0 L/100km
- Open period km: 100 km
- Random multiplier: 1.10 (110%)
- Target rate: 5.0 × 1.10 = 5.5 L/100km
- Suggested liters: (100 × 5.5) / 100 = **5.5 L**

### Open Period Calculation

Magic fill only considers kilometers in the **current open period** — the kilometers driven since the last full tank fill-up, up to the trip being edited:

```rust
fn get_open_period_km(chronological: &[Trip], stop_at_trip_id: Option<&Uuid>) -> f64 {
    let mut km_in_period = 0.0;

    for trip in chronological {
        km_in_period += trip.distance_km;

        // When editing, stop after the edited trip (don't count future trips)
        if let Some(stop_id) = stop_at_trip_id {
            if &trip.id == stop_id {
                break;
            }
        }

        // Full tank fillup closes the period
        if trip.full_tank && trip.fuel_liters > 0.0 {
            km_in_period = 0.0;
        }
    }

    km_in_period
}
```

**Period logic:**
- Partial fill-ups do NOT close the period
- Only a full tank fill-up resets the counter
- After a full tank, open period km starts at 0
- Only the selected year is considered
- Trips are processed chronologically (date, then odometer)
- **When editing a trip in the middle**, only km up to that trip are counted (not future trips)

### Existing Trip vs New Trip

The calculation handles new and existing trips differently to avoid double-counting:

| Scenario | Total KM Calculation |
|----------|---------------------|
| **New trip** | `open_period_km(None) + form_distance_km` — counts all km in period |
| **Editing existing trip** | `open_period_km(Some(id))` — counts km up to and including edited trip only |

This is controlled via the `editing_trip_id` parameter:
- `None` → new trip, add form's km to full period
- `Some(id)` → editing, stop counting at edited trip (don't include future trips)

### Buffer Kilometers (Related)

A related function `calculate_buffer_km()` calculates how many additional kilometers are needed to bring consumption rate down to a target margin:

```rust
// Formula:
target_rate = tp_rate × (1.0 + target_margin)  // e.g., 5.1 × 1.18 = 6.018
required_km = (liters_filled × 100.0) / target_rate
buffer_km = required_km - km_driven
```

**Example:**
- 50L filled, 800km driven, TP=5.1, target=18%
- Target rate: 5.1 × 1.18 = 6.018 L/100km
- Required km: 50 × 100 / 6.018 = 830.93 km
- Buffer: 830.93 - 800 = **30.93 km needed**

This is used for warning display when consumption exceeds the legal limit.

### Integration with Trip Form

The magic fill button is available in the trip row's edit mode:

```svelte
async function handleMagicFill() {
    const currentKm = formData.distanceKm ?? 0;
    if (currentKm <= 0) return;
    
    const tripId = trip?.id ?? null;
    const suggestedLiters = await onMagicFill(currentKm, tripId);
    
    if (suggestedLiters > 0) {
        formData.fuelLiters = suggestedLiters;
        formData.fullTank = true;
        onPreviewRequest(currentKm, suggestedLiters, true);
    }
}
```

**UI behavior:**
- Button only works when distance > 0
- Sets fuel liters to calculated value
- Automatically checks "Full Tank"
- Triggers live preview calculation

## Key Files

| File | Purpose |
|------|---------|
| [commands.rs](src-tauri/src/commands.rs) | `calculate_magic_fill_liters` — main calculation |
| [commands.rs](src-tauri/src/commands.rs) | `get_open_period_km` — open period helper |
| [calculations.rs](src-tauri/src/calculations.rs) | `calculate_buffer_km` — buffer calculation |
| [api.ts](src/lib/api.ts) | `calculateMagicFillLiters()` — frontend API |
| [TripRow.svelte](src/lib/components/TripRow.svelte) | `handleMagicFill()` — UI handler |
| [TripGrid.svelte](src/lib/components/TripGrid.svelte) | Magic fill callback wrapper |

## Edge Cases

| Condition | Behavior |
|-----------|----------|
| No trips in year | Returns suggestion based on current trip km (new trip); returns 0.0 when editing and open period is 0 |
| Total km ≤ 0 | Returns 0.0 |
| No vehicle found | Returns error |
| No TP consumption set | Uses default 5.0 L/100km |
| All periods closed | Returns calculation for new trip's km only; returns 0.0 when editing |
| Distance field empty/zero | Button does nothing |
| **Editing trip in middle of period** | Only counts km from last full tank up to edited trip; ignores trips that come after |

## Design Decisions

### Why Random Multiplier (105-120%)?

Provides natural variation in consumption rates, avoiding suspiciously consistent values while staying well under the 120% legal limit.

**Benefits**:
- Looks realistic (not exactly TP rate every time)
- Safe margin from legal limit
- Mimics real-world driving variation

### Why Auto-Check Full Tank?

Magic fill assumes the user wants to close the period, which requires a full tank for accurate consumption tracking.

**Reasoning**:
- Period-based calculation needs closing fill-up
- Partial fills don't give accurate rates
- Most common use case is end-of-period fill

### Why Live Preview Integration?

Immediately shows the resulting consumption rate so user can accept or modify the suggestion.

**User experience**:
- See the impact before saving
- Adjust if rate seems off
- Confidence in the calculation

### Why Separate from Buffer KM?

Magic fill calculates **liters from km**; buffer_km calculates **km from liters**. Different use cases:

| Feature | Input | Output | Use Case |
|---------|-------|--------|----------|
| Magic Fill | km driven | liters to add | Filling up at the pump |
| Buffer KM | liters filled | km still needed | Warning about high consumption |

### Why Period-Based, Not Trip-Based?

Works on accumulated km in open period, matching how consumption is actually calculated per fill-up window.

**Accurate calculation**:
- Matches the period-based rate system
- All trips in period get same rate
- Magic fill respects this by using period total
