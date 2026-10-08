**Date:** 2026-10-08
**Subject:** Loop generator around the trip's place (home + Bratislava districts), numbered waypoints
**Status:** Complete

# Loop Generator Bratislava Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A loop trip generates its loop around the trip's own place: the home set near home, the 17 Bratislava districts in Bratislava, an error elsewhere. The map numbers the vias.

**Architecture:** A pure area selector ([areas.rs](../../src-tauri/core/src/route_map/areas.rs), new) maps the anchor coordinate to a candidate set. The Bratislava set is the anchor at index 0 plus 17 bundled district points. Its matrix comes from one `table` call on the `RouteProvider` trait (OSRM `/table`, a straight-line mock offline). The GA does not change. The dispatcher reads the anchor from the DB before the async work. The page sends `tripId` and draws numbers in the handles.

**Tech Stack:** Rust (kniha-jazd-core, reqwest, serde, async-trait), SvelteKit + Leaflet, WebdriverIO.

**Spec:** [01-task.md](./01-task.md)

## Global Constraints

- Areas: home = straight-line distance to home node 0 <= 5 km; Bratislava = straight-line distance to `48.1486, 17.1077` <= 18 km. Named constants in one file.
- Home set: the bundled 67-node file, without changes.
- The matrix for a non-home set always comes from OSRM (or the mock), also when Sygic is selected.
- A missing matrix cell is an error. The GA never runs with a missing distance.
- Error marker `NO_LOOP_CANDIDATES` at the start of the error text. Slovak UI text: "Pre toto miesto generátor nemá kandidátov."
- Numbers on vias only, in waypoint-list order. Round trip: the return leg continues the numbers.
- No migration, no new env var, no image change.
- Prose in docs: ASD-STE100, keyboard-typable characters only.

## Review Focus

1. A place just outside both radii -> the `NO_LOOP_CANDIDATES` error, never a home loop. Test: Task 1 `selects_no_area_far_away` and the boundary tests.
2. A loop place without coordinates -> the place dialog opens, no crash. Test: Task 3 `loop_anchor_of_an_unplaced_place_is_none`; the page checks `plan.origin` (Task 4).
3. An OSRM `/table` answer with a `null` cell or a non-`Ok` code -> an error. Test: Task 2 `parse_table_rejects_a_null_cell`, `parse_table_rejects_a_non_ok_code`.
4. A saved Bratislava loop records the Bratislava file version, not the home version. Test: Task 3 `a_saved_bratislava_loop_records_the_bratislava_version`.
5. A very short target in Bratislava (5 km) -> a route with the deviation flag, no panic. Test: Task 3 `a_short_bratislava_target_still_returns_a_route`.

---

### Task 1: Bratislava district file and the area selector

**Files:**
- Create: `src-tauri/core/assets/bratislava.json`
- Create: `src-tauri/core/src/route_map/areas.rs`, `src-tauri/core/src/route_map/areas_tests.rs`
- Create: `_tasks/91-loop-generator-bratislava/fetch-districts.sh` (how the file was made)
- Modify: `src-tauri/core/src/route_map/dataset.rs`, `src-tauri/core/src/route_map/dataset_tests.rs`, `src-tauri/core/src/route_map/mod.rs`

**Interfaces:**
- Produces: `areas::LoopArea { Home, Bratislava }`, `areas::loop_area(lat: f64, lon: f64) -> Option<LoopArea>`, `areas::haversine_km(a: (f64, f64), b: (f64, f64)) -> f64`, `areas::NO_LOOP_CANDIDATES: &str`, `areas::HOME_RADIUS_KM`, `areas::BRATISLAVA_CENTRE`, `areas::BRATISLAVA_RADIUS_KM`.
- Produces: `Dataset::bratislava_districts() -> (Vec<Node>, String)` (nodes with `idx` 1..=17, version), `Dataset::anchored(anchor: Node, candidates: Vec<Node>, matrix: Vec<Vec<f64>>, version: String) -> Dataset`.

- [ ] **Step 1: Write the district file.** Source: Nominatim (Overpass returned 504 on 2026-10-08), the `boundary=administrative` point of each `Bratislava-<district>` search. Points fetched on 2026-10-08:

