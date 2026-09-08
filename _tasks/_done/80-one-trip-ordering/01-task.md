**Date:** 2026-09-08
**Subject:** One trip ordering, computed once in Rust
**Status:** Closed -- see [04-closed.md](04-closed.md).

## Goal

Make the grid agree with itself. Today three separate rules decide which trip comes
before which, and two of them live in Svelte. The same stored data therefore shows a
different "Km pred" depending on which rule produced it, and saving a row rewrites
rows the user never touched.

## How it was found

While checking the [task 79](../79-odometer-span-inconsistency/) warnings against a
local copy of the production book, the user set the vehicle's `initial_odometer` to
its correct value and reported that the first 2023 row still showed the same start
and end odometer. It then changed on its own, and the user observed: "the browser
feedback is NOT the same as the backend compute. I change the values, something
updates, yet when full refresh the DATA IS DIFFERENT."

That is correct. The cause is not a hidden backend layer -- a probe container proved
the backend writes only the row it is given (below). The cause is that the frontend
computes the odometer chain twice, by two rules, neither of which matches the
backend's.

## The three orderings

| Rule | Where | Orders by |
|---|---|---|
| `odometerStart`, shown in the Km pred column | `calculate_odometer_start` ([helpers.rs](../../src-tauri/core/src/commands_internal/helpers.rs)) fed by `chronological` ([statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs)) | date, then **odometer** |
| `previousOdometer`, used by the row editor to auto-fill ODO | [TripGrid.svelte:750, 822](../../src/lib/components/TripGrid.svelte) | display order, that is **trip number** |
| running total that rewrites the whole year on save | [TripGrid.svelte:443](../../src/lib/components/TripGrid.svelte) `recalculateAllOdo` | `[...trips].reverse()`, the **DB order** |

Trip numbers come from `calculate_trip_numbers(&trips)`, which sorts the DB result
(`start_datetime DESC, created_at ASC`, [db.rs:404](../../src-tauri/core/src/db.rs))
by date, datetime and `created_at`. The odometer chain sorts a list that was already
pre-sorted by odometer. Rust's `sort_by` is stable, so when all three keys tie the
two paths keep two different orders.

## Measured on the production copy (329 trips)

Rows where the number the editor would use differs from the number the grid displays:

| Year | Rows | Disagree |
|---|---|---|
| 2023 | 69 | 34 |
| 2024 | 84 | 36 |
| 2025 | 68 | 19 |
| 2026 | 108 | **0** |

2026 is clean because those rows were typed over time and carry distinct `created_at`.
2023 to 2025 came from one import that stamped 66 rows identically, so every rule
falls through to a different arbitrary tie-break.

## The backend is not the problem

On an isolated probe container holding a copy of the same database:

- `update_trip` changing only `purpose`: 0 other rows changed.
- `update_trip` changing the odometer 43091 -> 43098: exactly 1 row changed, the one
  sent.

There is no cascade in Rust and `get_trip_grid_data` writes nothing.

## The comparator

The secondary key was chosen by measurement, not by taste. Span warnings raised over
the whole book under each candidate ordering:

| Order | Warnings | Verdict |
|---|---|---|
| datetime, created_at, id | 65 | Declares two thirds of the imported book broken |
| datetime, odometer, id | 1 | Reads perfectly, but **silences all three 2026 errors** |
| **datetime, created_at, odometer, id** | **4** | 2023 and 2025 clean, 2024 boundary, 2026 errors kept |

Ordering by odometer alone makes the chain agree with itself by construction, so a
wrong odometer can never be detected. The two tie-breaks disagree in exactly one
group of the 31, the 2026-08-19 pair, and there the places settle it: `created_at`
order connects 2 of 2 place links (Mlynske Nivy -> OMV, then OMV -> Spisska Nova
Ves), odometer order connects 0 of 2.

The place chain itself cannot be the rule: it uniquely settles only 17 of the 31
groups and cannot settle 14 of them, because of round trips and gaps in the book.

**Decision to record:** order by `start_datetime`, then `created_at`, then `odometer`
ascending, then `id`. Each key is used only when the one before it carries no
information. `id` makes the order total, so nothing depends on the DB row order or on
a stable-sort accident.

## Requirements

### One comparator

- One ordering function in Rust, used by `calculate_trip_numbers`,
  `calculate_odometer_start` and `generate_month_end_rows`.
- Remove the odometer pre-sort of `chronological` in `build_trip_grid_data`; the
  comparator carries the odometer key itself.
- With one comparator, Km pred equals the previous row's ODO by construction. Any
  remaining inconsistency surfaces as a task 79 span warning instead of a jumping
  column.

### The editor stops computing the chain

- The row editor must use the backend `odometerStart` for the row it edits, not a
  display neighbour. Delete the `previousOdometer` prop plumbing at
  [TripGrid.svelte:717, 750, 822](../../src/lib/components/TripGrid.svelte).
- `formData.odometer = previousOdometer + km` ([TripRow.svelte:218, 243, 321](../../src/lib/components/TripRow.svelte))
  and the clamp at 347 and 419 are business arithmetic in Svelte, which
  [ADR-008](../../DECISIONS.md) forbids. They move to Rust.
- Keep the two-way km/odo behaviour the user relies on
  ([km-odo-bidirectional.spec.ts](../../tests/integration/specs/tier1/km-odo-bidirectional.spec.ts)).

### `recalculateAllOdo` moves to Rust

- One command over the canonical order, replacing the frontend loop at
  [TripGrid.svelte:443](../../src/lib/components/TripGrid.svelte).
- Decide during design whether it stays automatic on every save. Today it rewrites
  rows the user never touched and leaves no record of what changed, which is how a
  22 km correction walked through 2023 and then 2024 during a single browser session.

## Constraints

- **Trip numbers change for 2023 to 2025.** That is the legal "Poradové číslo jazdy"
  column. The renumbering is the point of the fix, but the user must confirm it
  against a copy before anything runs on the production book.
- The task 79 warnings must keep firing on the 2026 rows. A comparator that silences
  them is wrong, however clean the resulting book looks.
- Do not write to production. Per `CLAUDE.local.md` the live database is copy-from
  only.

## Related

- [Task 79](../79-odometer-span-inconsistency/) -- the warnings that made this
  visible, and the three rows still to correct.
- [ADR-008](../../DECISIONS.md) -- no frontend calculation duplication.
- [ADR-043](../../DECISIONS.md) -- why the span check must not be measured against an
  order derived from the odometer.
