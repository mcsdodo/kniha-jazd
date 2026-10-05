**Date:** 2026-10-05
**Subject:** Plan review for 89-home-place-mcp (02-plan.md)
**Status:** Phase 1 complete, waiting for user direction

# Plan Review: Home Place, Journeys and Read-Only MCP

**Input:** The skill got an empty plan path. This review uses the plan in the working
directory: [02-plan.md](./02-plan.md). The spec is [01-task.md](./01-task.md).

**Recommendation: Needs Revisions.** The plan structure, the TDD order, the test counts
and the `rmcp` 3.5.0 API are correct. One code block does not compile against the
task 88 interfaces, and one integration spec fails in `beforeEach`.

**Counts:** 1 Critical, 3 Important, 8 Minor.

## Context

[01-task.md](./01-task.md) line 4 says: "Write `02-plan.md` only after 88 is merged,
because 88 moves the code this plan will cite." Task 88 is not merged. `HEAD` is
`121dd95 chore: release v1.3.0`, and the folders `_tasks/_done/88-places-as-entities/` and
`_tasks/89-home-place-mcp/` are both untracked. So plan 89 cites the task 88 *plan*,
not code. Task 0 reduces this risk, but finding C1 shows that one block already breaks.
Do a Task 0 pass again after 88 merges.

## Findings

### Critical

- [ ] **C1. Task 3 Step 4: `Trip::from` does not exist after task 88.**
  The block ends with `Ok(rows.into_iter().map(Trip::from).collect())` (plan line 1086).
  The [88 plan](../../_done/88-places-as-entities/02-plan.md), Step 4 item 6, says: "Replace `impl From<TripRow> for Trip` with
  `impl Trip { pub fn from_row(row: TripRow, names: &HashMap<String, String>) -> Self`".
  The 88 plan, Step 5 item 2, adds the private `place_names(conn)` and uses it in each
  read that returns `Trip`. So the block does not compile.
  A workaround with an empty map compiles, but it fails the Task 4 test
  `list_trips_returns_place_names_and_minutes`, because that test asserts
  `origin == "Home St 1, Hometown"` and `destination == "City A"`.
  **Fix:** in the code block, after `.load::<TripRow>(conn)?`, add
  `let names = place_names(conn)?;` and map with `Trip::from_row(r, &names)`.

### Important

