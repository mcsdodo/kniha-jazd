**Date:** 2026-09-29
**Subject:** Route map: avoid paid roads per country (Sygic routing provider)
**Status:** Complete

# Route Avoid Tolls Per Country Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On `/mapa`, the user checks "avoid paid roads: CZ" and the route
avoids the Czech vignette roads but keeps the Slovak ones.

**Architecture:** A new `SygicRouteProvider` implements the existing
`RouteProvider` trait. The provider gets the avoid list when it is built, and
it returns `possible_avoids` (the `*:tolls` values of the route, together with
its own avoid list) in each `FetchedRoute`. One factory function picks Sygic,
OSRM or a test mock from the environment. The route map commands pass
`possible_avoids` through as `avoidOptions`, and the save commands store the
avoid list in a new `trip_routes.avoid` column.

**Tech Stack:** Rust (`reqwest`, `serde`, `async-trait`, `wiremock` for tests),
Diesel + SQLite, SvelteKit 5 (runes), typesafe-i18n, WebdriverIO.

**Spec:** [01-task.md](./01-task.md)

## Global Constraints

- The avoid type is `tolls` only. An accepted avoid value matches `^[a-z]{3}:tolls$`.
- `SYGIC_API_KEY` set: Sygic computes all routes. Unset: public OSRM, and no avoid checkboxes.
- `SYGIC_REFERER` set: the Sygic request sends it as the `Referer` header.
- No silent fallback to OSRM. A non-empty avoid list with no Sygic key is an error.
- Sygic takes `lat,lon` with 6 decimals. OSRM takes `lon,lat`. Do not mix them.
- Alternatives only for a request with exactly two points, in the order Sygic returns them, at most `MAX_ALTERNATIVES` (3).
- ADR-008: the value filter, the union and the provider choice are in Rust. The page only renders `avoidOptions` and sends the checked values back.
- The checkboxes show only in direct mode (one-way and round trip). In loop mode a click would run the GA again and replace the loop, so loop mode sends `avoid: []` and shows no checkboxes.
- The new `trip_routes` column is the LAST column. `RouteMapRow` binds by position.
- Do not write a key or a referer value into the repo. The repo is public.
- All user text through i18n (`sk` and `en`). Run `npm run i18n` after an i18n edit.
- Prose in docs: Simplified Technical English, no typographic glyphs (see the user's global rules).
- Run backend tests with `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "<filter>"`. Never `cd && cargo`.

## Review Focus

Five input classes plus one security rule.

1. **A saved route with an avoid list, reopened when `SYGIC_API_KEY` is gone.** Expect: the saved line still draws (no routing call). A recompute shows an error that names the missing key. It does not silently route through OSRM. Test in Task 4.
2. **A crafted RPC call with `avoid: ["cze:country"]` or `["x|y"]`.** Expect: an error, no request to Sygic. The value goes into a URL. Test in Task 1 and Task 5.
3. **The user checks CZ, and the new route no longer passes a CZ toll road.** Expect: the CZ checkbox stays visible and checked. Test in Task 3 (provider union) and Task 8 (UI).
4. **Sygic answers 403 `Allowed referers do not match.` or 401.** Expect: the error names Sygic and the status. The page shows the normal route error with Retry. Test in Task 3.
5. **The user checks a country, then saves.** Expect: the save stores the avoid list that produced the shown route (`routedAvoid`), not the newest checkbox state and not `[]`. Test in Task 8 (save, reopen, still checked).
6. **The Sygic key leaks into an error message.** The request URL carries `key=`, and reqwest can print the URL in an error. Expect: no error text contains the key. Test in Task 3.

---

## File Structure

| File | Change | Responsibility |
|---|---|---|
| [src-tauri/core/src/route_map/avoid.rs](../../../src-tauri/core/src/route_map/avoid.rs) | Create | Pure functions: validate the avoid list, filter `*:tolls`, merge option lists. |
| [src-tauri/core/src/route_map/avoid_tests.rs](../../../src-tauri/core/src/route_map/avoid_tests.rs) | Create | Tests for `avoid.rs`. |
| [src-tauri/core/src/route_map/osrm.rs](../../../src-tauri/core/src/route_map/osrm.rs) | Modify | `FetchedRoute.possible_avoids`. OSRM fills it with `[]`. |
| [src-tauri/core/src/route_map/sygic.rs](../../../src-tauri/core/src/route_map/sygic.rs) | Create | `SygicRouteProvider`: URL, headers, response mapping. |
| [src-tauri/core/src/route_map/sygic_tests.rs](../../../src-tauri/core/src/route_map/sygic_tests.rs) | Create | `wiremock` tests for Sygic. |
| [src-tauri/core/src/route_map/provider.rs](../../../src-tauri/core/src/route_map/provider.rs) | Create | `ProviderConfig` from env, `route_provider(avoid)` factory, `MockRouteProvider`. |
| [src-tauri/core/src/route_map/provider_tests.rs](../../../src-tauri/core/src/route_map/provider_tests.rs) | Create | Tests for the factory and the mock. |
| [src-tauri/core/src/route_map/mod.rs](../../../src-tauri/core/src/route_map/mod.rs) | Modify | Register the new modules. |
| [src-tauri/core/src/constants.rs](../../../src-tauri/core/src/constants.rs) | Modify | Env var names. |
| [src-tauri/core/src/commands_internal/route_maps.rs](../../../src-tauri/core/src/commands_internal/route_maps.rs) | Modify | `avoid_options` on `GeneratedRoute` and `RoundTripRoutes`. `avoid` on save, get. |
| [src-tauri/core/src/commands_internal/route_maps_tests.rs](../../../src-tauri/core/src/commands_internal/route_maps_tests.rs) | Modify | Tests for the pass-through and the round trip union. |
| [src-tauri/core/src/server/dispatcher_async.rs](../../../src-tauri/core/src/server/dispatcher_async.rs) | Modify | `avoid` arg on 3 commands, factory in place of `HttpRouteProvider::public()`. |
| [src-tauri/core/src/server/dispatcher.rs](../../../src-tauri/core/src/server/dispatcher.rs) | Modify | `avoid` arg on 2 save commands. |
| [src-tauri/core/migrations/2026-09-29-100000_add_trip_route_avoid/](../../../src-tauri/core/migrations/2026-09-29-100000_add_trip_route_avoid/) | Create | `up.sql`, `down.sql`. |
| [src-tauri/core/src/schema.rs](../../../src-tauri/core/src/schema.rs), `models.rs`, `db.rs` | Modify | The `avoid` column end to end. |
| [src-tauri/core/src/db_tests.rs](../../../src-tauri/core/src/db_tests.rs), `migration_tests.rs` | Modify | Round trip of the column, backfill to `[]`. |
| [src/lib/types.ts](../../../src/lib/types.ts), [src/lib/api.ts](../../../src/lib/api.ts) | Modify | `avoidOptions`, `avoid`, new API args. |
| [src/routes/mapa/+page.svelte](../../../src/routes/mapa/+page.svelte) | Modify | Avoid state, checkboxes, capture of `routedAvoid`. |
| [src/lib/i18n/sk/index.ts](../../../src/lib/i18n/sk/index.ts), [src/lib/i18n/en/index.ts](../../../src/lib/i18n/en/index.ts) | Modify | Labels and country names. |
| [tests/integration/wdio.server.conf.ts](../../../tests/integration/wdio.server.conf.ts), [.github/workflows/test.yml](../../../.github/workflows/test.yml) | Modify | Mock router env, scrub the Sygic key. |
| [tests/integration/specs/tier2/route-map.spec.ts](../../../tests/integration/specs/tier2/route-map.spec.ts) | Modify | Two new tests. |
| [DECISIONS.md](../../../DECISIONS.md), [CHANGELOG.md](../../../CHANGELOG.md), [docs/features/route-maps.md](../../../docs/features/route-maps.md), [CLAUDE.md](../../../CLAUDE.md), [README.md](../../../README.md), [README.en.md](../../../README.en.md) | Modify | Docs. |

---

### Task 1: Avoid value rules (`avoid.rs`)

**Files:**
- Create: [src-tauri/core/src/route_map/avoid.rs](../../../src-tauri/core/src/route_map/avoid.rs)
- Create: [src-tauri/core/src/route_map/avoid_tests.rs](../../../src-tauri/core/src/route_map/avoid_tests.rs)
- Modify: [src-tauri/core/src/route_map/mod.rs](../../../src-tauri/core/src/route_map/mod.rs)

**Interfaces:**
- Produces:
  - `pub fn normalise_avoid(values: Vec<String>) -> Result<Vec<String>, String>`: trims, lowercases, validates `^[a-z]{3}:tolls$`, sorts, dedupes.
  - `pub fn toll_options<'a>(possible: impl IntoIterator<Item = &'a str>, requested: &[String]) -> Vec<String>`: keeps the `*:tolls` values of `possible`, adds `requested`, sorts, dedupes.
  - `pub fn merge_options<'a>(lists: impl IntoIterator<Item = &'a [String]>) -> Vec<String>`: sorted union.

- [ ] **Step 1: Write the failing tests**

```rust
//! Tests for the avoid value rules.

use super::avoid::{merge_options, normalise_avoid, toll_options};

#[test]
fn normalise_accepts_tolls_values_and_sorts_them() {
    let got = normalise_avoid(vec!["svk:tolls".into(), " CZE:tolls ".into(), "svk:tolls".into()]).unwrap();
    assert_eq!(got, vec!["cze:tolls".to_string(), "svk:tolls".to_string()]);
}

#[test]
fn normalise_accepts_an_empty_list() {
    assert_eq!(normalise_avoid(vec![]).unwrap(), Vec::<String>::new());
}

#[test]
fn normalise_rejects_other_avoid_types() {
    // `highways` and `country` are real Sygic values, but this app offers
    // `tolls` only (01-task.md, decision 1).
    for bad in ["cze:highways", "cze:country", "tolls", "cz:tolls", "cze:tolls|svk:tolls", "cze:tolls&key=x", ""] {
        let err = normalise_avoid(vec![bad.to_string()]).expect_err(bad);
        assert!(err.contains("avoid"), "error should name the avoid value, got: {err}");
    }
}

#[test]
fn toll_options_keeps_only_tolls_and_adds_the_requested_values() {
    // Response for BA -> Brno with avoid=cze:tolls: `cze:tolls` is gone from
    // what Sygic offers, but the user still needs the checkbox to uncheck it.
    let possible = ["svk:highways", "svk:tolls", "svk:country", "cze:highways", "cze:country"];
    let got = toll_options(possible, &["cze:tolls".to_string()]);
    assert_eq!(got, vec!["cze:tolls".to_string(), "svk:tolls".to_string()]);
}

#[test]
fn merge_options_is_a_sorted_union() {
    let a = vec!["svk:tolls".to_string()];
    let b = vec!["cze:tolls".to_string(), "svk:tolls".to_string()];
    assert_eq!(
        merge_options([a.as_slice(), b.as_slice()]),
        vec!["cze:tolls".to_string(), "svk:tolls".to_string()]
    );
}
```

Register in `mod.rs` (after `pub mod dataset;` and with the other `#[cfg(test)]` blocks):

```rust
pub mod avoid;
```

```rust
#[cfg(test)]
#[path = "avoid_tests.rs"]
mod avoid_tests;
```

- [ ] **Step 2: Run the tests and see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core avoid_tests`
Expected: FAIL, `file not found for module avoid` or unresolved imports.

- [ ] **Step 3: Write the implementation**

```rust
//! Which "avoid" values the route map accepts and offers.
//!
//! Only `<iso3>:tolls` is supported: "paid roads" means the vignette sections,
//! and free highway sections stay allowed (01-task.md, decision 1). The value
//! is sent to the routing service in a URL, so anything else is refused before
//! a request is built.

const SUFFIX: &str = ":tolls";

fn is_toll_value(v: &str) -> bool {
    match v.strip_suffix(SUFFIX) {
        Some(iso) => iso.len() == 3 && iso.bytes().all(|b| b.is_ascii_lowercase()),
        None => false,
    }
}

/// Validate and canonicalise an avoid list from a caller.
pub fn normalise_avoid(values: Vec<String>) -> Result<Vec<String>, String> {
    let mut out = Vec::with_capacity(values.len());
    for raw in values {
        let v = raw.trim().to_ascii_lowercase();
        if !is_toll_value(&v) {
            return Err(format!(
                "Unsupported avoid value {raw:?}. Expected <country>:tolls, for example cze:tolls."
            ));
        }
        out.push(v);
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// The checkboxes a route offers: its own `*:tolls` values plus the values the
/// request already avoids. A checked country must stay visible even when the
/// new route no longer passes a toll road there.
pub fn toll_options<'a>(possible: impl IntoIterator<Item = &'a str>, requested: &[String]) -> Vec<String> {
    let mut out: Vec<String> = possible
        .into_iter()
        .filter(|v| is_toll_value(v))
        .map(str::to_string)
        .chain(requested.iter().cloned())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Sorted union of several option lists (a round trip has one per leg alternative).
pub fn merge_options<'a>(lists: impl IntoIterator<Item = &'a [String]>) -> Vec<String> {
    let mut out: Vec<String> = lists.into_iter().flatten().cloned().collect();
    out.sort();
    out.dedup();
    out
}
```

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core avoid_tests`
Expected: PASS, 5 tests.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/core/src/route_map/avoid.rs src-tauri/core/src/route_map/avoid_tests.rs src-tauri/core/src/route_map/mod.rs
git commit -m "feat(route-map): add avoid value rules for paid roads"
```

---

### Task 2: `FetchedRoute.possible_avoids`

**Files:**
- Modify: [src-tauri/core/src/route_map/osrm.rs](../../../src-tauri/core/src/route_map/osrm.rs) (struct `FetchedRoute`, the `.map(|route| FetchedRoute {...})` in `request`)
- Modify: [src-tauri/core/src/commands_internal/route_maps_tests.rs](../../../src-tauri/core/src/commands_internal/route_maps_tests.rs) (3 `FetchedRoute {` literals, including `fn fetched`)
- Modify: [src-tauri/core/src/route_map/osrm_tests.rs](../../../src-tauri/core/src/route_map/osrm_tests.rs)

**Interfaces:**
- Produces: `FetchedRoute { polyline: String, road_km: f64, duration_s: f64, possible_avoids: Vec<String> }`. Derive `Default` too, so test literals can use `..Default::default()`.

- [ ] **Step 1: Write the failing test** (append to `osrm_tests.rs`)

```rust
/// OSRM knows no countries, so it can offer no avoid values. An empty list is
/// what hides the checkboxes on the page.
#[tokio::test]
async fn osrm_offers_no_avoid_values() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/route/v1/driving/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "Ok",
            "routes": [{ "geometry": "_p~iF~ps|U_ulLnnqC", "distance": 1000.0, "duration": 60.0 }]
        })))
        .mount(&server)
        .await;

    let r = HttpRouteProvider::new(server.uri())
        .fetch(&[(48.935, 20.553), (48.997, 20.591)])
        .await
        .unwrap();
    assert!(r.possible_avoids.is_empty());
}
```

- [ ] **Step 2: Run it and see it fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core osrm_offers_no_avoid_values`
Expected: FAIL to compile, `no field possible_avoids on type FetchedRoute`.

