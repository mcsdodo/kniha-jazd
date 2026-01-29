**Date:** 2026-01-29
**Subject:** Legal Requirements Updates for Slovak Logbook (from 1.1.2026)
**Status:** Planning

## Overview

Slovak law introduces new requirements for vehicle logbooks effective January 1, 2026. This design covers the changes needed to comply with the new regulations.

## Requirements Summary

| # | Legal Requirement | Implementation |
|---|------------------|----------------|
| 4a | Sequential trip number (poradové číslo) | Calculated field, per-year, based on chronological order |
| 4b | Driver name per trip | Use vehicle-level `driver_name`, shown in every row |
| 4c | Start AND end time for each trip | Add `end_time` field, user enters manually |
| 4f | Odometer before AND after each trip | Derive `odometer_start` from previous trip |
| — | Month-end odometer state | Highlight trips on month-end, generate synthetic rows for gaps |

## Design Decisions

### Driver Name (4b)
- **Decision:** Use vehicle-level `driver_name` for all trips
- **Rationale:** Simpler data entry, covers typical single-driver scenarios
- **Display:** Show in every trip row (including export) for legal compliance

### Trip Times (4c)
- **Decision:** Add explicit `end_time` field
- **Rationale:** Full compliance, user has control over exact times
- **Data entry:** User manually enters both start and end times

### Odometer Before/After (4f)
- **Decision:** Derive `odometer_start` from previous trip's odometer
- **Rationale:** Mathematically correct for consecutive trips, no extra data entry
- **First trip:** Uses `vehicle.initial_odometer`

### Trip Numbering (4a)
- **Decision:** Per-year calculated field, not stored in DB
- **Numbering:** Based on chronological order (date, datetime, odometer)
- **Behavior:** Numbers adjust automatically if trip order changes

### Month-End Rows
- **Decision:** Auto-generated synthetic rows + highlight existing month-end trips
- **Logic:**
  - If trip exists on last day of month → highlight that trip row
  - If no trip on last day → generate synthetic row with carried-over values
  - Generate for ALL 12 months, even those with no trips

---

## Data Model Changes

### Database Migration

```sql
-- New column for trip end time
ALTER TABLE trips ADD COLUMN end_time TEXT DEFAULT '';
```

### Models (models.rs)

**Trip struct additions:**
```rust
pub end_time: Option<NaiveTime>,  // Trip end time (None = not entered)
```

**TripRow/NewTripRow additions:**
```rust
pub end_time: String,  // Stored as "HH:MM" or ""
```

### TripGridData Extensions

```rust
pub struct TripGridData {
    // ... existing fields ...

    // NEW: Trip numbering (1-based, per year, by chronological order)
    pub trip_numbers: HashMap<String, i32>,

    // NEW: Odometer at trip start (derived from previous trip)
    pub odometer_start: HashMap<String, f64>,

    // NEW: Trip IDs that fall on last day of month (for highlighting)
    pub month_end_trips: HashSet<String>,

    // NEW: Synthetic rows for months without trip on last day
    pub month_end_rows: Vec<MonthEndRow>,
}

/// Synthetic row for month-end display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthEndRow {
    pub date: NaiveDate,        // Last day of month
    pub odometer: f64,          // Carried from last trip before this date
    pub fuel_remaining: f64,    // Carried fuel state
    pub month: u32,             // 1-12 for identification
}
```

---

## UI Changes

### New Grid Columns

| Column Key | Header SK | Header EN | Source | Hideable |
|------------|-----------|-----------|--------|----------|
| `tripNumber` | Č. | # | `trip_numbers[id]` | Yes |
| `startTime` | Čas od | Start | `datetime` | Yes |
| `endTime` | Čas do | End | `end_time` | Yes |
| `driver` | Vodič | Driver | `vehicle.driver_name` | Yes |
| `odoStart` | Km pred | Odo Start | `odometer_start[id]` | Yes |
| `odoEnd` | Km po | Odo End | `odometer` (renamed) | No (core) |

