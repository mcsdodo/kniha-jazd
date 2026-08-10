**Date:** 2026-08-10
**Subject:** Route map integration — implementation plan
**Status:** Planning

# Route Map Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** A per-trip action generates a home-base loop matching that trip's `distance_km`, previews it on a map, and on confirmation saves the polyline; "Export for print" then appends one A4 map page per saved route, each referencing its trip by row number.

**Architecture:** All generation, routing and rendering is Rust in `kniha-jazd-core`, per [ADR-008](../../DECISIONS.md). A new `route_map` module holds the genetic algorithm (ported from [Task 61's POC](../61-route-map-poc/poc.html)), an OSRM client behind a trait, and a tile rasteriser. A new `trip_routes` table stores the polyline plus minimal metadata; rendered PNGs and OSM tiles live in a disposable app-data cache. The frontend adds one route (`/mapa`) and one row action, gated to server mode.

**Tech Stack:** Rust · Diesel/SQLite · [tiny-skia](https://crates.io/crates/tiny-skia) (new) · [rand](https://crates.io/crates/rand) 0.8 (existing) · [reqwest](https://crates.io/crates/reqwest) blocking (existing) · SvelteKit · [Leaflet](https://leafletjs.com/) (new npm dep) · typesafe-i18n

**Requirements:** [01-task.md](./01-task.md) · **Design:** [02-design.md](./02-design.md)

---

## Read before starting

Four facts that will otherwise cost you a rewrite:

1. **`Route` and `RouteRow` already exist** in [models.rs](../../src-tauri/core/src/models.rs) — they are the origin/destination autocomplete entities, unrelated to maps. The new model is **`RouteMap`** / `RouteMapRow` / `NewRouteMapRow`. Likewise the existing table is `routes`; the new one is `trip_routes`.
2. **The injected-RNG pattern already exists.** [calculations/time_inference.rs:13-31](../../src-tauri/core/src/calculations/time_inference.rs) defines `trait Jitter` + `ThreadRngJitter`. Mirror that shape exactly for the GA — do not invent a different abstraction.
3. **These four commands are dispatcher-only — no Tauri wrappers.** A command in this codebase normally needs two wirings: an arm in [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) (or `dispatcher_async.rs` if it awaits) **and** a `#[tauri::command]` wrapper registered in the desktop crate's `invoke_handler`. Not here. The only caller of `generate_route` / `get_trip_route` / `save_trip_route` / `delete_trip_route` is `/mapa`, which is gated off on desktop ([01-task.md](./01-task.md), web-first), so desktop wrappers would be dead code. **Add no files to the desktop crate in Task 8.** They get added in V2, when the desktop UI is unhidden.
4. **Export is the exception, and it is not a command.** Desktop and web share one database (multi-PC support), so a desktop user exports trips whose maps were created on web — the attachments must render there too. That happens through `collect_route_map_pages`, an internal function called inside the *existing* `export_html` command from both export paths (Task 14). No new command, no new wrapper — but skipping the desktop side is a bug.

**Indentation is not uniform.** Tabs in `.svelte`, `src/lib/api.ts` and `src/lib/i18n/`; 4 spaces in `src/lib/stores/capabilities.ts`; 2 spaces in `src/lib/api-adapter.ts`. Match the file you are editing.

**Test command** (never `cd &&`):

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "filter"
```

---

## Task 1: `trip_routes` table

**Files:**
- Create: `src-tauri/core/migrations/2026-08-10-100000_add_trip_routes/up.sql`
- Create: `src-tauri/core/migrations/2026-08-10-100000_add_trip_routes/down.sql`
- Modify: [src-tauri/core/src/schema.rs](../../src-tauri/core/src/schema.rs)
- Test: [src-tauri/core/src/db_tests.rs](../../src-tauri/core/src/db_tests.rs)

**Step 1: Write the failing test**

Append to [db_tests.rs](../../src-tauri/core/src/db_tests.rs):

```rust
#[test]
fn trip_routes_table_exists_after_migration() {
    let db = Database::in_memory().expect("Failed to create database");
    let conn = &mut *db.connection();
    diesel::sql_query("SELECT trip_id, waypoints, polyline, target_km, road_km, dataset_version, created_at FROM trip_routes")
        .execute(conn)
        .expect("trip_routes table must exist");
}
```

**Step 2: Run it to confirm it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "trip_routes_table_exists"
```

Expected: FAIL — `no such table: trip_routes`.

**Step 3: Write the migration**

`up.sql`:

```sql
-- Task 70: generated route maps, one per trip.
-- Stores only what the export needs to re-render: the OSRM polyline plus
-- minimal metadata. Rendered PNGs and OSM tiles live in a disposable
-- app-data cache, never here — see _tasks/70-route-map-integration/02-design.md.
CREATE TABLE trip_routes (
    trip_id TEXT PRIMARY KEY,
    waypoints TEXT NOT NULL,
    polyline TEXT NOT NULL,
    target_km REAL NOT NULL,
    road_km REAL NOT NULL,
    dataset_version TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (trip_id) REFERENCES trips(id) ON DELETE CASCADE
);
```

`down.sql`:

```sql
-- Forward-only in practice (ADR-012); no diesel CLI revert runs in this repo.
DROP TABLE trip_routes;
```

**Step 4: Add the Diesel table**

In [schema.rs](../../src-tauri/core/src/schema.rs), after the `paperless_trip_links` block:

```rust
// Added via migration 2026-08-10-100000_add_trip_routes (Task 70)
diesel::table! {
    trip_routes (trip_id) {
        trip_id -> Text,
        waypoints -> Text,
        polyline -> Text,
        target_km -> Double,
        road_km -> Double,
        dataset_version -> Nullable<Text>,
        created_at -> Text,
    }
}
```

Then extend the two macro calls at the bottom of the file:

```rust
diesel::joinable!(trip_routes -> trips (trip_id));

diesel::allow_tables_to_appear_in_same_query!(paperless_trip_links, receipts, routes, settings, trip_routes, trips, vehicles,);
```

**Step 5: Run the test to confirm it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "trip_routes_table_exists"
```

Expected: PASS.

**Step 6: Commit**

```bash
git add src-tauri/core/migrations/2026-08-10-100000_add_trip_routes/ src-tauri/core/src/schema.rs src-tauri/core/src/db_tests.rs
git commit -m "feat(route-map): add trip_routes table"
```

---

## Task 2: `RouteMap` model

**Files:**
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs)
- Test: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) (inline `#[cfg(test)]` or the existing tests module)

**Step 1: Write the failing test**

```rust
#[test]
fn waypoint_serialises_camel_case_and_omits_absent_node_idx() {
    let w = Waypoint { lat: 48.935, lon: 20.553, name: Some("Domov".into()), node_idx: None };
    let json = serde_json::to_string(&w).unwrap();
    assert!(json.contains("\"lat\":48.935"));
    assert!(!json.contains("nodeIdx"), "a hand-placed point must omit nodeIdx: {json}");
}
```

**Step 2: Run it to confirm it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "waypoint_serialises"
```

Expected: FAIL — `cannot find struct Waypoint`.

**Step 3: Add the models**

In [models.rs](../../src-tauri/core/src/models.rs), near the existing `Route`:

```rust
/// One point on a generated route. `node_idx` is present when the generator
/// picked the point from the bundled dataset and absent when a human placed
/// it (V2 editor) — see _tasks/70-route-map-integration/02-design.md.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Waypoint {
    pub lat: f64,
    pub lon: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_idx: Option<i32>,
}

/// A generated route map saved against a trip. NOT to be confused with
/// [`Route`], which is the origin/destination autocomplete entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteMap {
    pub trip_id: Uuid,
    pub waypoints: Vec<Waypoint>,
    /// Encoded polyline5 as returned by OSRM.
    pub polyline: String,
    pub target_km: f64,
    pub road_km: f64,
    pub dataset_version: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

Add `RouteMapRow` (Queryable) and `NewRouteMapRow` (Insertable) alongside, mirroring how `RouteRow` is written, plus a `RouteMap::try_from(RouteMapRow)` that parses `waypoints` from JSON and `created_at` from RFC3339.

**Step 4: Run the test to confirm it passes** — expected PASS.

**Step 5: Commit**

```bash
git add src-tauri/core/src/models.rs
git commit -m "feat(route-map): add RouteMap and Waypoint models"
```

---

## Task 3: Route map CRUD

**Files:**
- Modify: [src-tauri/core/src/db.rs](../../src-tauri/core/src/db.rs)
- Test: [src-tauri/core/src/db_tests.rs](../../src-tauri/core/src/db_tests.rs)

**Step 1: Write the failing tests**

```rust
#[test]
fn route_map_round_trips() {
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new_ice("V".into(), "BA-1".into(), 50.0, 6.5, 0.0);
    db.create_vehicle(&vehicle).unwrap();
    let mut trip = Trip::test_ice_trip(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(), 120.0, None, true);
    trip.vehicle_id = vehicle.id;
    db.create_trip(&trip).unwrap();

    let map = RouteMap {
        trip_id: trip.id,
        waypoints: vec![Waypoint { lat: 48.935, lon: 20.553, name: Some("Domov".into()), node_idx: Some(0) }],
        polyline: "_p~iF~ps|U".into(),
        target_km: 120.0,
        road_km: 118.4,
        dataset_version: Some("2026-05-03".into()),
        created_at: Utc::now(),
    };
    db.save_route_map(&map).unwrap();

    let loaded = db.get_route_map(&trip.id.to_string()).unwrap().expect("must exist");
    assert_eq!(loaded.polyline, "_p~iF~ps|U");
    assert_eq!(loaded.waypoints.len(), 1);
    assert_eq!(loaded.waypoints[0].node_idx, Some(0));
}

#[test]
fn save_route_map_replaces_existing() {
    // ... same setup, save twice with different polylines,
    // assert get returns the second and only one row exists.
}

#[test]
fn route_map_cascades_when_trip_deleted() {
    // ... same setup, then db.delete_trip(&trip.id.to_string()).unwrap();
    assert!(db.get_route_map(&trip.id.to_string()).unwrap().is_none());
}

#[test]
fn get_route_maps_for_trips_returns_only_requested() {
    // Seed two trips, save a map for one, request both ids,
    // assert the returned map has exactly one entry keyed by the right trip.
}
```

**Step 2: Run them to confirm they fail** — expected: `no method named save_route_map`.

**Step 3: Implement**

In [db.rs](../../src-tauri/core/src/db.rs), following the `create_trip` / `get_trip` style (lock the `Mutex`, build a row struct, return `QueryResult`):

- `pub fn save_route_map(&self, map: &RouteMap) -> QueryResult<()>` — upsert by deleting any existing row for the trip then inserting, so re-saving is idempotent.
- `pub fn get_route_map(&self, trip_id: &str) -> QueryResult<Option<RouteMap>>`
- `pub fn delete_route_map(&self, trip_id: &str) -> QueryResult<()>`
- `pub fn get_route_maps_for_trips(&self, trip_ids: &[String]) -> QueryResult<HashMap<String, RouteMap>>` — one `eq_any` query, used by the export path so it doesn't issue N queries.

The cascade is enforced by SQLite; the bundled build sets `SQLITE_DEFAULT_FOREIGN_KEYS=1`, so no extra code is needed — but the test proves it.

**Step 4: Run the tests to confirm they pass.**

**Step 5: Commit**

```bash
git add src-tauri/core/src/db.rs src-tauri/core/src/db_tests.rs
git commit -m "feat(route-map): add route map CRUD with cascade delete"
```

---

## Task 4: Bundle the node dataset

**Files:**
- Create: `src-tauri/core/assets/villages.json` (copy of [../61-route-map-poc/villages.json](../61-route-map-poc/villages.json))
- Create: `src-tauri/core/assets/matrix.json` (copy of [../61-route-map-poc/matrix.json](../61-route-map-poc/matrix.json))
- Create: `src-tauri/core/src/route_map/mod.rs`
- Create: `src-tauri/core/src/route_map/dataset.rs`
- Create: `src-tauri/core/src/route_map/dataset_tests.rs`
- Modify: [src-tauri/core/src/lib.rs](../../src-tauri/core/src/lib.rs)

**Step 1: Write the failing test**

`dataset_tests.rs`:

```rust
use super::*;

#[test]
fn bundled_dataset_loads_67_nodes() {
    let ds = Dataset::bundled();
    assert_eq!(ds.nodes.len(), 67);
    assert_eq!(ds.nodes[0].kind, "home");
    assert_eq!(ds.matrix.len(), 67);
    assert!(ds.matrix.iter().all(|row| row.len() == 67), "matrix must be square");
}

#[test]
fn dataset_distance_is_asymmetric_and_positive() {
    let ds = Dataset::bundled();
    assert!(ds.distance(0, 1) > 0.0);
    assert_eq!(ds.distance(5, 5), 0.0);
}

#[test]
fn dataset_version_is_the_generation_date() {
    assert_eq!(Dataset::bundled().version, "2026-05-03");
}
```

**Step 2: Run to confirm failure** — module does not exist.

**Step 3: Implement**

Copy the two JSON files into `src-tauri/core/assets/`. In `dataset.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Node {
    pub idx: usize,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub kind: String,
}

#[derive(Deserialize)]
struct VillagesFile {
    #[serde(rename = "generatedAt")]
    generated_at: String,
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct MatrixFile {
    distances: Vec<Vec<f64>>,
}

pub struct Dataset {
    pub nodes: Vec<Node>,
    pub matrix: Vec<Vec<f64>>,
    pub version: String,
}

const VILLAGES_JSON: &str = include_str!("../../assets/villages.json");
const MATRIX_JSON: &str = include_str!("../../assets/matrix.json");

impl Dataset {
    pub fn bundled() -> Self {
        let v: VillagesFile = serde_json::from_str(VILLAGES_JSON).expect("bundled villages.json must parse");
        let m: MatrixFile = serde_json::from_str(MATRIX_JSON).expect("bundled matrix.json must parse");
        Self { nodes: v.nodes, matrix: m.distances, version: v.generated_at }
    }

    pub fn distance(&self, from: usize, to: usize) -> f64 {
        self.matrix[from][to]
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}
```

`mod.rs`:

```rust
//! Generated trip route maps — GA route selection, OSRM geometry, tile rendering.

pub mod dataset;
pub use dataset::Dataset;

#[cfg(test)]
#[path = "dataset_tests.rs"]
mod dataset_tests;
```

Add `pub mod route_map;` to [lib.rs](../../src-tauri/core/src/lib.rs), alphabetically after `pub mod receipts;`.

**Step 4: Run the tests to confirm they pass.**

**Step 5: Commit**

```bash
git add src-tauri/core/assets/ src-tauri/core/src/route_map/ src-tauri/core/src/lib.rs
git commit -m "feat(route-map): bundle the 67-node Slovak dataset"
```

---

## Task 5: Genetic algorithm

Port of the POC's `generateGA` — see [the POC design](../61-route-map-poc/02-design.md) for why a GA rather than a deterministic heuristic, and for the hyperparameters. **Do not re-derive those decisions.**

**Files:**
- Create: `src-tauri/core/src/route_map/ga.rs`
- Create: `src-tauri/core/src/route_map/ga_tests.rs`
- Modify: `src-tauri/core/src/route_map/mod.rs`

**Step 1: Write the failing tests**

Mirror the `Jitter` pattern: the pure function takes `&mut impl RouteRng`, so tests inject a seeded RNG.

```rust
use super::*;
use crate::route_map::Dataset;

fn seeded(seed: u64) -> SeededRouteRng {
    SeededRouteRng::new(seed)
}

#[test]
fn route_starts_and_ends_at_home() {
    let ds = Dataset::bundled();
    let r = generate_route(120.0, &ds, &mut seeded(42));
    assert_eq!(*r.sequence.first().unwrap(), 0);
    assert_eq!(*r.sequence.last().unwrap(), 0);
}

#[test]
fn route_uses_only_valid_node_indices() {
    let ds = Dataset::bundled();
    let r = generate_route(120.0, &ds, &mut seeded(7));
    assert!(r.sequence.iter().all(|&i| i < ds.len()));
}

#[test]
fn route_visits_no_node_twice() {
    let ds = Dataset::bundled();
    let r = generate_route(150.0, &ds, &mut seeded(11));
    let mut mids = r.sequence[1..r.sequence.len() - 1].to_vec();
    let before = mids.len();
    mids.sort_unstable();
    mids.dedup();
    assert_eq!(mids.len(), before, "intermediate stops must be unique");
}

#[test]
fn route_lands_within_tolerance_across_targets() {
    let ds = Dataset::bundled();
    for (i, target) in [50.0, 100.0, 150.0, 200.0, 300.0].iter().enumerate() {
        let r = generate_route(*target, &ds, &mut seeded(100 + i as u64));
        let err = ((r.total_km - target) / target).abs();
        assert!(err <= 0.05, "target {target}: got {} ({:.1}% off)", r.total_km, err * 100.0);
    }
}

#[test]
fn different_seeds_produce_different_routes() {
    let ds = Dataset::bundled();
    let a = generate_route(120.0, &ds, &mut seeded(1));
    let b = generate_route(120.0, &ds, &mut seeded(2));
    assert_ne!(a.sequence, b.sequence, "variety is the feature — see 61-route-map-poc/02-design.md");
}

#[test]
fn same_seed_reproduces_the_same_route() {
    let ds = Dataset::bundled();
    let a = generate_route(120.0, &ds, &mut seeded(9));
    let b = generate_route(120.0, &ds, &mut seeded(9));
    assert_eq!(a.sequence, b.sequence);
}

#[test]
fn total_km_matches_the_matrix_sum() {
    let ds = Dataset::bundled();
    let r = generate_route(90.0, &ds, &mut seeded(3));
    let sum: f64 = r.sequence.windows(2).map(|w| ds.distance(w[0], w[1])).sum();
    assert!((r.total_km - sum).abs() < 1e-9);
}
```

**Step 2: Run to confirm failure.**

**Step 3: Implement**

```rust
//! Genetic route selection, ported from _tasks/61-route-map-poc/poc.html.
//!
//! Randomness is business logic (ADR-008): the pure `generate_route` takes an
//! injected RNG so tests are deterministic, while `generate_route_random`
//! supplies a thread RNG so production runs vary. This mirrors the
//! `Jitter` / `ThreadRngJitter` split in calculations/time_inference.rs.

use super::Dataset;

const HOME: usize = 0;
const POP: usize = 50;
const GENS: usize = 100;
const MUT: f64 = 0.25;
const ELITE: usize = 2;
const TOUR: usize = 3;
const MAX_STOPS: usize = 5;

/// Randomness source for route generation.
pub trait RouteRng {
    /// Uniform in `[0, n)`. Callers must pass `n > 0`.
    fn below(&mut self, n: usize) -> usize;
    /// Uniform in `[0.0, 1.0)`.
    fn unit(&mut self) -> f64;
}

/// Production [`RouteRng`] backed by `rand::thread_rng`.
pub struct ThreadRouteRng;

impl RouteRng for ThreadRouteRng {
    fn below(&mut self, n: usize) -> usize {
        use rand::Rng;
        rand::thread_rng().gen_range(0..n)
    }
    fn unit(&mut self) -> f64 {
        use rand::Rng;
        rand::thread_rng().gen()
    }
}

/// Deterministic [`RouteRng`] for tests.
pub struct SeededRouteRng(rand::rngs::StdRng);

impl SeededRouteRng {
    pub fn new(seed: u64) -> Self {
        use rand::SeedableRng;
        Self(rand::rngs::StdRng::seed_from_u64(seed))
    }
}

impl RouteRng for SeededRouteRng {
    fn below(&mut self, n: usize) -> usize {
        use rand::Rng;
        self.0.gen_range(0..n)
    }
    fn unit(&mut self) -> f64 {
        use rand::Rng;
        self.0.gen()
    }
}

#[derive(Debug, Clone)]
pub struct RouteResult {
    /// Node indices, starting and ending at [`HOME`].
    pub sequence: Vec<usize>,
    /// Matrix distance of the sequence, in km.
    pub total_km: f64,
}

pub fn generate_route(target_km: f64, ds: &Dataset, rng: &mut impl RouteRng) -> RouteResult { /* … */ }

pub fn generate_route_random(target_km: f64, ds: &Dataset) -> RouteResult {
    generate_route(target_km, ds, &mut ThreadRouteRng)
}
```

Port the body from [_build-poc.ps1](../61-route-map-poc/_build-poc.ps1) lines 60–145 one-for-one: `total_km`, `make_chromosome`, `tournament`, order crossover, mutate (insert / remove / swap), elitism, `GENS` generations, return the fittest. Fitness is `1.0 / (1.0 + (total_km - target_km).abs())`.

**There is no multi-session split.** The POC's `planSessions` / `SINGLE_SESSION_MAX_KM` is deliberately not ported — one trip is one map ([01-task.md](./01-task.md) non-goals).

Register in `mod.rs`:

```rust
pub mod ga;
pub use ga::{generate_route, generate_route_random, RouteResult, RouteRng, SeededRouteRng, ThreadRouteRng};

#[cfg(test)]
#[path = "ga_tests.rs"]
mod ga_tests;
```

**Step 4: Run the tests to confirm they pass.**

If `route_lands_within_tolerance_across_targets` fails only at 300 km, that is the known dataset ceiling — raise `MAX_STOPS` to 8 and re-run before assuming a port bug.

**Step 5: Commit**

```bash
git add src-tauri/core/src/route_map/ga.rs src-tauri/core/src/route_map/ga_tests.rs src-tauri/core/src/route_map/mod.rs
git commit -m "feat(route-map): port genetic route selection to Rust"
```

---

## Task 6: Polyline codec

**Files:**
- Create: `src-tauri/core/src/route_map/polyline.rs`
- Create: `src-tauri/core/src/route_map/polyline_tests.rs`
- Modify: `src-tauri/core/src/route_map/mod.rs`

**Step 1: Write the failing tests**

```rust
use super::*;

#[test]
fn decodes_the_reference_polyline() {
    // Canonical example from Google's polyline algorithm docs.
    let pts = decode("_p~iF~ps|U_ulLnnqC_mqNvxq`@");
    assert_eq!(pts.len(), 3);
    assert!((pts[0].0 - 38.5).abs() < 1e-5);
    assert!((pts[0].1 - -120.2).abs() < 1e-5);
    assert!((pts[2].0 - 43.252).abs() < 1e-5);
}

#[test]
fn round_trips_within_precision() {
    let pts = vec![(48.935, 20.553), (48.9973, 20.5911), (48.935, 20.553)];
    let decoded = decode(&encode(&pts));
    for (a, b) in pts.iter().zip(decoded.iter()) {
        assert!((a.0 - b.0).abs() < 1e-5 && (a.1 - b.1).abs() < 1e-5);
    }
}

#[test]
fn decoding_garbage_yields_no_points() {
    assert!(decode("!!!not-a-polyline").is_empty());
}
```

**Step 2: Run to confirm failure.**

**Step 3: Implement** standard polyline5 encode/decode over `Vec<(f64, f64)>` as `(lat, lon)`. `decode` must never panic on malformed input — return what it managed to parse.

**Step 4: Run the tests to confirm they pass.**

**Step 5: Commit**

```bash
git add src-tauri/core/src/route_map/polyline.rs src-tauri/core/src/route_map/polyline_tests.rs src-tauri/core/src/route_map/mod.rs
git commit -m "feat(route-map): add polyline5 codec"
```

---

## Task 7: OSRM client behind a trait

**Files:**
- Create: `src-tauri/core/src/route_map/osrm.rs`
- Create: `src-tauri/core/src/route_map/osrm_tests.rs`
- Modify: `src-tauri/core/src/route_map/mod.rs`

**Step 1: Write the failing tests**

Use `wiremock`, already a dev-dependency (see `dispatcher_async.rs` tests):

```rust
#[tokio::test]
async fn parses_a_successful_route_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path_regex(r"^/route/v1/driving/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "Ok",
            "routes": [{ "geometry": "_p~iF~ps|U_ulLnnqC", "distance": 118432.0 }]
        })))
        .mount(&server).await;

    let client = HttpRouteProvider::new(server.uri());
    let r = client.fetch(&[(48.935, 20.553), (48.997, 20.591)]).await.unwrap();
    assert_eq!(r.polyline, "_p~iF~ps|U_ulLnnqC");
    assert!((r.road_km - 118.432).abs() < 1e-3);
}