- [ ] **Step 3: Implement**

In `osrm.rs`:

```rust
#[derive(Debug, Clone, Default)]
pub struct FetchedRoute {
    // ... existing fields unchanged ...
    /// The `<iso3>:tolls` values this route could avoid, plus the values the
    /// request already avoided (see `avoid::toll_options`). Always empty for
    /// OSRM, which knows no countries.
    pub possible_avoids: Vec<String>,
}
```

and in `request`:

```rust
            .map(|route| FetchedRoute {
                polyline: route.geometry,
                road_km: route.distance / 1000.0,
                duration_s: route.duration,
                possible_avoids: Vec::new(),
            })
```

In `route_maps_tests.rs`, add `..Default::default()` to each of the 3 `FetchedRoute { ... }` literals, for example:

```rust
fn fetched(polyline: &str, road_km: f64, duration_s: f64) -> FetchedRoute {
    FetchedRoute { polyline: polyline.into(), road_km, duration_s, ..Default::default() }
}
```

- [ ] **Step 4: Run all route map tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core route_map`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/core/src/route_map/osrm.rs src-tauri/core/src/route_map/osrm_tests.rs src-tauri/core/src/commands_internal/route_maps_tests.rs
git commit -m "feat(route-map): carry possible avoid values on a fetched route"
```

---

### Task 3: `SygicRouteProvider`

**Files:**
- Create: [src-tauri/core/src/route_map/sygic.rs](../../../src-tauri/core/src/route_map/sygic.rs)
- Create: [src-tauri/core/src/route_map/sygic_tests.rs](../../../src-tauri/core/src/route_map/sygic_tests.rs)
- Modify: [src-tauri/core/src/route_map/mod.rs](../../../src-tauri/core/src/route_map/mod.rs)

**Interfaces:**
- Consumes: `avoid::toll_options` (Task 1), `FetchedRoute` (Task 2), `RouteProvider` trait (unchanged).
- Produces:
  - `pub const PUBLIC_SYGIC_URL: &str = "https://routing.api.sygic.com";`
  - `SygicRouteProvider::new(base_url: impl Into<String>, api_key: String, referer: Option<String>, avoid: Vec<String>) -> Self`. `avoid` must already be normalised (Task 1).

Facts from the tests of 2026-09-29 (see [01-task.md](./01-task.md)):
- `GET /v3/api/directions?origin=lat,lon&destination=lat,lon&waypoints=lat,lon|lat,lon&vehicle_type=car&avoid=cze:tolls&return_possible_avoids=true&compute_alternatives=true&key=...`
- Success body: `{"status":"OK","routes":[{"route":"<polyline5>","distance":{"value":130100},"duration":{"value":5280},"possible_avoids":["svk:tolls",...]}]}`
- A referer mismatch: HTTP 403 with the plain-text body `Allowed referers do not match.`

