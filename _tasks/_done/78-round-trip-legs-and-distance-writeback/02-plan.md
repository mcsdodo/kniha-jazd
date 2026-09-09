# Round-Trip Legs and Distance Write-Back Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Route each leg of a round trip separately so both legs offer alternatives, and let the user write the routed distance back onto the trip row behind a warning that names the effect on the consumption period and the 20 % legal margin.

**Architecture:** A round trip becomes two OSRM requests through one new Rust command, `route_round_trip`. It returns each leg's alternatives plus a precomputed table of combined distance and deviation for every pair, so the browser indexes a table and calculates nothing. A second new command, `apply_route_distance`, plans the write with the existing odometer cascade planner (ADR-046) and the existing period-rate calculator, returns that plan as a dry run for the confirmation modal, and writes the row plus its cascade in one transaction on the second call.

**Tech Stack:** Rust (`kniha-jazd-core`), Axum JSON-RPC, Diesel + SQLite, SvelteKit 5 (runes) + Leaflet, typesafe-i18n, WebdriverIO.

**Spec:** [01-task.md](./01-task.md)

---

## Global Constraints

Every task's requirements implicitly include this section.

- **ADR-008.** All calculation lives in Rust. The frontend displays values the backend produced. It never sums two distances, never derives a deviation, never assembles a waypoint list or a polyline.
- **TDD.** Write the failing test, run it and see it fail, write the minimal code, run it and see it pass, commit. Never write implementation code first.
- **Backend tests own the maths; integration tests own the UI flow.** Do not re-test a calculation through the browser.
- **Integration tests never call OSRM.** `generate_route`, `route_direct` and `route_round_trip` hit the public OSRM demo server, and there is no provider override. Every integration test seeds a route with a save command and a canned polyline. See the header of `tests/integration/specs/tier2/route-map.spec.ts`.
- **Write commands guard with `check_read_only!(app_state);` as their first statement.** A dry run writes nothing and is always allowed, even in read-only mode.
- **`RouteMapRow` binds POSITIONALLY.** A new column goes LAST in the migration, LAST in `schema.rs`, and LAST in both `RouteMapRow` and `NewRouteMapRow`. A field inserted between existing ones swaps two columns silently, with no compile error. The struct already carries this warning twice; keep it true.
- **Slovak is the source of truth for i18n.** Add every key to `src/lib/i18n/sk/index.ts` and `src/lib/i18n/en/index.ts`, then run `npm run i18n` in the same task. Nothing else regenerates `i18n-types.ts`, so `npm run check` reports phantom errors until it runs.
- **Keyboard-typable characters only.** Use `--`, `->`, `...` and straight quotes. Slovak diacritics are required in the Slovak locale.
- **Stage only the files of the task you are committing.** Never `git add -A`.
- **The legal test is `margin_percent <= 20.0`** (`calculations/mod.rs`, `is_within_legal_limit`). A period's rate is `period_fuel / period_km * 100`, and a period closes on a full-tank fill-up (`calculate_period_rates`, `statistics.rs`).

---

## Design decisions

The spec closes with four open questions. These are the answers this plan builds on, with the reasoning, so a reviewer can reject the answer rather than discover it.

### Q1: Does write-back shift every later trip's odometer, or only this row's?

**It shifts every later row of the same year, exactly as a distance edit typed into the grid already does.**

This is not a new decision. ADR-046 (task 81) already decided it for a distance edit: `plan_odometer_cascade` sets `odometer = anchor + km` for the edited row and moves every later row of the year by the same `delta`, behind a confirmation modal the user approves first. Write-back IS a distance edit -- the only difference is where the number comes from. Reusing the planner keeps the odometer invariant the spec measured (105 of 108 rows) and gives write-back the same review surface the grid already has.

Two consequences the plan carries rather than hides:

- The walk stops at the year end. `year_end_odometer_moved` and `next_year_chain_breaks` report the boundary, and `OdometerCascadeModal` already renders the second one.
- On the km-wins branch, `delta` also absorbs `delta_from_repair` for a row whose stored odometer disagrees with `anchor + km` (three such rows exist in the production book). `OdometerCascadeModal` already shows that as a separate line under `showRepair`. Do not suppress it for write-back: it is exactly the case where the user must see that two different corrections are travelling together.

### Q2: Is write-back allowed on a trip inside an already-closed period?

**Yes, and the warning names what it does to that period.**

A closed period's rate is what the legal margin is judged on, so refusing the write there would refuse it in exactly the case the spec was written for -- the worked example (50.0 recorded, 25.6 one-way, 51.5 round trip) is a row in a real, closed book. Blocking it would leave the user hand-editing the same number in the grid with no warning at all, which is strictly worse.

Instead, `apply_route_distance` returns a `PeriodMarginImpact`: the period's rate and margin before and after, and whether each side is over the 20 % limit. The modal shows both numbers and a prominent line when the change crosses the limit in either direction. A trip in the still-open period reports `periodClosed: false`, because that period's rate is the TP rate (an estimate), not a measured one.

### Q3: Does the user pick alternatives per leg independently, or pick a pair?

**Per leg, independently.** The spec says so: "Each leg offers its own alternatives, fastest-first, never reordered. The user can choose per leg." Two pickers, each holding its own leg's alternatives in OSRM's own order (ADR-038 -- never re-sort).

The deviation is a property of the pair, though, so the backend returns `combined[i][j]` for every pair it could produce (at most 3 x 3): combined road distance, combined duration, deviation percent and off-target flag, all from the same `deviation()` helper the one-way path uses. Selecting an alternative is then an index change, with no extra request and no arithmetic in the browser.

### Q4: Is the round-trip flag still one boolean, or must a saved route remember an alternative per leg?

**One boolean, plus one integer.**

`round_trip` stays a boolean. The chosen alternatives are not persisted -- exactly as today, where only the chosen geometry is saved and the picker comes back empty on a cold load until the user recalculates.

What the row does need is `turnaround_index`: the index, in the saved combined waypoint list, of the point where the outbound leg ends and the return leg begins. Without it, a saved `[A, B, v, A]` cannot be split back into its two legs, so reopening it and recalculating would put the return leg's via on the outbound one -- the exact bug this task exists to remove. The index also decides, on a cold load, whether each leg has a genuine intermediate stop, which is what the corrected `alternativesUnavailable` copy is gated on.

Legacy rows need no backfill and get none. Every round trip saved before this task was closed by appending exactly one clone of the first waypoint, so `[A, ...vias, B, A]` splits unambiguously at `len - 2`. A `NULL` index means "use that rule", and it is exact, not a guess.

### Two further decisions this plan makes

**`target_km` is read from the trip, not from the stored column.** `trip_routes.target_km` records what the trip measured when the map was saved. After a write-back (or any ordinary edit of the row) the trip's distance moves, and a map still showing the old target would report a deviation against a distance the book no longer holds. `get_trip_route_internal` therefore overrides `target_km` from the trip and keeps the stored column only as the fallback for a map whose trip has gone. This fixes the staleness for every distance edit, not only for write-back.

**`route_direct_internal` is not touched.** Its `round_trip` parameter and the symmetric normalisation of ADR-041 stay exactly as they are, with their regression tests. The map view no longer takes that path for a round trip, but the command is reachable over `POST /api/rpc`, and its normalisation is what protects a direct caller from an unroutable list. The new command sits beside it; it does not fold into it.

---

## File structure

| File | Change | Responsibility |
|---|---|---|
| `src-tauri/core/migrations/2026-09-09-100000_add_trip_route_turnaround_index/{up,down}.sql` | create | Adds `trip_routes.turnaround_index` |
| `src-tauri/core/src/schema.rs` | modify | Declares the new column LAST in `trip_routes!` |
| `src-tauri/core/src/models.rs` | modify | `RouteMap.turnaround_index`, `RouteMapRow`/`NewRouteMapRow` (LAST), `PeriodMarginImpact`, `DistanceWriteback` |
| `src-tauri/core/src/db.rs` | modify | `save_route_map` binds the new column |
| `src-tauri/core/src/commands_internal/route_maps.rs` | modify | `Leg`, `LegInsertPoint`, `LegRoute`, `CombinedLeg`, `RoundTripRoutes`, `route_round_trip_internal`, `persist_route_map`, `save_trip_round_trip_route_internal`, `get_trip_route_internal` target override |
| `src-tauri/core/src/commands_internal/route_maps_tests.rs` | modify | Leg routing, insertion per leg, save assembly, target override |
| `src-tauri/core/src/commands_internal/statistics.rs` | modify | `period_margin_impact` |
| `src-tauri/core/src/commands_internal/commands_tests.rs` | modify | `period_margin_impact` cases |
| `src-tauri/core/src/commands_internal/trips.rs` | modify | `apply_route_distance_internal` |
| `src-tauri/core/src/commands_internal/commands_tests.rs` | modify | Write-back cases (every `commands_internal` test lives in this one file) |
| `src-tauri/core/src/server/dispatcher.rs` | modify | `save_trip_round_trip_route`, `apply_route_distance` |
| `src-tauri/core/src/server/dispatcher_async.rs` | modify | `route_round_trip` |
| `src/lib/types.ts` | modify | `Leg`, `LegInsertPoint`, `LegRoute`, `CombinedLeg`, `RoundTripRoutes`, `PeriodMarginImpact`, `DistanceWriteback`, `RouteMap.turnaroundIndex` |
| `src/lib/api.ts` | modify | `routeRoundTrip`, `saveTripRoundTripRoute`, `applyRouteDistance` |
| `src/routes/mapa/+page.svelte` | modify | Two legs: state, drawing, pickers, drag, save; the write-back button; the unresolved-endpoint gate |
| `src/lib/components/OdometerCascadeModal.svelte` | modify | `kind: 'writeback'` and the margin block |
| `src/lib/components/TripGrid.svelte` | modify | Refresh on the `trip-distance-updated` broadcast |
| `src/lib/i18n/{sk,en}/index.ts` | modify | New keys |
| `tests/integration/specs/tier2/route-map.spec.ts` | modify | Corrected round-trip assertions, cold-load leg split |
| `tests/integration/specs/tier2/route-distance-writeback.spec.ts` | create | The write-back flow end to end |
| `DECISIONS.md` | modify | ADR-047, ADR-048, supersession line on ADR-039 |
| `docs/features/route-maps.md` | modify | Both "known limitation" sections, data model, command list |
| `CHANGELOG.md` | modify | `[Unreleased]` |

**Coverage split, stated plainly.** Two-leg routing is covered by Rust unit tests only, with a stub `RouteProvider` -- the integration suite has no way to stand in for OSRM. Distance write-back is covered by Rust unit tests for the maths and by integration tests end to end, because a *saved* route displays with no routing call, so the button, the modal and the write are all reachable without the network.

---

### Task 1: Persist the turnaround index

**Files:**
- Create: `src-tauri/core/migrations/2026-09-09-100000_add_trip_route_turnaround_index/up.sql`
- Create: `src-tauri/core/migrations/2026-09-09-100000_add_trip_route_turnaround_index/down.sql`
- Modify: `src-tauri/core/src/schema.rs` (the `trip_routes!` block)
- Modify: `src-tauri/core/src/models.rs` (`RouteMap`, `RouteMapRow`, `NewRouteMapRow`, `From<RouteMapRow> for RouteMap`)
- Modify: `src-tauri/core/src/db.rs` (`save_route_map`)
- Test: `src-tauri/core/src/db_tests.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `RouteMap.turnaround_index: Option<i32>`, round-tripped through SQLite.

- [ ] **Step 1: Write the failing test**

Append to `src-tauri/core/src/db_tests.rs`:

```rust
#[test]
fn route_map_round_trips_its_turnaround_index() {
    // RouteMap, RouteMode and Waypoint are already imported at the top of
    // this file; Utc and Uuid are not.
    use chrono::Utc;
    use uuid::Uuid;

    let db = Database::in_memory().unwrap();
    let trip_id = Uuid::new_v4();

    let map = RouteMap {
        trip_id,
        waypoints: vec![
            Waypoint { lat: 48.0, lon: 17.0, name: Some("A".into()), node_idx: None },
            Waypoint { lat: 49.0, lon: 18.0, name: Some("B".into()), node_idx: None },
            Waypoint { lat: 48.5, lon: 17.5, name: None, node_idx: None },
            Waypoint { lat: 48.0, lon: 17.0, name: Some("A".into()), node_idx: None },
        ],
        polyline: "abc".to_string(),
        target_km: 100.0,
        road_km: 102.0,
        mode: RouteMode::Direct,
        dataset_version: None,
        created_at: Utc::now(),
        round_trip: true,
        turnaround_index: Some(1),
    };
    db.save_route_map(&map).unwrap();

    let loaded = db.get_route_map(&trip_id.to_string()).unwrap().unwrap();
    assert_eq!(
        loaded.turnaround_index,
        Some(1),
        "the split point between the two legs must survive a save and a load"
    );
}

#[test]
fn a_route_map_without_a_turnaround_index_loads_as_none() {
    use chrono::Utc;
    use uuid::Uuid;

    let db = Database::in_memory().unwrap();
    let trip_id = Uuid::new_v4();

    let map = RouteMap {
        trip_id,
        waypoints: vec![
            Waypoint { lat: 48.0, lon: 17.0, name: Some("A".into()), node_idx: None },
            Waypoint { lat: 49.0, lon: 18.0, name: Some("B".into()), node_idx: None },
        ],
        polyline: "abc".to_string(),
        target_km: 100.0,
        road_km: 102.0,
        mode: RouteMode::Direct,
        dataset_version: None,
        created_at: Utc::now(),
        round_trip: false,
        turnaround_index: None,
    };
    db.save_route_map(&map).unwrap();

    let loaded = db.get_route_map(&trip_id.to_string()).unwrap().unwrap();
    assert_eq!(loaded.turnaround_index, None);
}
```

- [ ] **Step 2: Run the test and watch it fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core route_map_round_trips_its_turnaround_index`
Expected: FAIL to compile -- `RouteMap` has no field `turnaround_index`.

- [ ] **Step 3: Write the migration**

`src-tauri/core/migrations/2026-09-09-100000_add_trip_route_turnaround_index/up.sql`:

```sql
-- Task 78: a round trip is now two routed legs, and the saved row is the two
-- of them joined. This column is the index, in `waypoints`, of the point
-- where the outbound leg ends and the return leg begins.
--
-- NULL is the correct backfill and needs no data migration. Every round trip
-- saved before today was closed by appending exactly one clone of the first
-- waypoint, so `[A, ...vias, B, A]` splits at `len - 2` with no ambiguity.
-- NULL means "use that rule"; a value means "split here".
ALTER TABLE trip_routes ADD COLUMN turnaround_index INTEGER DEFAULT NULL;
```

`src-tauri/core/migrations/2026-09-09-100000_add_trip_route_turnaround_index/down.sql`:

```sql
-- Forward-only in practice (ADR-012); no diesel CLI revert runs in this repo.
ALTER TABLE trip_routes DROP COLUMN turnaround_index;
```

- [ ] **Step 4: Declare the column LAST in the schema**

In `src-tauri/core/src/schema.rs`, inside `diesel::table! { trip_routes (trip_id) { ... } }`, after `round_trip`:

```rust
        // Added via migration 2026-09-09-100000_add_trip_route_turnaround_index
        // (Task 78). Appended LAST for the third time, for the same reason as
        // `mode` and `round_trip` above: RouteMapRow is Queryable and binds
        // POSITIONALLY. Any future column goes after this one, never between
        // existing ones.
        turnaround_index -> Nullable<Integer>,
```

- [ ] **Step 5: Add the field to the models, LAST in both row structs**

In `src-tauri/core/src/models.rs`, add to `RouteMap` after `round_trip`:

```rust
    /// Round trips only: the index in `waypoints` where the outbound leg ends
    /// and the return leg begins. `None` for a one-way route, for a loop, and
    /// for a round trip saved before Task 78 -- those were always closed by
    /// appending one clone of the first waypoint, so they split at `len - 2`.
    pub turnaround_index: Option<i32>,
```

Add to `RouteMapRow` after `round_trip`:

```rust
    /// Appended LAST again, after `round_trip` (Task 78) -- same positional-
    /// bind hazard, same rule: any future column goes after this one.
    pub turnaround_index: Option<i32>,
```

Add to `NewRouteMapRow` after `round_trip`:

```rust
    pub turnaround_index: Option<i32>,
```

Add to `impl From<RouteMapRow> for RouteMap`, after `round_trip: row.round_trip,`:

```rust
            turnaround_index: row.turnaround_index,
```

- [ ] **Step 6: Bind the column in `save_route_map`**

In `src-tauri/core/src/db.rs`, inside the `NewRouteMapRow { ... }` literal in `save_route_map`, after `round_trip: map.round_trip,`:

```rust
                    turnaround_index: map.turnaround_index,
```

- [ ] **Step 7: Fix every other construction site**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
Expected: compile errors listing every place that builds a `RouteMap` literal. Add `turnaround_index: None` to each. `save_trip_route_internal` in `route_maps.rs` is one of them; the rest are in `route_maps_tests.rs`.

- [ ] **Step 8: Run the tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
Expected: PASS, including the two new tests.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/core/migrations/2026-09-09-100000_add_trip_route_turnaround_index \
        src-tauri/core/src/schema.rs \
        src-tauri/core/src/models.rs \
        src-tauri/core/src/db.rs \
        src-tauri/core/src/db_tests.rs \
        src-tauri/core/src/commands_internal/route_maps.rs \
        src-tauri/core/src/commands_internal/route_maps_tests.rs
