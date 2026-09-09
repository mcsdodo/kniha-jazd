**Date:** 2026-09-08
**Subject:** Route each round-trip leg separately, and let the routed distance be written back to the row
**Status:** In Progress

## Goal

Two changes the user asked for after driving [task 72](../_done/72-route-map-origin-destination/)
on real data:

1. A round trip should be **two routing requests** (A to B, then B to A), not one
   three-point request, so each leg gets its own alternatives and the return can take a
   different road.
2. The routed distance should be **writable back onto the trip row**, behind a warning
   that says what it does to the fuel-consumption buckets and the 20% legal margin.

## Background

Task 72 shipped origin/destination routing. A round trip is currently sent to OSRM as a
single three-point request `[A, B, A]`. Two consequences the user met immediately:

- **No alternatives.** OSRM only offers alternatives for two-point routes
  ([osrm.rs](../../src-tauri/core/src/route_map/osrm.rs), `route_url`), so a round trip
  falls into the "route goes through more than two points" branch and the picker is
  replaced by an unavailable message. Recorded as a known limitation in
  [docs/features/route-maps.md](../../docs/features/route-maps.md).
- **The return leg cannot differ.** OSRM routes A to B to A as one through-route, so the
  way back is whatever that produces. The user's point: the road home is often not the
  road out.

The write-back request came from the same session. Trip
`32631e0e-1ef8-4869-ad1a-75b95109a0dc` records **50.0 km**; the one-way route is
**25.6 km** (deviation -48.9%) and the round trip is **51.5 km** (+3.0%). The row is a
there-and-back written as one line. A second one-way trip checked independently showed
about the same -49%, so this is a recurring shape in this book, not one odd row.

Task 72 listed "no write-back to `distance_km`" as an explicit **non-goal**
([01-task.md](../_done/72-route-map-origin-destination/01-task.md)) and recorded it as
**ADR-039**. This task reverses that decision. The ADR must be superseded, not quietly
contradicted.

## Requirements

### Round trip as two legs

- With the round-trip option on, route **A to B** and **B to A** as separate requests.
- Each leg offers its own alternatives, fastest-first, never reordered.
- The user can choose per leg.
- Deviation is computed against the **combined** road distance of the chosen pair.
- Loop mode (origin equals destination) is untouched -- it stays the genetic algorithm.
- The `alternativesUnavailable` copy becomes true again: it should then only appear for a
  route that genuinely goes through an intermediate stop.

### Distance write-back

- From the map view, the user can write the routed distance onto the trip's
  `distance_km`.
- Before writing, show what it changes: the new distance, and the effect on the
  fuel-consumption period that contains the trip.
- **The warning must be specific.** A period closes on a full-tank fill-up
  ([calculations/mod.rs:101-119](../../src-tauri/core/src/calculations/mod.rs)) and its
  rate is `period_fuel / period_km * 100`. Changing one trip's distance moves that
  period's rate. The legal test is `margin_percent <= 20.0`
  ([calculations/mod.rs:46-52](../../src-tauri/core/src/calculations/mod.rs)). So the
  warning must show the **recalculated margin for the affected period and whether it
  crosses 20%**, before the user commits -- not a generic "this affects consumption".
- Writing must **preserve the odometer invariant** (see Technical Notes).
- Nothing is written without an explicit confirmation.

## Technical Notes

### The odometer invariant holds today -- do not break it

Measured against the production copy on 2026-09-08: **105 of 108 trips** have
`odometer_end - odometer_start == distance_km` exactly. The chain is sound, so write-back
must preserve it rather than defend against pre-existing drift.

- `Trip.odometer` is the trip's **ending** odometer, stored per row
  ([models.rs:201](../../src-tauri/core/src/models.rs)).
- A trip's **starting** odometer is derived as the previous trip's stored ending odometer
  ([helpers.rs:67-90](../../src-tauri/core/src/commands_internal/helpers.rs),
  `calculate_odometer_start`), sorted by date, then datetime, then `created_at`.
- The row editor already keeps km and odo in step in both directions -- changing one
  recalculates the other ([TripRow.svelte](../../src/lib/components/TripRow.svelte),
  covered by
  [km-odo-bidirectional.spec.ts](../../tests/integration/specs/tier1/km-odo-bidirectional.spec.ts)).
- `update_trip` already accepts both `distance_km` and `odometer`, so a user can already
  make this edit by hand. Write-back automates an existing path; it does not open a new
  one.

**The open question this task must answer:** changing a trip's distance changes its
ending odometer, which shifts the derived starting odometer of every later trip. Either
every later row's stored odometer shifts with it, or the invariant breaks for the next
row. Decide this deliberately during design -- it is the crux, and it is why this task
was not planned in the same session it was written.

### Related

- Three rows in the production book currently violate the invariant, including one with
  a **negative** span. That is a separate data-integrity task, not this one --
  see [task 79](../_done/79-odometer-span-inconsistency/).
- Dragging a via onto the **return** leg of a round trip currently places it on the
  outbound leg: `currentWaypoints()` returns the open `[A, v1, B]` while the polyline is
  the closed line, so the nearest-vertex search falls through and clamps
  ([route_maps.rs](../../src-tauri/core/src/commands_internal/route_maps.rs)). Routing
  each leg separately removes this by construction, so it should be fixed here.
- `busy` does not account for an unresolved endpoint, so dismissing the place dialog with
  Escape leaves the recalculate button enabled and the click dead-ends
  ([mapa/+page.svelte](../../src/routes/mapa/+page.svelte)). Small, and in the same file
  this task touches.

## Open questions for design

1. Does write-back shift every later trip's odometer, or only this row's?
2. Is write-back allowed at all on a trip inside an already-closed period, or only
   flagged? A closed period's rate is what the legal margin is judged on.
3. Does the user pick alternatives per leg independently, or pick a pair?
4. Is the round-trip flag still one boolean once there are two legs, or does a saved
   route need to remember a chosen alternative per leg?
