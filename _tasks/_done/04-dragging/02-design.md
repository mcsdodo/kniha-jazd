**Date:** 2025-12-23
**Subject:** Drag-and-drop reordering and insert-anywhere for trips
**Status:** Planning

## Summary

Enable users to:
1. Drag and drop trips to reorder them
2. Insert new entries at any position in the list
3. Maintain correct calculations (ODO, consumption, zostatok) after reordering

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Ordering mechanism | Explicit `sort_order` field + auto-adjust date | Flexible reordering; date stays logical |
| Date on reorder | Copy from trip above (newer neighbor) | Natural fit in newest-first display |
| Insert UI | "+" icon in Actions column, inserts above | Clear, per-row action |
| ODO handling | Auto-recalculate all affected trips | Legal compliance requires consistency |
| Drag feedback | Drop line indicator between rows | Clear, standard pattern |
| Non-draggable rows | Editing row, new unsaved row, "Prvý záznam" | Prevent confusing mid-edit states |
| Drag library | svelte-dnd-action | Smooth animations, Svelte-native, small footprint |
| Migration | Initialize sort_order by chronological date | Preserves current display order |

---

## 1. Data Model Changes

### Trip model - new field

```rust
// src-tauri/src/models.rs
pub struct Trip {
    // ... existing fields ...
    pub sort_order: i32,  // Explicit ordering within vehicle (0 = newest/top)
}
```

```typescript
// src/lib/types.ts
export interface Trip {
    // ... existing fields ...
    sort_order: number;
}
```

### Database migration

```sql
-- Add column
ALTER TABLE trips ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;

-- Initialize based on chronological order (newest = 0)
UPDATE trips SET sort_order = (
  SELECT COUNT(*) FROM trips t2
  WHERE t2.vehicle_id = trips.vehicle_id
  AND (t2.date > trips.date OR (t2.date = trips.date AND t2.odometer > trips.odometer))
);
```

### Sorting logic

- **Before:** Sort by `date DESC`, then `odometer DESC`
- **After:** Sort by `sort_order ASC` (0 = top/newest)

---

## 2. Backend API Changes

### New command: reorder_trip

```rust
#[tauri::command]
pub fn reorder_trip(
    trip_id: Uuid,
    new_sort_order: i32,
    new_date: NaiveDate,
) -> Result<Vec<Trip>, String>
```

Actions:
1. Update moved trip's `sort_order` and `date`
2. Shift other trips' `sort_order` to make room
3. Recalculate ODO for all affected trips
4. Return updated trip list

### Updated commands

| Command | Change |
|---------|--------|
| `create_trip` | Add optional `insert_at_sort_order` param. Shifts existing trips down. |
| `get_trips` | Return `ORDER BY sort_order ASC` |
| `delete_trip` | No change (gaps in sort_order are fine) |

### Insert logic

- **Insert above row X:** New trip gets X's `sort_order`; all `sort_order >= X` increment by 1
- **Add at top:** New trip gets `sort_order = 0`; all existing trips increment by 1

---

## 3. Frontend: TripRow Actions Redesign

### Current
Single "Odstrániť" text button.

### New
Three icon buttons with tooltips:

```svelte
<td class="actions">
  {#if !isEditing}
    <button class="icon-btn insert" on:click={handleInsertAbove} title="Vložiť záznam nad">
      <!-- Plus icon SVG -->
    </button>
    <button class="icon-btn delete" on:click={handleDeleteClick} title="Odstrániť záznam">
      <!-- Trash icon SVG -->
    </button>
    <div class="drag-handle" use:dragHandle title="Presunúť záznam">
      <!-- Grip/drag icon SVG -->
    </div>
  {/if}
</td>
```

