# Route Provider Switch Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Select OSRM or Sygic per route on `/mapa`, store it with the saved route.

**Architecture:** The server config decides which providers exist and the default. The three routing RPCs take an optional `provider`; every response reports the provider that routed it; the saves store it in `trip_routes.provider`.

---

## Task 1: Provider selection in Rust

**Files:** `src-tauri/core/src/route_map/provider.rs`, `provider_tests.rs`, `osrm.rs`, `sygic.rs`, `mod.rs`, `server/dispatcher_async.rs`

**Steps:**
1. Tests first: default without a request, OSRM on a keyed server, Sygic without a key refused (`PROVIDER_NEEDS_SYGIC`), OSRM with an avoid list refused, `info()` per config, mock as OSRM (90 km, no options), serde of the kind.
2. `RouteProviderKind`, `RouteProvidersInfo`, `ProviderConfig::info()`, `build_provider(config, requested, avoid)`, `RouteProvider::kind()`.
3. Optional `provider` arg on `generate_route`, `route_direct`, `route_round_trip`.

**Verification:** `cargo test -p kniha-jazd-core provider_tests`

## Task 2: Report and store the provider

**Files:** `commands_internal/route_maps.rs`, `route_maps_tests.rs`, `models.rs`, `schema.rs`, `db.rs`, `db_tests.rs`, `migration_tests.rs`, `server/dispatcher.rs`, migration `2026-09-29-110000_add_trip_route_provider`

**Steps:**
1. Tests first: each route response carries `provider`; both saves store it; NULL reads as unknown; migration backfill (osrm / sygic / NULL); RPC save + get; unknown value rejected; `get_route_providers`.
2. `provider` on `GeneratedRoute`, `RoundTripRoutes`, `SavedRouteMap`, `RouteMap`, the row structs (last column); `provider` param on both save functions; `get_route_providers` RPC.

**Verification:** `npm run test:backend`

## Task 3: Selector on `/mapa`

**Files:** `src/lib/types.ts`, `src/lib/api.ts`, `src/lib/i18n/{sk,en}/index.ts`, `src/routes/mapa/+page.svelte`, `tests/integration/specs/tier2/route-map.spec.ts`

**Steps:**
1. Integration test first: switch to OSRM routes again (90.0 km), clears the avoid list, the save stores it, the reopen selects it.
2. Types, API args, i18n (`npm run i18n`), the select in the toolbar, adopt `provider` from each response, send the route's own provider on save, `PROVIDER_NEEDS_SYGIC` error text.

**Verification:** `npm run check`, focused `route-map.spec.ts`, then the full integration suite.

## Task 4: Docs

**Files:** `CHANGELOG.md` (upgrade notes: 1 migration), `DECISIONS.md` (ADR-053), `docs/features/route-maps.md`, `README.md`, `README.en.md`, `_tasks/index.md`
