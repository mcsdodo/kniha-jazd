**Date:** 2026-09-03
**Subject:** Route maps V2 — origin/destination routing, alternatives, manual editing — implementation plan
**Status:** Planning

# Route Maps V2 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Route the journey the row actually records — geocode its origin and
destination, route A→B with alternatives, and let the user drag the line into the shape
they really drove.

**Architecture:** Mode is chosen in Rust from the row's own origin/destination: same
place twice → today's genetic-algorithm loop, unchanged; different places → a direct
A→B route. Free text becomes coordinates through a geocoder behind an injected trait,
cached in a `place_aliases` table so each distinct name is resolved once and confirmed
by a human once. Editing is mode-agnostic and re-routes only on pointer release, and
**where a dragged-in waypoint lands is computed in Rust**, not the browser.

**Tech Stack:** Rust (diesel/SQLite, reqwest, async-trait, wiremock), SvelteKit 5 runes,
Leaflet, typesafe-i18n, WebdriverIO.

**Read first:** [01-task.md](./01-task.md) · [02-design.md](./02-design.md) ·
[V1 as built](../_done/70-route-map-integration/)

---

## Ground rules for every task

- **TDD, always.** Write the failing test, run it, watch it fail *for the stated
  reason*, implement, run it green, commit. A test that passes before you implement
  anything is testing nothing.
- **No test touches the network.** [wiremock](https://docs.rs/wiremock/) for HTTP
  clients; hand-written stubs for the provider traits.
- **Commands:**
  ```bash
  # one filtered backend test
  cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "test_name_filter"
  # the whole backend suite
  cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
  # one integration spec (debug build required)
  npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/route-map.spec.ts
  ```
- **Stage only the files each task names.** Never `git add -A`.
- New `_internal` functions need no re-export work:
  [commands_internal/mod.rs](../../src-tauri/core/src/commands_internal/mod.rs) already
  does `pub use route_maps::*;`.

---

# Phase 1 — Geocoding foundation

## Task 1: `normalise()` — one notion of "the same place name"

**Files:**
- Create: [src-tauri/core/src/route_map/geocode.rs](../../src-tauri/core/src/route_map/geocode.rs)
- Create: [src-tauri/core/src/route_map/geocode_tests.rs](../../src-tauri/core/src/route_map/geocode_tests.rs)
- Modify: [src-tauri/core/src/route_map/mod.rs](../../src-tauri/core/src/route_map/mod.rs)

This function is the cache key *and* the mode comparison. If it is wrong, "Spisska" and
"Spišská" become two places and the alias cache silently stops working.

**Step 1: Write the failing tests**

Create `geocode_tests.rs`:

```rust
//! Tests for geocoding: normalisation and the HTTP provider.
//!
//! The provider tests run against `wiremock` — nothing here reaches a real
//! geocoding service.

use super::geocode::*;

#[test]
fn normalise_strips_slovak_diacritics() {
    assert_eq!(normalise("Spišská Nová Ves"), "spisska nova ves");
    assert_eq!(normalise("Košice"), "kosice");
    assert_eq!(normalise("Ľubochňa"), "lubochna");
    assert_eq!(normalise("Žilina"), "zilina");
    assert_eq!(normalise("Dolný Kubín"), "dolny kubin");
}

#[test]
fn normalise_folds_case() {
    assert_eq!(normalise("BRATISLAVA"), "bratislava");
    assert_eq!(normalise("BrAtIsLaVa"), "bratislava");
}

#[test]
fn normalise_collapses_whitespace() {
    assert_eq!(normalise("  Spisska   Nova  Ves "), "spisska nova ves");
    assert_eq!(normalise("\tBratislava\n"), "bratislava");
}

/// The whole point: a row typed without diacritics and one typed with them
/// must land on the same cache entry.
#[test]
fn normalise_makes_accented_and_unaccented_spellings_equal() {
    assert_eq!(normalise("Spišská"), normalise("Spisska"));
    assert_eq!(normalise("Prešov"), normalise("presov"));
}

#[test]
fn normalise_of_blank_input_is_empty() {
    assert_eq!(normalise(""), "");
    assert_eq!(normalise("   "), "");
}
```

Register the module in `route_map/mod.rs` — add `pub mod geocode;` beside the other
`pub mod` lines, `pub use geocode::{normalise, GeocodeProvider, HttpGeocodeProvider, Place};`
beside the other `pub use` lines, and the test wiring beside the others:

```rust
#[cfg(test)]
#[path = "geocode_tests.rs"]
mod geocode_tests;
```

**Step 2: Run to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "normalise"
```
Expected: FAIL — compile error, `geocode.rs` does not exist yet.

**Step 3: Implement**

Create `geocode.rs`:

```rust
//! Free-text place names to coordinates.
//!
//! Everything sits behind [`GeocodeProvider`] so tests never touch the
//! network, exactly as [`crate::route_map::RouteProvider`] does for OSRM.

/// Lowercase, fold Slovak diacritics, collapse whitespace.
///
/// This is the key the place-alias cache is stored under AND the comparison
/// that decides loop vs direct mode. One notion of "the same place name" in
/// the whole feature: if these two ever disagreed, a row could be routed as
/// A→B while its endpoints collided onto one cache entry.
///
/// A hand-written fold rather than a Unicode-normalisation dependency: the
/// Slovak alphabet is a closed set of 15 accented letters, the mapping is
/// obvious, and a table we can read beats a crate we would have to trust.
pub fn normalise(query: &str) -> String {
    let folded: String = query.to_lowercase().chars().map(fold_diacritic).collect();
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Applied AFTER lowercasing, so only lowercase forms need listing.
/// Czech and Polish forms are included because border towns get typed either
/// way and folding one extra letter costs nothing.
fn fold_diacritic(c: char) -> char {
    match c {
        'á' | 'ä' | 'à' | 'â' | 'ą' => 'a',
        'č' | 'ć' | 'ç' => 'c',
        'ď' => 'd',
        'é' | 'ě' | 'è' | 'ê' | 'ë' | 'ę' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ĺ' | 'ľ' | 'ł' => 'l',
        'ň' | 'ń' => 'n',
        'ó' | 'ô' | 'ö' | 'ò' | 'õ' => 'o',
        'ŕ' | 'ř' => 'r',
        'š' | 'ś' => 's',
        'ť' => 't',
        'ú' | 'ů' | 'ü' | 'ù' | 'û' => 'u',
        'ý' | 'ÿ' => 'y',
        'ž' | 'ź' | 'ż' => 'z',
        other => other,
    }
}
```

**Step 4: Run to verify it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "normalise"
```
Expected: PASS, 5 tests.

**Step 5: Commit**

```bash
git add src-tauri/core/src/route_map/geocode.rs src-tauri/core/src/route_map/geocode_tests.rs src-tauri/core/src/route_map/mod.rs
git commit -m "feat(route-map): add place-name normalisation for geocoding"
```

---

## Task 2: `GeocodeProvider` trait and the Nominatim client

**Files:**
- Modify: [src-tauri/core/src/route_map/geocode.rs](../../src-tauri/core/src/route_map/geocode.rs)
- Modify: [src-tauri/core/src/route_map/geocode_tests.rs](../../src-tauri/core/src/route_map/geocode_tests.rs)

**The gotcha this task exists to pin:** Nominatim returns `lat` and `lon` as **JSON
strings**, not numbers. Deserialising them into `f64` fails at runtime with a message
nobody expects, so the parsing test is the valuable one here.

**Step 1: Write the failing tests**

Append to `geocode_tests.rs`:

```rust
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Nominatim reports coordinates as STRINGS. Parsing them as f64 directly
/// fails; this test is why the response struct takes String and converts.
#[tokio::test]
async fn parses_string_coordinates_from_the_geocoder() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/search.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "lat": "48.9444",
                "lon": "20.5675",
                "display_name": "Spišská Nová Ves, okres Spišská Nová Ves, Slovensko"
            }
        ])))
        .mount(&server)
        .await;

    let client = HttpGeocodeProvider::new(server.uri());
    let places = client.search("Spisska").await.unwrap();

    assert_eq!(places.len(), 1);
    assert!((places[0].lat - 48.9444).abs() < 1e-6);
    assert!((places[0].lon - 20.5675).abs() < 1e-6);
    assert!(places[0].display_name.starts_with("Spišská Nová Ves"));
}

/// An unknown place is a legitimate answer meaning "place it by hand",
/// NOT an error the UI should show a Retry button for.
#[tokio::test]
async fn an_empty_result_is_not_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/search.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let client = HttpGeocodeProvider::new(server.uri());
    let places = client.search("Nikde").await.unwrap();
    assert!(places.is_empty());
}

#[tokio::test]
async fn surfaces_http_429_as_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/search.*"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let client = HttpGeocodeProvider::new(server.uri());
    let err = client
        .search("Bratislava")
        .await
        .expect_err("a rate-limit response must not look like 'no results'");
    assert!(err.contains("429"), "error should carry the status, got: {err}");
}

#[tokio::test]
async fn malformed_json_is_an_error_not_a_panic() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/search.*"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{not json"))
        .mount(&server)
        .await;

    let client = HttpGeocodeProvider::new(server.uri());
    assert!(client.search("Bratislava").await.is_err());
}

/// The usage policy requires an identifying User-Agent. Sending none gets the
/// application blocked, so this is a compliance test, not a nicety.
#[tokio::test]
async fn sends_an_identifying_user_agent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/search.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let client = HttpGeocodeProvider::new(server.uri());
    client.search("Bratislava").await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let ua = requests[0]
        .headers
        .get("user-agent")
        .expect("every request must identify the application")
        .to_str()
        .unwrap();
    assert!(ua.contains("kniha-jazd"), "got: {ua}");
}
```

**Step 2: Run to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "geocod"
```
Expected: FAIL — `HttpGeocodeProvider` not found.

**Step 3: Implement**

Append to `geocode.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Public Nominatim instance.
const PUBLIC_NOMINATIM_URL: &str = "https://nominatim.openstreetmap.org";

/// Required by the Nominatim usage policy: every request must identify the
/// application. The same obligation `tiles.rs` meets for OSM tiles.
const USER_AGENT: &str = concat!("kniha-jazd/", env!("CARGO_PKG_VERSION"));

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// The policy caps automated use at one request per second.
const MIN_REQUEST_INTERVAL: Duration = Duration::from_secs(1);

/// How many candidates to offer. Five fits a picker without scrolling and is
/// well past the point where a sixth would change anyone's choice.
const MAX_CANDIDATES: usize = 5;

/// A geocoded place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub lat: f64,
    pub lon: f64,
    /// What the geocoder called it — shown in the picker, and stored as the
    /// waypoint's `name` so the map view can label the endpoint.
    pub display_name: String,
}

#[async_trait::async_trait]
pub trait GeocodeProvider: Send + Sync {
    /// Up to [`MAX_CANDIDATES`] matches, best first.
    ///
    /// An empty vec is a valid answer meaning "no such place" — the caller
    /// asks the user to place the pin by hand. It is NOT an error.
    async fn search(&self, query: &str) -> Result<Vec<Place>, String>;
}

pub struct HttpGeocodeProvider {
    base_url: String,
    /// Kept as an error rather than panicking on a client-build failure, so it
    /// can reach the UI like any other lookup failure.
    client: Result<reqwest::Client, String>,
    /// When the last request went out, so [`MIN_REQUEST_INTERVAL`] can be
    /// honoured. A mutex rather than an atomic because the wait has to happen
    /// while holding the slot, or two concurrent lookups both "see" an old
    /// timestamp and fire together.
    last_request: tokio::sync::Mutex<Option<std::time::Instant>>,
}

impl HttpGeocodeProvider {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| format!("Could not create an HTTP client for the geocoder: {e}"));

        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
            last_request: tokio::sync::Mutex::new(None),
        }
    }

    pub fn public() -> Self {
        Self::new(PUBLIC_NOMINATIM_URL)
    }

    /// Block until at least [`MIN_REQUEST_INTERVAL`] has passed since the last
    /// request, then claim the slot.
    async fn throttle(&self) {
        let mut last = self.last_request.lock().await;
        if let Some(previous) = *last {
            let elapsed = previous.elapsed();
            if elapsed < MIN_REQUEST_INTERVAL {
                tokio::time::sleep(MIN_REQUEST_INTERVAL - elapsed).await;
            }
        }
        *last = Some(std::time::Instant::now());
    }
}