#[tokio::test]
async fn surfaces_a_non_ok_code_as_an_error() {
    // respond with {"code": "NoRoute"} -> Err containing "NoRoute"
}

#[tokio::test]
async fn surfaces_http_429_as_an_error() {
    // respond 429 -> Err mentioning the status, so the UI can offer Retry
}
```

**Step 2: Run to confirm failure.**

**Step 3: Implement**

```rust
pub struct FetchedRoute {
    pub polyline: String,
    pub road_km: f64,
}

#[async_trait::async_trait]
pub trait RouteProvider: Send + Sync {
    async fn fetch(&self, coords: &[(f64, f64)]) -> Result<FetchedRoute, String>;
}

pub struct HttpRouteProvider { base_url: String }
```

Request `{base}/route/v1/driving/{lon,lat;…}?geometries=polyline&overview=full&steps=false`. Default base URL [router.project-osrm.org](https://router.project-osrm.org/). Note coordinates are **lon,lat** in the URL but `(lat, lon)` in Rust — the POC's `fetchPolyline` gets this right; keep it right.

Add one dependency to [core/Cargo.toml](../../src-tauri/core/Cargo.toml) — [async-trait](https://crates.io/crates/async-trait) is not currently present:

```toml
async-trait = "0.1"
```

[wiremock](https://crates.io/crates/wiremock) and [tempfile](https://crates.io/crates/tempfile) are already dev-dependencies; no change needed for the tests.

**Step 4: Run the tests to confirm they pass.**

**Step 5: Commit**

```bash
git add src-tauri/core/src/route_map/osrm.rs src-tauri/core/src/route_map/osrm_tests.rs src-tauri/core/src/route_map/mod.rs src-tauri/core/Cargo.toml
git commit -m "feat(route-map): add OSRM route provider behind a trait"
```

---

## Task 8: Commands

**Files:**
- Create: `src-tauri/core/src/commands_internal/route_maps.rs`
- Create: `src-tauri/core/src/commands_internal/route_maps_tests.rs`
- Modify: [src-tauri/core/src/commands_internal/mod.rs](../../src-tauri/core/src/commands_internal/mod.rs)
- Modify: [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs), [dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs)
- **Not modified:** the desktop crate — see "Read before starting" point 3

**Step 1: Write the failing tests**

```rust
#[test]
fn save_rejects_read_only_mode() {
    // AppState marked read-only -> save_trip_route_internal returns Err
    // mentioning "len na čítanie" (the check_read_only! macro message).
}