- [ ] **I1. Task 5 Step 1: `seedVehicle` call fails in `beforeEach`.**
  The spec calls `seedVehicle({ name: 'Home Test Car', licensePlate: 'HOME-1' })`.
  The `create_vehicle` RPC args have `initial_odometer: f64` with no default
  ([dispatcher.rs](../../../src-tauri/core/src/server/dispatcher.rs)), so the call fails with `Invalid args`. Also,
  `create_vehicle_internal` ([vehicles.rs:43](../../../src-tauri/core/src/commands_internal/vehicles.rs#L43)) returns
  `"ICE vehicles require tank_size_liters and tp_consumption"`.
  Step 2 "see it fail" fails for the wrong reason, and Step 5 never passes.
  **Fix:** pass `initialOdometer: 10000, tankSizeLiters: 50, tpConsumption: 6.5`, the
  same as the other tier2 specs (for example [route-distance-writeback.spec.ts:80](../../../tests/integration/specs/tier2/route-distance-writeback.spec.ts#L80)).

- [ ] **I2. Task 1 Step 2: `is_home` goes in `Place::from_row`, not in `list_places_internal`.**
  In the 88 plan, `list_places_internal` calls `Place::from_row(row, n)`, and
  `Place::from_row` in `models.rs` builds the struct. The step tells the implementer
  to edit the wrong function. **Fix:** say "In `Place::from_row` (models.rs), set
  `is_home: row.is_home`." Then each path that builds a `Place` (also `create_place`)
  reports the mark.

- [ ] **I3. Task 4: the `list_trips` description says "oldest first", but the result is not.**
  With `vehicle_id = None`, `list_trips_in_range_internal` appends the trips of each
  vehicle one after the other. The order is "by vehicle, then oldest first", not
  "oldest first". An AI client that trusts the description reads a wrong timeline.
  **Fix:** sort the merged list by `start_datetime`, then `odometer`, in
  `list_trips_in_range_internal`, and add one assertion to
  `list_trips_without_vehicle_uses_active_vehicles_only` (two active vehicles,
  interleaved trips). Or change the description to "per vehicle, oldest first".

### Minor

- [ ] **M1. The home mark leaks across integration tests.** `resetDatabase` in
  [wdio.server.conf.ts:174](../../../tests/integration/wdio.server.conf.ts#L174) deletes only trips and vehicles. Places stay, so the
  home mark of test 2 stays for the next spec on the same server. Add an `afterEach`
  that calls `rpc('set_home_place', { id: null })`. Also use 88's `ensurePlace(name)`
  in place of the local `placeId()` lookup over `list_places`.
- [ ] **M2. [_tasks/index.md](../../index.md) has no row for task 88 or task 89.** Task 8 Step 2 updates
  a row that does not exist. Add the row when the plan is committed
  ([_tasks/CLAUDE.md](../../CLAUDE.md), "Update index.md on every state change").
- [ ] **M3. "Deviation from the spec, on purpose" is not a deviation.** The spec,
  section 3, already says "So the lists are wrapped in `vehicles` and `trips`." Remove
  the word "deviation" from Global Constraints and from the ADR content in Task 7.
- [ ] **M4. Spec test row 3 differs from the plan test.** The spec expects 0 journeys
  for "home -> Village, one leg, the next leg starts at home". The plan test
  `single_leg_day_trip_followed_by_departure_is_not_a_journey` adds a return leg and
  asserts 1 journey (the second departure). The plan test is stronger. Keep it, but add
  a comment that it covers spec row 3.
- [ ] **M5. The home -> home reset is not in the spec.** `(true, true) => open = None`
  drops an open chain without a trace, the same as Review Focus 1. Add this to the BIZ
  entry and to the feature doc, next to Review Focus 1.
- [ ] **M6. The glyph check covers only the new doc.** Task 7 Step 6 greps only
  `mcp-endpoint.md`. DECISIONS.md, CHANGELOG.md and the other feature docs already
  contain em-dashes, so a full-file grep is not useful. Check the added lines of all
  touched files: `git diff -U0 | grep '^+' | grep -P "[\x{2014}\x{2013}\x{2192}\x{201C}\x{201D}\x{2026}]"`.
- [ ] **M7. The MCP tools do DB work without `spawn_blocking`.** `rpc_handler`
  ([server/mod.rs:63](../../../src-tauri/core/src/server/mod.rs#L63)) wraps DB work in `tokio::task::spawn_blocking`. The MCP tools are
  sync and lock the `std::sync::Mutex` on a runtime thread. This is safe on the
  multi-thread runtime, but it is not consistent. Optional: make the tools `async` and
  use `spawn_blocking`.
- [ ] **M8. Mixed error semantics.** A tool `Err(ErrorData::invalid_params)` gives a
  JSON-RPC `error`, and a missing argument gives `isError: true`. "Home place is not
  set" is not an invalid parameter. Optional: use `ErrorData::internal_error` or a
  `CallToolResult` with `isError: true` for `HOME_NOT_SET`. Do not block on this.

## Verified (no change needed)

These claims were checked against the source. Phase 2 does not need to check them again.

| Claim in the plan | Evidence |
|---|---|
| `rmcp` 3.5.0 needs Rust 1.88 | `rmcp-3.5.0/Cargo.toml:14` `rust-version = "1.88"` |
| Workspace and Docker versions today | [src-tauri/Cargo.toml:8](../../../src-tauri/Cargo.toml#L8) `"1.77.2"`, [Dockerfile.web:4](../../../Dockerfile.web#L4) `rust:1.86-bookworm` |
| `rmcp` uses `axum` only as a dev-dependency | `[dev-dependencies.axum]` in the rmcp `Cargo.toml` |
| Config methods exist | `tower.rs`: `disable_allowed_hosts` :222, `with_sse_keep_alive` :244, `with_legacy_session_mode` :254, `with_json_response` :259 |
| Default host allowlist is loopback only | `tower.rs:203`; origin allowlist is empty and not checked by default (:204-205) |
| Import paths | `session::never::NeverSessionManager` (`Default`, `Clone`), `wrapper::{Json, Parameters}`, `router::tool::ToolRouter`, `ServerConfig = InitializeResult` (model.rs:1179), `Implementation::new(name, version)` |
| `StreamableHttpService::new(factory, Arc<M>, config)` and `Clone` | `tower.rs:1403`, `tower.rs:1332` |
| `HttpServer::start` signature and both router branches | [server/mod.rs:141](../../../src-tauri/core/src/server/mod.rs#L141) |
| `crate::db_tests` path | `lib.rs` re-exports `crate::db::db_tests` under `#[cfg(test)]` |
| DB methods | `connection()` :192, `get_vehicle` :286, `get_all_vehicles` :297, `get_trips_for_vehicle` :417, `get_route_maps_for_trips` :1181 |
| `trip_routes` raw insert in `mark_round_trip` | `mode` has `DEFAULT 'loop'`, `round_trip` has `DEFAULT 0`, `dataset_version` is nullable |
| `start_datetime` storage format | `models.rs:336` `"YYYY-MM-DDTHH:MM:SS"`, so the string range read is correct |
| `Vehicle::new_ice` (5 args), `Trip::test_ice_trip` (4 args), `AppState::enable_read_only(&str)` | [models.rs:94](../../../src-tauri/core/src/models.rs#L94), [models.rs:241](../../../src-tauri/core/src/models.rs#L241), [app_state.rs:160](../../../src-tauri/core/src/app_state.rs#L160) |
| `places_cmd` re-export, `crate::places::normalise` | `commands_internal/mod.rs`, `places/mod.rs:8` |
| No UNIQUE on `license_plate` | baseline migration: two test vehicles with `TEST-1` are OK |
| `WDIO_SERVER_URL` env name in `utils/mcp.ts` | same as [utils/db.ts:39](../../../tests/integration/utils/db.ts#L39) |
| Test counts | Task 2: 14, Task 3: 8, Task 4: 6 (5 HTTP + guard) |
| 88 interfaces used by plan 89 | `PlaceRow` (7 fields, `Selectable`), `Place` (camelCase), `Trip.origin_place_id`, migration `2026-10-05-100000_places_as_entities`, `place-row` + `data-place-id`, `list_places_internal(db)` lists all places |
| CI toolchain | [test.yml:78](../../../.github/workflows/test.yml#L78) `dtolnay/rust-toolchain@stable`, so the 1.88 bump does not break CI |

## Iterations

1. First pass over the full plan, the spec, the code and the 88 plan: C1, I1 to I3, M1 to M8.
2. Second pass: a grep of plan 89 for each symbol that 88 renames or removes
   (`Trip::from`, `From<TripRow>`, `normalize_location`, `distinct_trip_places`,
   `upsert_place`, `save_place`, `clear_place`, `origin: data.origin`). Only C1 matched.
   No new findings, so the review stops.

## Addendum: second reviewer pass

A second plan-review run found the same C1, I2, I3, M2, M3, M5 and M8. It also
found these items, which the first pass does not list. They are all Minor. With
them, the counts are 1 Critical, 3 Important, 14 Minor. The recommendation stays
**Needs Revisions**.

- [ ] **M9. `list_vehicles_read` duplicates `get_vehicles_internal`.**
  [vehicles.rs:10](../../../src-tauri/core/src/commands_internal/vehicles.rs#L10) has the
  same body (`db.get_all_vehicles().map_err(|e| e.to_string())`). Use
  `get_vehicles_internal` in Task 3 and Task 4, and remove `list_vehicles_read`. The
  name has no guard word, so `mcp_module_has_no_write_path` still passes.
- [ ] **M10. `insert_test_place` duplicates a 88 helper.** The
  [88 plan](../../_done/88-places-as-entities/02-plan.md) (Task 2, Step 5) adds
  `#[cfg(test)] Database::ensure_place_for_test(&self, name) -> Uuid`. Use it in
  Tasks 1, 3 and 4 in place of the raw-SQL helper.
- [ ] **M11. `$readOnly` does not exist.** The read-only flag is
  `$appModeStore.isReadOnly` ([appMode.ts](../../../src/lib/stores/appMode.ts)). Write
  the real name in Task 5 Step 4.
- [ ] **M12. Svelte event syntax is not fixed.** Task 5 uses `on:click`. The repo uses
  both styles (`onclick=` in [+page.svelte](../../../src/routes/+page.svelte), `on:click`
  in [settings/+page.svelte](../../../src/routes/settings/+page.svelte)), and the 88 plan
  does not say which style the new Miesta page uses. Svelte 5 rejects both styles in
  one component. Tell the implementer to use the style of the 88 page.
- [ ] **M13. The doc-check list has no place to go.** The spec says to list each
  feature doc as "updated" or "checked, no change" in the PR. Task 8 pushes to `main`,
  and no PR exists. Put the list in the commit body of Task 7, or in a `03-status.md`.
- [ ] **M14. The Task 0 Step 1 grep is too loose.** `grep -i "places as entities\|88"`
  also matches any commit hash or message that contains "88". Grep for
  `places_as_entities` (the migration name) or for the 88 commit subject.

Also for C1: add `Trip::from_row`, `place_names`, `Place::from_row` and
`ensure_place_for_test` to the Task 0 Step 2 greps, so that the next drift shows
before Task 1 starts.
