# Feature: Trip Entry Defaults (Copy, Time Inference, Date Prefill)

> Every new trip row opens with useful starting values: a date chosen by the prefill
> mode, start/end times inferred from the last drive on the same route (opt-in), and a
> one-click copy that reuses route, distance, purpose and times from an existing row
> without dragging along its fuel, energy or costs.

Three smaller features that together shape what a new trip row contains when it opens:

1. **Copy trip** (the row's copy icon) -- duplicates route, distance, purpose and
   time-of-day from an existing row.
2. **Time inference** (opt-in, Settings -> *Automaticky vyplniť časy podľa poslednej trasy*
   checkbox) -- suggests start/end times from the most recent completed trip on the same
   route, with a jittered delay and duration so rows do not look machine-made.
3. **Date prefill** (SegmentedToggle next to "Nový záznam") -- whether a new row is dated
   with the last trip's date + 1 day or with today.

## User Flow

### Copy a trip

1. Click the **copy icon** on any row. The row's controls are disabled while a copy or a
   new row is already pending, and copying is disabled while any row is open for editing.
2. A new row opens at the top of the grid, in **edit mode**, seeded with the source's
   origin, destination, purpose, distance and time-of-day.
3. **Fuel, energy, other costs, notes and invoice links are not copied** -- a fill-up is a
   one-off event, not a property of a route; copying it would feed a fabricated fill-up
   into the consumption rate and the 20% margin.
4. The **date** is today, clamped into the year being viewed: today for the current year,
   `31 Dec` for a past year, `1 Jan` for a future year -- so the copy stays visible in the
   open book. An overnight trip (22:00 -> 02:00) keeps its day span instead of collapsing
   to a negative duration.
5. The **odometer is not copied as a number** -- the backend derives it from the chain
   (`anchor + km`), the same rule a normal save uses, so the copied row cannot disagree
   with the book. The live preview fills it in once a distance is present.
6. Edit and save as usual; the save runs the normal odometer-cascade check (see
   [trip-odometer-cascade.md](./trip-odometer-cascade.md)).

### Time inference (opt-in)

1. Enable *Automaticky vyplniť časy podľa poslednej trasy* in Settings. Default is OFF.
2. On a **new** row only, once both origin and destination are chosen, the frontend asks
   the backend for the most recent completed trip on that route.
3. If one exists, start/end times are suggested: the historical start plus a random
   -15 to +15 minute offset, and the historical duration scaled by 0.85-1.15, rounded to
   a minute. The suggestion fills the time fields.
4. A 6-second toast with a **Vrátiť (Undo)** button appears; undoing restores the
   previous values and allows inference to re-trigger on the same row.
5. Inference failure is silent (logged, never blocking). A copied row suppresses
   inference for its own route -- the copy already carries the times.

### Date prefill

1. The trip-grid header next to "Nový záznam" has a two-way SegmentedToggle: **+1 deň**
   (last trip date + 1 day, the default) or **Dnes** (today).
2. The setting is global, saved immediately on toggle, and applies to the top new-row's
   date. With no trips in the book both modes fall back to today.

## Technical Implementation

### Copy: the backend decides the defaults

The command `get_copied_trip_defaults` loads the source trip and runs a pure rule,
`compute_copied_trip_defaults` in
[calculations/trip_copy.rs](../../src-tauri/core/src/calculations/trip_copy.rs):

```
target_date = resolve_copy_target_date(viewed_year, today):
    viewed_year == current  -> today
    viewed_year <  current  -> 31 Dec of viewed_year
    viewed_year >  current  ->  1 Jan of viewed_year

start_datetime = target_date + source's start HH:MM:SS
end_datetime   = target_date + source's end time + (source_end.date - source_start.date) days
copied: origin, destination, purpose, distance_km (only when 0 < km <= 9999)
not copied: fuel, energy, costs, notes, invoice links (the struct has no such fields)
```

- The `CopiedTripDefaults` struct *is* the exclusion contract: it has no fields for
  fuel, energy or costs, so the exclusion holds at compile time
  ([BIZ-024](../../DECISIONS.md)).
- An implausible distance (over 9999 km) is not copied -- the field opens blank. This
  mirrors the autofill guard so a delta-accumulation bug cannot spread.
- "Today" is the host's day (`Local::now()`), a known limitation in server mode where the
  container's clock, not the viewer's, decides the date.
- The frontend writes the ODO itself only via the live preview (`odoFollowsKm`); the
  authoritative odometer comes from the cascade command, which deliberately takes no
  odometer argument.

### Time inference: lookup and jitter stay in Rust (ADR-014)

The inference uses **no distance and no speed** -- the base is the most recent completed
trip's start time and duration on the same route:

```
lookup: same vehicle + origin + destination, end_datetime set,
        most recent by start_datetime
base   = (start HH:MM, duration in minutes)
start  = base_start + jitter.minutes()          where minutes() in [-15, 15]
length = round(base_duration_mins x duration_factor())   where factor in [0.85, 1.15]
end    = start + length
```

- The DB lookup is `find_most_recent_trip_times_for_route` in
  [db.rs](../../src-tauri/core/src/db.rs). No match -> `None` (no suggestion).
- Randomness is business logic and stays in Rust behind a `Jitter` trait
  ([ADR-014](../../DECISIONS.md)): production uses a thread RNG, tests inject a stub, so
  the pure `compute_inferred_times` rule is deterministic under test.
- The opt-in gate (`infer_trip_times`) sits at the public command boundary, not inside
  the pure helpers ([BIZ-014](../../DECISIONS.md)). The frontend never computes times; it
  writes the final ISO strings the backend returns.
- A duration jitter can roll `end` past midnight into the next day.

### Frontend wiring

- **TripRow.svelte** owns the copy seed (`copyFrom`), the `odoFollowsKm` preview
  behaviour, the inference trigger (`tryInferTimes`, guarded by an `inferredKey` dedup so
  the same route pair is not re-inferred on every keystroke) and the undo toast.
- **TripGrid.svelte** owns the copy handler (`handleCopy`, latched against an in-flight
  copy), the prefill SegmentedToggle and `defaultNewDate`.
- **Settings** (`settings/+page.svelte`) owns the `infer_trip_times` checkbox.

## Key Files

| File | Purpose |
|------|---------|
| [calculations/trip_copy.rs](../../src-tauri/core/src/calculations/trip_copy.rs) | Pure copy rule: `resolve_copy_target_date`, `compute_copied_trip_defaults`, `MAX_PLAUSIBLE_KM` |
| [calculations/time_inference.rs](../../src-tauri/core/src/calculations/time_inference.rs) | `Jitter` trait, `ThreadRngJitter`, `compute_inferred_times` |
| [commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) | `get_copied_trip_defaults_internal`, `get_inferred_trip_time_for_route_internal`, `inferred_trip_time_for_route` |
| [db.rs](../../src-tauri/core/src/db.rs) | `find_most_recent_trip_times_for_route` |
| [settings.rs](../../src-tauri/core/src/settings.rs) | `DatePrefillMode`, `date_prefill_mode`, `infer_trip_times` |
| [commands_internal/settings_cmd.rs](../../src-tauri/core/src/commands_internal/settings_cmd.rs) | `get/set_date_prefill_mode_internal`, `get/set_infer_trip_times_internal` |
| [models.rs](../../src-tauri/core/src/models.rs) | `InferredTripTime`, `CopiedTripDefaults` |
| [api.ts](../../src/lib/api.ts) | `getCopiedTripDefaults`, `getInferredTripTimeForRoute`, date-prefill and infer-trip-times wrappers |
| [TripRow.svelte](../../src/lib/components/TripRow.svelte) | Copy seed, preview odometer, inference trigger + undo toast, overnight-span preservation |
| [TripGrid.svelte](../../src/lib/components/TripGrid.svelte) | Copy handler, prefill toggle, `defaultNewDate` |
| [settings/+page.svelte](../../src/routes/settings/+page.svelte) | `infer_trip_times` checkbox |
| [tests/integration/specs/tier2/copy-trip.spec.ts](../../tests/integration/specs/tier2/copy-trip.spec.ts) | Copy flow: prefill, empty fuel, ODO recalc, km replacement, overnight span, inference suppression |

## Design Decisions

### Why fuel is not copied (BIZ-024)

A fill-up is a one-off event, not a property of a route. Copying `fuel_liters` would feed
a fabricated fill-up into the consumption rate and the 20% legal margin
([BIZ-003](../../DECISIONS.md)). The copied row is a travel record with an empty fuel
tank; the user fills in actual fuelling.

### Why the copied distance is a default, not a decision

Changing the destination on a copied row replaces the copied distance with the new
route's stored kilometres (the same autofill the grid uses), because the original number
belonged to the original route.

### Why inference is opt-in and reversible (BIZ-014)

Auto-filling times the user did not ask for is surprising. It defaults OFF, every
inference is surfaced with an Undo toast, and the rule lives in Rust so the frontend can
never drift from it.

### Why jitter exists (ADR-014)

Consecutive trips on the same route with identical clock times look fabricated. The
+/-15 min / +/-15% jitter makes the book read like real driving while staying within a
plausible window.

### Why the date rule lives in Rust

The "today / 31 Dec / 1 Jan" clamp is a business rule over the viewed year
([BIZ-024](../../DECISIONS.md), [ADR-008](../../DECISIONS.md)) -- the frontend could get it
wrong per-year, and the backend is the single implementation.

## Related

- [BIZ-024](../../DECISIONS.md): a copied row is dated today, clamped into the year being viewed
- [BIZ-014](../../DECISIONS.md): opt-in auto-fill of trip start/end times
- [ADR-014](../../DECISIONS.md): jitter stays in Rust; testability via the `Jitter` trait
- [trip-odometer-cascade.md](./trip-odometer-cascade.md) -- the save the copied row runs through
- [_tasks/_done/56-smart-trip-defaults/](../../_tasks/_done/56-smart-trip-defaults/) -- time inference origin
- [_tasks/_done/59-time-inference-toggle/](../../_tasks/_done/59-time-inference-toggle/) -- opt-in toggle + undo
- [_tasks/_done/71-copy-trip-row/](../../_tasks/_done/71-copy-trip-row/) -- the copy feature
- [_tasks/_done/37-date-prefill-setting/](../../_tasks/_done/37-date-prefill-setting/) -- the prefill setting