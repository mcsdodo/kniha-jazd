# Implementation Plan: Trip Time Field + Hideable Columns

**Date:** 2026-01-26
**Status:** Planning

## Overview

This plan implements two related features:
1. **Trip Time Field** — Add departure time to trips (datetime migration)
2. **Hideable Columns** — UI pattern to show/hide optional columns

## Phase 1: Database Migration + Backend (datetime)

### Step 1.1: Create Database Migration
**Files:** `src-tauri/migrations/YYYY-MM-DD-HHMMSS_add_trip_datetime/up.sql`

```sql
ALTER TABLE trips ADD COLUMN datetime TEXT NOT NULL DEFAULT '1970-01-01T00:00:00';
UPDATE trips SET datetime = date || 'T00:00:00';
```

**Test:** Migration runs without error, existing data preserved.

### Step 1.2: Update Diesel Schema
**Files:** `src-tauri/src/schema.rs`

Add `datetime` column to trips table schema.

Run `diesel print-schema` to regenerate.

### Step 1.3: Update Models
**Files:** `src-tauri/src/models.rs`

- Change `Trip.date: NaiveDate` → `Trip.datetime: NaiveDateTime`
- Update `TripRow` struct to include `datetime: String`
- Update `NewTripRow` struct
- Update `From<TripRow> for Trip` conversion (parse datetime)
- Keep `date` field in TripRow for now (migration leaves it)

**Tests:** Add test for datetime parsing in models.

### Step 1.4: Update Database CRUD
**Files:** `src-tauri/src/db.rs`

- Update `create_trip` to accept datetime
- Update `update_trip` to accept datetime
- Update trip queries to use datetime column
- Format datetime as ISO 8601 string for storage

**Tests:** Update existing db_tests.rs for datetime field.

### Step 1.5: Update Commands
**Files:** `src-tauri/src/commands.rs`

- Update `create_trip` command signature (date + time params or datetime param)
- Update `update_trip` command signature
- Parse frontend date+time into NaiveDateTime

**Tests:** Update commands_tests.rs for datetime.

## Phase 2: Backend (Hidden Columns Settings)

### Step 2.1: Extend LocalSettings
**Files:** `src-tauri/src/settings.rs`

```rust
pub struct LocalSettings {
    // ... existing ...
    pub hidden_columns: Option<Vec<String>>,
}
```

**Tests:** Add test for hidden_columns serialization/deserialization.

### Step 2.2: Add Hidden Columns Commands
**Files:** `src-tauri/src/commands.rs`

```rust
#[tauri::command]
pub fn get_hidden_columns(app_handle: AppHandle) -> Result<Vec<String>, String>

#[tauri::command]
pub fn set_hidden_columns(app_handle: AppHandle, columns: Vec<String>) -> Result<(), String>
```

Register in `lib.rs` invoke_handler.

**Tests:** Add command tests for get/set hidden columns.

## Phase 3: Frontend (Time Field)

### Step 3.1: Update TypeScript Types
**Files:** `src/lib/types.ts`

- Change `Trip.date: string` to `Trip.datetime: string`
- Or keep `date` and add `time` as separate display fields (parse from datetime)

### Step 3.2: Update API Functions
**Files:** `src/lib/api.ts`

- Update `createTrip` to accept time parameter
- Update `updateTrip` to accept time parameter
- Format date + time into datetime string for backend

### Step 3.3: Update TripRow Component
**Files:** `src/lib/components/TripRow.svelte`

**Display mode:**
- Extract and display time from datetime
- New column after date: `{time}` (HH:MM format)

**Edit mode:**
- Add `<input type="time">` field
- Bind to formData.time
- Combine date + time when saving

### Step 3.4: Update TripGrid Component
**Files:** `src/lib/components/TripGrid.svelte`

- Add "Čas" column header after "Dátum"
- Update column width calculations
- Pass time to TripRow

