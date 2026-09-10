**Date:** 2026-09-10
**Subject:** One guarded backend command resets the whole test state, so specs stop leaking into each other
**Status:** Planning

**Source:** [_TECH_DEBT/07-integration-db-reset-broken.md](../_TECH_DEBT/07-integration-db-reset-broken.md) (the cross-spec half)
**Supersedes:** [Task 41](../_done/41-integration-test-speedup/) (archived unbuilt, see its [04-superseded.md](../_done/41-integration-test-speedup/04-superseded.md))
**Blocks:** [Task 83](../83-integration-test-sharding/) -- sharding reorders specs, which is unsafe until the reset is complete

## Goal

Give the integration harness one call that returns the application to a known
empty state. Today no such call exists, so each spec cleans up the parts it
remembers, and the parts nobody remembers leak into the next spec.

## This is not a speedup

Task 41 sold this work as a speedup. That was measured and it is false. On an
instrumented run of `tier2/legal-compliance.spec.ts` (12 tests, 22.9 s):

| Hook | Per test | Across all 190 tests |
|---|---|---|
| `resetDatabase()` (the loop this task replaces) | 8 to 25 ms (avg 17) | about 3.2 s |
| `browser.refresh()` | 75 to 122 ms (avg 99) | about 19 s |
| The test body | about 1.8 s | about 340 s |

The measurement is local, under `xvfb-run`, on one spec. CI bounds it but does not
isolate it: in run `34349086458`, `backup-restore.spec.ts` runs 7 tests in 3.2 s, so
reset plus refresh plus test body is under 460 ms per test there. If the CI runner is
2 to 3 times slower than this machine, the refresh costs 40 to 60 s across the suite,
not 19 s. Either way the reset loop is milliseconds, which is what this task turns on.

Do not reintroduce a speed target here. The speed work is [Task 83](../83-integration-test-sharding/).

## The problem

`resetDatabase()` at [wdio.server.conf.ts:140](../../tests/integration/wdio.server.conf.ts)
calls four commands: `get_vehicles`, `get_trips_for_year`, `delete_trip`,
`delete_vehicle`. Three gaps follow:

1. **Receipts survive.** `delete_vehicle` sets `receipts.vehicle_id` to NULL, it
   does not delete the rows ([db.rs:300-316](../../src-tauri/core/src/db.rs)).
   This is the leak that made `multi-invoice.spec.ts` poison `receipts.spec.ts`.
2. **Most settings are not in the database.** Hidden columns, date prefill mode,
   infer trip times, HA, Paperless, receipt settings and backup retention live in
   `local.settings.json` in the data dir
   ([settings.rs:82-95](../../src-tauri/core/src/settings.rs), `LocalSettings::load`).
   Only `company_name`, `company_ico` and `buffer_trip_purpose` are DB rows
   ([schema.rs:47](../../src-tauri/core/src/schema.rs)). In Docker mode that file
   sits inside the container. No RPC clears it, so specs restore it by hand:
   `column-visibility.spec.ts:31` does, `time-inference-toggle.spec.ts:25` and
   `date-prefill.spec.ts:19` do not.
3. **Failures are silent.** Every call sits in `catch { /* ignore */ }`, with an
   outer `console.warn`. A trip outside the swept years (`year-1`, `year`,
   `year+1`) blocks `delete_vehicle` on the foreign key, and both the trip and the
   vehicle stay, with no message.

## Requirements

- One command empties every table and resets `local.settings.json`.
- The command is unusable in production.
- The harness calls it once and fails the run if it fails.
- No spec needs to restore a setting by hand afterwards.

## Approach

### Backend

- New `reset_database` command, dispatched in
  [server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs),
  implemented in a new `commands_internal/test_reset.rs`.
- It runs one transaction over all 8 tables, children before parents:
  `paperless_trip_links`, `trip_routes`, `receipts`, `trips`, `routes`, `places`,
  `settings`, `vehicles`. Confirm each foreign key while implementing; the list
  comes from [schema.rs](../../src-tauri/core/src/schema.rs).
- It then deletes `local.settings.json` (`constants::paths::SETTINGS_FILENAME`)
  from the data dir, so every spec starts on real defaults.
- It honours `check_read_only!` like every other write command.

### The guard is a runtime env var, not a compile-time gate

Add `TEST_RESET = "KNIHA_JAZD_TEST_RESET"` to `constants.rs` beside `MOCK_GEMINI_DIR`
and `MOCK_GEOCODER_DIR` (line 69-73). The command returns an error unless it is set.

A `#[cfg(debug_assertions)]` or a cargo feature cannot work: CI tests the Docker
image and then republishes **that same artifact** as `:main`
([ADR-031](../../DECISIONS.md)), so the tested binary is the shipped binary. The
command must exist in the published image and be disarmed there by the missing
variable. Production sets `KNIHA_JAZD_DATA_DIR` (`Dockerfile.web:58`,
`docker-compose.web.yml:14`), so that variable is not usable as the guard -- which
is the mistake Task 41's plan made.

### Harness

- `resetDatabase()` becomes one POST. It stops swallowing errors: a failed reset
  fails the run.
- Set `KNIHA_JAZD_TEST_RESET: '1'` in the spawned-server env block
  ([wdio.server.conf.ts:266-276](../../tests/integration/wdio.server.conf.ts)).
- Add `-e KNIHA_JAZD_TEST_RESET=1` to both `docker run` blocks in
  [test.yml](../../.github/workflows/test.yml) (lines 148 and 236).
- Remove the per-spec settings restores that the command makes redundant.

## Risks

- **Every spec now starts on default settings.** A spec that silently relied on a
  value left behind by an earlier spec will fail. That is the point, but expect
  failures on the first full sweep and fix them as real gaps.
- **A destructive command ships in the image.** Mitigation: the env var, plus a
  backend test asserting the command refuses when it is unset.
- The settings defaults must come from a missing file, not from seeded rows.
  Verify this rather than assume it.

## Acceptance criteria

- [ ] Backend test: `reset_database` returns an error when `KNIHA_JAZD_TEST_RESET` is unset.
- [ ] Backend test: with the var set, all 8 tables are empty and `local.settings.json` is gone.
- [ ] The harness calls the command once and fails loudly when it errors.
- [ ] No spec restores a setting by hand any more.
- [ ] Full integration sweep is green.
- [ ] Re-check whether `datetime-is-order` still needs `specFileRetries: 2`.

## Out of scope

- `browser.refresh()` in `beforeTest` and the 95 in-spec calls. Measured at 99 ms;
  removing them is not worth the isolation risk.
- Store reset helpers in the frontend (`window.__TEST_RESET_STORES__`). Task 41
  wanted them; the refresh already does that job.
- Any CI timing change. That is [Task 83](../83-integration-test-sharding/).
