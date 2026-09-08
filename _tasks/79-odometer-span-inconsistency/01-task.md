**Date:** 2026-09-08
**Subject:** Three trips whose odometer span does not match their recorded distance, one of them negative
**Status:** Partly done -- the warnings ship, the data correction is open

## Goal

Correct three rows in the production book whose odometer span contradicts their recorded
distance, and stop the app from accepting that shape silently in future.

## How it was found

While designing [task 78](../78-round-trip-legs-and-distance-writeback/), the odometer
invariant was measured rather than assumed, against a read-only copy of the production
database taken on 2026-09-08. **105 of 108 trips** satisfy
`odometer_end - odometer_start == distance_km` exactly. Three do not.

## The three rows

Vehicle BT014IN. `start` is derived, not stored: it is the previous trip's stored ending
odometer ([helpers.rs:67-90](../../src-tauri/core/src/commands_internal/helpers.rs)).

| Trip | Date | Route | start | end | span | recorded km |
|---|---|---|---|---|---|---|
| `03f46d80` | 2026-08-19 15:00 | Mlynske Nivy 14 -> OMV Strojnicka | 69059 | 69415 | **356** | 4 |
| `a51cb498` | 2026-08-19 15:00 | OMV Strojnicka -> Spisska Nova Ves | 69415 | 69411 | **-4** | 352 |
| `32631e0e` | 2026-08-27 15:00 | Spisska Nova Ves -> Ganovce | 69411 | 69465 | **54** | 50 |

A span of **-4 km** is physically impossible: that trip's ending odometer is lower than
its starting one.

## Root cause

The two 2026-08-19 rows carry the **identical** `start_datetime` of `15:00:00`. The grid
sorts by date, then datetime, then `created_at`
([helpers.rs:74-80](../../src-tauri/core/src/commands_internal/helpers.rs)), so with the
first two keys tied the order falls to `created_at`:

- `03f46d80` (4 km) created 2026-08-23 **12:50:19** -> sorts first
- `a51cb498` (352 km) created 2026-08-23 **12:51:12** -> sorts second

But the stored odometers only reconcile in the **opposite** order (352 km first: 69059 +
352 = 69411; then 4 km: 69411 + 4 = 69415).

The **places** settle which order is real. `03f46d80` ends at OMV Strojnicka and
`a51cb498` begins there, so the 4 km hop to the petrol station must come first. The sort
is therefore right and **the two stored odometers are wrong** -- they were entered as if
the long trip came first.

The third row is collateral: its start is `a51cb498`'s ending odometer, so it inherits
the 4 km error.

## Why it matters

Both 2026-08-19 rows are `fullTank = true`, and `03f46d80` records 45.2 litres. A
full-tank fill-up **closes a consumption period**
([calculations/mod.rs:101-119](../../src-tauri/core/src/calculations/mod.rs)), and a
period's rate is `period_fuel / period_km * 100`. These rows sit exactly on a period
boundary, so the wrong spans have been feeding the consumption figures -- and consumption
is judged against the 20% legal limit
([calculations/mod.rs:46-52](../../src-tauri/core/src/calculations/mod.rs)). This book is
legal evidence.

## Requirements

### Correct the data

- The arithmetic that reconciles all three rows is: `03f46d80` ending odometer
  69415 -> **69063**, and `a51cb498` ending odometer 69411 -> **69415**. The 2026-08-27
  row then spans 69465 - 69415 = 50, matching its recorded distance, with no edit of its
  own.
- **This is the user's book and legal evidence. Do not write to production without their
  explicit confirmation of these exact values.** Present the before and after, including
  what it does to the affected period's consumption rate and margin, and let them decide.
- Recompute and show the affected period's rate and margin before and after, so the
  correction is made with the legal consequence visible rather than discovered later.

### Stop it recurring

- The app should not silently accept a trip whose odometer span contradicts its recorded
  distance. A **negative** span in particular is never valid.
- Decide during design whether that is a hard validation on write, a visible warning on
  the row, or a report the user can run. A hard block risks trapping a user mid-correction
  when a chain is temporarily inconsistent -- which is exactly the state this task has to
  edit through.
- Consider the tie-break: two trips sharing a `start_datetime` to the second order by
  `created_at`, which reflects **data-entry order, not travel order**. That is what let
  the mismatch hide. Whether the fix is a better tie-break, a warning on same-timestamp
  rows, or nothing at all is a design question.

