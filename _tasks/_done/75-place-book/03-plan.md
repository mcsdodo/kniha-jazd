# Place Book Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Give every place string in the logbook a coordinate, confirmed once by a human, from a Miesta section in Settings.

**Architecture:** A `places` table stores coordinates keyed on a normalised name. The *list* of places is derived from `trips` at read time and never stored ([ADR-033](../../DECISIONS.md)). Geocoding sits behind an injected trait so no test touches the network, with a mock-directory mode mirroring the one [gemini.rs](../../src-tauri/core/src/gemini.rs) already uses. The frontend draws the list, confirms points, and gains nothing that decides anything ([ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication)).

**Tech Stack:** Rust (Diesel, reqwest, wiremock), SvelteKit 5 runes, Leaflet, WebdriverIO.

Requirements: [01-task.md](./01-task.md). Design and its reasoning: [02-design.md](./02-design.md). Decisions: ADR-032 … ADR-035 in [DECISIONS.md](../../DECISIONS.md).

---

## Ground rules for every task

- **Test first, always.** Write the failing test, watch it fail for the right reason, then implement. A test that passes before the implementation exists is testing nothing.
- **Backend tests own the logic; integration tests own the flow.** Never assert a calculation twice ([CLAUDE.md](../../CLAUDE.md)).
- **No test touches the network.** `GeocodeProvider` is injected; HTTP parsing is tested against `wiremock`, as [dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs) already does.
- **Commit after each task**, staging only the files that task names. Never `git add -A`.
- Run backend tests with `cargo test --manifest-path src-tauri/Cargo.toml --workspace`.

---

# Phase 1 — Backend

## Task 1: `normalise()`

The lookup key for every place. One implementation, in Rust, shared later with [task 72](../72-route-map-origin-destination/).

**Files:**
- Create: `src-tauri/core/src/places/mod.rs`
- Create: `src-tauri/core/src/places/normalise.rs`
- Create: `src-tauri/core/src/places/normalise_tests.rs`
- Modify: `src-tauri/core/src/lib.rs` (add `pub mod places;` after `pub mod paperless;`)

**Step 1: Write the failing tests**

`src-tauri/core/src/places/normalise_tests.rs`:

```rust
use super::normalise::normalise;

#[test]
fn folds_case() {
    assert_eq!(normalise("KOSICE"), "kosice");
}

#[test]
fn folds_slovak_diacritics() {
    assert_eq!(normalise("Spišská Nová Ves"), "spisska nova ves");
    assert_eq!(normalise("Ľubovňa"), "lubovna");
}

#[test]
fn folds_czech_and_hungarian_diacritics() {
    // The book is not country-restricted (ADR-035), so these must fold too.
    assert_eq!(normalise("Řež"), "rez");
    assert_eq!(normalise("Fót"), "fot");
    assert_eq!(normalise("Győr"), "gyor");
}

#[test]
fn collapses_whitespace() {
    assert_eq!(normalise("  Nova   Ves \t"), "nova ves");
}

#[test]
fn keeps_punctuation_that_distinguishes_addresses() {
    // "Street 1, Town" and "Street 11, Town" must not collide.
    assert_eq!(normalise("Hlavna 1, Mesto"), "hlavna 1, mesto");
}

#[test]
fn empty_input_is_empty_output() {
    assert_eq!(normalise("   "), "");
}

#[test]
fn a_letter_the_table_does_not_know_keeps_its_accent() {
    // The table is closed by design: unknown letters are left alone rather
    // than silently stripped, at the cost of getting their own key.
    assert_eq!(normalise("Ærø"), "ærø");
}
```

**Step 2: Run to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core normalise`
Expected: FAIL — `unresolved module` / `cannot find function`.

**Step 3: Implement**

`src-tauri/core/src/places/normalise.rs`:

```rust
//! The one notion of "the same place name" in this application.