- [ ] **Step 1: Write the failing tests** (`sygic_tests.rs`)

```rust
//! Tests for the Sygic routing provider. Every test runs against `wiremock`;
//! nothing here touches the real Sygic API.

use super::osrm::RouteProvider;
use super::sygic::SygicRouteProvider;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const BA: (f64, f64) = (48.1486, 17.1077);
const BRNO: (f64, f64) = (49.1951, 16.6068);

fn ok_body(routes: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "status": "OK", "routes": routes })
}

fn one_route() -> serde_json::Value {
    ok_body(serde_json::json!([{
        "route": "_p~iF~ps|U_ulLnnqC",
        "distance": { "value": 132900 },
        "duration": { "value": 6660 },
        "possible_avoids": ["svk:highways", "svk:tolls", "svk:country", "cze:highways", "cze:country"]
    }]))
}

fn provider(server: &MockServer, avoid: &[&str]) -> SygicRouteProvider {
    SygicRouteProvider::new(
        server.uri(),
        "test-key".into(),
        None,
        avoid.iter().map(|s| s.to_string()).collect(),
    )
}

fn only_request(requests: Vec<Request>) -> Request {
    assert_eq!(requests.len(), 1, "expected exactly one request");
    requests.into_iter().next().unwrap()
}

fn query(req: &Request, key: &str) -> Option<String> {
    req.url.query_pairs().find(|(k, _)| k == key).map(|(_, v)| v.into_owned())
}

#[tokio::test]
async fn maps_a_successful_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/v3/api/directions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(one_route()))
        .mount(&server).await;

    let r = provider(&server, &["cze:tolls"]).fetch(&[BA, BRNO]).await.unwrap();
    assert_eq!(r.polyline, "_p~iF~ps|U_ulLnnqC");
    assert!((r.road_km - 132.9).abs() < 1e-9);
    assert!((r.duration_s - 6660.0).abs() < 1e-9);
    // `*:tolls` only, and the requested `cze:tolls` stays although Sygic no
    // longer offers it (Review Focus 3).
    assert_eq!(r.possible_avoids, vec!["cze:tolls".to_string(), "svk:tolls".to_string()]);
}

/// Sygic takes `lat,lon`. OSRM takes `lon,lat`. A swap puts the route in the
/// wrong country with no error, so pin the exact strings.
#[tokio::test]
async fn sends_lat_lon_with_vias_between_origin_and_destination() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200).set_body_json(one_route()))
        .mount(&server).await;

    let via = (48.7, 17.0);
    provider(&server, &[]).fetch(&[BA, via, BRNO]).await.unwrap();

    let req = only_request(server.received_requests().await.unwrap());
    assert_eq!(query(&req, "origin").as_deref(), Some("48.148600,17.107700"));
    assert_eq!(query(&req, "destination").as_deref(), Some("49.195100,16.606800"));
    assert_eq!(query(&req, "waypoints").as_deref(), Some("48.700000,17.000000"));
    assert_eq!(query(&req, "vehicle_type").as_deref(), Some("car"));
    assert_eq!(query(&req, "return_possible_avoids").as_deref(), Some("true"));
    assert_eq!(query(&req, "key").as_deref(), Some("test-key"));
    assert_eq!(query(&req, "avoid"), None, "no avoid list, no avoid parameter");
    assert_eq!(query(&req, "compute_alternatives"), None, "vias: no alternatives");
}

#[tokio::test]
async fn joins_the_avoid_list_with_a_pipe() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(query_param("avoid", "cze:tolls|svk:tolls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(one_route()))
        .expect(1)
        .mount(&server).await;

    provider(&server, &["cze:tolls", "svk:tolls"]).fetch(&[BA, BRNO]).await.unwrap();
}

#[tokio::test]
async fn sends_the_referer_when_configured() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(header("referer", "https://example.test/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(one_route()))
        .expect(1)
        .mount(&server).await;

    SygicRouteProvider::new(server.uri(), "k".into(), Some("https://example.test/".into()), vec![])
        .fetch(&[BA, BRNO]).await.unwrap();
}

#[tokio::test]
async fn asks_for_alternatives_on_a_two_point_route_and_keeps_their_order() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(query_param("compute_alternatives", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body(serde_json::json!([
            { "route": "a", "distance": { "value": 132900 }, "duration": { "value": 6660 }, "possible_avoids": [] },
            { "route": "b", "distance": { "value": 137700 }, "duration": { "value": 7080 }, "possible_avoids": [] },
            { "route": "c", "distance": { "value": 150000 }, "duration": { "value": 7380 }, "possible_avoids": [] },
            { "route": "d", "distance": { "value": 160000 }, "duration": { "value": 9000 }, "possible_avoids": [] }
        ]))))
        .expect(1)
        .mount(&server).await;

    let routes = provider(&server, &[]).fetch_alternatives(&[BA, BRNO], 3).await.unwrap();
    let names: Vec<&str> = routes.iter().map(|r| r.polyline.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "c"], "service order, cut to max, never re-sorted");
}

#[tokio::test]
async fn names_the_status_and_body_of_a_403() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(403).set_body_string("Allowed referers do not match."))
        .mount(&server).await;

    let err = provider(&server, &[]).fetch(&[BA, BRNO]).await.expect_err("403 is not a route");
    assert!(err.contains("Sygic") && err.contains("403") && err.contains("Allowed referers"), "got: {err}");
}

#[tokio::test]
async fn names_http_429_so_the_page_can_offer_retry() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(429)).mount(&server).await;

    let err = provider(&server, &[]).fetch(&[BA, BRNO]).await.expect_err("429 is not a route");
    assert!(err.contains("429"), "got: {err}");
}

#[tokio::test]
async fn a_non_ok_status_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "status": "NO_ROUTE", "routes": [] })))
        .mount(&server).await;

    let err = provider(&server, &[]).fetch(&[BA, BRNO]).await.expect_err("no route");
    assert!(err.contains("NO_ROUTE"), "got: {err}");
}

#[tokio::test]
async fn an_ok_status_with_no_routes_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200).set_body_json(ok_body(serde_json::json!([]))))
        .mount(&server).await;

    assert!(provider(&server, &[]).fetch(&[BA, BRNO]).await.is_err());
}

/// Review Focus 6: the key is in the query string, and reqwest errors print
/// the URL. No error may carry it.
#[tokio::test]
async fn a_transport_failure_does_not_leak_the_key() {
    // Port 9 (discard) on localhost: nothing listens, the connect fails.
    let p = SygicRouteProvider::new("http://127.0.0.1:9", "secret-test-key".into(), None, vec![]);
    let err = p.fetch(&[BA, BRNO]).await.expect_err("nothing listens");
    assert!(!err.contains("secret-test-key"), "key leaked: {err}");
}

#[tokio::test]
async fn a_malformed_body_does_not_leak_the_key() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server).await;

    let p = SygicRouteProvider::new(server.uri(), "secret-test-key".into(), None, vec![]);
    let err = p.fetch(&[BA, BRNO]).await.expect_err("bad body");
    assert!(!err.contains("secret-test-key"), "key leaked: {err}");
}

#[tokio::test]
async fn one_point_is_an_error_not_a_request() {
    let server = MockServer::start().await;
    let err = provider(&server, &[]).fetch(&[BA]).await.expect_err("one point");
    assert!(err.contains("at least 2 points"), "got: {err}");
    assert!(server.received_requests().await.unwrap().is_empty());
}
```

Register in `mod.rs`:

```rust
pub mod sygic;
```

```rust
#[cfg(test)]
#[path = "sygic_tests.rs"]
mod sygic_tests;
```

- [ ] **Step 2: Run the tests and see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core sygic_tests`
Expected: FAIL, module `sygic` not found.

- [ ] **Step 3: Write the implementation** (`sygic.rs`)

```rust
//! Sygic route geometry provider (Task 85).
//!
//! Used in place of OSRM when `SYGIC_API_KEY` is set, because Sygic can avoid
//! the paid roads of ONE country (`avoid=cze:tolls`). The public OSRM servers
//! reject every `exclude` value and know no countries -- see
//! _tasks/85-route-avoid-tolls-per-country/01-task.md.
//!
//! The avoid list is fixed when the provider is built, so the `RouteProvider`
//! trait and its callers do not change.

use serde::Deserialize;
use std::time::Duration;

use super::avoid::toll_options;
use super::osrm::{FetchedRoute, RouteProvider};

pub const PUBLIC_SYGIC_URL: &str = "https://routing.api.sygic.com";

const USER_AGENT: &str = concat!(
    "kniha-jazd/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/mcsdodo/kniha-jazd)"
);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Enough of an error body to name the cause ("Allowed referers do not match.").
const MAX_ERROR_BODY: usize = 200;

pub struct SygicRouteProvider {
    base_url: String,
    api_key: String,
    referer: Option<String>,
    /// Already normalised by `avoid::normalise_avoid`.
    avoid: Vec<String>,
    client: Result<reqwest::Client, String>,
}