| idx | name | lat | lon |
|---|---|---|---|
| 1 | Staré Mesto | 48.14059 | 17.11233 |
| 2 | Ružinov | 48.14931 | 17.16450 |
| 3 | Vrakuňa | 48.14520 | 17.20049 |
| 4 | Podunajské Biskupice | 48.13004 | 17.20742 |
| 5 | Nové Mesto | 48.16716 | 17.13835 |
| 6 | Rača | 48.21179 | 17.15343 |
| 7 | Vajnory | 48.20491 | 17.20658 |
| 8 | Karlova Ves | 48.15924 | 17.05268 |
| 9 | Dúbravka | 48.18709 | 17.03750 |
| 10 | Lamač | 48.19120 | 17.05428 |
| 11 | Devín | 48.17459 | 16.98261 |
| 12 | Devínska Nová Ves | 48.20918 | 16.97389 |
| 13 | Záhorská Bystrica | 48.23929 | 17.03834 |
| 14 | Petržalka | 48.11097 | 17.11129 |
| 15 | Jarovce | 48.06524 | 17.11294 |
| 16 | Rusovce | 48.05265 | 17.14692 |
| 17 | Čunovo | 48.02964 | 17.19957 |

File shape (same `nodes` shape as `villages.json`, `kind` = `"district"`, names prefixed `Bratislava-`):

```json
{
  "area": "Bratislava",
  "source": "OpenStreetMap via Nominatim, boundary=administrative point of each city district",
  "generatedAt": "2026-10-08",
  "nodes": [ { "idx": 1, "name": "Bratislava-Staré Mesto", "lat": 48.14059, "lon": 17.11233, "kind": "district" } ]
}
```

- [ ] **Step 2: Write failing tests** in `dataset_tests.rs`:

```rust
#[test]
fn bratislava_file_holds_the_17_districts() {
    let (nodes, version) = Dataset::bratislava_districts();
    assert_eq!(nodes.len(), 17);
    assert_eq!(version, "2026-10-08");
    assert!(nodes.iter().all(|n| n.kind == "district"));
    let idx: Vec<usize> = nodes.iter().map(|n| n.idx).collect();
    assert_eq!(idx, (1..=17).collect::<Vec<_>>());
}

#[test]
fn an_anchored_dataset_puts_the_anchor_at_index_0() {
    let (nodes, version) = Dataset::bratislava_districts();
    let anchor = Node { idx: 0, name: "Kancelária".into(), lat: 48.15, lon: 17.11, kind: "home".into() };
    let n = nodes.len() + 1;
    let ds = Dataset::anchored(anchor, nodes, vec![vec![1.0; n]; n], version);
    assert_eq!(ds.len(), 18);
    assert_eq!(ds.nodes[0].name, "Kancelária");
    assert_eq!(ds.nodes[0].idx, 0);
    assert_eq!(ds.nodes[17].idx, 17);
}
```

and in `areas_tests.rs`:

```rust
use super::*;
use crate::route_map::Dataset;

fn home() -> (f64, f64) { let n = &Dataset::bundled().nodes[0]; (n.lat, n.lon) }
/// A point `km` north of `p` (1 degree of latitude is 111.195 km).
fn north(p: (f64, f64), km: f64) -> (f64, f64) { (p.0 + km / 111.195, p.1) }

#[test] fn selects_home_at_home() { let h = home(); assert_eq!(loop_area(h.0, h.1), Some(LoopArea::Home)); }
#[test] fn selects_home_just_inside_the_radius() { let p = north(home(), HOME_RADIUS_KM - 0.1); assert_eq!(loop_area(p.0, p.1), Some(LoopArea::Home)); }
#[test] fn selects_nothing_just_outside_the_home_radius() { let p = north(home(), HOME_RADIUS_KM + 0.1); assert_eq!(loop_area(p.0, p.1), None); }
#[test] fn selects_bratislava_in_the_centre() { assert_eq!(loop_area(48.1486, 17.1077), Some(LoopArea::Bratislava)); }
#[test] fn selects_bratislava_just_inside_the_radius() { let p = north(BRATISLAVA_CENTRE, BRATISLAVA_RADIUS_KM - 0.1); assert_eq!(loop_area(p.0, p.1), Some(LoopArea::Bratislava)); }
#[test] fn selects_nothing_just_outside_the_bratislava_radius() { let p = north(BRATISLAVA_CENTRE, BRATISLAVA_RADIUS_KM + 0.1); assert_eq!(loop_area(p.0, p.1), None); }
#[test] fn selects_no_area_far_away() { assert_eq!(loop_area(49.2231, 18.7394), None); } // Žilina
#[test] fn every_district_is_inside_the_bratislava_area() {
    for n in Dataset::bratislava_districts().0 { assert_eq!(loop_area(n.lat, n.lon), Some(LoopArea::Bratislava), "{}", n.name); }
}
#[test] fn haversine_is_zero_on_one_point_and_about_44_km_ba_to_trnava() {
    assert_eq!(haversine_km((48.0, 17.0), (48.0, 17.0)), 0.0);
    let d = haversine_km((48.1486, 17.1077), (48.3774, 17.5872));
    assert!((d - 43.7).abs() < 0.5, "got {d}");
}
```