git commit -m "feat(routes): persist where a saved round trip turns around"
```

---

### Task 2: Route a round trip as two legs

**Files:**
- Modify: `src-tauri/core/src/commands_internal/route_maps.rs`
- Modify: `src-tauri/core/src/commands_internal/mod.rs` (re-export the new items the dispatcher needs)
- Modify: `src-tauri/core/src/server/dispatcher_async.rs`
- Test: `src-tauri/core/src/commands_internal/route_maps_tests.rs`

**Interfaces:**
- Consumes: `RouteProvider::fetch_alternatives`, `insert_waypoint`, `deviation`, `decode_coordinates`, `MAX_ALTERNATIVES` -- all already in `route_maps.rs`.
- Produces:
  - `pub enum Leg { Outbound, Inbound }` (serde `camelCase`, so the wire values are `"outbound"` and `"inbound"`)
  - `pub struct LegInsertPoint { lat: f64, lon: f64, polyline: String, leg: Leg }`
  - `pub struct LegRoute { polyline: String, coordinates: Vec<[f64; 2]>, road_km: f64, duration_s: f64 }`
  - `pub struct CombinedLeg { road_km: f64, duration_s: f64, deviation_percent: f64, off_target: bool }`
  - `pub struct RoundTripRoutes { outbound_waypoints: Vec<Waypoint>, inbound_waypoints: Vec<Waypoint>, outbound: Vec<LegRoute>, inbound: Vec<LegRoute>, combined: Vec<Vec<CombinedLeg>>, target_km: f64 }`
  - `pub async fn route_round_trip_internal(provider: &dyn RouteProvider, outbound: Vec<Waypoint>, inbound: Vec<Waypoint>, target_km: f64, insert: Option<LegInsertPoint>) -> Result<RoundTripRoutes, String>`
  - RPC command `route_round_trip`

- [ ] **Step 1: Write the failing tests**

Append to `src-tauri/core/src/commands_internal/route_maps_tests.rs`. `MultiRouteProvider` and `fetched` already exist in that file; the new provider records what it was asked for.

```rust
// ---------------------------------------------------------------------------
// Round trip as two legs (Task 78)
// ---------------------------------------------------------------------------

/// Answers each call with its own routes and records the coordinate lists it
/// was asked for, so a test can prove there were TWO requests and see the
/// points of each.
struct TwoLegProvider {
    outbound: Vec<FetchedRoute>,
    inbound: Vec<FetchedRoute>,
    calls: std::sync::Mutex<Vec<Vec<(f64, f64)>>>,
}

impl TwoLegProvider {
    fn new(outbound: Vec<FetchedRoute>, inbound: Vec<FetchedRoute>) -> Self {
        Self { outbound, inbound, calls: std::sync::Mutex::new(Vec::new()) }
    }
}

#[async_trait::async_trait]
impl RouteProvider for TwoLegProvider {
    async fn fetch(&self, _coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        Ok(self.outbound[0].clone())
    }
    async fn fetch_alternatives(
        &self,
        coords: &[(f64, f64)],
        _max: usize,
    ) -> Result<Vec<FetchedRoute>, String> {
        let mut calls = self.calls.lock().unwrap();
        calls.push(coords.to_vec());
        if calls.len() == 1 {
            Ok(self.outbound.clone())
        } else {
            Ok(self.inbound.clone())
        }
    }
}

#[tokio::test]
async fn a_round_trip_is_two_requests_one_per_leg() {
    let provider = TwoLegProvider::new(
        vec![fetched("out1", 25.0, 1500.0), fetched("out2", 27.0, 1400.0)],
        vec![fetched("back1", 26.0, 1600.0)],
    );

    let result = route_round_trip_internal(
        &provider,
        direct_waypoints(),
        Vec::new(),
        50.0,
        None,
    )
    .await
    .unwrap();

    let calls = provider.calls.lock().unwrap();
    assert_eq!(calls.len(), 2, "each leg must be its own routing request");
    assert_eq!(calls[0].len(), 2, "the outbound leg is a two-point request");
    assert_eq!(calls[1].len(), 2, "the return leg is a two-point request");
    // Reversed: the return leg starts where the outbound one ended.
    assert_eq!(calls[1][0], calls[0][1]);
    assert_eq!(calls[1][1], calls[0][0]);

    assert_eq!(result.outbound.len(), 2, "the outbound leg keeps its alternatives");
    assert_eq!(result.inbound.len(), 1);
}

#[tokio::test]
async fn the_return_leg_is_derived_when_the_caller_sends_none() {
    let provider = TwoLegProvider::new(
        vec![fetched("out1", 25.0, 1500.0)],
        vec![fetched("back1", 26.0, 1600.0)],
    );

    let result =
        route_round_trip_internal(&provider, direct_waypoints(), Vec::new(), 50.0, None)
            .await
            .unwrap();

    let out = &result.outbound_waypoints;
    let back = &result.inbound_waypoints;
    assert_eq!(back.len(), 2);
    assert_eq!(back[0].lat, out[out.len() - 1].lat);
    assert_eq!(back[0].lon, out[out.len() - 1].lon);
    assert_eq!(back[1].lat, out[0].lat);
    assert_eq!(back[1].lon, out[0].lon);
}

#[tokio::test]
async fn the_two_legs_are_always_joined_even_when_the_caller_sends_them_apart() {
    // The user dragged the outbound leg's destination handle. The return leg
    // the browser still holds starts at the OLD point. The backend must not
    // route a pair that does not join.
    let provider = TwoLegProvider::new(
        vec![fetched("out1", 25.0, 1500.0)],
        vec![fetched("back1", 26.0, 1600.0)],
    );

    let stale_inbound = vec![
        Waypoint { lat: 1.0, lon: 1.0, name: None, node_idx: None },
        Waypoint { lat: 2.0, lon: 2.0, name: None, node_idx: None },
    ];

    let result = route_round_trip_internal(
        &provider,
        direct_waypoints(),
        stale_inbound,
        50.0,
        None,
    )
    .await
    .unwrap();

    let out = &result.outbound_waypoints;
    let back = &result.inbound_waypoints;
    assert_eq!(back[0].lat, out[out.len() - 1].lat);
    assert_eq!(back[back.len() - 1].lat, out[0].lat);
}

#[tokio::test]
async fn a_via_dropped_on_the_return_leg_stays_on_the_return_leg() {
    // The bug this task exists to remove: with one three-point request, a via
    // dragged onto the way home landed on the way out, because the search ran
    // against the open outbound list while the polyline was the closed line.
    let leg_points = vec![(48.9444, 20.5675), (48.55, 18.85), (48.1486, 17.1077)];
    let leg_polyline = encode(&leg_points);

    let provider = TwoLegProvider::new(
        vec![fetched("out1", 25.0, 1500.0)],
        vec![fetched(&leg_polyline, 26.0, 1600.0)],
    );

    let inbound = vec![
        Waypoint { lat: 48.9444, lon: 20.5675, name: Some("Spišská Nová Ves".into()), node_idx: None },
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
    ];

    let result = route_round_trip_internal(
        &provider,
        direct_waypoints(),
        inbound,
        50.0,
        Some(LegInsertPoint {
            lat: 48.55,
            lon: 18.85,
            polyline: leg_polyline.clone(),
            leg: Leg::Inbound,
        }),
    )
    .await
    .unwrap();

    assert_eq!(
        result.outbound_waypoints.len(),
        2,
        "the outbound leg must be untouched by a drag on the return leg"
    );
    assert_eq!(result.inbound_waypoints.len(), 3, "the via belongs to the return leg");
    assert!(result.inbound_waypoints[1].name.is_none(), "a dragged point is unnamed");
}

#[tokio::test]
async fn the_deviation_of_a_pair_is_measured_against_the_two_legs_together() {
    let provider = TwoLegProvider::new(
        vec![fetched("out1", 25.0, 1500.0), fetched("out2", 30.0, 1400.0)],
        vec![fetched("back1", 25.0, 1600.0)],
    );

    let result =
        route_round_trip_internal(&provider, direct_waypoints(), Vec::new(), 50.0, None)
            .await
            .unwrap();

    assert_eq!(result.combined.len(), 2);
    assert_eq!(result.combined[0].len(), 1);
    // 25 + 25 against a 50 km target is exact.
    assert!((result.combined[0][0].road_km - 50.0).abs() < 1e-9);
    assert!((result.combined[0][0].deviation_percent).abs() < 1e-9);
    assert!(!result.combined[0][0].off_target);
    // 30 + 25 is 55 km, ten percent long.
    assert!((result.combined[1][0].road_km - 55.0).abs() < 1e-9);
    assert!((result.combined[1][0].deviation_percent - 10.0).abs() < 1e-6);
    // Durations add too -- the panel shows the time of the whole journey.
    assert!((result.combined[0][0].duration_s - 3100.0).abs() < 1e-9);
}

#[tokio::test]
async fn a_leg_keeps_the_routing_services_own_order() {
    // ADR-038: fastest first, never re-sorted, even when a slower alternative
    // is closer to the target distance.
    let provider = TwoLegProvider::new(
        vec![fetched("fast", 40.0, 1000.0), fetched("slow", 25.0, 2000.0)],
        vec![fetched("back1", 25.0, 1600.0)],
    );

    let result =
        route_round_trip_internal(&provider, direct_waypoints(), Vec::new(), 50.0, None)
            .await
            .unwrap();

    assert_eq!(result.outbound[0].polyline, "fast");
    assert_eq!(result.outbound[1].polyline, "slow");
}

#[tokio::test]
async fn a_loop_row_cannot_be_routed_as_a_round_trip() {
    // A row whose two endpoints are one place is a Loop (`mode_for`), and a
    // loop has no second leg. The same first/last comparison ADR-041 uses:
    // coordinate AND name.
    let provider = TwoLegProvider::new(
        vec![fetched("out1", 25.0, 1500.0)],
        vec![fetched("back1", 26.0, 1600.0)],
    );

    let same_place = vec![
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
    ];

    let err = route_round_trip_internal(&provider, same_place, Vec::new(), 50.0, None)
        .await
        .unwrap_err();
    assert!(err.contains("two different endpoints"), "got: {err}");
}

#[tokio::test]
async fn a_round_trip_needs_a_start_and_an_end() {
    let provider = TwoLegProvider::new(
        vec![fetched("out1", 25.0, 1500.0)],
        vec![fetched("back1", 26.0, 1600.0)],
    );

    let one = vec![Waypoint { lat: 48.1, lon: 17.1, name: None, node_idx: None }];
    let err = route_round_trip_internal(&provider, one, Vec::new(), 50.0, None)
        .await
        .unwrap_err();
    assert!(err.contains("start and an end"), "got: {err}");
}
```

- [ ] **Step 2: Run the tests and watch them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core route_round_trip`
Expected: FAIL to compile -- `route_round_trip_internal`, `LegInsertPoint` and `Leg` do not exist.

- [ ] **Step 3: Write the types and the function**

In `src-tauri/core/src/commands_internal/route_maps.rs`, extend the `crate::route_map` import to bring in `FetchedRoute`:

```rust
use crate::route_map::{generate_route_random, Dataset, FetchedRoute, RouteProvider, TOLERANCE};
```

Then append, after `route_direct_internal`:

```rust
// ---------------------------------------------------------------------------
// Round trip as two legs (Task 78)
// ---------------------------------------------------------------------------

/// Which leg of a round trip a dragged-in point belongs to.
///
/// Reported by the caller, never re-derived here from geometry. The browser
/// knows it for certain -- the ghost handle is attached to one leg's own
/// polyline -- and the derivation is exactly what Task 72 got wrong: a via
/// dropped on the way home was searched for in the open outbound list against
/// the closed polyline, so it landed on the way out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Leg {
    Outbound,
    Inbound,
}

/// A point the user dragged off ONE leg's polyline.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegInsertPoint {
    pub lat: f64,
    pub lon: f64,
    /// That leg's own geometry -- the ordering that decides where in that
    /// leg's waypoint list the point belongs.
    pub polyline: String,
    pub leg: Leg,
}

/// One alternative for one leg. Carries no deviation of its own: a leg is
/// half a journey, and the target distance describes the whole one.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegRoute {
    pub polyline: String,
    /// Decoded `[lat, lon]` pairs, ready for L.polyline.
    pub coordinates: Vec<[f64; 2]>,
    pub road_km: f64,
    pub duration_s: f64,
}

impl LegRoute {
    fn from_fetched(route: &FetchedRoute) -> Self {
        Self {
            coordinates: decode_coordinates(&route.polyline),
            polyline: route.polyline.clone(),
            road_km: route.road_km,
            duration_s: route.duration_s,
        }
    }
}

/// What one pair of legs adds up to. Precomputed for every pair the two
/// requests could produce (at most 3 x 3), so selecting an alternative is an
/// index change in the browser and not a calculation (ADR-008).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CombinedLeg {
    pub road_km: f64,
    pub duration_s: f64,
    /// Signed percentage by which the PAIR misses the target -- the whole
    /// journey is what the trip records, so the whole journey is what the
    /// deviation measures.
    pub deviation_percent: f64,
    pub off_target: bool,
}

/// Both legs of a round trip, and the table of what each pair adds up to.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundTripRoutes {
    /// The normalised OPEN list for each leg. AUTHORITATIVE: the caller adopts
    /// these rather than keeping its own, exactly as `route_direct_internal`'s
    /// `waypoints` are adopted.
    pub outbound_waypoints: Vec<Waypoint>,
    pub inbound_waypoints: Vec<Waypoint>,
    pub outbound: Vec<LegRoute>,
    pub inbound: Vec<LegRoute>,
    /// `combined[i][j]` for outbound alternative `i` and return alternative `j`.
    pub combined: Vec<Vec<CombinedLeg>>,
    pub target_km: f64,
}

/// Route a round trip as TWO requests, one per leg.
///
/// This is the whole point of the task. OSRM offers alternatives only for a
/// two-point request, so the single `[A, B, A]` call this replaces could never
/// have offered a choice, and the way home was whatever the through-route
/// produced. Two requests give each leg its own alternatives and let the
/// return take a different road.
///
/// Persists NOTHING. The caller confirms with
/// `save_trip_round_trip_route_internal`.
///
/// The returned waypoint lists are authoritative in the same sense as
/// `route_direct_internal`'s (ADR-041): the caller's own shapes are never
/// trusted. The return leg is derived when it is absent, and its two ends are
/// overwritten from the outbound leg when it is present, so the pair always
/// joins -- dragging the outbound leg's destination handle moves the return
/// leg's start with it, and no caller can hand in a broken pair.
pub async fn route_round_trip_internal(
    provider: &dyn RouteProvider,
    outbound: Vec<Waypoint>,
    inbound: Vec<Waypoint>,
    target_km: f64,
    insert: Option<LegInsertPoint>,
) -> Result<RoundTripRoutes, String> {
    if outbound.len() < 2 {
        return Err(format!(
            "A route needs a start and an end, got {} point(s).",
            outbound.len()
        ));
    }

    // The same first/last comparison ADR-041 uses -- coordinate AND name. A
    // row naming one place twice is a Loop (`mode_for`), and a loop is already
    // closed: it has no second leg to route.
    let first = &outbound[0];
    let last = &outbound[outbound.len() - 1];
    if first.lat == last.lat && first.lon == last.lon && first.name == last.name {
        return Err("A round trip needs two different endpoints.".to_string());
    }

    let mut outbound = outbound;
    let mut inbound = inbound;

    // Insert into the named leg only, against that leg's own polyline, and
    // BEFORE the ends are re-joined below. `insert_waypoint` never returns a
    // list with a new first or last element, so a drag can never move where a
    // leg began or ended.
    if let Some(point) = insert {
        match point.leg {
            Leg::Outbound => {
                outbound = insert_waypoint(&outbound, &point.polyline, point.lat, point.lon);
            }
            Leg::Inbound => {
                if inbound.len() >= 2 {
                    inbound = insert_waypoint(&inbound, &point.polyline, point.lat, point.lon);
                }
                // A drag on a return leg the caller did not send is not a real
                // scenario -- the leg has to be on screen to be dragged -- and
                // inserting into a list that is about to be replaced wholesale
                // would only invent a via nobody placed.
            }
        }
    }

    if inbound.len() < 2 {
        inbound = vec![outbound[outbound.len() - 1].clone(), outbound[0].clone()];
    } else {
        let last_index = inbound.len() - 1;
        inbound[0] = outbound[outbound.len() - 1].clone();
        inbound[last_index] = outbound[0].clone();
    }

    let out_coords: Vec<(f64, f64)> = outbound.iter().map(|w| (w.lat, w.lon)).collect();
    let in_coords: Vec<(f64, f64)> = inbound.iter().map(|w| (w.lat, w.lon)).collect();

    let out_routes = provider
        .fetch_alternatives(&out_coords, MAX_ALTERNATIVES)
        .await?;
    let in_routes = provider
        .fetch_alternatives(&in_coords, MAX_ALTERNATIVES)
        .await?;

    let combined = out_routes
        .iter()
        .map(|out| {
            in_routes
                .iter()
                .map(|back| {
                    let road_km = out.road_km + back.road_km;
                    // The same `deviation` helper loop mode and one-way mode
                    // use. A second, separately measured notion of "close
                    // enough" is exactly what ADR-008 rules out.
                    let (deviation_percent, off_target) = deviation(target_km, road_km);
                    CombinedLeg {
                        road_km,
                        duration_s: out.duration_s + back.duration_s,
                        deviation_percent,
                        off_target,
                    }
                })
                .collect()
        })
        .collect();

    Ok(RoundTripRoutes {
        outbound_waypoints: outbound,
        inbound_waypoints: inbound,
        outbound: out_routes.iter().map(LegRoute::from_fetched).collect(),
        inbound: in_routes.iter().map(LegRoute::from_fetched).collect(),
        combined,
        target_km,
    })
}
```

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core route_round_trip`
Expected: PASS, nine tests.

- [ ] **Step 5: Register the RPC command**

`route_round_trip` awaits OSRM, so it belongs in `dispatcher_async.rs`. Add this arm immediately after the `"route_direct"` arm:

```rust
        "route_round_trip" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                outbound: Vec<crate::models::Waypoint>,
                // Absent on the first request after the checkbox is ticked --
                // the backend derives the return leg from the outbound one.
                #[serde(default)]
                inbound: Vec<crate::models::Waypoint>,
                target_km: f64,
                #[serde(default)]
                insert: Option<crate::commands_internal::LegInsertPoint>,
            }
            let a: Args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            let provider = crate::route_map::HttpRouteProvider::public();
            let result = crate::commands_internal::route_round_trip_internal(
                &provider,
                a.outbound,
                a.inbound,
                a.target_km,
                a.insert,
            )
            .await;
            Some(result.map(|v| serde_json::to_value(v).unwrap()))
        }
