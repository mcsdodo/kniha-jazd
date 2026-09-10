# Feature: Trip Odometer Cascade & Odometer Warnings

> A save never moves a later row's odometer silently: editing a distance, an odometer,
> adding or deleting a trip plans the shift to every later row of the year, shows it in a
> confirmation modal, and only then writes it, in one transaction. Broken chains surface
> as per-row warnings instead of being quietly rewritten.

This is the feature behind the "Posun tachometra" modal, the ODO-column ⚠ on trips whose
odometer does not match their recorded kilometres, the "Km pred" column, and the
RPC-only bulk `recalculate_odometers` command. It supersedes the old behaviour where a
save could rewrite later rows without asking ([ADR-046](../../DECISIONS.md)).

## User Flow

1. **Edit a row** (distance, odometer, or any other field) and save, **add a new row**
   ("Nový záznam" or insert-above), or **delete a row**.
2. The backend plans the change against the stored book -- never against what the screen
   happens to show -- and reports what would move: a summary naming the delta and the
   count of rows affected, plus one table row per affected record (trip number, date,
   route, old odometer, new odometer).
3. **If nothing would move**, the save writes immediately, with no modal. This is the
   common case: adding a trip at the end of the current year, or editing a field that
   does not affect the chain (a purpose typo), never asks.
4. **If rows would move**, the modal "Posun tachometra" appears. It splits the delta into
   what your change caused and what was a pre-existing repair of this trip's own
   odometer, and warns when the boundary to the next year opens up (`nextYearChainBreaks`).
5. **Confirm** -> every listed row is shifted by the same delta in one transaction; the
   edited row closes. **Cancel** -> nothing is written, not even the edited row itself; the
   row stays open on what you typed.
