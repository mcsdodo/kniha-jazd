**Date:** 2026-09-03
**Subject:** Route maps V2 — origin/destination routing, alternatives, manual editing — design
**Status:** Planning

Requirements in [01-task.md](./01-task.md). V1 this extends:
[../_done/70-route-map-integration/](../_done/70-route-map-integration/).

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  SvelteKit frontend — /mapa?trip={id}                            │
│    place picker (on cache miss)                                  │
│    Leaflet: active line + inactive alternatives                  │
│    drag endpoints · press line to add waypoint · click to remove │
│    [Prepočítať] [Generovať znova]¹ [Uložiť] [Odstrániť mapu]     │
└──────────────────────────────────────────────────────────────────┘
                    │ apiCall() — IPC or POST /api/rpc
                    ▼
┌──────────────────────────────────────────────────────────────────┐
│  core/src/commands_internal/route_maps.rs                        │
│    NEW  resolve_place · remember_place · route_direct            │
│    KEEP generate_route · get_trip_route · save_trip_route        │
│         · delete_trip_route                                      │
├──────────────────────────────────────────────────────────────────┤
│  core/src/route_map/                                             │
│    NEW  geocode.rs   normalise() + GeocodeProvider trait         │
│    ga.rs        unchanged — loop mode only                       │
│    osrm.rs      + alternatives, + duration                       │
│    render.rs    unchanged                                        │
│    tiles.rs     unchanged                                        │
├──────────────────────────────────────────────────────────────────┤
│  SQLite: trip_routes (+ mode) · place_aliases (NEW)              │
└──────────────────────────────────────────────────────────────────┘

¹ loop mode only
```

Per [ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication)
everything that decides anything is Rust: which mode a row is in, what a place name
means, which roads a route follows, and how far it misses the recorded distance. The
frontend draws coordinate lists and confirms them.

## Mode selection

One pure function, one call site, one rule:

```rust
/// Loop when the row names the same place twice, direct otherwise.
///
/// Compared after normalisation so "Domov" and "domov " are one place — the
/// same normalisation the alias cache keys on, so a row cannot be direct-mode
/// here and cache-collide there.
fn mode_for(origin: &str, destination: &str) -> Result<RouteMode, String>
```

Empty endpoints return `Err`. A silent fallback to loop mode would hand the user a map
of somewhere they never were, labelled as evidence — the one failure this feature must
not have.

## Geocoding

### `route_map/geocode.rs`

```rust
pub struct Place {
    pub lat: f64,
    pub lon: f64,
    /// What the geocoder called it — shown in the picker and stored as the
    /// waypoint's `name`.
    pub display_name: String,
}