/// Lowercase, strip diacritics, collapse whitespace, trim.
///
/// Digits and punctuation survive: they are what distinguishes one street
/// number from another, and the book's entries are addresses.
///
/// Not [`crate::db::normalize_location`]: that one only collapses whitespace
/// and keeps case and diacritics, because it rewrites the string a trip stores.
/// This one throws that information away to make a key.
///
/// Folding is a closed table, not Unicode NFD: NFD would pull in a new
/// dependency for a 44-letter problem, and being a one-liner in JS it invites
/// the frontend to grow a second implementation that disagrees (ADR-008). The
/// price is that a letter the table does not know keeps its accent, and so
/// gets a key of its own. Decomposed (NFD) input is the same case: `Kos` +
/// U+030C + `ice` keys apart from the precomposed spelling — a duplicate row
/// in the cache, not a wrong coordinate, and keyboards and Nominatim emit
/// precomposed.
pub fn normalise(query: &str) -> String {
    let folded: String = query.chars().flat_map(fold_char).collect();
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fold_char(c: char) -> Vec<char> {
    // `to_lowercase` can yield several chars — realistically only 'İ', which
    // becomes 'i' plus a combining dot. Taking the first drops the dot, which is
    // the key we want. The iterator is never empty, so the fallback never fires.
    let lower = c.to_lowercase().next().unwrap_or(c);
    let replacement = match lower {
        'á' | 'ä' | 'à' | 'â' | 'ą' | 'ă' => "a",
        'č' | 'ć' | 'ç' => "c",
        'ď' => "d",
        'é' | 'ě' | 'è' | 'ê' | 'ë' | 'ę' => "e",
        'í' | 'ì' | 'î' | 'ï' => "i",
        'ĺ' | 'ľ' | 'ł' => "l",
        'ň' | 'ń' => "n",
        'ó' | 'ô' | 'ö' | 'ő' | 'ò' | 'õ' => "o",
        'ŕ' | 'ř' => "r",
        'š' | 'ś' | 'ş' => "s",
        'ť' | 'ţ' => "t",
        'ú' | 'ů' | 'ü' | 'ű' | 'ù' | 'û' => "u",
        'ý' | 'ÿ' => "y",
        'ž' | 'ź' | 'ż' => "z",
        'ß' => "ss",
        _ => return vec![lower],
    };
    replacement.chars().collect()
}
```

`src-tauri/core/src/places/mod.rs`:

```rust
//! The place book: every place a trip names, with a coordinate confirmed by a
//! human. See _tasks/75-place-book/02-design.md.

mod normalise;

pub use normalise::normalise;

#[cfg(test)]
#[path = "normalise_tests.rs"]
mod normalise_tests;
```

Add `pub mod places;` to `src-tauri/core/src/lib.rs`, keeping the list alphabetical.

**Step 4: Run to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core normalise`
Expected: PASS, 7 tests.

**Step 5: Commit**

```bash
git add src-tauri/core/src/places/ src-tauri/core/src/lib.rs
git commit -m "feat(places): add normalise, the one notion of the same place name"
```

---

## Task 2: The `places` table

**Files:**
- Create: `src-tauri/core/migrations/2026-09-07-100000_add_places/up.sql`
- Create: `src-tauri/core/migrations/2026-09-07-100000_add_places/down.sql`
- Modify: `src-tauri/core/src/schema.rs`
- Modify: `src-tauri/core/src/models.rs`

**Step 1: Write the migration**

`up.sql`:

```sql
-- Task 75: the place book. One row per normalised place name, holding the
-- coordinate a human confirmed for it.
--
-- There is no list of places here: the list is derived from trips at read time
-- (ADR-033), so this table stores coordinates and nothing else. A row whose
-- trips are all deleted becomes a harmless orphan rather than a wrong answer.
CREATE TABLE places (
    normalised_name TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    lat REAL,
    lon REAL,
    source TEXT NOT NULL
);
```

`down.sql`:

```sql
DROP TABLE IF EXISTS places;
```

**Step 2: Add the schema entry**

In `src-tauri/core/src/schema.rs`, matching the existing `diesel::table!` blocks:

```rust
diesel::table! {
    places (normalised_name) {
        normalised_name -> Text,
        display_name -> Text,
        lat -> Nullable<Double>,
        lon -> Nullable<Double>,
        source -> Text,
    }
}
```

**Step 3: Add the model**

In `src-tauri/core/src/models.rs`, alongside the existing models:

```rust
/// How a place got its coordinates. Kept so a later reader can tell a
/// suggestion someone accepted from a pin someone dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaceSource {
    Geocoder,
    Manual,
}

/// One row of the Miesta list: a place a trip names, and its coordinate if a
/// human has confirmed one. `lat`/`lon` are None until then.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    /// The spelling trips use, verbatim (ADR-034).
    pub display_name: String,
    pub normalised_name: String,
    /// How many trip endpoints name this place.
    pub uses: i64,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub source: Option<PlaceSource>,
}
```

**Step 4: Verify the migration runs**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core migration`
Expected: PASS — the existing migration tests build a database from scratch, so a broken migration fails here.

**Step 5: Commit**

```bash
git add src-tauri/core/migrations/2026-09-07-100000_add_places/ src-tauri/core/src/schema.rs src-tauri/core/src/models.rs
git commit -m "feat(places): add the places table"
```

---

## Task 3: `list_places_internal` — the derived list

**The one thing to get right here:** the join between trips and `places` **cannot happen in SQL**, because the key is `normalise()`, a Rust function SQLite cannot call. So SQL groups the raw strings, and Rust folds and joins them. At tens of rows this is free.

**Files:**
- Modify: `src-tauri/core/src/db.rs` (add `distinct_trip_places` + `all_places`)
- Create: `src-tauri/core/src/commands_internal/places.rs`
- Create: `src-tauri/core/src/commands_internal/places_tests.rs`
- Modify: `src-tauri/core/src/commands_internal/mod.rs`

**Step 1: Write the failing tests**

`src-tauri/core/src/commands_internal/places_tests.rs`:

```rust
use crate::commands_internal::places::*;
use crate::db::Database;
use crate::models::PlaceSource;
use crate::db_tests::{create_test_vehicle, seed_trip}; // both pub(crate), see db_tests.rs:93

#[test]
fn lists_every_place_a_trip_names_with_its_use_count() {
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    seed_trip(&db, &v.id, "Office, City A", "Depot, City B");
    seed_trip(&db, &v.id, "Depot, City B", "Office, City A");
    seed_trip(&db, &v.id, "Office, City A", "Depot, City B");

    let places = list_places_internal(&db).unwrap();

    assert_eq!(places.len(), 2);
    let office = places.iter().find(|p| p.display_name == "Office, City A").unwrap();
    assert_eq!(office.uses, 3);
    assert!(office.lat.is_none(), "a place nobody has placed has no coordinates");
}

#[test]
fn a_place_with_coordinates_reports_them() {
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    seed_trip(&db, &v.id, "Office, City A", "Depot, City B");
    save_place_internal(&db, &app_state(), "Office, City A".into(), 48.1, 17.1, PlaceSource::Geocoder).unwrap();

    let places = list_places_internal(&db).unwrap();
    let office = places.iter().find(|p| p.display_name == "Office, City A").unwrap();
    assert_eq!(office.lat, Some(48.1));
    assert_eq!(office.source, Some(PlaceSource::Geocoder));
}

#[test]
fn spellings_that_normalise_alike_are_one_place() {
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    seed_trip(&db, &v.id, "Kosice", "Depot, City B");
    seed_trip(&db, &v.id, "KOŠICE", "Depot, City B");

    let places = list_places_internal(&db).unwrap();

    // One entry, both trips counted. The displayed spelling is the most-used
    // one — arbitrary between equals, but never both.
    let kosice: Vec<_> = places.iter().filter(|p| p.normalised_name == "kosice").collect();
    assert_eq!(kosice.len(), 1);
    assert_eq!(kosice[0].uses, 2);
}

#[test]
fn an_orphan_place_row_is_not_listed() {
    // A places row whose trips were all deleted keeps its coordinates on disk
    // but must not appear — the list is what trips say, not what the table holds.
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    save_place_internal(&db, &app_state(), "Nowhere, City Z".into(), 1.0, 2.0, PlaceSource::Manual).unwrap();

    assert!(list_places_internal(&db).unwrap().is_empty());
}
```

If `seed_trip` does not exist in [db_tests.rs](../../src-tauri/core/src/db_tests.rs), add it beside `create_test_vehicle` there (`:93`) as `pub(crate)`; do not duplicate trip construction in this file.

**Step 2: Run to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core places`
Expected: FAIL — `list_places_internal` not found.

**Step 3: Implement the database reads**

In `src-tauri/core/src/db.rs`, following the raw-SQL precedent of `get_purposes_for_vehicle` (`db.rs:489`), which exists for exactly this kind of query:

```rust
    /// Every place string any trip names, with how many trip endpoints use it.
    /// Raw SQL: this is a UNION ALL of two columns, which Diesel's DSL expresses
    /// far less clearly than the query itself.
    pub fn distinct_trip_places(&self) -> QueryResult<Vec<(String, i64)>> {
        let conn = &mut *self.conn.lock().unwrap();

        #[derive(QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Text)]
            raw: String,
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            uses: i64,
        }

        let rows = diesel::sql_query(
            "SELECT raw, SUM(uses) AS uses FROM (
                 SELECT origin AS raw, COUNT(*) AS uses FROM trips GROUP BY origin
                 UNION ALL
                 SELECT destination AS raw, COUNT(*) AS uses FROM trips GROUP BY destination
             ) GROUP BY raw",
        )
        .load::<Row>(conn)?;

        Ok(rows.into_iter().map(|r| (r.raw, r.uses)).collect())
    }

    /// Every stored coordinate, keyed by normalised name.
    pub fn all_places(&self) -> QueryResult<Vec<PlaceRow>> {
        let conn = &mut *self.conn.lock().unwrap();
        places::table.load::<PlaceRow>(conn)
    }
```

**Step 4: Implement the command**

`src-tauri/core/src/commands_internal/places.rs`:

```rust
//! The place book. See _tasks/75-place-book/02-design.md.

use crate::db::Database;
use crate::models::{Place, PlaceSource};
use crate::places::normalise;
use std::collections::HashMap;

/// Every place any trip names, joined onto its stored coordinate.
///
/// The join is done here rather than in SQL because the key is `normalise()`,
/// which SQLite cannot call. At tens of places the cost is nil, and the list is
/// derived rather than stored precisely so it cannot drift (ADR-033).
pub fn list_places_internal(db: &Database) -> Result<Vec<Place>, String> {
    let raw = db.distinct_trip_places().map_err(|e| e.to_string())?;
    let stored: HashMap<String, _> = db
        .all_places()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|r| (r.normalised_name.clone(), r))
        .collect();

    // Fold spellings onto one entry, keeping the most-used as what to display.
    let mut by_key: HashMap<String, (String, i64)> = HashMap::new();
    for (spelling, uses) in raw {
        let key = normalise(&spelling);
        if key.is_empty() {
            continue;
        }
        by_key
            .entry(key)
            .and_modify(|(display, total)| {
                if uses > *total {
                    *display = spelling.clone();
                }
                *total += uses;
            })
            .or_insert((spelling, uses));
    }

    let mut out: Vec<Place> = by_key
        .into_iter()
        .map(|(key, (display_name, uses))| {
            let row = stored.get(&key);
            Place {
                display_name,
                normalised_name: key,
                uses,
                lat: row.and_then(|r| r.lat),
                lon: row.and_then(|r| r.lon),
                source: row.and_then(|r| PlaceSource::parse(&r.source)),
            }
        })
        .collect();

    // Unplaced first — they are the work left to do — then by use, then by name
    // so the order is stable across calls.
    out.sort_by(|a, b| {
        a.lat.is_some()
            .cmp(&b.lat.is_some())
            .then(b.uses.cmp(&a.uses))
            .then(a.display_name.cmp(&b.display_name))
    });
    Ok(out)
}
```

**Step 5: Run the tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core places`
Expected: PASS.

**Step 6: Commit**

```bash
git add src-tauri/core/src/db.rs src-tauri/core/src/commands_internal/places.rs src-tauri/core/src/commands_internal/places_tests.rs src-tauri/core/src/commands_internal/mod.rs
git commit -m "feat(places): derive the place list from trips"
```

---

## Task 4: `save_place_internal` and `clear_place_internal`

**Files:**
- Modify: `src-tauri/core/src/db.rs` (upsert + delete)
- Modify: `src-tauri/core/src/commands_internal/places.rs`
- Modify: `src-tauri/core/src/commands_internal/places_tests.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn saving_a_place_stores_the_trips_own_spelling() {
    // ADR-034: display_name is what trips say, never the geocoder's rendering.
    let db = Database::in_memory().unwrap();
    save_place_internal(&db, &app_state(), "Office, City A".into(), 48.1, 17.1, PlaceSource::Manual).unwrap();
    let row = db.all_places().unwrap().pop().unwrap();
    assert_eq!(row.display_name, "Office, City A");
    assert_eq!(row.normalised_name, "office, city a");
}

#[test]
fn saving_the_same_place_twice_replaces_the_coordinate() {
    let db = Database::in_memory().unwrap();
    save_place_internal(&db, &app_state(), "Office, City A".into(), 1.0, 2.0, PlaceSource::Geocoder).unwrap();
    save_place_internal(&db, &app_state(), "OFFICE, CITY A".into(), 3.0, 4.0, PlaceSource::Manual).unwrap();

    let rows = db.all_places().unwrap();
    assert_eq!(rows.len(), 1, "the two spellings are one key");
    assert_eq!(rows[0].lat, Some(3.0));
}

#[test]
fn clearing_a_place_removes_its_coordinate() {
    let db = Database::in_memory().unwrap();
    save_place_internal(&db, &app_state(), "Office, City A".into(), 1.0, 2.0, PlaceSource::Manual).unwrap();
    clear_place_internal(&db, &app_state(), "Office, City A".into()).unwrap();
    assert!(db.all_places().unwrap().is_empty());
}

#[test]
fn writes_are_refused_in_read_only_mode() {
    let db = Database::in_memory().unwrap();
    let state = read_only_app_state();
    assert!(save_place_internal(&db, &state, "Office, City A".into(), 1.0, 2.0, PlaceSource::Manual).is_err());
    assert!(clear_place_internal(&db, &state, "Office, City A".into()).is_err());
}
```

Copy `read_only_app_state()` from whichever existing `_tests.rs` already builds one for `check_read_only!`; do not write a second one.

**Step 2–4:** Run (FAIL), implement with `check_read_only!` as its first line in both functions, run (PASS).

**Step 5: Commit**

```bash
git add src-tauri/core/src/db.rs src-tauri/core/src/commands_internal/places.rs src-tauri/core/src/commands_internal/places_tests.rs
git commit -m "feat(places): save and clear a place's coordinate"
```

---

## Task 5: `GeocodeProvider` and the Nominatim client

**Files:**
- Create: `src-tauri/core/src/places/geocode.rs`
- Create: `src-tauri/core/src/places/geocode_tests.rs`
- Modify: `src-tauri/core/src/places/mod.rs`
- Modify: `src-tauri/core/src/settings.rs` (add the mock env var constant)

**Step 1: Write the failing tests**

Mirror [osrm_tests.rs](../../src-tauri/core/src/route_map/osrm_tests.rs)'s use of `wiremock`:

```rust
#[tokio::test]
async fn parses_candidates_from_a_nominatim_response() { /* wiremock serves a canned body */ }

#[tokio::test]
async fn an_empty_result_is_not_an_error() {
    // "no candidates" means "drop a pin by hand", not "the service failed".
}

#[tokio::test]
async fn an_http_error_is_an_error() {}

#[tokio::test]
async fn malformed_json_is_an_error_not_a_panic() {}

#[tokio::test]
async fn the_request_carries_no_country_filter() {
    // ADR-035: five of the book's places are outside Slovakia.
    // Assert the query string has no `countrycodes`.
}

#[tokio::test]
async fn mock_mode_short_circuits_the_network() {
    // With KNIHA_JAZD_MOCK_GEOCODER_DIR set, no request reaches the server.
    // Assert wiremock received zero requests.
}
```

**Step 2: Run to verify they fail.**

**Step 3: Implement**

```rust
/// One geocoder match.
pub struct Candidate {
    pub lat: f64,
    pub lon: f64,
    /// What the geocoder calls it. Shown while choosing, never stored — the
    /// book keeps the trip's own spelling (ADR-034).
    pub label: String,
}

#[async_trait::async_trait]
pub trait GeocodeProvider: Send + Sync {
    /// Best matches first. An empty vec is a valid answer, not an error.
    async fn search(&self, query: &str) -> Result<Vec<Candidate>, String>;
}
```

`HttpGeocodeProvider` builds its client exactly as [tiles.rs](../../src-tauri/core/src/route_map/tiles.rs) does — the same `USER_AGENT` constant shape, a timeout, and a client-build failure kept as an error string rather than a panic. It requests `format=jsonv2&limit=5&accept-language=sk` and **no `countrycodes`**.

Mock mode mirrors `gemini.rs:283` exactly: if `KNIHA_JAZD_MOCK_GEOCODER_DIR` is set, read `{dir}/{normalise(query)}.json` and return its candidates, making no request. A missing file returns an empty vec, which the UI already handles as "place it by hand".

Add to `settings.rs`'s `env_vars`:

```rust
    /// Set to a directory of `{normalised-query}.json` files to make geocoding
    /// deterministic in tests. Mirrors KNIHA_JAZD_MOCK_GEMINI_DIR.
    pub const MOCK_GEOCODER_DIR: &str = "KNIHA_JAZD_MOCK_GEOCODER_DIR";
```

**Step 4: Run (PASS). Step 5: Commit.**

```bash
git add src-tauri/core/src/places/ src-tauri/core/src/settings.rs
git commit -m "feat(places): add GeocodeProvider with a Nominatim client and mock mode"
```

---

## Task 6: Dispatcher wiring

**Files:**
- Modify: `src-tauri/core/src/server/dispatcher.rs` (`list_places`, `save_place`, `clear_place`)
- Modify: `src-tauri/core/src/server/dispatcher_async.rs` (`geocode_place`)

**Step 1: Write the failing tests** — one per command, in the existing `mod tests` of each dispatcher, asserting the arm dispatches and that `save_place` / `clear_place` are refused read-only.

**Step 3: Implement**, following the `get_trip_route` / `generate_route` arms verbatim in shape. `geocode_place` constructs `HttpGeocodeProvider` inside the arm, as `generate_route` constructs `HttpRouteProvider`.

**Step 5: Commit**

```bash
git add src-tauri/core/src/server/dispatcher.rs src-tauri/core/src/server/dispatcher_async.rs
git commit -m "feat(places): dispatch the four place commands"
```

---

# Phase 2 — Frontend

## Task 7: i18n strings

**Files:** `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

Add a `places` group: section title, the *"N / M placed"* counter, search placeholder, the edit dialog's title, search button, "drop a pin" hint, "no matches — place it on the map", save, clear, and the unplaced badge. Slovak is the source of truth.

**Then run `npm run i18n`** — nothing else regenerates `i18n-types.ts`, and `npm run check` reports phantom errors until it does.

```bash
git add src/lib/i18n/ && git commit -m "feat(places): add Slovak and English strings"
```

---

## Task 8: Types and API wrappers

**Files:** `src/lib/types.ts`, `src/lib/api.ts`

```ts
export interface Place {
	displayName: string;
	normalisedName: string;
	uses: number;
	lat: number | null;
	lon: number | null;
	source: 'geocoder' | 'manual' | null;
}

export interface GeocodeCandidate { lat: number; lon: number; label: string; }
```

Four wrappers following `getTripRoute`'s shape (`api.ts:519`): `listPlaces`, `geocodePlace`, `savePlace`, `clearPlace`.

```bash
git add src/lib/types.ts src/lib/api.ts && git commit -m "feat(places): add types and API wrappers"
```

---

## Task 9: The Miesta section

**Files:** `src/routes/settings/+page.svelte`

A `<section class="settings-section" id="places">` modelled on the vehicles section (`:1338`): heading with an *N / M placed* counter, a filter box, and a row per place showing name, use count, coordinates or an unplaced marker, and an edit button. Add `data-testid` attributes for the integration test.

```bash
git add src/routes/settings/+page.svelte && git commit -m "feat(places): list places in settings"
```

---

## Task 10: The edit dialog

**Files:** `src/routes/settings/+page.svelte` (or a new `src/lib/components/PlaceModal.svelte` if the section exceeds ~150 lines — prefer the component)

Leaflet is imported lazily and never at module scope, because it touches `window` at import time — copy the pattern from [mapa/+page.svelte](../../src/routes/mapa/+page.svelte) (`:62`, and the `$effect` that waits for both the library and the container element).

Behaviour: search calls `geocodePlace`; candidates list with labels; clicking one moves the pin and sets source `geocoder`; dragging the pin sets `manual`; Save calls `savePlace` and refreshes the list. Nothing is written until Save.

```bash
git add src/lib/components/PlaceModal.svelte src/routes/settings/+page.svelte
git commit -m "feat(places): place a point from a map dialog"
```

---

## Task 11: Autocomplete from the book

**Files:** `src/lib/components/TripRow.svelte:166-168`

Replace the `routes`-derived `locationSuggestions` with the book's `displayName`s. Keep the alphabetical sort; the ordering was never meaningful (see [task 76](../76-route-usage-counter-drift/) — the backend's `usage_count DESC` is discarded here anyway).

