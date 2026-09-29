# Save And Apply Distance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Date:** 2026-09-29
**Subject:** One "Uložiť a použiť vzdialenosť" button for every route mode
**Status:** Complete

**Goal:** A map save always writes the route's whole-km distance to the trip, in one transaction, behind the existing dry-run modal, in every route mode.

**Architecture:** The two save commands get a `dry_run` flag and return a `DistanceWriteback`. They share one planner with a new saved-map sync command, and one new DB method writes the map row, the trip row and the odometer shifts in one transaction. The frontend keeps a copy of the save call from the click, runs the dry run, shows the modal only if the trip changes, and sends the same call again on confirm.

**Tech Stack:** Rust (`kniha-jazd-core`, diesel, SQLite), Axum JSON-RPC dispatcher, SvelteKit 5 (runes), typesafe-i18n, WebdriverIO.

**Spec:** [01-task.md](./01-task.md)

## Global Constraints

- ADR-008: all calculation and all "in sync" / "changes the trip" decisions are in Rust. The frontend only reads flags.
- Written km: `road_km.round()` (whole km, half away from zero). The stored `trip_routes.road_km` stays raw.
- Map row + trip row + odometer shifts commit in ONE transaction, or nothing commits.
- A dry run writes nothing and works in read-only mode. A commit calls `check_read_only!`.
- Every RPC that can write takes a required `dryRun` (no serde default).
- All UI text goes through i18n (`sk` and `en`), then `npm run i18n`.
- Only keyboard-typable characters in code, comments and docs (no em-dash, no arrow glyphs).
- Stage only the files of this task. Never `git add -A`.

## Review Focus

1. **Read-only database.** The dry run answers, the commit is refused, and no map row and no km are written. Test: Task 3, `save_and_apply_is_read_only_guarded`.
2. **The book moves between the dry run and Confirm** (a second tab edits the trip). The commit plans again from the stored book, not from the dry-run numbers. Test: Task 3, `commit_replans_from_the_book_not_from_the_dry_run`.
3. **Half km.** 25.5 km becomes 26, and 25.49 km becomes 25. Test: Task 2, `logbook_km_rounds_half_away_from_zero`.
4. **A shift fails inside the commit** (a later row was deleted after the plan). The whole transaction rolls back and the map is not saved. Test: Task 1, `save_route_map_with_trip_distance_rolls_back_on_a_bad_shift`.
5. **Sync on a trip with no saved map.** The command returns an error and writes nothing. Test: Task 2, `apply_saved_route_distance_needs_a_saved_map`.

---

### Task 1: One transaction for the map, the trip and the shifts

**Files:**
- Modify: [src-tauri/core/src/db.rs](../../src-tauri/core/src/db.rs) (`apply_odometer_shifts` near line 519, `update_trip_with_odometer_shift` near line 548, `save_route_map` near line 1095)
- Test: [src-tauri/core/src/db_tests.rs](../../src-tauri/core/src/db_tests.rs)

**Interfaces:**
- Produces: `Database::save_route_map_with_trip_distance(&self, map: &RouteMap, trip: Option<&Trip>, shifts: &[(String, f64)]) -> QueryResult<()>`
- Produces (private): `Database::insert_route_map_tx(tx: &mut SqliteConnection, map: &RouteMap) -> QueryResult<()>`, `Database::update_trip_tx(tx: &mut SqliteConnection, trip: &Trip, updated_at: &str) -> QueryResult<()>`

- [ ] **Step 1: Write the failing tests** in `db_tests.rs`. Reuse the fixtures that `save_route_map_replaces_existing` (line 1042) already uses to build a `RouteMap` and a trip.

```rust
#[test]
fn save_route_map_with_trip_distance_writes_all_three_together() {
    let db = Database::in_memory().unwrap();
    let (vehicle, mut a, b) = seed_two_chained_trips(&db); // a: 50 km @ 50050, b: 70 km @ 50120
    let map = sample_route_map(a.id, 61.0);

    a.distance_km = 61.0;
    a.odometer = 50061.0;
    a.updated_at = Utc::now();
    db.save_route_map_with_trip_distance(&map, Some(&a), &[(b.id.to_string(), 50131.0)])
        .unwrap();

    assert!(db.get_route_map(&a.id.to_string()).unwrap().is_some());
    assert_eq!(db.get_trip(&a.id.to_string()).unwrap().unwrap().distance_km, 61.0);
    assert_eq!(db.get_trip(&b.id.to_string()).unwrap().unwrap().odometer, 50131.0);
    let _ = vehicle;
}

#[test]
fn save_route_map_with_trip_distance_saves_the_map_alone_when_the_trip_does_not_change() {
    let db = Database::in_memory().unwrap();
    let (_vehicle, a, _b) = seed_two_chained_trips(&db);
    let map = sample_route_map(a.id, 50.0);

    db.save_route_map_with_trip_distance(&map, None, &[]).unwrap();

    assert!(db.get_route_map(&a.id.to_string()).unwrap().is_some());
    assert_eq!(db.get_trip(&a.id.to_string()).unwrap().unwrap().distance_km, 50.0);
}

#[test]
fn save_route_map_with_trip_distance_rolls_back_on_a_bad_shift() {
    let db = Database::in_memory().unwrap();
    let (_vehicle, mut a, _b) = seed_two_chained_trips(&db);
    let map = sample_route_map(a.id, 61.0);
    a.distance_km = 61.0;

    let result = db.save_route_map_with_trip_distance(
        &map,
        Some(&a),
        &[("00000000-0000-0000-0000-000000000000".to_string(), 1.0)],
    );

    assert!(result.is_err());
    assert!(db.get_route_map(&a.id.to_string()).unwrap().is_none(), "no map on a failed commit");
    assert_eq!(db.get_trip(&a.id.to_string()).unwrap().unwrap().distance_km, 50.0);
}
```

