# Implementation Plan: Trip Time Field + Hideable Columns

**Date:** 2026-01-26
**Status:** Planning
**Review:** Addressed Critical + Important findings from _plan-review.md

## Overview

This plan implements two related features:
1. **Trip Time Field** — Add departure time to trips (datetime migration)
2. **Hideable Columns** — UI pattern to show/hide optional columns

## Phase 1: Database Migration + Backend (datetime)

### Step 1.1: Create Database Migration
**Files:**
- `src-tauri/migrations/2026-01-27-000000_add_trip_datetime/up.sql`
- `src-tauri/migrations/2026-01-27-000000_add_trip_datetime/down.sql`

**up.sql:**
```sql
-- Add datetime column with default, then populate from existing date
ALTER TABLE trips ADD COLUMN datetime TEXT NOT NULL DEFAULT '';
UPDATE trips SET datetime = date || 'T00:00:00';
```

**down.sql:**
```sql
-- SQLite does not support DROP COLUMN directly.
-- The datetime column will remain but be ignored by older app versions.
-- This is intentional for forward compatibility.
SELECT 1; -- No-op placeholder for Diesel
```

**Verification:** Run `cargo test` in src-tauri to verify migration compiles.

### Step 1.2: Update Diesel Schema
**Files:** `src-tauri/src/schema.rs`

Manually add `datetime` column to trips table (do NOT use `diesel print-schema` as schema is manually maintained):

```rust
diesel::table! {
    trips (id) {
        // ... existing columns ...
        date -> Text,
        datetime -> Text,  // NEW: Add after date column
        // ... rest of columns ...
    }
}
```

**Verification:** `cargo check` passes.

### Step 1.3: Update Models
**Files:** `src-tauri/src/models.rs`

**Trip struct** — Keep `date` for backward compatibility, add `datetime`:
```rust
pub struct Trip {
    // ... existing fields ...
    pub date: NaiveDate,           // KEEP: derived from datetime for compatibility
    pub datetime: NaiveDateTime,   // NEW: source of truth
    // ...
}
```

**TripRow struct** — Add datetime column:
```rust
pub struct TripRow {
    // ... existing ...
    pub date: String,
    pub datetime: String,  // NEW
    // ...
}
```

**NewTripRow struct** — Add datetime:
```rust
pub struct NewTripRow<'a> {
    // ... existing ...
    pub date: &'a str,
    pub datetime: &'a str,  // NEW
    // ...
}
```

**From<TripRow> for Trip** — Parse datetime, derive date:
```rust
impl From<TripRow> for Trip {
    fn from(row: TripRow) -> Self {
        let datetime = NaiveDateTime::parse_from_str(&row.datetime, "%Y-%m-%dT%H:%M:%S")
            .unwrap_or_else(|_| {
                // Fallback: parse date-only and add 00:00:00
                NaiveDate::parse_from_str(&row.date, "%Y-%m-%d")
                    .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                    .unwrap_or_else(|_| Utc::now().naive_utc())
            });
        let date = datetime.date();

        Trip {
            date,
            datetime,
            // ... rest of fields ...
        }
    }
}
```

**Tests** (`models.rs` or `models_tests.rs`):
- Test valid datetime parsing: `"2026-01-15T08:30:00"` → correct NaiveDateTime
- Test fallback for legacy data: `date="2026-01-15"`, `datetime=""` → derives correctly
- Test edge case: midnight `"2026-01-15T00:00:00"` parses correctly

**Verification:** `cargo test` passes.

### Step 1.4: Update Database CRUD
**Files:** `src-tauri/src/db.rs`

**create_trip** — Accept datetime, also write to date column for year filtering:
```rust
pub fn create_trip(
    conn: &mut SqliteConnection,
    vehicle_id: &str,
    datetime: NaiveDateTime,  // Changed from date: NaiveDate
    // ... other params ...
) -> Result<Trip, diesel::result::Error> {
    let date_str = datetime.date().format("%Y-%m-%d").to_string();
    let datetime_str = datetime.format("%Y-%m-%dT%H:%M:%S").to_string();

    // Insert with both date and datetime columns
    // date column maintained for backward compatibility with year filtering
}
```

**update_trip** — Same pattern, update both columns.

**Year filtering** — Continue using `date` column (no change needed to existing queries):
```rust
// Existing query pattern still works:
.filter(trips::date.like(format!("{}-%", year)))
```

**Tests** (`db_tests.rs`):
- Test create trip with datetime
- Test update trip with datetime
- Test year filtering still works
- Test round-trip: create → read → verify datetime preserved

**Verification:** `cargo test` passes, specifically db_tests.

### Step 1.5: Update Commands
**Files:** `src-tauri/src/commands.rs`

**API approach:** Accept separate `date: String` and `time: String` params (easier frontend binding):