#[test]
fn get_returns_none_for_a_trip_without_a_map() { /* … */ }

#[test]
fn delete_is_idempotent() {
    // deleting a non-existent map must not error
}
```

**Step 2: Run to confirm failure.**

**Step 3: Implement**

```rust
//! Route map command implementations (framework-free).

use crate::check_read_only;
use crate::app_state::AppState;
use crate::db::Database;
use crate::models::RouteMap;
use crate::route_map::{osrm::RouteProvider, Dataset};

/// Generate a candidate route for `target_km`. Persists nothing — the caller
/// confirms with `save_trip_route_internal`.
pub async fn generate_route_internal(
    provider: &dyn RouteProvider,
    target_km: f64,
) -> Result<GeneratedRoute, String>;

pub fn get_trip_route_internal(db: &Database, trip_id: String) -> Result<Option<RouteMap>, String>;

pub fn save_trip_route_internal(
    db: &Database,
    app_state: &AppState,
    trip_id: String,
    waypoints: Vec<Waypoint>,
    polyline: String,
    target_km: f64,
    road_km: f64,
) -> Result<(), String>;

pub fn delete_trip_route_internal(
    db: &Database,
    app_state: &AppState,
    trip_id: String,
) -> Result<(), String>;
```

`GeneratedRoute` is what the map view renders, so it carries decoded coordinates as well as the polyline — saving the frontend a JS decoder:

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedRoute {
    pub waypoints: Vec<Waypoint>,
    pub polyline: String,
    /// Decoded `[lat, lon]` pairs, ready for `L.polyline`.
    pub coordinates: Vec<[f64; 2]>,
    pub target_km: f64,
    pub road_km: f64,
    pub dataset_version: String,
}
```