If `db_tests.rs` has no `seed_two_chained_trips` / `sample_route_map`, add them at the top of the new block: a vehicle with `initial_odometer = 50000.0`, trip `a` on 2026-04-01 (50 km, odometer 50050), trip `b` on 2026-04-02 (70 km, odometer 50120), and a `RouteMap` for `a` with two waypoints, the polyline `"_p~iF~ps|U"`, `mode: RouteMode::Direct`, `round_trip: false`, `turnaround_index: None`, `avoid: vec![]`, `provider: None`, and the given `road_km` as both `road_km` and `target_km`.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core save_route_map_with_trip_distance`
Expected: FAIL, compile error `no method named save_route_map_with_trip_distance`.

- [ ] **Step 3: Implement.** Move the insert body of `save_route_map` into `insert_route_map_tx`, and the `diesel::update(trips...)` body of `update_trip_with_odometer_shift` into `update_trip_tx`. Both public methods keep their behavior and call the helpers. Add the new method:

```rust
/// Save a route map and write the distance it implies, in one transaction
/// (task 87). `trip` is `None` when the rounded road distance already equals
/// the trip's: the map is saved and the trip row is not touched. A shift
/// naming an unknown row rolls back the map too -- a saved map whose distance
/// did not reach the trip is the state this method exists to prevent.
pub fn save_route_map_with_trip_distance(
    &self,
    map: &RouteMap,
    trip: Option<&Trip>,
    shifts: &[(String, f64)],
) -> QueryResult<()> {
    let conn = &mut *self.conn.lock().unwrap();
    conn.transaction::<_, diesel::result::Error, _>(|tx| {
        Self::insert_route_map_tx(tx, map)?;
        if let Some(trip) = trip {
            let updated_at = trip.updated_at.to_rfc3339();
            Self::update_trip_tx(tx, trip, &updated_at)?;
            Self::apply_odometer_shifts(tx, shifts, &updated_at)?;
        }
        Ok(())
    })
}
```

`insert_route_map_tx` must do the delete + insert that `save_route_map` does today (it serializes `waypoints` and `avoid` itself and maps a serde error to `diesel::result::Error::SerializationError`).

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core db_tests`
Expected: PASS, including the old `save_route_map_replaces_existing` and the cascade tests.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/core/src/db.rs src-tauri/core/src/db_tests.rs
git commit -m "feat(db): save a route map and its trip distance in one transaction"
```

---

### Task 2: Shared planner, whole-km rounding, saved-map sync

**Files:**
- Modify: [src-tauri/core/src/commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) (`apply_route_distance_internal` at line 748, its doc comment from line 730)
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) (`DistanceWriteback` at line 765)
- Test: [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) (the `test_apply_route_distance_*` block from line 5158)

**Interfaces:**
- Consumes: `Database::save_route_map_with_trip_distance` (Task 1)
- Produces:
  - `pub(crate) fn logbook_km(road_km: f64) -> f64`
  - `pub(crate) struct RouteDistancePlan { pub existing: Trip, pub plan: CascadePlan, pub margin: PeriodMarginImpact }`
  - `impl RouteDistancePlan { pub fn changes_trip(&self) -> bool; pub fn updated_trip(&self) -> Option<Trip>; pub fn shifts(&self) -> Vec<(String, f64)>; pub fn into_writeback(self, saved: Option<Trip>) -> DistanceWriteback }`
  - `pub(crate) fn plan_route_distance(db: &Database, trip_id: &str, road_km: f64) -> Result<RouteDistancePlan, String>`
  - `pub fn apply_saved_route_distance_internal(db: &Database, app_state: &AppState, trip_id: String, dry_run: bool) -> Result<DistanceWriteback, String>`
  - `DistanceWriteback` gains `pub changes_trip: bool` (serde: `changesTrip`)
  - `apply_route_distance_internal` is REMOVED.

- [ ] **Step 1: Write the failing tests.** Replace the `test_apply_route_distance_*` tests with these. They keep every property the old tests pinned (no end time invented, margin reported, read-only guard) and add the new ones. `save_map(db, trip_id, road_km)` is a local helper that calls `db.save_route_map(&RouteMap { .. })` directly, so this task does not depend on Task 3.

```rust
#[test]
fn logbook_km_rounds_half_away_from_zero() {
    assert_eq!(logbook_km(25.634), 26.0);
    assert_eq!(logbook_km(25.5), 26.0);
    assert_eq!(logbook_km(25.49), 25.0);
    assert_eq!(logbook_km(0.4), 0.0);
}

#[test]
fn plan_route_distance_rounds_before_it_plans() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);

    let p = plan_route_distance(&db, &a.to_string(), 61.4).unwrap();

    assert_eq!(p.plan.new_distance_km, 61.0);
    assert!((p.plan.delta - 11.0).abs() < 1e-9);
    assert!(p.changes_trip());
}

#[test]
fn plan_route_distance_is_a_no_op_when_the_rounded_km_already_matches() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);

    let p = plan_route_distance(&db, &a.to_string(), 50.3).unwrap();

    assert!(!p.changes_trip());
    assert!(p.plan.changes.is_empty());
    assert!(p.updated_trip().is_none());
}

#[test]
fn apply_saved_route_distance_dry_run_writes_nothing() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);
    save_map(&db, a, 61.5);

    let r = apply_saved_route_distance_internal(&db, &app_state, a.to_string(), true).unwrap();

    assert!(r.trip.is_none());
    assert!(r.changes_trip);
    assert_eq!(r.distance_before, 50.0);
    assert_eq!(r.distance_after, 62.0, "61.5 rounds to 62");
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().distance_km, 50.0);
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().odometer, 50120.0);
}