- [ ] **Step 3: Run, expect compile failure:** `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core route_map::` -> FAIL (`areas` / `bratislava_districts` not found).

- [ ] **Step 4: Implement.** `areas.rs`:

```rust
//! Which candidate set a loop uses, from the place it starts at (task 91).

use super::Dataset;

/// Stable marker at the start of the "no candidates here" error. The page
/// matches it (`NO_LOOP_CANDIDATES` in src/routes/mapa/+page.svelte).
pub const NO_LOOP_CANDIDATES: &str = "NO_LOOP_CANDIDATES";

pub const HOME_RADIUS_KM: f64 = 5.0;
pub const BRATISLAVA_CENTRE: (f64, f64) = (48.1486, 17.1077);
pub const BRATISLAVA_RADIUS_KM: f64 = 18.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopArea { Home, Bratislava }

/// Great-circle distance in km between two `(lat, lon)` points.
pub fn haversine_km(a: (f64, f64), b: (f64, f64)) -> f64 {
    const R: f64 = 6371.0;
    let (la1, la2) = (a.0.to_radians(), b.0.to_radians());
    let dla = la2 - la1;
    let dlo = (b.1 - a.1).to_radians();
    let h = (dla / 2.0).sin().powi(2) + la1.cos() * la2.cos() * (dlo / 2.0).sin().powi(2);
    2.0 * R * h.sqrt().asin()
}

/// `None` means the generator has no candidates for this place.
pub fn loop_area(lat: f64, lon: f64) -> Option<LoopArea> {
    let ds = Dataset::bundled();
    let home = (ds.nodes[0].lat, ds.nodes[0].lon);
    if haversine_km((lat, lon), home) <= HOME_RADIUS_KM {
        Some(LoopArea::Home)
    } else if haversine_km((lat, lon), BRATISLAVA_CENTRE) <= BRATISLAVA_RADIUS_KM {
        Some(LoopArea::Bratislava)
    } else {
        None
    }
}
```

`dataset.rs`: add `const BRATISLAVA_JSON: &str = include_str!("../../assets/bratislava.json");`, a `#[derive(Deserialize)] struct AreaFile { #[serde(rename = "generatedAt")] generated_at: String, nodes: Vec<Node> }`, and:

```rust
    /// The Bratislava city districts (task 91), indices 1..=17. No matrix:
    /// it depends on the anchor, see [`Dataset::anchored`].
    pub fn bratislava_districts() -> (Vec<Node>, String) {
        let f: AreaFile =
            serde_json::from_str(BRATISLAVA_JSON).expect("bundled bratislava.json must parse");
        (f.nodes, f.generated_at)
    }

    /// The anchor at index 0, then `candidates` in file order. `matrix` must be
    /// square over exactly these nodes, in this order.
    pub fn anchored(anchor: Node, candidates: Vec<Node>, matrix: Vec<Vec<f64>>, version: String) -> Self {
        let mut nodes = Vec::with_capacity(candidates.len() + 1);
        nodes.push(Node { idx: 0, ..anchor });
        nodes.extend(candidates);
        Self { nodes, matrix, version }
    }
```

