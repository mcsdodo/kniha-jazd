**Date:** 2026-09-29
**Subject:** Select the routing provider (OSRM or Sygic) per route on `/mapa`
**Status:** Complete

## Goal

On a server with `SYGIC_API_KEY`, route a trip with OSRM when Sygic gives the wrong road. The trigger: the Sygic map does not have the D1 Visnove tunnel. Its route from Spisska Nova Ves to Bratislava through Zilina uses the old I/18 road in the Strecno gorge (365.256 km). The OSRM route uses the tunnel (356.975 km). Measured on 2026-09-29, see ADR-053.

## Requirements

Decided with the user on 2026-09-29:

1. **Scope:** a selector on `/mapa` only. No bulk recalculation feature in the app.
2. **Persistence:** a saved route stores its provider (`trip_routes.provider`). A reopened route selects it.
3. **Default:** OSRM, also when `SYGIC_API_KEY` is set (changed after the first deploy, on the user's request). No new env var.
4. The selector shows only when the server offers more than one provider.
5. OSRM cannot avoid per country. A switch to OSRM clears the avoid list.
6. Loop mode: a switch does not regenerate the loop. It applies to the next "Regenerovať".

## Technical Notes

- `RouteProviderKind` (`osrm` / `sygic`) in `route_map/provider.rs`. `build_provider(config, requested, avoid)`: the config decides what exists and the default, the request picks.
- Errors with stable markers: `PROVIDER_NEEDS_SYGIC` (Sygic asked for, no key), `AVOID_NEEDS_SYGIC` (OSRM with an avoid list). No silent fallback.
- `RouteProvider::kind()` (default OSRM) lets the routing commands report the provider without a signature change.
- Migration `2026-09-29-110000_add_trip_route_provider` backfills only what can be proven: `osrm` before the first Sygic commit (c80e15e, 2026-09-29T07:39:53Z), `sygic` for a row with an avoid list, NULL otherwise.
- The mock router offers both providers; as OSRM it returns 90.0 km and no avoid options.
- Decision record: ADR-053. Feature doc: `docs/features/route-maps.md`, section "Provider per request".