#[test]
fn apply_saved_route_distance_writes_the_row_and_shifts_the_rest() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);
    save_map(&db, a, 61.5);

    apply_saved_route_distance_internal(&db, &app_state, a.to_string(), false).unwrap();

    let written = db.get_trip(&a.to_string()).unwrap().unwrap();
    assert_eq!(written.distance_km, 62.0);
    assert_eq!(written.odometer, 50062.0);
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().odometer, 50132.0);
    // The map row is not rewritten: its road km stays the raw routed value.
    assert_eq!(db.get_route_map(&a.to_string()).unwrap().unwrap().road_km, 61.5);
}

#[test]
fn apply_saved_route_distance_needs_a_saved_map() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);

    let err = apply_saved_route_distance_internal(&db, &app_state, a.to_string(), true)
        .unwrap_err();

    assert!(err.contains("no saved route map"), "{err}");
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().distance_km, 50.0);
}
```

Keep `test_apply_route_distance_does_not_invent_an_end_time`, `..._reports_the_margin_it_would_cause` and `..._is_read_only_guarded`. Rename them to `apply_saved_route_distance_*`, add `save_map(&db, a, <km>)` before the call, and call `apply_saved_route_distance_internal(&db, &app_state, a.to_string(), <dry_run>)`. In the margin test, save the map with `road_km = 90.0`.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core apply_saved_route_distance logbook_km plan_route_distance`
Expected: FAIL, compile errors for the missing functions.

- [ ] **Step 3: Implement** in `trips.rs`. Move the body of `apply_route_distance_internal` up to and including `period_margin_impact` into `plan_route_distance`, and keep its doc comment (the three-fields and the no-`end_datetime` reasons) on it.

```rust
/// Whole km: what a routed distance becomes in the logbook (task 87, D4).
/// The map keeps the raw value; only the trip's `distance_km` is rounded.
pub(crate) fn logbook_km(road_km: f64) -> f64 {
    road_km.round()
}

pub(crate) struct RouteDistancePlan {
    pub existing: Trip,
    pub plan: CascadePlan,
    pub margin: PeriodMarginImpact,
}

impl RouteDistancePlan {
    /// False when the rounded road km equals the stored km. The page then
    /// saves without the modal (task 87, D3).
    pub fn changes_trip(&self) -> bool {
        (self.plan.new_distance_km - self.existing.distance_km).abs() > CASCADE_EPSILON
    }

    /// The row to write: three fields move, nothing else. `None` for a no-op.
    pub fn updated_trip(&self) -> Option<Trip> {
        self.changes_trip().then(|| Trip {
            distance_km: self.plan.new_distance_km,
            odometer: self.plan.new_odometer,
            updated_at: Utc::now(),
            ..self.existing.clone()
        })
    }

    pub fn shifts(&self) -> Vec<(String, f64)> {
        self.plan.changes.iter().map(|c| (c.trip_id.clone(), c.new_odometer)).collect()
    }

    pub fn into_writeback(self, saved: Option<Trip>) -> DistanceWriteback {
        DistanceWriteback {
            trip_id: self.existing.id.to_string(),
            distance_before: self.existing.distance_km,
            distance_after: self.plan.new_distance_km,
            changes_trip: self.changes_trip(),
            plan: self.plan,
            margin: self.margin,
            trip: saved,
        }
    }
}

pub(crate) fn plan_route_distance(
    db: &Database,
    trip_id: &str,
    road_km: f64,
) -> Result<RouteDistancePlan, String> {
    if !road_km.is_finite() || road_km < 0.0 {
        return Err(format!(
            "Road distance {road_km} is not a distance that can be written to a trip"
        ));
    }
    let km = logbook_km(road_km);
    // ... the existing lookup of the trip, vehicle, year trips and year start,
    // then plan_odometer_cascade(&trips, year_start, trip_id, km, existing.odometer),
    // mark_next_year_chain_breaks, period_margin_impact(.., plan.new_distance_km)
    Ok(RouteDistancePlan { existing, plan, margin })
}

/// Write a SAVED map's road distance onto its trip (task 87, D2): the map was
/// saved before task 87, or the trip's km was edited after the save. The road
/// km is read from `trip_routes`, never taken from the browser.
pub fn apply_saved_route_distance_internal(
    db: &Database,
    app_state: &AppState,
    trip_id: String,
    dry_run: bool,
) -> Result<DistanceWriteback, String> {
    let map = db
        .get_route_map(&trip_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip {trip_id} has no saved route map"))?;
    let p = plan_route_distance(db, &trip_id, map.road_km)?;
    if dry_run {
        return Ok(p.into_writeback(None));
    }
    check_read_only!(app_state);
    let Some(trip) = p.updated_trip() else {
        return Ok(p.into_writeback(None));
    };
    db.update_trip_with_odometer_shift(&trip, &p.shifts())
        .map_err(|e| e.to_string())?;
    Ok(p.into_writeback(Some(trip)))
}
```