Both write commands start with `check_read_only!(app_state);`.

Register the module in [commands_internal/mod.rs](../../src-tauri/core/src/commands_internal/mod.rs) with the `pub mod` + `pub use` pair, then wire:
- `generate_route` → **`dispatcher_async.rs`** (it awaits OSRM)
- `get_trip_route`, `save_trip_route`, `delete_trip_route` → **`dispatcher.rs`**

That is the whole wiring. **No `#[tauri::command]` wrappers**, no `invoke_handler` change, no desktop-crate files — the only caller is web-gated, so wrappers would be dead code. V2 adds them alongside the desktop UI.

**Step 4: Run the tests to confirm they pass.**

**Step 5: Commit**

```bash
git add src-tauri/core/src/commands_internal/ src-tauri/core/src/server/
git commit -m "feat(route-map): add generate/get/save/delete route commands"
```

---

## Task 9: Capability flag

**Files:**
- Modify: [src-tauri/core/src/server/mod.rs](../../src-tauri/core/src/server/mod.rs)
- Modify: [src/lib/stores/capabilities.ts](../../src/lib/stores/capabilities.ts) (4-space indent)

**Step 1:** Add `"route_maps": true` to the `features` object in `capabilities_handler`.

**Step 2:** In `capabilities.ts` add `routeMaps: boolean` to the `features` interface, `routeMaps: false` to `defaultDesktop`, and `routeMaps: data.features.route_maps` to the server branch.

