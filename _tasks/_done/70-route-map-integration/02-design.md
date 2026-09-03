**Date:** 2026-08-10
**Subject:** Route map integration — design
**Status:** Planning

Requirements in [01-task.md](./01-task.md). POC this graduates:
[../61-route-map-poc/](../61-route-map-poc/).

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  SvelteKit frontend                                          │
│    TripRow action ──▶ window.open('/mapa?trip={id}')         │
│    /mapa: Leaflet + live OSM tiles, polyline only            │
│           [Generovať znova] [Uložiť] [Odstrániť mapu]        │
└──────────────────────────────────────────────────────────────┘
                    │ apiCall() — IPC or POST /api/rpc
                    ▼
┌──────────────────────────────────────────────────────────────┐
│  core/src/commands_internal/route_maps.rs                    │
│    generate_route · get_trip_route · save_trip_route         │
│    · delete_trip_route                                       │
├──────────────────────────────────────────────────────────────┤
│  core/src/route_map/                                         │
│    ga.rs        pure GA over injected RNG + node dataset     │
│    osrm.rs      polyline fetch (behind RouteProvider trait)  │
│    render.rs    tiles → stitch → project → stroke → PNG      │
│    tiles.rs     tile grid math + disposable disk cache       │
├──────────────────────────────────────────────────────────────┤
│  SQLite: trip_routes (polyline + meta, a few KB per trip)    │
│  App-data cache (disposable): tiles/ + rendered PNGs         │
└──────────────────────────────────────────────────────────────┘
```

Per [ADR-008](../../DECISIONS.md) all generation, routing and rendering is Rust.
The frontend displays a polyline and confirms it. Commands live in
[core/src/commands_internal/](../../src-tauri/core/src/commands_internal/) and
dispatch identically over Tauri IPC and `/api/rpc`, so nothing here is
mode-specific — see [ARCHITECTURE.md](../../ARCHITECTURE.md).

## Data model

### Waypoints are coordinates, not node indices

The single design decision that anticipates V2. The POC represents a route as
indices into its 67-node dataset, which cannot express a point a human dropped
somewhere else. Since V2 adds a manual editor, waypoints persist as coordinates
from the start:

```jsonc
[
  { "lat": 48.9350, "lon": 20.5533, "name": "Domov",   "node_idx": 0  },
  { "lat": 48.9973, "lon": 20.5911, "name": "Levoča",  "node_idx": 14 },
  { "lat": 48.9350, "lon": 20.5533, "name": "Domov",   "node_idx": 0  }
]
```

`node_idx` is present when the generator picked the point from the dataset and
absent when a human placed it. V2 needs no migration.

This also fixes the map view's shape for both versions. Its state is *an ordered
waypoint list*; everything downstream is derived:

```
waypoints ──(OSRM)──▶ polyline ──▶ Leaflet preview
    ▲                                     │
    │                             confirm │
 V1: GA fills it                          ▼
 V2: + drag / insert / remove         trip_routes row
