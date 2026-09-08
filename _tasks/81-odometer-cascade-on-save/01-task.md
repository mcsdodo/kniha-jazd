**Date:** 2026-09-08
**Subject:** Editing a trip's distance must move the odometer of every later row of that year
**Status:** Planning

## Goal

Make the odometer chain follow a distance edit again. When the user changes the km of
row N, the backend moves row N's odometer and shifts every later row of the same year by
the same amount. The user approves the shift in a modal before it is written. The grid
then shows the new numbers without a full reload, so the scroll position survives.

## The user's words

> I want to be able to edit KM in any trip and the trips ABOVE MUST update their ODO
> (obviously)

"Above" is the display direction. The grid sorts newest first by default
([TripGrid.svelte:30](../../src/lib/components/TripGrid.svelte)), so a row later in time
is drawn above. The chain itself only ever runs forward in time.

## Background

### Why one edit breaks the next row

Only the **end** odometer is stored on a row. The start is derived:
`start[K] = odometer[K-1]` in `trip_order`
([helpers.rs:91](../../src-tauri/core/src/commands_internal/helpers.rs)). One stored
number is therefore the end of row K and the start of row K+1.

The span check is `|odometer[K] - start[K] - distance_km[K]| >= 1 km`
([statistics.rs:1263](../../src-tauri/core/src/commands_internal/statistics.rs)). Move
`odometer[N]` by `d` and, in the same write, row N's span changes by `+d` and row N+1's
span changes by `-d`. The error is conserved. It moves one link forward and stops
nowhere until a row absorbs it.

Rows **before** N cannot be affected. Row K's span reads `odometer[K]` and
`odometer[K-1]` only, and an edit to row N touches neither for any `K < N`. This is why
the cascade must never look backwards, and why a full-year rebase is the wrong tool.

### What was removed, and when

`recalculateAllOdo` lived in `TripGrid.svelte` from **2025-12-23** (`651feb6`) to
**2026-09-08 09:56** (`4f451ee`). It ran after every create and every update. It is
still in the released build: production runs **0.44.0**, released 2026-09-04, and its
`get_trip_grid_data` response carries no `odometerSpanWarnings` key at all (measured
against `https://kniha-jazd.lacny.me` on 2026-09-08).

The user's report that led to its removal was a **sync** complaint, not a request to drop
the feature:

> I change the values, something updates, yet when full refresh the DATA IS DIFFERENT

The cause was that the browser did the walk itself, over the stale `trips` prop, in DB
order rather than in `trip_order`. The correct fix under
[ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication) is to move
the walk into Rust **and keep calling it**. Task 80 moved it and dropped the call. This
task restores the call.

### Why ADR-045 is superseded, not overturned