Update the module doc of `dataset.rs` to name both files. `mod.rs`: `pub mod areas;` and the `#[cfg(test)] #[path = "areas_tests.rs"] mod areas_tests;` block.

- [ ] **Step 5: Run, expect PASS:** same command.

- [ ] **Step 6: Commit** `feat(route-map): Bratislava district set and loop area selector`.

### Task 2: The matrix call (`table`) on the provider

**Files:**
- Modify: `src-tauri/core/src/route_map/osrm.rs`, `src-tauri/core/src/route_map/osrm_tests.rs`, `src-tauri/core/src/route_map/provider.rs`, `src-tauri/core/src/route_map/provider_tests.rs`

**Interfaces:**
- Consumes: `areas::haversine_km`.
- Produces: `RouteProvider::table(&self, coords: &[(f64, f64)]) -> Result<Vec<Vec<f64>>, String>` (km; `coords` are `(lat, lon)`), `osrm::parse_table(body: &str) -> Result<Vec<Vec<f64>>, String>`, `HttpRouteProvider::table_url(&self, coords) -> String`.

- [ ] **Step 1: Write failing tests** in `osrm_tests.rs`:

```rust
#[test]
fn parse_table_converts_metres_to_km() {
    let m = parse_table(r#"{"code":"Ok","distances":[[0,1500.0],[2000.0,0]]}"#).unwrap();
    assert_eq!(m, vec![vec![0.0, 1.5], vec![2.0, 0.0]]);
}
#[test]
fn parse_table_rejects_a_null_cell() {
    let e = parse_table(r#"{"code":"Ok","distances":[[0,null],[2000.0,0]]}"#).unwrap_err();
    assert!(e.contains("no road"), "{e}");
}
#[test]
fn parse_table_rejects_a_non_ok_code() {
    let e = parse_table(r#"{"code":"TooBig","message":"Too many table coordinates"}"#).unwrap_err();
    assert!(e.contains("TooBig"), "{e}");
}
#[test]
fn table_url_sends_lon_lat_and_asks_for_distances() {
    let p = HttpRouteProvider::new("http://x");
    assert_eq!(
        p.table_url(&[(48.1, 17.1), (48.2, 17.2)]),
        "http://x/table/v1/driving/17.100000,48.100000;17.200000,48.200000?annotations=distance"
    );
}
```

and in `provider_tests.rs`:

```rust
#[tokio::test]
async fn mock_table_is_straight_line_times_1_3() {
    let p = build_provider(ProviderConfig::Mock, None, vec![]).unwrap();
    let a = (48.1486, 17.1077);
    let b = (48.3774, 17.5872);
    let m = p.table(&[a, b]).await.unwrap();
    assert_eq!(m[0][0], 0.0);
    let want = crate::route_map::areas::haversine_km(a, b) * 1.3;
    assert!((m[0][1] - want).abs() < 1e-9);
    assert!((m[1][0] - want).abs() < 1e-9);
}
```

- [ ] **Step 2: Run, expect FAIL** (`parse_table` not found): `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core route_map::`

- [ ] **Step 3: Implement** in `osrm.rs`:

```rust
#[derive(Deserialize)]
struct TableResponse {
    code: String,
    message: Option<String>,
    distances: Option<Vec<Vec<Option<f64>>>>,
}

/// An OSRM `/table` body to a km matrix. A `null` cell (no road between two
/// points) is an error: the GA must never treat it as 0 km.
pub fn parse_table(body: &str) -> Result<Vec<Vec<f64>>, String> {
    let r: TableResponse = serde_json::from_str(body)
        .map_err(|e| format!("Unreadable distance table from the routing service: {e}"))?;
    if r.code != "Ok" {
        return Err(format!("The routing service refused the distance table: {} {}", r.code, r.message.unwrap_or_default()));
    }
    let rows = r.distances.ok_or("The distance table has no distances")?;
    rows.into_iter()
        .map(|row| row.into_iter()
            .map(|c| c.map(|m| m / 1000.0).ok_or_else(|| "The distance table has a pair of points with no road between them".to_string()))
            .collect())
        .collect()
}
```

On the trait, after `fetch_alternatives`:

