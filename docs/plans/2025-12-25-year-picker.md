# Year Picker Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add year picker to treat each year as standalone "kniha jázd" with year-scoped stats and data.

**Architecture:** Add `get_years_with_trips` backend command, modify existing commands to accept year parameter, add year dropdown in header next to vehicle picker, update main page and settings to use year-scoped data.

**Tech Stack:** Rust/Tauri backend, SvelteKit frontend, SQLite database.

---

## Task 1: Backend - Add get_years_with_trips command

**Files:**
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Add database method**

In `src-tauri/src/db.rs`, add after `get_trips_for_vehicle_in_year`:

```rust
pub fn get_years_with_trips(&self, vehicle_id: &str) -> Result<Vec<i32>> {
    let conn = self.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT DISTINCT CAST(strftime('%Y', date) AS INTEGER) as year
         FROM trips WHERE vehicle_id = ?1 ORDER BY year DESC",
    )?;

    let years = stmt
        .query_map([vehicle_id], |row| row.get(0))?
        .collect::<std::result::Result<Vec<i32>, _>>()?;

    Ok(years)
}
```

**Step 2: Add Tauri command**

In `src-tauri/src/commands.rs`, add:

```rust
#[tauri::command]
pub fn get_years_with_trips(
    db: State<Database>,
    vehicle_id: String,
) -> Result<Vec<i32>, String> {
    db.get_years_with_trips(&vehicle_id).map_err(|e| e.to_string())
}
```

**Step 3: Register command**

In `src-tauri/src/lib.rs`, add `commands::get_years_with_trips` to the `generate_handler!` macro.

**Step 4: Run tests and verify compilation**

```bash
cd src-tauri && cargo test && cargo build
```