In `models.rs`, add `pub changes_trip: bool` to `DistanceWriteback` with a doc line: `/// False when the write would change nothing (task 87). The page skips the modal.` Delete `apply_route_distance_internal`, and remove it from the `pub use` in `commands_internal/mod.rs` if it is listed there. Its RPC arm stops compiling. Task 4 replaces the arm. To keep this task green, change the arm to call `apply_saved_route_distance_internal` now and rename it in Task 4.

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/core/src/commands_internal/trips.rs src-tauri/core/src/commands_internal/commands_tests.rs src-tauri/core/src/models.rs src-tauri/core/src/commands_internal/mod.rs src-tauri/core/src/server/dispatcher.rs
git commit -m "feat(trips): plan a route distance in whole km, sync a saved map to its trip"
```

---

### Task 3: The save commands write the distance (every mode)

**Files:**
- Modify: [src-tauri/core/src/commands_internal/route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs) (`SavedRouteMap` near line 98, `From<RouteMap>` near line 135, `persist_route_map` at line 680, `save_trip_route_internal` at line 729, `save_trip_round_trip_route_internal` at line 756)
- Test: [src-tauri/core/src/commands_internal/route_maps_tests.rs](../../src-tauri/core/src/commands_internal/route_maps_tests.rs) (27 call sites of the two save functions)

**Interfaces:**
- Consumes: `plan_route_distance`, `logbook_km`, `RouteDistancePlan` (Task 2), `save_route_map_with_trip_distance` (Task 1)
- Produces:
  - `save_trip_route_internal(db, app_state, trip_id: String, waypoints: Vec<Waypoint>, polyline: String, road_km: f64, mode: RouteMode, round_trip: bool, avoid: Vec<String>, provider: Option<RouteProviderKind>, dry_run: bool) -> Result<DistanceWriteback, String>` (the `target_km` parameter is REMOVED: the stored target is the committed trip km)
  - `save_trip_round_trip_route_internal(db, app_state, trip_id, outbound_waypoints, inbound_waypoints, outbound_polyline, inbound_polyline, outbound_road_km, inbound_road_km, avoid, provider, dry_run: bool) -> Result<DistanceWriteback, String>` (`target_km` REMOVED)
  - `SavedRouteMap.distance_in_sync: bool` (serde: `distanceInSync`)

- [ ] **Step 1: Write the failing tests** in `route_maps_tests.rs`. `seed_trip` (line 53) gives a 120 km trip.

```rust
#[test]
fn a_dry_run_save_writes_neither_the_map_nor_the_km() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();

    let r = save_trip_route_internal(
        &db, &app_state, trip.id.to_string(), sample_waypoints(), polyline,
        133.7, RouteMode::Direct, false, vec![], None, true,
    )
    .unwrap();

    assert!(r.changes_trip);
    assert_eq!(r.distance_after, 134.0);
    assert!(db.get_route_map(&trip.id.to_string()).unwrap().is_none());
    assert_eq!(db.get_trip(&trip.id.to_string()).unwrap().unwrap().distance_km, 120.0);
}

#[test]
fn a_committed_direct_save_writes_the_map_and_the_whole_km() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();

    save_trip_route_internal(
        &db, &app_state, trip.id.to_string(), sample_waypoints(), polyline,
        133.7, RouteMode::Direct, false, vec![], None, false,
    )
    .unwrap();

    let map = db.get_route_map(&trip.id.to_string()).unwrap().unwrap();
    assert_eq!(map.road_km, 133.7, "the map keeps the raw road km");
    assert_eq!(map.target_km, 134.0, "the stored target is the committed trip km");
    assert_eq!(db.get_trip(&trip.id.to_string()).unwrap().unwrap().distance_km, 134.0);
}

#[test]
fn a_committed_loop_save_writes_the_km_too() {
    // Before task 87 a loop never wrote the distance (route-maps.md called it
    // circular). D1: every mode writes it.
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();

    save_trip_route_internal(
        &db, &app_state, trip.id.to_string(), sample_waypoints(), polyline,
        118.4, RouteMode::Loop, false, vec![], None, false,
    )
    .unwrap();

    assert_eq!(db.get_trip(&trip.id.to_string()).unwrap().unwrap().distance_km, 118.0);
}

#[test]
fn a_committed_round_trip_save_writes_the_rounded_sum_of_its_legs() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();
    let out = sample_waypoints();
    let back: Vec<Waypoint> = out.iter().rev().cloned().collect();

    save_trip_round_trip_route_internal(
        &db, &app_state, trip.id.to_string(), out, back,
        polyline.clone(), polyline, 60.3, 61.4, vec![], None, false,
    )
    .unwrap();

    assert_eq!(db.get_trip(&trip.id.to_string()).unwrap().unwrap().distance_km, 122.0);
}

#[test]
fn a_save_that_changes_nothing_still_saves_the_map() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();

    let r = save_trip_route_internal(
        &db, &app_state, trip.id.to_string(), sample_waypoints(), polyline,
        120.3, RouteMode::Direct, false, vec![], None, false,
    )
    .unwrap();

    assert!(!r.changes_trip);
    assert!(r.trip.is_none());
    assert!(db.get_route_map(&trip.id.to_string()).unwrap().is_some());
}

#[test]
fn save_and_apply_is_read_only_guarded() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();
    app_state.enable_read_only("newer migrations");

    let save = |dry_run| {
        save_trip_route_internal(
            &db, &app_state, trip.id.to_string(), sample_waypoints(), polyline.clone(),
            133.7, RouteMode::Direct, false, vec![], None, dry_run,
        )
    };

    assert!(save(true).is_ok(), "a dry run only reads");
    assert!(save(false).is_err());
    assert!(db.get_route_map(&trip.id.to_string()).unwrap().is_none());
    assert_eq!(db.get_trip(&trip.id.to_string()).unwrap().unwrap().distance_km, 120.0);
}

#[test]
fn commit_replans_from_the_book_not_from_the_dry_run() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let mut trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();
    let save = |dry_run| {
        save_trip_route_internal(
            &db, &app_state, trip.id.to_string(), sample_waypoints(), polyline.clone(),
            133.7, RouteMode::Direct, false, vec![], None, dry_run,
        )
    };

    let dry = save(true).unwrap();
    assert_eq!(dry.distance_before, 120.0);

    // Another tab edits the trip between the dry run and Confirm.
    trip.distance_km = 100.0;
    db.update_trip(&trip).unwrap();

    let done = save(false).unwrap();
    assert_eq!(done.distance_before, 100.0, "the commit reads the book as it is now");
    assert_eq!(db.get_trip(&trip.id.to_string()).unwrap().unwrap().distance_km, 134.0);
}

#[test]
fn a_map_in_sync_reports_it_and_is_never_off_target() {
    // 2.4 km written as 2 km is a 20 % deviation, over TOLERANCE (5 %). An
    // in-sync map must not show a warning the user can never clear.
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();
    save_trip_route_internal(
        &db, &app_state, trip.id.to_string(), sample_waypoints(), polyline,
        2.4, RouteMode::Direct, false, vec![], None, false,
    )
    .unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();
    assert!(saved.distance_in_sync);
    assert!(!saved.off_target);
    assert!(saved.deviation_percent > 19.0, "the real deviation is still reported");
}

