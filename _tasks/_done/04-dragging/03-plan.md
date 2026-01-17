**Date:** 2025-12-23
**Subject:** Drag-and-drop reordering implementation plan
**Status:** Planning

# Drag-and-Drop Reordering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable users to drag-and-drop trips to reorder them, insert new entries at any position, with auto-recalculating ODO and dates.

**Architecture:** Add `sort_order` field to Trip model for explicit ordering. Use `svelte-dnd-action` library for drag-drop UX. Redesign Actions column with icon buttons (+, trash, drag handle).

**Tech Stack:** Rust/SQLite (backend), SvelteKit/TypeScript (frontend), svelte-dnd-action (drag-drop)

---

## Task 1: Add sort_order to Trip Model (Backend)

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/db.rs`

**Step 1: Add sort_order field to Trip struct**

In `src-tauri/src/models.rs`, add `sort_order` to the Trip struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trip {
    pub id: Uuid,
    pub vehicle_id: Uuid,
    pub date: NaiveDate,
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub odometer: f64,
    pub purpose: String,
    pub fuel_liters: Option<f64>,
    pub fuel_cost_eur: Option<f64>,
    pub other_costs_eur: Option<f64>,
    pub other_costs_note: Option<String>,
    pub sort_order: i32,  // NEW FIELD
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Step 2: Run tests to see what breaks**

Run: `cd src-tauri && cargo test`
Expected: Multiple test failures due to missing `sort_order` field in Trip construction

**Step 3: Update db.rs - add migration for sort_order column**

In `src-tauri/src/db.rs`, add migration in `run_migrations()`:

```rust
fn run_migrations(&self) -> Result<()> {
    let conn = self.conn.lock().unwrap();

    // Run initial schema migration
    conn.execute_batch(include_str!("../migrations/001_initial.sql"))?;

    // Run migration to add initial_odometer column (ignore if already exists)
    let _ = conn.execute(
        "ALTER TABLE vehicles ADD COLUMN initial_odometer REAL NOT NULL DEFAULT 0",
        [],
    );

    // Run migration to add sort_order column (ignore if already exists)
    let _ = conn.execute(
        "ALTER TABLE trips ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0",
        [],
    );

    // Initialize sort_order for existing trips based on chronological order
    // This runs every time but only affects rows where sort_order = 0
    let _ = conn.execute_batch(
        "UPDATE trips SET sort_order = (
            SELECT COUNT(*) FROM trips t2
            WHERE t2.vehicle_id = trips.vehicle_id
            AND (t2.date > trips.date OR (t2.date = trips.date AND t2.odometer > trips.odometer))
        ) WHERE sort_order = 0"
    );

    Ok(())
}
```

**Step 4: Update db.rs - modify create_trip to include sort_order**

Find `create_trip` function and update:

```rust
pub fn create_trip(&self, trip: &Trip) -> Result<()> {
    let conn = self.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO trips (id, vehicle_id, date, origin, destination, distance_km, odometer, purpose, fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note, sort_order, created_at, updated_at)
         VALUES (:id, :vehicle_id, :date, :origin, :destination, :distance_km, :odometer, :purpose, :fuel_liters, :fuel_cost_eur, :other_costs_eur, :other_costs_note, :sort_order, :created_at, :updated_at)",
        rusqlite::named_params! {
            ":id": trip.id.to_string(),
            ":vehicle_id": trip.vehicle_id.to_string(),
            ":date": trip.date.to_string(),
            ":origin": trip.origin,
            ":destination": trip.destination,
            ":distance_km": trip.distance_km,
            ":odometer": trip.odometer,
            ":purpose": trip.purpose,
            ":fuel_liters": trip.fuel_liters,
            ":fuel_cost_eur": trip.fuel_cost_eur,
            ":other_costs_eur": trip.other_costs_eur,
            ":other_costs_note": trip.other_costs_note,
            ":sort_order": trip.sort_order,
            ":created_at": trip.created_at.to_rfc3339(),
            ":updated_at": trip.updated_at.to_rfc3339(),
        },
    )?;
    Ok(())
}
```

**Step 5: Update db.rs - modify get trip queries to include sort_order**

Update all trip query functions to SELECT and ORDER BY sort_order. In `get_trips_for_vehicle`:

```rust
pub fn get_trips_for_vehicle(&self, vehicle_id: &str) -> Result<Vec<Trip>> {
    let conn = self.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, vehicle_id, date, origin, destination, distance_km, odometer, purpose,
                fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note, sort_order, created_at, updated_at
         FROM trips WHERE vehicle_id = ?1
         ORDER BY sort_order ASC",
    )?;
    // ... update row mapping to include sort_order at index 12
}
```

**Step 6: Update db.rs - modify update_trip to include sort_order**

```rust
pub fn update_trip(&self, trip: &Trip) -> Result<()> {
    let conn = self.conn.lock().unwrap();
    conn.execute(
        "UPDATE trips SET date = :date, origin = :origin, destination = :destination,
         distance_km = :distance_km, odometer = :odometer, purpose = :purpose,
         fuel_liters = :fuel_liters, fuel_cost_eur = :fuel_cost_eur,
         other_costs_eur = :other_costs_eur, other_costs_note = :other_costs_note,
         sort_order = :sort_order, updated_at = :updated_at
         WHERE id = :id",
        rusqlite::named_params! {
            // ... include :sort_order
        },
    )?;
    Ok(())
}
```

**Step 7: Fix all test Trip constructions to include sort_order: 0**

Update all test files where Trip is constructed to include `sort_order: 0`.

**Step 8: Run tests to verify**

Run: `cd src-tauri && cargo test`
Expected: All 61 tests pass

**Step 9: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/db.rs
git commit -m "feat(db): add sort_order field to Trip model"
```

