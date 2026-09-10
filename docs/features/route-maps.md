# Feature: Route Maps

> Draws a road-following map for a trip -- a direct route between its actual origin and destination, or a loop sized to its recorded distance when the two are the same place -- lets the user pick an alternative or drag the line to fix it, and appends saved routes to the printed logbook as attachment pages.

The commands are served over the HTTP API like every other command — see
[The capability flag](#the-capability-flag).

## User Flow

1. **Open the logbook** in a browser (server mode). Every trip row's action cell shows a
   map-pin icon beside the existing insert-above and delete actions. The pin is outlined
   when the trip has no saved map and filled when it has one.
2. **Click the pin.** A new browser tab opens at `/mapa?trip={id}`.
3. **The backend decides the mode from the row's own text**, nothing the user picks: if
   "Odkiaľ" and "Kam" name the same place, the map opens as a **loop** sized to the trip's
   recorded distance (unchanged from V1); otherwise it opens as a **direct route** between
   the two. See [Two modes, chosen in Rust](#two-modes-chosen-in-rust).
4. **A direct route needs both endpoints placed.** Endpoint coordinates come from the place
   book ([Task 75](../../_tasks/_done/75-place-book/)), never from a fresh geocode. If either
   endpoint has no saved coordinate yet, the shared place dialog opens right there on the
   page; saving a pin resumes routing immediately, with no navigation away from the map. See
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
   "Uložiť mapu"** -- the user can retry until a route looks right.
9. **Saving** writes the route, tells the logbook tab to fill in that row's pin, and offers
   to close the map tab. **"Odstrániť mapu"** removes a saved route after confirmation.
   Saving never changes the trip's own recorded distance -- that needs the separate
   **"Použiť vzdialenosť"** action described below. See
   [The recorded distance is written back only explicitly](#the-recorded-distance-is-written-back-only-explicitly).
10. **"Export pre tlač"** appends one A4-landscape page per saved map after the trip table,
    each headed `Príloha č. N — záznam č. X`.

**Failure cases:**

- Origin or destination is blank → routing refuses with a visible error rather than
  falling back to a loop; a trip missing either field is not one the map can draw.
- Either endpoint has no place-book coordinate → the shared place dialog opens instead of
  failing (step 4 above).
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
dialog [the place book's Settings page](./place-book.md) uses -- in place, seeded with the
trip's own origin or destination text. Saving a pin calls `savePlace` and re-runs the
start-of-trip flow, which picks up the new coordinate and either asks for the other endpoint
next or proceeds to route. The map page never writes anywhere else in the place book; it
reuses the dialog's save contract exactly as Settings → Miesta does.

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
book's `savePlace` for the in-place dialog.

### Backend (Rust)

All generation, routing and rasterising is Rust
([ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication)). The
frontend draws a coordinate list and confirms it.

| Module | Responsibility |
|---|---|
| [dataset.rs](../../src-tauri/core/src/route_map/dataset.rs) | Loads the bundled 67-node settlement set and its 67×67 driving-distance matrix |
| [ga.rs](../../src-tauri/core/src/route_map/ga.rs) | Genetic algorithm picking the settlement sequence (Loop mode) |
| [osrm.rs](../../src-tauri/core/src/route_map/osrm.rs) | Fetches road-following geometry and, for a plain two-point request, up to three alternatives -- behind a `RouteProvider` trait |
| [polyline.rs](../../src-tauri/core/src/route_map/polyline.rs) | Polyline5 encode/decode; never panics on malformed input |
| [tiles.rs](../../src-tauri/core/src/route_map/tiles.rs) | Web Mercator tile geometry plus the cache-first tile fetcher |
| [render.rs](../../src-tauri/core/src/route_map/render.rs) | Composites tiles and strokes the route into a PNG |
| [route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs) | The six commands, `mode_for`, waypoint-insertion geometry, plus export attachment assembly |
| [places/normalise.rs](../../src-tauri/core/src/places/normalise.rs) | The text normalisation `mode_for` and the place book both key on |

**Commands are dispatcher-only.** `generate_route` (Loop), `route_direct` (Direct, plus
alternatives and edits) and `route_round_trip` (Direct round trip: two legs, one call each)
all await the routing service, so they live in
[dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs).
`start_route_for_trip`, `get_trip_route`, `save_trip_route`, `save_trip_round_trip_route` and
`delete_trip_route` need no network call -- the place-book lookup is a database read -- and
stay in [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs). `apply_route_distance`
(the distance write-back) lives there too, alongside the other trip-cascade commands in
[trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) rather than
`route_maps.rs` -- it writes the trip, not the map. The write commands are guarded by the
read-only check like every other write.

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
map was saved; after a distance write-back (or any ordinary edit of the row) the trip's
distance moves, and a target that did not follow it would show a deviation against a number
the book no longer holds. The stored column is the fallback only for a map whose trip has
since been deleted.

### Data Flow

Loop mode -- generation and preview, unchanged from V1:

```
Row pin → /mapa?trip=id → start_route_for_trip → mode: loop → generate_route
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
        mode_for(origin, destination) → direct (normalise() differs)
                            ↓
    each endpoint looked up in the place book (no geocode here) ──┐
                            ↓                                     │ missing coordinate
                    route_direct → OSRM /route                    ↓
                            ↓                          PlaceModal opens in place,
       up to 3 alternatives, fastest first,            savePlace, then retry lookup
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
| [src-tauri/core/src/places/normalise.rs](../../src-tauri/core/src/places/normalise.rs) | Text normalisation shared by `mode_for` and the place book |
| [src-tauri/core/src/commands_internal/statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs) | Adds the "which trips have maps" set to the grid data |
| [src-tauri/core/src/export.rs](../../src-tauri/core/src/export.rs) | Attachment page markup and print CSS |
| [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) | `Waypoint`, `RouteMap`, `RouteMode`, `RouteStart` |
| [src-tauri/core/migrations/2026-09-07-110000_add_trip_route_mode/](../../src-tauri/core/migrations/2026-09-07-110000_add_trip_route_mode/) | Adds `trip_routes.mode`, defaulted to `loop` for every pre-existing row |
| [src-tauri/core/migrations/2026-09-07-120000_add_trip_route_round_trip/](../../src-tauri/core/migrations/2026-09-07-120000_add_trip_route_round_trip/) | Adds `trip_routes.round_trip` |
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
compares `normalise(origin)` against `normalise(destination)` -- equal means Loop, unchanged
from V1; different means Direct, a fresh point-to-point route. See
[ADR-037](../../DECISIONS.md#adr-037-the-route-mode-comes-from-the-trips-own-text-decided-in-rust).

The frontend never makes this comparison itself (ADR-008): the map page reads `mode` off
whatever the backend already decided, whether that is a fresh `start_route_for_trip` call or
a saved route's own stored `mode` column. An origin or destination that normalises to empty
is a routing error, not a fallback to Loop -- a trip missing either field would otherwise draw
a plausible-looking map for a journey nobody described.

### Endpoints come from the place book

Direct mode needs coordinates for two free-text strings, and geocoding them on every map open
would be slow, rate-limited, and non-deterministic -- the same place typed identically twice
could resolve to two different pins. [Task 75](../../_tasks/_done/75-place-book/)'s place book
already solves exactly this for a different feature (trip-entry autocomplete), so
`start_route_for_trip` reads from it instead of calling a geocoder: `placed_endpoint` looks up
`normalise(origin)` / `normalise(destination)` in the book and returns nothing when no human
has confirmed a coordinate for that place yet
([ADR-032](../../DECISIONS.md#adr-032-places-are-placed-by-a-human-never-by-a-confidence-heuristic)).

A missing endpoint is not a routing failure. The map page opens the shared
[PlaceModal](../../src/lib/components/PlaceModal.svelte) -- the same dialog
[Settings → Miesta](./place-book.md) uses -- seeded with the trip's own text, right on the map
page. Saving a pin writes to the same place book Settings edits, so placing an endpoint from
the map also fixes every other trip already using that spelling.

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

This doubles as the interim answer to a deferred limitation: re-anchoring the genetic
algorithm at an arbitrary point -- so a distant "Bratislava -- Bratislava" loop draws around the
right town instead of the home base -- needs a distance matrix the app does not have. Until
that exists, dragging a mis-anchored loop into shape is the escape hatch, and it needed no new
mechanism: Direct mode's own editing needed exactly this already.

### The recorded distance is written back only explicitly

See [ADR-039](../../DECISIONS.md#adr-039-distance_km-is-never-rewritten-from-a-routes-road-distance)
(superseded) and [ADR-048](../../DECISIONS.md#adr-048-the-routed-distance-can-be-written-back-behind-the-warning-this-adr-asked-for).
A Direct route's road distance can differ from the trip's logged `distance_km`, sometimes
considerably, and saving the map never writes that number back onto the trip -- reconciling
the two is a decision about the trip, not a side effect of drawing its map. `distance_km`
feeds the consumption rate and the
[20% legal margin](../../DECISIONS.md#biz-003-legal-margin-limit) directly, so silently
moving it would shift which fuel period a fill-up belongs to.

An explicit **"Použiť vzdialenosť"** button offers the write instead, behind a warning. The
first click plans the write -- the odometer cascade (reusing
[ADR-046](../../DECISIONS.md#adr-046-a-save-cascades-the-odometer-by-delta-a-rebase-never-runs-on-its-own)'s
planner) and the consumption-period impact -- and shows it in a confirmation modal: the new
distance, the period's rate and margin before and after, whether the change crosses the 20%
legal limit, and every later row whose odometer moves. Nothing is written until the user
confirms. Only Direct mode gets the button: a Loop route's road distance is the genetic
algorithm's own approximation of the trip's recorded distance, so writing it back would be
circular.

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

- [ADR-047](../../DECISIONS.md#adr-047-a-round-trip-is-two-routing-requests-one-per-leg): a round trip is two routing requests, one per leg
- [ADR-048](../../DECISIONS.md#adr-048-the-routed-distance-can-be-written-back-behind-the-warning-this-adr-asked-for): the routed distance can be written back, behind the warning this ADR asked for
- [ADR-037](../../DECISIONS.md#adr-037-the-route-mode-comes-from-the-trips-own-text-decided-in-rust): the route mode comes from the trip's own text, decided in Rust
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
