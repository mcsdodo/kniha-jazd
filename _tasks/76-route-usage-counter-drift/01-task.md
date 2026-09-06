**Date:** 2026-09-06
**Subject:** The `routes` autocomplete cache counts saves, not trips — `usage_count` and `last_used` drift and never recover
**Status:** Planning

# Task 76: Route Usage Counter Drift

## Goal

Make the `routes` table tell the truth about how often a journey was actually
driven, and keep it true for every write path — including the two that currently
corrupt it (editing a trip) and the one that silently ignores it (deleting a
trip).

## What the table is

`routes` is a denormalised autocomplete cache: one row per
`(vehicle_id, origin, destination)` — the DDL in
[the baseline migration](../../src-tauri/core/migrations/2026-01-08-095218-0000_baseline/up.sql)
enforces that with a `UNIQUE` constraint — carrying `distance_km`, `usage_count`
and `last_used`. It exists to serve two things in the trip form: the list of
place names offered in the origin/destination inputs, and a prefilled km value
once both ends are chosen.

Do not confuse it with `trip_routes`
([its migration](../../src-tauri/core/migrations/2026-08-10-100000_add_trip_routes/up.sql)),
which stores map polylines for a single trip. Different table, different job,
not affected by this bug.

## The bug

`usage_count` is supposed to mean "number of trips using this pair". It does not.
It means "number of times a trip form was saved with this pair", and it only ever
goes up.

### 1. Editing a trip double-counts

`find_or_create_route` in [db.rs](../../src-tauri/core/src/db.rs) (line 544)
increments `usage_count` by one on every call. It is called from **both** write
paths in [trips.rs](../../src-tauri/core/src/commands_internal/trips.rs):
`create_trip_internal` (line 99) and `update_trip_internal` (line 171). Saving an
edit — even one that changes only the fuel field and leaves origin and
destination untouched — bumps the counter again. A row corrected five times
counts as six trips.

### 2. Deleting a trip decrements nothing

`delete_trip` in [db.rs](../../src-tauri/core/src/db.rs) (line 465) removes the
trip and its Paperless links. It does not touch `routes`. Nothing anywhere
decrements `usage_count`. The only code that ever removes a route row is
`delete_vehicle` ([db.rs](../../src-tauri/core/src/db.rs), line 310), which
cascades away every route for that vehicle.

### 3. Rows outlive their last trip

Follows from (2): delete every trip for a pair and the route row survives with
`usage_count >= 1`, still offering its place names and its km prefill. Its
`last_used` can point at a trip that no longer exists.

### Measured drift

Checked against the maintainer's production database (330 trips, 96 route rows):

| Measure | Value |
|---------|-------|
| Route rows whose `usage_count` disagrees with the real trip count for their pair | **52 of 96** |
| Worst stored-vs-actual ratios | 126 vs 15 · 113 vs 17 · 102 vs 27 |
| Vehicles with zero trips but leftover route rows | 1 (1 orphan row) |

No addresses, trip ids or dates are recorded here — this repository is public.
Illustrative examples in this document use invented places
("Warehouse, City B" → "Office, City A").

## Impact: cosmetic, not legal

`routes` feeds nothing that matters legally. Grep confirms `usage_count` and
`last_used` are read in exactly one place: the `ORDER BY usage_count DESC` in
`get_routes_for_vehicle` ([db.rs](../../src-tauri/core/src/db.rs), line 477). No
consumption rate, odometer value, margin check or export reads the table. The
20 % over-consumption calculation never sees it.

**It is even quieter than that.** The frontend discards the ordering. In
[TripRow.svelte](../../src/lib/components/TripRow.svelte), `locationSuggestions`
flattens origins and destinations into a `Set` and then sorts them
alphabetically, and `tryAutoFillDistance` finds its row by exact pair match —
unique by construction. So today the ranking has **no observable effect on the
UI at all**, which is exactly why years of drift went unnoticed.

Two reasons to fix it anyway:

1. The field is a lie that any future feature will believe. The place book in
   [task 75](../75-place-book/) is the obvious candidate to start ranking
   suggestions by real usage; it would inherit a counter that is 8× too high on
   some rows.
2. Orphan rows keep dead places in the autocomplete list and keep prefilling
   distances for journeys the user deleted.

## The design choice

Two defensible shapes. **This is the decision to make before any code is
written; it is not settled here.**

### Option A — keep the stored counters, maintain them properly

Every write path becomes responsible for the counter:

- `create_trip_internal` — increment (unchanged).
- `update_trip_internal` — compare the saved trip's old origin/destination
  against the new pair. Unchanged → touch nothing but `last_used`. Changed →
  decrement the old row (deleting it at zero) and increment the new one.
- `delete_trip_internal` — decrement the row for the deleted trip's pair, delete
  it at zero.

**For:** the read path stays a single indexed `SELECT`; the stored value is what
the schema already promises.

**Against:** correctness now depends on three call sites staying in sync forever,
and on every future write path remembering to join the protocol. It already
failed this test twice. It is also not the only way in — backup restore and the
place-text rewrite in [task 75](../75-place-book/) both mutate rows underneath
the counters. And it needs a backfill migration to repair the 52 rows already
wrong.

### Option B — derive `usage_count` and `last_used` from `trips`

`routes` keeps `distance_km` (and the pair identity). The counters stop being
stored state and become an aggregate computed by `get_routes_for_vehicle`:

```sql
SELECT r.id, r.vehicle_id, r.origin, r.destination, r.distance_km,
       COUNT(t.id)           AS usage_count,
       MAX(t.start_datetime) AS last_used
FROM routes r
JOIN trips t
  ON t.vehicle_id  = r.vehicle_id
 AND t.origin      = r.origin
 AND t.destination = r.destination
WHERE r.vehicle_id = ?
GROUP BY r.id
ORDER BY usage_count DESC
```

An inner join drops orphan rows from the result for free. `update_trip_internal`
and `delete_trip_internal` need no route bookkeeping at all;
`find_or_create_route` shrinks to "ensure the pair exists, remember its
distance".

**For:** the drift class stops existing — there is no second copy of the number
to disagree with the first. Data volume is trivial (hundreds of rows, one grouped
query per vehicle per grid load). It survives any future write path, including
task 75's SQL rewrite, without that path knowing routes exist. No counter
backfill is needed: the numbers are correct on the first read after deploy.

**Against:** the columns stay in the schema unless a migration drops them, so the
table briefly carries two truths; and the aggregate is marginally more work per
read than an ordered select — immaterial at this size, but it is a real
difference.

### Recommendation: Option B

Derive. The stored counter has been wrong for the entire life of the feature and
nobody noticed, which is the strongest possible evidence that it is not worth the
maintenance protocol Option A imposes. Option A adds three invariants that must
hold across every present and future write path in order to reproduce a number
SQLite can compute exactly, on demand, from data that is already authoritative —
over a table with fewer than a hundred rows. Option B deletes the failure mode
instead of policing it, and it is strictly less code: two call sites lose work
rather than gaining it.

If Option B is chosen, drop `usage_count` and `last_used` from the `routes`
schema in the same change (forward-only migrations per
[ADR-012](../../DECISIONS.md)) so no stale copy is left to mislead a reader.