Desktop defaults to `false` — that is the web-first gate from [01-task.md](./01-task.md). It is also what makes the missing Tauri wrappers safe: with this `false`, nothing on desktop can invoke a route-map command, so there is no path to a "command not found" error. Enabling desktop in V2 means flipping this value **and** adding the wrappers together — one without the other breaks.

**Step 3: Verify**

```bash
npm run check
```

Expected: no new errors.

**Step 4: Commit**

```bash
git add src-tauri/core/src/server/mod.rs src/lib/stores/capabilities.ts
git commit -m "feat(route-map): gate route maps behind a server-mode capability"
```

---

## Task 10: Tile geometry

Pure maths, no network. Web Mercator, standard OSM tile scheme.

**Files:**
- Create: `src-tauri/core/src/route_map/tiles.rs`
- Create: `src-tauri/core/src/route_map/tiles_tests.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn projects_known_coordinates_to_known_tiles() {
    // Null Island at zoom 1 is tile (1, 1).
    assert_eq!(tile_xy(0.0, 0.0, 1), (1, 1));
    // Home base at zoom 12 — precomputed reference.
    let (x, y) = tile_xy(48.935, 20.553, 12);
    assert_eq!((x, y), (2265, 1401));
}

#[test]
fn picks_the_largest_zoom_that_fits_the_bounds() {
    let bounds = Bounds { min_lat: 48.85, max_lat: 49.05, min_lon: 20.40, max_lon: 20.75 };
    let z = pick_zoom(&bounds, 1400, 900);
    assert!((9..=14).contains(&z), "unexpected zoom {z}");
    assert!(fits(&bounds, z, 1400, 900));
    assert!(!fits(&bounds, z + 1, 1400, 900), "z+1 must overflow the canvas");
}

#[test]
fn bounds_from_points_covers_every_point() { /* … */ }

#[test]
fn a_single_point_yields_a_valid_zoom() {
    // Degenerate bounds must not divide by zero or loop forever.
    let b = Bounds { min_lat: 48.9, max_lat: 48.9, min_lon: 20.5, max_lon: 20.5 };
    assert!(pick_zoom(&b, 1400, 900) <= MAX_ZOOM);
}

#[test]
fn grid_covers_the_bounds_inclusively() {
    // TileGrid::for_bounds must include the tiles containing both corners.
}
```

**Step 2: Run to confirm failure.**

**Step 3: Implement** `Bounds`, `bounds_from_points`, `tile_xy`, `pick_zoom` (largest zoom whose pixel span fits the canvas, capped at `MAX_ZOOM = 17`), `TileGrid::for_bounds`, and `project_to_pixel(lat, lon, zoom, origin) -> (f32, f32)`.

**Step 4: Run the tests to confirm they pass.**

**Step 5: Commit**

```bash
git add src-tauri/core/src/route_map/tiles.rs src-tauri/core/src/route_map/tiles_tests.rs
git commit -m "feat(route-map): add web mercator tile geometry"
```

---

## Task 11: Tile fetching and disposable cache

**Files:**
- Modify: `src-tauri/core/src/route_map/tiles.rs`
- Modify: `src-tauri/core/src/route_map/tiles_tests.rs`

**Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn a_cached_tile_is_not_refetched() {
    // wiremock expecting exactly 1 request; call fetch_tile twice with the
    // same cache dir; assert the mock saw one hit and both calls returned bytes.
}

#[tokio::test]
async fn sends_an_identifying_user_agent() {
    // OSM tile policy requires it. Assert the header matcher matches.
}

#[tokio::test]
async fn a_failed_tile_is_not_written_to_the_cache() {
    // respond 500; assert no file appears in the cache dir, so a later
    // successful run is not poisoned.
}
```

**Step 2: Run to confirm failure.**

**Step 3: Implement**

```rust
#[async_trait::async_trait]
pub trait TileFetcher: Send + Sync {
    async fn tile(&self, z: u8, x: u32, y: u32) -> Result<Vec<u8>, String>;
}

/// Cache-first OSM tile fetcher. The cache directory is disposable —
/// deleting it costs a re-fetch and nothing else, so it is never backed up
/// or moved with the database (02-design.md).
pub struct CachedTileFetcher {
    cache_dir: PathBuf,
    base_url: String,
    client: reqwest::Client,
}
```

Cache path `{cache_dir}/tiles/{z}/{x}/{y}.png`. User-Agent `kniha-jazd/{version} (+https://github.com/mcsdodo/kniha-jazd)`.

**Step 4: Run the tests to confirm they pass.**

**Step 5: Commit**

```bash
git add src-tauri/core/src/route_map/tiles.rs src-tauri/core/src/route_map/tiles_tests.rs
git commit -m "feat(route-map): add cache-first OSM tile fetcher"
```

---

## Task 12: Rasteriser

**Files:**
- Create: `src-tauri/core/src/route_map/render.rs`
- Create: `src-tauri/core/src/route_map/render_tests.rs`
- Create: `src-tauri/core/tests/fixtures/tile.png` (any small 256×256 PNG)
- Modify: [src-tauri/core/Cargo.toml](../../src-tauri/core/Cargo.toml)

**Add one dependency:**

```toml
tiny-skia = "0.11"
```

`tiny-skia` alone decodes tile PNGs (`Pixmap::decode_png`), composites them (`draw_pixmap`), strokes the route (`PathBuilder` + `stroke_path`) and encodes the result (`encode_png`). It renders **no text** — which is why OSM attribution goes in the export HTML caption rather than into the pixels (Task 13). Do not add `image` or `imageproc`.

**Step 1: Write the failing tests**

Use a stub `TileFetcher` returning the fixture bytes — no network.

```rust
#[tokio::test]
async fn renders_a_png_of_the_requested_size() {
    let png = render_route(&StubTiles, &sample_points(), 1400, 900).await.unwrap();
    let pm = tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert_eq!((pm.width(), pm.height()), (1400, 900));
}

#[tokio::test]
async fn the_route_stroke_is_actually_drawn() {
    // Count pixels close to #0066cc; assert > 0 against a blank-tile fixture,
    // so a silently-skipped stroke fails the test.
}

#[tokio::test]
async fn renders_on_a_plain_background_when_every_tile_fails() {
    // Stub fetcher always Err -> still Ok(png) of the right size with a
    // visible stroke. This is the export-time offline fallback (02-design.md).
}

#[tokio::test]
async fn a_partial_tile_failure_still_produces_a_png() { /* … */ }

#[tokio::test]
async fn an_empty_point_list_is_an_error_not_a_panic() { /* … */ }
```

**Step 2: Run to confirm failure.**

**Step 3: Implement**

```rust
pub async fn render_route(
    tiles: &dyn TileFetcher,
    points: &[(f64, f64)],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String>;
```

Pipeline: bounds → `pick_zoom` → `TileGrid` → fetch tiles concurrently → composite onto a `Pixmap` (fill `#f2efe9`, OSM's land colour, where a tile is missing) → build the path via `project_to_pixel` → stroke 5 px `#0066cc`, round caps and joins, anti-aliased → `encode_png`.

**Step 4: Run the tests to confirm they pass.**

**Step 5: Commit**

```bash
git add src-tauri/core/src/route_map/render.rs src-tauri/core/src/route_map/render_tests.rs src-tauri/core/tests/fixtures/ src-tauri/core/Cargo.toml
git commit -m "feat(route-map): rasterise routes over OSM tiles with tiny-skia"
```

---

## Task 13: Map pages in the export HTML

**Files:**
- Modify: [src-tauri/core/src/export.rs](../../src-tauri/core/src/export.rs)
- Modify: [src-tauri/core/src/export_tests.rs](../../src-tauri/core/src/export_tests.rs)

**Step 1: Write the failing tests**

```rust
#[test]
fn export_appends_one_page_per_route_map() {
    let mut data = sample_export_data();
    data.route_maps = vec![
        RouteMapPage { attachment_no: 1, row_number: 3, png_base64: "AAAA".into() },
        RouteMapPage { attachment_no: 2, row_number: 7, png_base64: "BBBB".into() },
    ];
    let html = generate_html(data).unwrap();
    assert_eq!(html.matches("class=\"map-page\"").count(), 2);
    assert!(html.contains("data:image/png;base64,AAAA"));
    assert!(html.contains("Príloha č. 1"));
    assert!(html.contains("záznam č. 3"));
    assert!(html.contains("Príloha č. 2"));
    assert!(html.contains("záznam č. 7"));
}

#[test]
fn export_without_route_maps_is_unchanged() {
    let html = generate_html(sample_export_data()).unwrap();
    assert!(!html.contains("map-page"), "no maps must mean no extra markup");
}

#[test]
fn map_pages_carry_osm_attribution() {
    // Licence requirement — tiny-skia renders no text, so the caption is the
    // only place attribution can live.
    let mut data = sample_export_data();
    data.route_maps = vec![RouteMapPage { attachment_no: 1, row_number: 1, png_base64: "A".into() }];
    assert!(generate_html(data).unwrap().contains("OpenStreetMap"));
}

#[test]
fn the_trip_table_gains_no_column() {
    // Guards the "attachment -> row, one way" decision: header cell count
    // must be identical with and without route maps.
}
```

**Step 2: Run to confirm failure.**

**Step 3: Implement**

Add to [export.rs](../../src-tauri/core/src/export.rs):

```rust
/// One appended A4-landscape attachment page. Carries only its attachment
/// number, the row it references, and the image — "minimum data for the
/// reviewer" (01-task.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteMapPage {
    pub attachment_no: usize,
    pub row_number: usize,
    pub png_base64: String,
}
```

Add `pub route_maps: Vec<RouteMapPage>` to `ExportData` and two label fields to `ExportLabels` (`attachment_heading`, `record_reference`). Emit after the closing `</table>`:

```html
<div class="map-page">
  <h2>Príloha č. {n} — záznam č. {row}</h2>
  <img src="data:image/png;base64,{png}" alt="">
  <p class="map-attribution">© OpenStreetMap contributors</p>
</div>
```

Print CSS inside the existing `<style>` block:

```css
.map-page { page-break-before: always; text-align: center; }
.map-page img { max-width: 100%; max-height: 170mm; }
.map-attribution { font-size: 9px; color: #666; margin-top: 4px; }
```

**Step 4: Run the tests to confirm they pass.**

**Step 5: Commit**

```bash
git add src-tauri/core/src/export.rs src-tauri/core/src/export_tests.rs
git commit -m "feat(route-map): append map attachment pages to the export"
```

---

## Task 14: Wire both export paths

**This is the task most likely to ship a subtle bug** — see [02-design.md](./02-design.md), "Row numbering is the correctness risk". The two paths number rows differently: desktop's `export_to_browser` injects a synthetic "Prvý záznam" row 0, `export_html_internal` does not.

**Files:**
- Modify: [src-tauri/core/src/commands_internal/export_cmd.rs](../../src-tauri/core/src/commands_internal/export_cmd.rs)
- Modify: [src-tauri/desktop/src/commands/export_cmd.rs](../../src-tauri/desktop/src/commands/export_cmd.rs)
- Create: helper `collect_route_map_pages` in `src-tauri/core/src/commands_internal/route_maps.rs`
- Test: `src-tauri/core/src/commands_internal/route_maps_tests.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn attachment_row_numbers_come_from_the_assembled_rows() {
    // Given assembled rows [row 1 = trip A, row 2 = trip B, row 3 = trip C]
    // and saved maps for A and C only,
    // assert pages == [(attachment 1, row 1), (attachment 2, row 3)].
    // A map for a trip absent from the rows must not appear at all.
}

#[test]
fn attachment_numbers_are_sequential_regardless_of_row_gaps() {
    // maps for rows 2 and 9 -> attachments 1 and 2, not 2 and 9.
}

#[test]
fn trips_without_maps_produce_no_pages() { /* … */ }
```

**Step 2: Run to confirm failure.**

**Step 3: Implement**

```rust
/// Build attachment pages from ALREADY-ASSEMBLED rows.
///
/// `rows` must be the same ordered (row_number, trip_id) list the printed
/// table is numbered from — never recomputed. Desktop injects a synthetic
/// row 0 that server mode does not, so an independently derived row number
/// makes the two modes cite different rows for the same map.
pub async fn collect_route_map_pages(
    db: &Database,
    tiles: &dyn TileFetcher,
    rows: &[(usize, String)],
) -> Vec<RouteMapPage>;
```

For each row with a saved map, in row order: decode the polyline, `render_route`, base64-encode, assign the next sequential `attachment_no`. Use `get_route_maps_for_trips` (one query). A render failure skips that page and logs — the rest of the export survives ([02-design.md](./02-design.md), error handling).

Then call it from **both** `export_html_internal` and desktop `export_to_browser`, each passing its own assembled row list, and set `export_data.route_maps`.

**Step 4: Run the tests to confirm they pass, then the full backend suite:**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```

**Step 5: Commit**

```bash
git add src-tauri/core/src/commands_internal/ src-tauri/desktop/src/commands/export_cmd.rs
git commit -m "feat(route-map): render map attachments in both export paths"
```

---

## Task 15: Frontend API layer

**Files:**
- Modify: [src/lib/types.ts](../../src/lib/types.ts)
- Modify: [src/lib/api.ts](../../src/lib/api.ts) (tabs)

**Step 1:** Add types:

```ts
export interface Waypoint {
	lat: number;
	lon: number;
	name?: string;
	nodeIdx?: number;
}

export interface GeneratedRoute {
	waypoints: Waypoint[];
	polyline: string;
	coordinates: [number, number][];
	targetKm: number;
	roadKm: number;
	datasetVersion: string;
}

export interface RouteMap {
	tripId: string;
	waypoints: Waypoint[];
	polyline: string;
	targetKm: number;
	roadKm: number;
	datasetVersion: string | null;
	createdAt: string;
}
```

**Step 2:** Add the four wrappers to `api.ts`, matching the existing one-line `apiCall` style:

```ts
// Route map commands (Task 70)
export async function generateRoute(targetKm: number): Promise<GeneratedRoute> {
	return await apiCall('generate_route', { targetKm });
}

export async function getTripRoute(tripId: string): Promise<RouteMap | null> {
	return await apiCall('get_trip_route', { tripId });
}

export async function saveTripRoute(tripId: string, route: GeneratedRoute): Promise<void> {
	return await apiCall('save_trip_route', {
		tripId,
		waypoints: route.waypoints,
		polyline: route.polyline,
		targetKm: route.targetKm,
		roadKm: route.roadKm,
	});
}

export async function deleteTripRoute(tripId: string): Promise<void> {
	return await apiCall('delete_trip_route', { tripId });
}
```

**Step 3: Verify** — `npm run check`, expected no new errors.

**Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts
git commit -m "feat(route-map): add frontend route map API"
```

---

## Task 16: i18n strings

**Files:**
- Modify: [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts), [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts) (tabs)

Add a `routeMap` section to both. Slovak is the source of truth for user-facing text:

```ts
	routeMap: {
		addMap: 'Pridať mapu trasy',
		viewMap: 'Zobraziť mapu trasy',
		title: 'Mapa trasy',
		regenerate: 'Generovať znova',
		save: 'Uložiť mapu',
		remove: 'Odstrániť mapu',
		generating: 'Generujem trasu...',
		saved: 'Mapa trasy uložená',
		removed: 'Mapa trasy odstránená',
		targetKm: 'Cieľová vzdialenosť',
		actualKm: 'Skutočná vzdialenosť',
		deviation: 'Odchýlka',
		stops: 'Zastávky',
		error: 'Trasu sa nepodarilo vygenerovať',
		retry: 'Skúsiť znova',
		confirmRemove: 'Naozaj odstrániť mapu trasy?',
	},
```

Export labels (used by the Rust export) go under the existing `export` section:

```ts
		attachmentHeading: 'Príloha č. {n}',
		recordReference: 'záznam č. {row}',
```

Then regenerate types and verify:

```bash
npm run check
```

**Commit**

```bash
git add src/lib/i18n/
git commit -m "feat(route-map): add Slovak and English route map strings"
```

---

## Task 17: The `/mapa` route

**Files:**
- Create: `src/routes/mapa/+page.svelte`
- Modify: [package.json](../../package.json)

Use **Svelte 5 runes** (`$state`, `$derived`) to match [doklady/+page.svelte](../../src/routes/doklady/+page.svelte), not the older `export let` style in `TripRow.svelte`.

**Step 1: Add Leaflet**

```bash
npm install leaflet
npm install --save-dev @types/leaflet
```

Bundled through Vite — **not** the POC's unpkg CDN, so the page works without a CDN round-trip and carries no external-origin dependency.

**Step 2: Build the page**

- Read `tripId` from `$page.url.searchParams`.
- `onMount`: load the trip, then `getTripRoute(tripId)`. If a map exists, decode and render it; if not, call `generateRoute(trip.distanceKm)`.
- Import Leaflet CSS: `import 'leaflet/dist/leaflet.css';`
- Render exactly as the POC does — OSM tile layer, one `L.polyline` at `#0066cc` weight 5, **no markers**, `fitBounds` with 30 px padding.
- Show target / actual / deviation % and the stop names, mirroring the POC's info line.
- Three buttons wired to `generateRoute`, `saveTripRoute`, `deleteTripRoute`.
- **Regenerate must not persist** — it only replaces local state.
- After a successful save: `new BroadcastChannel('kniha-jazd').postMessage({ type: 'route-map-saved', tripId })`, then show a "you may close this tab" confirmation.
- Guard: if `$capabilities.features.routeMaps` is false, render a "not available in this mode" message instead of the map.

**Step 3: Verify**

```bash
npm run check
npm run build
```

Expected: both clean.

**Step 4: Commit**

```bash
git add src/routes/mapa/ package.json package-lock.json
git commit -m "feat(route-map): add the map generation view"
```

---

## Task 18: Trip row action

**Files:**
- Modify: [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte) (tabs, `export let` style)
- Modify: [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte)

**Step 1:** In `TripRow.svelte` add two props beside the existing callbacks:

```svelte
	export let hasRouteMap: boolean = false;
	export let onOpenRouteMap: () => void = () => {};
```

**Step 2:** Add a third button to the display-row `col-actions` cell, **before** the delete button, following the exact shape of the existing icon buttons (`icon-btn` class, `on:click|stopPropagation`, `title` from `$LL`):

```svelte
				{#if $capabilities.features.routeMaps}
					<button
						class="icon-btn map"
						class:has-map={hasRouteMap}
						on:click|stopPropagation={onOpenRouteMap}
						title={hasRouteMap ? $LL.routeMap.viewMap() : $LL.routeMap.addMap()}
					>
						<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill={hasRouteMap ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2">
							<path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z"></path>
							<circle cx="12" cy="10" r="3"></circle>
						</svg>
					</button>
				{/if}
```

Import `capabilities` at the top of the file.

**Step 3:** In `TripGrid.svelte`, pass the two props on the existing-trip `<TripRow>` instance (around line 708):

```svelte
							hasRouteMap={routeMapTripIds.has(trip.id)}
							onOpenRouteMap={() => window.open(`/mapa?trip=${trip.id}`, '_blank')}
```

Maintain `routeMapTripIds` as a `Set<string>` loaded alongside grid data, and subscribe to the `kniha-jazd` `BroadcastChannel` in `onMount` — on a `route-map-saved` message, add the id so the icon updates without a reload. Tear the channel down in `onDestroy`.

**Step 4: Verify** — `npm run check`, then `npm run build`.

**Step 5: Commit**

```bash
git add src/lib/components/TripRow.svelte src/lib/components/TripGrid.svelte
git commit -m "feat(route-map): add the map action to trip rows"
```

---

## Task 19: Integration tests

Flows only — the GA, tile maths and rendering are already proven in Rust. **Do not re-test route generation here** ([CLAUDE.md](../../CLAUDE.md), "No Duplication, Full Coverage").

**Files:**
- Create: `tests/integration/specs/tier2/route-map.spec.ts`

Follow the conventions in [time-column.spec.ts](../../tests/integration/specs/tier2/time-column.spec.ts): `waitForAppReady`, `ensureLanguage('en')`, `seedVehicle`, `seedTrip`, `setActiveVehicle`, `waitForTripGrid`, `invokeTauri`.

Cover:
1. Map action appears on trip rows in server mode.
2. Saving a map via `invokeTauri('save_trip_route', …)` flips the row icon to its saved state.
3. `delete_trip_route` clears the icon.
4. Deleting the trip removes the map (cascade, observed through the API).

Run **only this spec** while iterating — a full sweep is ~10 minutes:

```bash
WDIO_SERVER_MODE=1 npx wdio run tests/integration/wdio.server.conf.ts --spec tests/integration/specs/tier2/route-map.spec.ts
```

**Commit**

```bash
git add tests/integration/specs/tier2/route-map.spec.ts
git commit -m "test(route-map): add tier 2 integration tests"
```

---

## Task 20: Documentation

**Files:**
- Modify: [DECISIONS.md](../../DECISIONS.md)
- Modify: [CHANGELOG.md](../../CHANGELOG.md)
- Create: `docs/features/route-maps.md`
- Modify: [_tasks/index.md](../index.md)

**Step 1:** Add **ADR-028** (next free id; 027 is current highest) recording the disposable-cache boundary — why the polyline lives in the database and rendered PNGs do not, and what that buys for Move Database, backups and [Task 32](../32-portable-csv-backup/).

**Step 2:** Add **ADR-029** recording coordinate-based waypoints — why `node_idx` is optional metadata rather than the primary key of a waypoint, and how that keeps the V2 editor migration-free.

Use `/decision` for both.

**Step 3:** Update the changelog via `/changelog` — user-visible: the row action, the map view, and map pages in the print export.

**Step 4:** Write `docs/features/route-maps.md` per [docs/CLAUDE.md](../../docs/CLAUDE.md): user flow, technical implementation, design rationale.

**Step 5:** Flip task 70 to ✅ in [index.md](../index.md) and move the folder to `_done/`.

**Step 6: Full verification**

```bash
npm run test:backend
npm run test:integration
```

Then run `/verify`.

**Step 7: Commit**

```bash
git add DECISIONS.md CHANGELOG.md docs/features/route-maps.md _tasks/index.md
git commit -m "docs(route-map): add ADRs, changelog and feature documentation"
```

---

## Deferred to V2

Recorded here so nobody implements them by accident:

- Manual waypoint editing (drag, insert, remove) in the map view.
- Honouring a row's origin and destination — start ≠ end routes, geocoding free-text place names.
- Desktop UI: add `#[tauri::command]` wrappers for the four route-map commands and register them in the desktop `invoke_handler`, flip `routeMaps` to `true` in `defaultDesktop`, and wrap `/mapa` in a full-screen overlay. Only the wrappers are new code — core already holds every `_internal` fn they would call, and desktop export already renders attachments (Task 14).
- Multi-session split for trips longer than the dataset can reach.
