# Feature: Route Maps

> Draws a road-following map for a trip -- a direct route between its actual origin and destination, or a loop sized to its recorded distance when the two are the same place -- lets the user pick an alternative or drag the line to fix it, and appends saved routes to the printed logbook as attachment pages.

The commands are served over the HTTP API like every other command — see
[The capability flag](#the-capability-flag).

## User Flow

1. **Open the logbook** in a browser (server mode). Every trip row's action cell shows a
   map-pin icon beside the existing insert-above and delete actions. The pin is outlined
   when the trip has no saved map and filled when it has one.
2. **Click the pin.** A new browser tab opens at `/mapa?trip={id}`.
3. **The backend decides the mode from the row's own places.** The user picks nothing.
   If "Odkiaľ" and "Kam" have the same place ID, the map opens as a **loop** sized to the
   trip's recorded distance (unchanged from V1). Otherwise it opens as a **direct route**
   between the two. See [Two modes, chosen in Rust](#two-modes-chosen-in-rust).
4. **A direct route needs both endpoints placed.** Endpoint coordinates come from the place
   book ([Task 75](../../_tasks/_done/75-place-book/)), found by the trip's
   `origin_place_id` and `destination_place_id`. The app never geocodes them again.
   A place from an old database can have no position. If an endpoint has none, the shared
   place dialog opens on the page. Saving a pin resumes routing at once. See
   [Endpoints come from the place book](#endpoints-come-from-the-place-book).
5. **A direct route offers alternatives** when a leg has exactly two points: a row of up to
   three options, fastest first, each labelled with its deviation from the trip's recorded
   distance. A one-way route has one leg and one picker; a round trip has two, one per leg,
   and each picks independently. Picking one redraws the map without re-fetching. See
   [Alternatives are ordered by duration, never by deviation](#alternatives-are-ordered-by-duration-never-by-deviation).
6. **Dragging the line** inserts a new stop and re-routes through it on release, in both
   modes -- the same mechanism that lets a mis-anchored loop be corrected also lets a direct
   route pick up a real via point. See
   [The waypoint editor doesn't know which mode drew the line](#the-waypoint-editor-doesnt-know-which-mode-drew-the-line).
7. **A direct route can be a round trip.** Ticking "Cesta tam a späť" routes the outbound leg
   and the return leg as two independent requests, so the way home can take a different road
   than the way out; the flag is saved with the route so reopening it restores the checkbox.
   The outbound line is drawn blue, the return line amber. See
   [A round trip is two routing requests, one per leg](#a-round-trip-is-two-routing-requests-one-per-leg).
8. **"Generovať znova" / "Prepočítať"** produces a different route for the same target
   (loop) or re-routes the current waypoint list (direct). **Nothing is persisted until
   "Uložiť a použiť vzdialenosť"** -- the user can retry until a route looks right.
9. **"Uložiť a použiť vzdialenosť"** saves the route and writes its distance, in whole km,
   to the trip. It works the same in every mode. If the trip km changes, a confirmation
   modal shows the change first. The save tells the logbook tab to fill in that row's pin,
   and offers to close the map tab. **"Odstrániť mapu"** removes a saved route after
   confirmation. It does not change the trip km. See
   [A map save writes its distance to the trip](#a-map-save-writes-its-distance-to-the-trip).
10. **"Export pre tlač"** appends one A4-landscape page per saved map after the trip table,
    each headed `Príloha č. N — záznam č. X`.

**Failure cases:**

- Either endpoint place has no position (a legacy place): the shared place dialog opens
  instead of failing (step 4 above). A trip always has both places, so a blank endpoint
  cannot occur any more.
- The loop generator cannot reach the target within tolerance → the best attempt is drawn
  and the deviation percentage is flagged. See
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
nothing" structural rather than a rule someone has to remember. `mode` itself is state the
page only ever reads off a backend response (`start_route_for_trip`, a saved route, or the
result of an edit) -- it is never derived here from the trip's origin and destination.

The page also holds the direct-route pieces: `baseWaypoints` (the open, un-closed outbound
waypoint list an edit or a round-trip toggle re-routes from), `alternatives` and
`activeIndex` (the backend's ordered list and which one is on screen, one-way mode only),
`roundTrip` (mirrors the persisted flag), and `unplacedField` (which endpoint, if any, has no
place-book coordinate yet -- this drives the shared place dialog described below).

Round-trip mode adds its own pieces, live only while `roundTripRoutes` is non-null:
`roundTripRoutes` itself (both legs as the backend last normalised them, with each leg's own
alternatives and the `combined[i][j]` table), `outboundIndex` / `inboundIndex` (which
alternative is chosen per leg, independent of each other), and `baseInbound` (the return
leg's open waypoint list -- the counterpart to `baseWaypoints`, which holds the outbound leg
in this mode). `savedLegs` holds a saved round trip already split into its two legs, and acts
as the fallback `baseWaypoints`/`baseInbound` fall back to before a live re-route has run.
`savedLegGeometry` is its counterpart for the LINES rather than the stops: the two leg
geometries the backend split out of the stored polyline, non-null only while the saved row
is what the map shows (no `generated` proposal, no live `roundTripRoutes`, the checkbox
still ticked). It is what lets a re-opened round trip draw in its two colours, hand both
legs their handles, and place a dragged-in stop against the right leg -- all before any
routing call. It carries no distance or duration, so the leg pickers stay empty until a
real routing run fills them.
`writeback` holds the dry-run plan awaiting the user's confirmation for the distance
write-back described below; nothing is written while it is null.

**Endpoint placement:** when `start_route_for_trip` reports an endpoint with no coordinate,
the page renders [PlaceModal.svelte](../../src/lib/components/PlaceModal.svelte) -- the same
dialog [the Miesta tab](./place-book.md) uses -- in place, for the trip's origin or
destination place. Saving a pin calls `setPlacePosition` with the place ID and re-runs the
start-of-trip flow, which picks up the new coordinate and either asks for the other endpoint
next or proceeds to route. The map page never writes anywhere else in the place book; it
reuses the dialog's save contract exactly as the Miesta tab does.

**Alternatives and editing:** picking a row in the alternatives list only swaps which
already-fetched `GeneratedRoute` is drawn -- no new request. Dragging the line calls the
same `reroute()` path a checkbox toggle or a "Prepočítať" click does: it always re-fetches
through `route_direct`, because an edited position can only be resolved by asking the
routing service again. A drag on a loop's line still works, and flips `mode` to `'direct'`
the moment it resolves -- see
[The waypoint editor doesn't know which mode drew the line](#the-waypoint-editor-doesnt-know-which-mode-drew-the-line).

**Row action:** [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte)
renders the pin, gated on the capability flag.
[src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte) owns the set
of trips that have maps, opens the map tab, and listens on a `BroadcastChannel` so a save or
a removal in the map tab updates the row icon without a reload.

**API wrappers:** [src/lib/api.ts](../../src/lib/api.ts) -- `generateRoute`, `routeDirect`,
`startRouteForTrip`, `getTripRoute`, `saveTripRoute`, `deleteTripRoute`, plus the place
book's `setPlacePosition` for the in-place dialog.

### Backend (Rust)

All generation, routing and rasterising is Rust
([ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication)). The
frontend draws a coordinate list and confirms it.

| Module | Responsibility |
|---|---|
| [dataset.rs](../../src-tauri/core/src/route_map/dataset.rs) | Loads the bundled 67-node settlement set and its 67×67 driving-distance matrix, and the Bratislava district set (task 91) |
| [areas.rs](../../src-tauri/core/src/route_map/areas.rs) | Selects the loop candidate set from the trip's place: home, Bratislava, or none (task 91) |
| [ga.rs](../../src-tauri/core/src/route_map/ga.rs) | Genetic algorithm picking the settlement sequence (Loop mode) |
| [osrm.rs](../../src-tauri/core/src/route_map/osrm.rs) | Fetches road-following geometry and, for a plain two-point request, up to three alternatives -- behind a `RouteProvider` trait |
| [sygic.rs](../../src-tauri/core/src/route_map/sygic.rs) | Sygic Routing API v3 client. Same trait as OSRM; adds `avoid` and returns the avoid options |
| [provider.rs](../../src-tauri/core/src/route_map/provider.rs) | Picks the provider: the environment decides what exists and the default (mock, Sygic or OSRM), the request can pick one of them (Task 86) |
| [avoid.rs](../../src-tauri/core/src/route_map/avoid.rs) | Validates the avoid values and builds the option union |
| [polyline.rs](../../src-tauri/core/src/route_map/polyline.rs) | Polyline5 encode/decode; never panics on malformed input |
| [tiles.rs](../../src-tauri/core/src/route_map/tiles.rs) | Web Mercator tile geometry plus the cache-first tile fetcher |
| [render.rs](../../src-tauri/core/src/route_map/render.rs) | Composites tiles and strokes the route into a PNG |
| [route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs) | The six commands, `mode_for`, waypoint-insertion geometry, plus export attachment assembly |
| [places/normalise.rs](../../src-tauri/core/src/places/normalise.rs) | The text normalisation the place book keys on (`mode_for` compares place IDs and does not use it) |

**Commands are dispatcher-only.** `generate_route` (Loop), `route_direct` (Direct, plus
alternatives and edits) and `route_round_trip` (Direct round trip: two legs, one call each)
all await the routing service, so they live in
[dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs).
`start_route_for_trip`, `get_trip_route`, `save_trip_route`, `save_trip_round_trip_route` and
`delete_trip_route` need no network call -- the place-book lookup is a database read -- and
stay in [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs). The two saves take a
required `dryRun` and return a `DistanceWriteback`: a save also writes the trip km (task 87).
`apply_saved_route_distance` (sync of a saved map) lives there too. Its planner,
`plan_route_distance`, is in [trips.rs](../../src-tauri/core/src/commands_internal/trips.rs)
with the other trip-cascade code, and the saves call the same planner. A dry run writes
nothing and also works in read-only mode. A commit is guarded by the read-only check like
every other write.

**Storage:** the `trip_routes` table
([migration](../../src-tauri/core/migrations/2026-08-10-100000_add_trip_routes/)), keyed by
`trip_id` with `ON DELETE CASCADE`. It holds the waypoints (JSON), the encoded polyline, the
target and road distances, the dataset version, a `mode`
([migration](../../src-tauri/core/migrations/2026-09-07-110000_add_trip_route_mode/)), a
`round_trip` flag
([migration](../../src-tauri/core/migrations/2026-09-07-120000_add_trip_route_round_trip/)),
a `turnaround_index`
([migration](../../src-tauri/core/migrations/2026-09-09-100000_add_trip_route_turnaround_index/))
and a timestamp -- a few KB per trip. No image is stored anywhere; see
[ADR-028](../../DECISIONS.md#adr-028-only-the-polyline-is-persisted-tiles-live-in-a-disposable-cache).
`round_trip` is meaningful for Direct mode only -- a saved loop is always stored `false`,
since a loop is already closed by construction.
A saved map with `round_trip = true` also makes the trip a day trip in the journey
grouping (the leg goes there and back on one row). See
[mcp-endpoint.md](./mcp-endpoint.md).

`turnaround_index` is where, in `waypoints`, the outbound leg ends and the return leg
begins -- meaningful for a round trip only. `NULL` means "split at `length - 2`": every round
trip saved before this column existed was closed by appending exactly one clone of the first
waypoint, so `[A, ...vias, B, A]` splits unambiguously at that position, and legacy rows need
no backfill. A value written by the new save path means "split here" exactly, because the two
legs can carry a via each and their combined length is no longer a fixed offset from the end.
`get_trip_route` resolves both cases before the frontend ever sees the row, so the browser
only ever slices a list -- it never decides where the split falls.

**The geometry is split on the way out too.** A round trip stores one polyline, and
`get_trip_route` returns it split at the turnaround as `legs.outbound` / `legs.inbound` --
each an encoded polyline plus its decoded coordinates, overlapping by one point exactly like
the waypoint lists above. The seam is found by searching the decoded line for the point
nearest `waypoints[turnaround_index]`, not by a stored offset: `save_trip_round_trip_route`
concatenates the two legs' polylines, so the turnaround sits in the line twice and the search
lands on it, while a legacy row is one continuous routing result whose seam can only be found
this way at all. `legs` is `null` for a one-way route and a loop. See
[ADR-049](../../DECISIONS.md#adr-049-a-saved-round-trips-geometry-is-split-back-into-legs-in-rust-and-the-split-point-is-derived).

The `target_km` a saved map reports comes from the **trip's own `distance_km` at read time**,
not from the column stored above. The stored column records what the trip measured when the
map was saved; after an edit of the row the trip's
distance moves, and a target that did not follow it would show a deviation against a number
the book no longer holds. The stored column is the fallback only for a map whose trip has
since been deleted.

### Data Flow

Loop mode -- generation and preview, unchanged from V1:

```
Row pin → /mapa?trip=id → start_route_for_trip → mode: loop → generate_route(tripId)
                            ↓
       trip's origin place → areas.rs: home set | Bratislava set + OSRM /table | error
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

Direct mode -- endpoint lookup, alternatives, and editing:

```
Row pin → /mapa?trip=id → start_route_for_trip
                            ↓
        mode_for(trip) -> direct (the two place IDs differ)
                            ↓
    each endpoint looked up in the place book (no geocode here) ──┐
                            ↓                                     │ missing coordinate
                    route_direct → OSRM /route                    ↓
                            ↓                          PlaceModal opens in place,
       up to 3 alternatives, fastest first,            setPlacePosition, then retry lookup
       each labelled with its deviation %                        │
                            ↓ ←─────────────────────────────────┘
       Leaflet draws the active alternative
                            ↓
    user drags the line (insert_waypoint) or ticks "Cesta tam a späť"
                            ↓
                 route_direct again, on release · never on drag
                            ↓
                    save_trip_route → trip_routes (mode, round_trip)
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
| [src/routes/mapa/+page.svelte](../../src/routes/mapa/+page.svelte) | Map view: mode-agnostic preview, alternatives, drag editing, round trip, save, remove |
| [src/lib/components/PlaceModal.svelte](../../src/lib/components/PlaceModal.svelte) | Shared place dialog; opened in place when an endpoint has no coordinate |
| [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte) | Map-pin row action, filled when a map is saved |
| [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte) | Opens the map tab; keeps pin state fresh over `BroadcastChannel` |
| [src-tauri/core/src/route_map/](../../src-tauri/core/src/route_map/) | Dataset, genetic algorithm, OSRM client (fetch + alternatives), polyline codec, tiles, rasteriser |
| [src-tauri/core/src/commands_internal/route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs) | The six commands, `mode_for`, `insert_waypoint`, round-trip normalisation, export attachment assembly |
| [src-tauri/core/src/places/normalise.rs](../../src-tauri/core/src/places/normalise.rs) | Text normalisation the place book keys on; `mode_for` compares place IDs |
| [src-tauri/core/src/commands_internal/statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs) | Adds the "which trips have maps" set to the grid data |
| [src-tauri/core/src/export.rs](../../src-tauri/core/src/export.rs) | Attachment page markup and print CSS |
| [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) | `Waypoint`, `RouteMap`, `RouteMode`, `RouteStart` |
| [src-tauri/core/migrations/2026-09-07-110000_add_trip_route_mode/](../../src-tauri/core/migrations/2026-09-07-110000_add_trip_route_mode/) | Adds `trip_routes.mode`, defaulted to `loop` for every pre-existing row |
| [src-tauri/core/migrations/2026-09-07-120000_add_trip_route_round_trip/](../../src-tauri/core/migrations/2026-09-07-120000_add_trip_route_round_trip/) | Adds `trip_routes.round_trip` |
| [src-tauri/core/assets/](../../src-tauri/core/assets/) | Bundled 67-node dataset and distance matrix, and the 17 Bratislava districts (`bratislava.json`) |

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
The Bratislava set (17 districts) has the same limit: targets below about 10 km can miss
tolerance. There is no algorithmic fix short of a denser dataset, so the map view **always shows the
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

The six route-map commands live only in the dispatchers that serve the HTTP API, and the
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

### Two modes, chosen in Rust

V1 always drew a loop from a home base, which matched the navigation-app test trips it was
built against, where a loop is the correct route shape. [Task 72](../../_tasks/_done/72-route-map-origin-destination/)
replaces that blanket rule: `mode_for` ([route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs))
compares `origin_place_id` against `destination_place_id` -- equal means Loop, unchanged
from V1; different means Direct, a fresh point-to-point route. Since
[task 88](../../_tasks/_done/88-places-as-entities/) the comparison is an ID equality. No string
fold is involved. See
[ADR-037](../../DECISIONS.md#adr-037-the-route-mode-comes-from-the-trips-own-text-decided-in-rust).

The frontend never makes this comparison itself (ADR-008): the map page reads `mode` off
whatever the backend already decided, whether that is a fresh `start_route_for_trip` call or
a saved route's own stored `mode` column. A trip always has both places,
so a blank endpoint cannot reach the router. The migration maps a blank endpoint to the
place `Neznáme miesto`.

### Endpoints come from the place book

Direct mode needs coordinates for two places, and geocoding them on every map open
would be slow, rate-limited, and non-deterministic -- the same place geocoded twice
could resolve to two different pins. [Task 75](../../_tasks/_done/75-place-book/)'s place book
already solves exactly this for a different feature (trip-entry autocomplete), so
`start_route_for_trip` reads from it instead of calling a geocoder: `placed_endpoint` finds
`origin_place_id` / `destination_place_id` in the book and returns nothing when no human
has confirmed a position for that place yet (only a legacy place from the migration can lack one)
([ADR-032](../../DECISIONS.md#adr-032-places-are-placed-by-a-human-never-by-a-confidence-heuristic)).

A missing endpoint is not a routing failure. The map page opens the shared
[PlaceModal](../../src/lib/components/PlaceModal.svelte) -- the same dialog
[Miesta tab](./place-book.md) uses -- right on the map
page. Saving a pin writes to the same place the Miesta tab edits, so placing an endpoint from
the map also fixes every other trip that uses the place.

### Alternatives are ordered by duration, never by deviation

See [ADR-038](../../DECISIONS.md#adr-038-alternatives-are-ordered-by-duration-deviation-labels-never-reorders).
OSRM's own fastest-first order is preserved exactly; the deviation percentage each alternative
carries is a label, not a sort key, so the option a driver would actually take is never
displaced by the option that happens to match the trip's logged kilometres.

Alternatives exist only for a plain two-point route -- OSRM ignores the `alternatives`
parameter once a via point is in play and returns a single through-route regardless. Any
route with an inserted via, and a round trip's own closing leg, therefore shows
`alternativesUnavailable` instead of a list.

### The waypoint editor doesn't know which mode drew the line

See [ADR-040](../../DECISIONS.md#adr-040-the-waypoint-editor-is-mode-agnostic).
`insert_waypoint` and the drag-to-edit flow operate on an ordered `{lat, lon}` list and a
polyline, with no idea whether the genetic algorithm or a pair of geocoded endpoints produced
either one. Dragging the line always ends by calling `route_direct`, which is also why an
edited loop becomes a direct route: the moment a drag decides the shape, the result is a
concrete road route, not a synthetic GA loop.

Every via handle shows its number in the waypoint list (task 91), so the order stays
visible while the user drags. The start and end point has no number. On a round trip the
return leg continues the numbers of the way out. The numbers are only display: the order is
the backend's.

### A loop starts at the trip's place

See [ADR-059](../../DECISIONS.md#adr-059-a-loop-starts-at-the-trips-place-candidate-sets-are-per-area)
and [Task 91](../../_tasks/91-loop-generator-bratislava/01-task.md). Until task 91, the
genetic algorithm always started at home node 0, so a "Bratislava -- Bratislava" loop was
drawn around the home town. Now `generate_route` takes the `tripId`, and the backend reads
the position of the trip's origin place. [areas.rs](../../src-tauri/core/src/route_map/areas.rs)
selects the candidate set from that position:

| Anchor | Candidate set | Matrix |
|---|---|---|
| <= 5 km from home node 0 | the bundled 67-node home set | bundled, offline |
| <= 18 km from the Bratislava centre | the anchor at index 0 + the 17 city districts | one OSRM `/table` call |
| any other place | none: error `NO_LOOP_CANDIDATES` | - |

The anchor itself is index 0 of the Bratislava set, so the loop starts and ends at the place
in the logbook, not at a district centre. The matrix always comes from OSRM, also when Sygic
is selected; the final `/route` call uses the selected provider. If the place has no
position, the page opens the place dialog before it asks to generate. A new area is a new
candidate file in [assets/](../../src-tauri/core/assets/) plus one branch in `loop_area`.

### A map save writes its distance to the trip

See [ADR-054](../../DECISIONS.md#adr-054-a-saved-map-always-writes-its-whole-km-distance-to-the-trip),
[ADR-048](../../DECISIONS.md#adr-048-the-routed-distance-can-be-written-back-behind-the-warning-this-adr-asked-for)
(superseded in part) and [ADR-039](../../DECISIONS.md#adr-039-distance_km-is-never-rewritten-from-a-routes-road-distance)
(superseded). If a trip has a saved map, the trip's `distance_km` agrees with the map.

The page has one button, **"Uložiť a použiť vzdialenosť"**. It saves the route and writes
the route's distance to the trip in one step. It does the same thing in every mode: Direct
one-way, Direct round trip ("Cesta tam a späť") and Loop.

- **Whole km.** The trip gets `road_km.round()`: 25.634 km becomes 26 km. The map keeps the
  raw `road_km`, so it still shows the real road distance. On commit, `trip_routes.target_km`
  is the new trip km.
- **Dry run first.** The click sends the save with `dryRun: true`. It plans the odometer
  cascade (the planner of
  [ADR-046](../../DECISIONS.md#adr-046-a-save-cascades-the-odometer-by-delta-a-rebase-never-runs-on-its-own))
  and the consumption-period impact, and it writes nothing, not even the map.
- **The modal.** If the trip km changes, the confirmation modal shows the new km, the
  period's rate and margin before and after, the 20% legal limit, and every later row whose
  odometer moves. `distance_km` feeds the consumption rate and the
  [20% legal margin](../../DECISIONS.md#biz-003-legal-margin-limit), so the user approves
  the numbers first.
- **No modal on a no-op.** If the rounded km already equals the trip km, the backend reports
  `changesTrip: false` and the page saves at once.
- **One transaction.** Confirm sends the same save with `dryRun: false`. The backend plans
  again from the stored book, then writes the map row, the trip row and the odometer shifts
  in one transaction (`save_route_map_with_trip_distance`). Cancel writes nothing. The page
  keeps a copy of the save call from the click, so a different alternative picked behind
  the modal does not change what Confirm commits.
- **Sync of a saved map.** A map saved before task 87, or a trip whose km was edited after
  the save, is not in sync. `SavedRouteMap.distanceInSync` reports this (a backend fact). The
  button is then enabled with no proposal, and it calls `apply_saved_route_distance`. The
  backend reads the road km from `trip_routes`.
- **No false warning.** A map in sync is never `offTarget`. Without this rule, 2.4 km written
  as 2 km is a 20% deviation, over the 5% tolerance, and the warning could never clear.
  `deviationPercent` still shows the real value.

The button is enabled if there is an unsaved proposal, or if the saved map is not in sync.
"Odstrániť mapu" removes the map and does not change the trip km.

### A round trip is two routing requests, one per leg

See [ADR-047](../../DECISIONS.md#adr-047-a-round-trip-is-two-routing-requests-one-per-leg).
A round trip is two independent OSRM requests, A-to-B and B-to-A, not one
`[origin, destination, origin]` call. That follows from what OSRM offers: alternatives only
exist for a two-point request, so a single three-point call could never have offered the
return leg a different road anyway, or offered a choice on either leg. Splitting the request
fixes both: each leg gets its own picker, fastest first, and the way home can legitimately
take a different road than the way out. `alternativesUnavailable` now appears only for a leg
that genuinely passes through a via -- a plain round trip, with no via on either leg, no
longer triggers it.

The saved row is still one row: the backend joins the two legs at their shared turnaround
point, concatenates the geometry and sums the distances, and records where the join happened
(`turnaround_index`, below) so reopening the map can split it back into its two legs
correctly.

### Avoid paid roads per country

See [ADR-052](../../DECISIONS.md#adr-052-sygic-is-the-optional-routing-provider-for-per-country-toll-avoidance) and [the task](../../_tasks/_done/85-route-avoid-tolls-per-country/01-task.md).

On `/mapa`, in direct mode only (one-way and round trip), one checkbox per country shows: "Vyhnúť sa spoplatneným cestám: SK / CZ ...". A click routes again with the new list. Loop mode has no checkboxes and sends no avoid list. A change of the list would run the genetic algorithm again and replace the loop.

**Provider choice** ([provider.rs](../../src-tauri/core/src/route_map/provider.rs)). The environment decides which providers exist. The first rule that matches wins:

1. `KNIHA_JAZD_MOCK_ROUTER` is set: the offline mock, which offers both. For the integration tests only.
2. `SYGIC_API_KEY` is set: OSRM and Sygic. If `SYGIC_REFERER` is set, a Sygic request sends it as the `Referer` header.
3. Otherwise: public OSRM only.

The default is **OSRM in every case**. Sygic is used only when the request asks for it (see "Provider per request" below).

OSRM refuses a non-empty avoid list. There is no silent fallback: a fallback route would ignore the avoid list and look correct. A saved route with an avoid list still draws when the key is gone, because it needs no routing call. A recompute shows an error. The page matches the marker `AVOID_NEEDS_SYGIC` at the start of that message. It then shows `routeMap.avoidNeedsSygic` with no Retry button, because a retry cannot succeed without the key.

**Provider per request** ([ADR-053](../../DECISIONS.md#adr-053-the-page-picks-the-routing-provider-per-request-the-server-decides-what-exists), [Task 86](../../_tasks/_done/86-route-provider-switch/01-task.md)). The rules above give what EXISTS and the default. The page can pick one of them per request:

- `get_route_providers` returns `{ available, default }`. The page shows the select "Smerovanie: OSRM (OpenStreetMap) / Sygic" only when `available` has two entries.
- `generate_route`, `route_direct` and `route_round_trip` take an optional `provider`. Absent means the default, OSRM. An avoid list with no provider therefore fails with `AVOID_NEEDS_SYGIC`: there is no silent switch to Sygic. `sygic` without a key fails with `PROVIDER_NEEDS_SYGIC`, and the page shows `routeMap.providerNeedsSygic` with no Retry button.
- Every response carries `provider`. The page adopts it for the next request and saves it (`save_trip_route` sends `route.provider`, the round-trip save sends `legs.provider`).
- `trip_routes.provider` (nullable, last column, migration `2026-09-29-110000_add_trip_route_provider`) stores it. `get_trip_route` returns it, and a reopened route selects it. NULL means unknown: `get_trip_route` returns `sygic` if the route has an avoid list (the migration's rule), otherwise NULL, and the page selects the default.
- A switch in direct mode routes again at once. A switch to OSRM clears the avoid list first. A switch in loop mode does not regenerate (a new random loop would replace the drawn one); it applies to the next "Regenerovať".

Why: the Sygic map has no D1 Visnove tunnel, so its Zilina route to Bratislava goes through the Strecno gorge. OSRM (OpenStreetMap) has the tunnel.

**The `tolls`-only rule** ([avoid.rs](../../src-tauri/core/src/route_map/avoid.rs)). The backend accepts only values that match `^[a-z]{3}:tolls$`, for example `cze:tolls`. Any other value is an error, and no request goes out, because the value goes into a URL. Free highway sections stay allowed.

**The option union.** Sygic returns the avoids that apply to the route (`return_possible_avoids=true`). The backend keeps the `*:tolls` values and adds the values in the current avoid list. Sygic does not offer an avoid again after it has been applied, so without this step a checked country would disappear. A round trip returns the sorted union of both legs, and both legs use the same avoid list. The page renders `avoidOptions` and does not compute anything.

**Saved column.** `trip_routes.avoid` holds a JSON list, `TEXT NOT NULL DEFAULT '[]'`, as the last column (migration `2026-09-29-100000_add_trip_route_avoid`). A save stores the list that produced the shown route, not the newest checkbox state. A reopened route shows its saved values as checked, until "Prepočítať". `get_trip_route` returns them as `avoid`.

**The mock.** With `KNIHA_JAZD_MOCK_ROUTER` set, an offline router answers all routing calls. It offers `cze:tolls`, so the integration tests can click a checkbox with no network. It offers both providers, like a keyed server. As OSRM it returns 90.0 km and no avoid options.

**Checked example** (2026-09-29, Bratislava to Brno): 130.1 km and 88 min with no avoid. 132.9 km and 111 min with `cze:tolls`. That route still uses the D2 from the border to the Breclav exit. The section has had no vignette since March 2025 ([source](https://www.novinykraje.cz/2025/03/07/dalnice-d2-na-hranicich-se-slovenskem-je-uz-prujezdna-bez-zpoplatneni/)), and Sygic knows it. The exemption is temporary: it lasts while the II/425 bridge Lanzhot-Brodske stays closed.

**Open items before the deploy:** the Sygic terms on stored geometry, and the request quota of the plan.

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

- [ADR-052](../../DECISIONS.md#adr-052-sygic-is-the-optional-routing-provider-for-per-country-toll-avoidance): Sygic is the optional routing provider for per-country toll avoidance
- [ADR-053](../../DECISIONS.md#adr-053-the-page-picks-the-routing-provider-per-request-the-server-decides-what-exists): the page picks the routing provider per request; the server decides what exists
- [ADR-047](../../DECISIONS.md#adr-047-a-round-trip-is-two-routing-requests-one-per-leg): a round trip is two routing requests, one per leg
- [ADR-048](../../DECISIONS.md#adr-048-the-routed-distance-can-be-written-back-behind-the-warning-this-adr-asked-for): the routed distance can be written back, behind the warning this ADR asked for
- [ADR-037](../../DECISIONS.md#adr-037-the-route-mode-comes-from-the-trips-own-text-decided-in-rust): the route mode comes from the trip's own text, decided in Rust (since task 88 it compares place IDs)
- [ADR-038](../../DECISIONS.md#adr-038-alternatives-are-ordered-by-duration-deviation-labels-never-reorders): alternatives are ordered by duration; deviation labels, never reorders
- [ADR-039](../../DECISIONS.md#adr-039-distance_km-is-never-rewritten-from-a-routes-road-distance): `distance_km` is never rewritten from a route's road distance -- **superseded by ADR-048**
- [ADR-040](../../DECISIONS.md#adr-040-the-waypoint-editor-is-mode-agnostic): the waypoint editor is mode-agnostic
- [ADR-041](../../DECISIONS.md#adr-041-round-trip-normalisation-is-symmetric-and-lives-entirely-in-rust): round-trip normalisation is symmetric, and lives entirely in Rust (the one-way path; unchanged by ADR-047)
- [ADR-046](../../DECISIONS.md#adr-046-a-save-cascades-the-odometer-by-delta-a-rebase-never-runs-on-its-own): a save cascades the odometer by delta -- the planner the distance write-back reuses
- [ADR-032](../../DECISIONS.md#adr-032-places-are-placed-by-a-human-never-by-a-confidence-heuristic): places are placed by a human, never by a confidence heuristic
- [ADR-028](../../DECISIONS.md#adr-028-only-the-polyline-is-persisted-tiles-live-in-a-disposable-cache): only the polyline is persisted; tiles live in a disposable cache
- [ADR-029](../../DECISIONS.md#adr-029-waypoints-persist-as-coordinates-not-dataset-indices): waypoints persist as coordinates, not dataset indices
- [ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication): all business logic in Rust
- [ADR-016](../../DECISIONS.md#adr-016-_internal-extraction-pattern-for-command-reuse): the `_internal` command pattern these commands follow
- [_tasks/78-round-trip-legs-and-distance-writeback/](../../_tasks/_done/78-round-trip-legs-and-distance-writeback/): requirements, design and implementation plan for two-leg round trips and the distance write-back
- [_tasks/_done/72-route-map-origin-destination/](../../_tasks/_done/72-route-map-origin-destination/): requirements, design and implementation plan for origin/destination routing, alternatives, editing and the round trip
- [_tasks/_done/70-route-map-integration/](../../_tasks/_done/70-route-map-integration/): requirements, design and implementation plan
- [_tasks/_done/61-route-map-poc/](../../_tasks/_done/61-route-map-poc/): the standalone POC this graduated, and the dataset rationale
- [docs/features/place-book.md](./place-book.md): the place book Direct-mode endpoints and the shared `PlaceModal` come from
- [docs/features/export-system.md](./export-system.md): the printed logbook these pages are appended to