#[test]
fn a_map_whose_trip_km_was_edited_later_is_not_in_sync() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let mut trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();
    save_trip_route_internal(
        &db, &app_state, trip.id.to_string(), sample_waypoints(), polyline,
        120.0, RouteMode::Direct, false, vec![], None, false,
    )
    .unwrap();

    trip.distance_km = 100.0;
    db.update_trip(&trip).unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();
    assert!(!saved.distance_in_sync);
}
```

Then update the 27 existing call sites: remove the `target_km` argument and add `false` as the last argument (`dry_run`). Where a test asserts `target_km` or the trip km after a save whose road km differs from 120, assert the new committed km. Example: `a_saved_map_reports_the_trips_distance_as_its_target` (line 1936) now needs no `db.update_trip`, because the save itself writes 118.

A save now plans against the trip's year, so the trip and its vehicle must be in the database. If a test saves a map for a trip id that was never inserted, it now fails with `Trip not found`. Seed the trip with `seed_trip` in that test. Do not weaken the planner.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core route_maps_tests`
Expected: FAIL, compile errors (argument count, `changes_trip`, `distance_in_sync`).

- [ ] **Step 3: Implement.** Split `persist_route_map` into a pure builder and a commit step. The builder keeps the validation (avoid list, uuid) and the stamps, and it does NOT call `check_read_only!`.

```rust
/// Builds the row. No read-only check: a dry run builds it too, to validate
/// the payload, and throws it away.
#[allow(clippy::too_many_arguments)]
fn build_route_map(
    trip_id: &str,
    waypoints: Vec<Waypoint>,
    polyline: String,
    road_km: f64,
    mode: RouteMode,
    round_trip: bool,
    turnaround_index: Option<i32>,
    avoid: Vec<String>,
    provider: Option<RouteProviderKind>,
) -> Result<RouteMap, String> {
    // body of persist_route_map without check_read_only! and without the
    // db call; target_km is set to road_km here and overwritten on commit
}

/// Task 87: a map save always writes the distance it implies. Dry run: plan
/// only. Commit: map row + trip row + odometer shifts in one transaction.
fn save_route_and_distance(
    db: &Database,
    app_state: &AppState,
    mut map: RouteMap,
    dry_run: bool,
) -> Result<DistanceWriteback, String> {
    let trip_id = map.trip_id.to_string();
    let p = plan_route_distance(db, &trip_id, map.road_km)?;
    if dry_run {
        return Ok(p.into_writeback(None));
    }
    check_read_only!(app_state);
    map.target_km = p.plan.new_distance_km;
    let trip = p.updated_trip();
    db.save_route_map_with_trip_distance(&map, trip.as_ref(), &p.shifts())
        .map_err(|e| e.to_string())?;
    Ok(p.into_writeback(trip))
}
```

`save_trip_route_internal` and `save_trip_round_trip_route_internal` call `build_route_map(...)`, then `save_route_and_distance(db, app_state, map, dry_run)`. The round trip passes `outbound_road_km + inbound_road_km` as `road_km`, as today.

In `SavedRouteMap`, add the field with a doc comment, and in `From<RouteMap>`:

```rust
// `target_km` is the trip's km here: get_trip_route_internal overwrote it.
let distance_in_sync = (logbook_km(map.road_km) - map.target_km).abs() < 0.001;
let (deviation_percent, off_target) = deviation(map.target_km, map.road_km);
// An in-sync map is the logbook's own number: never flag it (task 87, R4).
let off_target = off_target && !distance_in_sync;
```

Update the module doc at the top of `route_maps.rs` (line 3, "Generating and saving are deliberately separate") only if it says that a save never touches the trip.

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core`
Expected: PASS. The dispatcher still fails to compile until Task 4. If it does, do Task 4 Step 3 first and run the tests again.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/core/src/commands_internal/route_maps.rs src-tauri/core/src/commands_internal/route_maps_tests.rs
git commit -m "feat(route-map): a map save writes its whole-km distance to the trip in every mode"
```

---

### Task 4: RPC arms

**Files:**
- Modify: [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) (`apply_route_distance` arm at line 270, `save_trip_route` at line 849, `save_trip_round_trip_route` at line 891, the dispatcher tests near lines 1204 and 1236)

**Interfaces:**
- Consumes: the Task 2 and Task 3 signatures
- Produces (wire, camelCase):
  - `save_trip_route { tripId, waypoints, polyline, roadKm, mode, roundTrip?, avoid?, provider?, dryRun }` returns `DistanceWriteback`
  - `save_trip_round_trip_route { tripId, outboundWaypoints, inboundWaypoints, outboundPolyline, inboundPolyline, outboundRoadKm, inboundRoadKm, avoid?, provider?, dryRun }` returns `DistanceWriteback`
  - `apply_saved_route_distance { tripId, dryRun }` returns `DistanceWriteback`
  - `apply_route_distance` is REMOVED

- [ ] **Step 1: Write the failing test** next to the existing `save_trip_route` dispatcher test (line 1204):

```rust
#[tokio::test]
async fn save_trip_route_without_dry_run_is_refused() {
    // dryRun has no default: a client that forgets it must not write the
    // trip's km by accident.
    let (state, trip_id) = state_with_trip().await; // same setup as the test at line 1204
    let err = dispatch(
        "save_trip_route",
        json!({
            "tripId": trip_id,
            "waypoints": [{ "lat": 48.935, "lon": 20.553, "name": "Domov", "nodeIdx": 0 }],
            "polyline": "_p~iF~ps|U",
            "roadKm": 118.4,
            "mode": "loop",
        }),
        &state,
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("dryRun"), "{err}");
}
```