### Icon styling
- Default: Muted gray (#9e9e9e)
- Hover: Plus → blue (#3498db), Trash → red (#f44336), Grip → dark gray (#616161)
- Size: ~20px
- Cursor: `pointer` for buttons, `grab` for drag handle

---

## 4. Frontend: Drag-and-Drop Implementation

### Dependencies

```bash
npm install svelte-dnd-action
```

### TripGrid.svelte changes

```svelte
<script>
  import { dndzone } from 'svelte-dnd-action';

  $: dndItems = sortedTrips.map(t => ({ ...t, id: t.id }));
  $: dragDisabled = showNewRow || hasEditingRow;

  function handleDndConsider(e) {
    dndItems = e.detail.items;
  }

  async function handleDndFinalize(e) {
    const oldItems = dndItems;
    dndItems = e.detail.items;

    try {
      await saveNewOrder(dndItems);
    } catch (error) {
      dndItems = oldItems; // Revert on failure
      alert('Nepodarilo sa zmeniť poradie');
    }
  }
</script>

<tbody use:dndzone={{
         items: dndItems,
         dragDisabled,
         dropTargetStyle: { outline: '2px solid #3498db' }
       }}
       on:consider={handleDndConsider}
       on:finalize={handleDndFinalize}>
  <!-- TripRow components -->
</tbody>

<!-- "Prvý záznam" row OUTSIDE dndzone -->
<tbody>
  <tr class="first-record">...</tr>
</tbody>
```

### Drag handle configuration

Use `svelte-dnd-action`'s handle feature to restrict drag initiation to the grip icon only.

---

## 5. Auto-Recalculation Logic

### Date adjustment (on drop)

```typescript
function getNewDateForPosition(items: Trip[], dropIndex: number): string {
  if (dropIndex === 0) {
    // Dropped at top - keep original date
    return items[dropIndex].date;
  }
  // Copy date from the trip above (more recent in newest-first list)
  return items[dropIndex - 1].date;
}
```

### ODO recalculation (after reorder)

```typescript
async function recalculateAllOdo(trips: Trip[], initialOdometer: number) {
  // Sort by sort_order ascending, then reverse for chronological (oldest first)
  const chronological = [...trips]
    .sort((a, b) => a.sort_order - b.sort_order)
    .reverse();

  let runningOdo = initialOdometer;
  for (const trip of chronological) {
    runningOdo += trip.distance_km;
    if (Math.abs(trip.odometer - runningOdo) > 0.01) {
      await updateTrip(trip.id, { ...trip, odometer: runningOdo });
    }
  }
}
```

### Consumption and Zostatok

No changes needed - existing `calculateConsumptionRates()` and `calculateFuelRemaining()` already iterate by order. Once sort order changes, they recalculate automatically.

---

## 6. Edge Cases & Constraints

| Element | Draggable | Drop target |
|---------|-----------|-------------|
| Normal trip row | Yes | Yes |
| Row being edited | No | No |
| New unsaved row | No | No |
| "Prvý záznam" | No | No (fixed at bottom) |

### Insert button behavior

1. Click "+" on row X
2. Create new editable row directly above X
3. New row inherits date from row X (the one whose "+" was clicked)
4. New row gets auto-calculated ODO based on position
5. On save: assigns `sort_order = X.sort_order`, shifts X and below down by 1

### Header "Nový záznam" button

- Unchanged behavior (adds at top)
- Disabled while any row is being edited

### Error handling

- Backend failure on reorder: Revert UI to previous order, show alert
- Backend failure on insert: Remove unsaved row, show alert

---

## 7. Files to Modify

### Backend (Rust)
- `src-tauri/src/models.rs` - Add `sort_order` field
- `src-tauri/src/db.rs` - Update queries, add migration
- `src-tauri/src/lib.rs` - Add `reorder_trip` command, update `create_trip`

### Frontend (Svelte/TS)
- `src/lib/types.ts` - Add `sort_order` to Trip interface
- `src/lib/api.ts` - Add `reorderTrip()`, update `createTrip()`
- `src/lib/components/TripGrid.svelte` - Add dndzone, handle reorder
- `src/lib/components/TripRow.svelte` - Redesign Actions column with icons

### New files
- None (all changes to existing files)

### Dependencies
- `svelte-dnd-action` (npm)
