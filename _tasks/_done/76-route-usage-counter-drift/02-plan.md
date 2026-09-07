**Date:** 2026-09-06
**Subject:** Derive the route autocomplete's counters instead of storing them
**Status:** Complete

# Route Usage Counter Drift Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Stop storing `routes.usage_count` and `routes.last_used`, compute them from `trips` on read, and delete the columns.

**Architecture:** [get_routes_for_vehicle](../../src-tauri/core/src/db.rs) becomes one grouped query joining `routes` to `trips`. An **inner** join is the point: a route row no trip justifies stops being returned, which is how orphans disappear without a cleanup pass. `find_or_create_route` keeps storing `distance_km` and stops touching the counters. Then the columns go, forward-only per [ADR-012](../../DECISIONS.md).

**Tech Stack:** Rust, Diesel, SQLite.

Requirements and the options that were weighed: [01-task.md](./01-task.md). The decision: [ADR-033](../../DECISIONS.md).

---

## Why this is safe to do bluntly

Verified before planning, and worth re-checking if this sits for a while:

- **Nothing observable depends on the stored values.** They are read in exactly one place — the `ORDER BY usage_count DESC` in `get_routes_for_vehicle` — and [TripRow.svelte:166-168](../../src/lib/components/TripRow.svelte) then flattens the result into a `Set` and sorts it alphabetically, throwing the ordering away. The only other mentions in the whole tree are the two type fields at [types.ts:79-80](../../src/lib/types.ts).
- **The R5 normalisation risk did not materialise.** Against the production snapshot, 8 of 91 route rows join no trip — but every one is genuine junk, not a live route hidden by a spelling mismatch: an `A → B`, a row with **empty** origin and destination, a half-typed `kamen`, a deleted test trip's pair, and two carrying a `Bratilsava` typo that was corrected on the trips but left behind here.
- **Those 8 are currently offered in the trip form's autocomplete**, empty string included. The inner join removes them, which is a small user-visible win this task gets for free — and the reason Task 5 runs `/changelog`.

---

## Task 1: Derive the counters on read

**Files:**
- Modify: [src-tauri/core/src/db.rs](../../src-tauri/core/src/db.rs) (`get_routes_for_vehicle`, `:477`)
- Modify: [src-tauri/core/src/db_tests.rs](../../src-tauri/core/src/db_tests.rs)

**Step 1: Write the failing tests**

```rust
#[test]
fn route_usage_counts_trips_not_saves() {
    // The bug in one assertion: editing a trip must not inflate the count.
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();

    let trip = seed_trip_between(&db, &v.id, "Office, City A", "Depot, City B");
    db.update_trip(&trip).unwrap();
    db.update_trip(&trip).unwrap();

    let routes = db.get_routes_for_vehicle(&v.id.to_string()).unwrap();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].usage_count, 1, "one trip, however many times it was saved");
}

#[test]
fn deleting_the_last_trip_removes_the_suggestion() {
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    let trip = seed_trip_between(&db, &v.id, "Office, City A", "Depot, City B");

    db.delete_trip(&trip.id.to_string()).unwrap();

    assert!(db.get_routes_for_vehicle(&v.id.to_string()).unwrap().is_empty());
}

#[test]
fn last_used_is_the_most_recent_trip_on_that_pair() {
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    seed_trip_between_on(&db, &v.id, "Office, City A", "Depot, City B", "2024-03-01T08:00:00");
    seed_trip_between_on(&db, &v.id, "Office, City A", "Depot, City B", "2025-07-14T08:00:00");

    let routes = db.get_routes_for_vehicle(&v.id.to_string()).unwrap();
    assert_eq!(routes[0].usage_count, 2);
    assert!(routes[0].last_used.to_rfc3339().starts_with("2025-07-14"));
}

#[test]
fn suggestions_are_ordered_by_real_usage() {
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    seed_trip_between(&db, &v.id, "Rare, City C", "Depot, City B");
    seed_trip_between(&db, &v.id, "Office, City A", "Depot, City B");
    seed_trip_between(&db, &v.id, "Office, City A", "Depot, City B");

    let routes = db.get_routes_for_vehicle(&v.id.to_string()).unwrap();
    assert_eq!(routes[0].origin, "Office, City A", "most-used first");
}

/// The 8 rows the production database carries whose trips are gone — including
/// one with empty endpoints — must stop being suggested.
#[test]
fn a_route_row_no_trip_justifies_is_not_returned() {
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    // Written directly, the way a deleted trip leaves one behind.
    db.find_or_create_route(&v.id.to_string(), "Ghost, City X", "Nowhere", 10.0).unwrap();

    assert!(db.get_routes_for_vehicle(&v.id.to_string()).unwrap().is_empty());
}

#[test]
fn another_vehicles_trips_do_not_count_towards_this_ones_routes() {
    // routes are vehicle-scoped and the join must stay so.
    let db = Database::in_memory().unwrap();
    let a = create_test_vehicle("A");
    let b = create_test_vehicle("B");
    db.create_vehicle(&a).unwrap();
    db.create_vehicle(&b).unwrap();
    seed_trip_between(&db, &a.id, "Office, City A", "Depot, City B");
    seed_trip_between(&db, &b.id, "Office, City A", "Depot, City B");

    let routes = db.get_routes_for_vehicle(&a.id.to_string()).unwrap();
    assert_eq!(routes[0].usage_count, 1);
}
```