#[async_trait::async_trait]
pub trait GeocodeProvider: Send + Sync {
    /// Up to `MAX_CANDIDATES` matches, best first. An empty vec is a valid
    /// answer, not an error.
    async fn search(&self, query: &str) -> Result<Vec<Place>, String>;
}
```

`HttpGeocodeProvider` queries Nominatim (`format=jsonv2`, `limit=5`,
`countrycodes=sk`, `accept-language=sk`) carrying a User-Agent that identifies this
application by name and version, at most one request per second — the same two
obligations [tiles.rs](../../src-tauri/core/src/route_map/tiles.rs) already meets for
OSM tiles, for the same reason.

The trait is the hedge. Nominatim is weak on abbreviations, and if "Spisska" turns out
to resolve badly in practice, swapping in [Photon](https://photon.komoot.io/) — built
for typo-tolerant autocomplete over the same OSM data — is one new impl and one
construction site. Nothing above the trait knows which one it is.

### Why Nominatim despite the weakness

Because the alias table means **each distinct string is geocoded exactly once, ever**,
with a human confirming the result. Typo tolerance is a live-autocomplete virtue; here
it saves the user one candidate-list click, once per place, forever. That is not worth
paying for with a service whose terms are less clearly stated.

### Normalisation

```rust
/// Lowercase, strip diacritics, collapse internal whitespace, trim.
/// "Spišská Nová Ves" · "SPISSKA NOVA VES" · " spisska  nova ves "
///   → "spisska nova ves"
pub fn normalise(query: &str) -> String
```

Pure, exhaustively unit-tested, and used for **both** the alias key and the mode
comparison. One notion of "the same place name" in the whole feature.

## Data model

### Table `place_aliases` (new)

One new Diesel migration, `2026-09-03-100000_add_place_aliases` — see
[migration conventions](../../.claude/rules/migrations.md).

| column | type | notes |
|---|---|---|
| `normalised_query` | TEXT PK | the **normalised** free text — the cache key. Named in full rather than `query`, which reads ambiguously inside a `diesel::table!` block. |
| `lat` | REAL NOT NULL | |
| `lon` | REAL NOT NULL | |
| `display_name` | TEXT NOT NULL | what the geocoder called it; shown in the UI |
| `source` | TEXT NOT NULL | `'geocoder'` (picked from candidates) or `'manual'` (pin dropped by hand) |
| `created_at` | TEXT NOT NULL | |

**Deliberately not vehicle-scoped.** "Levoča" is in the same place whichever car drove
there. Scoping it per vehicle would make the user re-confirm every place for every
vehicle, for no gain.

`source` exists so a later audit can tell a machine's guess from a human's placement.
It costs one column and answers "why is this pin in a field" without guesswork.

### Table `trip_routes` (one added column)

```sql
ALTER TABLE trip_routes ADD COLUMN mode TEXT NOT NULL DEFAULT 'loop';
```

The default is what makes this a non-event for existing data: every route saved by
Task 70 *is* a loop, so the backfill is correct by construction and no migration logic
runs.

Everything else stays as Task 70 built it:

| column | direct mode holds |
|---|---|
| `waypoints` | `[origin, ...vias, destination]` — same JSON shape, `node_idx` null throughout |
| `polyline` | the chosen route's geometry, alternatives discarded |
| `target_km` | still the row's `distance_km`, so deviation means the same thing in both modes |
| `road_km` | what OSRM returned for the chosen route |
| `dataset_version` | `NULL` — no dataset node was used, and the column is already nullable |

**No reshaping is needed**, which is the point:
[ADR-029](../../DECISIONS.md#adr-029-waypoints-persist-as-coordinates-not-dataset-indices)
chose coordinates over dataset indices for precisely this feature, a version early.
The V1 design note — *"a future manual editor must be able to store a point that is
not in the dataset"* — cashes out here at a cost of zero.

### What is still not persisted

- **Alternatives.** The user picks one; the others were never anything but a proposal.
- **Duration.** Displayed while choosing, never printed. The export renders no text at
  all, so a stored duration would have no reader.
- **Rendered PNGs and tiles.** Unchanged — see
  [ADR-028](../../DECISIONS.md#adr-028-only-the-polyline-is-persisted-tiles-live-in-a-disposable-cache).

## Commands

Following the `_internal` pattern
([ADR-016](../../DECISIONS.md#adr-016-_internal-extraction-pattern-for-command-reuse)),
dispatcher-only, split by whether they await the network:

| command | dispatcher | notes |
|---|---|---|
| `resolve_place` | `dispatcher_async.rs` | cache hit → resolved, **no network call**; miss → candidates. Writes nothing. |
| `remember_place` | `dispatcher.rs` | write — `check_read_only!` |
| `route_direct` | `dispatcher_async.rs` | routes an ordered waypoint list; requests alternatives only for two-point input |
| `generate_route` | `dispatcher_async.rs` | **unchanged** |
| `get_trip_route` / `save_trip_route` / `delete_trip_route` | `dispatcher.rs` | `save` gains `mode` |

```rust
/// Cache first, geocoder second. `resolved` is Some on a hit; on a miss it is
/// None and `candidates` carries what the geocoder offered — possibly empty,
/// which means "place it by hand", not "error".
pub struct PlaceResolution {
    pub query: String, // as typed, for display
    pub resolved: Option<Place>,
    pub candidates: Vec<Place>,
}
```

Resolution and remembering are separate calls for the same reason generation and
saving were separated in V1: **looking is not committing.** A resolve that wrote its
first guess to the alias table would make a wrong guess permanent before the user ever
saw it.

`route_direct` returns alternatives **in the order OSRM returned them** — fastest
first, navigation-app convention. Each carries its own `road_km`, `duration_s`,
`deviation_percent` and `off_target`, all computed by the existing `deviation()`
helper. Deviation labels the options; it does not reorder them.

## UI flow

```
open /mapa?trip=id
        │
        ├── origin == destination ──▶ LOOP MODE (V1, unchanged)
        │                             generate → preview → save
        │
        └── origin != destination ──▶ DIRECT MODE
                    │
                    ├─ resolve both endpoints
                    │     hit          → straight through
                    │     candidates   → picker → remember_place
                    │     none         → "place it on the map" → remember_place
                    │
                    ├─ route_direct(alternatives = 3)
                    │     active = fastest, drawn blue
                    │     others = thin grey, click to promote
                    │     each listed: km · time · deviation vs distance_km
                    │
                    ├─ EDIT (both modes)
                    │     drag endpoint / waypoint    ─┐
                    │     press line -> new waypoint   ├─▶ on DROP only: route_direct
                    │     click waypoint -> remove    ─┘
                    │
                    └─ save_trip_route(mode) → BroadcastChannel → row pin fills
