**Date:** 2026-09-29
**Subject:** Route map: avoid paid roads per country (Sygic routing provider)
**Status:** Complete

## Goal

On the route map page (`/mapa`), the user can avoid the paid roads of one
country and keep the paid roads of other countries. The user does this with a
checkbox, not with manual vias.

Reference case: Bratislava to Brno. The user wants the Slovak D2 and no Czech
vignette roads.

## Why the current setup cannot do it

The app routes with the public OSRM demo server
(`HttpRouteProvider::public()` in
[dispatcher_async.rs](../../../src-tauri/core/src/server/dispatcher_async.rs)).
Checked on 2026-09-29, Bratislava to Brno:

| Service | Result |
|---|---|
| `router.project-osrm.org` | `exclude=motorway` and `exclude=toll` both return `InvalidValue Exclude flag combination is not supported.` |
| `routing.openstreetmap.de/routed-car` | Same error. |
| Public Valhalla, `use_highways: 0` | 151.3 km / 192 min. Global setting only, the SK D2 is lost too. |
| Google Routes API | `avoidTolls` / `avoidHighways` are global and only "where reasonable". No CZ/SK value in the `TollPass` enum. The terms, section 19.2: "Customer must not use Google Maps Content from the Routes API in conjunction with a non-Google map." |

OSRM can know countries only on a self-hosted server with a custom profile
(`--location-dependent-data`). Sygic gives the same result with no server to
run.

## Sygic Routing API: verified behaviour