Use the helper and the `dispatch` call form that the test at line 1204 already uses. If they differ from the names above, copy that test's setup exactly.

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core dispatcher`
Expected: FAIL (the arm accepts the payload, or a compile error from Task 3).

- [ ] **Step 3: Implement.** In both save arms: remove `target_km` from `Args`, add `dry_run: bool` with no `#[serde(default)]`, pass `a.dry_run`, and return `Ok(serde_json::to_value(v).unwrap())` with `v` the `DistanceWriteback`. Serde ignores unknown fields, so an old client that still sends `targetKm` does not fail on it. Replace the `apply_route_distance` arm:

```rust
"apply_saved_route_distance" => {
    // Trip id and dry run only: the road km comes from trip_routes, not from
    // the browser (task 87, D2).
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        trip_id: String,
        dry_run: bool,
    }
    let a: Args = parse_args(args)?;
    let v = crate::commands_internal::apply_saved_route_distance_internal(
        &state.db,
        &state.app_state,
        a.trip_id,
        a.dry_run,
    )?;
    Ok(serde_json::to_value(v).unwrap())
}
```

Update the two existing dispatcher tests (line 1204, line 1236): remove `targetKm`, add `"dryRun": false`, and fix any assert on the unit `()` result.

- [ ] **Step 4: Run the whole backend suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/core/src/server/dispatcher.rs
git commit -m "feat(rpc): dryRun on both route saves, apply_saved_route_distance replaces apply_route_distance"
```

---

### Task 5: One button on `/mapa`

**Files:**
- Modify: [src/lib/types.ts](../../src/lib/types.ts) (`DistanceWriteback` line 121, `RouteMap` line 556)
- Modify: [src/lib/api.ts](../../src/lib/api.ts) (`applyRouteDistance` line 152, `saveTripRoute` line 605, `saveTripRoundTripRoute` line 632)
- Modify: [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts), [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts) (`routeMap` block)
- Modify: [src/routes/mapa/+page.svelte](../../src/routes/mapa/+page.svelte) (`handleSave` line 1165, `handleApplyDistance` line 1211, `confirmWriteback` line 1229, `busy` line 280, toolbar buttons lines 1377 to 1396, modal line 1573)
- Test: [tests/integration/specs/tier2/route-distance-writeback.spec.ts](../../tests/integration/specs/tier2/route-distance-writeback.spec.ts) (rewrite), [tests/integration/specs/tier2/route-map.spec.ts](../../tests/integration/specs/tier2/route-map.spec.ts) (fixtures and two `save-btn` clicks)

**Interfaces:**
- Consumes: the Task 4 wire contract
- Produces: `data-test="save-apply-btn"`; the `saved-notice` element and the `route-map-saved` announcement stay; the `apply-distance-btn` and `save-btn` test ids are REMOVED

- [ ] **Step 1: Update the fixtures.** In `route-map.spec.ts`, in every `rpc('save_trip_route' | 'save_trip_round_trip_route', ...)` (lines 114, 133, 157, 178, 203, 658): remove `targetKm` and add `dryRun: false`. Every fixture sends `roadKm` equal to the trip km, so the save changes no trip. Check each one while you edit it. In the two tests that click `save-btn` (`routes again with the avoid value when a country is checked` and `switches provider, clears the avoid list for OSRM, and keeps the saved provider on reopen`), click `save-apply-btn`, then confirm the modal: the shown route (120 km and 100 km, both on Sygic) differs from the 65 km trip.

```ts
await $('[data-test="save-apply-btn"]').click();
await $('[data-testid="cascade-modal"]').waitForDisplayed({ timeout: 5000 });
await $('[data-testid="cascade-confirm"]').click();
await $('[data-test="saved-notice"]').waitForDisplayed();
```

- [ ] **Step 2: Rewrite `route-distance-writeback.spec.ts`.** Keep `openMap`, the vehicle `beforeEach` and the imports. Replace `saveRouteWithRoadKm` with a fixture that saves IN SYNC, and add a helper for the legacy state:

```ts
/** Persist a one-way direct route whose road km equals the trip km (no trip change). */
async function saveSyncedRoute(tripId: string, km: number) {
  await rpc('save_trip_route', {
    tripId,
    waypoints: CANNED_WAYPOINTS,
    polyline: CANNED_POLYLINE,
    roadKm: km,
    mode: 'direct',
    roundTrip: false,
    dryRun: false,
  });
}

/** A map saved before task 87, or a trip km edited after the save. */
async function editTripKm(trip: Record<string, unknown>, distanceKm: number) {
  await updateTrip({ ...trip, id: trip.id, distanceKm });
}
```

The tests. Since commit `6ff0bfb` the default provider is OSRM, also with a key (ADR-053). The mock router returns 90.0 km for OSRM, 100.0 km for Sygic without avoid, and 120.0 km for Sygic with avoid (`MockRouteProvider` in [provider.rs](../../src-tauri/core/src/route_map/provider.rs)). A route with no provider routes as OSRM, so each `recalculate-btn` click below gives 90.0 km:

1. `saves the map and writes its distance, shifting the later rows`: trip A 50 km @ 50050, trip B 40 km @ 50090. `saveSyncedRoute(A, 50)`, open the map, click `recalculate-btn`, wait for `actual-km` = `90.0 km`, click `save-apply-btn`. The modal summary contains `50` and `90`. Confirm. Expect A `distanceKm` 90, A `odometer` 50090, B `odometer` 50130, `get_trip_route(A).roadKm` 90, and `deviation` shows `0.0`.
2. `writes nothing when the confirmation is dismissed`: same setup. Cancel. Expect A `distanceKm` 50 and `get_trip_route(A).roadKm` still 50 (the proposal was not saved).
3. `saves at once when the distance already matches`: trip 90 km, `saveSyncedRoute(T, 90)`, recalculate (90.0 km), click `save-apply-btn`. `saved-notice` shows, `cascade-modal` never shows (`isExisting()` is false right after the notice).
4. `writes a loop route's distance too`: trip `Bratislava` to `Bratislava`, 37 km, no saved map. Open the map (Loop mode generates at once through the mock, as OSRM: 90.0 km). Click `save-apply-btn`, confirm. Expect `distanceKm` 90 and a saved map with `mode` `loop`.
5. `syncs a saved map whose trip km was edited later`: trip 100 km, `saveSyncedRoute(T, 100)`, `editTripKm(T, 80)`. Open the map. `save-apply-btn` is enabled with no proposal. Click, confirm. Expect `distanceKm` 100.
6. `disables the button on a saved map that is in sync`: trip 100 km, `saveSyncedRoute(T, 100)`, open the map. Expect `save-apply-btn` disabled.
7. `names the 20 % legal limit before it is crossed`: trip A 100 km @ 50100, trip B 100 km @ 50200 with `fuelLiters: 12, fullTank: true` (200 km on 12 l = 6.0 l/100km, 20.0 %). `saveSyncedRoute(A, 100)`, recalculate (90.0 km), click `save-apply-btn`. `writeback-margin` contains `20.0` and `26.3` (190 km on 12 l), and `writeback-crosses-limit` is displayed.