**Test:** an integration assertion that a place used by another vehicle appears in the datalist, which is the behaviour change — the old source was per-vehicle.

```bash
git add src/lib/components/TripRow.svelte && git commit -m "feat(places): feed trip autocomplete from the book"
```

---

# Phase 3 — Verification and documentation

## Task 12: Integration test

**Files:**
- Create: `tests/integration/specs/tier2/places.spec.ts`
- Create: `tests/integration/data/geocoder/*.json` (mock candidates)
- Modify: `tests/integration/wdio.server.conf.ts` (set `KNIHA_JAZD_MOCK_GEOCODER_DIR`, as it already does for Gemini)
- Modify: `.github/workflows/test.yml` (pass `-e KNIHA_JAZD_MOCK_GEOCODER_DIR=/testdata/geocoder` beside the existing `KNIHA_JAZD_MOCK_GEMINI_DIR` at `:154` and `:241`)

**One spec, three assertions** — the flow, not the logic:
1. Seeding a trip makes its places appear in Miesta, marked unplaced.
2. Placing one from the dialog persists it and survives a reload.
3. A place used by the other vehicle appears in the trip form's suggestions.

Do **not** assert that a geocoder returns the right place for an address — that is Nominatim's property, not this code's.

```bash
git add tests/integration/specs/tier2/places.spec.ts tests/integration/data/geocoder/ tests/integration/wdio.server.conf.ts .github/workflows/test.yml
git commit -m "test(places): cover the Miesta flow with a stubbed geocoder"
```

---

## Task 13: Verify and document

**Steps:**

1. `npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web`
2. `npm run test:all` — full sweep, not a focused run. This is the merge gate.
3. `npm run check` — must be clean; if it is not, `npm run i18n` was skipped.
4. Create `docs/features/place-book.md` per [docs/CLAUDE.md](../../docs/CLAUDE.md): the user flow, why every place is confirmed by hand, and why the list is derived.
5. `/changelog` — this one *is* user-visible, unlike the planning commits.
6. Update [_tasks/index.md](../index.md): task 75 → ✅ Complete, and move the folder to `_done/`.

```bash
git add docs/features/place-book.md CHANGELOG.md _tasks/index.md
git commit -m "docs(places): document the place book"
```

---

## Deferred, recorded so nobody builds them by accident

- **Merge, rename or alias UI.** The 2026-09-06 cleanup did it once as a data fix. Revisit only if duplicates recur in practice.
- **Re-geocoding a place after its trips change.** A confirmed coordinate stays until someone clears it.
- **Task 72's routing phases.** Its Phase 1 is superseded by this task; the rest is untouched.
- **Pointing the *router* at a stub.** This task adds the mock-geocoder env var; the OSRM equivalent stays task 72's business, though it now has a pattern to copy.