impl SygicRouteProvider {
    pub fn new(
        base_url: impl Into<String>,
        api_key: String,
        referer: Option<String>,
        avoid: Vec<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| format!("Could not create an HTTP client for the routing service: {e}"));
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            referer,
            avoid,
            client,
        }
    }

    /// Query parameters for `coords` (`(lat, lon)` pairs in visit order).
    /// Sygic wants `lat,lon` -- the OPPOSITE of OSRM.
    fn params(&self, coords: &[(f64, f64)], alternatives: bool) -> Vec<(&'static str, String)> {
        let point = |(lat, lon): (f64, f64)| format!("{lat:.6},{lon:.6}");
        let mut p = vec![
            ("origin", point(coords[0])),
            ("destination", point(coords[coords.len() - 1])),
            ("vehicle_type", "car".to_string()),
            ("return_possible_avoids", "true".to_string()),
            ("key", self.api_key.clone()),
        ];
        if coords.len() > 2 {
            let vias = coords[1..coords.len() - 1].iter().map(|&c| point(c)).collect::<Vec<_>>().join("|");
            p.push(("waypoints", vias));
        }
        if !self.avoid.is_empty() {
            p.push(("avoid", self.avoid.join("|")));
        }
        // Same rule as OSRM (DECISIONS.md, alternatives ADR): only a plain
        // two-point request offers a choice.
        if alternatives && coords.len() == 2 {
            p.push(("compute_alternatives", "true".to_string()));
        }
        p
    }

    async fn request(&self, coords: &[(f64, f64)], alternatives: bool) -> Result<Vec<FetchedRoute>, String> {
        if coords.len() < 2 {
            return Err(format!(
                "Route needs at least 2 points, got {}. Nothing was requested from Sygic.",
                coords.len()
            ));
        }
        let client = self.client.as_ref().map_err(|e| e.clone())?;
        let mut req = client
            .get(format!("{}/v3/api/directions", self.base_url))
            .query(&self.params(coords, alternatives));
        if let Some(referer) = &self.referer {
            req = req.header(reqwest::header::REFERER, referer);
        }

        // `without_url`: reqwest prints the URL in its errors, and the URL
        // carries the key (Review Focus 6).
        let response = req.send().await.map_err(|e| {
            format!(
                "Could not reach the Sygic routing service: {}. Check your internet connection and try again.",
                e.without_url()
            )
        })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let body: String = body.trim().chars().take(MAX_ERROR_BODY).collect();
            return Err(format!(
                "Sygic routing service returned HTTP {} ({}){}. Try again in a moment.",
                status.as_u16(),
                status.canonical_reason().unwrap_or("unknown"),
                if body.is_empty() { String::new() } else { format!(": {body}") }
            ));
        }

        // `without_url`: a decode error prints the URL, and the URL carries the key.
        let body: SygicResponse = response
            .json()
            .await
            .map_err(|e| format!("Could not read the Sygic routing service response: {}", e.without_url()))?;
        if body.status != "OK" {
            return Err(format!("Sygic routing service could not build a route: {}", body.status));
        }
        if body.routes.is_empty() {
            return Err("Sygic routing service reported success but returned no route.".to_string());
        }

        Ok(body
            .routes
            .into_iter()
            .map(|r| FetchedRoute {
                possible_avoids: toll_options(r.possible_avoids.iter().map(String::as_str), &self.avoid),
                polyline: r.route,
                road_km: r.distance.value / 1000.0,
                duration_s: r.duration.value,
            })
            .collect())
    }
}

#[derive(Deserialize)]
struct SygicResponse {
    status: String,
    #[serde(default)]
    routes: Vec<SygicRoute>,
}

#[derive(Deserialize)]
struct SygicRoute {
    /// Google encoded polyline, precision 1e5 -- the same polyline5 we store.
    route: String,
    distance: SygicValue,
    duration: SygicValue,
    #[serde(default)]
    possible_avoids: Vec<String>,
}

#[derive(Deserialize)]
struct SygicValue {
    value: f64,
}

#[async_trait::async_trait]
impl RouteProvider for SygicRouteProvider {
    async fn fetch(&self, coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        let mut routes = self.request(coords, false).await?;
        Ok(routes.remove(0))
    }

    async fn fetch_alternatives(&self, coords: &[(f64, f64)], max: usize) -> Result<Vec<FetchedRoute>, String> {
        let mut routes = self.request(coords, max > 1).await?;
        routes.truncate(max.max(1));
        Ok(routes)
    }
}
```

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core sygic_tests`
Expected: PASS, 12 tests.

- [ ] **Step 5: Manual check against the real API (optional, needs the key)**

Only if the key is in the repo `.env`. Do not print the key and do not `source` the file.
Write a throwaway `#[ignore]` test or a scratch binary that reads `SYGIC_API_KEY` and
`SYGIC_REFERER` from the environment and routes BA to Brno with `["cze:tolls"]`.
Expected: about 132.9 km, `possible_avoids` has `svk:tolls` and `cze:tolls`. Do not commit it.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/core/src/route_map/sygic.rs src-tauri/core/src/route_map/sygic_tests.rs src-tauri/core/src/route_map/mod.rs
git commit -m "feat(route-map): add Sygic routing provider with per-country toll avoid"
```

---

### Task 4: Provider factory and test mock (`provider.rs`)

**Files:**
- Create: [src-tauri/core/src/route_map/provider.rs](../../../src-tauri/core/src/route_map/provider.rs)
- Create: [src-tauri/core/src/route_map/provider_tests.rs](../../../src-tauri/core/src/route_map/provider_tests.rs)
- Modify: [src-tauri/core/src/route_map/mod.rs](../../../src-tauri/core/src/route_map/mod.rs)
- Modify: [src-tauri/core/src/constants.rs](../../../src-tauri/core/src/constants.rs) (`env_vars`)

**Interfaces:**
- Consumes: `SygicRouteProvider`, `PUBLIC_SYGIC_URL` (Task 3), `HttpRouteProvider` (existing), `avoid::toll_options` (Task 1), `polyline::encode` (existing).
- Produces:
  - `constants::env_vars::{SYGIC_API_KEY, SYGIC_REFERER, MOCK_ROUTER}`.
  - `pub enum ProviderConfig { Mock, Sygic { api_key: String, referer: Option<String> }, Osrm }`
  - `impl ProviderConfig { pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self; pub fn from_env() -> Self }`
  - `pub fn build_provider(config: ProviderConfig, avoid: Vec<String>) -> Result<Box<dyn RouteProvider>, String>`
  - `pub fn route_provider(avoid: Vec<String>) -> Result<Box<dyn RouteProvider>, String>` (= `build_provider(ProviderConfig::from_env(), avoid)`)
  - `pub struct MockRouteProvider { avoid: Vec<String> }`, re-exported from `mod.rs` with `route_provider`.

Mock behaviour, used only by the integration suite: a straight line through the points, `road_km` = `100.0` with no avoid and `120.0` with any avoid, `duration_s` = `3600.0`, `possible_avoids` = `toll_options(["cze:tolls"], avoid)`. The fixed numbers let a spec see that a checkbox click routed again.

- [ ] **Step 1: Add the env var names** (`constants.rs`, inside `pub mod env_vars`)

```rust
    /// Sygic Routing API key. Set: Sygic computes every route map, and the
    /// page offers per-country "avoid paid roads". Unset: public OSRM.
    pub const SYGIC_API_KEY: &str = "SYGIC_API_KEY";

    /// Optional `Referer` header for a Sygic key with a referer restriction.
    pub const SYGIC_REFERER: &str = "SYGIC_REFERER";

    /// Set to any non-empty value to route with a fixed, offline mock
    /// (integration tests). Wins over `SYGIC_API_KEY`.
    pub const MOCK_ROUTER: &str = "KNIHA_JAZD_MOCK_ROUTER";
```

- [ ] **Step 2: Write the failing tests** (`provider_tests.rs`)

```rust
//! Tests for the routing provider choice. `from_lookup` takes a closure, so
//! nothing here reads or writes the real process environment.

use super::provider::{build_provider, ProviderConfig};
use crate::constants::env_vars::{MOCK_ROUTER, SYGIC_API_KEY, SYGIC_REFERER};

fn lookup(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
    move |k| pairs.iter().find(|(name, _)| *name == k).map(|(_, v)| v.to_string())
}

#[test]
fn no_key_selects_osrm() {
    assert!(matches!(ProviderConfig::from_lookup(lookup(&[])), ProviderConfig::Osrm));
}

#[test]
fn a_blank_key_counts_as_unset() {
    assert!(matches!(ProviderConfig::from_lookup(lookup(&[(SYGIC_API_KEY, "  ")])), ProviderConfig::Osrm));
}

#[test]
fn a_key_selects_sygic_with_the_optional_referer() {
    match ProviderConfig::from_lookup(lookup(&[(SYGIC_API_KEY, "k"), (SYGIC_REFERER, "https://r.test/")])) {
        ProviderConfig::Sygic { api_key, referer } => {
            assert_eq!(api_key, "k");
            assert_eq!(referer.as_deref(), Some("https://r.test/"));
        }
        _ => panic!("expected Sygic"),
    }
}