- [ ] **Step 3: Build and run the specs to see them fail**

Run: `npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web && npx wdio run tests/integration/wdio.server.conf.ts --spec tests/integration/specs/tier2/route-distance-writeback.spec.ts`
Expected: FAIL, `save-apply-btn` not found.

- [ ] **Step 4: Types and API.** In `types.ts`, add `changesTrip: boolean` to `DistanceWriteback` (doc: `False when the write changes nothing: the page saves without the modal.`) and `distanceInSync: boolean` to `RouteMap` (doc: `Whether the trip km equals the rounded road km. Computed by the backend.`). In `api.ts`, delete `applyRouteDistance` and add:

```ts
/**
 * Sync a SAVED map's distance onto its trip (task 87). The backend reads the
 * road km from the saved map, so none crosses the wire.
 */
export async function applySavedRouteDistance(
	tripId: string,
	dryRun: boolean
): Promise<DistanceWriteback> {
	return await apiCall('apply_saved_route_distance', { tripId, dryRun });
}
```

`saveTripRoute(tripId, route, roundTrip, avoid, dryRun: boolean): Promise<DistanceWriteback>` stops sending `targetKm` and sends `dryRun`. `saveTripRoundTripRoute(tripId, outboundWaypoints, inboundWaypoints, outboundPolyline, inboundPolyline, outboundRoadKm, inboundRoadKm, avoid, provider, dryRun: boolean): Promise<DistanceWriteback>` loses `targetKm` and sends `dryRun`. Update both doc comments: a save writes the whole-km distance to the trip.

- [ ] **Step 5: i18n.** In the `routeMap` block of `sk/index.ts`, delete `save`, `applyDistance`, `applyDistanceTitle`, `applyDistanceDone`, `applyDistanceError`, and change `saved`. Add:

```ts
saveAndApply: 'Uložiť a použiť vzdialenosť',
saveAndApplyTitle: 'Uloží trasu a zapíše jej vzdialenosť do jazdy, zaokrúhlenú na celé km',
saved: 'Trasa uložená, vzdialenosť jazdy zodpovedá mape',
saveError: 'Trasu a vzdialenosť sa nepodarilo uložiť',
```

`en/index.ts`: `saveAndApply: 'Save and apply distance'`, `saveAndApplyTitle: 'Saves the route and writes its distance to the trip, rounded to whole km'`, `saved: 'Route saved, the trip distance matches the map'`, `saveError: 'Could not save the route and the distance'`. Run `npm run i18n`.

- [ ] **Step 6: The page.** Replace `handleSave`, `handleApplyDistance` and `confirmWriteback` with this. Remove `applying` (also from `busy` at line 280).

```ts
type SaveCall = (dryRun: boolean) => Promise<DistanceWriteback>;

/** The call Confirm sends. Captured at the click, so a change on the page
 *  behind the modal cannot change what the user approved (ADR-048). */
let pendingSave: SaveCall | null = null;

let canSaveAndApply = $derived(
	!!generated || !!roundTripRoutes || (savedRoute !== null && !savedRoute.distanceInSync)
);

/** Snapshot of what the button would save now. Null if nothing to save. */
function captureSave(): SaveCall | null {
	if (!tripId) return null;
	const id = tripId;
	const avoidNow = [...routedAvoid];
	if (roundTripRoutes) {
		// Only which alternative was picked crosses the wire. The backend
		// joins the legs and sums the distances (ADR-008).
		const legs = roundTripRoutes;
		const out = legs.outbound[outboundIndex];
		const back = legs.inbound[inboundIndex];
		return (dryRun) =>
			saveTripRoundTripRoute(
				id, legs.outboundWaypoints, legs.inboundWaypoints,
				out.polyline, back.polyline, out.roadKm, back.roadKm,
				avoidNow, legs.provider, dryRun
			);
	}
	if (generated) {
		const route = generated;
		const closeLoop = roundTrip;
		return (dryRun) => saveTripRoute(id, route, closeLoop, avoidNow, dryRun);
	}
	if (savedRoute && !savedRoute.distanceInSync) {
		return (dryRun) => applySavedRouteDistance(id, dryRun);
	}
	return null;
}

/** Plans the save. Writes NOTHING: the dry run fills the modal. If the trip
 *  would not change, it saves at once (task 87, D3). */
async function handleSaveAndApply() {
	const run = captureSave();
	if (!run) return;
	saving = true;
	try {
		const plan = await run(true);
		if (plan.changesTrip) {
			pendingSave = run;
			writeback = plan;
			return;
		}
		await commitSave(run);
	} catch (e) {
		console.error('Failed to plan the route save:', e);
		toast.error($LL.routeMap.saveError());
	} finally {
		saving = false;
	}
}

async function confirmWriteback() {
	const run = pendingSave;
	pendingSave = null;
	writeback = null;
	if (run) await commitSave(run);
}

function cancelWriteback() {
	pendingSave = null;
	writeback = null;
}

async function commitSave(run: SaveCall) {
	saving = true;
	try {
		const result = await run(false);
		// The trip's distance IS the map's target: re-read the row and the
		// saved map rather than patch numbers here.
		const vehicle = $activeVehicleStore;
		if (vehicle) {
			const trips = await getTrips(vehicle.id);
			yearTrips = trips;
			trip = trips.find((t) => t.id === tripId) ?? trip;
		}
		savedRoute = await getTripRoute(tripId);
		generated = null;
		// The alternatives panel guards on `alternatives.length`, not on
		// `generated`: leaving it populated would show the picker for a
		// proposal that no longer exists.
		alternatives = [];
		activeIndex = 0;
		roundTripRoutes = null;
		if (result.changesTrip) announce('trip-distance-updated');
		announce('route-map-saved');
		savedNotice = true;
		toast.success($LL.routeMap.saved());
	} catch (e) {
		console.error('Failed to save the route and its distance:', e);
		toast.error($LL.routeMap.saveError());
	} finally {
		saving = false;
	}
}
```

