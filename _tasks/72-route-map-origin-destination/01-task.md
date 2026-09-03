**Date:** 2026-09-03
**Subject:** Route maps V2 — honour the row's origin/destination, with alternatives and manual waypoint editing
**Status:** Planning

## Background

[Task 70](../_done/70-route-map-integration/) shipped generated route maps: a genetic
algorithm picks settlements from a bundled 67-node dataset, builds a **loop from a
hardcoded Home Base** sized to the row's `distance_km`, OSRM turns it into a polyline,
and the saved route prints as an export attachment.

That shape is correct for the rows it was built for — *testovanie navigačnej aplikácie*
trips, which really are loops from home. It is wrong for every other row. A trip
recorded as **Bratislava → Spišská** gets a map of a loop around Spiš that never goes
near Bratislava, because the row's own origin and destination are ignored entirely.

Task 70 deferred this deliberately and by name:

> - Manual waypoint editing (drag, insert, remove) in the map view.
> - Honouring a row's origin and destination — start ≠ end routes, geocoding free-text place names.
>
> — [70-route-map-integration/03-plan.md](../_done/70-route-map-integration/03-plan.md), "Deferred to V2"

This task is that V2.

## Goals

- **Route the row that was actually recorded.** When `origin ≠ destination`, geocode
  both and route A→B. When `origin == destination`, keep today's genetic-algorithm
  loop, unchanged.
- **Resolve free-text place names once, then never again.** A row may say "Spisska",
  not "Spišská Nová Ves". The user confirms the match once; the app remembers it for
  every later trip naming the same place.
- **Propose alternatives** the way a navigation app does — fastest first — each labelled
  with its own distance, duration, and deviation from the recorded `distance_km`.
- **Let the user correct the route by hand.** Drag the line onto the road actually
  driven; drag or remove waypoints. Editing works on *any* saved route, loop or direct.
- **Change nothing about the printed export.** Attachment pages stay a bare stroked
  line with no text and no markers.

## Non-goals

- **No write-back to `distance_km`.** A deviation is flagged and shown; reconciling it
  is the user's decision, made by editing the route or the row. Writing the road
  distance into the trip would shift the odometer and every downstream consumption
  calculation.
- **No shared route geometry across trips.** One map per trip, exactly as today. Two
  trips on the same pair are curated independently and may legitimately differ (a
  detour taken on one day only). *Geocoded place coordinates are the exception —
  those are shared globally, which is the whole point of the alias table.*
- **No sorting alternatives by distance fit.** OSRM returns them fastest-first; that
  order is preserved. Deviation is shown per alternative, not used to reorder.
- **No Settings UI for the place-alias book.** A wrong pick is corrected in the map
  view — re-pick from the candidate list, or drag the endpoint.
- **No desktop UI.** Still web/server mode only, behind the existing `routeMaps`
  capability flag. No new flag.
- **No re-anchoring the genetic algorithm.** See Known limitations.

## Requirements

### Mode selection

- `normalise(origin) == normalise(destination)` → **loop mode**, today's behaviour,
  bit-for-bit unchanged.
- Otherwise → **direct mode**, A→B through the geocoded endpoints.
- Automatic. No toggle in the UI.
- An empty origin or destination is an error the user can see, not a silent fallback
  to a loop.

### Geocoding

- Free text → coordinates via a geocoding service behind an injected trait, so no test
  touches the network (mirrors `RouteProvider` and `TileFetcher`).
- **Cache hit** (the normalised string is already in the alias table) → resolve
  instantly, no network call.
- **Cache miss** → show up to 5 candidates and let the user pick. The pick is
  remembered. Nothing is written until the user chooses.
- **No candidates** → the user places the pin on the map by hand, and that is
  remembered too.
- Normalisation (lowercase, strip diacritics, collapse whitespace) is the cache key,
  so "Spisska", "spisska" and "Spišská" are one entry.

### Map view

- Direct mode draws the fastest route active and up to two alternatives as inactive
  lines, clickable to promote.
- Each route in the list shows road km, duration, and its signed deviation from the
  row's `distance_km`, flagged when it exceeds tolerance.
- **Editing, in both modes:** endpoints and waypoints are draggable; pressing the
  active line creates a new waypoint; clicking a waypoint removes it.
- **Re-routing happens on drop only.** Nothing is requested from the routing service
  while the pointer is down.
- Alternatives are only available for a two-point route. Once a waypoint exists the
  panel says so explicitly rather than silently going empty.
- Regenerating and editing still **persist nothing** until an explicit save — Task 70's
  rule survives intact.

### Storage

- Saved routes record which mode produced them, so re-opening one offers the right
  actions.
- Waypoints keep Task 70's coordinate shape
  ([ADR-029](../../DECISIONS.md#adr-029-waypoints-persist-as-coordinates-not-dataset-indices)) —
  it was chosen for exactly this feature and needs no reshaping.
- Existing saved loop routes keep rendering, with no user-visible change.

### Export

- Untouched. No new column, no text, no markers on the rendered page.

## Constraints

- **[ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication):**
  geocoding, routing, mode selection, normalisation and deviation are Rust. The
  frontend draws coordinates and confirms them. In particular the frontend must not
  grow its own notion of "close enough" — deviation and its tolerance keep the single
  home Task 70 gave them.
- Commands live in [core/src/commands_internal/](../../src-tauri/core/src/commands_internal/)
  and dispatch identically over Tauri IPC and `/api/rpc`.
- Write commands are guarded by `check_read_only!`.
- The geocoding service's usage policy is binding: an identifying User-Agent and a
  request-rate cap, the same discipline
  [tiles.rs](../../src-tauri/core/src/route_map/tiles.rs) already applies to OSM tiles.
- All user-facing strings go through i18n, Slovak as the source of truth. Remember
  `npm run i18n` after editing a locale file.

## Known limitations (accepted)

- **A distant A–A row still loops from Home Base.** "Bratislava – Bratislava" produces
  a loop around Spiš, because the genetic algorithm's 67-node distance matrix is
  anchored at home and covers roughly a 50 km radius. Re-anchoring the GA at an
  arbitrary geocoded point would need a distance matrix the app does not have.
  **Mitigation:** the manual editor is deliberately mode-agnostic, so such a route can
  be dragged into shape rather than being stuck.
- **Below ~30 km the dataset still floors out** in loop mode — unchanged from Task 70,
  and unrelated to direct mode, which routes real roads at any distance.

## Open questions carried into the design

- Whether the geocoder returns usable results for heavily abbreviated Slovak place
  names, and whether the candidate picker needs to appear more often than "on cache
  miss" as a result.
- Whether OSRM's public server returns alternatives reliably enough to be worth
  showing, given it never guarantees them.

## Source

Brainstormed in conversation 2026-09-03. Design in [02-design.md](./02-design.md).