#[test]
fn the_mock_wins_over_a_real_key() {
    // A developer with SYGIC_API_KEY in the shell must never spend real
    // requests, or get real routes, from the integration suite.
    let cfg = ProviderConfig::from_lookup(lookup(&[(MOCK_ROUTER, "1"), (SYGIC_API_KEY, "k")]));
    assert!(matches!(cfg, ProviderConfig::Mock));
}

#[test]
fn osrm_refuses_a_non_empty_avoid_list() {
    // Review Focus 1: a saved route with an avoid list, recomputed after the
    // key is gone, must not come back from OSRM as if the avoid had worked.
    let err = build_provider(ProviderConfig::Osrm, vec!["cze:tolls".into()])
        .err()
        .expect("OSRM cannot avoid per country");
    assert!(err.contains("SYGIC_API_KEY"), "got: {err}");
}

#[test]
fn osrm_accepts_an_empty_avoid_list() {
    assert!(build_provider(ProviderConfig::Osrm, vec![]).is_ok());
}

#[tokio::test]
async fn the_mock_is_deterministic_and_reflects_the_avoid_list() {
    let plain = build_provider(ProviderConfig::Mock, vec![]).unwrap();
    let r = plain.fetch(&[(48.1486, 17.1077), (49.1951, 16.6068)]).await.unwrap();
    assert_eq!(r.road_km, 100.0);
    assert_eq!(r.possible_avoids, vec!["cze:tolls".to_string()]);

    let avoiding = build_provider(ProviderConfig::Mock, vec!["cze:tolls".into()]).unwrap();
    let r = avoiding.fetch(&[(48.1486, 17.1077), (49.1951, 16.6068)]).await.unwrap();
    assert_eq!(r.road_km, 120.0);
    assert_eq!(r.possible_avoids, vec!["cze:tolls".to_string()]);
}
```

Register in `mod.rs`:

```rust
pub mod provider;
pub use provider::route_provider;
```

```rust
#[cfg(test)]
#[path = "provider_tests.rs"]
mod provider_tests;
```

- [ ] **Step 3: Run the tests and see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core provider_tests`
Expected: FAIL, module `provider` not found.

- [ ] **Step 4: Write the implementation** (`provider.rs`)

```rust
//! Which routing service computes a route map (Task 85).
//!
//! One place decides, so the three async commands cannot disagree. The avoid
//! list is fixed per provider instance, because the dispatcher builds a new
//! provider for every request anyway.

use crate::constants::env_vars::{MOCK_ROUTER, SYGIC_API_KEY, SYGIC_REFERER};

use super::avoid::toll_options;
use super::osrm::{FetchedRoute, HttpRouteProvider, RouteProvider};
use super::polyline::encode;
use super::sygic::{SygicRouteProvider, PUBLIC_SYGIC_URL};

#[derive(Debug)]
pub enum ProviderConfig {
    Mock,
    Sygic { api_key: String, referer: Option<String> },
    Osrm,
}

impl ProviderConfig {
    /// Pure: `lookup` stands in for `std::env::var`. Blank values count as unset.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let get = |k: &str| lookup(k).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        if get(MOCK_ROUTER).is_some() {
            return Self::Mock;
        }
        match get(SYGIC_API_KEY) {
            Some(api_key) => Self::Sygic { api_key, referer: get(SYGIC_REFERER) },
            None => Self::Osrm,
        }
    }

    pub fn from_env() -> Self {
        Self::from_lookup(|k| std::env::var(k).ok())
    }
}

/// `avoid` must already be normalised (`avoid::normalise_avoid`).
pub fn build_provider(config: ProviderConfig, avoid: Vec<String>) -> Result<Box<dyn RouteProvider>, String> {
    match config {
        ProviderConfig::Mock => Ok(Box::new(MockRouteProvider { avoid })),
        ProviderConfig::Sygic { api_key, referer } => {
            Ok(Box::new(SygicRouteProvider::new(PUBLIC_SYGIC_URL, api_key, referer, avoid)))
        }
        ProviderConfig::Osrm if avoid.is_empty() => Ok(Box::new(HttpRouteProvider::public())),
        // No silent fallback (01-task.md, decision 5): OSRM would return a
        // route that ignores the avoid list and looks correct.
        ProviderConfig::Osrm => Err(format!(
            "Avoiding paid roads ({}) needs the Sygic routing service. Set SYGIC_API_KEY on the server, or uncheck the avoid options.",
            avoid.join(", ")
        )),
    }
}

pub fn route_provider(avoid: Vec<String>) -> Result<Box<dyn RouteProvider>, String> {
    build_provider(ProviderConfig::from_env(), avoid)
}

/// Offline stand-in for the integration suite (`KNIHA_JAZD_MOCK_ROUTER`).
/// Fixed numbers, so a spec can see that a checkbox click routed again.
pub struct MockRouteProvider {
    avoid: Vec<String>,
}

#[async_trait::async_trait]
impl RouteProvider for MockRouteProvider {
    async fn fetch(&self, coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        if coords.len() < 2 {
            return Err(format!("Route needs at least 2 points, got {}.", coords.len()));
        }
        Ok(FetchedRoute {
            polyline: encode(coords),
            road_km: if self.avoid.is_empty() { 100.0 } else { 120.0 },
            duration_s: 3600.0,
            possible_avoids: toll_options(["cze:tolls"], &self.avoid),
        })
    }
}
```

Check `polyline::encode` takes `&[(f64, f64)]`. `route_maps.rs` calls `encode(outbound)` on a `&[(f64, f64)]` slice, so it does.

- [ ] **Step 5: Run the tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core provider_tests`
Expected: PASS, 7 tests.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/core/src/route_map/provider.rs src-tauri/core/src/route_map/provider_tests.rs src-tauri/core/src/route_map/mod.rs src-tauri/core/src/constants.rs
git commit -m "feat(route-map): choose Sygic, OSRM or a test mock from the environment"
```

---

### Task 5: `avoidOptions` in the route commands, `avoid` on the RPC

**Files:**
- Modify: [src-tauri/core/src/commands_internal/route_maps.rs](../../../src-tauri/core/src/commands_internal/route_maps.rs) (`GeneratedRoute`, `RoundTripRoutes`, the 3 `*_internal` builders)
- Modify: [src-tauri/core/src/commands_internal/route_maps_tests.rs](../../../src-tauri/core/src/commands_internal/route_maps_tests.rs)
- Modify: [src-tauri/core/src/server/dispatcher_async.rs](../../../src-tauri/core/src/server/dispatcher_async.rs) (arms `generate_route`, `route_direct`, `route_round_trip`, and the existing arg tests near line 395)

**Interfaces:**
- Consumes: `FetchedRoute.possible_avoids` (Task 2), `normalise_avoid`, `merge_options` (Task 1), `route_provider` (Task 4).
- Produces:
  - `GeneratedRoute.avoid_options: Vec<String>` (JSON `avoidOptions`) = that route's `possible_avoids`.
  - `RoundTripRoutes.avoid_options: Vec<String>` = `merge_options` over every alternative of both legs.
  - RPC args `avoid: string[]` (optional, default `[]`) on `generate_route`, `route_direct`, `route_round_trip`.

- [ ] **Step 1: Write the failing tests** (append to `route_maps_tests.rs`)

```rust
fn fetched_with_avoids(polyline: &str, road_km: f64, avoids: &[&str]) -> FetchedRoute {
    FetchedRoute {
        polyline: polyline.into(),
        road_km,
        duration_s: 1.0,
        possible_avoids: avoids.iter().map(|s| s.to_string()).collect(),
    }
}

#[tokio::test]
async fn direct_routes_carry_each_routes_own_avoid_options() {
    let provider = MultiRouteProvider {
        routes: vec![
            fetched_with_avoids(&encode(&[(48.1, 17.1), (49.2, 16.6)]), 130.0, &["cze:tolls", "svk:tolls"]),
            fetched_with_avoids(&encode(&[(48.1, 17.1), (49.2, 16.6)]), 160.0, &["aut:tolls"]),
        ],
    };
    let routes = route_direct_internal(&provider, direct_waypoints(), 130.0, None, false).await.unwrap();
    assert_eq!(routes[0].avoid_options, vec!["cze:tolls".to_string(), "svk:tolls".to_string()]);
    assert_eq!(routes[1].avoid_options, vec!["aut:tolls".to_string()]);
}

#[tokio::test]
async fn a_round_trip_offers_the_union_of_both_legs() {
    let provider = MultiRouteProvider {
        routes: vec![
            fetched_with_avoids(&encode(&[(48.1, 17.1), (49.2, 16.6)]), 130.0, &["svk:tolls"]),
            fetched_with_avoids(&encode(&[(48.1, 17.1), (49.2, 16.6)]), 150.0, &["cze:tolls", "svk:tolls"]),
        ],
    };
    let rt = route_round_trip_internal(&provider, direct_waypoints(), vec![], 260.0, None).await.unwrap();
    assert_eq!(rt.avoid_options, vec!["cze:tolls".to_string(), "svk:tolls".to_string()]);
}
```

