**Date:** 2026-09-08
**Subject:** Three trips whose odometer span does not match their recorded distance, one of them negative
**Status:** Planning

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