```

If `commands_internal/mod.rs` re-exports `route_maps` items by name rather than with a glob, add `Leg`, `LegInsertPoint`, `LegRoute`, `CombinedLeg`, `RoundTripRoutes` and `route_round_trip_internal` to that list.

- [ ] **Step 6: Write the dispatcher test**

Append to the test module in `src-tauri/core/src/server/dispatcher_async.rs`, beside `route_direct_is_an_async_command_taking_waypoints`:

```rust
    /// route_round_trip awaits OSRM twice, so it must be routed here and not
    /// by dispatch_sync. Like the route_direct tests beside it, this asserts
    /// on the ARGUMENT parsing only -- it leans on the function's own
    /// endpoint guard to fail before any network call is made.
    #[tokio::test]
    async fn route_round_trip_is_an_async_command_and_inbound_is_optional() {
        let state = test_state();
        let result = dispatch_async(
            "route_round_trip",
            json!({
                "outbound": [{ "lat": 48.1, "lon": 17.1 }],
                "targetKm": 50.0
            }),
            &state,
        )
        .await
        .expect("route_round_trip must be handled here, not by dispatch_sync");

        let err = result.unwrap_err();
        assert!(
            err.contains("start and an end"),
            "a one-point outbound leg must reach the function's own guard, not \
             fail on a missing `inbound` field: {err}"
        );
    }
```

Copy the `test_state()` helper name from the neighbouring `route_direct` tests -- use whatever those already call.

- [ ] **Step 7: Run the whole backend suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
Expected: PASS. In particular, every `route_direct` and round-trip normalisation test from ADR-041 still passes -- this task adds a command beside that one, it does not change it.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/core/src/commands_internal/route_maps.rs \
        src-tauri/core/src/commands_internal/route_maps_tests.rs \
        src-tauri/core/src/commands_internal/mod.rs \
        src-tauri/core/src/server/dispatcher_async.rs
git commit -m "feat(routes): route each leg of a round trip on its own"
```

---

### Task 3: Save the chosen pair, and read the target from the trip

**Files:**
- Modify: `src-tauri/core/src/commands_internal/route_maps.rs`
- Modify: `src-tauri/core/src/server/dispatcher.rs`
- Test: `src-tauri/core/src/commands_internal/route_maps_tests.rs`

**Interfaces:**
- Consumes: `Leg`, `RoundTripRoutes` (Task 2), `RouteMap.turnaround_index` (Task 1).
- Produces:
  - `pub fn save_trip_round_trip_route_internal(db, app_state, trip_id: String, outbound_waypoints: Vec<Waypoint>, inbound_waypoints: Vec<Waypoint>, outbound_polyline: String, inbound_polyline: String, outbound_road_km: f64, inbound_road_km: f64, target_km: f64) -> Result<(), String>`
  - `SavedRouteMap.turnaround_index: Option<i32>`, **always `Some(_)` for a round trip** -- the legacy fallback is resolved here, not in the browser
  - RPC command `save_trip_round_trip_route`
  - `get_trip_route_internal` now reports `target_km` from the trip.

- [ ] **Step 1: Write the failing tests**

Append to `src-tauri/core/src/commands_internal/route_maps_tests.rs`:

```rust
#[test]
fn saving_a_round_trip_joins_the_two_legs_into_one_row() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);

    let out_points = vec![(48.1486, 17.1077), (48.5, 18.0), (48.9444, 20.5675)];
    let back_points = vec![(48.9444, 20.5675), (48.6, 18.4), (48.1486, 17.1077)];

    let outbound_waypoints = vec![
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
        Waypoint { lat: 48.9444, lon: 20.5675, name: Some("Spišská Nová Ves".into()), node_idx: None },
    ];
    let inbound_waypoints = vec![
        Waypoint { lat: 48.9444, lon: 20.5675, name: Some("Spišská Nová Ves".into()), node_idx: None },
        Waypoint { lat: 48.6, lon: 18.4, name: None, node_idx: None },
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
    ];

    save_trip_round_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        outbound_waypoints,
        inbound_waypoints,
        encode(&out_points),
        encode(&back_points),
        25.0,
        27.0,
        50.0,
    )
    .unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();

    // The shared turnaround point is stored once, not twice.
    assert_eq!(saved.waypoints.len(), 4);
    assert_eq!(saved.turnaround_index, Some(1));
    assert_eq!(saved.waypoints[1].name.as_deref(), Some("Spišská Nová Ves"));
    assert!(saved.round_trip);
    assert_eq!(saved.mode, RouteMode::Direct);

    // The distance is the sum of the two legs -- the backend adds it, not the
    // browser.
    assert!((saved.road_km - 52.0).abs() < 1e-9);

    // The geometry is the two legs, in order, as one line.
    assert_eq!(saved.coordinates.len(), out_points.len() + back_points.len());
    assert!((saved.coordinates[0][0] - 48.1486).abs() < 1e-4);
    assert!((saved.coordinates[3][0] - 48.9444).abs() < 1e-4);
}

#[test]
fn a_round_trip_save_refuses_a_leg_that_is_not_a_leg() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);

    let one = vec![Waypoint { lat: 48.1, lon: 17.1, name: None, node_idx: None }];
    let two = direct_waypoints();

    let err = save_trip_round_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        one,
        two,
        "a".into(),
        "b".into(),
        1.0,
        1.0,
        2.0,
    )
    .unwrap_err();
    assert!(err.contains("two legs"), "got: {err}");
}

#[test]
fn a_round_trip_saved_before_the_index_existed_resolves_its_own_split_point() {
    // The legacy shape, written the way the old code wrote it: one clone of
    // the first waypoint appended to close the route, and no stored index.
    // The rule that recovers the split -- the outbound leg ended at `len - 2`
    // -- is a rule about how this application wrote its own data, so it is
    // resolved in Rust and never in the browser (ADR-008).
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();

    let mut closed = direct_waypoints();
    let via = Waypoint { lat: 48.5, lon: 18.0, name: None, node_idx: None };
    closed.insert(1, via);
    closed.push(closed[0].clone());

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        closed,
        polyline,
        trip.distance_km,
        120.0,
        RouteMode::Direct,
        true,
    )
    .unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();
    assert_eq!(saved.waypoints.len(), 4, "[A, via, B, A]");
    assert_eq!(
        saved.turnaround_index,
        Some(2),
        "the outbound leg of a legacy round trip ended at len - 2"
    );
}

#[test]
fn a_one_way_saved_map_resolves_no_split_point() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        sample_waypoints(),
        polyline,
        trip.distance_km,
        120.0,
        RouteMode::Direct,
        false,
    )
    .unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();
    assert_eq!(saved.turnaround_index, None);
}

#[test]
fn a_saved_map_reports_the_trips_distance_as_its_target() {
    // trip_routes.target_km records what the trip measured when the map was
    // saved. After a write-back (or any ordinary edit of the row) the trip's
    // distance moves, and a map still reporting the old target would show a
    // deviation against a distance the book no longer holds.
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let mut trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        sample_waypoints(),
        polyline,
        trip.distance_km,
        118.0,
        RouteMode::Direct,
        false,
    )
    .unwrap();

    trip.distance_km = 118.0;
    db.update_trip(&trip).unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();
    assert!((saved.target_km - 118.0).abs() < 1e-9);
    assert!(
        saved.deviation_percent.abs() < 1e-9,
        "a route whose distance now matches the trip has no deviation left"
    );
    assert!(!saved.off_target);
}
```

- [ ] **Step 2: Run the tests and watch them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core round_trip_save`
Expected: FAIL to compile -- `save_trip_round_trip_route_internal` does not exist.

- [ ] **Step 3: Extract the one writer, and add the round-trip save**

In `src-tauri/core/src/commands_internal/route_maps.rs`, add the polyline codec import:

```rust
use crate::route_map::polyline::{decode, encode};
```

(The file already imports `decode`; extend that line rather than adding a second.)

Replace the body of `save_trip_route_internal` with a delegation, and add the shared writer above it:

```rust
/// The one place a `trip_routes` row is built. Both save paths funnel through
/// it, so they cannot disagree about what the backend stamps and what it takes
/// from the caller.
///
/// `dataset_version` and `created_at` are stamped here rather than accepted:
/// they describe what the backend used and when it stored it, so a client
/// cannot misreport either. `round_trip` gets the same treatment -- a loop is
/// already closed, so it is forced to `false` for `RouteMode::Loop`.
#[allow(clippy::too_many_arguments)]
fn persist_route_map(
    db: &Database,
    app_state: &AppState,
    trip_id: String,
    waypoints: Vec<Waypoint>,
    polyline: String,
    target_km: f64,
    road_km: f64,
    mode: RouteMode,
    round_trip: bool,
    turnaround_index: Option<i32>,
) -> Result<(), String> {
    check_read_only!(app_state);
    let trip_uuid = Uuid::parse_str(&trip_id).map_err(|e| format!("Invalid trip id: {e}"))?;

    let round_trip = match mode {
        RouteMode::Loop => false,
        RouteMode::Direct => round_trip,
    };

    let map = RouteMap {
        trip_id: trip_uuid,
        waypoints,
        polyline,
        target_km,
        road_km,
        mode,
        dataset_version: match mode {
            // Only a loop actually used the bundled node set.
            RouteMode::Loop => Some(Dataset::bundled().version),
            RouteMode::Direct => None,
        },
        created_at: Utc::now(),
        round_trip,
        // A split point is meaningless without a return leg, and NULL rather
        // than 0 so a one-way route can never be split at its own origin.
        turnaround_index: if round_trip { turnaround_index } else { None },
    };

    db.save_route_map(&map).map_err(|e| e.to_string())
}

/// Save (or replace) the map for a trip. One-way and loop routes.
#[allow(clippy::too_many_arguments)]
pub fn save_trip_route_internal(
    db: &Database,
    app_state: &AppState,
    trip_id: String,
    waypoints: Vec<Waypoint>,
    polyline: String,
    target_km: f64,
    road_km: f64,
    mode: RouteMode,
    round_trip: bool,
) -> Result<(), String> {
    persist_route_map(
        db, app_state, trip_id, waypoints, polyline, target_km, road_km, mode, round_trip, None,
    )
}

/// Save the chosen pair of legs as one route.
///
/// Everything the row stores is assembled HERE, from values the routing
/// response itself produced: the waypoint list is the two legs joined at their
/// shared turnaround point, the geometry is the two polylines concatenated,
/// and the road distance is their sum. The browser assembles none of it
/// (ADR-008) -- it only says which alternative it picked.
#[allow(clippy::too_many_arguments)]
pub fn save_trip_round_trip_route_internal(
    db: &Database,
    app_state: &AppState,
    trip_id: String,
    outbound_waypoints: Vec<Waypoint>,
    inbound_waypoints: Vec<Waypoint>,
    outbound_polyline: String,
    inbound_polyline: String,
    outbound_road_km: f64,
    inbound_road_km: f64,
    target_km: f64,
) -> Result<(), String> {
    if outbound_waypoints.len() < 2 || inbound_waypoints.len() < 2 {
        return Err("A round trip needs two legs of at least two points each.".to_string());
    }

    let turnaround_index = i32::try_from(outbound_waypoints.len() - 1)
        .map_err(|_| "The outbound leg has too many points".to_string())?;

    // The legs share their turnaround point, so the return leg's first
    // waypoint is dropped rather than stored twice. `turnaround_index` above
    // is computed BEFORE the join, from the outbound leg's own length.
    let mut waypoints = outbound_waypoints;
    waypoints.extend(inbound_waypoints.into_iter().skip(1));

    let mut points = decode(&outbound_polyline);
    points.extend(decode(&inbound_polyline));

    persist_route_map(
        db,
        app_state,
        trip_id,
        waypoints,
        encode(&points),
        target_km,
        outbound_road_km + inbound_road_km,
        RouteMode::Direct,
        true,
        Some(turnaround_index),
    )
}
```

- [ ] **Step 4: Carry the index out to the frontend, and read the target from the trip**

Add to `SavedRouteMap`, after `round_trip`:

```rust
    /// Round trips only: where the outbound leg ends in `waypoints`. `None`
    /// for a one-way route, a loop, and a round trip saved before Task 78 --
    /// see the note on `RouteMap::turnaround_index`.
    pub turnaround_index: Option<i32>,
```

Add to `impl From<RouteMap> for SavedRouteMap`, after `round_trip: map.round_trip,`:

```rust
            turnaround_index: map.turnaround_index,
```

Replace `get_trip_route_internal` with:

```rust
/// The saved map for a trip, with `target_km` taken from the trip as it stands
/// now, and the round trip's split point resolved.
///
/// `trip_routes.target_km` records what the trip measured when the map was
/// saved. A distance write-back -- or any ordinary edit of the row -- moves
/// `trips.distance_km` afterwards, and a map that kept the old number would
/// report a deviation against a distance the book no longer holds. The target
/// is a fact about the trip, so the trip is where it is read from. The stored
/// column stays as the fallback for a map whose trip has gone.
///
/// The legacy split point is resolved here too. A round trip saved before
/// Task 78 stores no index, and the rule that recovers it -- the old code
/// closed a route by appending exactly one clone of the first waypoint, so the
/// outbound leg ended at `len - 2` -- is a rule about how this application
/// wrote its own data. It belongs in Rust (ADR-008), and resolving it here
/// means every round trip that reaches the browser carries a real index, so
/// the browser only ever slices a list.
pub fn get_trip_route_internal(
    db: &Database,
    trip_id: String,
) -> Result<Option<SavedRouteMap>, String> {
    let Some(mut map) = db.get_route_map(&trip_id).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    if let Some(trip) = db.get_trip(&trip_id).map_err(|e| e.to_string())? {
        map.target_km = trip.distance_km;
    }
    if map.round_trip && map.turnaround_index.is_none() && map.waypoints.len() >= 3 {
        map.turnaround_index = i32::try_from(map.waypoints.len() - 2).ok();
    }
    Ok(Some(SavedRouteMap::from(map)))
}
```

- [ ] **Step 5: Run the tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core`
Expected: PASS.

- [ ] **Step 6: Register the RPC command**

`save_trip_round_trip_route` writes and awaits nothing, so it belongs in `dispatcher.rs`. Add this arm immediately after the `"save_trip_route"` arm:

```rust
        "save_trip_round_trip_route" => {
            // No `waypoints`, `polyline`, `roadKm` or `mode`: the backend
            // assembles all four from the two legs (ADR-008). Everything here
            // is a value the routing response itself produced.
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                trip_id: String,
                outbound_waypoints: Vec<crate::models::Waypoint>,
                inbound_waypoints: Vec<crate::models::Waypoint>,
                outbound_polyline: String,
                inbound_polyline: String,
                outbound_road_km: f64,
                inbound_road_km: f64,
                target_km: f64,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::save_trip_round_trip_route_internal(
                &state.db,
                &state.app_state,
                a.trip_id,
                a.outbound_waypoints,
                a.inbound_waypoints,
                a.outbound_polyline,
                a.inbound_polyline,
                a.outbound_road_km,
                a.inbound_road_km,
                a.target_km,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
```

- [ ] **Step 7: Run the whole backend suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/core/src/commands_internal/route_maps.rs \
        src-tauri/core/src/commands_internal/route_maps_tests.rs \
        src-tauri/core/src/server/dispatcher.rs
git commit -m "feat(routes): save a round trip as its two legs joined"
```

---

### Task 4: The map view routes, draws and saves two legs

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/api.ts`
- Modify: `src/routes/mapa/+page.svelte`
- Modify: `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`
- Test: covered by Task 8 (integration) and by the Rust tests of Tasks 2 and 3. This task has no test of its own: nothing in it is a calculation, and the browser cannot reach the routing service.

**Interfaces:**
- Consumes: `route_round_trip`, `save_trip_round_trip_route` (Tasks 2 and 3), `SavedRouteMap.turnaroundIndex`.
- Produces: `routeRoundTrip()`, `saveTripRoundTripRoute()` in `api.ts`; the `data-test` hooks Task 8 asserts on: `leg-outbound`, `leg-inbound`, `leg-alternative-btn`, `alternatives-unavailable-outbound`, `alternatives-unavailable-inbound`, `endpoint-missing`, `place-endpoint-btn`.

- [ ] **Step 1: Add the types**

In `src/lib/types.ts`, after `InsertPoint`:

```ts
/** Which leg of a round trip an edit belongs to. */
export type Leg = 'outbound' | 'inbound';

/** A point dragged off ONE leg's polyline. The leg is reported, not derived:
 *  the ghost handle is attached to a specific leg's line, so the browser knows
 *  it for certain, and deriving it from geometry is what put a via on the
 *  wrong leg before. */
export interface LegInsertPoint {
	lat: number;
	lon: number;
	polyline: string;
	leg: Leg;
}

/** One alternative for one leg. It carries no deviation: a leg is half a
 *  journey, and the target distance describes the whole one. */
export interface LegRoute {
	polyline: string;
	coordinates: [number, number][];
	roadKm: number;
	durationS: number;
}

/** What one pair of legs adds up to -- computed in Rust for every pair, so
 *  picking an alternative is an index change here and never a sum (ADR-008). */
export interface CombinedLeg {
	roadKm: number;
	durationS: number;
	deviationPercent: number;
	offTarget: boolean;
}

/** Both legs of a round trip, as the backend last normalised them. */
export interface RoundTripRoutes {
	/** The OPEN list for each leg. Authoritative -- adopt them, do not keep
	 *  your own: the backend derives the return leg and re-joins the ends. */
	outboundWaypoints: Waypoint[];
	inboundWaypoints: Waypoint[];
	outbound: LegRoute[];
	inbound: LegRoute[];
	/** combined[outboundIndex][inboundIndex]. */
	combined: CombinedLeg[][];
	targetKm: number;
}
```

Add to the `RouteMap` interface, after `roundTrip`:

```ts
	/** Round trips only: where the outbound leg ends in `waypoints`. null for
	 *  a one-way route, a loop, and a round trip saved before this column
	 *  existed -- those always closed by appending one clone of the first
	 *  waypoint, so they split at `length - 2`. */
	turnaroundIndex: number | null;
```

- [ ] **Step 2: Add the API calls**

In `src/lib/api.ts`, after `routeDirect`:

```ts
/**
 * Route a round trip as TWO requests, one per leg, so each leg gets its own
 * alternatives and the way home can differ from the way out.
 *
 * Send `inbound: []` on the first call after the checkbox is ticked -- the
 * backend derives the return leg from the outbound one. On every later call,
 * send the lists the previous response returned: they are authoritative, and
 * the backend re-joins their ends anyway.
 *
 * Alternatives come back in the routing service's own order (fastest first).
 * Never re-sort them (ADR-038).
 */
export async function routeRoundTrip(
	outbound: Waypoint[],
	inbound: Waypoint[],
	targetKm: number,
	insert?: LegInsertPoint
): Promise<RoundTripRoutes> {
	return await apiCall('route_round_trip', {
		outbound,
		inbound,
		targetKm,
		insert: insert ?? null
	});
}
```

