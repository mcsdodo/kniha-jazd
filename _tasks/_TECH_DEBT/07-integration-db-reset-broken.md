# Tech Debt: Integration-test per-test DB cleanup silently broken

**Date:** 2026-07-15
**Priority:** Medium
**Effort:** Medium (2-8h)
**Component:** [tests/integration/wdio.conf.ts](../../tests/integration/wdio.conf.ts)
**Status:** Open

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

Fold into [Task 41](../41-integration-test-speedup/) (IPC-based DB reset), which replaces
file deletion with an explicit backend reset command:

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

- [Task 41 — Integration Test Speedup](../41-integration-test-speedup/)
- [Task 66 — Multi-Invoice Support](../66-multi-invoice/) (where the leak surfaced)