```rust
#[tauri::command]
pub fn create_trip(
    // ... existing params ...
    date: String,   // "2026-01-15"
    time: String,   // "08:30" or "" for 00:00
    // ... rest ...
) -> Result<Trip, String> {
    // Combine into datetime
    let time_part = if time.is_empty() { "00:00" } else { &time };
    let datetime_str = format!("{}T{}:00", date, time_part);
    let datetime = NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%dT%H:%M:%S")
        .map_err(|e| format!("Invalid datetime: {}", e))?;

    // Call db::create_trip with datetime
}
```

**update_trip** — Same pattern.

**Tests** (`commands_tests.rs`):
- Test create with time: `date="2026-01-15"`, `time="08:30"` → datetime correct
- Test create without time: `date="2026-01-15"`, `time=""` → defaults to 00:00
- Test invalid time format handling

**Verification:** `cargo test` passes.

## Phase 2: Backend (Hidden Columns Settings)

### Step 2.1: Extend LocalSettings
**Files:** `src-tauri/src/settings.rs`

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalSettings {
    // ... existing fields ...
    pub hidden_columns: Option<Vec<String>>,
}
```

**Tests** (in `settings.rs` test module):
- Test empty array serializes: `{"hidden_columns": []}`
- Test with values: `{"hidden_columns": ["time", "fuelConsumed"]}`
- Test missing field defaults to None
- Test unknown column names preserved (future-proofing)

**Verification:** `cargo test` passes.

### Step 2.2: Add Hidden Columns Commands
**Files:** `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`

```rust
#[tauri::command]
pub fn get_hidden_columns(app_handle: AppHandle) -> Result<Vec<String>, String> {
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_data_dir);
    Ok(settings.hidden_columns.unwrap_or_default())
}

#[tauri::command]
pub fn set_hidden_columns(app_handle: AppHandle, columns: Vec<String>) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.hidden_columns = Some(columns);
    settings.save(&app_data_dir).map_err(|e| e.to_string())
}
```

**Register in lib.rs** — Add to `invoke_handler`:
```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    commands::get_hidden_columns,
    commands::set_hidden_columns,
])
```

**Tests** (`commands_tests.rs`):
- Test get_hidden_columns returns empty by default
- Test set_hidden_columns persists values
- Test round-trip: set → get → verify

**Verification:** `cargo test` passes.

## Phase 3: Frontend (Time Field)

### Step 3.1: Update TypeScript Types
**Files:** `src/lib/types.ts`

Keep `date` for display, add `time` helper (parsed from datetime):
```typescript
export interface Trip {
    // ... existing ...
    date: string;      // "2026-01-15" - KEEP for display
    datetime: string;  // "2026-01-15T08:30:00" - NEW: from backend
    // ...
}

// Helper to extract time from datetime
export function extractTime(datetime: string): string {
    // "2026-01-15T08:30:00" → "08:30"
    const match = datetime.match(/T(\d{2}:\d{2})/);
    return match ? match[1] : '00:00';
}
```

### Step 3.2: Update API Functions
**Files:** `src/lib/api.ts`

```typescript
export async function createTrip(
    vehicleId: string,
    date: string,      // "2026-01-15"
    time: string,      // "08:30" or ""
    origin: string,
    // ... rest ...
): Promise<Trip> {
    return invoke('create_trip', {
        vehicleId,
        date,
        time,
        origin,
        // ...
    });
}

// Same pattern for updateTrip
```

### Step 3.3: Update TripRow Component
**Files:** `src/lib/components/TripRow.svelte`

**Add to formData:**
```typescript
let formData = {
    date: trip?.date || defaultDate,
    time: trip ? extractTime(trip.datetime) : '',  // NEW
    // ... rest ...
};
```

**Display mode** — New cell after date:
```svelte
<td>{new Date(trip.date).toLocaleDateString('sk-SK')}</td>
<td>{extractTime(trip.datetime)}</td>  <!-- NEW -->
```

**Edit mode** — Time input:
```svelte
<td>
    <input type="date" bind:value={formData.date} />
</td>
<td>
    <input type="time" bind:value={formData.time} />  <!-- NEW -->
</td>
```

**Save handler** — Pass time to API:
```typescript
onSave({
    ...formData,
    time: formData.time || '00:00',
});
```

### Step 3.4: Update TripGrid Component
**Files:** `src/lib/components/TripGrid.svelte`

**Header** — Add after Dátum:
```svelte
<th>{$LL.trips.columns.date()}</th>
<th>{$LL.trips.columns.time()}</th>  <!-- NEW -->
```

**Column widths** — Adjust to accommodate new column (reduce other columns slightly).

**First record row** — Add time cell:
```svelte
<td>{trip.date.split('-').reverse().join('.')}</td>
<td>00:00</td>  <!-- NEW -->
```

### Step 3.5: Update i18n
**Files:** `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