---

## Task 2: Add sort_order to Frontend Types

**Files:**
- Modify: `src/lib/types.ts`

**Step 1: Add sort_order to Trip interface**

```typescript
export interface Trip {
	id: string;
	vehicle_id: string;
	date: string;
	origin: string;
	destination: string;
	distance_km: number;
	odometer: number;
	purpose: string;
	fuel_liters?: number | null;
	fuel_cost_eur?: number | null;
	other_costs_eur?: number | null;
	other_costs_note?: string | null;
	sort_order: number;  // NEW FIELD
	created_at: string;
	updated_at: string;
}
```

**Step 2: Commit**

```bash
git add src/lib/types.ts
git commit -m "feat(types): add sort_order to Trip interface"
```

---

## Task 3: Add reorder_trip Backend Command

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/db.rs`

**Step 1: Add reorder_trips command to commands.rs**

```rust
#[tauri::command]
pub fn reorder_trip(
    db: State<Database>,
    trip_id: String,
    new_sort_order: i32,
    new_date: String,
) -> Result<Vec<Trip>, String> {
    let trip_uuid = Uuid::parse_str(&trip_id).map_err(|e| e.to_string())?;
    let parsed_date = NaiveDate::parse_from_str(&new_date, "%Y-%m-%d").map_err(|e| e.to_string())?;

    // Get the trip to find its vehicle_id
    let trip = db.get_trip(&trip_id)
        .map_err(|e| e.to_string())?
        .ok_or("Trip not found")?;

    // Reorder trips in database
    db.reorder_trip(&trip_id, new_sort_order, parsed_date)
        .map_err(|e| e.to_string())?;

    // Return updated trip list
    db.get_trips_for_vehicle(&trip.vehicle_id.to_string())
        .map_err(|e| e.to_string())
}
```

**Step 2: Add reorder_trip to db.rs**

```rust
pub fn reorder_trip(&self, trip_id: &str, new_sort_order: i32, new_date: NaiveDate) -> Result<()> {
    let conn = self.conn.lock().unwrap();

    // Get current trip info
    let (vehicle_id, old_sort_order): (String, i32) = conn.query_row(
        "SELECT vehicle_id, sort_order FROM trips WHERE id = ?1",
        [trip_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    if old_sort_order < new_sort_order {
        // Moving down: decrement sort_order for trips between old and new position
        conn.execute(
            "UPDATE trips SET sort_order = sort_order - 1
             WHERE vehicle_id = ?1 AND sort_order > ?2 AND sort_order <= ?3",
            rusqlite::params![vehicle_id, old_sort_order, new_sort_order],
        )?;
    } else if old_sort_order > new_sort_order {
        // Moving up: increment sort_order for trips between new and old position
        conn.execute(
            "UPDATE trips SET sort_order = sort_order + 1
             WHERE vehicle_id = ?1 AND sort_order >= ?2 AND sort_order < ?3",
            rusqlite::params![vehicle_id, new_sort_order, old_sort_order],
        )?;
    }

    // Update the moved trip
    conn.execute(
        "UPDATE trips SET sort_order = ?1, date = ?2, updated_at = ?3 WHERE id = ?4",
        rusqlite::params![new_sort_order, new_date.to_string(), Utc::now().to_rfc3339(), trip_id],
    )?;

    Ok(())
}
```

**Step 3: Register command in lib.rs**

Add `reorder_trip` to the `invoke_handler` in `src-tauri/src/lib.rs`.

**Step 4: Run tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/db.rs src-tauri/src/lib.rs
git commit -m "feat(api): add reorder_trip command"
```

---

## Task 4: Add reorderTrip to Frontend API

**Files:**
- Modify: `src/lib/api.ts`

**Step 1: Add reorderTrip function**

```typescript
export async function reorderTrip(
	tripId: string,
	newSortOrder: number,
	newDate: string
): Promise<Trip[]> {
	return await invoke('reorder_trip', {
		tripId,
		newSortOrder,
		newDate
	});
}
```

**Step 2: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(api): add reorderTrip frontend function"
```

---

## Task 5: Install svelte-dnd-action

**Step 1: Install package**

Run: `npm install svelte-dnd-action`

**Step 2: Commit**

```bash
git add package.json package-lock.json
git commit -m "deps: add svelte-dnd-action for drag-drop"
```

---

## Task 6: Redesign TripRow Actions Column

**Files:**
- Modify: `src/lib/components/TripRow.svelte`

**Step 1: Add icon SVGs and new action buttons**

Replace the actions section in TripRow.svelte. Add new props and handlers:

```svelte
<script lang="ts">
	// ... existing imports ...

	export let onInsertAbove: () => void = () => {};
	export let dragDisabled: boolean = false;
</script>
```

**Step 2: Replace actions TD in non-editing mode**

```svelte
{:else if trip}
	<tr on:dblclick={handleEdit}>
		<!-- ... existing TDs ... -->
		<td class="actions">
			<button
				class="icon-btn insert"
				on:click|stopPropagation={onInsertAbove}
				title="Vložiť záznam nad"
			>
				<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<line x1="12" y1="5" x2="12" y2="19"></line>
					<line x1="5" y1="12" x2="19" y2="12"></line>
				</svg>
			</button>
			<button
				class="icon-btn delete"
				on:click|stopPropagation={handleDeleteClick}
				title="Odstrániť záznam"
			>
				<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<polyline points="3 6 5 6 21 6"></polyline>
					<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
				</svg>
			</button>
			{#if !dragDisabled}
				<div class="drag-handle" title="Presunúť záznam">
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
						<circle cx="9" cy="6" r="1.5"></circle>
						<circle cx="15" cy="6" r="1.5"></circle>
						<circle cx="9" cy="12" r="1.5"></circle>
						<circle cx="15" cy="12" r="1.5"></circle>
						<circle cx="9" cy="18" r="1.5"></circle>
						<circle cx="15" cy="18" r="1.5"></circle>
					</svg>
				</div>
			{/if}
		</td>
	</tr>
{/if}
```

**Step 3: Add styles for icon buttons**

```css
.icon-btn {
	background: none;
	border: none;
	padding: 0.25rem;
	cursor: pointer;
	color: #9e9e9e;
	border-radius: 4px;
	transition: color 0.2s, background-color 0.2s;
}

.icon-btn:hover {
	background-color: rgba(0, 0, 0, 0.05);
}

.icon-btn.insert:hover {
	color: #3498db;
}

.icon-btn.delete:hover {
	color: #f44336;
}

.drag-handle {
	display: inline-flex;
	padding: 0.25rem;
	cursor: grab;
	color: #9e9e9e;
	border-radius: 4px;
	transition: color 0.2s, background-color 0.2s;
}

.drag-handle:hover {
	color: #616161;
	background-color: rgba(0, 0, 0, 0.05);
}

.actions {
	display: flex;
	gap: 0.25rem;
	justify-content: center;
	align-items: center;
}
```

**Step 4: Test visually**

Run: `npm run tauri dev`
Verify: Icons display correctly, hover states work

**Step 5: Commit**

```bash
git add src/lib/components/TripRow.svelte
git commit -m "feat(ui): redesign Actions column with icon buttons"
```

---

## Task 7: Implement Drag-and-Drop in TripGrid

**Files:**
- Modify: `src/lib/components/TripGrid.svelte`

**Step 1: Import dndzone and add state**

```svelte
<script lang="ts">
	import { dndzone, SHADOW_PLACEHOLDER_ITEM_ID } from 'svelte-dnd-action';
	import { flip } from 'svelte/animate';
	import { reorderTrip } from '$lib/api';

	// ... existing code ...

	let editingTripId: string | null = null;

	$: dragDisabled = showNewRow || editingTripId !== null;

	// Items for dndzone (needs id property)
	$: dndItems = sortedTrips.map(t => ({ ...t }));

	function handleDndConsider(e: CustomEvent) {
		dndItems = e.detail.items;
	}

	async function handleDndFinalize(e: CustomEvent) {
		const newItems = e.detail.items.filter(
			(item: any) => item.id !== SHADOW_PLACEHOLDER_ITEM_ID
		);
		dndItems = newItems;

		// Find which item moved and to where
		const info = e.detail.info;
		if (info.trigger === 'droppedIntoZone') {
			const movedTripId = info.id;
			const newIndex = newItems.findIndex((t: Trip) => t.id === movedTripId);

			if (newIndex !== -1) {
				// Get new date from trip above (or keep same if at top)
				const newDate = newIndex > 0
					? newItems[newIndex - 1].date
					: newItems[newIndex].date;

				try {
					await reorderTrip(movedTripId, newIndex, newDate);
					onTripsChanged();
				} catch (error) {
					console.error('Failed to reorder trip:', error);
					alert('Nepodarilo sa zmeniť poradie');
					onTripsChanged(); // Refresh to revert
				}
			}
		}
	}

	function handleEditStart(tripId: string) {
		editingTripId = tripId;
	}

	function handleEditEnd() {
		editingTripId = null;
	}
</script>
```

**Step 2: Update sorting to use sort_order**

```svelte
// Sort trips by sort_order ascending (0 = top/newest)
$: sortedTrips = [...trips].sort((a, b) => a.sort_order - b.sort_order);
```

**Step 3: Wrap tbody with dndzone**

```svelte
<tbody
	use:dndzone={{
		items: dndItems,
		dragDisabled,
		flipDurationMs: 200,
		dropTargetStyle: { outline: '2px dashed #3498db' }
	}}
	on:consider={handleDndConsider}
	on:finalize={handleDndFinalize}
>
	{#if showNewRow}
		<!-- New row (not draggable) -->
	{/if}
	{#each dndItems as trip (trip.id)}
		<tr animate:flip={{ duration: 200 }}>
			<TripRow
				{trip}
				{routes}
				isNew={false}
				dragDisabled={dragDisabled}
				onEditStart={() => handleEditStart(trip.id)}
				onEditEnd={handleEditEnd}
				onInsertAbove={() => handleInsertAbove(trip)}
				<!-- ... other props ... -->
			/>
		</tr>
	{/each}
</tbody>
<!-- Prvý záznam row in separate tbody (not part of dndzone) -->
<tbody>
	<tr class="first-record">...</tr>
</tbody>
```

**Step 4: Add handleInsertAbove function**

```svelte
async function handleInsertAbove(targetTrip: Trip) {
	// Create new row that will be inserted above targetTrip
	insertAtSortOrder = targetTrip.sort_order;
	insertDate = targetTrip.date;
	showNewRow = true;
}

let insertAtSortOrder: number | null = null;
let insertDate: string | null = null;
```

**Step 5: Update handleSaveNew to use insertAtSortOrder**

```svelte
async function handleSaveNew(tripData: Partial<Trip>) {
	try {
		// If inserting at specific position, shift other trips first
		if (insertAtSortOrder !== null) {
			// Backend will handle shifting when we pass sort_order
			// For now, create at position 0 and reorder after
		}

		await createTrip(
			vehicleId,
			tripData.date!,
			// ... other fields ...
		);
		showNewRow = false;
		insertAtSortOrder = null;
		insertDate = null;
		onTripsChanged();
		await loadRoutes();
	} catch (error) {
		// ... error handling ...
	}
}
```

**Step 6: Test drag-and-drop**

Run: `npm run tauri dev`
Verify:
- Drag handle shows on each row
- Dragging shows drop indicator
- Drop reorders the list
- Date updates to match position

**Step 7: Commit**

```bash
git add src/lib/components/TripGrid.svelte
git commit -m "feat(ui): implement drag-and-drop reordering"
```

---

## Task 8: Update TripRow for Edit State Callbacks

**Files:**
- Modify: `src/lib/components/TripRow.svelte`

**Step 1: Add edit state callbacks**

```svelte
<script lang="ts">
	export let onEditStart: () => void = () => {};
	export let onEditEnd: () => void = () => {};

	function handleEdit() {
		isEditing = true;
		onEditStart();
	}

	function handleSave() {
		// ... existing save logic ...
		isEditing = false;
		onEditEnd();
	}

	function handleCancel() {
		// ... existing cancel logic ...
		if (!isNew) {
			isEditing = false;
			onEditEnd();
		}
	}
</script>
```

**Step 2: Commit**

```bash
git add src/lib/components/TripRow.svelte
git commit -m "feat(ui): add edit state callbacks to TripRow"
```

---

## Task 9: Implement ODO Recalculation After Reorder

**Files:**
- Modify: `src/lib/components/TripGrid.svelte`

**Step 1: Add recalculateAllOdo function**

```svelte
async function recalculateAllOdo() {
	// Sort by sort_order, then reverse for chronological (oldest first)
	const chronological = [...trips]
		.sort((a, b) => a.sort_order - b.sort_order)
		.reverse();

	let runningOdo = initialOdometer;
	for (const trip of chronological) {
		runningOdo += trip.distance_km;
		if (Math.abs(trip.odometer - runningOdo) > 0.01) {
			await updateTrip(
				trip.id,
				trip.date,
				trip.origin,
				trip.destination,
				trip.distance_km,
				runningOdo,
				trip.purpose,
				trip.fuel_liters,
				trip.fuel_cost_eur,
				trip.other_costs_eur,
				trip.other_costs_note
			);
		}
	}
}
```

**Step 2: Call recalculateAllOdo after successful reorder**

Update `handleDndFinalize`:

```svelte
try {
	await reorderTrip(movedTripId, newIndex, newDate);
	await recalculateAllOdo();
	onTripsChanged();
} catch (error) {
	// ...
}
```

**Step 3: Test ODO recalculation**

Run: `npm run tauri dev`
Verify: After reordering, ODO values update correctly

**Step 4: Commit**

```bash
git add src/lib/components/TripGrid.svelte
git commit -m "feat(calc): recalculate ODO after drag-drop reorder"
```

---

## Task 10: Update create_trip to Support Insert Position

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/db.rs`
- Modify: `src/lib/api.ts`

**Step 1: Add insert_at_sort_order parameter to create_trip command**

```rust
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_trip(
    db: State<Database>,
    vehicle_id: String,
    date: String,
    // ... other params ...
    insert_at_sort_order: Option<i32>,  // NEW
) -> Result<Trip, String> {
    // ... existing validation ...

    // Determine sort_order
    let sort_order = if let Some(pos) = insert_at_sort_order {
        // Shift existing trips down
        db.shift_trips_from_position(&vehicle_id, pos)
            .map_err(|e| e.to_string())?;
        pos
    } else {
        // Insert at top (sort_order = 0), shift all others
        db.shift_trips_from_position(&vehicle_id, 0)
            .map_err(|e| e.to_string())?;
        0
    };

    let trip = Trip {
        // ... existing fields ...
        sort_order,
        // ...
    };

    // ... rest of function ...
}
```

**Step 2: Add shift_trips_from_position to db.rs**

```rust
pub fn shift_trips_from_position(&self, vehicle_id: &str, from_position: i32) -> Result<()> {
    let conn = self.conn.lock().unwrap();
    conn.execute(
        "UPDATE trips SET sort_order = sort_order + 1
         WHERE vehicle_id = ?1 AND sort_order >= ?2",
        rusqlite::params![vehicle_id, from_position],
    )?;
    Ok(())
}
```

**Step 3: Update frontend createTrip to accept insertAtSortOrder**

```typescript
export async function createTrip(
	vehicleId: string,
	date: string,
	// ... other params ...
	insertAtSortOrder?: number | null
): Promise<Trip> {
	return await invoke('create_trip', {
		vehicleId,
		date,
		// ... other params ...
		insertAtSortOrder
	});
}
```

**Step 4: Update TripGrid to pass insertAtSortOrder**

```svelte
await createTrip(
	vehicleId,
	tripData.date!,
	tripData.origin!,
	tripData.destination!,
	tripData.distance_km!,
	tripData.odometer!,
	tripData.purpose!,
	tripData.fuel_liters,
	tripData.fuel_cost_eur,
	tripData.other_costs_eur,
	null,
	insertAtSortOrder  // Pass the insert position
);
```

**Step 5: Test insert above**

Run: `npm run tauri dev`
Verify: Clicking + on a row creates new row above it

**Step 6: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/db.rs src/lib/api.ts src/lib/components/TripGrid.svelte
git commit -m "feat(api): support inserting trips at specific position"
```

---

## Task 11: Final Testing and Cleanup

**Step 1: Run all backend tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass

**Step 2: Manual E2E testing**

Run: `npm run tauri dev`

Test checklist:
- [ ] Drag row to new position
- [ ] Date updates to match position above
- [ ] ODO recalculates for all affected trips
- [ ] Click + inserts new row above
- [ ] Trash icon deletes row
- [ ] Cannot drag while editing a row
- [ ] Cannot drag new unsaved row
- [ ] Prvý záznam stays at bottom

**Step 3: Run linting**

Run: `npm run lint && npm run format`
Fix any issues.

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete drag-drop reordering implementation"
```

---

## Summary

| Task | Description | Est. Size |
|------|-------------|-----------|
| 1 | Add sort_order to Trip model (backend) | Medium |
| 2 | Add sort_order to frontend types | Small |
| 3 | Add reorder_trip backend command | Medium |
| 4 | Add reorderTrip frontend API | Small |
| 5 | Install svelte-dnd-action | Small |
| 6 | Redesign TripRow Actions column | Medium |
| 7 | Implement drag-drop in TripGrid | Large |
| 8 | Add edit state callbacks to TripRow | Small |
| 9 | Implement ODO recalculation | Medium |
| 10 | Support insert at position | Medium |
| 11 | Final testing and cleanup | Medium |
