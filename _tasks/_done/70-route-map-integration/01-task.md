**Date:** 2026-08-10
**Subject:** Route map integration — per-trip generated route maps, printed as export attachments
**Status:** Planning

## Background

[Task 61](../61-route-map-poc/) built a standalone POC ([poc.html](../61-route-map-poc/poc.html)):
given a target km, a genetic algorithm picks waypoints from a 67-node Slovak
settlement matrix, OSRM returns a road-following polyline, and Leaflet renders it
as a marker-free map suitable for screenshotting. The POC is deliberately
app-external — no DB, no Tauri commands, no export integration.

This task graduates the POC into the app. A trip row gains an action that opens a
map view, generates a route matching that trip's `distance_km`, and — on explicit
confirmation — saves it. "Export for print" then appends one map page per saved
route to the printed logbook, each referencing its trip by row number.

First run targets rows recording *testovanie navigačnej aplikácie* trips, where a
loop from home base is the correct route shape. Non-loop routes (honouring a row's
origin and destination) are out of scope until the V2 editor exists.

## Goals

- Per-row action in the trip grid: generate → preview → confirm → save a route map.
- Re-open an existing map to regenerate or remove it.
- Persist only what the export needs: the polyline plus minimal metadata.
- Append one A4-landscape map page per saved route to the print export, each
  referencing the trip by its row number in the printed table.
- Ship to **web/server mode first**, gated behind a capability flag. All backend
  work is mode-agnostic and enabling desktop later is a flag flip plus one
  frontend component.
- Data model must not need reshaping when the V2 editor lands.

## Non-goals

- **No manual route editing** — dragging, inserting or removing waypoints by hand
  is V2. V1 has exactly one route producer: the genetic algorithm.
- **No origin/destination honouring.** Routes are home-base loops sized to the
  trip's `distance_km`. Geocoding a row's free-text origin/destination and
  generating start ≠ end routes is V2, alongside the editor.
- **No multi-session split.** The POC splits targets over `SINGLE_SESSION_MAX_KM`
  into several day-loops; here one trip is one map. A `distance_km` beyond what
  the 67-node pool can reach yields the best attempt with its error % shown.
- **No PNG in the database.** Rendered images live in a disposable cache.
- **No desktop UI in this task** (see capability gate above).

## Requirements

### Trip grid

- Every trip row gains a map action, alongside the existing insert-above and
  delete actions. No purpose filter — the user decides which rows warrant a map.
- The action indicates whether a route is already saved for that row.

### Map view

- Opens in a new browser tab at `/mapa?trip={id}`.
- Target km is the trip's `distance_km`.
- With no saved route, generates one on open; with a saved route, renders it.
- Actions: regenerate, save, remove.
- **Regenerating persists nothing.** Only an explicit save writes to the database,
  so the user can spin until a route looks right.
- Rendering follows the POC: Leaflet, live OSM tiles, polyline only, no markers.

### Export

- Map pages append after the trip table, one per saved route, page-broken.
- Each page carries only `Príloha č. N — záznam č. {row}` and the map. No date,
  no origin/destination, no km — minimum data for the reviewer.
- The main trip table is unchanged; no new column. The reference runs one way:
  attachment → row.

### Storage

- Only the polyline and minimal metadata are persisted (a few KB per trip).
- Rendered PNGs and fetched map tiles live in a **disposable** app-data cache.
  Deleting the cache costs a re-fetch and nothing more — so Move Database,
  the backups folder and [Task 32](../32-portable-csv-backup/)'s portable CSV
  backup all need no changes.

## Constraints

- **[ADR-008](../../DECISIONS.md):** all business logic in Rust. The genetic
  algorithm, route generation and image rendering are backend concerns; the
  frontend displays and confirms.
- Randomness is business logic and belongs in Rust. The generator splits into a
  pure function taking an injected RNG plus a thin wrapper supplying it, so tests
  run deterministically while production stays varied.
- Backend work lives in [core/src/commands_internal/](../../src-tauri/core/src/commands_internal/)
  and dispatches identically over Tauri IPC and `/api/rpc` — see
  [ARCHITECTURE.md](../../ARCHITECTURE.md).
- [OSRM](https://project-osrm.org/) and [OSM](https://www.openstreetmap.org/) tile
  fetching sit behind an injected trait; no test touches the network.
- The [OSM tile usage policy](https://operations.osmfoundation.org/policies/tiles/)
  requires an identifying User-Agent and attribution baked into rendered images.

## Open questions carried into the plan

- Exact canvas dimensions and DPI for the rendered PNG (working assumption
  ~1400×900 px for A4 landscape).
- Which stroking crate — [imageproc](https://crates.io/crates/imageproc) or
  [tiny-skia](https://crates.io/crates/tiny-skia) — alongside
  [image](https://crates.io/crates/image).
- Practical `distance_km` ceiling before the 67-node pool stops reaching target,
  and whether `MAX_STOPS` should rise from the POC's default.

## Source

Brainstormed in conversation 2026-08-10. Design in [02-design.md](./02-design.md).
