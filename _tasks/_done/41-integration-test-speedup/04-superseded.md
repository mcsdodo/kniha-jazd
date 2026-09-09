**Date:** 2026-09-09
**Subject:** Why this task is archived without being implemented
**Status:** Complete (superseded)

# Superseded

This task was written for the Tauri desktop app. [Task 73](../_done/73-web-first-migration/)
retired that app ([ADR-030](../../DECISIONS.md)), and it deleted every file this plan
edits. The surviving work moved to [Task 82](../82-integration-db-reset/).

## What the plan asked for, and what happened to it

| Plan item | State on 2026-09-09 |
|---|---|
| `reset_test_database` Tauri IPC command in `src-tauri/src/commands.rs` | Dead. There is no Tauri command layer. The harness resets over JSON-RPC, in `resetDatabase()` at [wdio.server.conf.ts:140](../../tests/integration/wdio.server.conf.ts). |
| Replace file deletion, journal unlink and the lock retries | **Delivered** by Task 73. The harness deletes no files and waits for no IPC bridge. |
| Store reset helper `window.__TEST_RESET_STORES__` | Never built. Task 82 does not want it either: the per-test `browser.refresh()` gives the same isolation, and the plan is to drop the refresh only where a spec does not need it. |
| Drop `browser.refresh()` between tests | **Open.** `beforeTest` still refreshes, and the specs hold 95 more `browser.refresh()` calls. |
| Edits to `tests/integration/wdio.conf.ts` | The file is deleted. |
| [_plan-review.md](_plan-review.md) findings | All three "Critical" items are Diesel and `State<Database>` specifics for code that no longer exists. The review is not reusable as a checklist. |

## Two findings that carry over to Task 82

1. **The RPC reset is incomplete, not only slow.** `resetDatabase()` calls only
   `get_vehicles`, `get_trips_for_year`, `delete_trip` and `delete_vehicle`. It clears no
   receipts, routes, places or settings, and it sweeps trips for `[year-1, year, year+1]`
   only. Every error is swallowed, so a trip outside that window blocks `delete_vehicle`
   on the foreign key and the vehicle leaks too, without a message. This is the
   cross-spec half of [_TECH_DEBT/07](../_TECH_DEBT/07-integration-db-reset-broken.md).

2. **Do not reuse the guard.** The plan gates the reset command on
   `env::var("KNIHA_JAZD_DATA_DIR").is_err()`. That variable is a production knob today:
   [Dockerfile.web:58](../../Dockerfile.web) and
   [docker-compose.web.yml:14](../../docker-compose.web.yml) both set it to `/data`. The
   guard would arm a destructive truncate on the live instance.

## Numbers

The savings table in [01-task.md](01-task.md) (4 to 5 seconds per test, 39 tier-1 tests,
"50% reduction") is Tauri-era and dead. Task 82 measures a new baseline first.