/// One Nominatim result. `lat` and `lon` arrive as STRINGS — deserialising
/// them straight into f64 fails, which is what `parses_string_coordinates_from_the_geocoder`
/// pins.
#[derive(Deserialize)]
struct NominatimPlace {
    lat: String,
    lon: String,
    display_name: String,
}

#[async_trait::async_trait]
impl GeocodeProvider for HttpGeocodeProvider {
    async fn search(&self, query: &str) -> Result<Vec<Place>, String> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let client = self.client.as_ref().map_err(|e| e.clone())?;
        self.throttle().await;

        let response = client
            .get(format!("{}/search", self.base_url))
            .query(&[
                ("q", trimmed),
                ("format", "jsonv2"),
                ("limit", &MAX_CANDIDATES.to_string()),
                ("countrycodes", "sk"),
                ("accept-language", "sk"),
            ])
            .send()
            .await
            .map_err(|e| {
                format!(
                    "Could not reach the geocoding service at {}: {e}. Check your internet connection and try again.",
                    self.base_url
                )
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "Geocoding service returned HTTP {} ({}). Try again in a moment.",
                status.as_u16(),
                status.canonical_reason().unwrap_or("unknown")
            ));
        }

        let body: Vec<NominatimPlace> = response
            .json()
            .await
            .map_err(|e| format!("Could not read the geocoding service response: {e}"))?;

        // A result whose coordinates will not parse is dropped rather than
        // failing the whole lookup — one bad row must not cost the user the
        // four good ones next to it.
        Ok(body
            .into_iter()
            .filter_map(|p| {
                Some(Place {
                    lat: p.lat.parse().ok()?,
                    lon: p.lon.parse().ok()?,
                    display_name: p.display_name,
                })
            })
            .take(MAX_CANDIDATES)
            .collect())
    }
}
```

**Step 4: Run to verify it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "geocod"
```
Expected: PASS, 10 tests.

**Step 5: Commit**

```bash
git add src-tauri/core/src/route_map/geocode.rs src-tauri/core/src/route_map/geocode_tests.rs
git commit -m "feat(route-map): add Nominatim geocoding behind a provider trait"
```

---

## Task 3: `place_aliases` table

**Files:**
- Create: [2026-09-03-100000_add_place_aliases/up.sql](../../src-tauri/core/migrations/2026-09-03-100000_add_place_aliases/up.sql)
- Create: [2026-09-03-100000_add_place_aliases/down.sql](../../src-tauri/core/migrations/2026-09-03-100000_add_place_aliases/down.sql)
- Modify: [src-tauri/core/src/schema.rs](../../src-tauri/core/src/schema.rs)
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs)
- Modify: [src-tauri/core/src/db.rs](../../src-tauri/core/src/db.rs)
- Modify: [src-tauri/core/src/db_tests.rs](../../src-tauri/core/src/db_tests.rs)

**Step 1: Write the failing test**

Append to `db_tests.rs`:

```rust
#[test]
fn place_alias_round_trips() {
    let db = Database::in_memory().unwrap();
    let alias = PlaceAlias {
        normalised_query: "spisska".into(),
        lat: 48.9444,
        lon: 20.5675,
        display_name: "Spišská Nová Ves".into(),
        source: AliasSource::Geocoder,
        created_at: Utc::now(),
    };
    db.save_place_alias(&alias).unwrap();

    let loaded = db.get_place_alias("spisska").unwrap().unwrap();
    assert_eq!(loaded.display_name, "Spišská Nová Ves");
    assert!((loaded.lat - 48.9444).abs() < 1e-9);
    assert_eq!(loaded.source, AliasSource::Geocoder);
}

#[test]
fn unknown_place_alias_is_none() {
    let db = Database::in_memory().unwrap();
    assert!(db.get_place_alias("nikde").unwrap().is_none());
}

/// Re-confirming a place must replace the old pin, not fail on the primary
/// key — this is how a user corrects a wrong pick.
#[test]
fn saving_a_place_alias_twice_replaces_it() {
    let db = Database::in_memory().unwrap();
    let mut alias = PlaceAlias {
        normalised_query: "spisska".into(),
        lat: 1.0,
        lon: 1.0,
        display_name: "Wrong".into(),
        source: AliasSource::Geocoder,
        created_at: Utc::now(),
    };
    db.save_place_alias(&alias).unwrap();

    alias.lat = 48.9444;
    alias.display_name = "Spišská Nová Ves".into();
    alias.source = AliasSource::Manual;
    db.save_place_alias(&alias).unwrap();

    let loaded = db.get_place_alias("spisska").unwrap().unwrap();
    assert_eq!(loaded.display_name, "Spišská Nová Ves");
    assert_eq!(loaded.source, AliasSource::Manual);
}
```

**Step 2: Run to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "place_alias"
```
Expected: FAIL — `PlaceAlias` not found.

**Step 3: Implement**

`up.sql`:

```sql
-- Task 72: remembered geocoding results, keyed by NORMALISED free text.
-- Global, not vehicle-scoped: a village is in the same place whichever car
-- drove there, and per-vehicle scoping would make the user re-confirm every
-- place for every vehicle. See _tasks/72-route-map-origin-destination/02-design.md.
CREATE TABLE place_aliases (
    normalised_query TEXT PRIMARY KEY,
    lat REAL NOT NULL,
    lon REAL NOT NULL,
    display_name TEXT NOT NULL,
    -- 'geocoder' (picked from candidates) or 'manual' (pin dropped by hand).
    source TEXT NOT NULL,
    created_at TEXT NOT NULL
);
```

`down.sql`:

```sql
-- Forward-only in practice (ADR-012); no diesel CLI revert runs in this repo.
DROP TABLE IF EXISTS place_aliases;
```

In `schema.rs`, beside the other `diesel::table!` blocks:

```rust
// Added via migration 2026-09-03-100000_add_place_aliases (Task 72)
diesel::table! {
    place_aliases (normalised_query) {
        normalised_query -> Text,
        lat -> Double,
        lon -> Double,
        display_name -> Text,
        source -> Text,
        created_at -> Text,
    }
}
```

and add `place_aliases` to the existing `allow_tables_to_appear_in_same_query!` list.

In `models.rs`:

```rust
/// Who decided where this place is. Kept so a later audit can tell a
/// machine's guess from a human's placement without having to infer it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AliasSource {
    Geocoder,
    Manual,
}

impl AliasSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            AliasSource::Geocoder => "geocoder",
            AliasSource::Manual => "manual",
        }
    }
}

/// A remembered geocoding result. `normalised_query` is the output of
/// `route_map::geocode::normalise`, never raw user text.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceAlias {
    pub normalised_query: String,
    pub lat: f64,
    pub lon: f64,
    pub display_name: String,
    pub source: AliasSource,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Queryable)]
pub struct PlaceAliasRow {
    pub normalised_query: String,
    pub lat: f64,
    pub lon: f64,
    pub display_name: String,
    pub source: String,
    pub created_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = place_aliases)]
pub struct NewPlaceAliasRow<'a> {
    pub normalised_query: &'a str,
    pub lat: f64,
    pub lon: f64,
    pub display_name: &'a str,
    pub source: &'a str,
    pub created_at: &'a str,
}

impl From<PlaceAliasRow> for PlaceAlias {
    fn from(row: PlaceAliasRow) -> Self {
        PlaceAlias {
            normalised_query: row.normalised_query,
            lat: row.lat,
            lon: row.lon,
            display_name: row.display_name,
            // An unrecognised value means a human did NOT vouch for it, so
            // 'geocoder' is the safe reading of a corrupted row.
            source: match row.source.as_str() {
                "manual" => AliasSource::Manual,
                _ => AliasSource::Geocoder,
            },
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
}
```

Import `place_aliases` in the schema-use line at the top of `models.rs` alongside
`trip_routes`.

In `db.rs`, after the route-map CRUD block, mirroring `save_route_map`'s
delete-then-insert transaction:

```rust
    // ========================================================================
    // Place aliases — remembered geocoding results (Task 72)
    // ========================================================================

    /// Upsert. Re-confirming a place must REPLACE the old pin rather than
    /// fail on the primary key: that is how a user corrects a wrong pick.
    pub fn save_place_alias(&self, alias: &PlaceAlias) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        let created_at_str = alias.created_at.to_rfc3339();

        conn.transaction::<_, diesel::result::Error, _>(|tx| {
            diesel::delete(
                place_aliases::table
                    .filter(place_aliases::normalised_query.eq(&alias.normalised_query)),
            )
            .execute(tx)?;
            diesel::insert_into(place_aliases::table)
                .values(&NewPlaceAliasRow {
                    normalised_query: &alias.normalised_query,
                    lat: alias.lat,
                    lon: alias.lon,
                    display_name: &alias.display_name,
                    source: alias.source.as_str(),
                    created_at: &created_at_str,
                })
                .execute(tx)?;
            Ok(())
        })
    }