```

V1 wires exactly one producer into that list. V2 adds more and nothing else moves.

### Table `trip_routes`

One new Diesel migration, `2026-08-10-100000_add_trip_routes` — see
[migration conventions](../../.claude/rules/migrations.md) and existing migrations
in [core/migrations/](../../src-tauri/core/migrations/).

| column | type | notes |
|---|---|---|
| `trip_id` | TEXT PK | FK → `trips(id)` ON DELETE CASCADE — at most one map per trip |
| `waypoints` | TEXT NOT NULL | JSON array, shape above |
| `polyline` | TEXT NOT NULL | encoded polyline5 from OSRM, ~2–5 KB |
| `target_km` | REAL NOT NULL | the trip's `distance_km` at generation time |
| `road_km` | REAL NOT NULL | what OSRM actually returned |
| `dataset_version` | TEXT NULL | provenance of the node set; null once V2 edits exist |
| `created_at` | TEXT NOT NULL | |

Bounding box is **derived** from the polyline at render time, not stored.

### What is deliberately not persisted

Nothing bulky, and nothing that a backup or a database move has to carry.

| | stored | why not more |
|---|---|---|
| GA seed | no | the polyline *is* the result; "regenerate" means a fresh random route, so there is nothing to replay |
| bounding box | no | derived from the polyline in microseconds |
| rendered PNG | no | ~200 KB × N would grow the DB ~10 MB/year, bloat backups, and force [Task 32](../32-portable-csv-backup/) to base64 a binary column |
| OSM tiles | no | disposable cache |

> **As built:** only tiles are cached. The attachment PNG is rasterised at
> export time and base64'd straight into the HTML — no route-hash PNG cache was
> implemented, so a repeat export re-stitches from cached tiles rather than
> being instant. The disposability argument below is unaffected, and in fact
> strengthened: less is written to disk, not more.

Rendered PNGs and fetched tiles live in an app-data **cache** directory keyed by
route hash. Deleting it costs a re-fetch and nothing else, so
[Move Database](../../docs/features/move-database.md), the backups folder and
portable CSV backup all stay untouched. The cache is warm in practice because
every route starts at the same home base and overlaps heavily at the zoom levels
involved.

## Route generation

The genetic algorithm ports from
[poc.html](../61-route-map-poc/poc.html) essentially unchanged; the node dataset
([villages.json](../61-route-map-poc/villages.json),
[matrix.json](../61-route-map-poc/matrix.json)) ships as `include_str!` assets in
`core`. Rationale for GA-over-heuristic, dataset sizing and hyperparameters is in
[the POC design](../61-route-map-poc/02-design.md) and not repeated here.

Randomness is business logic, so it stays in Rust and splits in two for
testability — a pure function taking an injected RNG, plus a thin wrapper that
supplies it:

```rust
pub fn generate_route(target_km: f64, ds: &Dataset, rng: &mut impl Rng) -> RouteResult;
pub fn generate_route_random(target_km: f64, ds: &Dataset) -> RouteResult;
```

Tests drive the first deterministically; production calls the second and stays
varied. **No multi-session split** — the POC's `SINGLE_SESSION_MAX_KM` logic does
not apply, since one trip is one map. A `distance_km` the 67-node pool cannot
reach yields the best attempt with its error % surfaced, as in the POC.

OSRM sits behind a `RouteProvider` trait so tests run offline against fixtures.

## UI flow

**Trip grid.** [TripRow.svelte](../../src/lib/components/TripRow.svelte)'s actions
cell — today insert-above and delete — gains a map-pin icon, outline when no route
is saved and filled when one is. Both states open the same view. No purpose
filter: the user decides which rows warrant a map.

**Map view**, a new route at `/mapa?trip={id}`, opened with `window.open`. It
loads the trip (target km = `distance_km`) and any saved route, generating one on
open when none exists. Controls are three buttons — *Generovať znova*, *Uložiť*,
*Odstrániť mapu* — plus the POC's target / actual / error readout. Rendering is
the POC verbatim: Leaflet, live OSM tiles, single polyline, no markers.

**Regenerating persists nothing.** Each click re-runs the generator; only
*Uložiť* writes. That is what makes confirmation load-bearing — the user spins
until a route looks right, then commits it. On save the view posts
`save_trip_route`, pings the grid tab over `BroadcastChannel`, and that row's icon
updates.

All user-facing strings go through i18n — see
[svelte-frontend.md](../../.claude/rules/svelte-frontend.md).

**Capability gate.** `/api/capabilities`
([core/src/server/mod.rs](../../src-tauri/core/src/server/mod.rs)) gains
`route_maps: true`;
[capabilities.ts](../../src/lib/stores/capabilities.ts) defaults it to `false`
under Tauri, so the icon does not render in the desktop build. Enabling desktop
later is that default plus an overlay component wrapping the same view — the
backend needs no change.

## Export

[export.rs](../../src-tauri/core/src/export.rs)'s `ExportData` gains
`route_maps: Vec<RouteMapPage> { attachment_no, row_number, png_base64 }`. After
the closing `</table>`, `generate_html` emits one `<div class="map-page">` per
entry with `page-break-before: always`, a heading, and a full-bleed
`<img src="data:image/png;base64,…">`. The existing `@page { size: A4 landscape }`
rule applies unchanged.

Each page carries **only** `Príloha č. N — záznam č. {row}` and the map. The main
trip table is untouched — no new column — so the reference runs one way,
attachment → row.

### Row numbering is the correctness risk

Row numbers are the only cross-reference, and the two export paths number rows
differently today:

- desktop's `export_to_browser`
  ([desktop/src/commands/export_cmd.rs](../../src-tauri/desktop/src/commands/export_cmd.rs))
  injects a synthetic "Prvý záznam" row 0;
- `export_html_internal`
  ([core/src/commands_internal/export_cmd.rs](../../src-tauri/core/src/commands_internal/export_cmd.rs))
  does not, and additionally hardcodes `hidden_columns: vec![]` and
  `sort_direction: "asc"`.

So `row_number` **must be read out of the same assembled row list that numbers the
printed table**, never computed alongside it. Computing it independently makes
web and desktop attachments cite different rows for the same map.

### Render pipeline

Runs at export time only — the map view itself uses live Leaflet tiles.

1. Decode polyline5 → coordinates.
2. Compute bounding box → pick the zoom that fits it in ~1400×900 px → derive the
   tile grid.
3. Fetch tiles, cache-first, into the disposable cache.
4. Stitch into a canvas; project coordinates to pixels; stroke 5 px `#0066cc`.
5. Bake OSM attribution into the image, bottom-right.
6. Encode PNG, cache by route hash, base64 into the HTML.

