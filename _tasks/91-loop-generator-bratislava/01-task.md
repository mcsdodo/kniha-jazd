**Date:** 2026-10-08
**Subject:** Loop generator around the trip's place, with Bratislava districts as the second candidate set, and numbered waypoints on the map
**Status:** Planning

## Goal

A trip with origin = destination opens `/mapa` in loop mode. Today the loop
always starts at the fixed home node, so a "Bratislava -> Bratislava" trip of
43 km gets a loop around Spišská Nová Ves. After this task:

1. The loop starts and ends at the trip's own place.
2. The loop target is the km value of the trip, as today.
3. The generator knows two areas: the home area (as today) and Bratislava.
4. The user drags the waypoints after generation. A km change after a drag is
   acceptable (this works today, see ADR-040).
5. The map shows a number on each via, so the user can see the order.

## Why it works only at home today

- [src-tauri/core/src/route_map/ga.rs:29](../../src-tauri/core/src/route_map/ga.rs): `const HOME: usize = 0;`
- [src-tauri/core/src/route_map/dataset.rs:3](../../src-tauri/core/src/route_map/dataset.rs): "67 nodes (1 home base + 22 towns
  within 50 km + 44 villages within 20 km)", compiled into the binary.
- [src-tauri/core/src/commands_internal/route_maps.rs:268](../../src-tauri/core/src/commands_internal/route_maps.rs):
  `generate_route_internal(provider, target_km)` always uses
  `Dataset::bundled()` and does not get the trip's place.