Add `seed_trip_between` / `seed_trip_between_on` beside `create_test_vehicle` (`db_tests.rs:93`) as `pub(crate)`; [task 75](../75-place-book/03-plan.md) needs the same helper, so write it once.

**Step 2: Run to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core route_usage`
Expected: FAIL — `usage_count` still counts saves, and the orphan test returns one row.

**Step 3: Implement**

Replace the body of `get_routes_for_vehicle`. Raw SQL, following `get_purposes_for_vehicle` (`db.rs:489`) — the aggregate join reads far better as the query itself than through the DSL:

```rust
    /// Autocomplete suggestions for a vehicle, most-used first.
    ///
    /// `usage_count` and `last_used` are computed from `trips` rather than
    /// stored (ADR-033): three write paths were meant to keep stored copies
    /// current and none did, leaving 52 of 96 rows wrong in production.
    ///
    /// The join is INNER by design. A `routes` row whose trips have all been
    /// deleted is a suggestion for a journey the logbook no longer contains,
    /// so it drops out here instead of needing a cleanup pass.
    pub fn get_routes_for_vehicle(&self, vehicle_id: &str) -> QueryResult<Vec<Route>> {
        let conn = &mut *self.conn.lock().unwrap();

        #[derive(QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            id: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Text)]
            vehicle_id: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            origin: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            destination: String,
            #[diesel(sql_type = diesel::sql_types::Double)]
            distance_km: f64,
            #[diesel(sql_type = diesel::sql_types::Integer)]
            usage_count: i32,
            #[diesel(sql_type = diesel::sql_types::Text)]
            last_used: String,
        }

        let rows = diesel::sql_query(
            "SELECT r.id, r.vehicle_id, r.origin, r.destination, r.distance_km,
                    COUNT(t.id) AS usage_count,
                    MAX(t.start_datetime) AS last_used
               FROM routes r
               JOIN trips t
                 ON t.vehicle_id = r.vehicle_id
                AND t.origin = r.origin
                AND t.destination = r.destination
              WHERE r.vehicle_id = ?
              GROUP BY r.id
              ORDER BY usage_count DESC",
        )
        .bind::<diesel::sql_types::Text, _>(vehicle_id)
        .load::<Row>(conn)?;

        rows.into_iter().map(Route::try_from).collect()
    }