```

### Re-routing fires on drop, never during drag

A drag emits a coordinate stream; the routing service is a rate-limited public server
capped at one request per second. Routing mid-drag would exhaust the budget on frames
nobody sees and make the line lag the cursor. During the drag the frontend moves a
local handle and rubber-bands the affected segment; `pointerup` sends one request for
the new ordered waypoint list.

### Where a new waypoint lands is decided in Rust

Dragging an *existing* waypoint needs no geometry — the frontend edits that entry's
coordinates and sends the list back. Dragging a *new* point off the line does: someone
has to work out which pair of existing waypoints it belongs between.

That someone is the backend. `route_direct` accepts an optional `insert_point` and the
polyline it was dragged from; it locates the nearest polyline vertex, maps each
waypoint to its own nearest vertex, and inserts into the slot those boundaries imply.
The response's `waypoints` is authoritative and the frontend adopts it wholesale.

The alternative — computing the index in the browser — would put a genuinely tricky
piece of index arithmetic outside the reach of Rust unit tests, and put a second notion
of waypoint ordering in the codebase. This way the frontend's only geometry is where to
draw a handle, which decides nothing, and one round trip on drop covers both the
insertion and the re-route.

### Alternatives vanish once a waypoint exists

OSRM computes alternatives for two-point queries; with vias it returns the single
through-route. This matches how navigation apps behave once a stop is added, so it is
not surprising — but the panel **says so** rather than going quietly empty, because an
empty list otherwise reads as "the service failed".

### The editor is mode-agnostic

Both modes are the same thing underneath: an ordered waypoint list resolved to a
polyline. So editing is written once, against the list, and loop routes get it free.
That is what rescues the one case mode selection gets wrong — a distant A–A row like
"Bratislava – Bratislava", which the Home-Base-anchored genetic algorithm cannot
express (see 01-task.md, Known limitations). Without a mode-agnostic editor that route
would be unfixable; with one it is a drag.

Loop mode keeps **Generovať znova** in addition; direct mode has nothing to
re-randomise and offers **Prepočítať** instead.

### Markers exist in the browser and nowhere else

V1 decided a bare line reads as a drive while numbered pins read as a plan. Editing
needs grab handles, so the interactive map now has them — but
[render.rs](../../src-tauri/core/src/route_map/render.rs) is untouched and still
strokes a bare line, rendering no text and no markers. The decision holds exactly
where it was load-bearing: on the printed page.

## Error handling

| failure | behaviour |
|---|---|
| Geocoder unreachable / rate-limited | error with Retry; the endpoint stays unresolved and nothing is written |
| Geocoder returns nothing | not an error — "place it on the map by hand", then remembered as `source='manual'` |
| Empty origin or destination | explicit error naming the field; **never** a silent loop |
| Routing service unreachable mid-edit | the last good route stays drawn; the pending edit is rejected with Retry, so an unroutable waypoint cannot be saved behind an error banner |
| OSRM returns no alternatives | the one route is shown; the panel says alternatives were unavailable |
| Existing V1 loop routes | render exactly as before, `mode='loop'` by migration default |

The V1 invariant carries: **a failed proposal is dropped, never left saveable.**

## Testing

Per [CLAUDE.md](../../CLAUDE.md), backend tests own the logic exhaustively and
integration tests own the UI flow without re-testing the math. No test touches the
network — `GeocodeProvider` joins `RouteProvider` and `TileFetcher` as an injected fake.

**Backend unit** — new [geocode_tests.rs](../../src-tauri/core/src/route_map/geocode_tests.rs)
plus additions to [route_maps_tests.rs](../../src-tauri/core/src/commands_internal/route_maps_tests.rs):

- `normalise`: diacritics, case, internal and surrounding whitespace, empty input
- `mode_for`: equal after normalisation → loop; different → direct; empty either side → `Err`
- alias cache hit resolves with **zero provider calls** (the fake counts them)
- alias cache miss returns candidates and **writes nothing**
- geocoder response parsing: candidates, empty result, HTTP error, malformed JSON
- `remember_place` is refused in read-only mode
- waypoint order survives into the OSRM request as `lon,lat` — mirroring V1's existing
  `sends_coordinates_as_lon_lat_in_order`, which exists because transposing coordinates
  yields a plausible route in the wrong country rather than an error
- alternatives: OSRM's order preserved; per-alternative deviation computed by the
  **same** `deviation()` as loop mode; a single-route response yields one entry
- a request carrying vias does not ask for alternatives
- save/load round-trip for `mode='direct'` with vias and `dataset_version = None`

**Migration** — [migration_tests.rs](../../src-tauri/core/src/migration_tests.rs): a
`trip_routes` row written before the migration reads back as `mode='loop'` and still
renders.

**Integration** — extending
[route-map.spec.ts](../../tests/integration/specs/tier2/route-map.spec.ts):

- an A→B row opens with a drawn route, both endpoint names, and a deviation readout
- an ambiguous origin shows the picker; after picking, **reopening the page skips it**
  (this is the alias table's whole value, so it is the one thing worth asserting twice)
- clicking an inactive alternative promotes it and updates the panel
- dragging to add a waypoint fires **exactly one** re-route, on drop
- saving fills the row pin in the grid tab

## File layout

| file | change |
|---|---|
| [route_map/geocode.rs](../../src-tauri/core/src/route_map/geocode.rs) | **new** — `normalise`, `Place`, `GeocodeProvider`, `HttpGeocodeProvider` |
| [route_map/geocode_tests.rs](../../src-tauri/core/src/route_map/geocode_tests.rs) | **new** |
| [route_map/mod.rs](../../src-tauri/core/src/route_map/mod.rs) | register the module |
| [route_map/osrm.rs](../../src-tauri/core/src/route_map/osrm.rs) | alternatives + `duration_s` on `FetchedRoute` |
| [commands_internal/route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs) | `resolve_place`, `remember_place`, `route_direct`, `mode_for`; `save` takes `mode` |
| [server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) | `remember_place` |
| [server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs) | `resolve_place`, `route_direct` |
| [models.rs](../../src-tauri/core/src/models.rs) | `Place`, `PlaceAlias`, `RouteMode`; `RouteMap.mode` |
| [schema.rs](../../src-tauri/core/src/schema.rs) | `place_aliases`, `trip_routes.mode` |
| [db.rs](../../src-tauri/core/src/db.rs) | alias get/put; `mode` in route map read/write |
| [migrations/2026-09-03-100000_add_place_aliases/](../../src-tauri/core/migrations/) | **new** |
| [mapa/+page.svelte](../../src/routes/mapa/+page.svelte) | picker, alternatives panel, drag editing |
| [lib/api.ts](../../src/lib/api.ts) | `resolvePlace`, `rememberPlace`, `routeDirect` |
| [i18n/sk](../../src/lib/i18n/sk/index.ts) · [i18n/en](../../src/lib/i18n/en/index.ts) | new strings — then `npm run i18n` |

[ga.rs](../../src-tauri/core/src/route_map/ga.rs),
[render.rs](../../src-tauri/core/src/route_map/render.rs),
[tiles.rs](../../src-tauri/core/src/route_map/tiles.rs),
[polyline.rs](../../src-tauri/core/src/route_map/polyline.rs),
[export.rs](../../src-tauri/core/src/export.rs),
[TripRow.svelte](../../src/lib/components/TripRow.svelte) and
[TripGrid.svelte](../../src/lib/components/TripGrid.svelte) are **not** touched.

## Decisions to record via `/decision`

- Free-text place names resolve through a cached alias table keyed on normalised text,
  confirmed once by a human — not re-geocoded per trip.
- Alternatives are ordered by duration, as a navigation app does; deviation from the
  recorded distance labels them but never reorders them.
- The recorded `distance_km` is never rewritten from a route's road distance.
- The waypoint editor is mode-agnostic, which is what makes a mis-anchored loop
  recoverable.