```rust
    /// Driving-distance matrix in km between all `coords` (`(lat, lon)`),
    /// row = from, column = to. Defaulted to the public OSRM server, so a
    /// provider without its own matrix (Sygic) uses OSRM (task 91).
    async fn table(&self, coords: &[(f64, f64)]) -> Result<Vec<Vec<f64>>, String> {
        HttpRouteProvider::public().table_matrix(coords).await
    }
```

On `HttpRouteProvider`: `table_url` (same lon,lat flip as `route_url`, `?annotations=distance`), and `pub async fn table_matrix(&self, coords)` that GETs it with `self.client` and calls `parse_table` on the body (map errors the same way `fetch` does). Implement `table` in the `impl RouteProvider for HttpRouteProvider` block as `self.table_matrix(coords).await`, so a custom `base_url` is used.

In `provider.rs`, `impl RouteProvider for MockRouteProvider`:

```rust
    async fn table(&self, coords: &[(f64, f64)]) -> Result<Vec<Vec<f64>>, String> {
        Ok(coords.iter()
            .map(|&a| coords.iter().map(|&b| super::areas::haversine_km(a, b) * 1.3).collect())
            .collect())
    }
```

- [ ] **Step 4: Run, expect PASS.**

- [ ] **Step 5: Commit** `feat(route-map): driving-distance matrix call on the route provider`.

### Task 3: Generate around the anchor; stamp the right version on save

**Files:**
- Modify: `src-tauri/core/src/commands_internal/route_maps.rs`, `src-tauri/core/src/commands_internal/route_maps_tests.rs`, `src-tauri/core/src/server/dispatcher_async.rs`, `src-tauri/core/src/commands_internal/mod.rs` (only if a re-export is needed)

**Interfaces:**
- Consumes: Task 1 and Task 2 names.
- Produces: `pub struct LoopAnchor { pub lat: f64, pub lon: f64, pub name: String }`, `pub fn loop_anchor_internal(db: &Database, trip_id: &str) -> Result<Option<LoopAnchor>, String>` (`None` = the place has no position), `pub async fn loop_dataset(provider: &dyn RouteProvider, anchor: &LoopAnchor) -> Result<Dataset, String>`, `pub async fn generate_route_internal(provider: &dyn RouteProvider, anchor: &LoopAnchor, target_km: f64) -> Result<GeneratedRoute, String>`, `fn loop_dataset_version(first: Option<&Waypoint>) -> Option<String>`. RPC: `generate_route { tripId, targetKm, avoid, provider }`.

- [ ] **Step 1: Write failing tests** in `route_maps_tests.rs`. Add a `table` override on `StubProvider` that returns straight-line x 1.3 and counts calls (`table_calls: std::sync::atomic::AtomicUsize`, add the field and set it to `0.into()` in `encoding`), so no test reaches the network. Then:

