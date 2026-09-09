# Tech Debt: Integration-test per-test DB cleanup silently broken

**Date:** 2026-07-15
**Priority:** Medium
**Effort:** Medium (2-8h)
**Component:** `tests/integration/wdio.conf.ts` (deleted)
**Status:** Moot — resolved 2026-09-04

## Resolution

Moot as of [Task 73](../_done/73-web-first-migration/). The launcher/worker split this
describes was specific to `wdio.conf.ts`, the tauri-driver harness, which was deleted
when the desktop app was retired ([ADR-030](../../DECISIONS.md)). The surviving harness
(`wdio.server.conf.ts`) resets the database over RPC from the worker, so there is no
cross-process hook to get wrong.

Note the *cross-spec* state sharing this file also touches is NOT resolved: specs in a
tier still share one backend and one database, which is why `datetime-is-order` can fail
under a full-tier run and pass in isolation. `specFileRetries: 2` absorbs it today. That
part belongs to [Task 82](../82-integration-db-reset/), which also names the tables the
RPC reset misses today (receipts, routes, places, settings).

## Problem

The `beforeTest` hook in [wdio.conf.ts](../../tests/integration/wdio.conf.ts) is supposed to
delete the test database before every test ("fresh DB per test"). It never does:

- `testDataDir` is a module-scope variable set in `onPrepare`, which runs in the wdio
  **launcher** process.
- `beforeTest` runs in the **worker** process, where the module is re-imported and
  `testDataDir` is still `''`.
- `getTestDbPath()` therefore returns `join('', 'kniha-jazd.db')` — a relative path that
  never exists — so `existsSync()` is false and the deletion silently no-ops.
- Zero `"Cleaned up test database"` lines appear in any full-suite log.

Every test in a wdio run shares one database. The suite passes anyway because specs seed
their own vehicles/trips and query vehicle-scoped data — until a spec seeds data that is
NOT vehicle-scoped.

## Impact

- Discovered 2026-07-15: the new [multi-invoice.spec.ts](../../tests/integration/specs/tier2/multi-invoice.spec.ts)
  seeded receipts that leaked into [receipts.spec.ts](../../tests/integration/specs/tier2/receipts.spec.ts)
  (`getReceipts(year)` is not vehicle-scoped, and the tests picked `receipts[0]`),
  failing 3 tests only in full-sweep order. Worked around by (a) an `after()` cleanup
  hook in multi-invoice.spec deleting its seeded receipts + placeholder files, and
  (b) selecting receipts by `fileName` instead of `[0]` in receipts.spec.
- Any future spec seeding receipts (or other non-vehicle-scoped data) must remember to
  clean up after itself, or it will poison later specs in order-dependent ways.
- Test isolation is an illusion; "fresh DB per test" comments in specs are wrong.

## Root Cause

wdio launcher/worker process split: module state set in `onPrepare` does not exist in the
worker that runs `beforeTest`. Probably broken since the hooks were split across
processes; masked because nothing leaked visibly.

## Recommended Solution

Superseded. This section was written for the Tauri harness. Task 41 was archived unbuilt
(see [_done/41-integration-test-speedup/04-superseded.md](../_done/41-integration-test-speedup/04-superseded.md));
[Task 82](../82-integration-db-reset/) carries the surviving work. Keep the original text
below for the reasoning, not for the file paths:

1. Add a test-only `reset_database` IPC command (guarded by `KNIHA_JAZD_DATA_DIR` /
   debug builds) that truncates all tables in the open connection — no file locking
   issues, works in the worker, works in Docker mode too.
2. Call it from `beforeTest` instead of `unlinkSync(getTestDbPath())`.
3. Alternatively (quick fix): derive the path in the worker from
   `process.env.KNIHA_JAZD_DATA_DIR` (workers do inherit it — `seedReceipt` relies on
   it). Beware: enabling deletion after it never ran may surface Windows file-lock
   failures (SQLite keeps the file open) and change the behavior of all specs at once —
   verify the whole suite when doing this.

## Related

- [Task 82 — Integration DB Reset](../82-integration-db-reset/) -- the live successor
- [Task 41 — Integration Test Speedup](../_done/41-integration-test-speedup/) -- archived unbuilt
- [Task 66 — Multi-Invoice Support](../66-multi-invoice/) (where the leak surfaced)
