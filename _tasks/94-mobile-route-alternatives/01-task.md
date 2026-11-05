# Task 94: Route map, alternatives and provider on the phone manual add

**Date:** 2026-11-05
**Subject:** Route map, route alternatives and the OSRM / Sygic switch on `/m/nova`
**Status:** In Progress (implemented on branch `feat/mobile-route-alternatives`, not merged)

## Goal

When the user adds a trip by hand on the phone (`/m/nova`), the page shows the route
on a map, offers the route alternatives and the routing service (OSRM or Sygic), and
saves the chosen route with the trip.

The design is screens 6 and 9 of the artifact "Kniha jázd na mobile"
([link](https://claude.ai/artifact/TyYTKii3cU4ryvN98giUGW)).

## Decisions

| Question | Answer |
|---|---|
| Provider switch | On the form only, not remembered. No title ("Smerovanie" was rejected). Shows only with 2+ services. |
| Typed km | The route stays. `target_km` = the trip km, so the map shows the deviation. |
| "Avoid paid roads" on the phone | No, desktop `/mapa` only. |
| How many alternatives | What the router gives, at most 3 (`MAX_ALTERNATIVES`, as `/mapa`). |
| Map height | 200 px, fixed. |
| Save | A second write after the trip commit (`save_manual_trip_route`), as with the Fuelio GPS route. A failed route save keeps the trip. |

## Done

- [x] Backend: `get_manual_trip_estimate` returns `routes[]` + `provider`, takes `provider` (tests in [mobile_tests.rs](../../src-tauri/core/src/commands_internal/mobile_tests.rs)).
- [x] Backend: `save_manual_trip_route` in [mobile.rs](../../src-tauri/core/src/commands_internal/mobile.rs) (RPC parse test in [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs)).
- [x] Frontend: [RoutePickerMap.svelte](../../src/lib/components/mobile/RoutePickerMap.svelte), the switch, the chips and the route save in [+page.svelte](../../src/routes/m/nova/+page.svelte).
- [x] Integration: [mobile-manual-route.spec.ts](../../tests/integration/specs/tier2/mobile-manual-route.spec.ts).
- [x] Docs: [mobile.md](../../docs/features/mobile.md), [CHANGELOG.md](../../CHANGELOG.md), [README.md](../../README.md), [README.en.md](../../README.en.md).

## Not covered

- The chip tap in an integration test: the mock router gives one route per service.
- Not tested on a real phone.