Spec: [routing v3 swagger.yaml](https://routing.api.sygic.com/swagger/routing/v3/swagger.yaml),
endpoint `GET|POST /v3/api/directions`. Tested on 2026-09-29 with a real key.

Spec text for `avoid`:
> Values: tolls|highways|ferries|unpaved|congestioncharges
> To avoid a country use syntax iso:country, eg. svk:country

Bratislava (48.1486,17.1077) to Brno (49.1951,16.6068), `vehicle_type=car`:

| `avoid` | km | min | `possible_avoids` in the response |
|---|---|---|---|
| (none) | 130.1 | 88 | `svk:highways, svk:tolls, svk:country, cze:tolls, cze:highways, cze:country` |
| `cze:tolls` | 132.9 | 111 | `svk:highways, svk:tolls, svk:country, cze:highways, cze:country` |
| `cze:highways` | 150.1 | 144 | `svk:highways, svk:tolls, svk:country, cze:country` |
| `highways` | 159.5 | 159 | `svk:country, aut:tolls, aut:country, cze:country` |

Other results:

- `route` is a Google encoded polyline, precision 1e5 (polyline5). Decoded
  start `(48.14806, 17.10727)`, end `(49.19512, 16.60641)`. This is the same
  format that `RouteMap.polyline` stores now.
- `distance.value` is in metres, `duration.value` is in seconds.
- `compute_alternatives=true` with `avoid=cze:tolls` returned 3 routes.
- `waypoints` with 10 and with 25 points worked. The response has one leg per
  segment.
- Parameter order is `lat,lon` (`origin=48.1486,17.1077`). OSRM uses `lon,lat`.
- After `avoid=cze:tolls`, the list no longer contains `cze:tolls`. The new
  route has no CZ toll road, so Sygic does not offer that avoid again.
- The key can have a referer restriction. Without a matching `Referer` header
  the API returns `HTTP 403 Allowed referers do not match.`
- The spec lists OAuth2 client credentials as the auth scheme. A `key=` query
  parameter works too (tested).

## Decisions made in brainstorming

1. **Avoid type is `tolls`, not `highways`.** "Paid roads" means the vignette
   sections. Free highway sections stay allowed (BA to Brno: 132.9 km with
   `cze:tolls`, against 150.1 km with `cze:highways`).
2. **Sygic computes all routes when `SYGIC_API_KEY` is set.** Routes with and
   without an avoid then come from the same engine, so their km values
   compare. Without the key the app uses public OSRM as today, and the page
   shows no avoid checkboxes.
3. **The avoid list is per route and saved.** It is stored on the route map
   row, so a reopen or a recompute keeps it.
4. **The checkboxes come from `return_possible_avoids`.** The backend keeps only
   the `*:tolls` values. The page shows the union of these values and the
   current avoid list, so a checked country does not disappear.
5. **No silent fallback to OSRM.** If Sygic fails, the page shows the error. A
   fallback route would ignore the avoid list and would look correct.

## Requirements

### Backend

- R1. The avoid list is fixed when a provider is built. The `RouteProvider`
  trait does not change. OSRM refuses a non-empty avoid list (decision 5).
- R2. New `SygicRouteProvider` in [route_map/](../../../src-tauri/core/src/route_map/):
  - Sends `origin`, `destination`, `waypoints` (pipe-separated) in `lat,lon`
    order with 6 decimals.
  - Sends `avoid` joined with `|`, `return_possible_avoids=true`,
    `vehicle_type=car`, and the key.
  - Sends `compute_alternatives=true` only for a request with exactly two
    points. This keeps the rule of the alternatives ADR in
    [DECISIONS.md](../../../DECISIONS.md): order as returned, never re-sorted.
  - Sends a `Referer` header when `SYGIC_REFERER` is set.
  - Maps `route`, `distance.value / 1000`, `duration.value` and the
    `*:tolls` values of `possible_avoids`, plus its own avoid list, into
    `FetchedRoute`.
  - Keeps the error style of [osrm.rs](../../../src-tauri/core/src/route_map/osrm.rs): HTTP status in the message, 429
    readable by the existing Retry prompt, no panic on a bad client build.
- R3. `FetchedRoute` gets `possible_avoids: Vec<String>`. OSRM returns an empty
  list.
- R4. One function selects the provider from the environment: the test mock,
  Sygic or OSRM. All three async commands use it, in place of the three
  `HttpRouteProvider::public()` calls.
- R5. `generate_route`, `route_direct` and `route_round_trip` take an optional
  `avoid: Vec<String>` (serde default `[]`), so an old caller still works.
- R6. `GeneratedRoute` gets `avoidOptions`. A round trip returns the union of
  both legs. Both legs use the same avoid list.
- R7. Migration: add `avoid TEXT NOT NULL DEFAULT '[]'` to `trip_routes`, as the
  LAST column (`RouteMapRow` binds by position, see the notes in
  [schema.rs](../../../src-tauri/core/src/schema.rs)).
- R8. `save_trip_route` and `save_trip_round_trip_route` store the list.
  `get_trip_route` returns it as `RouteMap.avoid`.

### Frontend (`/mapa`)

- R9. One checkbox per value in `avoidOptions`. The backend already adds the
  avoided values, so a checked country stays visible. Label from i18n, for example "Vyhnúť sa spoplatneným cestám: CZ".
  Country names for at least SVK, CZE, AUT, HUN, POL, with the ISO code as the
  fallback for other countries.
- R10. A checkbox change fetches the route again in the current direct mode
  (one-way or round trip) with the new list. Loop mode has no checkboxes.
- R11. A reopened saved route shows its saved list as checked.
- R12. No checkboxes when the union is empty (OSRM, or a route with no paid
  road).
- R13. All text goes through i18n (`sk` and `en`). Run `npm run i18n`.

### Configuration

| Env var | Default | Purpose |
|---|---|---|
| `SYGIC_API_KEY` | unset | If set, Sygic computes all routes. If unset, public OSRM. |
| `SYGIC_REFERER` | unset | `Referer` header for a key with a referer restriction. |
| `KNIHA_JAZD_MOCK_ROUTER` | unset | Any value: an offline mock computes all routes. For the integration suite only. |

Do not write a key or a referer value into this repo. The repo is public.

### Tests

- Backend unit tests for the Sygic URL builder: `lat,lon` order, `avoid`
  joining, `waypoints`, `compute_alternatives` only for two points.
- Backend unit tests for the response mapping: polyline, km, seconds, the
  `*:tolls` filter, errors (non-`OK` status, empty `routes`, HTTP 401, 403,
  429).
- Provider choice tests: mock, Sygic, OSRM, and OSRM refusing an avoid list.
- A `route_maps` test with a fake provider: `avoidOptions` reaches the result,
  round trip union included.
- A DB test: save and get keep the avoid list. A row from before the migration
  reads as `[]`.
- Integration tests with the mock router: a reopened saved route shows its
  checked box, and a click routes again. The suite uses `KNIHA_JAZD_MOCK_ROUTER`
  (Task 8).

### Documentation

- ADR through `/decision`: Sygic as an optional routing provider, and why not
  Google, Valhalla or public OSRM.
- [CHANGELOG.md](../../../CHANGELOG.md) through `/changelog`.
- [route-maps.md](../../../docs/features/route-maps.md), the env var table in
  [CLAUDE.md](../../../CLAUDE.md), [README.md](../../../README.md) and
  [README.en.md](../../../README.en.md).

## Open items

- **Sygic terms and quota.** The app stores the route geometry with no time
  limit. Check that the Sygic terms allow this, and check the request quota of
  the plan. Not verified.
- **Deploy.** The live stack needs `SYGIC_API_KEY` and `SYGIC_REFERER`. That
  change belongs to the infra repo, not to this task.
- **POST or GET.** The GET form worked in the tests. POST with a JSON body is
  also available. Choose one in the plan.

## Technical Notes

- ADR-008: the avoid filter (`*:tolls`), the round trip union and the provider
  choice are backend logic. The page only renders `possibleAvoids` and sends
  the checked values back.
- Loop mode shows no checkboxes and sends no avoid list. A change of the list
  would call `generate_route` again, and the GA would replace the loop with a
  new random one. The bundled dataset is Slovak, so a loop rarely crosses a
  border anyway.
- A saved route keeps its polyline. Only a recompute calls the provider again.
