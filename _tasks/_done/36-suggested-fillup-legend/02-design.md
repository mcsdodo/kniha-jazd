# Design: Suggested Fillup Legend Indicator

## Data Model Changes

### Backend (Rust)

Add to `TripData` struct in `commands.rs`:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedFillup {
    pub liters: f64,           // e.g., 38.45
    pub consumption_rate: f64, // e.g., 5.78 (l/100km)
}

pub struct TripData {
    // ... existing fields ...
    pub suggested_fillup: Option<SuggestedFillup>, // NEW
}
```

### Frontend (TypeScript)

Add to `types.ts`:

```typescript
interface SuggestedFillup {
    liters: number;
    consumptionRate: number;
}

interface TripData {
    // ... existing fields ...
    suggestedFillup?: SuggestedFillup;
}
```

## Calculation Logic

In `get_trip_grid_data`, after processing trips:

1. Identify trips in open period (from last full tank or start)
2. For each trip in open period:
   - Calculate `open_period_km` up to and including this trip
   - Generate random multiplier 1.05-1.20
   - `liters = (open_period_km * tp_rate * multiplier) / 100`
   - `consumption_rate = liters / open_period_km * 100`
   - Set `trip.suggested_fillup = Some(SuggestedFillup { liters, consumption_rate })`
3. Trips NOT in open period: `suggested_fillup = None`

## Frontend Display

### Legend Component (TripGrid.svelte)

```svelte
{#if suggestedFillup}
    <span class="legend-item suggested-fillup">
        <span class="suggested-indicator">💡</span>
        {$LL.trips.legend.suggestedFillup({
            liters: suggestedFillup.liters.toFixed(2),
            rate: suggestedFillup.consumptionRate.toFixed(2)
        })}
    </span>
{/if}
```

Where `suggestedFillup` is derived from the last trip that has a suggestion:
```typescript
$: suggestedFillup = trips.findLast(t => t.suggestedFillup)?.suggestedFillup;
```

### Styling

```css
.suggested-fillup {
    color: var(--accent-success);
}

.suggested-indicator {
    margin-right: 0.25rem;
}
```

## Magic Button Simplification

### Before (TripRow.svelte)
```typescript
async function handleMagicFill() {
    const liters = await calculateMagicFillLiters(vehicleId, year, km, tripId);
    fuelLitersInput = liters;
}
```

### After
```typescript
function handleMagicFill() {
    if (trip.suggestedFillup) {
        fuelLitersInput = trip.suggestedFillup.liters;
        fullTank = true;
    }
}
```

No async, no backend call, instant response.

## i18n Translations

### Slovak (sk/index.ts)
```typescript
suggestedFillup: "Návrh tankovania: {liters} L → {rate} l/100km"
```

### English (en/index.ts)
```typescript
suggestedFillup: "Suggested fillup: {liters} L → {rate} l/100km"
```

## Edge Cases

1. **No trips**: No legend shown (existing behavior)
2. **All trips have full tank**: `suggestedFillup = None` for all, indicator not shown
3. **BEV vehicle**: Skip calculation entirely (no fuel)
4. **PHEV vehicle**: Calculate for fuel portion only (existing TP rate logic handles this)
5. **Very short open period** (e.g., 5 km): Show suggestion anyway (small liters is valid)

## Testing Strategy

### Backend Unit Tests (commands_tests.rs)
1. `test_suggested_fillup_in_open_period` - trips without full tank get suggestions
2. `test_suggested_fillup_closed_period` - trips after full tank have None
3. `test_suggested_fillup_consumption_rate_calculation` - verify math
4. `test_suggested_fillup_bev_skipped` - BEV vehicles return None

### Integration Tests
1. Verify legend displays when open period exists
2. Verify legend hidden when all periods closed
3. Verify magic button uses pre-calculated value