```

**Do not reuse `impl From<RouteRow> for Route`.** This query returns its own `Row`, and the conversion differs in the one way that matters: `last_used` arrives as `trips.start_datetime` in `%Y-%m-%dT%H:%M:%S`, not the RFC 3339 the dropped column held. Parse it the way `find_most_recent_trip_times_for_route` already does (`db.rs:513`) — a second format path invented here is how a "works on my machine" date bug starts. Write the conversion inline in `db.rs` next to the query, or as a small private fn; it is not part of the model's public surface.

**Step 4: Run to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core route`
Expected: PASS, including the pre-existing route tests.

**Step 5: Commit**

```bash
git add src-tauri/core/src/db.rs src-tauri/core/src/db_tests.rs
git commit -m "fix(routes): count trips instead of saves"
```

---

## Task 2: Stop writing the counters

**Files:**
- Modify: [src-tauri/core/src/db.rs](../../src-tauri/core/src/db.rs) (`find_or_create_route`, `:544`)
- Modify: [src-tauri/core/src/db_tests.rs](../../src-tauri/core/src/db_tests.rs) (`test_find_or_create_route_upsert`, `:306`)

**Step 1: Rewrite the test that encodes the bug**

`test_find_or_create_route_upsert` currently asserts `usage_count == 2` after two calls — it is a written-down statement of the defect and must be **rewritten, not extended**:

```rust
#[test]
fn test_find_or_create_route_upsert() {
    let db = Database::in_memory().unwrap();
    let vehicle = create_test_vehicle("Test Car");
    db.create_vehicle(&vehicle).unwrap();

    let route1 = db
        .find_or_create_route(&vehicle.id.to_string(), "Budapest", "Prague", 500.0)
        .unwrap();
    let route2 = db
        .find_or_create_route(&vehicle.id.to_string(), "Budapest", "Prague", 500.0)
        .unwrap();

    // One row per pair, still. What changed is that saving twice no longer
    // pretends the journey was driven twice — usage is counted from trips now.
    assert_eq!(route2.id, route1.id);
    assert_eq!(db.all_route_rows_for_test(&vehicle.id.to_string()).unwrap().len(), 1);
}
```

`get_routes_for_vehicle` cannot be used to make this assertion any more — after Task 1 it inner-joins trips, and this test creates none, so it would return empty whether there were one row or five. Add a `#[cfg(test)]` accessor on `Database` that loads `routes` rows unjoined:

```rust
    /// Rows as stored, bypassing the trips join. Tests about the table itself
    /// need this; nothing in the application does.
    #[cfg(test)]
    pub fn all_route_rows_for_test(&self, vehicle_id: &str) -> QueryResult<Vec<RouteRow>> {
        let conn = &mut *self.conn.lock().unwrap();
        routes::table
            .filter(routes::vehicle_id.eq(vehicle_id))
            .load::<RouteRow>(conn)
    }
```

**Step 2: Run — it fails** (`usage_count` still incremented; the accessor does not exist yet).

**Step 3: Implement.** In `find_or_create_route`, delete the `usage_count`/`last_used` update on the existing-row branch and the initial values on the insert branch. The function's remaining job is: one row per `(vehicle, origin, destination)`, carrying `distance_km` for the prefill.

**Step 4: Run — PASS. Step 5: Commit.**

```bash
git add src-tauri/core/src/db.rs src-tauri/core/src/db_tests.rs
git commit -m "fix(routes): stop incrementing a counter nothing reads"
```

---

## Task 3: Drop the columns

Only now — with nothing writing them and nothing reading the stored values — do they go. Forward-only, per [ADR-012](../../DECISIONS.md) and the [migration conventions](../../.claude/rules/migrations.md).

**Files:**
- Create: `src-tauri/core/migrations/2026-09-06-110000_drop_route_counters/up.sql`
- Create: `src-tauri/core/migrations/2026-09-06-110000_drop_route_counters/down.sql`
- Modify: [src-tauri/core/src/schema.rs](../../src-tauri/core/src/schema.rs)
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) (`RouteRow`, `NewRouteRow`; `Route` keeps both fields — they are now computed)
- Modify: [src-tauri/core/src/migration_tests.rs](../../src-tauri/core/src/migration_tests.rs)