## Technical Notes

- Read the production database **copy only**. Per `CLAUDE.local.md` the live file at
  `root@192.168.0.112:~/kniha-jazd/data/kniha-jazd.db` is **copy-from only**; the
  measurement above used a copy served by a local container.
- `update_trip` already accepts `odometer`, so the correction needs no new command.
- The row editor keeps km and odo in step in both directions
  ([km-odo-bidirectional.spec.ts](../../tests/integration/specs/tier1/km-odo-bidirectional.spec.ts)),
  which is why 105 of 108 rows are clean. The invariant is real and worth defending.
- Scope note: [task 78](../78-round-trip-legs-and-distance-writeback/) will let a routed
  distance be written onto a row. It must preserve this same invariant, so these two
  tasks share a constraint but not a fix.

## Progress, 2026-09-08

### Landed: the warnings

The "stop it recurring" requirement is implemented as **warnings, never a block**
(ADR-042). Two checks, split by what each one is for (ADR-043):

- `calculate_odometer_span_warnings` -- `odometer - odometer_start` against
  `distance_km`, 1 km tolerance. This is the detector. The sign sits on the odo
  cell and its tooltip names both numbers.
- `calculate_duplicate_datetime_warnings` -- trips sharing an exact
  `start_datetime`. This is context, not a detector. The sign sits on the start
  datetime cell.

Both ride along with `get_trip_grid_data`. Backend unit tests own the rules
(12 tests); `tests/integration/specs/tier2/odometer-chain-warnings.spec.ts`
covers the UI flow.

### What the measurement changed

The task above reads the tie as the root cause. Measured against the read-only
snapshot at `_tmp/75-place-cleanup/prod-snapshot.db` (329 trips), it is not the
whole one:

| Measure | Count |
|---|---|
| Rows in a tied-datetime group | 68 of 329 |
| Tied groups | 31 |
| Tied groups at exactly `00:00:00` | **30 of 31** |
| Tied rows that are actually wrong | 2 |
| Rows breaking the span invariant | 5 (3 of them this task's rows) |

`00:00` is the default a new row gets, so a tie is what the app produces
whenever the user does not type a time. A warning on the tie alone would mark
68 rows to find 2, and would still miss `32631e0e`, whose datetime is unique.
The span check is the one that finds all three.

The other two span hits are not this task's rows:

- 2025-01-12, span 88.5 vs 88 km recorded. A half kilometre carried over the
  2024/2025 boundary. The 1 km tolerance keeps it quiet.
- 2023-04-25, span -34910. The book starts at odometer 3147, but the vehicle's
  `initial_odometer` is 38057. Setting `initial_odometer` to 3125 clears it.
  **Not changed here** -- it is production data and needs the user's decision.

### Also confirmed while measuring

No write path checks the odometer at all. `update_trip_internal`
([trips.rs](../../src-tauri/core/src/commands_internal/trips.rs)) validates only
the SoC range. The km/odo guards in
[TripRow.svelte](../../src/lib/components/TripRow.svelte) run in the open row
editor and use the current display neighbour, so they stop applying once the row
closes. That is how `32631e0e` broke without ever being edited.

### Still open

- **The data correction.** The three rows in the production book are unchanged.
  The values in "Correct the data" above still need the user's explicit
  confirmation, with the affected period's rate and margin shown before and
  after.
- **The three rows are independent of [task 80](../80-one-trip-ordering/).** 2026
  has zero ordering disagreements, because those rows carry distinct `created_at`
  values. The data correction can happen before or after that refactor.

### Moved to task 80

The third requirement above, "Consider the tie-break", is answered in
[task 80](../80-one-trip-ordering/) rather than here. The measurement that settles
it: the two candidate tie-breaks agree in 30 of the 31 tied groups, and the one
group where they disagree is the 2026-08-19 pair, where the place chain proves
`created_at` right and the odometer wrong. Task 80 also covers what the same
investigation uncovered: the order is decided in three different places, two of them
in Svelte, so the row editor and the grid can show different numbers for the same
row.

- **The `00:00` default** stays untouched in both tasks. It is what makes the ties
  common, but changing it does not fix an existing book.