    pub fn get_place_alias(&self, normalised_query: &str) -> QueryResult<Option<PlaceAlias>> {
        let conn = &mut *self.conn.lock().unwrap();
        let row = place_aliases::table
            .filter(place_aliases::normalised_query.eq(normalised_query))
            .first::<PlaceAliasRow>(conn)
            .optional()?;
        Ok(row.map(PlaceAlias::from))
    }
```

Add `place_aliases` to the `use crate::schema::{...}` list and `AliasSource, PlaceAlias,
PlaceAliasRow, NewPlaceAliasRow` to the models import at the top of `db.rs`.

**Step 4: Run to verify it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "place_alias"
```
Expected: PASS, 3 tests.

**Step 5: Commit**

```bash
git add src-tauri/core/migrations/2026-09-03-100000_add_place_aliases/ src-tauri/core/src/schema.rs src-tauri/core/src/models.rs src-tauri/core/src/db.rs src-tauri/core/src/db_tests.rs
git commit -m "feat(route-map): add place_aliases table for remembered geocoding"
```

---

## Task 4: `resolve_place` and `remember_place` commands

**Files:**
- Modify: [src-tauri/core/src/commands_internal/route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs)
- Modify: [src-tauri/core/src/commands_internal/route_maps_tests.rs](../../src-tauri/core/src/commands_internal/route_maps_tests.rs)

**The invariant to protect:** looking is not committing. A resolve that wrote its first
guess would make a wrong guess permanent before the user ever saw it.

**Step 1: Write the failing tests**

Append to `route_maps_tests.rs`:

```rust
use crate::models::{AliasSource, PlaceAlias};
use crate::route_map::{GeocodeProvider, Place};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Counts its calls, so "a cache hit costs no network" is asserted rather
/// than assumed.
struct CountingGeocoder {
    results: Vec<Place>,
    calls: AtomicUsize,
}

impl CountingGeocoder {
    fn returning(results: Vec<Place>) -> Self {
        Self { results, calls: AtomicUsize::new(0) }
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl GeocodeProvider for CountingGeocoder {
    async fn search(&self, _query: &str) -> Result<Vec<Place>, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.results.clone())
    }
}

fn a_place(name: &str) -> Place {
    Place { lat: 48.9444, lon: 20.5675, display_name: name.into() }
}

#[tokio::test]
async fn a_cached_alias_resolves_without_calling_the_geocoder() {
    let db = Database::in_memory().unwrap();
    db.save_place_alias(&PlaceAlias {
        normalised_query: "spisska".into(),
        lat: 48.9444,
        lon: 20.5675,
        display_name: "Spišská Nová Ves".into(),
        source: AliasSource::Geocoder,
        created_at: chrono::Utc::now(),
    })
    .unwrap();

    let geocoder = CountingGeocoder::returning(vec![a_place("should not be used")]);
    let result = resolve_place_internal(&db, &geocoder, "Spišská".into())
        .await
        .unwrap();

    assert_eq!(geocoder.calls(), 0, "a cache hit must cost no network call");
    let resolved = result.resolved.expect("cached alias must resolve");
    assert_eq!(resolved.display_name, "Spišská Nová Ves");
    assert!(result.candidates.is_empty());
}

#[tokio::test]
async fn a_cache_miss_returns_candidates_and_writes_nothing() {
    let db = Database::in_memory().unwrap();
    let geocoder = CountingGeocoder::returning(vec![a_place("Levoča"), a_place("Levočská")]);

    let result = resolve_place_internal(&db, &geocoder, "Levoca".into())
        .await
        .unwrap();

    assert_eq!(geocoder.calls(), 1);
    assert!(result.resolved.is_none());
    assert_eq!(result.candidates.len(), 2);
    assert!(
        db.get_place_alias("levoca").unwrap().is_none(),
        "resolving must not persist a guess the user has not confirmed"
    );
}

#[tokio::test]
async fn no_candidates_is_a_successful_empty_result() {
    let db = Database::in_memory().unwrap();
    let geocoder = CountingGeocoder::returning(vec![]);
    let result = resolve_place_internal(&db, &geocoder, "Nikde".into())
        .await
        .unwrap();
    assert!(result.resolved.is_none());
    assert!(result.candidates.is_empty());
}

#[tokio::test]
async fn remembering_a_place_stores_it_under_the_normalised_key() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();

    remember_place_internal(
        &db,
        &app_state,
        "  Spišská  ".into(),
        a_place("Spišská Nová Ves"),
        AliasSource::Geocoder,
    )
    .unwrap();

    // Stored under the normalised key, so a differently-typed row hits it.
    assert!(db.get_place_alias("spisska").unwrap().is_some());

    let geocoder = CountingGeocoder::returning(vec![a_place("unused")]);
    let result = resolve_place_internal(&db, &geocoder, "SPISSKA".into())
        .await
        .unwrap();
    assert_eq!(geocoder.calls(), 0);
    assert!(result.resolved.is_some());
}

#[test]
fn remembering_a_place_is_refused_in_read_only_mode() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    app_state.set_read_only(true);

    assert!(remember_place_internal(
        &db,
        &app_state,
        "Spisska".into(),
        a_place("Spišská Nová Ves"),
        AliasSource::Geocoder,
    )
    .is_err());
}

#[tokio::test]
async fn a_blank_place_name_is_an_error() {
    let db = Database::in_memory().unwrap();
    let geocoder = CountingGeocoder::returning(vec![]);
    assert!(resolve_place_internal(&db, &geocoder, "   ".into())
        .await
        .is_err());
}
```

> Check how `AppState` exposes read-only before writing the last test — match whatever
> the existing read-only tests in this file already do rather than inventing a setter.

**Step 2: Run to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "place"
```
Expected: FAIL — `resolve_place_internal` not found.

**Step 3: Implement**

Append to `route_maps.rs` (and extend its `use` lines with
`crate::models::{AliasSource, PlaceAlias}` and
`crate::route_map::geocode::{normalise, GeocodeProvider, Place}`):

```rust
// ---------------------------------------------------------------------------
// Place resolution (Task 72)
// ---------------------------------------------------------------------------

/// The answer to "where is this place?".
///
/// `resolved` is `Some` only on a cache hit — a place a human has already
/// confirmed. On a miss it is `None` and `candidates` carries what the
/// geocoder offered, which may be empty: that means "place it by hand", not
/// "something failed".
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceResolution {
    /// As typed, for display. The cache key is the normalised form.
    pub query: String,
    pub resolved: Option<Place>,
    pub candidates: Vec<Place>,
}

/// Cache first, geocoder second. **Writes nothing** — see the module docs on
/// why proposing and committing are separate calls.
pub async fn resolve_place_internal(
    db: &Database,
    provider: &dyn GeocodeProvider,
    query: String,
) -> Result<PlaceResolution, String> {
    let key = normalise(&query);
    if key.is_empty() {
        return Err("Trip has no origin or destination to look up.".to_string());
    }

    if let Some(alias) = db.get_place_alias(&key).map_err(|e| e.to_string())? {
        return Ok(PlaceResolution {
            query,
            resolved: Some(Place {
                lat: alias.lat,
                lon: alias.lon,
                display_name: alias.display_name,
            }),
            candidates: Vec::new(),
        });
    }

    let candidates = provider.search(&query).await?;
    Ok(PlaceResolution { query, resolved: None, candidates })
}

/// Commit the user's choice, so this name never needs looking up again.
pub fn remember_place_internal(
    db: &Database,
    app_state: &AppState,
    query: String,
    place: Place,
    source: AliasSource,
) -> Result<(), String> {
    check_read_only!(app_state);
    let key = normalise(&query);
    if key.is_empty() {
        return Err("Cannot remember a place with no name.".to_string());
    }

    db.save_place_alias(&PlaceAlias {
        normalised_query: key,
        lat: place.lat,
        lon: place.lon,
        display_name: place.display_name,
        source,
        // Stamped here, not accepted from the caller — same reasoning as
        // save_trip_route_internal.
        created_at: Utc::now(),
    })
    .map_err(|e| e.to_string())
}
```

**Step 4: Run to verify it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "place"
```
Expected: PASS, 9 tests (6 new + the 3 db tests).

**Step 5: Commit**

```bash
git add src-tauri/core/src/commands_internal/route_maps.rs src-tauri/core/src/commands_internal/route_maps_tests.rs
git commit -m "feat(route-map): add resolve_place and remember_place commands"
```

---

# Phase 2 — Direct routing

## Task 5: `RouteMode` and `mode_for()`

**Files:**
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs)
- Modify: [src-tauri/core/src/commands_internal/route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs)
- Modify: [src-tauri/core/src/commands_internal/route_maps_tests.rs](../../src-tauri/core/src/commands_internal/route_maps_tests.rs)

**Step 1: Write the failing tests**

```rust
use crate::models::RouteMode;

#[test]
fn the_same_place_twice_is_a_loop() {
    assert_eq!(mode_for("Domov", "Domov").unwrap(), RouteMode::Loop);
}

/// The mode comparison and the alias cache MUST share one notion of sameness,
/// or a row could route A→B while its endpoints collide onto one cache entry.
#[test]
fn sameness_is_judged_after_normalisation() {
    assert_eq!(mode_for("Spišská", "spisska ").unwrap(), RouteMode::Loop);
}

#[test]
fn different_places_are_a_direct_route() {
    assert_eq!(
        mode_for("Bratislava", "Spišská Nová Ves").unwrap(),
        RouteMode::Direct
    );
}

/// A blank endpoint must NOT quietly become a home loop: that hands the user
/// a map of somewhere they never were, labelled as evidence.
#[test]
fn a_blank_endpoint_is_an_error_not_a_loop() {
    assert!(mode_for("", "Košice").is_err());
    assert!(mode_for("Košice", "   ").is_err());
    assert!(mode_for("", "").is_err());
}
```

**Step 2: Run to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "mode_for OR is_a_loop OR direct_route"
```
Expected: FAIL — `mode_for` not found.

**Step 3: Implement**

In `models.rs`:

```rust
/// Which producer built a route. Serialised lowercase so the enum, the JSON
/// the frontend sees, and the `trip_routes.mode` column all read the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RouteMode {
    /// Genetic-algorithm loop from the bundled dataset's home base (V1).
    Loop,
    /// A→B through the row's own geocoded origin and destination (V2).
    Direct,
}