### Step 3.5: Update i18n
**Files:** `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

```typescript
trips: {
    columns: {
        time: 'Čas',  // SK
        // time: 'Time',  // EN
    }
}
```

## Phase 4: Frontend (Hideable Columns)

### Step 4.1: Add API Functions for Hidden Columns
**Files:** `src/lib/api.ts`

```typescript
export async function getHiddenColumns(): Promise<string[]>
export async function setHiddenColumns(columns: string[]): Promise<void>
```

### Step 4.2: Create ColumnVisibilityDropdown Component
**Files:** `src/lib/components/ColumnVisibilityDropdown.svelte` (new)

- Eye icon button (lucide `eye` / `eye-off`)
- Badge showing hidden count
- Dropdown with checkboxes for each hideable column
- Click outside to close
- Immediate save on toggle

**Hideable columns:**
- `time` — Čas
- `fuelConsumed` — Spotrebované (l)
- `fuelRemaining` — Zostatok (l)
- `otherCosts` — Iné (€)
- `otherCostsNote` — Iné poznámka

### Step 4.3: Integrate into TripGrid Header
**Files:** `src/lib/components/TripGrid.svelte`

- Import ColumnVisibilityDropdown
- Add to header-actions div (after date prefill toggle)
- Load hidden columns on mount
- Pass to column rendering logic

### Step 4.4: Conditionally Render Columns
**Files:** `src/lib/components/TripGrid.svelte`, `src/lib/components/TripRow.svelte`

- Check `hiddenColumns.includes('columnId')` before rendering
- Apply to both header (`<th>`) and cells (`<td>`)
- Pass hiddenColumns set to TripRow as prop

### Step 4.5: Update i18n for Column Visibility
**Files:** `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

```typescript
trips: {
    columnVisibility: {
        title: 'Stĺpce',
        time: 'Čas',
        fuelConsumed: 'Spotrebované (l)',
        fuelRemaining: 'Zostatok (l)',
        otherCosts: 'Iné (€)',
        otherCostsNote: 'Iné poznámka',
    }
}
```

## Phase 5: Export

### Step 5.1: Update HTML Export
**Files:** `src-tauri/src/export.rs`

- Add "Čas" column after "Dátum"
- Extract time from datetime for display
- Ensure all columns exported (ignore hidden_columns setting)

**Tests:** Update export tests for new column.

## Phase 6: Integration Testing

### Step 6.1: Backend Integration Tests
- Test datetime round-trip (create → read → update)
- Test hidden columns persistence

### Step 6.2: Frontend Integration Tests
**Files:** `tests/integration/`

- Test time input in new trip
- Test time display in trip row
- Test column visibility toggle
- Test hidden columns persist after reload

## Verification Checklist

- [ ] Migration runs cleanly on existing database
- [ ] Existing trips show 00:00 as time
- [ ] New trips can have time entered
- [ ] Time displays correctly in grid
- [ ] Column visibility dropdown works
- [ ] Hidden columns persist across app restart
- [ ] Export includes all columns (including hidden)
- [ ] All backend tests pass
- [ ] Integration tests pass

## Files Changed Summary

**Backend (Rust):**
- `migrations/*/up.sql` — New migration
- `schema.rs` — Add datetime column
- `models.rs` — Update Trip, TripRow, NewTripRow
- `db.rs` — Update CRUD for datetime
- `commands.rs` — Update trip commands, add hidden columns commands
- `lib.rs` — Register new commands
- `settings.rs` — Add hidden_columns
- `export.rs` — Add time column

**Frontend (Svelte/TS):**
- `types.ts` — Update Trip type
- `api.ts` — Update trip API, add hidden columns API
- `TripGrid.svelte` — Time column, visibility toggle
- `TripRow.svelte` — Time input/display, conditional columns
- `ColumnVisibilityDropdown.svelte` — New component
- `i18n/sk/index.ts` — Slovak translations
- `i18n/en/index.ts` — English translations

**Tests:**
- `commands_tests.rs` — Datetime tests
- `db_tests.rs` — Datetime tests
- `settings.rs` — Hidden columns tests
- `export.rs` — Export with time tests
- `tests/integration/` — E2E tests