In the toolbar, delete the `apply-distance-btn` button (inside the Direct block) and the `save-btn` button, and put this after the `{/if}` that closes the mode blocks, so it shows in every mode:

```svelte
<button
	class="button"
	data-test="save-apply-btn"
	title={$LL.routeMap.saveAndApplyTitle()}
	onclick={handleSaveAndApply}
	disabled={busy || !canSaveAndApply}
>
	{$LL.routeMap.saveAndApply()}
</button>
```

In the modal block, use `onCancel={cancelWriteback}`. Fix the imports (`applySavedRouteDistance` in, `applyRouteDistance` out). Check `saveTripRoute(tripId, generated!, roundTrip, routedAvoid)` has no other caller.

- [ ] **Step 7: Type check**

Run: `npm run check && npm run typecheck:tests`
Expected: 0 errors.

- [ ] **Step 8: Run the two specs to see them pass**

Run: `npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web && npx wdio run tests/integration/wdio.server.conf.ts --spec tests/integration/specs/tier2/route-distance-writeback.spec.ts --spec tests/integration/specs/tier2/route-map.spec.ts`
Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts src/lib/i18n/i18n-types.ts src/routes/mapa/+page.svelte tests/integration/specs/tier2/route-distance-writeback.spec.ts tests/integration/specs/tier2/route-map.spec.ts
git commit -m "feat(route-map): one 'Uložiť a použiť vzdialenosť' button for every route mode"
```

---

### Task 6: Documentation

**Files:**
- Modify: [DECISIONS.md](../../DECISIONS.md) (via `/decision`)
- Modify: [docs/features/route-maps.md](../../docs/features/route-maps.md) (lines 43, 99, 154, the section from line 440)
- Modify: [docs/features/trip-odometer-cascade.md](../../docs/features/trip-odometer-cascade.md) (lines 105, 164, 168, 216)
- Modify: [docs/features/read-only-mode.md](../../docs/features/read-only-mode.md) (lines 79, 90)
- Modify: [CHANGELOG.md](../../CHANGELOG.md) (via `/changelog`)
- Check: [README.md](../../README.md), [README.en.md](../../README.en.md)

- [ ] **Step 1: ADR-054.** Run `/decision`. Title: "A saved map always writes its whole-km distance to the trip". Record D1 to D4 from [01-task.md](./01-task.md), the one transaction, and the in-sync rule for `off_target`. Mark it as a replacement for the Direct-only scope of ADR-048 and for the "circular" Loop reason. Add a "Superseded in part by ADR-054" line under ADR-048. The dry-run modal and the replan on commit stay from ADR-048.
- [ ] **Step 2: Feature docs.** Replace every `apply_route_distance` with `apply_saved_route_distance` or the save commands, as fits. In `route-maps.md`, rewrite the section from line 440: one button, all modes, whole km, no modal on a no-op, sync of a saved map. Delete the "Only Direct mode gets the button" paragraph.
- [ ] **Step 3: CHANGELOG.** Run `/changelog`. Put a "Zmenené" entry in `[Unreleased]`: one button "Uložiť a použiť vzdialenosť" in place of two, the write-back now also for Loop routes, the written km rounded to whole km. The `Pokyny k aktualizácii` block gets no new migration and no new env var. Keep its existing lines from task 86.
- [ ] **Step 4: READMEs.** Search both READMEs for the map buttons and for "vzdialenos" / "distance". Update both in the same way, or leave both alone if neither names the buttons.
- [ ] **Step 5: Commit**

```bash
git add DECISIONS.md docs/features/route-maps.md docs/features/trip-odometer-cascade.md docs/features/read-only-mode.md CHANGELOG.md README.md README.en.md
git commit -m "docs: ADR-054, one save-and-apply button, whole-km write-back in every mode"
```

---

### Task 7: Full verification

- [ ] **Step 1:** Run `cargo test --manifest-path src-tauri/Cargo.toml --workspace`. Expected: PASS.
- [ ] **Step 2:** Run `npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web && npm run test:integration`. Expected: PASS.
- [ ] **Step 3:** Run `grep -rn "apply_route_distance\|applyRouteDistance\|apply-distance-btn\|save-btn\|applyDistance" src src-tauri tests docs`. Expected: no hits, except history in `_tasks/_done` and in superseded ADR text.
- [ ] **Step 4:** Run `/verify`.
- [ ] **Step 5:** Set **Status** in [01-task.md](./01-task.md) and in this file to `Complete`, and set the row in [../index.md](../index.md) to ✅. Commit.