impl RouteMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            RouteMode::Loop => "loop",
            RouteMode::Direct => "direct",
        }
    }
}
```

In `route_maps.rs`:

```rust
/// Loop when the row names the same place twice, direct otherwise.
///
/// Compared after `normalise` so this and the alias cache agree on what "the
/// same place" means. A blank endpoint is an error, never a silent loop.
pub fn mode_for(origin: &str, destination: &str) -> Result<RouteMode, String> {
    let o = normalise(origin);
    let d = normalise(destination);
    if o.is_empty() {
        return Err("Trip has no origin, so its route cannot be built.".to_string());
    }
    if d.is_empty() {
        return Err("Trip has no destination, so its route cannot be built.".to_string());
    }
    Ok(if o == d { RouteMode::Loop } else { RouteMode::Direct })
}
```

**Step 4: Run to verify it passes** — same command. Expected: PASS, 4 tests.

**Step 5: Commit**

```bash
git add src-tauri/core/src/models.rs src-tauri/core/src/commands_internal/route_maps.rs src-tauri/core/src/commands_internal/route_maps_tests.rs
git commit -m "feat(route-map): choose loop vs direct mode from the row's endpoints"
```

---

## Task 6: OSRM alternatives and duration

**Files:**
- Modify: [src-tauri/core/src/route_map/osrm.rs](../../src-tauri/core/src/route_map/osrm.rs)
- Modify: [src-tauri/core/src/route_map/osrm_tests.rs](../../src-tauri/core/src/route_map/osrm_tests.rs)
- Modify: [src-tauri/core/src/commands_internal/route_maps_tests.rs](../../src-tauri/core/src/commands_internal/route_maps_tests.rs) (StubProvider gains `duration_s`)

`FetchedRoute` gains `duration_s`, and the trait gains `fetch_alternatives` **with a
default implementation** that wraps `fetch` — so every existing stub keeps compiling and
only the HTTP provider overrides it.

**Step 1: Write the failing tests**

Append to `osrm_tests.rs`:

```rust
#[tokio::test]
async fn returns_duration_alongside_distance() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/route/v1/driving/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "Ok",
            "routes": [{
                "geometry": "_p~iF~ps|U_ulLnnqC",
                "distance": 118432.0,
                "duration": 5400.0
            }]
        })))
        .mount(&server)
        .await;

    let client = HttpRouteProvider::new(server.uri());
    let r = client.fetch(&[(48.935, 20.553), (48.997, 20.591)]).await.unwrap();
    assert!((r.duration_s - 5400.0).abs() < 1e-6);
}

/// OSRM lists alternatives fastest-first. That order IS the product decision
/// (navigation-app convention), so it must survive untouched — never re-sorted
/// by distance.
#[tokio::test]
async fn preserves_osrm_alternative_order() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/route/v1/driving/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "Ok",
            "routes": [
                { "geometry": "aaa", "distance": 120000.0, "duration": 5000.0 },
                { "geometry": "bbb", "distance": 100000.0, "duration": 6000.0 },
                { "geometry": "ccc", "distance": 130000.0, "duration": 7000.0 }
            ]
        })))
        .mount(&server)
        .await;

    let client = HttpRouteProvider::new(server.uri());
    let routes = client
        .fetch_alternatives(&[(48.935, 20.553), (48.997, 20.591)], 3)
        .await
        .unwrap();

    assert_eq!(routes.len(), 3);
    // The SHORTEST route is second. If anything ever sorts by distance this
    // assertion is what catches it.
    assert_eq!(routes[0].polyline, "aaa");
    assert_eq!(routes[1].polyline, "bbb");
    assert_eq!(routes[2].polyline, "ccc");
}

#[tokio::test]
async fn requests_alternatives_only_for_two_point_routes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/route/v1/driving/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "Ok",
            "routes": [{ "geometry": "aaa", "distance": 1000.0, "duration": 100.0 }]
        })))
        .mount(&server)
        .await;

    let client = HttpRouteProvider::new(server.uri());
    client
        .fetch_alternatives(
            &[(48.9, 20.5), (48.95, 20.55), (49.0, 20.6)],
            3,
        )
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let url = requests[0].url.to_string();
    assert!(
        !url.contains("alternatives=true"),
        "OSRM computes alternatives only for two-point queries; asking with vias \
         wastes the request. Got: {url}"
    );
}

/// A single-route response is one alternative, not a failure.
#[tokio::test]
async fn a_lone_route_is_returned_as_one_alternative() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/route/v1/driving/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "Ok",
            "routes": [{ "geometry": "aaa", "distance": 1000.0, "duration": 100.0 }]
        })))
        .mount(&server)
        .await;

    let client = HttpRouteProvider::new(server.uri());
    let routes = client
        .fetch_alternatives(&[(48.9, 20.5), (49.0, 20.6)], 3)
        .await
        .unwrap();
    assert_eq!(routes.len(), 1);
}
```

**Step 2: Run to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "alternativ OR duration"
```
Expected: FAIL — `fetch_alternatives` not found, `duration_s` not found.

**Step 3: Implement**

In `osrm.rs`:

```rust
pub struct FetchedRoute {
    pub polyline: String,
    pub road_km: f64,
    /// Estimated driving time in seconds (OSRM reports seconds directly).
    /// Displayed while choosing between alternatives; never persisted — the
    /// printed export renders no text, so a stored duration has no reader.
    pub duration_s: f64,
}
```

Add `duration` to `OsrmRoute`:

```rust
#[derive(Deserialize)]
struct OsrmRoute {
    geometry: String,
    /// Metres.
    distance: f64,
    /// Seconds.
    #[serde(default)]
    duration: f64,
}
```

Extend the trait with a defaulted method:

```rust
#[async_trait::async_trait]
pub trait RouteProvider: Send + Sync {
    /// `coords` are `(lat, lon)` pairs in visit order.
    async fn fetch(&self, coords: &[(f64, f64)]) -> Result<FetchedRoute, String>;

    /// Up to `max` routes for the same points, **in the order the service
    /// returned them** — OSRM lists them fastest first, which is the order the
    /// UI shows and must never re-sort.
    ///
    /// Defaulted to a single `fetch` so stubs need no extra impl. Only the
    /// HTTP provider overrides it.
    async fn fetch_alternatives(
        &self,
        coords: &[(f64, f64)],
        max: usize,
    ) -> Result<Vec<FetchedRoute>, String> {
        let _ = max;
        Ok(vec![self.fetch(coords).await?])
    }
}
```

Make `route_url` take the alternatives count, and add the override. OSRM computes
alternatives only for two-point queries, so asking with vias is a wasted parameter:

```rust
    fn route_url(&self, coords: &[(f64, f64)], alternatives: Option<usize>) -> String {
        let points = coords
            .iter()
            .map(|(lat, lon)| format!("{lon:.6},{lat:.6}"))
            .collect::<Vec<_>>()
            .join(";");

        let mut url = format!(
            "{}/route/v1/driving/{}?geometries=polyline&overview=full&steps=false",
            self.base_url, points
        );
        // Only meaningful for exactly two points — with vias OSRM returns the
        // single through-route regardless.
        if let Some(n) = alternatives {
            if coords.len() == 2 && n > 1 {
                url.push_str(&format!("&alternatives={}", n - 1));
            }
        }
        url
    }
```

Factor the existing `fetch` body into a private `request(&self, url) -> Result<Vec<FetchedRoute>, String>`
that keeps every current error branch (connection, non-2xx, non-`Ok` code, empty
`routes`) and maps each `OsrmRoute` to a `FetchedRoute`. Then:

```rust
    async fn fetch(&self, coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        // unchanged guard
        if coords.len() < 2 { /* ...existing error... */ }
        let mut routes = self.request(&self.route_url(coords, None)).await?;
        Ok(routes.remove(0))
    }

    async fn fetch_alternatives(
        &self,
        coords: &[(f64, f64)],
        max: usize,
    ) -> Result<Vec<FetchedRoute>, String> {
        if coords.len() < 2 { /* ...same error as fetch... */ }
        self.request(&self.route_url(coords, Some(max))).await
    }
```

Finally update `StubProvider` in
[route_maps_tests.rs](../../src-tauri/core/src/commands_internal/route_maps_tests.rs) to
set `duration_s` (any value, e.g. `3600.0`) — the compiler will point at it.

**Step 4: Run to verify it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core route_map
```
Expected: PASS, including the pre-existing OSRM tests.

**Step 5: Commit**

```bash
git add src-tauri/core/src/route_map/osrm.rs src-tauri/core/src/route_map/osrm_tests.rs src-tauri/core/src/commands_internal/route_maps_tests.rs
git commit -m "feat(route-map): fetch OSRM alternatives and durations"
```

---

## Task 7: Waypoint insertion placement (pure Rust)

**Files:**
- Modify: [src-tauri/core/src/commands_internal/route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs)
- Modify: [src-tauri/core/src/commands_internal/route_maps_tests.rs](../../src-tauri/core/src/commands_internal/route_maps_tests.rs)

When the user drags a new point off the line, something must decide which pair of
existing waypoints it belongs between. That is index arithmetic, and it belongs where
unit tests can reach it — see
[02-design.md](./02-design.md#where-a-new-waypoint-lands-is-decided-in-rust).

**Step 1: Write the failing tests**

```rust
use crate::route_map::polyline::encode;

/// Points along a straight west→east line, so "between" is unambiguous.
fn line_points() -> Vec<(f64, f64)> {
    (0..=10).map(|i| (48.9, 20.0 + i as f64 * 0.1)).collect()
}

fn wp(lat: f64, lon: f64) -> Waypoint {
    Waypoint { lat, lon, name: None, node_idx: None }
}

#[test]
fn a_point_dragged_mid_route_lands_between_the_endpoints() {
    let points = line_points();
    let waypoints = vec![wp(48.9, 20.0), wp(48.9, 21.0)];
    // Dragged off the middle of the line.
    let inserted = insert_waypoint(&waypoints, &encode(&points), 48.95, 20.5);

    assert_eq!(inserted.len(), 3);
    assert!((inserted[1].lat - 48.95).abs() < 1e-9);
    assert!((inserted[1].lon - 20.5).abs() < 1e-9);
}

#[test]
fn a_point_dragged_from_the_first_leg_lands_in_the_first_slot() {
    let points = line_points();
    // Three waypoints: start, middle of the line, end.
    let waypoints = vec![wp(48.9, 20.0), wp(48.9, 20.5), wp(48.9, 21.0)];
    let inserted = insert_waypoint(&waypoints, &encode(&points), 48.95, 20.2);

    assert_eq!(inserted.len(), 4);
    assert!(
        (inserted[1].lon - 20.2).abs() < 1e-9,
        "a point on the first leg belongs before the middle waypoint, got {:?}",
        inserted.iter().map(|w| w.lon).collect::<Vec<_>>()
    );
}

#[test]
fn a_point_dragged_from_the_last_leg_lands_in_the_last_slot() {
    let points = line_points();
    let waypoints = vec![wp(48.9, 20.0), wp(48.9, 20.5), wp(48.9, 21.0)];
    let inserted = insert_waypoint(&waypoints, &encode(&points), 48.95, 20.8);

    assert_eq!(inserted.len(), 4);
    assert!((inserted[2].lon - 20.8).abs() < 1e-9);
}

/// A new waypoint is never an endpoint: dragging must not silently change
/// where the journey started or finished.
#[test]
fn insertion_never_displaces_an_endpoint() {
    let points = line_points();
    let waypoints = vec![wp(48.9, 20.0), wp(48.9, 21.0)];
    let inserted = insert_waypoint(&waypoints, &encode(&points), 48.95, 20.01);

    assert!((inserted[0].lon - 20.0).abs() < 1e-9, "origin moved");
    assert!(
        (inserted.last().unwrap().lon - 21.0).abs() < 1e-9,
        "destination moved"
    );
}