**Step 5: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(backend): add get_years_with_trips command"
```

---

## Task 2: Backend - Add year parameter to calculate_trip_stats

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Step 1: Update calculate_trip_stats signature and implementation**

Change from:
```rust
pub fn calculate_trip_stats(
    vehicle_id: String,
    db: State<Database>,
) -> Result<TripStats, String> {
```

To:
```rust
pub fn calculate_trip_stats(
    vehicle_id: String,
    year: i32,
    db: State<Database>,
) -> Result<TripStats, String> {
```

**Step 2: Update trips query**

Replace:
```rust
let trips = db.get_trips_for_vehicle(&vehicle_id).map_err(|e| e.to_string())?;
```

With:
```rust
let trips = db.get_trips_for_vehicle_in_year(&vehicle_id, year).map_err(|e| e.to_string())?;
```

**Step 3: Verify compilation**

```bash
cd src-tauri && cargo build
```

**Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(backend): add year parameter to calculate_trip_stats"
```

---

## Task 3: Backend - Add year parameter to get_trip_grid_data

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Step 1: Update get_trip_grid_data signature**

Change from:
```rust
pub fn get_trip_grid_data(
    db: State<Database>,
    vehicle_id: String,
) -> Result<TripGridData, String> {
```

To:
```rust
pub fn get_trip_grid_data(
    db: State<Database>,
    vehicle_id: String,
    year: i32,
) -> Result<TripGridData, String> {
```

**Step 2: Update trips query**

Replace:
```rust
let trips = db.get_trips_for_vehicle(&vehicle_id).map_err(|e| e.to_string())?;
```

With:
```rust
let trips = db.get_trips_for_vehicle_in_year(&vehicle_id, year).map_err(|e| e.to_string())?;
```

**Step 3: Verify compilation**

```bash
cd src-tauri && cargo build
```

**Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(backend): add year parameter to get_trip_grid_data"
```

---

## Task 4: Frontend - Add API functions

**Files:**
- Modify: `src/lib/api.ts`

**Step 1: Add getYearsWithTrips function**

```typescript
export async function getYearsWithTrips(vehicleId: string): Promise<number[]> {
	return await invoke('get_years_with_trips', { vehicleId });
}
```

**Step 2: Update getTripGridData signature**

Change from:
```typescript
export async function getTripGridData(vehicleId: string): Promise<TripGridData> {
	return await invoke('get_trip_grid_data', { vehicleId });
}
```

To:
```typescript
export async function getTripGridData(vehicleId: string, year: number): Promise<TripGridData> {
	return await invoke('get_trip_grid_data', { vehicleId, year });
}
```

**Step 3: Update calculateTripStats signature**

Change from:
```typescript
export async function calculateTripStats(vehicleId: string): Promise<TripStats> {
	return await invoke('calculate_trip_stats', { vehicleId });
}
```

To:
```typescript
export async function calculateTripStats(vehicleId: string, year: number): Promise<TripStats> {
	return await invoke('calculate_trip_stats', { vehicleId, year });
}
```

**Step 4: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(api): add getYearsWithTrips and year params to existing functions"
```

---

## Task 5: Frontend - Add selectedYearStore

**Files:**
- Create: `src/lib/stores/year.ts`

**Step 1: Create the store file**

```typescript
import { writable } from 'svelte/store';

// Initialize to current calendar year
export const selectedYearStore = writable<number>(new Date().getFullYear());

// Helper to reset to current year (used when switching vehicles)
export function resetToCurrentYear(): void {
	selectedYearStore.set(new Date().getFullYear());
}
```

**Step 2: Commit**

```bash
git add src/lib/stores/year.ts
git commit -m "feat(store): add selectedYearStore for year picker state"
```

---

## Task 6: Frontend - Add year dropdown to header

**Files:**
- Modify: `src/routes/+layout.svelte`

**Step 1: Add imports**

Add to script section:
```typescript
import { selectedYearStore, resetToCurrentYear } from '$lib/stores/year';
import { getYearsWithTrips } from '$lib/api';
```

**Step 2: Add years state and loading**

Add after existing state:
```typescript
let availableYears: number[] = [];

async function loadYears() {
	if (!$activeVehicleStore) {
		availableYears = [];
		return;
	}
	try {
		const yearsWithData = await getYearsWithTrips($activeVehicleStore.id);
		const currentYear = new Date().getFullYear();
		// Combine current year with years that have data, deduplicate, sort descending
		const allYears = new Set([currentYear, ...yearsWithData]);
		availableYears = [...allYears].sort((a, b) => b - a);
	} catch (error) {
		console.error('Failed to load years:', error);
		availableYears = [new Date().getFullYear()];
	}
}
```

**Step 3: Update onMount to load years**

```typescript
onMount(async () => {
	try {
		const [vehicles, activeVehicle] = await Promise.all([
			getVehicles(),
			getActiveVehicle()
		]);
		vehiclesStore.set(vehicles);
		activeVehicleStore.set(activeVehicle);
		await loadYears();
	} catch (error) {
		console.error('Failed to load initial data:', error);
	}
});
```

**Step 4: Update vehicle change handler to reset year**

```typescript
async function handleVehicleChange(event: Event) {
	const select = event.target as HTMLSelectElement;
	const vehicleId = select.value;
	if (vehicleId) {
		try {
			await setActiveVehicle(vehicleId);
			const activeVehicle = $vehiclesStore.find((v) => v.id === vehicleId) || null;
			activeVehicleStore.set(activeVehicle);
			resetToCurrentYear();
			await loadYears();
		} catch (error) {
			console.error('Failed to set active vehicle:', error);
		}
	}
}
```

**Step 5: Add year change handler**

```typescript
function handleYearChange(event: Event) {
	const select = event.target as HTMLSelectElement;
	selectedYearStore.set(parseInt(select.value, 10));
}
```

**Step 6: Add year dropdown to template**

After the vehicle-selector div, add:
```svelte
{#if $activeVehicleStore}
	<div class="year-selector">
		<label for="year-select">Rok:</label>
		<select
			id="year-select"
			value={$selectedYearStore}
			onchange={handleYearChange}
		>
			{#each availableYears as year}
				<option value={year}>{year}</option>
			{/each}
		</select>
	</div>
{/if}
```

**Step 7: Add year-selector styles**

```css
.year-selector {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	margin-left: 1rem;
}
```

**Step 8: Update header-content to use flex gap**

Wrap vehicle-selector and year-selector in a container div with class `header-right`:
```svelte
<div class="header-right">
	<div class="vehicle-selector">...</div>
	{#if $activeVehicleStore}
		<div class="year-selector">...</div>
	{/if}
</div>
```

Add style:
```css
.header-right {
	display: flex;
	align-items: center;
	gap: 1rem;
}
```

**Step 9: Commit**

```bash
git add src/routes/+layout.svelte
git commit -m "feat(ui): add year picker dropdown to header"
```

---

## Task 7: Frontend - Update main page to use year

**Files:**
- Modify: `src/routes/+page.svelte`

**Step 1: Add import**

```typescript
import { selectedYearStore } from '$lib/stores/year';
```

**Step 2: Update loadTrips to use year**

Change:
```typescript
trips = await getTrips($activeVehicleStore.id);
stats = await calculateTripStats($activeVehicleStore.id);
```

To:
```typescript
trips = await getTripsForYear($activeVehicleStore.id, $selectedYearStore);
stats = await calculateTripStats($activeVehicleStore.id, $selectedYearStore);
```

**Step 3: Update imports**

Change `getTrips` to `getTripsForYear` in the import line.

**Step 4: Add reactive reload on year change**

Add after the existing vehicle reactive statement:
```typescript
$: if ($activeVehicleStore && $selectedYearStore) {
	loadTrips(true);
}
```

Remove the duplicate vehicle-only reactive statement or combine them.

**Step 5: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat(ui): main page uses selected year for trips and stats"
```

---

## Task 8: Frontend - Update TripGrid to use year

**Files:**
- Modify: `src/lib/components/TripGrid.svelte`

**Step 1: Add year prop**

```typescript
export let year: number = new Date().getFullYear();
```

**Step 2: Update loadGridData to use year**

Change:
```typescript
gridData = await getTripGridData(vehicleId);
```

To:
```typescript
gridData = await getTripGridData(vehicleId, year);
```

**Step 3: Update +page.svelte to pass year**

In `src/routes/+page.svelte`, update TripGrid:
```svelte
<TripGrid
	vehicleId={$activeVehicleStore.id}
	{trips}
	year={$selectedYearStore}
	tankSize={$activeVehicleStore.tank_size_liters}
	tpConsumption={$activeVehicleStore.tp_consumption}
	initialOdometer={$activeVehicleStore.initial_odometer}
	onTripsChanged={handleTripsChanged}
/>
```

**Step 4: Commit**

```bash
git add src/lib/components/TripGrid.svelte src/routes/+page.svelte
git commit -m "feat(ui): TripGrid uses year parameter for grid data"
```

---

## Task 9: Frontend - Update settings export dropdown

**Files:**
- Modify: `src/routes/settings/+page.svelte`

**Step 1: Add import**

```typescript
import { getYearsWithTrips } from '$lib/api';
```

**Step 2: Add years state**

Replace hardcoded years logic with:
```typescript
let exportYears: number[] = [];

async function loadExportYears() {
	if (!$activeVehicleStore) {
		exportYears = [];
		return;
	}
	try {
		exportYears = await getYearsWithTrips($activeVehicleStore.id);
		// Set default to most recent year with data
		if (exportYears.length > 0 && !exportYears.includes(selectedYear)) {
			selectedYear = exportYears[0];
		}
	} catch (error) {
		console.error('Failed to load export years:', error);
		exportYears = [];
	}
}
```

**Step 3: Call loadExportYears on mount and vehicle change**

Add to onMount:
```typescript
await loadExportYears();
```

Add reactive statement:
```typescript
$: if ($activeVehicleStore) {
	loadExportYears();
}
```

**Step 4: Update template**

Replace the hardcoded year loop:
```svelte
{#if exportYears.length > 0}
	<select id="export-year" bind:value={selectedYear}>
		{#each exportYears as year}
			<option value={year}>{year}</option>
		{/each}
	</select>
{:else}
	<p class="no-data">Žiadne dáta na export</p>
{/if}
```

**Step 5: Add no-data style**

```css
.no-data {
	color: #7f8c8d;
	font-style: italic;
	margin: 0;
}
```

**Step 6: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(ui): export dropdown shows only years with data"
```

---

## Task 10: Update DECISIONS.md

**Files:**
- Modify: `DECISIONS.md`

**Step 1: Add decision entry**

Add at the top (after template):
```markdown
## 2025-12-25: Year Picker

### ADR-009: Year-Scoped Vehicle Logbook

**Context:** Each year is a standalone "kniha jázd" for legal purposes.

**Decision:**
- Year picker in header next to vehicle dropdown
- Stats and trips scoped to selected year
- App starts on current calendar year
- Export only shows years with actual data
- ODO carries over from previous year, zostatok starts fresh (full tank assumption)

**Reasoning:** Slovak legal requirements treat each year as independent logbook. Fresh zostatok per year simplifies accounting.
```

**Step 2: Commit**

```bash
git add DECISIONS.md
git commit -m "docs: add ADR-009 for year-scoped logbook"
```

---

## Task 11: Final verification

**Step 1: Run all tests**

```bash
cd src-tauri && cargo test
```

**Step 2: Start dev server and verify**

```bash
npm run tauri dev
```

**Step 3: Manual verification checklist**

- [ ] Year dropdown appears next to vehicle dropdown
- [ ] Current year selected by default
- [ ] Switching years reloads trips and stats
- [ ] Switching vehicles resets to current year
- [ ] Export dropdown only shows years with data
- [ ] Empty year shows empty grid (can add trips)

---

Plan complete and saved to `docs/plans/2025-12-25-year-picker.md`. Two execution options:

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

Which approach?
