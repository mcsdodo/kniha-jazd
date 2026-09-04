# Feature: Route Maps

> Generates a plausible driving route matching a trip's recorded distance, previews it on a map, and appends the saved routes to the printed logbook as attachment pages.

The commands are served over the HTTP API like every other command — see
[The capability flag](#the-capability-flag).

## User Flow

1. **Open the logbook** in a browser (server mode). Every trip row's action cell shows a
   map-pin icon beside the existing insert-above and delete actions. The pin is outlined
   when the trip has no saved map and filled when it has one.
2. **Click the pin.** A new browser tab opens at `/mapa?trip={id}`.
3. **The route generates on open** (if none is saved), targeting the trip's recorded
   distance. The map shows the route as a single blue line — no markers, no pins — over
   live OpenStreetMap tiles, plus a readout of target distance, actual road distance, the
   deviation in percent, and the settlement names the route passes through.
4. **"Generovať znova"** produces a different route for the same target. **Nothing is
   persisted until "Uložiť mapu"** — the user can regenerate until a route looks right.
5. **Saving** writes the route, tells the logbook tab to fill in that row's pin, and offers
   to close the map tab. **"Odstrániť mapu"** removes a saved route after confirmation.
6. **"Export pre tlač"** appends one A4-landscape page per saved map after the trip table,
   each headed `Príloha č. N — záznam č. X`.

**Failure cases:**

- The generator cannot reach the target within tolerance → the best attempt is drawn and
  the deviation percentage is flagged. See
  [Why short trips can miss target](#why-short-trips-can-legitimately-miss-target).
- The routing service is unreachable or rate-limits → an error with a Retry button; the
  stale proposal is dropped so it cannot be saved behind the error banner.
- Tile servers unreachable at export time with a cold cache → the route is drawn on a plain
  background and the export still succeeds. A whole logbook failing over an unreachable
  tile server would be far worse than one plain-background map.
- A single map fails to render → that attachment is skipped and logged; the rest of the
  export is intact, and attachment numbers close the gap rather than leaving a hole.
- Deleting a trip cascades its map away.

## Technical Implementation

### Frontend

**Map view:** [src/routes/mapa/+page.svelte](../../src/routes/mapa/+page.svelte) — Svelte 5
runes. Leaflet is bundled through Vite (not loaded from a CDN) and imported lazily inside
`onMount`, because it touches `window` at import time. The page holds two separate pieces
of state: the *saved* route and the *generated* proposal. Only save and remove touch the
first; regenerate only replaces the second. That split is what makes "regenerating persists
nothing" structural rather than a rule someone has to remember.

**Row action:** [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte)
renders the pin, gated on the capability flag.
[src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte) owns the set
of trips that have maps, opens the map tab, and listens on a `BroadcastChannel` so a save or
a removal in the map tab updates the row icon without a reload.

**API wrappers:** [src/lib/api.ts](../../src/lib/api.ts) — `generateRoute`, `getTripRoute`,
`saveTripRoute`, `deleteTripRoute`.

### Backend (Rust)

All generation, routing and rasterising is Rust
([ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication)). The
frontend draws a coordinate list and confirms it.

| Module | Responsibility |
|---|---|
| [dataset.rs](../../src-tauri/core/src/route_map/dataset.rs) | Loads the bundled 67-node settlement set and its 67×67 driving-distance matrix |
| [ga.rs](../../src-tauri/core/src/route_map/ga.rs) | Genetic algorithm picking the settlement sequence |
| [osrm.rs](../../src-tauri/core/src/route_map/osrm.rs) | Fetches road-following geometry, behind a `RouteProvider` trait |
| [polyline.rs](../../src-tauri/core/src/route_map/polyline.rs) | Polyline5 encode/decode; never panics on malformed input |
| [tiles.rs](../../src-tauri/core/src/route_map/tiles.rs) | Web Mercator tile geometry plus the cache-first tile fetcher |
| [render.rs](../../src-tauri/core/src/route_map/render.rs) | Composites tiles and strokes the route into a PNG |
| [route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs) | The four commands, plus export attachment assembly |

**Commands** are dispatcher-only. `generate_route` awaits the routing service so it lives in
[dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs); `get_trip_route`,
`save_trip_route` and `delete_trip_route` are in
[dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs). The two write commands are
guarded by the read-only check like every other write.

**Storage:** the `trip_routes` table
([migration](../../src-tauri/core/migrations/2026-08-10-100000_add_trip_routes/)), keyed by
`trip_id` with `ON DELETE CASCADE`. It holds the waypoints (JSON), the encoded polyline, the
target and road distances, the dataset version and a timestamp — a few KB per trip. No image
is stored anywhere; see
[ADR-028](../../DECISIONS.md#adr-028-only-the-polyline-is-persisted-tiles-live-in-a-disposable-cache).

### Data Flow

Generation and preview:

```
Row pin → /mapa?trip=id → generate_route
                            ↓
       genetic algorithm picks a settlement sequence (offline, matrix only)
                            ↓
       OSRM /route → encoded polyline + real road distance
                            ↓
       backend decodes to [lat, lon] pairs, computes deviation %
                            ↓
       Leaflet draws one polyline · user regenerates or saves
                            ↓
                    save_trip_route → trip_routes
```

Export:

```
Export for print → assemble the printed table's rows (record no., trip id)
                            ↓
        one batched query for every saved map among those trips
                            ↓
   per map: decode polyline → pick zoom → fetch tiles (cache-first) →
            composite → stroke the route → PNG → base64
                            ↓
       one <div class="map-page"> appended per map, page-broken
```

### Route generation, in outline

```
chromosome = 1..5 distinct settlements between two home visits
fitness    = 1 / (1 + |loop distance - target km|)
repeat 100 generations over a population of 50:
    carry the 2 fittest forward unchanged
    fill the rest by: tournament-select 2 parents (sample 3, keep the fittest)
                      order crossover, capped at 5 stops
                      with p=0.25 insert / remove / swap one stop
return the fittest chromosome as home → stops → home
```

The distance matrix is **asymmetric** (one-way streets, different routing direction), so the
order of the stops is significant, not just the set.

Randomness is business logic and stays in Rust
([ADR-014](../../DECISIONS.md#adr-014-jitter-stays-in-rust-testability-via-jitter-trait)).
The generator splits in two: a pure function taking an injected RNG, plus a thin wrapper
supplying a thread RNG. Tests run deterministically against seeded runs; production stays
varied. This mirrors the `Jitter` split in
[time_inference.rs](../../src-tauri/core/src/calculations/time_inference.rs).

### Export rendering

Runs at export time only — the interactive map is Leaflet in the browser and never goes
through the rasteriser. Canvas is 1400×900 px, sized for the attachment page's `170mm`
maximum height at roughly 150 dpi. The route is stroked 5 px in `#0066cc`, dark enough to
stay legible when the logbook is printed in greyscale. Missing tiles leave OpenStreetMap's
land colour rather than a black hole.

Two constraints come straight from the
[OSM tile usage policy](https://operations.osmfoundation.org/policies/tiles/): tiles are
fetched at most two at a time, and every request carries a User-Agent identifying this
application by name and version. Attribution is a caption in the export HTML next to the
image rather than text baked into the pixels — the rasteriser renders no text at all, by
design.

## Key Files

| File | Purpose |
|------|---------|
| [src/routes/mapa/+page.svelte](../../src/routes/mapa/+page.svelte) | Map view: preview, regenerate, save, remove |
| [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte) | Map-pin row action, filled when a map is saved |
| [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte) | Opens the map tab; keeps pin state fresh over `BroadcastChannel` |
| [src-tauri/core/src/route_map/](../../src-tauri/core/src/route_map/) | Dataset, genetic algorithm, OSRM client, polyline codec, tiles, rasteriser |
| [src-tauri/core/src/commands_internal/route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs) | Commands + export attachment assembly |
| [src-tauri/core/src/commands_internal/statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs) | Adds the "which trips have maps" set to the grid data |
| [src-tauri/core/src/export.rs](../../src-tauri/core/src/export.rs) | Attachment page markup and print CSS |
| [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) | `Waypoint`, `RouteMap` |
| [src-tauri/core/assets/](../../src-tauri/core/assets/) | Bundled 67-node dataset and distance matrix |

## Design Decisions

### Why a genetic algorithm rather than a deterministic heuristic

Both were built and compared during the
[POC](../../_tasks/_done/61-route-map-poc/02-design.md). The GA's non-determinism turned out to be
the load-bearing property: several proof-of-driving maps at similar distances **must not look
alike**, or the synthetic pattern is obvious to anyone reading them side by side. A
deterministic heuristic returns the same answer for the same target every time — exactly the
wrong property here. Variety is the feature, and the GA hits target accurately enough that
it costs nothing to prefer it.

Measured over 200 generated routes spread across 50–500 km targets: **all 200 landed within
the 5% tolerance**, and 200 seeds at one fixed target produce essentially 200 different
routes (measured 188–200 distinct, depending on the target).

### Why short trips can legitimately miss target

Below roughly 30 km the dataset floors out. The nearest settlements quantise the shortest
possible loop — the closest is a 2.8 km round trip, and the next is 10 km — so a 5 km target
has nothing to reach it with, and targets in the 10–25 km range hit tolerance only sometimes.
There is no algorithmic fix short of a denser dataset, so the map view **always shows the
deviation percentage** and highlights it when it exceeds tolerance, rather than silently
presenting a route that does not match the trip.

Tolerance is a single constant in Rust and is applied to the *road* distance the finished
route covers, not to the matrix distance the algorithm optimises internally. The frontend
displays the backend's verdict and never derives its own — otherwise the page could flag a
route the backend considers perfectly in tolerance.

### Why attachments cite a record number, never a position

An attachment page's only link back to the logbook is `záznam č. X`, so getting it wrong
points the printed evidence at the wrong journey. The number is read from the same
`trip_numbers` map the printed table's first column is rendered from — never from a position
in a list. Positions are not comparable: the export injects a synthetic "Prvý záznam" first
record, may sort descending, and interleaves month-end summary rows into the table. A
positional index would therefore cite a different journey than the one the map belongs to.

The export and the on-screen grid call the same row-assembly helper for exactly this
reason, and a backend test asserts they produce the same record number for the same map.

The synthetic "Prvý záznam" row is skipped: it prints an empty record number, so an
attachment citing it would point at a row that carries no number at all. Month-end rows are
skipped because they are not trips and can hold no map.

### The capability flag

The four route-map commands live only in the dispatchers that serve the HTTP API, and the
capabilities endpoint reports `route_maps: true`. The flag dates from when a second frontend
existed that did not register them; with the browser as the only client it now reads as a
plain "this deployment has route maps".

Export attachment assembly is not a command at all — it is an internal function the export
path calls directly.

### Other choices

- **Why not store the rendered image?** — See
  [ADR-028](../../DECISIONS.md#adr-028-only-the-polyline-is-persisted-tiles-live-in-a-disposable-cache).
  Roughly 10 MB of database growth a year, in every backup, for something recomputable.
- **Why coordinates rather than dataset indices for waypoints?** — See
  [ADR-029](../../DECISIONS.md#adr-029-waypoints-persist-as-coordinates-not-dataset-indices).
  A future manual editor must be able to store a point that is not in the dataset.
- **Why does the backend return decoded coordinates as well as the polyline?** — So the
  frontend needs no polyline decoder of its own, which
  [ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication) rules out.
- **Why no markers on the map?** — The POC established the visual: a single line reads as a
  drive; numbered pins read as a plan.
- **Why loops from a home base, and no origin/destination honouring?** — V1 targets rows
  recording navigation-app testing, where a loop is the correct route shape. Geocoding a
  row's free-text origin and destination is V2, alongside the manual editor.

## Working on this feature

The i18n strings live in [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts) and
[src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts) (Slovak is the source of truth).
Editing a locale file does **not** regenerate
[src/lib/i18n/i18n-types.ts](../../src/lib/i18n/i18n-types.ts) on its own — the generator
otherwise only runs in `vite dev` watch mode, so `npm run check` reports errors for keys that
do exist until the types catch up. Regenerate them explicitly:

```bash
npm run i18n     # typesafe-i18n --no-watch
npm run check
```

Two export labels also sit under the `export` section (`attachmentHeading`,
`recordReference`) and are consumed by the Rust export. They are plain prefixes rather than
`{n}` / `{row}` templates on purpose: typesafe-i18n parses braces as its own interpolation
and has no escape syntax, so a templated label would consume the placeholder before Rust ever
saw it.

## Related

- [ADR-028](../../DECISIONS.md#adr-028-only-the-polyline-is-persisted-tiles-live-in-a-disposable-cache): only the polyline is persisted; tiles live in a disposable cache
- [ADR-029](../../DECISIONS.md#adr-029-waypoints-persist-as-coordinates-not-dataset-indices): waypoints persist as coordinates, not dataset indices
- [ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication): all business logic in Rust
- [ADR-016](../../DECISIONS.md#adr-016-_internal-extraction-pattern-for-command-reuse): the `_internal` command pattern these four commands follow
- [_tasks/70-route-map-integration/](../../_tasks/_done/70-route-map-integration/): requirements, design and implementation plan
- [_tasks/61-route-map-poc/](../../_tasks/_done/61-route-map-poc/): the standalone POC this graduated, and the dataset rationale
- [docs/features/export-system.md](./export-system.md): the printed logbook these pages are appended to
