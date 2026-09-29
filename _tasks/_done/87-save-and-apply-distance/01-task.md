**Date:** 2026-09-29
**Subject:** One "Uložiť a použiť vzdialenosť" button: a saved map always writes its distance to the trip, in every route mode
**Status:** Complete

## Goal

If a trip has a saved map, the trip's `distance_km` agrees with the map. The `/mapa`
page has one button, "Uložiť a použiť vzdialenosť". It saves the route and writes the
route's distance to the trip in one step. The button does the same thing for every
route mode: Direct one-way, Direct round trip ("Cesta tam a späť") and Loop.

## Current behavior

| Item | Today |
|------|-------|
| "Uložiť mapu" | Writes only `trip_routes`. The trip does not change. |
| "Použiť vzdialenosť" | Writes `trips.distance_km` and the odometer cascade, behind a dry-run modal (ADR-048). Direct mode only. |
| Loop mode (origin equals destination, `mode_for` in [route_maps.rs:948](../../../src-tauri/core/src/commands_internal/route_maps.rs#L948)) | No write-back. [route-maps.md:450](../../../docs/features/route-maps.md) calls a Loop write-back "circular". |
| Written value | Raw road km, for example `25.634` (`apply_route_distance_internal`, [trips.rs:748](../../../src-tauri/core/src/commands_internal/trips.rs#L748)). |

## Decisions from the brainstorm (2026-09-29)

| # | Question | Decision |
|---|----------|----------|
| D1 | Which modes write the distance? | All: Direct one-way, Direct round trip, Loop. |
| D2 | Saved map whose road km differs from the trip km (saved before this task, or the km was edited later) | The button is enabled. It writes the saved road km behind the same modal. The map row does not change. |
| D3 | The distance already agrees | Skip the modal. Save the map at once. |
| D4 | Precision of the written value | Round to whole km. 25.634 becomes 26. |

## Requirements

### R1. One button

- "Uložiť mapu" and "Použiť vzdialenosť" go away. One button, "Uložiť a použiť
  vzdialenosť", replaces them (`data-test="save-apply-btn"`).
- The button shows in every mode.
- The button is enabled if one of these is true:
  - There is an unsaved proposal (`generated` or `roundTripRoutes`).
  - There is no proposal, a saved map exists, and the backend reports it as not in sync (R4).
- "Odstrániť mapu" stays. Removing a map does not change the trip km.

### R2. Save and write in one transaction

- The click runs a dry run first. The dry run writes nothing.
- If the dry run reports no change (the rounded km equals the trip km and no odometer moves),
  the page commits at once, with no modal (D3).
- If it reports a change, the existing [OdometerCascadeModal](../../../src/lib/components/OdometerCascadeModal.svelte) (`kind="writeback"`) shows the
  plan: the new km, the period rate and margin before and after, the 20 % limit, and every
  later row whose odometer moves.
- Confirm writes the map row, the trip row and the odometer shifts in ONE database
  transaction. A partial write (map saved, km not) must not be possible.
- Cancel writes nothing: no map and no km.
- Confirm sends the payload the user approved. The page keeps a copy of the save payload,
  taken at the click. If the user picks a different alternative behind the modal, the commit
  does not change (ADR-048 rule).

### R3. Rounding (D4)

- Rust rounds the road km to a whole number before it plans the cascade:
  `written_km = road_km.round()`.
- The stored `trip_routes.road_km` stays raw. The map still shows the real road distance.
- This changes the Direct write-back from raw km to whole km.
- On commit, `trip_routes.target_km` is the new trip km.

### R4. "In sync" is a backend fact

- `SavedRouteMap` gets `distance_in_sync: bool`, defined in Rust as
  `road_km.round() == trip.distance_km` (with the cascade epsilon).
- The frontend reads the flag. It does not compare numbers (ADR-008).
- If the map is in sync, `off_target` is `false`. Without this, a short trip shows a
  permanent warning after a sync: 2.4 km written as 2 km is a 20 % deviation, and
  `TOLERANCE` is 5 % ([ga.rs:26](../../../src-tauri/core/src/route_map/ga.rs#L26)). `deviation_percent` still shows the real value.

### R5. Sync of a saved map (D2)

- The backend reads `road_km` from `trip_routes`. The browser sends only the trip id and
  `dry_run`.
- This replaces `apply_route_distance(tripId, roadKm, dryRun)`. That command and its
  frontend wrapper go away.

### R6. Read-only mode

- The dry run works in read-only mode. The commit is refused by `check_read_only!`.
- The dry run must not reach the `check_read_only!` in `persist_route_map`.

## Technical Notes

- **Atomicity.** `Database::save_route_map` and `Database::update_trip_with_odometer_shift`
  each lock the connection and open their own transaction ([db.rs:1095](../../../src-tauri/core/src/db.rs#L1095), [db.rs:548](../../../src-tauri/core/src/db.rs#L548)).
  Factor both bodies into `tx`-level helpers, and add one method that runs both in one
  transaction.
- **Shared planner.** Extract the plan part of `apply_route_distance_internal`
  (trip, vehicle, year trips, year start, `plan_odometer_cascade`,
  `mark_next_year_chain_breaks`, `period_margin_impact`) into one function. The save
  commands and the saved-map sync both call it.
- **No-op detection comes free.** Round before `plan_odometer_cascade`. Its
  `CASCADE_EPSILON` branch then returns an empty plan with `delta == 0`, and the frontend
  skips the modal on that result (the backend tells, the frontend only reads).
- **Integration fixtures.** [route-map.spec.ts](../../../tests/integration/specs/tier2/route-map.spec.ts) and [route-distance-writeback.spec.ts](../../../tests/integration/specs/tier2/route-distance-writeback.spec.ts) seed
  maps through `save_trip_route` and `save_trip_round_trip_route` (7 calls). After this task
  each seeded map also writes the trip km. Audit each fixture. If a fixture must seed a
  mismatched map (legacy state), save it in sync and then change the trip km with
  `update_trip_cascade`. That is the real legacy path ("the km was edited later").
- **Loop in integration tests.** `MockRouteProvider` ([provider.rs:153](../../../src-tauri/core/src/route_map/provider.rs#L153)) answers
  `route_direct` offline. Check whether `generate_route` (Loop) also runs offline under
  `KNIHA_JAZD_MOCK_ROUTER`. If it does not, the backend unit test is the authoritative test
  for Loop, and the integration test covers Direct only.
- **Superseded text.** [ADR-048](../../../DECISIONS.md) limits the write-back to Direct mode, and [route-maps.md:450](../../../docs/features/route-maps.md)
  gives the "circular" reason for Loop. A new ADR-054 records D1 to D4 and supersedes that
  scope.

## Out of scope

- A migration that syncs existing mismatched maps. D2 covers them one at a time, with the
  warning.
- Any change to the route generators or to the routing providers.
- Revert of the km when a map is removed.