After `saveTripRoute`:

```ts
/**
 * Persist the chosen pair of legs as one route.
 *
 * No waypoint list, no polyline, no combined distance: the backend joins the
 * legs, concatenates the geometry and sums the distances itself (ADR-008).
 * Every value sent here is one the routing response produced.
 */
export async function saveTripRoundTripRoute(
	tripId: string,
	outboundWaypoints: Waypoint[],
	inboundWaypoints: Waypoint[],
	outboundPolyline: string,
	inboundPolyline: string,
	outboundRoadKm: number,
	inboundRoadKm: number,
	targetKm: number
): Promise<void> {
	return await apiCall('save_trip_round_trip_route', {
		tripId,
		outboundWaypoints,
		inboundWaypoints,
		outboundPolyline,
		inboundPolyline,
		outboundRoadKm,
		inboundRoadKm,
		targetKm
	});
}
```

Add `LegInsertPoint`, `RoundTripRoutes` to the type import at the top of `api.ts`.

- [ ] **Step 3: Add the i18n keys**

In `src/lib/i18n/sk/index.ts`, inside `routeMap`, after `roundTripHint`:

```ts
		legOutbound: 'Tam',
		legInbound: 'Späť',
		endpointMissing: 'Miesto odchodu alebo príchodu nie je umiestnené na mape.',
		placeEndpoint: 'Umiestniť miesto',
```

In `src/lib/i18n/en/index.ts`, same place:

```ts
		legOutbound: 'There',
		legInbound: 'Back',
		endpointMissing: 'The origin or the destination has no place on the map yet.',
		placeEndpoint: 'Place it',
```

- [ ] **Step 4: Regenerate the i18n types**

Run: `npm run i18n && npm run check`
Expected: the generator writes `src/lib/i18n/i18n-types.ts`; `npm run check` reports no error about the new keys.

- [ ] **Step 5: Add the round-trip state to the map view**

In `src/routes/mapa/+page.svelte`, extend the type import:

```ts
	import type {
		GeneratedRoute,
		RouteMap,
		Trip,
		RouteMode,
		Waypoint,
		InsertPoint,
		Place,
		PlaceSource,
		Leg,
		LegInsertPoint,
		LegRoute,
		RoundTripRoutes
	} from '$lib/types';
```

and the api import:

```ts
	import {
		generateRoute,
		getTripRoute,
		saveTripRoute,
		saveTripRoundTripRoute,
		deleteTripRoute,
		getTrips,
		startRouteForTrip,
		routeDirect,
		routeRoundTrip,
		savePlace
	} from '$lib/api';
```

Add state, beside `baseWaypoints`:

```ts
	/** Round-trip mode only: both legs as the backend last normalised them,
	 *  with each leg's own alternatives and the table of what every pair adds
	 *  up to. Null in one-way and loop mode. */
	let roundTripRoutes = $state<RoundTripRoutes | null>(null);
	/** Which alternative is chosen on each leg. Independent -- the spec asks
	 *  for a choice per leg, not a choice of pairs. */
	let outboundIndex = $state(0);
	let inboundIndex = $state(0);
	/** The return leg's open waypoint list. `baseWaypoints` holds the outbound
	 *  one in round-trip mode, so the two fields stay a matched pair and
	 *  unticking the checkbox already has the right list to route. */
	let baseInbound = $state<Waypoint[] | null>(null);
	/** The saved route split back into its two legs, kept separately from
	 *  `baseWaypoints`/`baseInbound` so it can act as the FALLBACK when those
	 *  are null.
	 *
	 *  This is load-bearing, not a convenience. `runDirect`'s catch block nulls
	 *  `baseWaypoints` on a failed request, and without this the next
	 *  `currentWaypoints()` would fall through to `savedRoute.waypoints` --
	 *  the CLOSED list. Sent with `round_trip: false`, ADR-041's normaliser
	 *  strips one trailing point off it, which recovered the outbound leg
	 *  before this task and does not any more: a saved `[A, B, v, A]` would
	 *  become the one-way route `A -> B -> v`, moving the return leg's via
	 *  onto the way out. That is the very bug this task removes. */
	let savedLegs = $state<{ outbound: Waypoint[]; inbound: Waypoint[] } | null>(null);
```

Add derived values, replacing the existing `deviationPercent` / `deviationOffTarget` / `busy` block:

```ts
	/** The pair currently on screen, as the backend measured it. Null unless a
	 *  round trip has actually been routed. */
	let combinedSelection = $derived(
		roundTripRoutes?.combined?.[outboundIndex]?.[inboundIndex] ?? null
	);
	/** True when there is anything to show: a round trip has no
	 *  `GeneratedRoute`, so `displayRoute` alone would hide the whole panel. */
	let hasRoute = $derived(!!roundTripRoutes || !!displayRoute);
	let displayTargetKm = $derived(roundTripRoutes?.targetKm ?? displayRoute?.targetKm ?? null);
	let displayRoadKm = $derived(combinedSelection?.roadKm ?? displayRoute?.roadKm ?? null);
	// Both come from the backend. The tolerance is a business rule and has one
	// home in Rust (ADR-008); deriving it here would measure road distance
	// against a threshold the algorithm applies to a different quantity.
	let deviationPercent = $derived(
		combinedSelection?.deviationPercent ?? displayRoute?.deviationPercent ?? null
	);
	let deviationOffTarget = $derived(combinedSelection?.offTarget ?? displayRoute?.offTarget ?? false);
	/** Per leg, because the message is per leg now: it appears only where a
	 *  leg genuinely passes through an intermediate stop. A round trip on its
	 *  own no longer triggers it -- that was the point of splitting the
	 *  request. Read off the leg lists, so it is right on a cold load too. */
	let outboundHasVias = $derived((baseWaypoints?.length ?? 0) > 2);
	let inboundHasVias = $derived((baseInbound?.length ?? 0) > 2);
	/** Direct mode cannot route until the book holds a coordinate for both
	 *  ends. Dismissing the place dialog with Escape leaves them unresolved,
	 *  and without this the Recalculate button stayed enabled and its click
	 *  dead-ended in Rust's own "a route needs a start and an end". */
	let endpointsMissing = $derived(
		mode === 'direct' && (!resolvedOrigin || !resolvedDestination)
	);
	let busy = $derived(loading || generating || saving || removing);
```

Update `stopNames` so a round trip lists both legs:

```ts
	let stopNames = $derived(
		roundTripRoutes
			? [...roundTripRoutes.outboundWaypoints, ...roundTripRoutes.inboundWaypoints.slice(1)]
					.map((w) => w.name)
					.filter((name): name is string => !!name)
			: displayRoute
				? displayRoute.waypoints.map((w) => w.name).filter((name): name is string => !!name)
				: []
	);
```

- [ ] **Step 6: Route, and re-route, a round trip**

Add beside `runDirect`:

```ts
	/** Routes and displays a round trip as two legs. Persists nothing -- only
	 *  handleSave does. The returned waypoint lists are adopted wholesale: the
	 *  backend derives the return leg when none is sent and re-joins the two
	 *  ends on every call, so its lists are the only correct ones. */
	async function runRoundTrip(
		outbound: Waypoint[],
		inbound: Waypoint[],
		targetKm: number,
		insert?: LegInsertPoint
	) {
		generating = true;
		error = null;
		savedNotice = false;
		try {
			const routes = await routeRoundTrip(outbound, inbound, targetKm, insert);
			if (routes.outbound.length === 0 || routes.inbound.length === 0) {
				throw new Error('no routes returned');
			}
			roundTripRoutes = routes;
			outboundIndex = 0;
			inboundIndex = 0;
			baseWaypoints = routes.outboundWaypoints;
			baseInbound = routes.inboundWaypoints;
			// A round trip is not a GeneratedRoute. Leaving the one-way state
			// populated would show a stale proposal behind the leg panel and
			// let Save persist the wrong shape.
			generated = null;
			alternatives = [];
			activeIndex = 0;
		} catch (e) {
			console.error('Failed to route the round trip:', e);
			// Same rule as the other two modes: drop the proposal so an error
			// banner can never have a saveable route sitting behind it.
			roundTripRoutes = null;
			error = $LL.routeMap.routeError();
		} finally {
			generating = false;
		}
	}

	/** The outbound leg any re-route starts from. In round-trip mode
	 *  `baseWaypoints` holds the OPEN outbound list, so unticking the checkbox
	 *  already has the right list and needs no stripping. */
	function currentOutbound(): Waypoint[] {
		return baseWaypoints ?? savedLegs?.outbound ?? waypointsFromEndpoints();
	}

	/** The return leg, or an empty list: the backend derives it from the
	 *  outbound leg when it gets nothing. */
	function currentInbound(): Waypoint[] {
		return baseInbound ?? savedLegs?.inbound ?? [];
	}

	/** Re-route after an edit on one leg of a round trip. */
	async function rerouteLegs(outbound: Waypoint[], inbound: Waypoint[], insert?: LegInsertPoint) {
		if (!trip) return;
		// The map already shows the dragged position; re-fitting bounds would
		// re-zoom for a result the user is already looking at.
		skipFit = true;
		mode = 'direct';
		await runRoundTrip(outbound, inbound, trip.distanceKm, insert);
	}
```

Update `reroute` to drop any round trip -- an edit in one-way mode produces a one-way route:

```ts
	async function reroute(waypoints: Waypoint[], insert?: InsertPoint) {
		if (!trip) return;
		skipFit = true;
		mode = 'direct';
		roundTripRoutes = null;
		baseInbound = null;
		await runDirect(waypoints, trip.distanceKm, insert);
	}
```

Update `handleRegenerate` and `handleRetry` to branch on the checkbox:

```ts
	function handleRegenerate() {
		if (!trip) return;
		if (mode === 'loop') {
			void runGenerate(trip.distanceKm);
		} else if (mode === 'direct') {
			if (roundTrip) {
				void runRoundTrip(currentOutbound(), currentInbound(), trip.distanceKm);
			} else {
				roundTripRoutes = null;
				baseInbound = null;
				void runDirect(currentWaypoints(), trip.distanceKm);
			}
		}
	}
```

`handleRetry` gets the same `mode === 'direct'` branch -- copy the three lines above into it rather than calling `handleRegenerate`, so the existing `error = null` and the no-trip fallback stay as they are.

- [ ] **Step 7: Draw both legs**

Add the colours and the layer handles beside `inactiveLayers`:

```ts
	/** Outbound blue, return amber. "Choose per leg" is not usable if the two
	 *  legs are indistinguishable on the map. */
	const OUTBOUND_COLOR = '#0066cc';
	const INBOUND_COLOR = '#d97706';
	const INACTIVE_COLOR = '#94a3b8';
	/** The active line of each leg. Plain handles, like `routeLayer`. */
	let legLayers: Polyline[] = [];
	/** Which line the current ghost was created on. A ghost carries its leg in
	 *  its dragend closure, so moving the cursor from one leg's line to the
	 *  other's must REPLACE it, not move it -- otherwise a via dropped on the
	 *  return leg would be reported against the outbound one. */
	let ghostOwner: Polyline | null = null;
```

In the draw effect, clear `legLayers` alongside `inactiveLayers`:

```ts
		for (const layer of legLayers) {
			map.removeLayer(layer);
		}
		legLayers = [];
```