/// Undecodable geometry must not lose the point or panic — append before the
/// destination, which is the only slot that is always valid.
#[test]
fn a_broken_polyline_still_places_the_point() {
    let waypoints = vec![wp(48.9, 20.0), wp(48.9, 21.0)];
    let inserted = insert_waypoint(&waypoints, "", 48.95, 20.5);
    assert_eq!(inserted.len(), 3);
    assert!((inserted[1].lon - 20.5).abs() < 1e-9);
}
```

**Step 2: Run to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "insert"
```
Expected: FAIL — `insert_waypoint` not found.

**Step 3: Implement**

```rust
/// Place a dragged-in point into an ordered waypoint list.
///
/// The polyline is the geometry the point was dragged off, so its vertices
/// give the ordering that matters: every existing waypoint lies on the line,
/// so mapping each to its nearest vertex yields the leg boundaries, and the
/// new point's nearest vertex says which leg it came from.
///
/// Comparing squared degrees rather than true distances is deliberate — over a
/// single route's extent the distortion cannot reorder two candidates, and
/// nothing here needs a distance, only an argmin.
///
/// Never returns a list with a new first or last element: a drag must not
/// silently move where the journey began or ended.
pub fn insert_waypoint(
    waypoints: &[Waypoint],
    polyline: &str,
    lat: f64,
    lon: f64,
) -> Vec<Waypoint> {
    let new_point = Waypoint { lat, lon, name: None, node_idx: None };
    let mut out = waypoints.to_vec();

    // Fewer than two waypoints is not a route; appending is the only sane act.
    if out.len() < 2 {
        out.push(new_point);
        return out;
    }

    let points = decode(polyline);
    // No usable geometry: put it immediately before the destination, the one
    // slot that is always valid.
    if points.len() < 2 {
        out.insert(out.len() - 1, new_point);
        return out;
    }

    let nearest = |lat: f64, lon: f64| -> usize {
        points
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let da = (a.0 - lat).powi(2) + (a.1 - lon).powi(2);
                let db = (b.0 - lat).powi(2) + (b.1 - lon).powi(2);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    };

    let new_vertex = nearest(lat, lon);
    // The first slot a new point may take is 1, the last is len()-1.
    let mut slot = out.len() - 1;
    for (i, wp) in out.iter().enumerate().skip(1) {
        if new_vertex <= nearest(wp.lat, wp.lon) {
            slot = i;
            break;
        }
    }
    let slot = slot.clamp(1, out.len() - 1);

    out.insert(slot, new_point);
    out
}
```

**Step 4: Run to verify it passes** — same command. Expected: PASS, 5 tests.

**Step 5: Commit**

```bash
git add src-tauri/core/src/commands_internal/route_maps.rs src-tauri/core/src/commands_internal/route_maps_tests.rs
git commit -m "feat(route-map): place dragged-in waypoints in the right leg"
```

---

## Task 8: `route_direct_internal`

**Files:**
- Modify: [src-tauri/core/src/commands_internal/route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs)
- Modify: [src-tauri/core/src/commands_internal/route_maps_tests.rs](../../src-tauri/core/src/commands_internal/route_maps_tests.rs)

`GeneratedRoute` gains `duration_s` and `mode`, and `dataset_version` becomes
`Option<String>` (a direct route used no dataset). `generate_route_internal` sets
`mode: Loop` and `dataset_version: Some(...)` — otherwise unchanged.

**Step 1: Write the failing tests**

```rust
/// Returns as many alternatives as asked for, each with its own distance —
/// enough to prove per-alternative deviation is computed, not copied.
struct MultiRouteProvider {
    routes: Vec<FetchedRoute>,
}

#[async_trait::async_trait]
impl RouteProvider for MultiRouteProvider {
    async fn fetch(&self, _coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        Ok(self.routes[0].clone())
    }
    async fn fetch_alternatives(
        &self,
        _coords: &[(f64, f64)],
        _max: usize,
    ) -> Result<Vec<FetchedRoute>, String> {
        Ok(self.routes.clone())
    }
}

fn fetched(polyline: &str, road_km: f64, duration_s: f64) -> FetchedRoute {
    FetchedRoute { polyline: polyline.into(), road_km, duration_s }
}

fn direct_waypoints() -> Vec<Waypoint> {
    vec![
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
        Waypoint { lat: 48.9444, lon: 20.5675, name: Some("Spišská Nová Ves".into()), node_idx: None },
    ]
}

#[tokio::test]
async fn direct_routes_are_returned_in_provider_order() {
    let provider = MultiRouteProvider {
        routes: vec![
            fetched(&encode(&[(48.1, 17.1), (48.9, 20.5)]), 400.0, 14000.0),
            fetched(&encode(&[(48.1, 17.1), (49.0, 20.6)]), 380.0, 16000.0),
        ],
    };

    let routes = route_direct_internal(&provider, direct_waypoints(), 420.0, None)
        .await
        .unwrap();

    assert_eq!(routes.len(), 2);
    assert!((routes[0].road_km - 400.0).abs() < 1e-9, "fastest must stay first");
    assert!((routes[1].road_km - 380.0).abs() < 1e-9);
}

/// Each alternative is measured against the row's own distance_km, by the
/// SAME deviation helper loop mode uses — one tolerance, one home.
#[tokio::test]
async fn every_alternative_carries_its_own_deviation() {
    let provider = MultiRouteProvider {
        routes: vec![
            fetched(&encode(&[(48.1, 17.1), (48.9, 20.5)]), 420.0, 14000.0),
            fetched(&encode(&[(48.1, 17.1), (49.0, 20.6)]), 300.0, 16000.0),
        ],
    };

    let routes = route_direct_internal(&provider, direct_waypoints(), 420.0, None)
        .await
        .unwrap();

    assert!(routes[0].deviation_percent.abs() < 1e-6);
    assert!(!routes[0].off_target);
    assert!(routes[1].deviation_percent < -20.0);
    assert!(routes[1].off_target, "a 300 km route for a 420 km row must be flagged");
}

#[tokio::test]
async fn a_direct_route_is_marked_direct_and_claims_no_dataset() {
    let provider = MultiRouteProvider {
        routes: vec![fetched(&encode(&[(48.1, 17.1), (48.9, 20.5)]), 400.0, 14000.0)],
    };
    let routes = route_direct_internal(&provider, direct_waypoints(), 420.0, None)
        .await
        .unwrap();

    assert_eq!(routes[0].mode, RouteMode::Direct);
    assert!(
        routes[0].dataset_version.is_none(),
        "no dataset node was used, so claiming a dataset version would be a lie"
    );
}

/// The insert point is applied BEFORE routing, and the returned waypoints are
/// authoritative — that is what lets the frontend adopt them wholesale.
#[tokio::test]
async fn an_insert_point_is_applied_before_routing() {
    let geometry = encode(&[(48.9, 20.0), (48.9, 20.5), (48.9, 21.0)]);
    let provider = MultiRouteProvider {
        routes: vec![fetched(&geometry, 400.0, 14000.0)],
    };

    let routes = route_direct_internal(
        &provider,
        direct_waypoints(),
        420.0,
        Some(InsertPoint { lat: 48.95, lon: 20.5, polyline: geometry.clone() }),
    )
    .await
    .unwrap();

    assert_eq!(routes[0].waypoints.len(), 3, "the dragged point must be in the result");
    assert!((routes[0].waypoints[1].lat - 48.95).abs() < 1e-9);
}

#[tokio::test]
async fn a_route_needs_at_least_two_waypoints() {
    let provider = MultiRouteProvider {
        routes: vec![fetched("aaa", 1.0, 1.0)],
    };
    assert!(
        route_direct_internal(&provider, vec![direct_waypoints()[0].clone()], 10.0, None)
            .await
            .is_err()
    );
}
```

Add `#[derive(Clone)]` to `FetchedRoute` in `osrm.rs` so these stubs can clone it.

**Step 2: Run to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "direct"
```
Expected: FAIL — `route_direct_internal` not found.

**Step 3: Implement**

Update `GeneratedRoute`:

```rust
pub struct GeneratedRoute {
    pub waypoints: Vec<Waypoint>,
    pub polyline: String,
    pub coordinates: Vec<[f64; 2]>,
    pub target_km: f64,
    pub road_km: f64,
    /// Estimated driving time in seconds. Shown while choosing; not persisted.
    pub duration_s: f64,
    pub deviation_percent: f64,
    pub off_target: bool,
    /// `None` for direct routes — no dataset node was involved.
    pub dataset_version: Option<String>,
    pub mode: RouteMode,
}
```

Set `mode: RouteMode::Loop`, `duration_s: fetched.duration_s` and
`dataset_version: Some(ds.version)` in `generate_route_internal`.

Then:

```rust
/// How many routes to offer. Three is what a navigation app shows and what
/// fits a panel; more is noise nobody reads.
const MAX_ALTERNATIVES: usize = 3;

/// A point the user dragged off `polyline`, to be placed into the waypoint
/// list before routing.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertPoint {
    pub lat: f64,
    pub lon: f64,
    /// The geometry it was dragged from — the ordering that decides which leg
    /// it belongs to.
    pub polyline: String,
}

/// Route an ordered waypoint list, offering alternatives where the routing
/// service can produce them.
///
/// Persists NOTHING — like `generate_route_internal`, the caller confirms with
/// `save_trip_route_internal`.
///
/// The returned `waypoints` are AUTHORITATIVE: when `insert` is present they
/// already include the new point in its computed slot, so the frontend adopts
/// the list rather than maintaining its own ordering.
pub async fn route_direct_internal(
    provider: &dyn RouteProvider,
    waypoints: Vec<Waypoint>,
    target_km: f64,
    insert: Option<InsertPoint>,
) -> Result<Vec<GeneratedRoute>, String> {
    let waypoints = match insert {
        Some(p) => insert_waypoint(&waypoints, &p.polyline, p.lat, p.lon),
        None => waypoints,
    };

    if waypoints.len() < 2 {
        return Err(format!(
            "A route needs a start and an end, got {} point(s).",
            waypoints.len()
        ));
    }

    let coords: Vec<(f64, f64)> = waypoints.iter().map(|w| (w.lat, w.lon)).collect();
    let fetched = provider
        .fetch_alternatives(&coords, MAX_ALTERNATIVES)
        .await?;

    Ok(fetched
        .into_iter()
        .map(|route| {
            // The same deviation helper loop mode uses. A second, separately
            // measured notion of "close enough" is exactly what ADR-008 rules
            // out.
            let (deviation_percent, off_target) = deviation(target_km, route.road_km);
            GeneratedRoute {
                coordinates: decode_coordinates(&route.polyline),
                polyline: route.polyline,
                waypoints: waypoints.clone(),
                target_km,
                road_km: route.road_km,
                duration_s: route.duration_s,
                deviation_percent,
                off_target,
                dataset_version: None,
                mode: RouteMode::Direct,
            }
        })
        .collect())
}
```

**Step 4: Run to verify it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core route_map
```
Expected: PASS.