```typescript
// Slovak
trips: {
    columns: {
        // ... existing ...
        time: 'Čas',
    },
}

// English
trips: {
    columns: {
        // ... existing ...
        time: 'Time',
    },
}
```

**Verification:** `npm run dev` — app loads, time column visible.

## Phase 4: Frontend (Hideable Columns)

### Step 4.1: Add API Functions for Hidden Columns
**Files:** `src/lib/api.ts`

```typescript
export async function getHiddenColumns(): Promise<string[]> {
    return invoke('get_hidden_columns');
}

export async function setHiddenColumns(columns: string[]): Promise<void> {
    return invoke('set_hidden_columns', { columns });
}
```

### Step 4.2: Create ColumnVisibilityDropdown Component
**Files:** `src/lib/components/ColumnVisibilityDropdown.svelte` (new)

```svelte
<script lang="ts">
    import { getHiddenColumns, setHiddenColumns } from '$lib/api';
    import LL from '$lib/i18n/i18n-svelte';

    export let hiddenColumns: string[] = [];
    export let onChange: (columns: string[]) => void;

    let isOpen = false;

    const HIDEABLE_COLUMNS = [
        { id: 'time', label: () => $LL.trips.columnVisibility.time() },
        { id: 'fuelConsumed', label: () => $LL.trips.columnVisibility.fuelConsumed() },
        { id: 'fuelRemaining', label: () => $LL.trips.columnVisibility.fuelRemaining() },
        { id: 'otherCosts', label: () => $LL.trips.columnVisibility.otherCosts() },
        { id: 'otherCostsNote', label: () => $LL.trips.columnVisibility.otherCostsNote() },
    ];

    $: hiddenCount = hiddenColumns.length;

    async function toggleColumn(id: string) {
        const newHidden = hiddenColumns.includes(id)
            ? hiddenColumns.filter(c => c !== id)
            : [...hiddenColumns, id];
        await setHiddenColumns(newHidden);
        onChange(newHidden);
    }
</script>

<div class="column-visibility">
    <button on:click={() => isOpen = !isOpen} title={$LL.trips.columnVisibility.title()}>
        {#if hiddenCount > 0}
            <!-- eye-off icon + badge -->
            <svg>...</svg>
            <span class="badge">{hiddenCount}</span>
        {:else}
            <!-- eye icon -->
            <svg>...</svg>
        {/if}
    </button>

    {#if isOpen}
        <div class="dropdown">
            <div class="dropdown-header">{$LL.trips.columnVisibility.title()}</div>
            {#each HIDEABLE_COLUMNS as col}
                <label>
                    <input
                        type="checkbox"
                        checked={!hiddenColumns.includes(col.id)}
                        on:change={() => toggleColumn(col.id)}
                    />
                    {col.label()}
                </label>
            {/each}
        </div>
    {/if}
</div>
```

### Step 4.3: Integrate into TripGrid Header
**Files:** `src/lib/components/TripGrid.svelte`

```svelte
<script>
    import ColumnVisibilityDropdown from './ColumnVisibilityDropdown.svelte';
    import { getHiddenColumns } from '$lib/api';

    let hiddenColumns: string[] = [];

    onMount(async () => {
        hiddenColumns = await getHiddenColumns();
        // ... existing onMount code ...
    });
</script>

<div class="header-actions">
    <button class="new-record">...</button>
    <SegmentedToggle ... />
    <ColumnVisibilityDropdown
        {hiddenColumns}
        onChange={(cols) => hiddenColumns = cols}
    />
</div>
```

### Step 4.4: Conditionally Render Columns
**Files:** `src/lib/components/TripGrid.svelte`, `src/lib/components/TripRow.svelte`

**TripGrid** — Pass hiddenColumns, conditionally render headers:
```svelte
{#if !hiddenColumns.includes('time')}
    <th>{$LL.trips.columns.time()}</th>
{/if}
```

**TripRow** — Accept prop, conditionally render cells:
```svelte
<script>
    export let hiddenColumns: string[] = [];
</script>

{#if !hiddenColumns.includes('time')}
    <td>{extractTime(trip.datetime)}</td>
{/if}
```

### Step 4.5: Update i18n for Column Visibility
**Files:** `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

```typescript
// Slovak
trips: {
    columnVisibility: {
        title: 'Stĺpce',
        time: 'Čas',
        fuelConsumed: 'Spotrebované (l)',
        fuelRemaining: 'Zostatok (l)',
        otherCosts: 'Iné (€)',
        otherCostsNote: 'Iné poznámka',
    },
}

