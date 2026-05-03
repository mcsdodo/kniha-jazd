**Date:** 2026-05-03
**Subject:** Trip route map generator (POC)
**Status:** Planning

## Background

When a kniha jázd entry needs evidence ("proof of driving") for inspection, the user
needs a map showing the road route taken. Real Google Maps screenshots are awkward:
multi-stop routes lose intermediate waypoints when transcribed to OSM-based viewers,
[GraphHopper Maps](https://graphhopper.com/maps/) shows distracting waypoint pins, and
the recorded `distance_km` on a trip rarely matches the direct road distance between
origin and destination (real trips include unlogged stops, errands, detours).

This POC explores an algorithmic generator: given a target distance, produce a
plausible loop route on real Slovak roads starting and ending at the user's home base.
The output is a clean polyline-only map suitable for screenshotting as evidence.

## Goals

- Generate a road-following polyline whose total distance is within ±5% of a
  target km value, starting and ending at home base.
- Use only real Slovak settlements as intermediate waypoints (no fabricated
  points along roads). Source set: 22 cities/towns within 50 km aerial radius
  (any population), plus 44 villages within 20 km aerial radius (population ≥ 500).
- Render in a self-contained HTML file ([Leaflet](https://leafletjs.com/) +
  [OSM tiles](https://www.openstreetmap.org/), no markers, no controls — just the
  polyline) that can be screenshotted directly.
- A small genetic algorithm produces the waypoint sequence. Its
  per-run randomness is intentional: generating multiple trips at the
  same target km should yield visibly different routes (an inspector
  reading five identical maps notices, five GA-varied maps doesn't).
- Multi-session split: if target exceeds a single-day max (~400 km), split into
  N independent loop-trips, each rendered separately.

## Non-goals

- **Not** integrated into the kniha-jazd app (no DB, no Tauri commands, no PDF
  embedding). POC lives in [this folder](.) only.
- **Not** geocoding origin/destination free-text from existing trips. Origin =
  always home base for the POC.
- **Not** padding *downward* (cannot honestly shorten a real road route). If
  target < 5 km the POC refuses; if target ≤ direct route to nearest village,
  the POC just routes directly with a warning.
- **Not** authenticated, persisted, or shared online. Pure local browser file.

## Home base

Kamenný obrázok 26, 052 01 Spišská Nová Ves-Tarča
GPS: 48.9350604, 20.5533207

## Data files

See [villages.json](./villages.json) for the 67-node list (1 home + 22 cities/towns
+ 44 villages) and [matrix.json](./matrix.json) for the 67×67 driving distance
matrix in km. Both are pre-generated; see [_fetch-matrix.ps1](./_fetch-matrix.ps1)
for the regeneration script.

## Source

Brainstormed in conversation 2026-05-03. The "matrix" idea is the user's — driven
by the observation that GraphHopper's pin clutter and OSM directions' two-waypoint
limitation made manual route generation painful enough to want automation.