### Column Order (suggested)

```
Č. | Dátum | Čas od | Čas do | Vodič | Odkiaľ | Kam | Účel | Km | Km pred | Km po | Palivo...
```

### Month-End Styling

**Real trip on month-end:**
- Light blue background (`#e8f4fc`)
- Subtle bottom border

**Synthetic month-end row:**
- Gray background (`#f0f0f0`)
- Italic text
- Shows: date, odometer (same for start/end), driver
- Trip# shows "—" or empty

### Trip Form Changes

- Add "Čas ukončenia" (End time) input field
- Time picker or HH:MM text input
- Optional field (can be left empty for historical data)

---

## Export Changes

### ExportLabels Additions

```rust
pub col_trip_number: String,    // "Č." / "#"
pub col_start_time: String,     // "Čas od" / "Start"
pub col_end_time: String,       // "Čas do" / "End"
pub col_driver: String,         // "Vodič" / "Driver"
pub col_odo_start: String,      // "Km pred" / "Odo Start"
pub col_odo_end: String,        // "Km po" / "Odo End"
```

### Export Styling

```css
/* Synthetic month-end rows */
tr.month-end-synthetic {
  background: #f0f0f0;
  font-style: italic;
}

/* Real trips on month-end */
tr.month-end-trip {
  background: #e8f4fc;
  border-bottom: 2px solid #4a90d9;
}
```

---

## Implementation Plan

### Phase 1: Backend Data Model
1. Add database migration for `end_time`
2. Update `Trip`, `TripRow`, `NewTripRow` in `models.rs`
3. Update `schema.rs` (Diesel schema)
4. Update `db.rs` CRUD operations

### Phase 2: Backend Calculations
1. Add new fields to `TripGridData`
2. Implement `trip_numbers` calculation (sort + enumerate)
3. Implement `odometer_start` derivation
4. Implement `month_end_trips` detection
5. Implement `month_end_rows` generation
6. Write unit tests for all new calculations

### Phase 3: Backend Export
1. Add new labels to `ExportLabels`
2. Update `generate_html()` with new columns
3. Add month-end row styling
4. Update hidden_columns handling for new columns

### Phase 4: Frontend Trip Form
1. Add end time input to trip editor component
2. Wire up to save/update commands
3. Handle empty/optional end time

### Phase 5: Frontend Grid
1. Add new columns to trip grid
2. Add column visibility toggles for new columns
3. Implement month-end row styling
4. Handle synthetic row display (non-editable)

### Phase 6: Frontend i18n
1. Add Slovak translations for new labels
2. Add English translations

### Phase 7: Testing
1. Backend unit tests (calculations, edge cases)
2. Integration tests (grid display, export)
3. Manual testing of month-end scenarios

---

## Test Scenarios

### Trip Numbering
- [ ] Trips numbered 1, 2, 3... in chronological order
- [ ] Reordering trips updates numbers
- [ ] Numbers reset at year boundary

### Odometer Derivation
- [ ] First trip uses `initial_odometer` as start
- [ ] Subsequent trips use previous trip's ending odo
- [ ] Works correctly across year boundary

### Month-End Rows
- [ ] Trip on Jan 31 → highlighted, no synthetic row
- [ ] Last trip on Jan 28 → synthetic row for Jan 31
- [ ] No trips in February → synthetic row for Feb 28/29
- [ ] Leap year February handled correctly
- [ ] All 12 months covered even with sparse data

### Export
- [ ] All new columns appear in export
- [ ] Hidden columns respected
- [ ] Month-end styling renders in HTML/PDF
- [ ] Driver name shows in every row

---

## Backward Compatibility

- `end_time` column has `DEFAULT ''` — older app versions can read DB
- New columns added at end of table (Diesel requirement)
- Existing data continues to work (end_time empty for historical trips)
