# Bug: ODO Recalculation Uses Stale Data When Inserting Trip

## Summary

When inserting a trip "in between" existing trips, the odometer recalculation runs on stale data, causing incorrect ODO values and wrong fill-up window assignments.

## Symptoms

- Newly inserted trip has incorrect ODO value
- Existing trips aren't recalculated to account for the insert
- Fill-up window calculations assign fuel to wrong trips
- PHM (fuel liters) appears on wrong trip in the grid

## Root Cause

In `TripGrid.svelte`, the `handleCreate()` function has incorrect operation order:

```javascript
await createTrip(...);           // 1. Creates trip in DB
await recalculateAllOdo();       // 2. Runs on OLD trips list! ❌
onTripsChanged();                // 3. Refreshes trips from DB
```

The `recalculateAllOdo()` function uses the `trips` prop, which hasn't been refreshed yet at that point. The newly created trip isn't in the list, so:
1. The new trip keeps its initial ODO value (calculated from `previousOdometer + km`)
2. Existing trips aren't shifted to make room for the insert

## Fix

Swap the order and wait for Svelte reactivity:

```javascript
await createTrip(...);
await onTripsChanged();          // First: refresh trips from DB
await tick();                    // Wait for Svelte prop update
await recalculateAllOdo();       // Then: recalculate on updated list
```

## Files Changed

- `src/lib/components/TripGrid.svelte`:
  - Added `tick` import from svelte
  - Fixed operation order in `handleCreate()`
  - Fixed same issue in `handleMoveUp()` and `handleMoveDown()` (reorder operations)

## Testing

Manual test:
1. Add several trips with fill-ups
2. Insert a new trip "in between" using the + button
3. Verify ODO values are recalculated correctly
4. Verify fill-up window assignments are correct

## Related

- ADR-008: Backend-only calculations (note: `recalculateAllOdo` is a frontend calculation for ODO only, which is acceptable since ODO is a simple cumulative sum)