**Step 5: Commit**

```bash
git add src-tauri/core/src/route_map/osrm.rs src-tauri/core/src/commands_internal/route_maps.rs src-tauri/core/src/commands_internal/route_maps_tests.rs
git commit -m "feat(route-map): add direct A-to-B routing with alternatives"
```

---

## Task 9: Persist the mode

**Files:**
- Create: [2026-09-03-110000_add_trip_route_mode/up.sql](../../src-tauri/core/migrations/2026-09-03-110000_add_trip_route_mode/up.sql)
- Create: [2026-09-03-110000_add_trip_route_mode/down.sql](../../src-tauri/core/migrations/2026-09-03-110000_add_trip_route_mode/down.sql)
- Modify: [schema.rs](../../src-tauri/core/src/schema.rs) · [models.rs](../../src-tauri/core/src/models.rs) · [db.rs](../../src-tauri/core/src/db.rs)
- Modify: [route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs) · [route_maps_tests.rs](../../src-tauri/core/src/commands_internal/route_maps_tests.rs)
- Modify: [migration_tests.rs](../../src-tauri/core/src/migration_tests.rs)

**Step 1: Write the failing tests**

In `route_maps_tests.rs`:

```rust
#[test]
fn a_saved_direct_route_round_trips_with_its_mode_and_vias() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);

    let waypoints = vec![
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
        Waypoint { lat: 48.7, lon: 19.1, name: None, node_idx: None },
        Waypoint { lat: 48.9444, lon: 20.5675, name: Some("Spišská".into()), node_idx: None },
    ];

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        waypoints.clone(),
        encode(&[(48.1, 17.1), (48.9, 20.5)]),
        420.0,
        400.0,
        RouteMode::Direct,
    )
    .unwrap();

    let loaded = get_trip_route_internal(&db, trip.id.to_string())
        .unwrap()
        .unwrap();
    assert_eq!(loaded.mode, RouteMode::Direct);
    assert_eq!(loaded.waypoints.len(), 3);
    assert!(
        loaded.dataset_version.is_none(),
        "a direct route must not claim a dataset version"
    );
}

#[test]
fn a_saved_loop_route_still_stamps_the_dataset_version() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        sample_waypoints(),
        encode(&[(48.9, 20.5), (49.0, 20.6)]),
        120.0,
        118.0,
        RouteMode::Loop,
    )
    .unwrap();

    let loaded = get_trip_route_internal(&db, trip.id.to_string())
        .unwrap()
        .unwrap();
    assert_eq!(loaded.mode, RouteMode::Loop);
    assert!(loaded.dataset_version.is_some());
}
```

In `migration_tests.rs` — the backfill guarantee:

```rust
/// Every route saved by Task 70 IS a loop, so the DEFAULT backfills correctly
/// by construction. This test is what proves that claim rather than assuming it.
#[test]
fn existing_route_maps_become_loop_mode() {
    let db = legacy_db_before("2026-09-03-110000_add_trip_route_mode");
    seed_vehicle(&db, VEHICLE_ID);
    seed_trip(&db, TRIP_ID, VEHICLE_ID, None);
    exec(
        &db,
        &format!(
            "INSERT INTO trip_routes (trip_id, waypoints, polyline, target_km, road_km, \
                                      dataset_version, created_at) \
             VALUES ('{TRIP_ID}', '[]', 'abc', 100.0, 98.0, '2026-05-03', \
                     '2026-01-01T00:00:00+00:00')"
        ),
    );

    run_remaining_migrations(&db);

    let map = db.get_route_map(TRIP_ID).unwrap().unwrap();
    assert_eq!(map.mode, RouteMode::Loop);
}
```

> Match the existing helper names in `migration_tests.rs` — it already has a way to open
> a DB at a given migration and run the rest. Reuse it; do not add a second mechanism.

**Step 2: Run to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "mode"
```
Expected: FAIL — `save_trip_route_internal` takes 7 arguments, not 8.

**Step 3: Implement**

`up.sql`:

```sql
-- Task 72: which producer built a route map.
-- DEFAULT 'loop' backfills correctly by construction: every route saved before
-- this migration came from the Task 70 genetic-algorithm loop.
ALTER TABLE trip_routes ADD COLUMN mode TEXT NOT NULL DEFAULT 'loop';
```

`down.sql`:

```sql
-- Forward-only in practice (ADR-012); no diesel CLI revert runs in this repo.
ALTER TABLE trip_routes DROP COLUMN mode;
```

Add `mode -> Text` to the `trip_routes` block in `schema.rs`; add `pub mode: RouteMode`
to `RouteMap`, `pub mode: String` to `RouteMapRow`, `pub mode: &'a str` to
`NewRouteMapRow`, and parse it in `From<RouteMapRow>` (`"direct" => Direct, _ => Loop` —
an unrecognised value reads as the V1 default, which is always the safe reading). Pass
`map.mode.as_str()` in `save_route_map`.

Add `pub mode: RouteMode` to `SavedRouteMap` and carry it through
`impl From<RouteMap> for SavedRouteMap`. Give `save_trip_route_internal` a trailing
`mode: RouteMode` parameter and stamp `dataset_version` from it:

```rust
        dataset_version: match mode {
            // Only a loop actually used the bundled node set.
            RouteMode::Loop => Some(Dataset::bundled().version),
            RouteMode::Direct => None,
        },
```

**Step 4: Run to verify it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```
Expected: PASS, whole backend suite.

**Step 5: Commit**

```bash
git add src-tauri/core/migrations/2026-09-03-110000_add_trip_route_mode/ src-tauri/core/src/schema.rs src-tauri/core/src/models.rs src-tauri/core/src/db.rs src-tauri/core/src/commands_internal/route_maps.rs src-tauri/core/src/commands_internal/route_maps_tests.rs src-tauri/core/src/migration_tests.rs
git commit -m "feat(route-map): persist which producer built each route map"
```

---

## Task 10: Dispatcher wiring

**Files:**
- Modify: [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs)
- Modify: [src-tauri/core/src/server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs)

**Step 1: Write the failing tests**

In `dispatcher_async.rs`'s test module, following
`generate_route_is_an_async_command_taking_target_km`:

```rust
/// Both new async commands must be routed HERE (they await the network) and
/// must take the argument names src/lib/api.ts sends. Bad args fail during
/// parsing, so this pins the names without touching the network.
#[tokio::test]
async fn resolve_place_and_route_direct_are_async_commands() {
    let state = ServerState {
        db: std::sync::Arc::new(crate::db::Database::in_memory().unwrap()),
        app_state: std::sync::Arc::new(crate::app_state::AppState::new()),
        app_dir: std::env::temp_dir(),
        static_dir: std::env::temp_dir(),
    };

    let err = dispatch_async("resolve_place", json!({}), &state)
        .await
        .expect("resolve_place must be handled here")
        .unwrap_err();
    assert!(err.contains("query"), "got: {err}");

    let err = dispatch_async("route_direct", json!({}), &state)
        .await
        .expect("route_direct must be handled here")
        .unwrap_err();
    assert!(err.contains("waypoints"), "got: {err}");
}
```

In `dispatcher.rs`'s test module, extending the existing
`route_map_commands_round_trip_with_frontend_argument_names` pattern:

```rust
#[test]
fn remember_place_round_trips_with_frontend_argument_names() {
    let state = /* same ServerState construction the neighbouring test uses */;

    dispatch_sync(
        "remember_place",
        json!({
            "query": "Spišská",
            "place": { "lat": 48.9444, "lon": 20.5675, "displayName": "Spišská Nová Ves" },
            "source": "geocoder"
        }),
        &state,
    )
    .unwrap();

    assert!(state.db.get_place_alias("spisska").unwrap().is_some());
}
```

**Step 2: Run to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core dispatch
```
Expected: FAIL — "Unknown command: resolve_place".

**Step 3: Implement**

In `dispatcher_async.rs`, in the route-maps section:

```rust
        "resolve_place" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                query: String,
            }
            let a: Args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            let provider = crate::route_map::HttpGeocodeProvider::public();
            let result =
                crate::commands_internal::resolve_place_internal(&state.db, &provider, a.query)
                    .await;
            Some(result.map(|v| serde_json::to_value(v).unwrap()))
        }
        "route_direct" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                waypoints: Vec<crate::models::Waypoint>,
                target_km: f64,
                #[serde(default)]
                insert: Option<crate::commands_internal::InsertPoint>,
            }
            let a: Args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            let provider = crate::route_map::HttpRouteProvider::public();
            let result = crate::commands_internal::route_direct_internal(
                &provider,
                a.waypoints,
                a.target_km,
                a.insert,
            )
            .await;
            Some(result.map(|v| serde_json::to_value(v).unwrap()))
        }
```

Update the section comment above `generate_route` — it says "Route maps — async (1)".

In `dispatcher.rs`, in the route-maps section:

```rust
        "remember_place" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                query: String,
                place: crate::route_map::Place,
                source: crate::models::AliasSource,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::remember_place_internal(
                &state.db,
                &state.app_state,
                a.query,
                a.place,
                a.source,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
```

and add `mode` to the `save_trip_route` `Args` struct, passing it through:

```rust
                mode: crate::models::RouteMode,
```

Update both section comments ("sync (3)" → "sync (4)").

**Step 4: Run to verify it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```
Expected: PASS, whole backend suite.

**Step 5: Commit**

```bash
git add src-tauri/core/src/server/dispatcher.rs src-tauri/core/src/server/dispatcher_async.rs
git commit -m "feat(route-map): dispatch the geocoding and direct-routing commands"
```

---

# Phase 3 — Frontend

## Task 11: Types and API wrappers

**Files:**
- Modify: [src/lib/types.ts](../../src/lib/types.ts)
- Modify: [src/lib/api.ts](../../src/lib/api.ts)

**Step 1: Implement** (no test step — these are typed pass-throughs with no logic; the
dispatcher tests above already pin every argument name, and Task 16's integration specs
exercise the wrappers end to end)

In `types.ts`:

```ts
export type RouteMode = 'loop' | 'direct';
export type AliasSource = 'geocoder' | 'manual';

/** A geocoded place. */
export interface Place {
	lat: number;
	lon: number;
	displayName: string;
}

/**
 * Where a place name resolved to. `resolved` is set only on a cache hit — a
 * place a human already confirmed. Otherwise `candidates` holds what the
 * geocoder offered, and an EMPTY list means "place it by hand", not an error.
 */