// English
trips: {
    columnVisibility: {
        title: 'Columns',
        time: 'Time',
        fuelConsumed: 'Fuel consumed (l)',
        fuelRemaining: 'Fuel remaining (l)',
        otherCosts: 'Other (€)',
        otherCostsNote: 'Other note',
    },
}
```

**Verification:** `npm run dev` — visibility toggle works, persists on reload.

## Phase 5: Export

### Step 5.1: Update HTML Export
**Files:** `src-tauri/src/export.rs`

**Add to ExportLabels struct:**
```rust
pub struct ExportLabels {
    // ... existing ...
    pub col_time: String,  // NEW
}
```

**Update label loading from i18n:**
```rust
let labels = ExportLabels {
    // ... existing ...
    col_time: /* load from i18n or hardcode "Čas" */,
};
```

**Add time column header:**
```rust
// In header row generation
<th>{}</th>  // Date
<th>{}</th>  // Time - NEW
```

**Add time cell in row rendering:**
```rust
// Extract time from trip.datetime
let time_str = trip.datetime.format("%H:%M").to_string();

// In row generation
<td>{}</td>  // Date
<td>{}</td>  // Time - NEW
```

**Tests** (`export.rs` tests):
- Test export includes time column
- Test time displays correctly (HH:MM format)
- Test 00:00 time exports as "00:00"

**Verification:** Export to HTML, verify time column appears.

### Step 5.2: Update Export i18n
**Files:** `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

```typescript
export: {
    // ... existing ...
    colTime: 'Čas',  // SK
    // colTime: 'Time',  // EN
}
```

## Phase 6: Testing

### Step 6.1: Backend Unit Tests
Run after each phase: `cd src-tauri && cargo test`

**Key test files:**
- `models.rs` or `models_tests.rs` — datetime parsing
- `db_tests.rs` — CRUD with datetime
- `commands_tests.rs` — API with date+time params
- `settings.rs` — hidden_columns persistence

### Step 6.2: Integration Tests
**Files:** `tests/integration/`

**New test files:**
- `time-column.spec.ts`:
  - Create trip with time → verify displays
  - Edit trip time → verify saves
  - Default time is 00:00

- `column-visibility.spec.ts`:
  - Toggle column off → verify hidden
  - Toggle column on → verify visible
  - Reload app → verify persists

**Verification:** `npm run test:integration`

## Verification Checklist

### After Phase 1 (Backend datetime):
- [ ] `cargo test` passes
- [ ] Migration runs on fresh DB
- [ ] Existing trips have datetime = date + "T00:00:00"

### After Phase 2 (Hidden columns backend):
- [ ] `cargo test` passes
- [ ] get/set hidden_columns commands work

### After Phase 3 (Frontend time):
- [ ] Time column displays in grid
- [ ] Can enter time in new trip
- [ ] Time persists on save

### After Phase 4 (Frontend hideable columns):
- [ ] Eye icon shows in header
- [ ] Dropdown toggles columns
- [ ] Hidden state persists on reload

### After Phase 5 (Export):
- [ ] HTML export includes time column
- [ ] All columns exported (regardless of hidden state)

### Final:
- [ ] All backend tests pass (`npm run test:backend`)
- [ ] Integration tests pass (`npm run test:integration`)

## Files Changed Summary

**Backend (Rust):**
- `migrations/2026-01-27-000000_add_trip_datetime/up.sql` — Add datetime column
- `migrations/2026-01-27-000000_add_trip_datetime/down.sql` — No-op for SQLite
- `schema.rs` — Add datetime to trips table
- `models.rs` — Add datetime to Trip, TripRow, NewTripRow; update From impl
- `db.rs` — Update CRUD for datetime
- `commands.rs` — Update trip commands (date+time params), add hidden columns commands
- `lib.rs` — Register get_hidden_columns, set_hidden_columns
- `settings.rs` — Add hidden_columns field
- `export.rs` — Add col_time to ExportLabels, add time column rendering

**Frontend (Svelte/TS):**
- `types.ts` — Add datetime to Trip, add extractTime helper
- `api.ts` — Update createTrip/updateTrip (add time param), add hidden columns API
- `TripGrid.svelte` — Time column header, hiddenColumns state, ColumnVisibilityDropdown
- `TripRow.svelte` — Time display/input, conditional column rendering
- `ColumnVisibilityDropdown.svelte` — New component
- `i18n/sk/index.ts` — trips.columns.time, trips.columnVisibility.*, export.colTime
- `i18n/en/index.ts` — English translations

**Tests:**
- `models.rs` / `models_tests.rs` — Datetime parsing tests
- `db_tests.rs` — CRUD with datetime
- `commands_tests.rs` — Date+time params, hidden columns commands
- `settings.rs` — Hidden columns serialization
- `export.rs` — Time column in export
- `tests/integration/time-column.spec.ts` — E2E time tests
- `tests/integration/column-visibility.spec.ts` — E2E visibility tests