- [docs/features/route-maps.md](../../docs/features/route-maps.md) ("The waypoint editor doesn't know which mode
  drew the line") lists this limit: re-anchoring "needs a distance matrix the
  app does not have".

The GA does not depend on the area. It needs a list of points with the start at
index 0, and an asymmetric driving-distance matrix between them.

## Scope

In scope:

- One new bundled candidate file: the 17 Bratislava city districts.
- Selection of the candidate set from the trip's place.
- One OSRM `/table` call for the Bratislava matrix.
- Numbers on the waypoint handles.

Out of scope (the user said "add JUST the bratislava districts for now.
nothing else."):

- A national list of Slovak towns and villages.
- Districts of other cities (Košice and others).
- Places abroad.
- A cache for the matrix. One `/table` call costs about 0.3 s.
- Changes to the GA, to the tolerance, or to the 67-node home dataset.

## Design

### 1. Data

- Add `src-tauri/core/assets/bratislava.json`. It holds the 17 districts
  (mestské časti) of Bratislava: `idx`, `name`, `lat`, `lon`, `kind`
  (`"district"`), and a `generatedAt` value. Use the same shape as
  [villages.json](../../src-tauri/core/assets/villages.json). The file has no matrix.
- Source: OpenStreetMap, the `boundary=administrative` + `admin_level=10`
  relations with a name that starts with `Bratislava-`, one point per district
  (the `center` of the relation, or its `label`/`admin_centre` node if it has
  one). Fetch it once with a script in this task folder and commit the result.
  Overpass needs a `User-Agent` header (without one it returns HTTP 406). On
  2026-10-08 the public servers returned 504 and 500 under load, so the script
  must retry.
- Check the result by hand: 17 rows, each point inside its district, on a road
  network (not in a forest, for example Bratislava-Nové Mesto has a large
  forest area). Move a point by hand if needed and record the change in the
  script folder.
- Keep [villages.json](../../src-tauri/core/assets/villages.json) and [matrix.json](../../src-tauri/core/assets/matrix.json) without changes.

### 2. Selection of the candidate set (Rust)

`generate_route_internal` gets the anchor: the coordinates of the trip's
origin place from the place book (the trip is a loop, so origin = destination).

| Anchor | Candidate set | Matrix |
|---|---|---|
| Straight-line distance to home node 0 (`48.9350604, 20.5533207`) <= 5 km | the 67-node home set, as today | bundled, offline |
| Straight-line distance to the Bratislava centre (`48.1486, 17.1077`) <= 18 km | anchor at index 0 + the 17 districts | one OSRM `/table` call, 18 x 18 |
| Any other place | none | error (see section 5) |

- Put the two radii and the two centre points in one place in Rust, as named
  constants.
- In the Bratislava case, index 0 is the real coordinate of the trip's place,
  not a district centre. The GA uses index 0 as the start and end, so the GA
  needs no change.
- If the anchor place has no coordinates, return the same error as a direct
  route with an endpoint without coordinates, so the page opens the place
  dialog. Use the existing flow, do not add a new one.
- Split the selection into a pure function (anchor -> which set) and the
  network part, so the unit tests need no network.

### 3. The matrix call

- Add a `table(coords) -> Result<Vec<Vec<f64>>, String>` operation for the
  matrix, in km. Measured on 2026-10-08: the public OSRM server accepts up to
  100 coordinates per `/table` call (101 returns `TooBig`, "Too many table
  coordinates"). 18 points is far below the limit.
- The matrix always comes from OSRM, also when Sygic is the selected provider.
  The final `/route` call uses the selected provider, as today.
- The mock router (`KNIHA_JAZD_MOCK_ROUTER`, [route_map/provider.rs:150](../../src-tauri/core/src/route_map/provider.rs)) gets
  a fake `table`: straight-line distance x 1.3. This keeps the integration
  tests offline.
- If one cell of the matrix is `null` (OSRM found no route), return an error.
  Do not run the GA with a missing distance.

### 4. RPC change

- `generate_route` gets a new required argument `tripId`. The backend reads
  the trip, then the place, then selects the set. The frontend does not send
  coordinates ([ADR-008](../../CLAUDE.md#architecture-backend-only-calculations): the backend decides).
- Update [src/lib/api.ts](../../src/lib/api.ts) (`generateRoute`) and the call in
  [src/routes/mapa/+page.svelte:981](../../src/routes/mapa/+page.svelte).
- Update the dispatcher test `generate_route_is_an_async_command_taking_target_km`
  ([server/dispatcher_async.rs:458](../../src-tauri/core/src/server/dispatcher_async.rs)).
- `dataset_version` in the result and in `trip_routes` takes the `generatedAt`
  value of the set that was used. A saved loop around Bratislava then records
  the Bratislava file version.

### 5. Errors (Slovak UI text through i18n)

- Anchor outside both areas: "Pre toto miesto generátor nemá kandidátov."
  The page shows it in the same place as other routing errors.
- `/table` fails or has a missing cell: the existing routing error text.

### 6. Numbered waypoints (frontend)

- `handleIcon()` ([src/routes/mapa/+page.svelte:476](../../src/routes/mapa/+page.svelte)) gets an optional number.
  Every via shows its number in travel order: 1, 2, 3, ...
- The start and end point keeps the larger dot without a number. In a loop the
  start and end are the same point.
- Round trip (`drawLegHandles`): the numbers continue from the outbound leg into
  the return leg. The return leg draws no endpoints today, so its first via
  gets the number after the last outbound via.
- This applies in all modes, because the handle code does not know the mode
  ([ADR-040](../../DECISIONS.md#adr-040-the-waypoint-editor-is-mode-agnostic)).
- The numbers update after each drag, insert and removal, because
  `drawHandles()` draws all handles again.
- The number is only display. It is the index in the waypoint list, which the
  backend already ordered. The frontend does not calculate an order.

## Flow, example

1. Trip "Bratislava -> Bratislava", 43 km. The user clicks the map pin.
2. `/mapa?trip=id` starts in loop mode. The backend selects the Bratislava set,
   calls `/table`, runs the GA, and calls `/route`.
3. The map shows a loop of about 43 km through some districts, with the vias
   numbered 1..k.
4. The user drags vias. The deviation in km shows, as today.
5. The user saves ("Uložiť a použiť vzdialenosť", task 87).

## Tests

Backend unit tests (TDD, write them first):

- Selection: an anchor near home node 0 selects the home set; an anchor in
  Bratislava selects the Bratislava set; an anchor in Žilina returns the
  "no candidates" error. Test the radius boundary on both sides.
- Bratislava set: index 0 is the anchor coordinate, indices 1..17 are the
  districts.
- Matrix: an OSRM `/table` response converts to km; a `null` cell returns an
  error.
- GA with the Bratislava set and a seeded RNG: the sequence starts and ends at
  index 0.
- The bundled `bratislava.json` parses and has 17 rows.
- Dispatcher: `generate_route` requires `tripId`.

Integration test (one spec, mock router):

- A Bratislava loop trip -> open the map -> the generated route shows numbered
  via handles (1..k). Do not test the km math again.

## Documentation

- [docs/features/route-maps.md](../../docs/features/route-maps.md): replace the "deferred limitation" text,
  update the data flow for loop mode and the key-files table.
- [DECISIONS.md](../../DECISIONS.md) (`/decision`): an ADR for "the loop anchors at the trip's
  place; candidate sets are per area; the matrix for a non-home set comes from
  one OSRM `/table` call at generation time".
- [CHANGELOG.md](../../CHANGELOG.md) (`/changelog`): user-visible. The upgrade-notes block says:
  no migration, no new env var, no image change.

## Known limits

- In Bratislava, targets below about 10 km can miss the 5% tolerance, because
  17 points are few. The deviation warning shows this, as for the home set.
- A new area needs its own candidate file and one row in the selection table.
- The home set stays anchored at home node 0, not at the home place's
  coordinates. This is the same as today.