```rust
fn home_anchor() -> LoopAnchor {
    let n = &Dataset::bundled().nodes[0];
    LoopAnchor { lat: n.lat, lon: n.lon, name: "Domov".into() }
}
fn ba_anchor() -> LoopAnchor { LoopAnchor { lat: 48.1530, lon: 17.1200, name: "Kancelária BA".into() } }

#[tokio::test]
async fn a_home_anchor_uses_the_bundled_set_without_a_table_call() {
    let provider = StubProvider::encoding(&sample_geometry().0, 117.2);
    let ds = loop_dataset(&provider, &home_anchor()).await.unwrap();
    assert_eq!(ds.len(), 67);
    assert_eq!(provider.table_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_bratislava_anchor_uses_the_districts_with_one_table_call() {
    let provider = StubProvider::encoding(&sample_geometry().0, 43.0);
    let ds = loop_dataset(&provider, &ba_anchor()).await.unwrap();
    assert_eq!(ds.len(), 18);
    assert_eq!(ds.nodes[0].name, "Kancelária BA");
    assert_eq!((ds.nodes[0].lat, ds.nodes[0].lon), (48.1530, 17.1200));
    assert_eq!(ds.matrix.len(), 18);
    assert_eq!(ds.version, Dataset::bratislava_districts().1);
    assert_eq!(provider.table_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn an_anchor_outside_both_areas_is_refused_with_the_marker() {
    let provider = StubProvider::encoding(&sample_geometry().0, 43.0);
    let zilina = LoopAnchor { lat: 49.2231, lon: 18.7394, name: "Žilina".into() };
    let err = generate_route_internal(&provider, &zilina, 43.0).await.unwrap_err();
    assert!(err.starts_with(NO_LOOP_CANDIDATES), "{err}");
}

#[tokio::test]
async fn a_bratislava_loop_starts_and_ends_at_the_anchor() {
    let provider = StubProvider::encoding(&sample_geometry().0, 43.0);
    let route = generate_route_internal(&provider, &ba_anchor(), 43.0).await.unwrap();
    let (first, last) = (route.waypoints.first().unwrap(), route.waypoints.last().unwrap());
    assert_eq!((first.lat, first.lon), (48.1530, 17.1200));
    assert_eq!((last.lat, last.lon), (48.1530, 17.1200));
    assert!(route.waypoints.len() >= 3);
    assert!(route.waypoints[1..route.waypoints.len() - 1].iter()
        .all(|w| w.name.as_deref().unwrap_or("").starts_with("Bratislava-")));
    assert_eq!(route.dataset_version, Some(Dataset::bratislava_districts().1));
}

#[tokio::test]
async fn a_short_bratislava_target_still_returns_a_route() {
    let provider = StubProvider::encoding(&sample_geometry().0, 9.0);
    let route = generate_route_internal(&provider, &ba_anchor(), 5.0).await.unwrap();
    assert!(route.off_target, "9 km for a 5 km target is outside tolerance");
}

#[test]
fn loop_anchor_comes_from_the_trip_origin_place() {
    let db = Database::in_memory().unwrap();
    let trip = seed_trip_between(&db, "Kancelária BA", "Kancelária BA");
    db.set_place_position(&trip.origin_place_id.to_string(), 48.153, 17.12, "manual").unwrap();
    let a = loop_anchor_internal(&db, &trip.id.to_string()).unwrap().unwrap();
    assert_eq!((a.lat, a.lon, a.name.as_str()), (48.153, 17.12, "Kancelária BA"));
}

#[test]
fn loop_anchor_of_an_unplaced_place_is_none() {
    let db = Database::in_memory().unwrap();
    db.ensure_unplaced_place_for_test("Nikde");
    let trip = seed_trip_between(&db, "Nikde", "Nikde");
    assert!(loop_anchor_internal(&db, &trip.id.to_string()).unwrap().is_none());
}

#[test]
fn a_saved_bratislava_loop_records_the_bratislava_version() {
    let wps = vec![
        Waypoint { lat: 48.153, lon: 17.12, name: Some("Kancelária BA".into()), node_idx: Some(0) },
        Waypoint { lat: 48.11097, lon: 17.11129, name: Some("Bratislava-Petržalka".into()), node_idx: Some(14) },
        Waypoint { lat: 48.153, lon: 17.12, name: Some("Kancelária BA".into()), node_idx: Some(0) },
    ];
    let map = build_route_map("00000000-0000-0000-0000-000000000001", wps, "x".into(), 43.0,
        RouteMode::Loop, false, None, vec![], None).unwrap();
    assert_eq!(map.dataset_version, Some(Dataset::bratislava_districts().1));
}
```

Change the two existing tests that call `generate_route_internal(&provider, 120.0)` to `generate_route_internal(&provider, &home_anchor(), 120.0)`. The existing save test that expects `Dataset::bundled().version` keeps working, because its waypoints start at home (check `sample_waypoints()`; if they do not start within 5 km of home, change the expectation to what `loop_dataset_version` returns for them and say so in the commit).

Update the dispatcher test `generate_route_is_an_async_command_taking_target_km`: a call without `tripId` must fail during argument parsing.

- [ ] **Step 2: Run, expect FAIL** (missing names): `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core route_maps`

- [ ] **Step 3: Implement** in `route_maps.rs`:

```rust
/// Where a loop starts and ends: the trip's origin place (task 91).
#[derive(Debug, Clone)]
pub struct LoopAnchor { pub lat: f64, pub lon: f64, pub name: String }

/// The trip's origin place as a loop anchor. `Ok(None)`: the place has no
/// position; the page opens the place dialog before it asks to generate.
pub fn loop_anchor_internal(db: &Database, trip_id: &str) -> Result<Option<LoopAnchor>, String> {
    let trip = db.get_trip(trip_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip not found: {trip_id}"))?;
    let places = list_places_internal(db)?;
    Ok(placed_endpoint(&places, trip.origin_place_id).map(|p| LoopAnchor {
        lat: p.lat.expect("placed_endpoint keeps only placed rows"),
        lon: p.lon.expect("placed_endpoint keeps only placed rows"),
        name: p.name,
    }))
}

/// The candidate set for a loop around `anchor`.
pub async fn loop_dataset(provider: &dyn RouteProvider, anchor: &LoopAnchor) -> Result<Dataset, String> {
    match loop_area(anchor.lat, anchor.lon) {
        Some(LoopArea::Home) => Ok(Dataset::bundled()),
        Some(LoopArea::Bratislava) => {
            let (districts, version) = Dataset::bratislava_districts();
            let start = Node { idx: 0, name: anchor.name.clone(), lat: anchor.lat, lon: anchor.lon, kind: "home".into() };
            let mut coords = vec![(anchor.lat, anchor.lon)];
            coords.extend(districts.iter().map(|n| (n.lat, n.lon)));
            let matrix = provider.table(&coords).await?;
            Ok(Dataset::anchored(start, districts, matrix, version))
        }
        None => Err(format!(
            "{NO_LOOP_CANDIDATES}: The loop generator has no candidate places near {} ({:.4}, {:.4}).",
            anchor.name, anchor.lat, anchor.lon
        )),
    }
}

/// The version of the set a saved loop came from, found from its start point.
fn loop_dataset_version(first: Option<&Waypoint>) -> Option<String> {
    match first.and_then(|w| loop_area(w.lat, w.lon)) {
        Some(LoopArea::Home) | None => Some(Dataset::bundled().version),
        Some(LoopArea::Bratislava) => Some(Dataset::bratislava_districts().1),
    }
}
```

