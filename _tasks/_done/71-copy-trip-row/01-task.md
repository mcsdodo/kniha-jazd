**Date:** 2026-09-03
**Subject:** Copy button on trip rows — duplicate a route into a new today-dated row, pre-opened in edit mode
**Status:** Planning

# Task 71: Copy Trip Row

## Goal

Cut repeat data entry for recurring drives. A user who makes the same journey
regularly should be able to click one button on an existing row and get a new,
editable row already carrying that journey's route, distance, purpose and
time-of-day — dated today — instead of retyping it.

## User Story

> As a user, when I drive the same route I already logged, I click the copy
> icon on that row. A new row opens in edit mode at the top of the grid with
> today's date, the same start/end times, the same origin, destination, km and
> purpose. Fuel and cost fields are empty. I adjust whatever differs and save.

## Requirements

### 1. Copied fields

| Copied | Not copied |
|--------|-----------|
| `origin` | `fuel_liters`, `fuel_cost_eur`, `full_tank` |
| `destination` | `energy_kwh`, `energy_cost_eur`, `full_charge`, `soc_override_percent` |
| `distance_km` → ODO recalculated | `other_costs_eur`, `other_costs_note` |
| `purpose` | invoice / receipt links |
| start + end time-of-day | route map |
| date → resolved target date (below) | |

**Rationale for the exclusions:** a fill-up is a one-off event, not a property
of a route. Copying `fuel_liters` into a repeat trip would silently corrupt the
consumption rate and the 20 % margin calculation. Costs and invoice links are
likewise per-event. The copied set is exactly "what makes this journey *this
journey*".

### 2. Target date resolution

The grid has a year picker, so "today" is not always a date the current grid can
show. Resolution rule, given the viewed `year`:

| Condition | Target date |
|-----------|-------------|
| `today.year() == year` | today |
| `today.year() > year` (viewing a past year) | 31 Dec of `year` |
| `today.year() < year` (viewing a future year) | 1 Jan of `year` |

The copied row therefore always lands inside the grid the user is looking at.

### 3. Time-of-day transfer

- `start_datetime` = target date + source's start `HH:MM:SS`.
- `end_datetime` = target date + source's end `HH:MM:SS`, **plus the source's
  day offset** (`source_end.date() − source_start.date()`). Without the offset a
  22:00 → 02:00 overnight trip collapses into a negative duration.
- Source with a null `end_datetime` → copied end stays null.

### 4. Interaction with existing smart defaults ([Task 56](../_done/56-smart-trip-defaults/), [Task 59](../_done/59-time-inference-toggle/))

Copied values are explicit user intent and must win over inference:

- **Time inference** (`tryInferTimes`) applies jitter to start/end once origin
  and destination are both filled. On a copied row it must not fire — the copied
  times would be silently jittered away.
- **Route distance auto-fill** (`tryAutoFillDistance`) only fires when
  `distanceKm === null`, so a copied row suppresses it without extra work.

### 5. UI placement and constraints

- Copy icon in the actions column, between insert-above (`+`) and the map pin.
- The synthetic "Prvý záznam" row renders an empty actions cell, so it is
  excluded with no extra guard.
- `showNewRow` is a single boolean — only one new row exists at a time. The copy
  button is disabled while a new row is open, matching the existing
  `disabled={showNewRow}` on the "Nový záznam" button.
- The copied row opens in the **top** new-row slot (`insertAtTripId = null`),
  not inserted above the source row: its date is today, so it belongs newest-first.

## Approach

**Backend command `get_copied_trip_defaults(trip_id, year)`** returning a
`CopiedTripDefaults` struct. Chosen over pure-frontend prefill because the date
resolution and day-offset rules are genuine business rules with real edge cases,
and ADR-008 keeps those in Rust where they can be unit-tested. This mirrors
`get_inferred_trip_time_for_route` from [Task 56](../_done/56-smart-trip-defaults/), which put the analogous rule in
Rust rather than splitting it across the IPC boundary.

Rejected alternative: a `copy_trip` command that writes the row immediately and
opens it in edit mode. Simpler state, but the row is already persisted, so
"fine-tune before applying" becomes "fine-tune after applying" and Cancel has to
issue a delete. Contradicts the requirement.

### Data contract

```rust
// core/src/models.rs
pub struct CopiedTripDefaults {
    pub start_datetime: String,          // "YYYY-MM-DDTHH:MM:SS"
    pub end_datetime: Option<String>,
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub purpose: String,
}
```

The struct *is* the field-exclusion contract. Fuel, energy, costs and notes are
absent, so "route fields only" holds at compile time rather than depending on a
frontend that remembers to skip them.

### Testability seam

```rust
// core/src/commands_internal/trips.rs

// Thin wrapper: loads the trip, reads the clock.
pub fn get_copied_trip_defaults_internal(
    db: &Database, trip_id: String, year: i32,
) -> Result<CopiedTripDefaults, String>

// Pure: no DB, no clock. Fully unit-testable.
pub fn compute_copied_trip_defaults(
    source: &Trip, year: i32, today: NaiveDate,
) -> CopiedTripDefaults
```