New dependencies: [image](https://crates.io/crates/image) (PNG only) plus
[imageproc](https://crates.io/crates/imageproc) or
[tiny-skia](https://crates.io/crates/tiny-skia) for a decently stroked line.
[reqwest](https://crates.io/crates/reqwest) (blocking) and
[base64](https://crates.io/crates/base64) are already in `core`. The
[OSM tile usage policy](https://operations.osmfoundation.org/policies/tiles/)
requires an identifying User-Agent on every tile request.

## Error handling

| condition | behaviour |
|---|---|
| GA cannot reach target within tolerance | render best attempt, surface actual error % (POC behaviour) |
| OSRM unreachable / rate-limited in map view | show error with retry; waypoints kept so retry skips the GA |
| tiles unreachable at export, cache cold | render the polyline on a plain background — the export still succeeds rather than failing the whole document |
| tile fetch partially fails | stitch what arrived; missing tiles render blank |
| a `trip_routes` row is undecodable | skip that attachment, log it, leave the rest of the export intact |
| trip deleted | route row cascades away |

## Testing

Per [CLAUDE.md](../../CLAUDE.md): every use-case gets exactly one authoritative
test. Calculation and generation logic is proven in Rust; integration tests cover
UI flows only and do not re-test the maths.

**Backend unit tests** — see
[rust-backend.md](../../.claude/rules/rust-backend.md):

- GA under a seeded RNG: sequence starts and ends at home; every index valid;
  within ±5 % across a spread of targets; different seeds produce different
  routes.
- Zoom selection and tile-grid derivation as pure functions.
- Coordinate → pixel projection at known zoom and tile origin.
- Polyline encode/decode round-trip.
- `save` / `get` / `delete_trip_route` round-trip, including cascade delete when
  the trip goes.
- `generate_html` with N route maps: N page-break blocks, correct attachment
  numbers, and row numbers matching the assembled table rows.
- Renderer against a fixture tile set — expected PNG dimensions, no network.

**Integration tests** (tier 2, WebdriverIO) — see
[integration-tests.md](../../.claude/rules/integration-tests.md):

- Map action visible in server mode; add map → save → row icon reflects it.
- Regenerating without saving persists nothing.
- Removing a map clears the icon.

Neither OSRM nor live tiles are called in any test — both sit behind injected
traits.

## File layout

```
_tasks/70-route-map-integration/
├── 01-task.md      (see ./01-task.md)
├── 02-design.md    (this file)
└── 03-plan.md      (created later by superpowers:writing-plans)
```