**Step 1: Write the migration test**

```rust
#[test]
fn dropping_the_route_counters_keeps_the_suggestions() {
    // A database written before this migration must still produce autocomplete
    // suggestions afterwards, with counts that now reflect its trips.
    // Build a routes row + two trips on the same pair, migrate, and assert
    // get_routes_for_vehicle returns usage_count == 2.
}
```

**Step 2: Write the migration**

`up.sql`:

```sql
-- Task 76: usage_count and last_used were stored aggregates of trips, maintained
-- by write paths that never agreed - 52 of 96 rows were wrong. Both are computed
-- in get_routes_for_vehicle now (ADR-033), so the stored copies are deleted
-- rather than corrected: there is nothing left that could drift.
--
-- distance_km stays. It is the value typed for the pair, not an aggregate.
ALTER TABLE routes DROP COLUMN usage_count;
ALTER TABLE routes DROP COLUMN last_used;
```

`down.sql`:

```sql
-- Restores the columns but not their values: the numbers they held were wrong,
-- and recreating them accurately would mean recomputing from trips, which is
-- what dropping them made unnecessary.
ALTER TABLE routes ADD COLUMN usage_count INTEGER NOT NULL DEFAULT 1;
ALTER TABLE routes ADD COLUMN last_used TEXT NOT NULL DEFAULT '';
```

**Step 3:** Remove both fields from `RouteRow` and `NewRouteRow` and from the `routes` block in `schema.rs`. **Leave `Route` itself alone** — it still carries `usage_count` and `last_used`, now populated by the query rather than the table.

**Step 4:** Run `cargo test --manifest-path src-tauri/Cargo.toml --workspace`. Expected: PASS.

**Step 5: Commit**

```bash
git add src-tauri/core/migrations/2026-09-06-110000_drop_route_counters/ src-tauri/core/src/schema.rs src-tauri/core/src/models.rs src-tauri/core/src/migration_tests.rs
git commit -m "refactor(routes): drop the stored counters"
```

---

## Task 4: Confirm the frontend needs nothing

`usageCount` and `lastUsed` still exist at [types.ts:79-80](../../src/lib/types.ts) and the backend still sends both, so **no frontend change is required**. Verify that rather than assuming it:

**Step 1:** `grep -rn "usageCount\|lastUsed" src/` — expect only the two type declarations.
**Step 2:** `npm run check` — expect clean.

If the grep finds a reader, stop: this plan assumed there is none, and that assumption is load-bearing for Task 1's claim that ordering is unobservable.

No commit unless something turned up.

---

## Task 5: Verify and document

1. `npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web`
2. `npm run test:all` — the merge gate.
3. **Check the real fix on real data.** Point a server at a copy of the production database and confirm the trip form's suggestions no longer include the empty entry, `kamen`, `A`, or the `Bratilsava` rows. This is the user-visible change and the only step that proves it.
4. `/changelog` — user-visible: junk suggestions disappear from the origin/destination autocomplete.
5. Update [_tasks/index.md](../index.md) → ✅ Complete, move the folder to `_done/`.

```bash
git add CHANGELOG.md _tasks/index.md
git commit -m "docs: record the route suggestion cleanup"
```

---

## Not doing

- **A counter backfill.** There is nothing to correct once the columns are gone. This is the main saving over the maintained-counters option.
- **Deleting the orphan rows.** The inner join stops returning them; deleting them is a separate decision about someone else's data, and they cost bytes rather than correctness. Revisit only if they ever become visible again.
- **Deriving `distance_km` too.** Noted as an open sub-question in [01-task.md](./01-task.md) — its current semantics ("the distance of the first trip saved on this pair") are not the same as any aggregate, so changing it is a behaviour change, not a cleanup.