and clear `ghostOwner = null;` wherever `ghost = null;` is set (the draw effect's cleanup, the `mouseout` handler, and the `dragend` handler).

Replace the alternatives-and-active-line section of the effect with:

```ts
		if (roundTripRoutes) {
			drawLegLayers(roundTripRoutes.outbound, outboundIndex, OUTBOUND_COLOR, 'outbound');
			drawLegLayers(roundTripRoutes.inbound, inboundIndex, INBOUND_COLOR, 'inbound');
			if (!shouldSkipFit && legLayers.length > 0) {
				let bounds = legLayers[0].getBounds();
				for (const layer of legLayers.slice(1)) {
					bounds = bounds.extend(layer.getBounds());
				}
				map.fitBounds(bounds, { padding: [30, 30] });
			}
		} else {
			// Inactive alternatives sit UNDER the active line and are clickable.
			alts.forEach((route, i) => {
				if (i === active || route.coordinates.length === 0) return;
				const layer = leaflet!
					.polyline(route.coordinates, { color: INACTIVE_COLOR, weight: 4, opacity: 0.6 })
					.addTo(map!);
				layer.on('click', () => selectAlternative(i));
				inactiveLayers.push(layer);
			});

			if (route && route.coordinates.length > 0) {
				routeLayer = leaflet
					.polyline(route.coordinates, { color: OUTBOUND_COLOR, weight: 5, opacity: 0.85 })
					.addTo(map);
				attachGhost(routeLayer);
				if (!shouldSkipFit) {
					map.fitBounds(routeLayer.getBounds(), { padding: [30, 30] });
				}
			}
		}
```

The effect must also read the new state so it re-runs when a leg selection changes. Add at the top of the effect, beside the existing `const route = displayRoute;`:

```ts
		const legs = roundTripRoutes;
		const outIndex = outboundIndex;
		const inIndex = inboundIndex;
```

(These reads are what register the dependency; the branch above uses the state directly, which is fine because the effect already re-runs.)

Add the helper:

```ts
	/** One leg: its unchosen alternatives grey underneath, its chosen line on
	 *  top in `color`. The chosen line carries the ghost handle for that leg. */
	function drawLegLayers(routes: LegRoute[], active: number, color: string, leg: Leg) {
		if (!map || !leaflet) return;
		routes.forEach((route, i) => {
			if (i === active || route.coordinates.length === 0) return;
			const layer = leaflet!
				.polyline(route.coordinates, { color: INACTIVE_COLOR, weight: 4, opacity: 0.6 })
				.addTo(map!);
			layer.on('click', () => selectLeg(leg, i));
			inactiveLayers.push(layer);
		});
		const chosen = routes[active];
		if (!chosen || chosen.coordinates.length === 0) return;
		const layer = leaflet
			.polyline(chosen.coordinates, { color, weight: 5, opacity: 0.85 })
			.addTo(map);
		attachGhost(layer, leg);
		legLayers.push(layer);
	}

	/** Moves which alternative is active on ONE leg. Never reorders the list
	 *  -- that order is the routing service's and is the product decision
	 *  (ADR-038). */
	function selectLeg(leg: Leg, index: number) {
		if (leg === 'outbound') {
			outboundIndex = index;
		} else {
			inboundIndex = index;
		}
		skipFit = true;
	}
```

- [ ] **Step 8: Make the ghost and the handles leg-aware**

Change `attachGhost`'s signature to `function attachGhost(layer: Polyline, leg?: Leg)`, add the owner check as the first statement of its `mousemove` handler:

```ts
		layer.on('mousemove', (e: LeafletMouseEvent) => {
			// The cursor crossed from one leg's line to the other's. The
			// existing ghost's dragend closure still names the leg it was
			// created for, so it must be replaced, not moved -- otherwise a
			// via dropped on the return leg is reported against the outbound
			// one, which is the exact bug this task removes.
			if (ghost && ghostOwner !== layer) {
				map!.removeLayer(ghost);
				ghost = null;
				ghostOwner = null;
			}
			if (!ghost) {
				// ... unchanged creation, then: ...
				ghostOwner = layer;
			} else {
				ghost.setLatLng(e.latlng);
			}
		});
```

and replace the body of its `dragend` handler with:

```ts
				ghost.on('dragend', () => {
					dragging = false;
					const { lat, lng } = ghost!.getLatLng();
					map!.removeLayer(ghost!);
					ghost = null;
					ghostOwner = null;
					if (leg && roundTripRoutes) {
						const polyline =
							leg === 'outbound'
								? roundTripRoutes.outbound[outboundIndex].polyline
								: roundTripRoutes.inbound[inboundIndex].polyline;
						void rerouteLegs(currentOutbound(), currentInbound(), {
							lat,
							lon: lng,
							polyline,
							leg
						});
						return;
					}
					const polyline = generated?.polyline ?? savedRoute?.polyline ?? '';
					void reroute(currentWaypoints(), { lat, lon: lng, polyline });
				});
```

Add the round-trip branch at the top of `drawHandles`, and the per-leg helper:

```ts
	function drawHandles() {
		if (!map || !leaflet) return;
		waypointMarkers.forEach((m) => map!.removeLayer(m));
		waypointMarkers = [];

		if (roundTripRoutes && baseWaypoints && baseInbound) {
			drawLegHandles(baseWaypoints, 'outbound');
			drawLegHandles(baseInbound, 'inbound');
			return;
		}

		// ... the existing one-way and loop path, unchanged ...
	}

	/**
	 * Draggable handles for one leg.
	 *
	 * Only the outbound leg draws endpoints. The return leg's two ends are the
	 * SAME points -- Rust re-joins them on every call -- so drawing them again
	 * would put two handles on one coordinate, and dragging the lower one
	 * would silently be undone by the join.
	 */
	function drawLegHandles(points: Waypoint[], leg: Leg) {
		const legs = (next: Waypoint[]): [Waypoint[], Waypoint[]] =>
			leg === 'outbound' ? [next, currentInbound()] : [currentOutbound(), next];

		points.forEach((wp, i) => {
			const endpoint = i === 0 || i === points.length - 1;
			if (leg === 'inbound' && endpoint) return;

			const marker = leaflet!
				.marker([wp.lat, wp.lon], { draggable: true, icon: handleIcon(leaflet!, endpoint) })
				.addTo(map!);

			// ONE request, on release -- never during the drag.
			marker.on('dragend', () => {
				const { lat, lng } = marker.getLatLng();
				const next = points.map((p, j) => (j === i ? { ...p, lat, lon: lng } : p));
				const [outbound, inbound] = legs(next);
				void rerouteLegs(outbound, inbound);
			});

			if (!endpoint) {
				marker.bindTooltip($LL.routeMap.removeWaypoint());
				marker.on('click', () => {
					const [outbound, inbound] = legs(points.filter((_, j) => j !== i));
					void rerouteLegs(outbound, inbound);
				});
			}

			waypointMarkers.push(marker);
		});
	}
```

Change `currentWaypoints()` -- the one-way path -- to prefer the split
outbound leg over the saved closed list:

```ts
	/** The waypoints any one-way re-route should start from. On a re-opened
	 *  saved route these may include vias -- always prefer this over
	 *  `waypointsFromEndpoints()`, which drops them.
	 *
	 *  `savedLegs?.outbound` sits AHEAD of `savedRoute?.waypoints`: for a
	 *  saved round trip the latter is the closed list, and handing it to
	 *  `route_direct` with `round_trip: false` makes ADR-041's normaliser
	 *  strip one trailing point -- which recovered the outbound leg before
	 *  this task, and now turns `[A, B, v, A]` into the one-way route
	 *  `A -> B -> v`. Reachable without doing anything unusual: untick the
	 *  box on a saved round trip, let the request fail (`runDirect`'s catch
	 *  nulls `baseWaypoints`), then press Retry. */
	function currentWaypoints(): Waypoint[] {
		return baseWaypoints ?? savedLegs?.outbound ?? savedRoute?.waypoints ?? waypointsFromEndpoints();
	}
```

- [ ] **Step 9: Split a saved round trip back into its legs on a cold load**

Add:

```ts
	/**
	 * Split a saved round trip back into its two legs.
	 *
	 * A slice and a range check, and no rule of its own: `get_trip_route`
	 * resolves the index for every round trip, including the legacy rows that
	 * store none, so there is nothing left to infer here. Deciding how to read
	 * persisted data is business logic and belongs in Rust (ADR-008) -- and
	 * inference in this file is precisely what put a via on the wrong leg
	 * before.
	 *
	 * The two lists overlap by one point on purpose -- the turnaround belongs
	 * to both legs, and both routing requests need it.
	 */
	function splitSavedLegs(route: RouteMap): { outbound: Waypoint[]; inbound: Waypoint[] } | null {
		const at = route.turnaroundIndex;
		const points = route.waypoints;
		if (at === null || at < 1 || at > points.length - 2) return null;
		return { outbound: points.slice(0, at + 1), inbound: points.slice(at) };
	}
```

Replace the saved-route branch of `loadRoute` with:

```ts
		savedRoute = await getTripRoute(tripId);
		if (savedRoute) {
			mode = savedRoute.mode;
			roundTrip = savedRoute.roundTrip;
			const legs = savedRoute.roundTrip ? splitSavedLegs(savedRoute) : null;
			savedLegs = legs;
			rehydrateEndpoints(savedRoute, legs);
			if (legs) {
				baseWaypoints = legs.outbound;
				baseInbound = legs.inbound;
			} else {
				baseWaypoints = savedRoute.roundTrip
					? savedRoute.waypoints.slice(0, -1)
					: savedRoute.waypoints;
				baseInbound = null;
			}
			return;
		}
		await startForTrip();
```

Change `rehydrateEndpoints` to take the legs and read the destination from the outbound leg:

```ts
	/**
	 * A saved route already contains its endpoints -- recover them so
	 * Recalculate and editing work on a re-opened map without re-geocoding.
	 *
	 * A round trip's stored list is closed, so its FIRST and LAST points are
	 * both the origin. Reading the destination off the last point resolves it
	 * to the origin -- wrong before this task, and latent rather than visible:
	 * the only reader is `waypointsFromEndpoints()`, which is the LAST
	 * fallback in `currentWaypoints()` and is never reached while a saved
	 * route is loaded. This task adds a second reader (`endpointsMissing`), so
	 * fix it here rather than leave a wrong value one reader away from
	 * mattering. The outbound leg's own last point is the destination.
	 */
	function rehydrateEndpoints(
		route: RouteMap,
		legs: { outbound: Waypoint[]; inbound: Waypoint[] } | null
	) {
		const points = legs ? legs.outbound : route.waypoints;
		if (points.length < 2) return;
		const first = points[0];
		const last = points[points.length - 1];
		resolvedOrigin = { lat: first.lat, lon: first.lon, displayName: first.name ?? '' };
		resolvedDestination = { lat: last.lat, lon: last.lon, displayName: last.name ?? '' };
	}
```

- [ ] **Step 10: Save the chosen pair**

Replace `handleSave` with:

```ts
	async function handleSave() {
		if (!tripId) return;
		const legs = roundTripRoutes;
		if (!generated && !legs) return;
		saving = true;
		try {
			if (legs) {
				// Only which alternative was picked crosses the wire. The
				// backend joins the legs, concatenates the geometry and sums
				// the distances (ADR-008).
				await saveTripRoundTripRoute(
					tripId,
					legs.outboundWaypoints,
					legs.inboundWaypoints,
					legs.outbound[outboundIndex].polyline,
					legs.inbound[inboundIndex].polyline,
					legs.outbound[outboundIndex].roadKm,
					legs.inbound[inboundIndex].roadKm,
					legs.targetKm
				);
			} else {
				await saveTripRoute(tripId, generated!, roundTrip);
			}
			// Re-read so the displayed route is the persisted one, not a local copy.
			savedRoute = await getTripRoute(tripId);
			generated = null;
			alternatives = [];
			activeIndex = 0;
			roundTripRoutes = null;
			announce('route-map-saved');
			savedNotice = true;
			toast.success($LL.routeMap.saved());
		} catch (e) {
			console.error('Failed to save route map:', e);
			toast.error($LL.routeMap.error());
		} finally {
			saving = false;
		}
	}
```

- [ ] **Step 11: Update the markup**

Recalculate button and the checkbox both gain the endpoint gate:

```svelte
				disabled={busy || !trip || endpointsMissing}
```

Save button:

```svelte
				disabled={busy || (!generated && !roundTripRoutes)}
```

Add the endpoint hint under the toolbar, before the `{#if savedNotice}` block:

```svelte
	{#if endpointsMissing && !busy}
		<div class="error-box" data-test="endpoint-missing">
			<span>{$LL.routeMap.endpointMissing()}</span>
			<button
				class="button-small"
				data-test="place-endpoint-btn"
				onclick={() => (unplacedField = resolvedOrigin ? 'destination' : 'origin')}
			>
				{$LL.routeMap.placeEndpoint()}
			</button>
		</div>
	{/if}
```

Change the info panel's guard from `{:else if displayRoute}` to `{:else if hasRoute}`, and its three values to the new derived ones:

```svelte
			<span class="info-item">
				<span class="label">{$LL.routeMap.targetKm()}</span>
				<span class="value" data-test="target-km">{(displayTargetKm ?? 0).toFixed(1)} km</span>
			</span>
			<span class="info-item">
				<span class="label">{$LL.routeMap.actualKm()}</span>
				<span class="value" data-test="actual-km">{(displayRoadKm ?? 0).toFixed(1)} km</span>
			</span>
```

Replace the alternatives block with:

```svelte
		{#if mode === 'direct' && roundTrip}
			<div class="alternatives legs" data-test="leg-alternatives">
				<div class="leg" data-test="leg-outbound">
					<span class="label">{$LL.routeMap.alternatives()} - {$LL.routeMap.legOutbound()}</span>
					{#if outboundHasVias}
						<p class="hint" data-test="alternatives-unavailable-outbound">
							{$LL.routeMap.alternativesUnavailable()}
						</p>
					{:else if roundTripRoutes}
						<ul>
							{#each roundTripRoutes.outbound as leg, i}
								<li>
									<button
										class="alternative"
										class:active={i === outboundIndex}
										data-test="leg-alternative-btn"
										aria-pressed={i === outboundIndex}
										onclick={() => selectLeg('outbound', i)}
									>
										<span>{leg.roadKm.toFixed(1)} km</span>
										<span title={$LL.routeMap.duration()}>{formatDuration(leg.durationS)}</span>
									</button>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
				<div class="leg" data-test="leg-inbound">
					<span class="label">{$LL.routeMap.alternatives()} - {$LL.routeMap.legInbound()}</span>
					{#if inboundHasVias}
						<p class="hint" data-test="alternatives-unavailable-inbound">
							{$LL.routeMap.alternativesUnavailable()}
						</p>
					{:else if roundTripRoutes}
						<ul>
							{#each roundTripRoutes.inbound as leg, i}
								<li>
									<button
										class="alternative"
										class:active={i === inboundIndex}
										data-test="leg-alternative-btn"
										aria-pressed={i === inboundIndex}
										onclick={() => selectLeg('inbound', i)}
									>
										<span>{leg.roadKm.toFixed(1)} km</span>
										<span title={$LL.routeMap.duration()}>{formatDuration(leg.durationS)}</span>
									</button>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			</div>
		{:else if mode === 'direct' && !hasVias && alternatives.length > 0}
			<!-- ... the existing one-way picker, unchanged ... -->
		{:else if mode === 'direct' && hasVias}
			<p class="hint" data-test="alternatives-unavailable">
				{$LL.routeMap.alternativesUnavailable()}
			</p>
		{/if}
```

Add the layout rule to the `<style>` block:

```css
	.alternatives.legs {
		display: flex;
		gap: 1.5rem;
		flex-wrap: wrap;
	}

	.alternatives.legs .leg {
		flex: 1 1 14rem;
		min-width: 0;
	}
```

- [ ] **Step 12: Check and build**

Run: `npm run check && npm run build`
Expected: no errors.

- [ ] **Step 13: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts src/routes/mapa/+page.svelte \
        src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts src/lib/i18n/i18n-types.ts
git commit -m "feat(routes): pick an alternative on each leg of a round trip"
```

---

### Task 5: Measure what a new distance does to the consumption period

**Files:**
- Modify: `src-tauri/core/src/models.rs`
- Modify: `src-tauri/core/src/commands_internal/statistics.rs`
- Test: `src-tauri/core/src/commands_internal/commands_tests.rs`

**Interfaces:**
- Consumes: `calculate_period_rates`, `calculate_margin_percent`, `is_within_legal_limit`, `trip_order` -- all already in the crate.
- Produces:
  - `pub struct PeriodMarginImpact { period_closed: bool, tp_consumption: f64, rate_before: f64, rate_after: f64, margin_before: f64, margin_after: f64, over_limit_before: bool, over_limit_after: bool }`
  - `pub fn period_margin_impact(trips: &[Trip], tp_consumption: f64, trip_id: &str, new_distance_km: f64) -> PeriodMarginImpact`

- [ ] **Step 1: Write the failing tests**

Append to `src-tauri/core/src/commands_internal/commands_tests.rs`:

```rust
// ============================================================================
// Task 78: what a distance write-back does to the consumption period
// ============================================================================

/// Two trips in one closed period: 100 km with no fill-up, then 100 km closing
/// on 12 litres. 200 km on 12 l is 6.0 l/100km, which against a 5.0 l/100km TP
/// rate is exactly the 20 % legal limit.
fn seed_closed_period(db: &Database, vehicle_id: Uuid) -> (Uuid, Uuid) {
    let a = seed_chain_trip(db, vehicle_id, 1, 100.0, 50100.0);
    let date = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
    let mut b = make_trip_detailed(date, 100.0, Some(12.0), true);
    b.vehicle_id = vehicle_id;
    b.odometer = 50200.0;
    db.create_trip(&b).unwrap();
    (a, b.id)
}

#[test]
fn test_period_margin_impact_moves_the_period_rate_and_the_margin() {
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Impact".to_string(), "BA1".to_string(), 60.0, 5.0, 50000.0);
    db.create_vehicle(&vehicle).unwrap();
    let (a, _b) = seed_closed_period(&db, vehicle.id);

    let trips = db.get_trips_for_vehicle_in_year(&vehicle.id.to_string(), 2026).unwrap();

    // Shorten the first trip from 100 km to 90 km: the period keeps its 12
    // litres but loses 10 km, so its rate goes up.
    let impact = period_margin_impact(&trips, 5.0, &a.to_string(), 90.0);

    assert!(impact.period_closed, "a full-tank fill-up closed this period");
    assert!((impact.rate_before - 6.0).abs() < 1e-9);
    assert!((impact.margin_before - 20.0).abs() < 1e-9);
    assert!(!impact.over_limit_before, "exactly 20 % is still legal");

    // 12 l over 190 km is 6.3158 l/100km, 26.3 % over a 5.0 TP rate.
    assert!((impact.rate_after - (1200.0 / 190.0)).abs() < 1e-9);
    assert!((impact.margin_after - 26.315_789_473_684_2).abs() < 1e-6);
    assert!(impact.over_limit_after, "the write crosses the 20 % legal limit");
}

#[test]
fn test_period_margin_impact_can_bring_a_period_back_under_the_limit() {
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Impact".to_string(), "BA1".to_string(), 60.0, 5.0, 50000.0);
    db.create_vehicle(&vehicle).unwrap();
    let (a, _b) = seed_closed_period(&db, vehicle.id);
    let trips = db.get_trips_for_vehicle_in_year(&vehicle.id.to_string(), 2026).unwrap();

    // Lengthen the first trip to 140 km: 240 km on 12 l is 5.0 l/100km, the TP
    // rate exactly, so the margin falls to zero.
    let impact = period_margin_impact(&trips, 5.0, &a.to_string(), 140.0);
    assert!((impact.rate_after - 5.0).abs() < 1e-9);
    assert!((impact.margin_after).abs() < 1e-9);
    assert!(!impact.over_limit_after);
}

#[test]
fn test_period_margin_impact_reports_an_open_period_as_open() {
    // No full-tank fill-up, so nothing closed. The period's rate is the TP
    // rate -- an estimate, not a measurement -- and the modal has to say so
    // rather than present it as a legal number.
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Impact".to_string(), "BA1".to_string(), 60.0, 5.0, 50000.0);
    db.create_vehicle(&vehicle).unwrap();
    let a = seed_chain_trip(&db, vehicle.id, 1, 100.0, 50100.0);
    let trips = db.get_trips_for_vehicle_in_year(&vehicle.id.to_string(), 2026).unwrap();

    let impact = period_margin_impact(&trips, 5.0, &a.to_string(), 90.0);
    assert!(!impact.period_closed);
    assert!((impact.rate_before - 5.0).abs() < 1e-9);
    assert!((impact.rate_after - 5.0).abs() < 1e-9);
}

#[test]
fn test_period_margin_impact_leaves_other_periods_alone() {
    // Period membership is decided by order and by the full_tank flag, never
    // by distance, so exactly one period's rate can move.
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Impact".to_string(), "BA1".to_string(), 60.0, 5.0, 50000.0);
    db.create_vehicle(&vehicle).unwrap();
    let (a, b) = seed_closed_period(&db, vehicle.id);

    let date = NaiveDate::from_ymd_opt(2026, 3, 3).unwrap();
    let mut c = make_trip_detailed(date, 100.0, Some(5.0), true);
    c.vehicle_id = vehicle.id;
    c.odometer = 50300.0;
    db.create_trip(&c).unwrap();

    let trips = db.get_trips_for_vehicle_in_year(&vehicle.id.to_string(), 2026).unwrap();

    let second = period_margin_impact(&trips, 5.0, &c.id.to_string(), 90.0);
    assert!((second.rate_before - 5.0).abs() < 1e-9, "100 km on 5 l");

    let first = period_margin_impact(&trips, 5.0, &a.to_string(), 90.0);
    assert!((first.rate_before - 6.0).abs() < 1e-9, "the first period is untouched");

    // b sits in the first period, so it reports the first period's numbers.
    let same_period = period_margin_impact(&trips, 5.0, &b.to_string(), 100.0);
    assert!((same_period.rate_before - 6.0).abs() < 1e-9);
}
```

Add `period_margin_impact` to the `use crate::commands_internal::statistics::{...}` list at the top of `commands_tests.rs`.

- [ ] **Step 2: Run the tests and watch them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core period_margin_impact`
Expected: FAIL to compile -- `period_margin_impact` does not exist.

- [ ] **Step 3: Add the model**

In `src-tauri/core/src/models.rs`, beside `CascadePlan`:

```rust
/// What a change to one trip's distance does to the fuel-consumption period
/// that contains it.
///
/// A period closes on a full-tank fill-up and its rate is
/// `period_fuel / period_km * 100`. Changing one trip's distance moves that
/// period's kilometres, so it moves the rate, the margin, and which side of
/// the 20 % legal limit the period sits on ([BIZ-003]). The user has to see
/// that BEFORE the write, not discover it in the grid afterwards.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodMarginImpact {
    /// False when the trip sits in the still-open period. That period's rate
    /// is the vehicle's TP rate -- an estimate, not a measurement -- so it is
    /// not a legal number and must not be presented as one.
    pub period_closed: bool,
    /// The vehicle's TP rate. Zero for a vehicle that has none (a BEV), which
    /// is the display's cue to omit the margin entirely rather than print a
    /// row of zeroes.
    pub tp_consumption: f64,
    pub rate_before: f64,
    pub rate_after: f64,
    pub margin_before: f64,
    pub margin_after: f64,
    pub over_limit_before: bool,
    pub over_limit_after: bool,
}
```

- [ ] **Step 4: Implement it**

In `src-tauri/core/src/commands_internal/statistics.rs`, after `has_any_period_over_limit`:

```rust
/// What setting `trip_id`'s distance to `new_distance_km` would do to the
/// consumption period that contains it.
///
/// Period MEMBERSHIP does not move: `calculate_period_rates` groups by order
/// and by the `full_tank` flag, never by distance. So exactly one period's
/// rate changes, and it is the period this trip is in.
///
/// The rate is read through `calculate_period_rates` rather than recomputed,
/// so the number in the warning is the same number the grid shows for that
/// row -- a warning measured a second way would be worse than no warning.
pub fn period_margin_impact(
    trips: &[Trip],
    tp_consumption: f64,
    trip_id: &str,
    new_distance_km: f64,
) -> PeriodMarginImpact {
    let mut before: Vec<Trip> = trips.to_vec();
    before.sort_by(|a, b| trip_order(a, b));

    let mut after = before.clone();
    if let Some(trip) = after.iter_mut().find(|t| t.id.to_string() == trip_id) {
        trip.distance_km = new_distance_km;
    }

    let (rates_before, estimated) = calculate_period_rates(&before, tp_consumption);
    let (rates_after, _) = calculate_period_rates(&after, tp_consumption);

    let rate_before = rates_before.get(trip_id).copied().unwrap_or(tp_consumption);
    let rate_after = rates_after.get(trip_id).copied().unwrap_or(tp_consumption);
    let margin_before = calculate_margin_percent(rate_before, tp_consumption);
    let margin_after = calculate_margin_percent(rate_after, tp_consumption);

    PeriodMarginImpact {
        period_closed: !estimated.contains(trip_id),
        tp_consumption,
        rate_before,
        rate_after,
        margin_before,
        margin_after,
        over_limit_before: !is_within_legal_limit(margin_before),
        over_limit_after: !is_within_legal_limit(margin_after),
    }
}
```

Add `PeriodMarginImpact` to the `use crate::models::{...}` list at the top of `statistics.rs`.

- [ ] **Step 5: Run the tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core period_margin_impact`
Expected: PASS, four tests.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/core/src/models.rs \
        src-tauri/core/src/commands_internal/statistics.rs \
        src-tauri/core/src/commands_internal/commands_tests.rs
git commit -m "feat(trips): measure what a new distance does to a period's margin"
```

---

### Task 6: Write the routed distance onto the trip

**Files:**
- Modify: `src-tauri/core/src/models.rs`
- Modify: `src-tauri/core/src/commands_internal/trips.rs`
- Modify: `src-tauri/core/src/server/dispatcher.rs`
- Test: `src-tauri/core/src/commands_internal/commands_tests.rs`

**Interfaces:**
- Consumes: `plan_odometer_cascade`, `mark_next_year_chain_breaks`, `get_year_start_odometer`, `db.update_trip_with_odometer_shift`, `period_margin_impact` (Task 5), and the test helper `seed_closed_period` (also Task 5) -- run Task 5 first.
- Produces:
  - `pub struct DistanceWriteback { trip_id: String, distance_before: f64, distance_after: f64, plan: CascadePlan, margin: PeriodMarginImpact, trip: Option<Trip> }`
  - `pub fn apply_route_distance_internal(db, app_state, trip_id: String, road_km: f64, dry_run: bool) -> Result<DistanceWriteback, String>`
  - RPC command `apply_route_distance`

- [ ] **Step 1: Write the failing tests**

Append to `src-tauri/core/src/commands_internal/commands_tests.rs`:

```rust
#[test]
fn test_apply_route_distance_dry_run_writes_nothing() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);

    let result =
        apply_route_distance_internal(&db, &app_state, a.to_string(), 61.5, true).unwrap();

    assert!(result.trip.is_none(), "a dry run returns no saved trip");
    assert!((result.distance_before - 50.0).abs() < 1e-9);
    assert!((result.distance_after - 61.5).abs() < 1e-9);
    assert!((result.plan.delta - 11.5).abs() < 1e-9);
    assert_eq!(result.plan.changes.len(), 1, "the one row after it would move");

    // Nothing moved.
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().distance_km, 50.0);
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().odometer, 50120.0);
}

#[test]
fn test_apply_route_distance_writes_the_row_and_shifts_the_rest() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);
    let c = seed_chain_trip(&db, vehicle.id, 3, 30.0, 50150.0);

    apply_route_distance_internal(&db, &app_state, a.to_string(), 61.5, false).unwrap();

    let written = db.get_trip(&a.to_string()).unwrap().unwrap();
    assert!((written.distance_km - 61.5).abs() < 1e-9);
    // The odometer follows the distance: anchor (the year start) + km.
    assert!((written.odometer - 50061.5).abs() < 1e-9);
    // The invariant holds for the edited row and for every row after it.
    assert!((db.get_trip(&b.to_string()).unwrap().unwrap().odometer - 50131.5).abs() < 1e-9);
    assert!((db.get_trip(&c.to_string()).unwrap().unwrap().odometer - 50161.5).abs() < 1e-9);
}

#[test]
fn test_apply_route_distance_does_not_invent_an_end_time() {
    // `update_trip_cascade_internal` rebuilds a row from submitted strings and
    // writes `end_datetime: Some(...)` unconditionally. Routing that path here
    // would stamp an end time onto a trip that stored none, as a side effect
    // of applying a distance. This write touches three fields and no others.
    //
    // No stored row is affected TODAY: `create_trip`, `update_trip` and
    // `update_trip_cascade` all take `end_datetime: String`, so no caller can
    // write a NULL, and the production copy holds none (329 rows, 0 null,
    // measured 2026-09-09). This is a guard on the write-back path, not a
    // repair -- it keeps the property true before something else can create
    // such a row.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    assert!(db.get_trip(&a.to_string()).unwrap().unwrap().end_datetime.is_none());

    let before = db.get_trip(&a.to_string()).unwrap().unwrap();
    apply_route_distance_internal(&db, &app_state, a.to_string(), 61.5, false).unwrap();
    let after = db.get_trip(&a.to_string()).unwrap().unwrap();

    assert!(after.end_datetime.is_none(), "a trip with no end time keeps none");
    assert_eq!(after.start_datetime, before.start_datetime);
    assert_eq!(after.origin, before.origin);
    assert_eq!(after.destination, before.destination);
    assert_eq!(after.purpose, before.purpose);
    assert_eq!(after.fuel_liters, before.fuel_liters);
    assert_eq!(after.full_tank, before.full_tank);
    assert_eq!(after.created_at, before.created_at);
}

#[test]
fn test_apply_route_distance_reports_the_margin_it_would_cause() {
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Writeback".to_string(), "BA2".to_string(), 60.0, 5.0, 50000.0);
    db.create_vehicle(&vehicle).unwrap();
    let app_state = crate::app_state::AppState::new();
    let (a, _b) = seed_closed_period(&db, vehicle.id);

    let result =
        apply_route_distance_internal(&db, &app_state, a.to_string(), 90.0, true).unwrap();

    assert!(result.margin.period_closed);
    assert!(!result.margin.over_limit_before);
    assert!(
        result.margin.over_limit_after,
        "the warning must say the write crosses the 20 % limit"
    );
}

#[test]
fn test_apply_route_distance_is_read_only_guarded() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    app_state.enable_read_only("newer migrations");

    // Reading is always allowed, so the dry run still answers.
    assert!(apply_route_distance_internal(&db, &app_state, a.to_string(), 61.5, true).is_ok());
    // Writing is not.
    assert!(apply_route_distance_internal(&db, &app_state, a.to_string(), 61.5, false).is_err());
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().distance_km, 50.0);
}

#[test]
fn test_apply_route_distance_refuses_a_distance_that_is_not_one() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);

    assert!(apply_route_distance_internal(&db, &app_state, a.to_string(), -1.0, true).is_err());
    assert!(apply_route_distance_internal(&db, &app_state, a.to_string(), f64::NAN, true).is_err());
}
```

- [ ] **Step 2: Run the tests and watch them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core apply_route_distance`
Expected: FAIL to compile -- `apply_route_distance_internal` does not exist.

- [ ] **Step 3: Add the model**

In `src-tauri/core/src/models.rs`, after `CascadeResult`:

```rust
/// The answer `apply_route_distance` gives. `trip` is `None` on a dry run,
/// because a dry run saves nothing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistanceWriteback {
    pub trip_id: String,
    pub distance_before: f64,
    pub distance_after: f64,
    /// What the write does to the odometer chain of the year (ADR-046).
    pub plan: CascadePlan,
    /// What it does to the consumption period the row sits in (BIZ-003).
    pub margin: PeriodMarginImpact,
    pub trip: Option<Trip>,
}
```

- [ ] **Step 4: Implement it**

In `src-tauri/core/src/commands_internal/trips.rs`, after `update_trip_cascade_internal`:

```rust
/// Write a route's road distance onto the trip it illustrates.
///
/// This reverses ADR-039, behind the two things that ADR named as the reason
/// not to do it silently: the odometer chain and the consumption period. Both
/// are computed first and returned as a dry run, so the user approves a
/// specific set of numbers rather than a general idea.
///
/// It deliberately does NOT go through `update_trip_cascade_internal`. That
/// path rebuilds the whole row from submitted strings, and `build_updated_trip`
/// writes `end_datetime: Some(parse(...))` unconditionally -- a trip that
/// stored `None` would silently gain an end time as a side effect of applying
/// a distance. Here the stored row is the base and exactly three fields move:
/// `distance_km`, `odometer` and `updated_at`. `find_or_create_route` is not
/// called either: the origin and the destination did not change.
///
/// The cascade itself is not reimplemented -- `plan_odometer_cascade` is the
/// same planner the grid's own save uses, so the two paths cannot disagree
/// about what a distance change does to the chain.
pub fn apply_route_distance_internal(
    db: &Database,
    app_state: &AppState,
    trip_id: String,
    road_km: f64,
    dry_run: bool,
) -> Result<DistanceWriteback, String> {
    if !dry_run {
        check_read_only!(app_state);
    }

    if !road_km.is_finite() || road_km < 0.0 {
        return Err(format!(
            "Road distance {road_km} is not a distance that can be written to a trip"
        ));
    }

    let existing = db
        .get_trip(&trip_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip not found: {trip_id}"))?;
    let distance_before = existing.distance_km;

    let vehicle_id = existing.vehicle_id.to_string();
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    let year = existing.start_datetime.year();
    let trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;
    let year_start = get_year_start_odometer(db, &vehicle_id, year, vehicle.initial_odometer)?;

    // The stored odometer is submitted unchanged, so `plan_odometer_cascade`
    // takes its km-wins branch: the row ends at `anchor + road_km` and every
    // later row of the year moves by the same delta. On a row whose stored
    // odometer already disagreed with `anchor + km`, that delta also carries
    // the repair -- the plan reports the two parts separately and the modal
    // shows both.
    let mut plan =
        plan_odometer_cascade(&trips, year_start, &trip_id, road_km, existing.odometer)?;
    mark_next_year_chain_breaks(db, &vehicle_id, year, &mut plan)?;

    let margin = period_margin_impact(
        &trips,
        vehicle.tp_consumption.unwrap_or_default(),
        &trip_id,
        plan.new_distance_km,
    );

    if dry_run {
        return Ok(DistanceWriteback {
            trip_id,
            distance_before,
            distance_after: plan.new_distance_km,
            plan,
            margin,
            trip: None,
        });
    }

    let trip = Trip {
        distance_km: plan.new_distance_km,
        odometer: plan.new_odometer,
        updated_at: Utc::now(),
        ..existing
    };

    // The row and its shift go to the database in one transaction. They are
    // one correction to a legal record, so a partial write is never
    // acceptable.
    let shifts: Vec<(String, f64)> = plan
        .changes
        .iter()
        .map(|c| (c.trip_id.clone(), c.new_odometer))
        .collect();
    db.update_trip_with_odometer_shift(&trip, &shifts)
        .map_err(|e| e.to_string())?;

    Ok(DistanceWriteback {
        trip_id,
        distance_before,
        distance_after: plan.new_distance_km,
        plan,
        margin,
        trip: Some(trip),
    })
}
```

Add `DistanceWriteback` and `PeriodMarginImpact` to the `use crate::models::{...}` list at the top of `trips.rs`, and `period_margin_impact` to the `use super::{...}` (or `use crate::commands_internal::statistics::{...}`) list -- whichever form the file already uses for `get_year_start_odometer`.

- [ ] **Step 5: Run the tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core apply_route_distance`
Expected: PASS, six tests.

- [ ] **Step 6: Register the RPC command**

In `src-tauri/core/src/server/dispatcher.rs`, after the `"update_trip_cascade"` arm:

```rust
        "apply_route_distance" => {
            // Two arguments and nothing else. The trip's other fields are not
            // resubmitted, so this command cannot change them even by mistake
            // -- which is the point (task 78).
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                trip_id: String,
                road_km: f64,
                dry_run: bool,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::apply_route_distance_internal(
                &state.db,
                &state.app_state,
                a.trip_id,
                a.road_km,
                a.dry_run,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
```

- [ ] **Step 7: Run the whole backend suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/core/src/models.rs \
        src-tauri/core/src/commands_internal/trips.rs \
        src-tauri/core/src/commands_internal/commands_tests.rs \
        src-tauri/core/src/server/dispatcher.rs
git commit -m "feat(trips): apply a routed distance to the trip it illustrates"
```

---

### Task 7: The map view offers the write-back, behind the warning

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/api.ts`
- Modify: `src/lib/components/OdometerCascadeModal.svelte`
- Modify: `src/routes/mapa/+page.svelte`
- Modify: `src/lib/components/TripGrid.svelte`
- Modify: `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`
- Test: Task 8

**Interfaces:**
- Consumes: `apply_route_distance` (Task 6), `displayRoadKm` (Task 4).
- Produces: `applyRouteDistance()` in `api.ts`; `OdometerCascadeModal` accepts `kind="writeback"` and `margin`; the `data-test` hooks Task 8 asserts on: `apply-distance-btn`, and the `data-testid` hooks `writeback-margin`, `writeback-crosses-limit`, `writeback-leaves-limit`.

- [ ] **Step 1: Add the types**

In `src/lib/types.ts`, beside `CascadePlan`:

```ts
/** What a change to one trip's distance does to the consumption period that
 *  contains it. A period closes on a full-tank fill-up, and its rate decides
 *  the 20 % legal margin -- so this is what the user must see BEFORE the
 *  write, not discover in the grid afterwards. */
export interface PeriodMarginImpact {
	/** False when the trip is in the still-open period, whose rate is the TP
	 *  rate: an estimate, not a legal number. */
	periodClosed: boolean;
	/** Zero for a vehicle with no TP rate (a BEV). The cue to omit the margin
	 *  lines entirely rather than print a row of zeroes. */
	tpConsumption: number;
	rateBefore: number;
	rateAfter: number;
	marginBefore: number;
	marginAfter: number;
	overLimitBefore: boolean;
	overLimitAfter: boolean;
}

/** The answer `apply_route_distance` gives. `trip` is null on a dry run. */
export interface DistanceWriteback {
	tripId: string;
	distanceBefore: number;
	distanceAfter: number;
	plan: CascadePlan;
	margin: PeriodMarginImpact;
	trip: Trip | null;
}
```

- [ ] **Step 2: Add the API call**

In `src/lib/api.ts`, beside `updateTripCascade`:

```ts
/**
 * Write a route's road distance onto the trip it illustrates.
 *
 * Call it twice: `dryRun: true` fills the confirmation modal and writes
 * nothing, then `dryRun: false` writes. The apply call plans again from the
 * stored book rather than replaying the dry run's numbers, so a book that
 * moved in between is corrected against as it is now.
 *
 * Only the distance crosses the wire. The trip's other fields are not
 * resubmitted, so this command cannot change them even by mistake.
 */
export async function applyRouteDistance(
	tripId: string,
	roadKm: number,
	dryRun: boolean
): Promise<DistanceWriteback> {
	return await apiCall('apply_route_distance', { tripId, roadKm, dryRun });
}
```

- [ ] **Step 3: Add the i18n keys**

In `src/lib/i18n/sk/index.ts`, inside `trips`, after the `cascade` block:

```ts
		writeback: {
			title: 'Zápis vzdialenosti trasy',
			summary: 'Vzdialenosť jazdy sa zmení z {oldKm:string} na {newKm:string} km.',
			noShift: 'Žiadna nasledujúca jazda sa neposunie.',
			periodOpen: 'Jazda patrí do neuzavretého obdobia. Spotreba sa zatiaľ počíta z hodnoty v TP, takže sa nemení.',
			periodRate: 'Spotreba obdobia: {before:string} -> {after:string} l/100km',
			periodMargin: 'Odchýlka od TP: {before:string} % -> {after:string} % (zákonný limit 20 %)',
			crossesLimit: 'Pozor: po zápise obdobie prekročí zákonný limit 20 %.',
			staysOverLimit: 'Obdobie je nad zákonným limitom 20 % pred zápisom aj po ňom.',
			leavesLimit: 'Po zápise sa obdobie vráti pod zákonný limit 20 %.',
			confirm: 'Zapísať vzdialenosť',
		},
```

In `src/lib/i18n/sk/index.ts`, inside `routeMap`:

```ts
		applyDistance: 'Použiť vzdialenosť',
		applyDistanceTitle: 'Zapísať vzdialenosť trasy do jazdy',
		applyDistanceDone: 'Vzdialenosť zapísaná do jazdy',
		applyDistanceError: 'Vzdialenosť sa nepodarilo zapísať',
```

In `src/lib/i18n/en/index.ts`, the mirrors:

```ts
		writeback: {
			title: 'Apply the route distance',
			summary: 'The trip distance changes from {oldKm} to {newKm} km.',
			noShift: 'No following trip moves.',
			periodOpen: 'This trip is in an open period. Its consumption still comes from the TP rate, so it does not change.',
			periodRate: 'Period consumption: {before} -> {after} l/100km',
			periodMargin: 'Deviation from TP: {before} % -> {after} % (legal limit 20 %)',
			crossesLimit: 'Warning: after this write the period goes over the 20 % legal limit.',
			staysOverLimit: 'The period is over the 20 % legal limit before and after this write.',
			leavesLimit: 'After this write the period comes back under the 20 % legal limit.',
			confirm: 'Apply the distance',
		},
```

```ts
		applyDistance: 'Apply distance',
		applyDistanceTitle: 'Write the route distance onto the trip',
		applyDistanceDone: 'The distance was written onto the trip',
		applyDistanceError: 'Could not write the distance',
```

- [ ] **Step 4: Regenerate the i18n types**

Run: `npm run i18n && npm run check`
Expected: no error about the new keys.

- [ ] **Step 5: Teach the modal the write-back**

In `src/lib/components/OdometerCascadeModal.svelte`:

```ts
	import type { CascadePlan, PeriodMarginImpact, Trip } from '$lib/types';

	export let kind: 'edit' | 'insert' | 'delete' | 'writeback' = 'edit';
	/** Write-back only: what the change does to the consumption period. Null
	 *  for the three grid kinds, which do not carry one. */
	export let margin: PeriodMarginImpact | null = null;
```

Add the rate formatter beside `km`:

```ts
	/** Litres per 100 km and percentages, to one decimal. The grid shows
	 *  consumption to one decimal, and the warning must not look more precise
	 *  than the number it is warning about. */
	function rate(value: number): string {
		return (Math.round(value * 10) / 10 || 0).toFixed(1);
	}
```

Change the title:

```svelte
		<h2>{kind === 'writeback' ? $LL.trips.writeback.title() : $LL.trips.cascade.title()}</h2>
```

Add a write-back branch at the top of the summary paragraph, before the `insert` branch:

```svelte
					{#if kind === 'writeback'}
						{$LL.trips.writeback.summary({
							oldKm: km(oldDistanceKm),
							newKm: km(plan.newDistanceKm)
						})}
					{:else if kind === 'insert'}
```

Add the margin block and the shift line immediately after the `</p>` that closes the summary:

```svelte
			{#if kind === 'writeback' && margin}
				<div class="margin" data-testid="writeback-margin">
					{#if !margin.periodClosed}
						<p data-testid="writeback-period-open">{$LL.trips.writeback.periodOpen()}</p>
					{:else if margin.tpConsumption > 0}
						<p>
							{$LL.trips.writeback.periodRate({
								before: rate(margin.rateBefore),
								after: rate(margin.rateAfter)
							})}
						</p>
						<p>
							{$LL.trips.writeback.periodMargin({
								before: rate(margin.marginBefore),
								after: rate(margin.marginAfter)
							})}
						</p>
						{#if margin.overLimitAfter && !margin.overLimitBefore}
							<p class="crosses" data-testid="writeback-crosses-limit">
								{$LL.trips.writeback.crossesLimit()}
							</p>
						{:else if margin.overLimitAfter}
							<p class="crosses" data-testid="writeback-stays-over-limit">
								{$LL.trips.writeback.staysOverLimit()}
							</p>
						{:else if margin.overLimitBefore}
							<p data-testid="writeback-leaves-limit">{$LL.trips.writeback.leavesLimit()}</p>
						{/if}
					{/if}
				</div>
			{/if}
			{#if kind === 'writeback' && plan.changes.length === 0}
				<p data-testid="writeback-no-shift">{$LL.trips.writeback.noShift()}</p>
			{:else if kind === 'writeback'}
				<p>{$LL.trips.cascade.summary({ count: plan.changes.length, delta: signed(plan.delta) })}</p>
			{/if}
```

Guard the changes table so an empty plan renders no empty table:

```svelte
			{#if plan.changes.length > 0}
				<div class="changes">
					<!-- ... the existing table, unchanged ... -->
				</div>
			{/if}
```

Change the confirm label:

```svelte
			<button class="button-small" on:click={handleConfirm} data-testid="cascade-confirm">
				{#if kind === 'delete'}
					{$LL.trips.cascade.confirmDelete()}
				{:else if kind === 'writeback'}
					{$LL.trips.writeback.confirm()}
				{:else}
					{$LL.trips.cascade.confirm()}
				{/if}
			</button>
```

Add the styles:

```css
	.margin {
		margin: 0 0 0.75rem 0;
	}

	.margin p {
		margin: 0.25rem 0;
		font-size: 0.875rem;
	}

	.crosses {
		color: var(--accent-warning-dark);
		font-weight: 600;
	}
```

The `fromDistance` breakdown line already reads correctly for a write-back ("your change to the distance (50 -> 61.5 km)"), and the repair line under `showRepair` stays: a row whose stored odometer already disagreed with `anchor + km` has that repair travelling with the write, and hiding it would hide two corrections behind one number.

- [ ] **Step 6: Wire the button and the modal into the map view**

In `src/routes/mapa/+page.svelte`, extend the imports:

```ts
	import OdometerCascadeModal from '$lib/components/OdometerCascadeModal.svelte';
```
```ts
	import { applyRouteDistance } from '$lib/api';   // add to the existing import list
```
```ts
	import type { DistanceWriteback } from '$lib/types';   // add to the existing type import
```

Add state:

```ts
	/** The dry run currently awaiting the user's approval. Nothing is written
	 *  while this is null, and nothing is written when it is dismissed. */
	let writeback = $state<DistanceWriteback | null>(null);
	let applying = $state(false);
	/** The vehicle's trips, so the modal can name the rows the shift moves.
	 *  The plan reports ids only. */
	let yearTrips = $state<Trip[]>([]);
```

Add `applying` to `busy`:

```ts
	let busy = $derived(loading || generating || saving || removing || applying);
```

Store the trips in `loadTripAndRoute`, right after `const trips = await getTrips(vehicleId);`:

```ts
			yearTrips = trips;
```

Add the two handlers, beside `handleSave`:

```ts
	/** Plans the write and opens the modal. Writes NOTHING -- the dry run is
	 *  what fills the warning the user then approves. */
	async function handleApplyDistance() {
		if (!trip || displayRoadKm === null) return;
		applying = true;
		try {
			writeback = await applyRouteDistance(tripId, displayRoadKm, true);
		} catch (e) {
			console.error('Failed to plan the distance write-back:', e);
			toast.error($LL.routeMap.applyDistanceError());
		} finally {
			applying = false;
		}
	}

	/** Writes the distance the user approved -- `writeback.distanceAfter`, not
	 *  whatever the panel shows now: picking a different alternative behind the
	 *  modal must not change what Confirm commits. */
	async function confirmWriteback() {
		const approved = writeback;
		writeback = null;
		if (!approved || !trip) return;
		applying = true;
		try {
			await applyRouteDistance(tripId, approved.distanceAfter, false);
			// The trip's distance IS the map's target, so both the target and
			// the deviation move with it -- re-read the row and the saved map
			// rather than patching the numbers here.
			const vehicle = $activeVehicleStore;
			if (vehicle) {
				const trips = await getTrips(vehicle.id);
				yearTrips = trips;
				trip = trips.find((t) => t.id === tripId) ?? trip;
			}
			savedRoute = await getTripRoute(tripId);
			announce('trip-distance-updated');
			toast.success($LL.routeMap.applyDistanceDone());
		} catch (e) {
			console.error('Failed to write the distance back:', e);
			toast.error($LL.routeMap.applyDistanceError());
		} finally {
			applying = false;
		}
	}
```

Widen `announce`:

```ts
	function announce(type: 'route-map-saved' | 'route-map-removed' | 'trip-distance-updated') {
```

Add the toolbar button INSIDE the `{:else if mode === 'direct'}` branch,
after the round-trip label -- not beside the Save button, which sits outside
both mode branches:

```svelte
			<button
				class="button secondary"
				data-test="apply-distance-btn"
				title={$LL.routeMap.applyDistanceTitle()}
				onclick={handleApplyDistance}
				disabled={busy || displayRoadKm === null}
			>
				{$LL.routeMap.applyDistance()}
			</button>
```

**Direct mode only, deliberately.** A loop route is a genetic-algorithm route
GENERATED to match the trip's own recorded distance -- that is what `target_km`
is to the algorithm. Writing its road distance back would overwrite a real
logged number with the algorithm's approximation of that same number, and move
the legal margin to do it. The number carries no information about the real
journey, so there is nothing to reconcile. A direct route's distance does carry
that information, which is the whole case ADR-048 rests on.

Add the modal, beside the `confirmingRemove` one:

```svelte
{#if writeback}
	<OdometerCascadeModal
		kind="writeback"
		plan={writeback.plan}
		margin={writeback.margin}
		trips={yearTrips}
		oldDistanceKm={writeback.distanceBefore}
		onConfirm={confirmWriteback}
		onCancel={() => (writeback = null)}
	/>
{/if}
```

- [ ] **Step 7: Refresh the logbook tab**

The map opens in its own tab, so a write-back leaves an open logbook showing the old distance and the old odometers. In `src/lib/components/TripGrid.svelte`, extend the broadcast handler:

```ts
			routeMapChannel.onmessage = (event: MessageEvent<RouteMapMessage>) => {
				const data = event.data;
				if (!data?.tripId) return;
				// Reassign - mutating the Set in place would not trigger a re-render.
				if (data.type === 'route-map-saved') {
					routeMapTripIds = new Set(routeMapTripIds).add(data.tripId);
				} else if (data.type === 'route-map-removed') {
					const next = new Set(routeMapTripIds);
					next.delete(data.tripId);
					routeMapTripIds = next;
				} else if (data.type === 'trip-distance-updated') {
					// The map view wrote a routed distance onto a row. That
					// moves the row's own km and the odometer of every later
					// row of the year, so nothing short of a reload is right.
					void loadGridData();
				}
			};
```

- [ ] **Step 8: Check and build**

Run: `npm run check && npm run build`
Expected: no errors.

- [ ] **Step 9: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts \
        src/lib/components/OdometerCascadeModal.svelte \
        src/lib/components/TripGrid.svelte \
        src/routes/mapa/+page.svelte \
        src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts src/lib/i18n/i18n-types.ts
git commit -m "feat(routes): offer the routed distance to the trip, behind its warning"
```

---

### Task 8: Integration tests

**Files:**
- Modify: `tests/integration/specs/tier2/route-map.spec.ts`
- Create: `tests/integration/specs/tier2/route-distance-writeback.spec.ts`

**Interfaces:**
- Consumes: every `data-test` hook added in Tasks 4 and 7; the RPC commands `save_trip_round_trip_route` and `apply_route_distance`.
- Produces: nothing other tasks consume.

Build the two artifacts the harness needs before running anything:

```bash
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
```

- [ ] **Step 1: Correct the existing round-trip assertions**

`route-map.spec.ts` currently asserts that a saved round trip DISPLAYS `alternatives-unavailable` and that its text contains "more than two points". Under the new rule that message appears only for a leg that genuinely passes through an intermediate stop, and a plain round trip has none. In the test `reopens an already-closed round trip without corrupting its stop count`, replace the last block:

```ts
      // Task 78: a round trip is two routed legs now, so it is no longer a
      // route through more than two points and the unavailable copy is false
      // for it. Neither leg of this fixture has a via, so neither leg shows
      // the message.
      expect(await $('[data-test="alternatives-unavailable"]').isExisting()).toBe(false);
      expect(await $('[data-test="alternatives-unavailable-outbound"]').isExisting()).toBe(false);
      expect(await $('[data-test="alternatives-unavailable-inbound"]').isExisting()).toBe(false);
      // The two leg pickers exist even before a recalculation -- each is empty
      // until the routing service answers, which this file never asks it to.
      expect(await $('[data-test="leg-outbound"]').isDisplayed()).toBe(true);
      expect(await $('[data-test="leg-inbound"]').isDisplayed()).toBe(true);
```

- [ ] **Step 2: Add a saved round trip with a via on the return leg**

Add the fixture beside `saveDirectRoundTrip`:

```ts
/**
 * A round trip whose VIA is on the return leg -- the shape that could not be
 * recovered before Task 78. `turnaroundIndex: 1` says the outbound leg is
 * `[A, B]` and the return leg is `[B, via, A]`; without it, the old
 * "split at length - 2" rule would put the via on the way out.
 *
 * Saved through the new command, so the backend does the joining: the request
 * carries two legs and the row comes back as one four-point list.
 */
async function saveRoundTripWithReturnVia(tripId: string, targetKm: number): Promise<void> {
  await rpc<null>('save_trip_round_trip_route', {
    tripId,
    outboundWaypoints: CANNED_WAYPOINTS,
    inboundWaypoints: [
      { lat: 48.3774, lon: 17.5872, name: 'Trnava' },
      { lat: 48.28, lon: 17.35 },
      { lat: 48.1486, lon: 17.1077, name: 'Bratislava' },
    ],
    outboundPolyline: CANNED_POLYLINE,
    inboundPolyline: CANNED_POLYLINE,
    outboundRoadKm: targetKm / 2,
    inboundRoadKm: targetKm / 2,
    targetKm,
  });
}
```

- [ ] **Step 3: Write the failing test for the return-leg split**

Add to the same describe block:

```ts
    it('reopens a round trip with the via on the leg it was placed on', async () => {
      // Task 78: the saved row is one list, `[A, B, via, A]`, and only the
      // stored turnaround index says where the outbound leg ended. Splitting
      // it wrong is what used to move a via from the way home to the way out.
      const trip = await seedTrip({
        vehicleId,
        startDatetime: '2026-03-18T08:00',
        endDatetime: '2026-03-18T10:00',
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 65,
        odometer: 50365,
        purpose: 'Business trip',
      });

      await saveRoundTripWithReturnVia(trip.id as string, 65);
      await openMap(trip.id as string);
      await waitForMapOutcome('route');

      // Four stops: the turnaround is stored once, not twice.
      const stopsText = await $('[data-test="stops"]').getText();
      expect(stopsText).toContain('(3)');
      expect(stopsText).toContain('Bratislava → Trnava → Bratislava');

      const checkbox = await $('[data-test="round-trip-checkbox"]');
      expect(await checkbox.isSelected()).toBe(true);

      // The via is on the RETURN leg, so only that leg reports that it can
      // offer no alternatives.
      expect(await $('[data-test="alternatives-unavailable-inbound"]').isDisplayed()).toBe(true);
      expect(await $('[data-test="alternatives-unavailable-outbound"]').isExisting()).toBe(false);
    });
```

Note on the stop count: `stopNames` filters out unnamed waypoints, and a dragged via is unnamed, so a four-point list with one unnamed via prints three names. That is the existing behaviour of the stops line, not something this task changes.

- [ ] **Step 4: Run the corrected spec**

Run:
```bash
npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/route-map.spec.ts
```
Expected: PASS. If the round-trip stop count differs from the assertion above, fix the ASSERTION to what the page actually prints -- the filter on unnamed waypoints is pre-existing behaviour, not a regression.

- [ ] **Step 5: Write the write-back spec**

Create `tests/integration/specs/tier2/route-distance-writeback.spec.ts`:

```ts
/**
 * Tier 2: writing a routed distance back onto a trip (Task 78).
 *
 * This flow is fully testable here, unlike leg routing: a SAVED route is drawn
 * with no call to the routing service, so the Apply button, its modal and the
 * write are all reachable without the network. The same constraint as
 * route-map.spec.ts still applies -- nothing in this file calls
 * `generate_route`, `route_direct` or `route_round_trip`.
 *
 * NOT covered here on purpose: the period rate, the margin and the odometer
 * cascade are proven in Rust (`period_margin_impact` and
 * `apply_route_distance_internal` in commands_tests.rs). This file proves the
 * UI reaches them and shows what they answered.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle, rpc } from '../../utils/db';

const CANNED_POLYLINE = 'w_{dHcjlgBg}L{pd@wv]_bw@';
const CANNED_WAYPOINTS = [
  { lat: 48.1486, lon: 17.1077, name: 'Bratislava' },
  { lat: 48.3774, lon: 17.5872, name: 'Trnava' },
];

/** Persist a one-way direct route whose road distance differs from the row. */
async function saveRouteWithRoadKm(tripId: string, targetKm: number, roadKm: number) {
  await rpc<null>('save_trip_route', {
    tripId,
    waypoints: CANNED_WAYPOINTS,
    polyline: CANNED_POLYLINE,
    targetKm,
    roadKm,
    mode: 'direct',
    roundTrip: false,
  });
}

async function openMap(tripId: string) {
  await navigateTo(`/mapa?trip=${tripId}`);
  await $('[data-test="route-map-page"]').waitForExist({ timeout: 10000 });
  await $('[data-test="actual-km"]').waitForDisplayed({ timeout: 15000 });
}

describe('Route distance write-back', () => {
  let vehicleId: string;

  before(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  beforeEach(async () => {
    const vehicle = await seedVehicle({
      name: 'Writeback Car',
      licensePlate: 'BA-WB-1',
      tankSizeLiters: 60,
      tpConsumption: 5.0,
      initialOdometer: 50000,
    });
    vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);
  });

  it('writes the routed distance onto the trip and shifts the later rows', async () => {
    const first = await seedTrip({
      vehicleId,
      startDatetime: '2026-04-01T08:00',
      endDatetime: '2026-04-01T10:00',
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 50,
      odometer: 50050,
      purpose: 'Business trip',
    });
    const second = await seedTrip({
      vehicleId,
      startDatetime: '2026-04-02T08:00',
      endDatetime: '2026-04-02T10:00',
      origin: 'Trnava',
      destination: 'Bratislava',
      distanceKm: 40,
      odometer: 50090,
      purpose: 'Business trip',
    });

    await saveRouteWithRoadKm(first.id as string, 50, 61.5);
    await openMap(first.id as string);

    await $('[data-test="apply-distance-btn"]').click();

    const modal = await $('[data-testid="cascade-modal"]');
    await modal.waitForDisplayed({ timeout: 5000 });
    const summary = await $('[data-testid="cascade-summary"]').getText();
    expect(summary).toContain('50');
    expect(summary).toContain('61.5');

    await $('[data-testid="cascade-confirm"]').click();
    await modal.waitForDisplayed({ timeout: 5000, reverse: true });

    // The row moved, and so did the one after it.
    const trips = await rpc<Array<Record<string, unknown>>>('get_trips', { vehicleId });
    const a = trips.find((t) => t.id === first.id)!;
    const b = trips.find((t) => t.id === second.id)!;
    expect(a.distanceKm).toBe(61.5);
    expect(a.odometer).toBe(50061.5);
    expect(b.odometer).toBe(50101.5);

    // The map now measures against the new distance, so the deviation is gone.
    await browser.waitUntil(
      async () => (await $('[data-test="target-km"]').getText()).includes('61.5'),
      { timeout: 5000, timeoutMsg: 'the target distance did not follow the write' }
    );
    expect(await $('[data-test="deviation"]').getText()).toContain('0.0');
  });

  it('writes nothing when the confirmation is dismissed', async () => {
    const trip = await seedTrip({
      vehicleId,
      startDatetime: '2026-04-03T08:00',
      endDatetime: '2026-04-03T10:00',
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 50,
      odometer: 50050,
      purpose: 'Business trip',
    });

    await saveRouteWithRoadKm(trip.id as string, 50, 61.5);
    await openMap(trip.id as string);

    await $('[data-test="apply-distance-btn"]').click();
    const modal = await $('[data-testid="cascade-modal"]');
    await modal.waitForDisplayed({ timeout: 5000 });
    await $('[data-testid="cascade-cancel"]').click();
    await modal.waitForDisplayed({ timeout: 5000, reverse: true });

    const trips = await rpc<Array<Record<string, unknown>>>('get_trips', { vehicleId });
    expect(trips.find((t) => t.id === trip.id)!.distanceKm).toBe(50);
  });

  it('names the 20 % legal limit before it is crossed', async () => {
    // 100 km, then 100 km closing on 12 litres: 200 km on 12 l is 6.0
    // l/100km, exactly 20 % over the 5.0 l/100km TP rate. Shortening the
    // first row to 90 km pushes the same 12 litres over 190 km, which is
    // 26.3 % -- over the limit.
    const first = await seedTrip({
      vehicleId,
      startDatetime: '2026-04-04T08:00',
      endDatetime: '2026-04-04T10:00',
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 100,
      odometer: 50100,
      purpose: 'Business trip',
    });
    await seedTrip({
      vehicleId,
      startDatetime: '2026-04-05T08:00',
      endDatetime: '2026-04-05T10:00',
      origin: 'Trnava',
      destination: 'Bratislava',
      distanceKm: 100,
      odometer: 50200,
      purpose: 'Business trip',
      fuelLiters: 12,
      fullTank: true,
    });

    await saveRouteWithRoadKm(first.id as string, 100, 90);
    await openMap(first.id as string);

    await $('[data-test="apply-distance-btn"]').click();
    await $('[data-testid="cascade-modal"]').waitForDisplayed({ timeout: 5000 });

    expect(await $('[data-testid="writeback-margin"]').isDisplayed()).toBe(true);
    expect(await $('[data-testid="writeback-crosses-limit"]').isDisplayed()).toBe(true);
    const marginText = await $('[data-testid="writeback-margin"]').getText();
    expect(marginText).toContain('20.0');
    expect(marginText).toContain('26.3');
  });
});
```

Match the helper names and options of `seedVehicle` / `seedTrip` to what `tests/integration/utils/db.ts` actually exports -- copy the call shapes from `route-map.spec.ts` rather than the field names guessed here, and adjust if `fuelLiters` / `fullTank` are named differently.

- [ ] **Step 6: Run the new spec**

Run:
```bash
npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/route-distance-writeback.spec.ts
```
Expected: PASS, three tests.

- [ ] **Step 7: Run the whole suite once**

Run: `xvfb-run -a -s "-screen 0 1280x1024x24" npm run test:integration`
Expected: PASS. Run this once, at the end -- a full sweep takes about ten minutes, and focused runs are what to iterate with.

- [ ] **Step 8: Commit**

```bash
git add tests/integration/specs/tier2/route-map.spec.ts \
        tests/integration/specs/tier2/route-distance-writeback.spec.ts
git commit -m "test(routes): pin the leg split and the distance write-back"
```

---

### Task 9: Documentation

**Files:**
- Modify: `DECISIONS.md`
- Modify: `docs/features/route-maps.md`
- Modify: `CHANGELOG.md`
- Modify: `README.md` and `README.en.md` (only if they describe the round trip or the map's read-only relationship to the distance -- check with `grep -n "trasy\|round trip" README.md README.en.md` and leave them alone if neither is mentioned)

**Interfaces:**
- Consumes: everything built in Tasks 1 to 8.
- Produces: nothing code depends on.

- [ ] **Step 1: Mark ADR-039 superseded**

`ADR-039` promised this task in its own text. Follow the supersession shape ADR-045 already uses -- a bold line directly under the heading, not a rewrite of the record. In `DECISIONS.md`, immediately after the `### ADR-039: ...` heading:

```markdown
**Superseded by [ADR-048](#adr-048-the-routed-distance-can-be-written-back-behind-the-warning-this-adr-asked-for):** the Decision below is no longer true. `apply_route_distance` writes `distance_km` from a route's road distance. What survives is the Reasoning: the write is never a side effect, it names the consumption period and the 20 % margin it moves before it happens, and it carries the odometer cascade with it. This ADR named "a follow-up task ... carrying exactly that warning" as the intended path; ADR-048 is that task.
```

Also replace the last item of ADR-039's `**Related:**` line -- "the follow-up task that will add a warned distance-reconciliation action" -- with a link to ADR-048.

- [ ] **Step 2: Write the two new ADRs**

`DECISIONS.md` is newest first, so both go at the very top, under a new date heading above `## 2026-09-09: Odometer Cascade On Save`:

```markdown
## 2026-09-09: Round-Trip Legs and Distance Write-Back

### ADR-047: A round trip is two routing requests, one per leg

**Context:** [Task 72](./_tasks/_done/72-route-map-origin-destination/) sent a round trip to OSRM as a single `[origin, destination, origin]` request. Two consequences showed up on real data immediately. OSRM offers alternatives only for a two-point request, so a round trip always showed `alternativesUnavailable` instead of a choice. And a through-route decides the way home for itself, which the user's own point contradicts: the road home is often not the road out. A third consequence was a bug rather than a limitation -- a via dragged onto the return leg landed on the outbound one, because `insert_waypoint` searched the OPEN outbound waypoint list against the CLOSED polyline, so the nearest-vertex match fell through and clamped.

**Decision:** `route_round_trip_internal` issues two `fetch_alternatives` calls, A to B and B to A, and returns each leg's alternatives separately along with `combined[i][j]` -- the road distance, duration, deviation and off-target flag of every pair, computed with the same `deviation()` helper the one-way path uses. The user picks per leg. The saved row stays one row: `save_trip_round_trip_route_internal` joins the two waypoint lists at their shared turnaround point, concatenates the two polylines and sums the two distances, all in Rust, and stores where the join happened in `trip_routes.turnaround_index`. The frontend reports which leg an edit belongs to (`LegInsertPoint.leg`) rather than having Rust infer it from geometry. `route_direct_internal` is unchanged, ADR-041 and all: it is still the one-way path, it is still reachable over RPC, and its symmetric normalisation is still what protects a direct caller.

**Reasoning:** Splitting the request is the only thing that can produce alternatives at all -- it is a property of the routing service, not a design preference. Everything else follows from that. The deviation has to be a property of the PAIR, because the pair is what the trip drove; precomputing all nine combinations costs nothing and keeps the browser from ever adding two distances (ADR-008). The turnaround index is stored rather than inferred because inference is exactly what produced the via bug: two legs cannot be recovered from `[A, B, v, A]` without knowing where the outbound one ended. Legacy rows need no backfill -- the old code appended exactly one clone of the first waypoint, so a `NULL` index means "split at `length - 2`", which is exact rather than a guess. Reporting the leg from the browser is the same call: the ghost handle is attached to one leg's own line, so the browser knows it for certain, and the geometry does not.

**Consequence for the copy:** `alternativesUnavailable` becomes true again. It now appears only for a leg that genuinely passes through an intermediate stop, which is what it always said. A plain round trip no longer triggers it.

**Related:** [Task 78](./_tasks/78-round-trip-legs-and-distance-writeback/); [ADR-038](#adr-038-alternatives-are-ordered-by-duration-deviation-labels-never-reorders) (each leg keeps the service's own order); [ADR-041](#adr-041-round-trip-normalisation-is-symmetric-and-lives-entirely-in-rust) (the one-way normalisation this does not replace); [ADR-040](#adr-040-the-waypoint-editor-is-mode-agnostic); [ADR-008](#adr-008-remove-frontend-calculation-duplication); [docs/features/route-maps.md](./docs/features/route-maps.md).

### ADR-048: The routed distance can be written back, behind the warning this ADR asked for

**Supersedes [ADR-039](#adr-039-distance_km-is-never-rewritten-from-a-routes-road-distance).**

**Context:** ADR-039 banned any route-map write to `distance_km` and named the follow-up that would lift the ban: an explicit action carrying a warning about the consumption period and the legal margin. Real data made the case for it. Trip `32631e0e` records 50.0 km; its one-way route is 25.6 km and its round trip 51.5 km, so the row is a there-and-back written as one line. A second one-way trip showed the same shape independently.

**Decision:** `apply_route_distance` writes `trips.distance_km` from a route's road distance, in two calls. The first is a dry run: it plans the odometer cascade with `plan_odometer_cascade` (the same planner the grid's own save uses, ADR-046) and measures the consumption period with `period_margin_impact`, and it writes nothing. The modal shows the new distance, the period's rate and margin before and after, whether the change crosses the 20 % legal limit, and every row whose odometer moves. The second call writes the row and its cascade in one transaction. It re-plans from the stored book rather than replaying the dry run's numbers.

The write touches three fields -- `distance_km`, `odometer`, `updated_at` -- and no others. It does not go through `update_trip_cascade_internal`, which rebuilds a row from submitted strings and would stamp an `end_datetime` onto a trip that stored none. It does not call `find_or_create_route`: the origin and the destination did not change.

`get_trip_route_internal` now reports `target_km` from the trip rather than from `trip_routes.target_km`. The stored column records what the trip measured when the map was saved, and a target that does not follow the row would show a deviation against a distance the book no longer holds.

**Reasoning:** ADR-039's reasoning was never "this must not be possible" -- it was "this must not be silent". Every clause of that reasoning is now a visible number in the modal: which period moves, what its rate becomes, and which side of the 20 % limit it lands on. Refusing the write in a closed period was considered and rejected: the worked example above IS a closed period, so refusing there refuses the case the feature exists for, and leaves the user hand-editing the same number in the grid with no warning at all. Reusing the cascade planner rather than writing a second one is what keeps a routed distance and a typed distance from meaning different things to the odometer chain.

**Related:** [Task 78](./_tasks/78-round-trip-legs-and-distance-writeback/); [ADR-039](#adr-039-distance_km-is-never-rewritten-from-a-routes-road-distance) (superseded); [ADR-046](#adr-046-a-save-cascades-the-odometer-by-delta-a-rebase-never-runs-on-its-own) (the cascade this reuses); [BIZ-003](#biz-003-legal-margin-limit) (the limit the warning names); [ADR-008](#adr-008-remove-frontend-calculation-duplication).
```

Check the two anchor slugs against how the file's existing cross-links are written, and correct them if the heading text you actually used differs.

- [ ] **Step 3: Update the feature doc**

In `docs/features/route-maps.md`:

1. Delete the section `### Known limitation: a round trip is one three-point request` in full and replace it with a description of the two-leg behaviour, linked to ADR-047. It is no longer a limitation.
2. In the section above it, replace the paragraph promising "A follow-up task ... is expected to add an explicit action for applying a route's distance to its trip" with a description of the Apply button, its modal, and a link to ADR-048.
3. Update numbered point 5 ("A direct route offers alternatives when it has exactly two points") and point 7 ("A direct route can be a round trip") to describe the two pickers and the per-leg message.
4. Add `turnaround_index` to the data-model section beside `round_trip`, with its migration link, and the "NULL means split at `length - 2`" rule.
5. Add `route_round_trip`, `save_trip_round_trip_route` and `apply_route_distance` to the command table, and say which dispatcher each lives in.
6. Update the frontend-state paragraph: it currently lists `alternatives`/`activeIndex`/`roundTrip`; add `roundTripRoutes`, `outboundIndex`, `inboundIndex`, `baseInbound` and `writeback`.
7. Add ADR-047 and ADR-048 to the `## Related` list, and mark ADR-039 there as superseded.
8. Extend the header of `tests/integration/specs/tier2/route-map.spec.ts` -- its numbered list of what is deliberately not covered -- with leg routing, for the same reason as the rest: no provider override exists.

- [ ] **Step 4: Update the changelog**

Run `/changelog`, or write the entries by hand in Slovak, in `[Unreleased]`:

Under `### Pridané`:

```markdown
- **Cesta tam a späť sa počíta ako dve samostatné trasy** - pri zaškrtnutej voľbe "Cesta tam a späť" sa cesta tam a cesta späť hľadajú zvlášť. Pre každú stranu tak dostaneš vlastnú ponuku trás zoradenú od najrýchlejšej a vyberáš si na každej strane samostatne, takže cesta domov môže viesť inou cestou než cesta tam. Odchýlka od zapísaných kilometrov sa počíta z oboch vybraných strán spolu. Na mape je cesta tam modrá a cesta späť oranžová. Medzizastávka pridaná potiahnutím čiary zostane na tej strane, na ktorej si ju pridal.
- **Vzdialenosť trasy vieš zapísať do jazdy** - na mape pribudlo tlačidlo "Použiť vzdialenosť", ktoré zapíše vypočítanú vzdialenosť trasy do počtu kilometrov jazdy. Pred zápisom aplikácia ukáže, čo sa zmení: nový počet kilometrov, spotrebu obdobia pred zmenou a po nej, odchýlku od hodnoty v TP a upozornenie, ak sa obdobie dostane nad zákonný limit 20 %. Rovnako ukáže, o koľko sa posunie stav tachometra všetkých neskorších jázd toho istého roka. Bez potvrdenia sa nezapíše nič.
```

Under `### Opravené`:

```markdown
- **Medzizastávka pridaná na ceste späť skončila na ceste tam** - pri okružnej trase sa nová zastávka umiestňovala vždy do cesty tam, aj keď si ju pridal potiahnutím čiary na ceste späť. Obe strany sa teraz počítajú zvlášť, takže zastávka zostane tam, kam si ju dal.
- **Tlačidlo "Prepočítať" zostalo aktívne aj bez určeného miesta** - ak si dialóg na umiestnenie miesta zavrel klávesom Esc, tlačidlo sa dalo stlačiť, ale nič sa nestalo. Teraz je neaktívne a na stránke sa ukáže, ktoré miesto ešte nemá bod na mape, aj s tlačidlom na jeho umiestnenie.
- **Uložená mapa ukazovala odchýlku voči starej vzdialenosti** - cieľová vzdialenosť sa brala z hodnoty uloženej spolu s mapou, takže po zmene počtu kilometrov jazdy mapa naďalej hlásila odchýlku voči starému číslu. Berie sa teraz priamo z jazdy.
```

- [ ] **Step 5: Verify**

Run: `/verify`
Expected: tests pass, the changelog carries the entries, and `git status` shows nothing unexpected.

- [ ] **Step 6: Commit**

```bash
git add DECISIONS.md docs/features/route-maps.md CHANGELOG.md \
        tests/integration/specs/tier2/route-map.spec.ts
git commit -m "docs(78): record the two-leg round trip and the distance write-back"
```

---

## Self-review

Run after the plan is written, before execution. It found and fixed the following.

**Spec coverage.** Every requirement maps to a task:

| Requirement (01-task.md) | Task |
|---|---|
| Round trip routes A to B and B to A as separate requests | 2 |
| Each leg offers its own alternatives, fastest-first, never reordered | 2 (order), 4 (display) |
| The user can choose per leg | 4 |
| Deviation against the combined road distance of the chosen pair | 2 (`combined[i][j]`), 4 (display) |
| Loop mode untouched | 2 (guard + test), 4 (the loop branch is unchanged) |
| `alternativesUnavailable` only for a genuine intermediate stop | 4 (per-leg gate), 8 (the corrected assertion) |
| Write the routed distance onto `distance_km` from the map | 6, 7 |
| Show the new distance and the effect on the period before writing | 5 (measure), 7 (show), 8 (pin) |
| The warning names the recalculated margin and whether it crosses 20 % | 5, 7, 8 |
| Write-back preserves the odometer invariant | 6 (reuses `plan_odometer_cascade`), 6 tests |
| Nothing is written without explicit confirmation | 6 (`dry_run`), 7 (the modal shows unconditionally), 8 (the dismiss test) |
| ADR-039 superseded, not quietly contradicted | 9 |
| Fix: a via on the return leg lands on the outbound one | 2, by construction; pinned by `a_via_dropped_on_the_return_leg_stays_on_the_return_leg` |
| Fix: `busy` does not account for an unresolved endpoint | 4 (`endpointsMissing`) |

**Open questions.** All four are answered in **Design decisions** above, with the reasoning and with the prior decision each leans on.

**Type consistency.** Checked across tasks: `Leg` is `'outbound' | 'inbound'` in TypeScript and `Leg::Outbound`/`Leg::Inbound` with `#[serde(rename_all = "camelCase")]` in Rust, so the wire values match. `LegRoute` carries no deviation in either language. `turnaround_index` is `Option<i32>` in Rust and `number | null` in TypeScript, on `RouteMap`, `RouteMapRow`, `NewRouteMapRow` and `SavedRouteMap`. `PeriodMarginImpact` and `DistanceWriteback` have identical field sets on both sides. `combined` is `Vec<Vec<CombinedLeg>>` / `CombinedLeg[][]`, indexed `[outbound][inbound]` everywhere.

**Two corrections made while reviewing:**

1. The first draft routed write-back through `update_trip_cascade_internal`. That would have stamped an `end_datetime` onto every trip that stored `None`, because `build_updated_trip` writes `Some(parse(...))` unconditionally. Task 6 builds the row from the stored one instead, and `test_apply_route_distance_does_not_invent_an_end_time` pins it. **No stored row is affected today** -- `create_trip`, `update_trip` and `update_trip_cascade` all take `end_datetime: String`, so nothing can write a NULL, and the production copy holds none (329 rows, 0 null, measured 2026-09-09). The guard keeps the property true rather than repairing a live fault, and the wider point -- that no RPC caller can express "this trip has no end time" -- is out of scope here.
2. The first draft left `trip_routes.target_km` as the map's target. After a write-back it would have shown a deviation against a distance the book no longer held. Task 3 reads the target from the trip; `a_saved_map_reports_the_trips_distance_as_its_target` pins it.

**Three further corrections, from the review of this plan:**

3. `currentWaypoints()` fell through to `savedRoute.waypoints` -- the CLOSED
   list -- whenever `runDirect`'s catch had nulled `baseWaypoints`. Sent with
   `round_trip: false`, ADR-041's normaliser strips one trailing point off it.
   That recovered the outbound leg before this task, because a via could never
   sit after the turnaround; post-split it turns a saved `[A, B, v, A]` into
   the one-way route `A -> B -> v`, moving the return leg's via onto the way
   out. Task 4 adds `savedLegs` and puts `savedLegs?.outbound` ahead of
   `savedRoute?.waypoints` in both fallback chains.
4. `splitSavedLegs` carried the legacy `?? length - 2` rule in Svelte -- an
   inference about persisted data, in the one file that has leaked a stale
   shape three times, and against ADR-047's own argument for storing the index.
   Task 3 resolves it in `get_trip_route_internal` instead, with two Rust
   tests, so the browser only ever slices a list.
5. The Apply button was placed outside the mode branches, so a loop route would
   have got it. A loop's road distance is the genetic algorithm's approximation
   of the trip's own recorded distance, so writing it back is circular. Task 7
   gates the button on `mode === 'direct'` and says why.

**One deliberate addition beyond the spec:** drawing the two legs in different colours. "Choose per leg" is not usable when the legs are indistinguishable on the map. It stops there -- no leg hover highlighting, no on-map leg labels.

**One deliberate non-change:** `route_direct_internal` keeps its `round_trip` parameter and its ADR-041 normalisation, with every regression test. The map view no longer sends `round_trip: true` to it, but the command is reachable over `POST /api/rpc`, and its normalisation is what protects a direct caller from an unroutable list. ADR-047 says so explicitly, so a reviewer does not read it as something the task forgot.