[ADR-045](../../DECISIONS.md#adr-045-the-odometer-rewrite-is-a-command-the-user-runs-never-a-side-effect-of-a-save)
says no save may cascade. Its evidence measures one specific operation: the **full-year
rebase** from `initial_odometer` over `distance_km`, which is what the removed code did
and what `recalculate_odometers` still does. That rebase would rewrite 69 rows in 2023
and 68 in 2025 on the first save, none of them touched by the user, and it would erase
the span warnings by making the chain agree with itself.

Those figures condemn the rebase. They say nothing about a **delta shift**, which:

- touches only rows after the edit, so it never reaches row 1 of 2023;
- moves both ends of every later span by the same amount, so no downstream warning is
  created, cleared or changed;
- leaves the 2025 half kilometre where it is instead of chasing it.

ADR-045 stated its ban more broadly than its measurement supports. This task keeps the
ban on the automatic rebase and permits the delta shift.

## Requirements

### R1: Three new commands, one per write that moves the chain

`update_trip` stays as it is: one row, no cascade. The task 79 correction procedure
depends on that ([79/02-correction.md](../_done/79-odometer-span-inconsistency/02-correction.md),
Part 5). `recalculate_odometers` also stays unchanged and UI-less. It is the deliberate
full-year rebase.

Three new commands, one per write that moves the chain. Each takes the arguments of the
command it wraps plus `dryRun: bool`, and each returns the same `CascadePlan`:

| Command | Wraps | What it does |
|---|---|---|
| `update_trip_cascade` | `update_trip` | Saves row N and shifts every later row. |
| `create_trip_cascade` | `create_trip` | Inserts a row and shifts every later row. |
| `delete_trip_cascade` | `delete_trip` | Removes a row and shifts every later row. |

`update_trip`, `create_trip` and `delete_trip` all stay as they are.

### R2: The delta rule

`anchor` is row N's start odometer: the previous row's stored odometer in `trip_order`,
or `yearStartOdometer` when N is the first row of the year.

| Submitted values | Result |
|---|---|
| `distanceKm` differs from stored | `odometer := anchor + distanceKm`. The km wins. |
| km same, `odometer` differs from stored | `odometer := submitted`, `distanceKm := odometer - anchor`. The ODO wins. |
| neither differs | Nothing moves. `d = 0`. No cascade and no modal. |

Then `d = new_odometer[N] - stored_odometer[N]`, and every row after N **in the same
year** gets `odometer += d`.

The frontend supplies no odometer arithmetic. The backend reads the stored row and
decides. This is what removes the guessing from `TripRow.svelte`.

**A re-dated row does not cascade.** If the submitted `startDatetime` differs from the
stored one, the row can move to another position in `trip_order`, and then it leaves one
place in the chain and arrives at another. Two positions shift, not one. That is not
modelled here. In that case the command writes the row and cascades nothing, `d` is
reported as 0, and the span warnings show the result. No modal opens, because no other
row moves. This is a known limit, listed under "Out of scope, and open".

### R3: The rule repairs row N's own span, and the modal must say so

`odometer := anchor + km` makes row N's span equal its recorded distance. If row N was
already broken, `d` carries that repair on top of the user's edit.

This matters most at a year boundary, where `anchor` comes from the previous year. Row 1
of 2025 is the live case. Measured on the local copy of the book:

| Item | Value |
|---|---|
| `anchor` (2024-12-31 ending odometer) | 38056.5 |
| Row 1 of 2025, stored odometer | 38145 |
| Row 1 of 2025, recorded km | 88 |
| Span today | 88.5 |

Change that km from 88 to 90 and the rule gives `odometer := 38056.5 + 90 = 38146.5`, so
`d = +1.5`, not `+2`. Sixty-seven later rows move by 1.5 km. The user typed a 2 km change.

**Decision: keep the repair, and make the modal decompose `d`.** A special case that
skips the repair for the first row of a year would make the rule inconsistent and would
leave a row that can never be corrected by editing its km. Instead the modal must show
both parts:

```
37 rows move by +1.5 km
   +2.0 km   your change to the distance (88 -> 90)
   -0.5 km   correcting this row's odometer, which sits 0.5 km
             above its start (carried over from 2024)
```

The second line appears only when row N's stored span already differed from its recorded
distance. When it names an anchor from another year, it says so.

### R4: The cascade stops at the year end

The shift never crosses into the next year. This matches `recalculateAllOdo`, which
walked only the loaded year from `gridData.yearStartOdometer`
([TripGrid.svelte:42,448](../../src/lib/components/TripGrid.svelte) at `4f451ee^`).

The consequence must be stated, not hidden. If the last row of the year moves, the
boundary to the next year opens by `d`, and the first row of the next year raises a span
warning. Under the old code that break was invisible. Now it is signposted.

The plan reports two facts, not one:

- `yearEndOdometerMoved` -- the year's last stored odometer changed.
- `nextYearChainBreaks` -- that, **and** a later year actually has trips.

Only the second is worth telling the user about. Adding a trip to the newest year moves
its year end every single time and breaks nothing, because there is no next year. A
warning on that would be noise on the most common action in the app.

### R5: The modal is a gate, not a notice

- Nothing else is affected: apply at once, no modal. This is the common case -- an edit
  to the purpose, the times or the litres, and every trip appended to the end of the
  newest year.
- One or more other rows move, **or** `nextYearChainBreaks` is true: show the modal
  first. It lists trip number, date, route, old odometer and new odometer for every row,
  in a scrollable table, under the decomposed summary of R3.
- For a delete, the modal replaces the existing delete confirmation
  ([TripRow.svelte:557](../../src/lib/components/TripRow.svelte)) rather than following
  it. Two dialogs for one action is worse than one dialog that says more.
- **OK** applies. **Cancel** writes nothing at all, including row N, and leaves the row
  open in edit mode. The row editor therefore cannot close itself on Save: `onSave` must
  report whether the write happened, and the editor stays open when it did not.

The apply call recomputes from the stored book. It does not replay the numbers the dry
run returned. Same discipline as the task 79 procedure: if the book moved in between, the
apply must act on the book as it is.

Every write of one cascade runs in one transaction. All rows, or none.

### R6: The grid updates in place

No full reload, and no loss of scroll position. The existing path already satisfies this
and must not regress:

- `handleTripsChanged` calls `loadTrips(false)`, which does not set `initialLoading`, so
  `TripGrid` never unmounts
  ([+page.svelte:86,105](../../src/routes/+page.svelte));
- the rows are a keyed each block on `row.data.id`
  ([TripGrid.svelte:693](../../src/lib/components/TripGrid.svelte)), so Svelte patches
  the changed rows in place;
- display mode reads `trip.odometer` straight off the prop
  ([TripRow.svelte:835](../../src/lib/components/TripRow.svelte)).

One real gap must be closed. `TripRow.formData` is seeded once at construction and is
never re-seeded after a save; the component says so at
[TripRow.svelte:155-160](../../src/lib/components/TripRow.svelte). Today that leaves
stale form state on one row. After a cascade it leaves stale form state on every shifted
row, and opening one for edit would show and then write back the pre-cascade odometer.
`formData` must re-seed from the `trip` prop whenever the row is not in edit mode.

### R7: The frontend keeps no odometer arithmetic

`handleOdoBlur`, the `handleSave` clamp, `odoFollowsKm` and `manualOdoEdit` all exist
because the browser had to guess the anchor
([TripRow.svelte:398-430, 483-520](../../src/lib/components/TripRow.svelte)). The backend
has the anchor. They go.

`tests/integration/specs/tier1/km-odo-bidirectional.spec.ts` pins the behaviour of those
guards. It was extended three times in the last day (`d101c08`, `9d67262`, `c818fe7`). It
must be rewritten to state the new expected behaviour for each case it currently covers,
not patched until it passes.

### R8: Creating a trip shifts the rows after it

A new row lands at a position in `trip_order`. Its own odometer is `anchor + km`, where
`anchor` is the stored odometer of the row before that position. The next row's start then
moves by the new row's distance, so:

`d = distanceKm of the new row`, and every row after the insert position shifts by `+d`.

A new row cannot carry a pre-existing span error, so `deltaFromRepair` is always 0 and
`deltaFromDistance` is always `d`.

The position is decided by `start_datetime`, then `created_at`. A new row's `created_at`
is the moment it is written, so it sorts last inside any group it ties with. The odometer
key of `trip_order` never decides an insert, which is what keeps the position and the
odometer from depending on each other.

The common case -- appending to the end of the newest year -- moves no other row and has
no next year, so it stays a single silent write. That must not regress.

### R9: Deleting a trip shifts the rows after it

Removing row X hands its start to the row that followed it, so that row's span shrinks by
X's **span**, not by its recorded distance:

`d = -(odometer[X] - anchor[X])`, and every row after X shifts by `d`.

The span is the right number because the span is what the chain actually loses. When X was
consistent the two are equal. When X was broken, the span keeps the chain continuous and
the distance would not. The decomposition still holds: `deltaFromDistance` is
`-distanceKm[X]`, and `deltaFromRepair` is the rest.

The old `recalculateAllOdo` never cascaded on delete, so today every delete leaves a hole
and the next row warns. This closes that.

## Out of scope, and open

- **Re-dating a trip.** See R2. The row moves in `trip_order`, so two positions shift, not
  one. The command writes the row and cascades nothing, and the span warnings show the
  result.

Also out of scope, deliberately:

- The `00:00` default start time (ADR-043).
- The 2023 anchor question, closed in [task 79](../_done/79-odometer-span-inconsistency/03-closed.md).
- The 2025 half kilometre. The delta shift preserves it. Only a rebase would move it,
  and nothing in this task runs one.

## Documentation this task must change

- **[DECISIONS.md](../../DECISIONS.md), ADR-045.** Its sentence "A save now writes exactly
  the row the user edited" becomes false. Supersede the never-automatic claim. Keep the
  rebase ban, which the 69 and 68 row measurements still support.
- **A new ADR** for the delta rule, the repair, the per-year stop and the modal gate.
- **[CHANGELOG.md](../../CHANGELOG.md)**, user-visible behaviour.

## Technical notes

- The whole book must not be read to find row N's neighbours. `get_trips_for_vehicle_in_year`
  plus `trip_order` is what every other command already does.
- `db.rs` has the transaction pattern to copy: `conn.transaction::<_, diesel::result::Error, _>`
  at [db.rs:981,1154,1262](../../src-tauri/core/src/db.rs).
- `OdometerChange` ([models.rs:905](../../src-tauri/core/src/models.rs)) is already the
  right shape for the modal rows. It carries `tripId`, `tripNumber`, `oldOdometer` and
  `newOdometer`. Date and route must be added, or the modal must read them from the
  `trips` it already holds.
- `preview_trip_calculation` computes the same odometer for the open row. It and
  `update_trip_cascade` must agree for row N. Pin that with a test.
- Do not write to the production database. Test against a copy under `_tmp/`.