**Open sub-question, deliberately out of scope:** if the counters are derived,
`distance_km` is the only stored field left, and it is *also* derivable from
`trips`. Note that its current semantics ("distance of the first trip ever saved
for this pair" — `find_or_create_route` never updates it) differ from the obvious
derived form ("most recent trip's distance"). Collapsing the table entirely would
change prefill behaviour and would collide with [task 75](../75-place-book/),
which may repurpose route rows. Leave the table in place; record the observation.

## Requirements

### R1 — Editing a trip must not inflate the count

Saving an edit that leaves origin and destination unchanged leaves the count
unchanged. Saving an edit that moves the trip from "Warehouse, City B → Office,
City A" to "Warehouse, City B → Depot, City C" must not leave both pairs claiming
the trip.

### R2 — Deleting a trip must be reflected

After deleting a trip, the count for its pair reflects the remaining trips.

### R3 — A route with no trips must not be offered

When the last trip using a pair is deleted, that pair stops appearing in
autocomplete and stops prefilling a distance.

### R4 — Existing data must be repaired

Fixing the write paths does not repair the 52 already-wrong rows. **Scope depends
on the option chosen:**

- **Option A** — a data migration under
  [src-tauri/core/migrations/](../../src-tauri/core/migrations/) that recomputes
  `usage_count` and `last_used` for every row from `trips`, and deletes rows with
  no trips. Follow the precedent of
  [the receipt-currency backfill](../../src-tauri/core/migrations/2026-01-21-110000_backfill_receipt_currency/up.sql).
  Migrations back the database up before running
  (see [.claude/rules/migrations.md](../../.claude/rules/migrations.md)).
- **Option B** — no counter backfill exists to write. A migration is still wanted
  to drop the two dead columns and prune orphan rows, but it repairs nothing
  user-visible: the derived read is already correct.

### R5 — Normalisation must be verified, not assumed

Both write paths in
[trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) run
`normalize_location` before storing, and `find_or_create_route` normalises again.
Rows written before normalisation existed, or restored from a backup, may not
match byte-for-byte. Under Option B a mismatch silently reports zero uses and
hides the route; under Option A the backfill zeroes it. **Confirm against the
production snapshot that every route row joins at least one trip before shipping
either**, and cover the mismatch case in a test rather than trusting the
invariant.

## Test plan (TDD — failing test first, per [CLAUDE.md](../../CLAUDE.md))

Backend unit tests own all of this; the behaviour is data logic with no new UI.
Per [.claude/rules/rust-backend.md](../../.claude/rules/rust-backend.md), tests go
in the companion `*_tests.rs` files, never in the source file.

In [db_tests.rs](../../src-tauri/core/src/db_tests.rs) — alongside the existing
`test_find_or_create_route_upsert`, which asserts `usage_count == 2` after two
calls and therefore **encodes the bug**; it must be rewritten, not deleted:

| Test | Asserts |
|------|---------|
| `test_route_usage_count_matches_trip_count` | three trips on one pair → count 3 |
| `test_route_usage_count_unchanged_when_trip_edited` | R1, unchanged pair |
| `test_route_usage_count_follows_edited_pair` | R1, changed pair — old down, new up |
| `test_route_usage_count_drops_when_trip_deleted` | R2 |
| `test_route_absent_when_last_trip_deleted` | R3 |
| `test_route_last_used_matches_latest_trip` | `last_used` tracks the newest trip |
| `test_routes_ordered_by_usage_count_desc` | ordering contract of `get_routes_for_vehicle` |
| `test_route_with_unmatched_place_text` | R5 — a route row whose text no trip matches |

In [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs)
— the same behaviour driven through `create_trip_internal`,
`update_trip_internal` and `delete_trip_internal`, so the fix is proven at the
command boundary the RPC dispatcher actually calls, not only at the DB helper.

In [migration_tests.rs](../../src-tauri/core/src/migration_tests.rs) — **only if
Option A is chosen**: seed a legacy-schema DB with a deliberately inflated counter
and an orphan row, run the migration, assert the counter matches the trip count
and the orphan is gone. Name: `test_route_usage_backfill_matches_trip_counts`.

**No integration test.** There is no user-visible flow to cover: as established
above, the frontend sorts suggestions alphabetically and matches distances by
exact pair, so nothing in the UI changes. Adding a WebdriverIO spec here would be
the filler the testing strategy in [CLAUDE.md](../../CLAUDE.md) explicitly
forbids. If a later task makes the ranking visible, the integration test belongs
to *that* task.

## Relationship to task 75

[Task 75](../75-place-book/) contains an already-written one-off cleanup that
rewrites place text in both `trips` and `routes`, folding rows that collide on the
`UNIQUE(vehicle_id, origin, destination)` constraint by summing their
`usage_count` and keeping the later `last_used`. It deliberately **preserves the
existing counter values rather than recomputing them** — it is a text fix, not a
counter fix.

The two are therefore orthogonal and may land in either order:

- **76 first** — task 75's fold sums counters that are already correct, and its
  result stays correct.
- **75 first** — task 75's fold sums counters that are wrong, and task 76 either
  recomputes them (Option A) or stops reading them (Option B). Either way the
  drift is erased afterwards.

Under Option B the interaction disappears entirely: task 75 can rewrite place text
however it likes, because the counters follow `trips` and never need folding. That
is a point in Option B's favour, not a dependency.

## Out of scope

- Collapsing `routes` into a fully derived view (see the open sub-question above).
- Any change to the place book itself — that is [task 75](../75-place-book/).
- Making the ranking visible in the UI (alphabetical suggestions stay as they are).
- `trip_routes` map polylines — unrelated table.

## Next step

Pick Option A or Option B. Once chosen, the step-by-step plan is written into this
folder via the `superpowers:writing-plans` skill as required by
[the task conventions](../CLAUDE.md), and a
`/decision` entry (ADR-032) records the choice in
[DECISIONS.md](../../DECISIONS.md) — this is exactly the "choosing between
multiple valid approaches" case that file asks to be documented.