export interface PlaceResolution {
	query: string;
	resolved: Place | null;
	candidates: Place[];
}

/** A point dragged off `polyline`, for the backend to slot into the list. */
export interface InsertPoint {
	lat: number;
	lon: number;
	polyline: string;
}
```

Add to `GeneratedRoute` and `RouteMap`: `durationS: number;` and `mode: RouteMode;`, and
widen `GeneratedRoute.datasetVersion` to `string | null`.

In `api.ts`:

```ts
export async function resolvePlace(query: string): Promise<PlaceResolution> {
	return await apiCall('resolve_place', { query });
}

export async function rememberPlace(
	query: string,
	place: Place,
	source: AliasSource
): Promise<void> {
	return await apiCall('remember_place', { query, place, source });
}

/**
 * Route an ordered waypoint list. Pass `insert` to have the backend slot a
 * dragged-in point into the list first — the returned routes' `waypoints` are
 * authoritative and should be adopted as-is.
 *
 * Returns alternatives in the routing service's own order (fastest first).
 * Never re-sort them: that ordering is the product decision.
 */
export async function routeDirect(
	waypoints: Waypoint[],
	targetKm: number,
	insert?: InsertPoint
): Promise<GeneratedRoute[]> {
	return await apiCall('route_direct', { waypoints, targetKm, insert: insert ?? null });
}
```

and add `mode: route.mode` to the `saveTripRoute` payload.

**Step 2: Verify it typechecks**

```bash
npm run check
```
Expected: no errors from `api.ts` or `types.ts`. (i18n key errors are expected until
Task 12 — ignore those for now.)

**Step 3: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts
git commit -m "feat(route-map): add frontend types and API wrappers for V2"
```

---

## Task 12: i18n strings

**Files:**
- Modify: [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts) (source of truth)
- Modify: [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts)
- Regenerated: [src/lib/i18n/i18n-types.ts](../../src/lib/i18n/i18n-types.ts)

**Step 1: Implement**

Add to the existing `routeMap:` block in `sk/index.ts`:

```ts
		recalculate: 'Prepočítať',
		alternatives: 'Alternatívne trasy',
		alternativesUnavailable: 'Alternatívy nie sú dostupné, keď má trasa medzizastávky.',
		duration: 'Čas jazdy',
		pickPlace: 'Ktoré miesto to je?',
		pickPlaceHint: 'Vyberte správne miesto. Zapamätáme si ho pre ďalšie jazdy.',
		placeNotFound: 'Miesto sa nenašlo. Kliknite na mapu a označte ho.',
		placeRemembered: 'Miesto uložené',
		geocodeError: 'Miesto sa nepodarilo vyhľadať',
		routeError: 'Trasu sa nepodarilo vypočítať',
		missingEndpoints: 'Jazda nemá vyplnené miesto odchodu alebo príchodu.',
		editHint: 'Potiahnutím čiary pridáte medzizastávku. Kliknutím na zastávku ju odstránite.',
		removeWaypoint: 'Odstrániť zastávku',
```

and the matching English:

```ts
		recalculate: 'Recalculate',
		alternatives: 'Alternative routes',
		alternativesUnavailable: 'Alternatives are unavailable once the route has stops.',
		duration: 'Driving time',
		pickPlace: 'Which place is this?',
		pickPlaceHint: 'Pick the right place. We will remember it for future trips.',
		placeNotFound: 'Place not found. Click the map to mark it.',
		placeRemembered: 'Place saved',
		geocodeError: 'Could not look up the place',
		routeError: 'Could not calculate the route',
		missingEndpoints: 'This trip has no origin or destination filled in.',
		editHint: 'Drag the line to add a stop. Click a stop to remove it.',
		removeWaypoint: 'Remove stop',
```

**Step 2: Regenerate the types — REQUIRED**

Nothing else regenerates `i18n-types.ts`; the generator otherwise runs only in vite dev
watch mode, so `npm run check` reports phantom errors for keys that do exist.

```bash
npm run i18n
npm run check
```
Expected: no i18n errors.

**Step 3: Commit**

```bash
git add src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts src/lib/i18n/i18n-types.ts
git commit -m "feat(route-map): add i18n strings for V2 map view"
```

---

## Task 13: Map view — mode branch and place picker

**Files:**
- Modify: [src/routes/mapa/+page.svelte](../../src/routes/mapa/+page.svelte)

Today `loadTripAndRoute` unconditionally calls `runGenerate(trip.distanceKm)`. That
becomes a branch.

**Step 1: Implement**

Add state and helpers to the `<script>` block:

```ts
	let mode = $state<RouteMode | null>(null);
	/** Alternatives for the current direct route, in the backend's order. */
	let alternatives = $state<GeneratedRoute[]>([]);
	let activeIndex = $state(0);
	/** The endpoint currently awaiting a pick, if any. */
	let pendingPlace = $state<{ field: 'origin' | 'destination'; resolution: PlaceResolution } | null>(null);
	let resolvedOrigin = $state<Place | null>(null);
	let resolvedDestination = $state<Place | null>(null);
```

Replace the tail of `loadTripAndRoute` with:

```ts
			savedRoute = await getTripRoute(tripId);
			if (savedRoute) {
				mode = savedRoute.mode;
				return;
			}
			await startForTrip(trip);
```

and add:

```ts
	/** Loop or direct — the backend decides, from the row's own endpoints. */
	async function startForTrip(t: Trip) {
		const origin = t.origin?.trim() ?? '';
		const destination = t.destination?.trim() ?? '';
		if (!origin || !destination) {
			error = $LL.routeMap.missingEndpoints();
			retryable = false;
			return;
		}

		if (normaliseForCompare(origin) === normaliseForCompare(destination)) {
			mode = 'loop';
			await runGenerate(t.distanceKm);
			return;
		}

		mode = 'direct';
		await resolveEndpoints(t);
	}

	/**
	 * Display-only echo of the backend's mode rule, so the page can pick which
	 * flow to start without a round trip. The backend's `mode_for` remains the
	 * authority — this only decides which request to make first, and a
	 * disagreement costs a redundant lookup, never a wrong saved route.
	 */
	function normaliseForCompare(s: string): string {
		return s
			.toLowerCase()
			.normalize('NFD')
			.replace(/[̀-ͯ]/g, '')
			.split(/\s+/)
			.filter(Boolean)
			.join(' ');
	}

	async function resolveEndpoints(t: Trip) {
		generating = true;
		error = null;
		try {
			const origin = await resolvePlace(t.origin);
			if (!origin.resolved) {
				pendingPlace = { field: 'origin', resolution: origin };
				return;
			}
			resolvedOrigin = origin.resolved;

			const destination = await resolvePlace(t.destination);
			if (!destination.resolved) {
				pendingPlace = { field: 'destination', resolution: destination };
				return;
			}
			resolvedDestination = destination.resolved;

			await runDirect(waypointsFromEndpoints(), t.distanceKm);
		} catch (e) {
			console.error('Failed to resolve trip endpoints:', e);
			error = $LL.routeMap.geocodeError();
		} finally {
			generating = false;
		}
	}

	function waypointsFromEndpoints(): Waypoint[] {
		if (!resolvedOrigin || !resolvedDestination) return [];
		return [
			{ lat: resolvedOrigin.lat, lon: resolvedOrigin.lon, name: resolvedOrigin.displayName },
			{
				lat: resolvedDestination.lat,
				lon: resolvedDestination.lon,
				name: resolvedDestination.displayName
			}
		];
	}

	/** The user picked a candidate (or dropped a pin). Remember it, then carry on. */
	async function handlePlacePicked(place: Place, source: AliasSource) {
		if (!pendingPlace || !trip) return;
		const { field, resolution } = pendingPlace;
		try {
			await rememberPlace(resolution.query, place, source);
		} catch (e) {
			// Remembering is a convenience; failing it must not block the route.
			console.error('Failed to remember place:', e);
		}
		if (field === 'origin') resolvedOrigin = place;
		else resolvedDestination = place;
		pendingPlace = null;
		await resolveEndpoints(trip);
	}

	/** Routes and displays. Persists nothing — only handleSave does. */
	async function runDirect(waypoints: Waypoint[], targetKm: number, insert?: InsertPoint) {
		generating = true;
		error = null;
		savedNotice = false;
		try {
			const routes = await routeDirect(waypoints, targetKm, insert);
			if (routes.length === 0) throw new Error('no routes returned');
			alternatives = routes;
			activeIndex = 0;
			generated = routes[0];
		} catch (e) {
			console.error('Failed to route trip:', e);
			// Same rule as V1: drop the proposal so an error banner can never
			// have a stale, saveable route sitting behind it.
			generated = null;
			alternatives = [];
			error = $LL.routeMap.routeError();
		} finally {
			generating = false;
		}
	}
```

Make `handleRegenerate` and `handleRetry` mode-aware: loop mode calls `runGenerate`,
direct mode calls `runDirect(generated?.waypoints ?? waypointsFromEndpoints(), trip.distanceKm)`.

Pass the mode when saving:

```ts
			await saveTripRoute(tripId, generated);   // generated.mode carries it
```

Toolbar: show **Generovať znova** only when `mode === 'loop'`, and **Prepočítať**
(calling `runDirect`) only when `mode === 'direct'`.

Add the picker markup above the map canvas:

```svelte
		{#if pendingPlace}
			<div class="place-picker" data-test="place-picker">
				<p class="picker-title">
					{$LL.routeMap.pickPlace()} <strong>{pendingPlace.resolution.query}</strong>
				</p>
				{#if pendingPlace.resolution.candidates.length > 0}
					<p class="picker-hint">{$LL.routeMap.pickPlaceHint()}</p>
					<ul class="candidates">
						{#each pendingPlace.resolution.candidates as candidate}
							<li>
								<button
									class="candidate"
									data-test="place-candidate"
									onclick={() => handlePlacePicked(candidate, 'geocoder')}
								>
									{candidate.displayName}
								</button>
							</li>
						{/each}
					</ul>
				{:else}
					<p class="picker-hint" data-test="place-not-found">
						{$LL.routeMap.placeNotFound()}
					</p>
				{/if}
			</div>
		{/if}
```

When the picker is showing with no candidates, a map click resolves the endpoint. Add to
the map-creation `$effect`:

```ts
		map.on('click', (e: LeafletMouseEvent) => {
			if (!pendingPlace) return;
			void handlePlacePicked(
				{ lat: e.latlng.lat, lon: e.latlng.lng, displayName: pendingPlace.resolution.query },
				'manual'
			);
		});
```

**Step 2: Verify**

```bash
npm run check
```
Expected: no errors.

**Step 3: Commit**

```bash
git add src/routes/mapa/+page.svelte
git commit -m "feat(route-map): route A-to-B from the row's geocoded endpoints"
```

