# Design: Trip Time Field + Hideable Columns

## Data Model

### Trip DateTime Field

**Change:** Migrate from `date: NaiveDate` to `datetime: NaiveDateTime`

```rust
pub struct Trip {
    // Before: pub date: NaiveDate,
    pub datetime: NaiveDateTime,  // Date + time combined
    // ...
}
```

**Rationale:**
- Single field instead of separate date + time
- Time defaults to 00:00 when not specified
- No special "no time" sentinel — 00:00 is valid and always displayed

### Database Migration

Two-step approach (Option A from brainstorming):

**Migration 1:** Add column and populate
```sql
ALTER TABLE trips ADD COLUMN datetime TEXT NOT NULL DEFAULT '1970-01-01T00:00:00';
UPDATE trips SET datetime = date || 'T00:00:00';
```

**Migration 2 (future, optional):** Drop old `date` column via table rebuild

**Why two-step:**
- Simpler, safer migration
- No table rebuild needed initially
- Unused `date` column costs nothing
- Can clean up later or never

### LocalSettings Extension

```rust
pub struct LocalSettings {
    // ... existing fields ...
    pub hidden_columns: Option<Vec<String>>,  // e.g., ["time", "fuelConsumed"]
}
```

**Column identifiers:**
- `time` — Čas column
- `fuelConsumed` — Spotrebované (l)
- `fuelRemaining` — Zostatok (l)
- `otherCosts` — Iné (€)
- `otherCostsNote` — Iné poznámka

## UI Design

### Column Visibility Toggle

**Location:** Header bar, after existing controls

```
┌─────────────────────────────────────────────────────────────────────┐
│ Záznamy (42)            [Nový záznam]  [+1 deň ○ Dnes]  [👁 / 👁̸(2)] │
└─────────────────────────────────────────────────────────────────────┘
```

**Icon states:**
- Open eye (`eye` from Lucide) — all columns visible, no badge
- Crossed eye (`eye-off` from Lucide) + badge "(N)" — N columns hidden

**Dropdown content:**
```
┌──────────────────────────┐
│ Stĺpce                   │
│ ☑ Čas                    │
│ ☑ Spotrebované (l)       │
│ ☑ Zostatok (l)           │
│ ☑ Iné (€)                │
│ ☑ Iné poznámka           │
└──────────────────────────┘
```

**Behavior:**
- Checkboxes toggle immediately (no save button)
- Changes persist to `local.settings.json`
- Dropdown closes on click outside

**Default state:** All columns visible (empty `hidden_columns` array)

### Time Column

**Placement:** After Date column

```
│ Dátum      │ Čas   │ Odkiaľ     │ Kam        │ ...
│ 15.05.2024 │ 08:30 │ Bratislava │ Košice     │ ...
│ 16.05.2024 │ 00:00 │ Košice     │ Bratislava │ ...
```

**Display mode:**
- Always show HH:MM format
- No special handling for 00:00 (displayed as-is)

**Edit mode:**
- HTML5 `<input type="time" />`
- Native browser time picker
- Empty input saves as 00:00

**New row:**
- Default empty (saves as 00:00)
- No prefill logic

### Columns Always Visible (Not Hideable)

- Dátum (Date)
- Odkiaľ (Origin)
- Kam (Destination)
- km (Distance)
- ODO (Odometer)
- Účel (Purpose)
- PHM (l) (Fuel liters)
- Cena € (Fuel cost)
- l/100km (Consumption rate)
- Akcie (Actions)

## Export Behavior

**Rule:** All columns always exported, regardless of UI visibility settings

**Export columns:**
```
Dátum | Čas | Odkiaľ | Kam | km | ODO | Účel | PHM (l) | Cena € | Spotrebované | l/100km | Zostatok | Iné € | Iné pozn.
```

Time is a new column in export, always included.

## Component Changes

### TripGrid.svelte
- Add `hiddenColumns` state (loaded from settings)
- Add column visibility toggle button + dropdown
- Pass visibility to TripRow
- Conditionally render columns based on visibility

### TripRow.svelte
- Accept `hiddenColumns` prop
- Add time input field (edit mode)
- Add time display (view mode)
- Conditionally render hideable columns

### settings.rs
- Add `hidden_columns: Option<Vec<String>>` to LocalSettings

### commands.rs
- Add `get_hidden_columns` command
- Add `set_hidden_columns` command

### models.rs
- Change Trip.date to Trip.datetime (NaiveDateTime)
- Update TripRow struct for datetime column

### db.rs
- Update trip CRUD for datetime field
- Parse/format datetime instead of date

### export.rs
- Add Čas column to HTML export
- Always include all columns

## i18n Keys

```typescript
trips: {
    columns: {
        time: 'Čas',
        // existing...
    },
    columnVisibility: {
        title: 'Stĺpce',
        showAll: 'Zobraziť všetky',
    },
}
```