Same split as `compute_inferred_times` — the impure dependency (there, the RNG;
here, the clock) is hoisted into the wrapper so the rule itself is a pure function.

### Registration seams (5)

Mirroring `get_inferred_trip_time_for_route`:

1. [src-tauri/core/src/commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) — internal + pure fn
2. [src-tauri/core/src/commands_internal/mod.rs](../../src-tauri/core/src/commands_internal/mod.rs) — re-export
3. [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) — server-mode dispatch arm
4. [src-tauri/desktop/src/commands/trips.rs](../../src-tauri/desktop/src/commands/trips.rs) + [src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs) — Tauri command + `invoke_handler`
5. [src/lib/api.ts](../../src/lib/api.ts) — `getCopiedTripDefaults(tripId, year)`

### Frontend wiring

**[TripRow.svelte](../../src/lib/components/TripRow.svelte)**
- New props: `onCopy: () => void`, `copyDisabled: boolean`,
  `copyFrom: CopiedTripDefaults | null`.
- `copyFrom` seeds `formData` at component init.
- Suppress inference: pre-set `inferredKey` to the copied origin/destination
  pair (same `␟` separator the existing code uses) so `tryInferTimes`
  short-circuits.
- Leave `manualOdoEdit = false` so `odometer` stays derived as
  `previousOdometer + distanceKm` and keeps recomputing if the user edits km.

**[TripGrid.svelte](../../src/lib/components/TripGrid.svelte)**
- `let copyDefaults: CopiedTripDefaults | null = null;`
- `handleCopy(trip)` awaits the command, then sets
  `insertAtTripId = null; insertDate = null; showNewRow = true;`
- Pass `copyFrom={copyDefaults}` to the top new-row `TripRow`, and
  `defaultDate` from the response rather than `defaultNewDate`.
- Clear `copyDefaults = null` in both `handleSaveNew` and `handleCancelNew`.
- Pass `onCopy={() => handleCopy(trip)}` and `copyDisabled={showNewRow}` to each
  existing trip row.

### ODO correctness

The top new-row slot passes `previousOdometer={lastOdometer}` (the highest ODO),
correct whenever today is the newest date — the common case. If the copied row
lands mid-list chronologically, the existing post-save `recalculateAllOdo()`
corrects the whole chain. [Task 65](../_done/65-datetime-is-order/) made `start_datetime` the sole source of order,
so no insertion-position plumbing is needed.

## Testing

### Backend unit tests — `compute_copied_trip_defaults`

- Viewed year == current year → target date is today
- Viewing a past year → 31 Dec of that year
- Viewing a future year → 1 Jan of that year
- Source `end_datetime` is null → copied end stays null
- Overnight source (22:00 → 02:00 next day) → copied end lands on target + 1 day
- Same-day source → copied end lands on the target date itself

### Integration test — new tier-2 spec `copy-trip.spec.ts`

UI flow only. Does **not** re-test the date rule (backend owns it):

- Click copy on an existing row → a new row appears in edit mode with origin,
  destination, km and purpose prefilled
- Adjust km → save → grid shows the new trip with recalculated ODO
- Copy button is disabled while a new row is open

## Documentation

- i18n key `trips.copyRecord` (sk "Kopírovať záznam" / en "Copy record"), then
  `npm run i18n`
- CHANGELOG `[Unreleased]` entry
- `/decision` BIZ entry for the year-clamping rule — it is a new business rule,
  not an application of an existing one

## Files Touched

| File | Change |
|------|--------|
| [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) | `CopiedTripDefaults` struct |
| [src-tauri/core/src/commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) | wrapper + pure fn |
| [src-tauri/core/src/commands_internal/mod.rs](../../src-tauri/core/src/commands_internal/mod.rs) | re-export |
| [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) | unit tests |
| [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) | dispatch arm |
| [src-tauri/desktop/src/commands/trips.rs](../../src-tauri/desktop/src/commands/trips.rs) | Tauri command |
| [src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs) | `invoke_handler` registration |
| [src/lib/api.ts](../../src/lib/api.ts) | `getCopiedTripDefaults` |
| [src/lib/types.ts](../../src/lib/types.ts) | `CopiedTripDefaults` type |
| [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte) | copy icon, `copyFrom` seeding, inference suppression |
| [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte) | `copyDefaults` state, `handleCopy` |
| [sk/index.ts](../../src/lib/i18n/sk/index.ts) / [en/index.ts](../../src/lib/i18n/en/index.ts) | `trips.copyRecord` |
| [tests/integration/specs/tier2/copy-trip.spec.ts](../../tests/integration/specs/tier2/copy-trip.spec.ts) | new spec |
| [CHANGELOG.md](../../CHANGELOG.md) | Unreleased entry |
| [DECISIONS.md](../../DECISIONS.md) | BIZ entry for year clamping |