---

## Task 14: Map view — alternatives panel

**Files:**
- Modify: [src/routes/mapa/+page.svelte](../../src/routes/mapa/+page.svelte)

**Step 1: Implement**

Draw inactive alternatives behind the active line. Extend the existing draw `$effect` —
keep a `Polyline[]` of inactive layers alongside `routeLayer`, clear them the same way,
and add:

```ts
		// Inactive alternatives sit UNDER the active line and are clickable.
		alternatives.forEach((route, i) => {
			if (i === activeIndex || route.coordinates.length === 0) return;
			const layer = leaflet!
				.polyline(route.coordinates, { color: '#94a3b8', weight: 4, opacity: 0.6 })
				.addTo(map!);
			layer.on('click', () => selectAlternative(i));
			inactiveLayers.push(layer);
		});
```

```ts
	function selectAlternative(index: number) {
		activeIndex = index;
		generated = alternatives[index];
	}

	function formatDuration(seconds: number): string {
		const total = Math.round(seconds / 60);
		const h = Math.floor(total / 60);
		const m = total % 60;
		return h > 0 ? `${h} h ${m} min` : `${m} min`;
	}
```

Panel markup, after the existing `.route-info` block:

```svelte
		{#if mode === 'direct' && alternatives.length > 0}
			<div class="alternatives" data-test="alternatives">
				<span class="label">{$LL.routeMap.alternatives()}</span>
				<ul>
					{#each alternatives as route, i}
						<li>
							<button
								class="alternative"
								class:active={i === activeIndex}
								data-test="alternative"
								onclick={() => selectAlternative(i)}
							>
								<span>{route.roadKm.toFixed(1)} km</span>
								<span>{formatDuration(route.durationS)}</span>
								<span class:off-target={route.offTarget}>
									{formatDeviation(route.deviationPercent)}
								</span>
							</button>
						</li>
					{/each}
				</ul>
			</div>
		{:else if mode === 'direct' && hasWaypoints}
			<p class="hint" data-test="alternatives-unavailable">
				{$LL.routeMap.alternativesUnavailable()}
			</p>
		{/if}
```

with `let hasWaypoints = $derived((generated?.waypoints.length ?? 0) > 2);`.

**Do not sort `alternatives`.** The backend hands them over in the routing service's
fastest-first order and that order is the product decision — a `.sort()` here silently
reverses it.

**Step 2: Verify**

```bash
npm run check
```

**Step 3: Commit**

```bash
git add src/routes/mapa/+page.svelte
git commit -m "feat(route-map): show route alternatives fastest-first"
```

---

## Task 15: Map view — drag editing

**Files:**
- Modify: [src/routes/mapa/+page.svelte](../../src/routes/mapa/+page.svelte)

Two gestures, one rule: **nothing is requested while the pointer is down.**

**Step 1: Implement**

```ts
	let waypointMarkers: Marker[] = [];
	let ghost: Marker | null = null;

	/** Small circular handle. Endpoints are visually heavier than vias. */
	function handleIcon(L: typeof import('leaflet'), endpoint: boolean) {
		return L.divIcon({
			className: endpoint ? 'wp-handle wp-endpoint' : 'wp-handle',
			iconSize: [endpoint ? 14 : 10, endpoint ? 14 : 10]
		});
	}

	function currentWaypoints(): Waypoint[] {
		return generated?.waypoints ?? savedRoute?.waypoints ?? [];
	}

	function drawHandles() {
		if (!map || !leaflet) return;
		waypointMarkers.forEach((m) => map!.removeLayer(m));
		waypointMarkers = [];

		const points = currentWaypoints();
		points.forEach((wp, i) => {
			const endpoint = i === 0 || i === points.length - 1;
			const marker = leaflet!
				.marker([wp.lat, wp.lon], {
					draggable: true,
					icon: handleIcon(leaflet!, endpoint)
				})
				.addTo(map!);

			// ONE request, on release. Never during the drag: the routing
			// service is capped at a request a second, and mid-drag routing
			// would spend that budget on frames nobody sees.
			marker.on('dragend', () => {
				const { lat, lng } = marker.getLatLng();
				const next = points.map((p, j) => (j === i ? { ...p, lat, lon: lng } : p));
				void reroute(next);
			});

			// Clicking a via removes it. Endpoints are not removable — that
			// would change where the journey started or ended.
			if (!endpoint) {
				marker.on('click', () => {
					void reroute(points.filter((_, j) => j !== i));
				});
			}

			waypointMarkers.push(marker);
		});
	}

	/**
	 * Ghost handle: appears on the active line under the cursor, and dragging
	 * it off creates a new waypoint. Where that waypoint LANDS in the ordered
	 * list is decided by the backend — see 02-design.md.
	 */
	function attachGhost(layer: Polyline) {
		if (!map || !leaflet) return;
		layer.on('mousemove', (e: LeafletMouseEvent) => {
			if (!ghost) {
				ghost = leaflet!
					.marker(e.latlng, { draggable: true, icon: handleIcon(leaflet!, false) })
					.addTo(map!);
				ghost.on('dragend', () => {
					const { lat, lng } = ghost!.getLatLng();
					const polyline = generated?.polyline ?? savedRoute?.polyline ?? '';
					map!.removeLayer(ghost!);
					ghost = null;
					void reroute(currentWaypoints(), { lat, lon: lng, polyline });
				});
			} else {
				ghost.setLatLng(e.latlng);
			}
		});
	}

	/**
	 * Re-route through an edited waypoint list. Works in BOTH modes: a route
	 * is an ordered waypoint list either way, which is what lets a
	 * mis-anchored loop be dragged into shape.
	 */
	async function reroute(waypoints: Waypoint[], insert?: InsertPoint) {
		if (!trip) return;
		// Editing produces a concrete road route, so an edited loop becomes a
		// direct route — which is exactly the escape hatch the design wants.
		mode = 'direct';
		await runDirect(waypoints, trip.distanceKm, insert);
	}
```

Call `drawHandles()` at the end of the draw `$effect`, and `attachGhost(routeLayer)`
right after the active layer is added. Clear `waypointMarkers` and `ghost` in
`onDestroy` alongside `routeLayer`.

Add the handle styles and the edit hint (`{$LL.routeMap.editHint()}`) beside the
existing `.stops` line.

**Step 2: Verify**

```bash
npm run check
```

**Step 3: Commit**

```bash
git add src/routes/mapa/+page.svelte
git commit -m "feat(route-map): drag the line to edit a route, re-routing on drop"
```

---

# Phase 4 — Verification and documentation

## Task 16: Integration tests

**Files:**
- Modify: [tests/integration/specs/tier2/route-map.spec.ts](../../tests/integration/specs/tier2/route-map.spec.ts)

Read the existing spec first and follow its setup helpers exactly. These cover UI flows
only — the routing and deviation math is already proven by backend tests and must not be
re-asserted here.

**Step 1: Write the failing tests**

1. **An A→B row draws a route.** Seed a trip with different origin/destination, open
   `/mapa?trip={id}`, assert `[data-test="route-map-canvas"] path` exists and
   `[data-test="deviation"]` shows a value.
2. **The alias is remembered.** With an ambiguous origin, assert
   `[data-test="place-picker"]` appears; click the first
   `[data-test="place-candidate"]`; reload the page and assert the picker does **not**
   appear. This is the alias table's entire value, so it is worth asserting twice.
3. **An alternative can be promoted.** Assert more than one
   `[data-test="alternative"]`, click the second, assert it gains `.active` and
   `[data-test="actual-km"]` changes.
4. **A missing endpoint is reported.** Seed a trip with an empty destination, open the
   map, assert `[data-test="route-map-error"]` is shown and no route is drawn.
5. **A same-place row still loops.** Seed origin == destination, assert
   `[data-test="regenerate-btn"]` is displayed — the V1 flow is intact.

**Step 2: Run to verify they fail**

```bash
npm run tauri build -- --debug
npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/route-map.spec.ts
```

**Step 3: Fix whatever they catch, then re-run the same single spec** until green. Do
not run the full suite while iterating — a sweep is ~10 minutes, one spec is under a
minute.

**Step 4: Commit**

```bash
git add tests/integration/specs/tier2/route-map.spec.ts
git commit -m "test(route-map): cover origin/destination routing UI flows"
```

---

## Task 17: Full verification

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
npm run check
npm run test:integration
```

All three green before Task 18. Then run `/verify`.

If the integration suite fails somewhere unrelated to this feature, say so explicitly
rather than quietly excluding the spec.

---

## Task 18: Documentation

**Files:**
- Modify: [DECISIONS.md](../../DECISIONS.md) — via `/decision`, one entry each:
  - place names resolve through a cached alias table keyed on normalised text, confirmed
    once by a human;
  - alternatives are ordered by duration; deviation labels them but never reorders them;
  - the recorded `distance_km` is never rewritten from a route's road distance;
  - the waypoint editor is mode-agnostic, which is what makes a mis-anchored loop
    recoverable.
- Modify: [CHANGELOG.md](../../CHANGELOG.md) — via `/changelog`, under `[Unreleased]`.
  User-visible: maps now follow the trip's own origin and destination, propose
  alternatives, and can be edited by dragging.
- Modify: [docs/features/route-maps.md](../../docs/features/route-maps.md) — this is the
  big one. Replace the "Why loops from a home base, and no origin/destination honouring?"
  entry (it now describes shipped behaviour, not a deferral), document both modes, the
  alias table, alternatives and editing, and add the new files to the Key Files table.
  Keep the export section as-is — nothing there changed.
- Modify: [_tasks/index.md](../index.md) — status 📋 → ✅, move the row to Completed
  Tasks, repoint the link at `_done/72-route-map-origin-destination/`.
- Move the task folder to `_tasks/_done/`.

```bash
git add DECISIONS.md CHANGELOG.md docs/features/route-maps.md _tasks/index.md _tasks/_done/72-route-map-origin-destination/
git commit -m "docs(route-map): document origin/destination routing and editing"
```

---

## Deferred to a later task

Recorded so nobody implements them by accident:

- **Re-anchoring the genetic algorithm** at an arbitrary geocoded point, so a distant
  A–A row ("Bratislava – Bratislava") loops around the right town. Needs a distance
  matrix the app does not have. The mode-agnostic editor is the interim answer.
- **Desktop UI** — still no Tauri wrappers for any route-map command, and `routeMaps`
  stays `false` in `defaultDesktop`. Enabling it means adding wrappers for all
  **seven** commands now, not four.
- **A Settings screen for the place-alias book** — reviewing, editing and clearing
  remembered places outside the map view.
- **Re-using a curated route across trips on the same origin/destination pair.**
  Deliberately rejected for now
  ([01-task.md](./01-task.md#non-goals)); revisit only if curating repeats proves
  tedious in practice.