(`None` keeps today's value for a legacy loop: every loop saved before task 91 started at home.) In `build_route_map`, compute `let version = loop_dataset_version(waypoints.first());` before `waypoints` moves, and use it in the `RouteMode::Loop` arm. `generate_route_internal` takes `anchor: &LoopAnchor` and calls `loop_dataset(provider, anchor).await?` instead of `Dataset::bundled()`. Import `Node` from `crate::route_map::dataset` and `loop_area, LoopArea, NO_LOOP_CANDIDATES` from `crate::route_map::areas`.

In `dispatcher_async.rs`, the `generate_route` arm: add `trip_id: String` to `Args`, then before building the provider:

```rust
let anchor = match crate::commands_internal::loop_anchor_internal(&state.db, &a.trip_id) {
    Ok(Some(anchor)) => anchor,
    Ok(None) => return Some(Err("The loop place has no position. Place it on the map first.".into())),
    Err(e) => return Some(Err(e)),
};
```

and call `generate_route_internal(provider.as_ref(), &anchor, a.target_km)`. Match the arm's real local names when you edit it.

- [ ] **Step 4: Run, expect PASS**, then the whole workspace: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`.

- [ ] **Step 5: Commit** `feat(route-map): generate the loop around the trip's place`.

### Task 4: The page sends `tripId`, handles the two new cases, and numbers the vias

**Files:**
- Modify: `src/lib/api.ts`, `src/routes/mapa/+page.svelte`, `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`, `src/lib/i18n/i18n-types.ts` (generated)
- Test: `tests/integration/specs/tier2/route-map.spec.ts`

**Interfaces:**
- Consumes: RPC `generate_route { tripId, targetKm, avoid, provider }`, error marker `NO_LOOP_CANDIDATES`.
- Produces: `.wp-handle .wp-num` elements with the via number as text.

- [ ] **Step 1: Write the failing integration test** in the `Map View (V2, offline-reachable flows)` block. Seeded places sit at `48.15, 17.11` (`ensurePlace` in `tests/integration/utils/db.ts`), which is inside the Bratislava area, and the mock router serves `table`:

```ts
    it('generates a Bratislava loop with numbered vias', async () => {
      const trip = await seedTrip({
        vehicleId,
        startDatetime: '2026-03-15T08:00',
        endDatetime: '2026-03-15T09:00',
        origin: 'Bratislava',
        destination: 'Bratislava',
        distanceKm: 43,
        odometer: 50043,
        purpose: 'Business trip',
      });

      await openMap(trip.id as string);
      await waitForMapOutcome('route');

      const nums = await $$('[data-test="route-map-canvas"] .wp-handle .wp-num').map((el) => el.getText());
      expect(nums.length).toBeGreaterThan(0);
      expect(nums).toEqual(nums.map((_, i) => String(i + 1)));
    });
```

- [ ] **Step 2: Build and run it, expect FAIL** (no `.wp-num`):

```bash
npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
npx wdio run tests/integration/wdio.server.conf.ts --spec tests/integration/specs/tier2/route-map.spec.ts
```

- [ ] **Step 3: Implement.**
  - `api.ts`: `generateRoute(tripId: string, targetKm: number, avoid: string[] = [], provider: RouteProviderKind | null = null)` sends `{ tripId, targetKm, avoid, provider }`.
  - `+page.svelte` `runGenerate`: `generateRoute(tripId, targetKm, [], provider)`. In its `catch`, if `String(e)` includes `NO_LOOP_CANDIDATES` (a new constant next to `AVOID_NEEDS_SYGIC`, with the same "Same string as" comment pointing to `route_map/areas.rs`), set `error = $LL.routeMap.noLoopCandidates()` and `retryable = false`; else keep today's `$LL.routeMap.error()`.
  - `startForTrip`, loop branch: before `runGenerate`, `if (!plan.origin) { unplacedField = 'origin'; return; }`. `handlePlaceSaved` already calls `startForTrip()` again.
  - `handleIcon(L, endpoint, num?: number)`: when `num` is set, `html: '<span class="wp-num">' + num + '</span>'`, class `wp-handle wp-numbered`, `iconSize: [18, 18]`. `drawHandles`: vias get `num = i`. `drawLegHandles(points, leg, offset)`: vias get `num = offset + i`; the outbound call passes `0`, the inbound call passes the outbound via count (`baseWaypoints.length - 2`).
  - CSS: `:global(.wp-numbered)` centres the number (flex, `font-size: 10px`, `font-weight: 600`, `color: var(--bg-surface)`, `line-height: 1`).
  - i18n `routeMap.noLoopCandidates`: sk `'Pre toto miesto generátor nemá kandidátov.'`, en `'The loop generator has no candidate places near this place.'`. Run `npm run i18n`.

- [ ] **Step 4: Run, expect PASS:** `npm run check`, then the build and spec commands from Step 2.

- [ ] **Step 5: Commit** `feat(mapa): numbered vias, tripId for loop generation`.

### Task 5: Docs, full test run, local deploy

**Files:**
- Modify: `docs/features/route-maps.md`, `DECISIONS.md`, `CHANGELOG.md`, `_tasks/index.md`, this plan's `Status`.

- [ ] **Step 1:** `route-maps.md`: replace the "deferred limitation" paragraph, add the anchor and area selection to the loop data flow, add `areas.rs` and `bratislava.json` to the key-files tables, and the "short trips" section notes the Bratislava set.
- [ ] **Step 2:** `/decision`: ADR "A loop anchors at the trip's place; candidate sets are per area; a non-home matrix comes from one `/table` call at generation time".
- [ ] **Step 3:** `/changelog`: user-visible entry under [Unreleased], upgrade notes: no migration, no env var, no image change.
- [ ] **Step 4:** Full run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`, `npm run typecheck:tests`, `npm run test:integration`. All pass.
- [ ] **Step 5: Commit** `docs: loop generator around the trip's place (task 91)`.
- [ ] **Step 6: Local deploy:** `docker compose -f docker-compose.web.yml up -d --build`. Check `get_app_version` and `get_vehicles` on `http://localhost:3456/api/rpc`, then call `generate_route` for a Bratislava loop trip (real OSRM) and check that the waypoints are Bratislava districts.