In `dispatcher_async.rs`, next to `route_direct_is_an_async_command_taking_waypoints`, add a test that a bad avoid value fails before any network call. Copy the setup of that existing test, and send:

```rust
serde_json::json!({
    "waypoints": [{ "lat": 48.1486, "lon": 17.1077 }, { "lat": 49.1951, "lon": 16.6068 }],
    "targetKm": 130.0,
    "avoid": ["cze:tolls|svk:tolls"]
})
```

and assert that the result is `Some(Err(e))` with `e` containing `"Unsupported avoid value"`.

- [ ] **Step 2: Run the tests and see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core avoid_options`
Expected: FAIL to compile, `no field avoid_options`.

- [ ] **Step 3: Implement in `route_maps.rs`**

Add to `GeneratedRoute`, after `mode`:

```rust
    /// `<iso3>:tolls` values the page offers as "avoid paid roads" checkboxes.
    /// Empty with OSRM. Includes the values this request already avoided.
    pub avoid_options: Vec<String>,
```

Add to `RoundTripRoutes`, after `target_km`:

```rust
    /// Union over every alternative of both legs. One list for the whole round
    /// trip, because both legs are routed with the same avoid list.
    pub avoid_options: Vec<String>,
```

Set them:
- `generate_route_internal`: `avoid_options: fetched.possible_avoids.clone(),` (before `polyline: fetched.polyline` moves it, or reorder the fields).
- `route_direct_internal`, inside the `.map(|route| ...)`: `avoid_options: route.possible_avoids.clone(),`.
- `route_round_trip_internal`, before `Ok(RoundTripRoutes {`:

```rust
    let avoid_options = merge_options(
        out_routes.iter().chain(in_routes.iter()).map(|r| r.possible_avoids.as_slice()),
    );
```

and `avoid_options,` in the struct. Import `use crate::route_map::avoid::merge_options;`.

- [ ] **Step 4: Implement in `dispatcher_async.rs`**

In each of the three `Args` structs add:

```rust
                // Task 85. Defaulted so a caller that predates it still routes.
                #[serde(default)]
                avoid: Vec<String>,
```

and replace `let provider = crate::route_map::HttpRouteProvider::public();` in each arm with:

```rust
            let provider = match crate::route_map::avoid::normalise_avoid(a.avoid)
                .and_then(crate::route_map::route_provider)
            {
                Ok(p) => p,
                Err(e) => return Some(Err(e)),
            };
```

Pass `provider.as_ref()` where `&provider` was passed. In `generate_route`, `a` has only `target_km` today. Keep the `a.target_km` use as it is.

- [ ] **Step 5: Run the tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core route_map`
then `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core dispatcher`
Expected: PASS. The existing dispatcher arg tests still pass: they send no `avoid`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/core/src/commands_internal/route_maps.rs src-tauri/core/src/commands_internal/route_maps_tests.rs src-tauri/core/src/server/dispatcher_async.rs
git commit -m "feat(route-map): take an avoid list and return avoid options on every route"
```

---

### Task 6: Store the avoid list (`trip_routes.avoid`)

**Files:**
- Create: [src-tauri/core/migrations/2026-09-29-100000_add_trip_route_avoid/up.sql](../../../src-tauri/core/migrations/2026-09-29-100000_add_trip_route_avoid/up.sql), `down.sql`
- Modify: [src-tauri/core/src/schema.rs](../../../src-tauri/core/src/schema.rs) (`trip_routes`, after `turnaround_index`)
- Modify: [src-tauri/core/src/models.rs](../../../src-tauri/core/src/models.rs) (`RouteMap`, `RouteMapRow`, `NewRouteMapRow`, `From<RouteMapRow> for RouteMap`)
- Modify: [src-tauri/core/src/db.rs](../../../src-tauri/core/src/db.rs) (`save_route_map`)
- Modify: [src-tauri/core/src/commands_internal/route_maps.rs](../../../src-tauri/core/src/commands_internal/route_maps.rs) (`persist_route_map`, both save fns, `SavedRouteMap`)
- Modify: [src-tauri/core/src/server/dispatcher.rs](../../../src-tauri/core/src/server/dispatcher.rs) (`save_trip_route`, `save_trip_round_trip_route` args)
- Modify: [src-tauri/core/src/db_tests.rs](../../../src-tauri/core/src/db_tests.rs) (4 `RouteMap {` literals), [src-tauri/core/src/migration_tests.rs](../../../src-tauri/core/src/migration_tests.rs), `route_maps_tests.rs`

**Interfaces:**
- Consumes: `normalise_avoid` (Task 1).
- Produces:
  - `RouteMap.avoid: Vec<String>`, `SavedRouteMap.avoid: Vec<String>` (JSON `avoid`).
  - `save_trip_route_internal(..., round_trip: bool, avoid: Vec<String>)`, `save_trip_round_trip_route_internal(..., target_km: f64, avoid: Vec<String>)`.
  - RPC args `avoid: string[]` (optional, default `[]`) on `save_trip_route`, `save_trip_round_trip_route`.

- [ ] **Step 1: Write the migration**

`up.sql`:

```sql
-- Task 85: the "avoid paid roads" countries a route was computed with, as a
-- JSON array of `<iso3>:tolls` strings. '[]' is the correct backfill: every
-- route saved before today was computed with no avoid list.
ALTER TABLE trip_routes ADD COLUMN avoid TEXT NOT NULL DEFAULT '[]';
```

`down.sql`:

```sql
-- Forward-only in practice (ADR-012); no diesel CLI revert runs in this repo.
ALTER TABLE trip_routes DROP COLUMN avoid;
```

- [ ] **Step 2: Write the failing tests**

`migration_tests.rs`, after the Task 20 test, same helpers:

```rust
// ============================================================================
// Task 85 -- per-country avoid list (2026-09-29-100000)
// ============================================================================

#[test]
fn existing_route_maps_backfill_an_empty_avoid_list() {
    let db = open_db_legacy_before("2026-09-29-100000");
    seed_vehicle(&db, "v1");
    seed_trip(&db, "t1", "v1", None);
    exec(
        &db,
        "INSERT INTO trip_routes (trip_id, waypoints, polyline, target_km, road_km, \
                                  dataset_version, created_at) \
         VALUES ('t1', '[]', 'abc', 100.0, 98.0, '2026-05-03', \
                 '2026-01-01T00:00:00+00:00')",
    );

    migrate_to_current(&db);

    let map = db.get_route_map("t1").unwrap().unwrap();
    assert!(map.avoid.is_empty(), "a route saved before the avoid column must read as no avoid");
}
```

`route_maps_tests.rs` (use the existing `seed_trip`, `sample_waypoints` and `AppState::new()` pattern):

```rust
#[test]
fn a_saved_avoid_list_comes_back_normalised() {
    let db = Database::in_memory().unwrap();
    let trip = seed_trip(&db);
    let app_state = AppState::new();
    save_trip_route_internal(
        &db, &app_state, trip.id.to_string(), sample_waypoints(),
        encode(&[(48.1, 17.1), (49.2, 16.6)]), 130.0, 132.9, RouteMode::Direct, false,
        vec!["svk:tolls".into(), "CZE:tolls".into()],
    ).unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();
    assert_eq!(saved.avoid, vec!["cze:tolls".to_string(), "svk:tolls".to_string()]);
}

#[test]
fn saving_a_bad_avoid_value_is_refused() {
    let db = Database::in_memory().unwrap();
    let trip = seed_trip(&db);
    let err = save_trip_route_internal(
        &db, &AppState::new(), trip.id.to_string(), sample_waypoints(),
        encode(&[(48.1, 17.1), (49.2, 16.6)]), 130.0, 132.9, RouteMode::Direct, false,
        vec!["cze:highways".into()],
    ).expect_err("only tolls values are stored");
    assert!(err.contains("avoid"), "got: {err}");
}
```

Add the same kind of check for `save_trip_round_trip_route_internal` with `vec!["cze:tolls".into()]` and assert `saved.avoid == vec!["cze:tolls"]`.

- [ ] **Step 3: Run the tests and see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core avoid`
Expected: FAIL to compile (`no field avoid`, wrong argument count).

- [ ] **Step 4: Implement the column end to end**

`schema.rs`, as the LAST line of `trip_routes`:

```rust
        // Added via migration 2026-09-29-100000_add_trip_route_avoid (Task 85).
        // Appended LAST for the fourth time: RouteMapRow binds POSITIONALLY.
        // It is Text like `mode` and `created_at`, so a wrong position here
        // would compile and swap them silently.
        avoid -> Text,
```

`models.rs`:
- `RouteMap`: add `pub avoid: Vec<String>,` with a doc comment ("`<iso3>:tolls` values the route was computed with. Empty for OSRM and for routes saved before Task 85.").
- `RouteMapRow`: add `pub avoid: String,` as the LAST field, with the same positional-bind comment as `turnaround_index`.
- `NewRouteMapRow`: add `pub avoid: &'a str,`.
- `From<RouteMapRow> for RouteMap`: `avoid: serde_json::from_str(&row.avoid).unwrap_or_default(),`.

`db.rs`, `save_route_map`: serialise like `waypoints_json`:

```rust
        let avoid_json = serde_json::to_string(&map.avoid)
            .map_err(|e| diesel::result::Error::SerializationError(Box::new(e)))?;
```

and `avoid: &avoid_json,` in `NewRouteMapRow`.

`route_maps.rs`:
- `persist_route_map`: add the last parameter `avoid: Vec<String>`. At the top, after `check_read_only!`: `let avoid = normalise_avoid(avoid)?;`. Put `avoid` into the `RouteMap`.
- `save_trip_route_internal` and `save_trip_round_trip_route_internal`: add `avoid: Vec<String>` as the last parameter and pass it on.
- `SavedRouteMap`: add `pub avoid: Vec<String>,` (doc: "Restores the checked boxes on reopen. Also the only options a saved route offers until it is routed again.") and `avoid: map.avoid,` in `From<RouteMap>`.

`dispatcher.rs`: in both save arms add

```rust
                // Task 85. Defaulted: a payload that predates it avoided nothing.
                #[serde(default)]
                avoid: Vec<String>,
```

and pass `a.avoid` as the last argument.

`db_tests.rs`: add `avoid: vec![],` to the 4 `RouteMap {` literals. Fix every other caller that the compiler names (`route_maps_tests.rs` calls to the two save fns get `vec![]`).

- [ ] **Step 5: Run the backend suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
Expected: PASS, 0 failures.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/core/migrations/2026-09-29-100000_add_trip_route_avoid src-tauri/core/src/schema.rs src-tauri/core/src/models.rs src-tauri/core/src/db.rs src-tauri/core/src/db_tests.rs src-tauri/core/src/migration_tests.rs src-tauri/core/src/commands_internal/route_maps.rs src-tauri/core/src/commands_internal/route_maps_tests.rs src-tauri/core/src/server/dispatcher.rs
git commit -m "feat(route-map): save the avoid list with the route map"
```

---

### Task 7: Checkboxes on `/mapa`

**Files:**
- Modify: [src/lib/types.ts](../../../src/lib/types.ts) (`GeneratedRoute`, `RoundTripRoutes`, `RouteMap`)
- Modify: [src/lib/api.ts](../../../src/lib/api.ts) (`generateRoute`, `routeDirect`, `routeRoundTrip`, `saveTripRoute`, `saveTripRoundTripRoute`)
- Modify: [src/routes/mapa/+page.svelte](../../../src/routes/mapa/+page.svelte)
- Modify: [src/lib/i18n/sk/index.ts](../../../src/lib/i18n/sk/index.ts), [src/lib/i18n/en/index.ts](../../../src/lib/i18n/en/index.ts) (`routeMap` block)

**Interfaces:**
- Consumes: JSON fields `avoidOptions` (Task 5) and `avoid` (Task 6), RPC arg `avoid`.
- Produces: `data-test="avoid-options"` (container), `data-test="avoid-<value>"` on each checkbox, for example `data-test="avoid-cze:tolls"`. Task 8 uses these.

- [ ] **Step 1: Types**

`types.ts`: add to `GeneratedRoute` and `RoundTripRoutes`:

```ts
	/** `<iso3>:tolls` values offered as "avoid paid roads" checkboxes. Empty
	 *  with OSRM. Already includes the values the request avoided (backend). */
	avoidOptions: string[];
```

and to `RouteMap`:

```ts
	/** The `<iso3>:tolls` values this route was computed with. */
	avoid: string[];
```

- [ ] **Step 2: API**

`api.ts`: add `avoid: string[] = []` as the LAST parameter of the five functions and send it:
- `generateRoute(targetKm, avoid)` -> `{ targetKm, avoid }`
- `routeDirect(waypoints, targetKm, insert?, roundTrip?, avoid = [])` -> add `avoid`
- `routeRoundTrip(outbound, inbound, targetKm, insert?, avoid = [])` -> add `avoid`
- `saveTripRoute(tripId, route, roundTrip, avoid = [])` -> add `avoid`
- `saveTripRoundTripRoute(..., targetKm, avoid = [])` -> add `avoid`

- [ ] **Step 3: i18n** (`routeMap` block, after `roundTripHint`)

`sk`:

```ts
		avoidTolls: 'Vyhnúť sa spoplatneným cestám',
		avoidTollsHint: 'Trasa sa vyhne úsekom s povinnou diaľničnou známkou alebo mýtom v danej krajine.',
		countries: {
			svk: 'SK',
			cze: 'CZ',
			aut: 'AT',
			hun: 'HU',
			pol: 'PL',
		},
```

`en`:

```ts
		avoidTolls: 'Avoid paid roads',
		avoidTollsHint: 'The route avoids sections that need a vignette or a toll in that country.',
		countries: {
			svk: 'SK',
			cze: 'CZ',
			aut: 'AT',
			hun: 'HU',
			pol: 'PL',
		},
```

Run: `npm run i18n`

- [ ] **Step 4: Page state and logic** (`+page.svelte` script)

Next to the `roundTrip` state:

```ts
	/** The avoid values the NEXT request sends. Restored from the saved route
	 *  in `loadRoute()`, changed by the checkboxes. */
	let avoid = $state<string[]>([]);
	/** The avoid values the SHOWN route was computed with. Captured when a
	 *  request starts, adopted when it succeeds: the save must store what
	 *  produced the line on screen, not the newest checkbox state. */
	let routedAvoid = $state<string[]>([]);

	/** Options come from the backend (ADR-008). A saved route that was not
	 *  routed again offers only the values it was saved with. */
	let avoidOptions = $derived(
		roundTripRoutes?.avoidOptions ?? generated?.avoidOptions ?? savedRoute?.avoid ?? []
	);

	function countryLabel(value: string): string {
		const iso = value.split(':')[0];
		const names = $LL.routeMap.countries as unknown as Record<string, () => string>;
		return names[iso]?.() ?? iso.toUpperCase();
	}

	function toggleAvoid(value: string, checked: boolean) {
		avoid = checked ? [...avoid, value] : avoid.filter((v) => v !== value);
		handleRegenerate();
	}
```

In `loadRoute()`, where `roundTrip = savedRoute.roundTrip;` is set, also set:

```ts
			avoid = [...savedRoute.avoid];
			routedAvoid = [...savedRoute.avoid];
```

In `runDirect` and `runRoundTrip`: at the top, next to `const closeLoop = roundTrip;` (or at the same place in `runRoundTrip`), capture:

```ts
		const requestAvoid = [...avoid];
```

pass `requestAvoid` as the new last argument of `routeDirect` / `routeRoundTrip`, and after a successful response set `routedAvoid = requestAvoid;`.

`runGenerate` (loop mode) does not change: `generateRoute(targetKm)` sends the default `avoid = []`. In it, after a successful response, set `routedAvoid = [];` so a loop is never saved with an avoid list.

In `handleSave`: pass `routedAvoid` as the last argument of `saveTripRoundTripRoute(...)` and `saveTripRoute(tripId, generated!, roundTrip, routedAvoid)`.

- [ ] **Step 5: Markup** (right after the round-trip `<label>`, inside the same controls row)

```svelte
			{#if mode === 'direct' && avoidOptions.length > 0}
				<span class="avoid-options" data-test="avoid-options" title={$LL.routeMap.avoidTollsHint()}>
					{$LL.routeMap.avoidTolls()}:
					{#each avoidOptions as value (value)}
						<label class="avoid-label">
							<input
								type="checkbox"
								data-test={`avoid-${value}`}
								checked={avoid.includes(value)}
								onchange={(e) => toggleAvoid(value, e.currentTarget.checked)}
								disabled={busy || !trip || endpointsMissing}
							/>
							{countryLabel(value)}
						</label>
					{/each}
				</span>
			{/if}
```

Style `.avoid-options` and `.avoid-label` like `.round-trip-label` (same file, `<style>` block). Copy its rules.

- [ ] **Step 6: Type check and build**

Run: `npm run check`
Expected: 0 errors.
Run: `npm run build`
Expected: success.

- [ ] **Step 7: Manual check with the real key (optional)**

Start the backend with `SYGIC_API_KEY` and `SYGIC_REFERER` in the environment (from `.env`, without printing it), then `npm run dev`. Open a trip Bratislava to Brno on `/mapa`. Expected: checkboxes SK and CZ. Check CZ: the route is about 132.9 km. Save, reload: CZ is still checked.

- [ ] **Step 8: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts src/routes/mapa/+page.svelte src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts src/lib/i18n/i18n-types.ts
git commit -m "feat(route-map): per-country avoid paid roads checkboxes"
```

---

### Task 8: Integration tests with the mock router

**Files:**
- Modify: [tests/integration/wdio.server.conf.ts](../../../tests/integration/wdio.server.conf.ts) (`onPrepare`: `process.env` and the `spawn` env)
- Modify: [.github/workflows/test.yml](../../../.github/workflows/test.yml) (both `docker run` steps, lines about 175 and 261)
- Modify: [tests/integration/specs/tier2/route-map.spec.ts](../../../tests/integration/specs/tier2/route-map.spec.ts)
- Modify: [.claude/rules/integration-tests.md](../../../.claude/rules/integration-tests.md) (section "Pass the geocoder mock to the container")

`grep -rn MOCK_GEOCODER_DIR` on 2026-09-29 found the env only in `wdio.server.conf.ts`
(2 places), `test.yml` (2 places) and docs. No script in `scripts/` or `package.json`
starts a container. Set the router mock in the same places.

**Interfaces:**
- Consumes: `KNIHA_JAZD_MOCK_ROUTER` (Task 4), `data-test="avoid-cze:tolls"`, `data-test="actual-km"` (Task 7 and existing).

The mock turns routing on for the whole suite. Before this task, no spec could route (see the header of `route-map.spec.ts`, constraint 1 and 3). A spec that expected a routing failure now gets a mock route. Run the full suite in Step 5 to find such a spec.

- [ ] **Step 1: Pass the mock env and scrub the real key**

`wdio.server.conf.ts`, in `onPrepare` next to the geocoder line:

```ts
    // Mock router (Task 85): fixed, offline routes. Wins over SYGIC_API_KEY,
    // and the key is scrubbed below too, so the suite never calls Sygic.
    process.env.KNIHA_JAZD_MOCK_ROUTER = '1';
```

In the `spawn` env, after `KNIHA_JAZD_MOCK_GEOCODER_DIR`:

```ts
        KNIHA_JAZD_MOCK_ROUTER: '1',
        SYGIC_API_KEY: '',
        SYGIC_REFERER: '',
```

`test.yml`, in both `docker run` commands, after the geocoder line:

```yaml
            -e KNIHA_JAZD_MOCK_ROUTER=1 \
```

- [ ] **Step 1b: Update the docker-mode note**

In [.claude/rules/integration-tests.md](../../../.claude/rules/integration-tests.md), rename the section to "Pass the mocks to the container" and add:

```markdown
`KNIHA_JAZD_MOCK_ROUTER=1` is required too. Without it `route-map.spec.ts` fails,
or calls a real routing service if the container has `SYGIC_API_KEY`.
```

- [ ] **Step 2: Write the tests** (in `describe('Map View (V2, offline-reachable flows)')`)

Both tests use a direct route. Loop mode shows no checkboxes (Global Constraints).

A saved route sets both endpoints from its own waypoints (`rehydrateEndpoints`),
so `recalculate-btn` is enabled with no place-book entries. With the mock,
`recalculate-btn` routes the 3 saved points and returns one route of 100.0 km.

```ts
    it('reopens a saved route with its avoid list checked', async () => {
      const trip = await seedTrip({
        vehicleId,
        startDatetime: '2026-03-13T08:00',
        endDatetime: '2026-03-13T10:00',
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 65,
        odometer: 50065,
        purpose: 'Business trip',
      });
      await rpc<null>('save_trip_route', {
        tripId: trip.id as string,
        waypoints: CANNED_VIA_WAYPOINTS,
        polyline: CANNED_POLYLINE,
        targetKm: 65,
        roadKm: 65,
        mode: 'direct',
        avoid: ['cze:tolls'],
      });

      await openMap(trip.id as string);
      await waitForMapOutcome('route');

      const box = await $('[data-test="avoid-cze:tolls"]');
      await box.waitForDisplayed();
      expect(await box.isSelected()).toBe(true);
    });

    it('routes again with the avoid value when a country is checked', async () => {
      const trip = await seedTrip({
        vehicleId,
        startDatetime: '2026-03-14T08:00',
        endDatetime: '2026-03-14T10:00',
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 65,
        odometer: 50065,
        purpose: 'Business trip',
      });
      await saveDirectRouteWithVia(trip.id as string, 65);

      await openMap(trip.id as string);
      await waitForMapOutcome('route');
      // A saved route with no avoid list offers no options until it is routed.
      expect(await $('[data-test="avoid-options"]').isExisting()).toBe(false);

      await $('[data-test="recalculate-btn"]').click();
      await expect($('[data-test="actual-km"]')).toHaveText('100.0 km');

      await $('[data-test="avoid-cze:tolls"]').click();
      // The mock returns 120.0 km for any non-empty avoid list.
      await expect($('[data-test="actual-km"]')).toHaveText('120.0 km');
      expect(await $('[data-test="avoid-cze:tolls"]').isSelected()).toBe(true);

      // Review Focus 5: the save must carry the avoid list that produced
      // the shown route. Reopen from the database to prove it was stored.
      await $('[data-test="save-btn"]').click();
      await $('[data-test="saved-notice"]').waitForDisplayed();
      await openMap(trip.id as string);
      await waitForMapOutcome('route');
      const reopened = await $('[data-test="avoid-cze:tolls"]');
      await reopened.waitForDisplayed();
      expect(await reopened.isSelected()).toBe(true);
    });
```

Update the file header: constraint 1 and 3 now have a mock router for the avoid flow. Keep the text short.

- [ ] **Step 3: Build the artifacts**

Run: `npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web`

- [ ] **Step 4: Run the spec and see it pass**

Run: `npx wdio run tests/integration/wdio.server.conf.ts --spec tests/integration/specs/tier2/route-map.spec.ts`
Expected: PASS, with the 2 new tests.

- [ ] **Step 5: Run the full integration suite**

Run: `npm run test:integration`
Expected: PASS. If a spec now fails because routing succeeds, fix that spec to match the mock, and say so in the commit body.

- [ ] **Step 6: Type check the tests**

Run: `npm run typecheck:tests`
Expected: 0 errors.

- [ ] **Step 7: Commit**

```bash
git add tests/integration/wdio.server.conf.ts tests/integration/specs/tier2/route-map.spec.ts .github/workflows/test.yml .claude/rules/integration-tests.md
git commit -m "test(route-map): cover avoid checkboxes with an offline mock router"
```

---

### Task 9: Documentation

**Files:**
- Modify: [DECISIONS.md](../../../DECISIONS.md) (through `/decision`)
- Modify: [CHANGELOG.md](../../../CHANGELOG.md) (through `/changelog`)
- Modify: [docs/features/route-maps.md](../../../docs/features/route-maps.md), [CLAUDE.md](../../../CLAUDE.md) (the env var table under "Database Location"), [README.md](../../../README.md), [README.en.md](../../../README.en.md)
- Modify: [_tasks/85-route-avoid-tolls-per-country/01-task.md](../../../_tasks/_done/85-route-avoid-tolls-per-country/01-task.md) (Status, and the integration test note), [_tasks/index.md](../../../_tasks/index.md)

- [ ] **Step 1: ADR** through `/decision`: "Sygic as the optional routing provider". Content: per-country avoid needs a service that knows countries. Public OSRM rejects `exclude`, Valhalla and Google are global only, and Google's terms forbid its routes on a non-Google map (section 19.2). Sygic is chosen when `SYGIC_API_KEY` is set. No silent fallback to OSRM. Link [01-task.md](./01-task.md).
- [ ] **Step 2: CHANGELOG** through `/changelog`, section `Pridané` (Slovak): the per-country "Vyhnúť sa spoplatneným cestám" checkboxes, available when the server has `SYGIC_API_KEY`.
- [ ] **Step 3: Env var table** in `CLAUDE.md`: add `SYGIC_API_KEY`, `SYGIC_REFERER`, `KNIHA_JAZD_MOCK_ROUTER` with default and purpose. Same rows in the README env sections (Slovak in `README.md`). No key or referer values.
- [ ] **Step 4: Feature doc** `docs/features/route-maps.md`: a section "Avoid paid roads per country": provider choice, the `tolls`-only rule, the option union, the saved column, the mock.
- [ ] **Step 5: Task status**: set `**Status:** Complete` in `01-task.md`, replace "Check first how the integration suite fakes the routing service today" with "The suite uses `KNIHA_JAZD_MOCK_ROUTER` (Task 8)". Move the index row per [_tasks/CLAUDE.md](../../CLAUDE.md).
- [ ] **Step 6: Final check** with `/verify`: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`, `npm run check`, `npm run typecheck:tests`, `npm run test:integration`.
- [ ] **Step 7: Commit**

```bash
git add DECISIONS.md CHANGELOG.md docs/features/route-maps.md CLAUDE.md README.md README.en.md _tasks/85-route-avoid-tolls-per-country/01-task.md _tasks/index.md
git commit -m "docs: document per-country toll avoidance and the Sygic provider"
```

---

## Out of scope

- The homelab stack env vars (`SYGIC_API_KEY`, `SYGIC_REFERER`). That change is in the infra repo, after this merges.
- The Sygic terms on stored geometry and the plan quota (open items in [01-task.md](./01-task.md)). Check them before the deploy, not in code.