6. **Bulk year recalculate** is a separate, deliberate command (`recalculate_odometers`)
   with no UI button -- see [Why it has no button](#why-recalculate_odometers-has-no-ui-button-adr-045).

**While a row is open for editing**, every control that could move its odometer -- new
record, insert-above, copy, delete, opening a second editor -- is disabled, so an open row
cannot be saved against a number that meanwhile stopped being true. A short arming window
closes the gap between clicking delete and its confirmation dialog.

**Read-only mode**: dry runs still work (they write nothing); only the apply call is
blocked -- see [read-only-mode.md](./read-only-mode.md).

## Technical Implementation

### The one order of the book

Every cascade, every calculation and the on-screen grid sort by the same comparator,
`trip_order` in [helpers.rs](../../src-tauri/core/src/commands_internal/helpers.rs):

```
start_datetime -> created_at -> odometer -> id
```

Each key applies only when the one before carries no information; `id` makes the order
total. The odometer sits **below** `created_at` on purpose: ordering by odometer first
would make the chain agree with itself by construction and silence the span warnings
that exist to test those very odometers (see [ADR-044](../../DECISIONS.md)). The SQL
`get_trips_for_vehicle_in_year` orders by `start_datetime DESC, created_at ASC`; the
canonical comparator is applied in Rust afterwards, which is why the odometer and `id`
tiebreakers do not appear in the query.

### The cascade planners

All three planners are pure functions over the trip list -- they plan, they never write:

| Planner | What triggers it | What it computes |
|---|---|---|
| `plan_odometer_cascade` | Edit of an existing row | New odometer/km pair, delta, per-row changes |
| `plan_insert_cascade` | New row (add or insert-above) | Position by order, odometer = anchor + km, tail shift |
| `plan_delete_cascade` | Row deletion | Tail shift by the removed row's **span**, not its recorded km |

**Anchor rule** ([ADR-046](../../DECISIONS.md)): the anchor is the previous row's stored
odometer, or the year-carryover odometer for the first row of the year. Which number
gives way depends on which one the user changed:

- **Distance edited** -> the odometer follows: `odometer = anchor + km`. Distance is the
  legally weighted number, so it wins.
- **Odometer edited** -> the distance follows: `km = odometer - anchor`.
- **Neither edited** (a purpose typo, say) -> a no-op plan that moves nothing. A
  deliberately broken row stays exactly as it was.

A resulting negative distance is rejected -- it means the odometer went backwards. The
cascade stops at the end of the year; a `year_end_odometer_moved` flag tells the caller
the boundary to the next year has opened, and `mark_next_year_chain_breaks` marks the
first row of that year for a warning line in the modal.

The float tolerance is **0.001 km** (`CASCADE_EPSILON`): two numbers within that are "the
same number". This is the cascade and recalculate equality check, distinct from the
1 km span-warning threshold below.

### The dry-run / apply contract

Every write path runs **dry run first**:

1. Frontend calls `update_trip_cascade` / `create_trip_cascade` / `delete_trip_cascade`
   with `dryRun: true`. The backend plans against the stored book and returns a
   `CascadePlan` with the affected rows.
2. `needsApproval(plan)` is true iff `changes` is non-empty or `nextYearChainBreaks` is
   set. No change -> the write proceeds immediately. A change -> the modal opens.
3. On confirm the frontend calls the same command with `dryRun: false`, and the backend
   **plans again from the stored book** rather than replaying the dry run's numbers -- so
   a concurrent edit cannot make the confirm write a stale plan. Row + shift commit in
   **one transaction** (`update_trip_with_odometer_shift` and friends), so a crash cannot
   leave half the year shifted.

`CascadeResult.trip` is `None` on a dry run; the command's write arm returns the saved
trip. `apply_route_distance` (route-map distance write-back) is a separate command with
the same two-call contract, though it answers with a `DistanceWriteback` rather than a
`CascadeResult` -- see
[route-maps.md](./route-maps.md#the-recorded-distance-is-written-back-only-explicitly).

### Odometer warnings

The grid's ODO column shows a ⚠ when a trip's odometer does not match its recorded
kilometres. The detector, `calculate_odometer_span_warnings` in
[statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs), flags a trip when

```
|odometer - start - distance_km| >= 1.0 km
```

where `start` is the previous row's odometer (year carryover for the first row). The
tolerance is 1 km on purpose: the grid shows whole kilometres and a year carryover can
hold a half kilometre, so 1 km is the first difference that means a real error. A
`spans` map ships the measured span for flagged rows only, so the tooltip ("The odometer
shows X km, the row records Y km") needs no frontend math ([ADR-008](../../DECISIONS.md)).

A second, separate warning marks every member of a group sharing an exact
`start_datetime` (`duplicate_datetime_warnings`) -- it sits on the start-datetime cell and
explains why two rows with the same timestamp ordered one way or the other. The span
check is the detector; the tied-datetime warning is only its explanation
([ADR-043](../../DECISIONS.md)). Both feed the legend counts above the table.

**Warnings never block** ([ADR-042](../../DECISIONS.md)): no write path validates the
odometer. A save always succeeds; the warning is how the error surfaces.

### The "Km pred" column

`odoStart` (Slovak "Km pred") shows each trip's starting odometer -- the previous row's
ending value. It is computed by the same `calculate_odometer_start` helper the preview
command reads, so the editor's suggested odometer and the grid's column cannot drift.

### recalculate_odometers -- the deliberate rebase

`recalculate_odometers_internal` walks the whole year in `trip_order`, keeping a running
total seeded from the year-carryover odometer, and reports (and, on a write, rewrites)
**only** the rows where `|odometer - running| > 0.001`. It returns a list of
`OdometerChange` rows so a correction can be reviewed before it is written.

It is the thing [ADR-045](../../DECISIONS.md) bans from running automatically. A dry run
against the book measured 69 rows in 2023, 0 in 2024, 68 in 2025 and 3 in 2026 -- mostly
rows the user never touched -- and a rebase erases the span warnings by making the chain
agree with itself. So the command is **RPC-only**:

- Its only reachable path is the dispatcher arm; `api.ts` deliberately ships no
  wrapper for it (or for the plain trip CRUD), so nothing in the UI can invoke it by
  accident.
- It takes a mandatory `dryRun` flag. A dry run is allowed in read-only mode; the write
  path is guarded like any other write.
- It is the fallback for a year whose chain has drifted and needs one clean pass.

## Key Files

| File | Purpose |
|------|---------|
| [commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) | The three planners, `update_trip_cascade_internal`, `create_trip_cascade_internal`, `delete_trip_cascade_internal`, `recalculate_odometers_internal`, `apply_route_distance_internal`, `mark_next_year_chain_breaks` |
| [commands_internal/helpers.rs](../../src-tauri/core/src/commands_internal/helpers.rs) | `trip_order` (the one comparator), `calculate_trip_numbers`, `calculate_odometer_start` |
| [commands_internal/statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs) | `calculate_odometer_span_warnings`, `calculate_odometer_spans`, `calculate_duplicate_datetime_warnings`, preview anchor |
| [models.rs](../../src-tauri/core/src/models.rs) | `OdometerChange`, `CascadePlan`, `CascadeResult`, warning sets on `TripGridData` |
| [server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) | RPC arms: `update_trip_cascade`, `create_trip_cascade`, `delete_trip_cascade`, `apply_route_distance`, `recalculate_odometers` |
| [api.ts](../../src/lib/api.ts) | `updateTripCascade`, `createTripCascade`, `deleteTripCascade`, `applyRouteDistance`; the no-wrapper rule for plain CRUD |
| [TripGrid.svelte](../../src/lib/components/TripGrid.svelte) | Dry-run-then-apply handlers, `needsApproval`, pending-cascade state, arming windows |
| [OdometerCascadeModal.svelte](../../src/lib/components/OdometerCascadeModal.svelte) | The confirmation modal |
| [TripRow.svelte](../../src/lib/components/TripRow.svelte) | km/odo edit handlers, warning glyphs, row-action disabling while editing |
| [tests/integration/specs/tier2/odometer-cascade.spec.ts](../../tests/integration/specs/tier2/odometer-cascade.spec.ts) | UI flow: modal, arming window, per-year stop |

## Design Decisions

### Why a save cascades at all (ADR-046)

Before this feature a save rewrote the odometer of later rows silently, or left the chain
broken with no way to reconcile it. The cascade makes the shift explicit, reviewed, and
atomic: the user sees exactly which rows move and by how much, and confirms before any
byte changes. Distance wins over the odometer because distance is the number the
consumption rate and the 20% legal margin ([BIZ-003](../../DECISIONS.md)) are computed
from.

### Why warnings, not blocks (ADR-042)

A block on a broken odometer would make a legal-compliance record un-editable -- you could
never fix the very row that is wrong. Warnings keep the book editable and surface the
error where the user can act on it.

### Why recalculate_odometers has no UI button (ADR-045)

A full-year rebase rewrites rows the user never touched and makes the chain agree with
itself, which would erase the span warnings that are the whole point. It stays a manual,
RPC-only command with a dry-run review, so it can only ever be a deliberate act.

### Why the odometer is the third sort key, not the first (ADR-044)

Ordering by the odometer first would make the chain self-consistent by construction and
silence the span check. Putting it below `created_at` keeps the warnings honest while
still giving deterministic order for a same-timestamp group.

### Why cancel writes nothing at all

The edited row and the cascade are one decision. If the user rejects the plan, saving
just the row anyway would leave the very inconsistency the modal was about to fix.

## Related

- [ADR-042](../../DECISIONS.md): a broken odometer chain warns, it never blocks the save
- [ADR-043](../../DECISIONS.md): the span is the check that finds the error; the tied datetime only explains it
- [ADR-044](../../DECISIONS.md): one comparator decides trip order, and the odometer is its third key
- [ADR-045](../../DECISIONS.md): the odometer rewrite is a command the user runs, never a side effect of a save
- [ADR-046](../../DECISIONS.md): a save cascades the odometer by delta; a rebase never runs on its own
- [route-maps.md](./route-maps.md) -- the distance write-back (`apply_route_distance`) reuses the same planner
- [read-only-mode.md](./read-only-mode.md) -- dry runs stay allowed in read-only mode
- [_tasks/_done/79-odometer-span-inconsistency/](../../_tasks/_done/79-odometer-span-inconsistency/) -- the warnings
- [_tasks/_done/80-one-trip-ordering/](../../_tasks/_done/80-one-trip-ordering/) -- the one comparator
- [_tasks/_done/81-odometer-cascade-on-save/](../../_tasks/_done/81-odometer-cascade-on-save/) -- the cascade spec (R1-R9)