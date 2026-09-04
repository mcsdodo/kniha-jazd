**Date:** 2026-09-04
**Subject:** Plan review — Web-first migration (retire the Tauri desktop app)
**Reviewed:** [03-plan.md](./03-plan.md) against [01-task.md](./01-task.md), [02-research.md](./02-research.md) and the codebase as built
**Status:** Closed — all 16 findings applied to [03-plan.md](./03-plan.md) on 2026-09-04

## Resolution

All 16 findings applied to [03-plan.md](./03-plan.md) on 2026-09-04. Every Critical was
independently verified against the code before being accepted — C1's "Prvý záznam"
divergence, C2's `set X=Y&&`-under-`sh` no-op, and C3's `seedReceipt` filesystem
requirement all check out, as do the Important and Minor claims spot-checked (I2's three
`capabilities.mode` sites, I6's broken `/release` steps, M2's already-present
`waitForDisplayed`, M3's surviving `getDbLocation` callers).

Notable consequences beyond the plan edits:

- [02-research.md §2.1](./02-research.md) carried the C1 error ("only the column/sort
  arguments are lost") and has been corrected in place, with the correction dated and
  attributed rather than silently rewritten.
- [01-task.md](./01-task.md) R1.1 restated as a three-part gap.
- Task 6 split: **Task 6** (receipts, fixture paths) and **Task 6b** (`seedReceipt` across
  the container boundary) — 20 tasks now, not 19.
- The path helper gained a second mapping and was renamed
  `backendFixturePath` / `backendWorkPath`, because "data" was the word both mappings
  wanted.
- I5 **rejected as written**: the plan no longer shrinks `backend-tests` to ubuntu-only.
  The reviewer was right that it is a coverage reduction smuggled into a cleanup step.
- One item escalated to the user rather than decided: empty-year export behaviour (desktop
  emits a placeholder-only document, core errors). The plan keeps the error and flags it at
  the Phase 1 gate.

## Verdict (as filed)

**3 Critical · 6 Important · 7 Minor — Needs Revisions.**

The shape of the plan is right. Sequencing is genuinely load-bearing and the plan knows it:
closing gaps first, proving every test fix green under Docker *while the Tauri harness still
exists*, and putting the irreversible deletions last. Task 1 and Task 3 are correct down to
the field names — I checked every API they assume. The phase gates are real gates.

The three Critical findings are all the same species: a place where the plan's stated
diagnosis does not match what the code does, so the task as written would not deliver what
the phase promises. One is a use-case that silently disappears (the export's opening row),
one is a CI job that would not run the suite it exists to run, and one is a spec the plan
believes a path helper fixes when the blocker is somewhere else entirely.

---

## Critical

### [x] C1 — The printed logbook loses its "Prvý záznam" opening row

R1.1 is described as a two-argument gap. It is a three-part gap, and the third part is the
one that changes what the legal document says.

[desktop/src/commands/export_cmd.rs](../../src-tauri/desktop/src/commands/export_cmd.rs)
`export_to_browser` builds a synthetic `Trip` with `Uuid::nil()`, purpose `"Prvý záznam"`
and `odometer: grid_data.year_start_odometer`, pushes it onto the grid, and injects the
matching `fuel_remaining` / `trip_numbers` / `odometer_start` entries before rendering.
[core/src/commands_internal/export_cmd.rs](../../src-tauri/core/src/commands_internal/export_cmd.rs)
`export_html_internal` does none of that. [export.rs:332](../../src-tauri/core/src/export.rs)
renders the row specially (`is_first_record`), so the machinery is in core — only the caller
that builds the row is not.

The divergence is already documented in the repo, in the doc comment of
[route_maps_tests.rs:491](../../src-tauri/core/src/commands_internal/route_maps_tests.rs):

> "The two export paths differ: desktop prepends a synthetic 'Prvý záznam' row **and**
> honours the user's sort direction, server mode does neither."

The plan closes the second half of that sentence and leaves the first. After Phase 8
`export_html` is the only export path, so the printed logbook permanently loses its
year-opening odometer baseline — and stops matching the on-screen grid, which still renders
the row ([TripGrid.svelte:435](../../src/lib/components/TripGrid.svelte),
`FIRST_RECORD_ID`). That is a functional regression of exactly the kind
[Goal 2](./01-task.md#goals) forbids, and Task 1's test would pass while it ships.

**Fix:** move the synthetic-first-record block into `export_html_internal` as part of Task 1,
with an assertion that the nil-uuid row renders and carries `year_start_odometer`. Two
follow-ons while you are there: `export_html_internal` returns
`Err("No trips found for this year")` on an empty year where desktop pushes the first record
regardless — decide which behaviour survives; and
`both_export_modes_cite_the_same_record_for_the_same_trip`'s doc comment goes stale the
moment Task 1 lands, so update it rather than leaving a comment that describes the old world.

### [x] C2 — The new env-pinned CI job sets no environment, so it will not run the env suite

The [package.json](../../package.json) scripts use Windows `set X=Y&&` syntax. On `ubuntu-latest` that runs under `sh`,
where `set WDIO_EXTERNAL_SERVER=1` sets positional parameters and exports nothing. This is
already known in the repo: the existing `integration-test-docker` job does **not** rely on
the script's prefix, it passes the variables in the step's own `env:` block
([test.yml:372-376](../../.github/workflows/test.yml)).

Task 8 Step 3 adds:

```yaml
      - name: Run env-pinned integration tests
        run: npm run test:integration:docker:env
```

with no `env:` block. At config load `EXTERNAL_SERVER` and `ENV_PINNED` are both false, so
`getSpecs()` returns all four tier globs instead of `./specs/env/**`, and `onPrepare` skips
the external-server branch and tries to spawn a binary that CI never built. The job that
exists solely to close the standing I1 violation would run the wrong suite and fail — or, if
someone later "fixes" it by loosening the wait, pass without having run the 8 tests at all.

**Fix:** give the step the same `env:` block the docker job uses —
`WDIO_SERVER_MODE: '1'`, `WDIO_EXTERNAL_SERVER: '1'`, `WDIO_ENV_PINNED: '1'`. The same
applies to Step 4's local verification if it is run under Git Bash rather than cmd.

### [x] C3 — Task 6 does not make [multi-invoice.spec.ts](../../tests/integration/specs/tier2/multi-invoice.spec.ts) runnable in Docker; the diagnosis is wrong

The plan treats both Docker skips as one problem: "the container cannot see
[tests/integration/data/](../../tests/integration/data/). Mount it and add a path helper."

That is right for [receipts.spec.ts](../../tests/integration/specs/tier2/receipts.spec.ts) —
six literal `join(__dirname, '..', '..', 'data', 'invoices')` at lines 148, 227, 296, 384,
432, 489, all handed to `setReceiptsFolderPath`. `backendDataPath()` plus the `/testdata:ro`
mount fixes those.

It is wrong for [multi-invoice.spec.ts](../../tests/integration/specs/tier2/multi-invoice.spec.ts),
which contains **no such joins**. It seeds through `seedReceipt`
([utils/db.ts:681](../../tests/integration/utils/db.ts)), which:

1. reads `getTestDataDir()` = `process.env.KNIHA_JAZD_DATA_DIR` — unset in the *test process*
   in Docker mode — and throws `'KNIHA_JAZD_DATA_DIR not set — seedReceipt requires the
   sandboxed test data dir'` before doing anything else;
2. writes a placeholder file from the test process into `<dataDir>/seeded-receipts`;
3. hands the backend that same absolute path via `set_receipts_folder_path` and scans.

The spec's own header says so: *"Docker mode is skipped: seedReceipt writes placeholder files
that the backend must see on the same filesystem."* A read-only mount of the repo's fixture
directory does not give the test process a writable location the backend can also read.

Second-order: the `after()` cleanup at
[multi-invoice.spec.ts:119-121](../../tests/integration/specs/tier2/multi-invoice.spec.ts)
is guarded by `if (dataDir)`, so in Docker mode it silently no-ops and the seeded receipts
persist in the container's `/data` volume — which is precisely the cross-spec poisoning its
own comment warns about ("leftover seeded receipts poison later specs").

**Fix:** the new [paths.ts](../../tests/integration/utils/paths.ts) needs a second mapping for the *writable* data dir (host `$PWD/data` ↔
container `/data`, which the CI `docker run` already bind-mounts), `seedReceipt` needs to use
it instead of `getTestDataDir()`, and the cleanup needs the host side of that pair. Budget it
as its own task — it is a helper redesign, not the search-and-replace Task 6 describes.

---

## Important

### [x] I1 — Nothing gives the R1.1 fix an end-to-end test, though the plan says [export.spec.ts](../../tests/integration/specs/tier1/export.spec.ts) does

Task 2's closing note says *"the integration test for this lands in Task 7"*, and Task 7
Step 2 says *"The export spec exercises Phase 1's work end-to-end."* It does not.
[export.spec.ts](../../tests/integration/specs/tier1/export.spec.ts) asserts only that city
names, licence plate and company name appear in the exported HTML (lines 143-155). There is
no assertion about hidden columns and none about row order.

Worse, the spec is structurally toothless. Every assertion sits inside
`if (handles.length > originalHandles.length)`; the `else` branch asserts only when the URL
happens to contain `export` or `blob`; and a missing export button produces
`console.log('Export button not found, skipping test'); return;`. If the export window never
opens, the test passes having asserted nothing. Unskipping it buys a green tick, not
coverage — and under [I2](./01-task.md#coverage-invariants) the export-argument use-case
would remain uncovered end-to-end after desktop is gone.

**Fix:** in Task 7, add real assertions (hide a column in the grid → export → assert the
header is absent; flip the sort → assert row order) and delete the silent-pass escapes.

### [x] I2 — Task 16 misses the three `$capabilities.mode === 'desktop'` branches, including a whole settings section

The frontend gates desktop behaviour three ways: `IS_TAURI`, `$capabilities.features.*`, and
`$capabilities.mode === 'desktop'`. Task 16 handles the first two and never mentions the
third. All three of its sites are in
[settings/+page.svelte](../../src/routes/settings/+page.svelte):

- line 87 — the desktop `revealSecret` path (the server path is the PIN-gated one from
  [Task 69](../_done/69-pin-gated-secret-reveal/), and must survive)
- line 759 — the `getServerStatus` load
- line 1531 — the entire **Server Mode** section: port input, start/stop button, server URL
  display, error line, plus its `settings.serverMode*` i18n keys

That section is driven by `startServer` / `stopServer` / `getServerStatus`, which are
desktop-only commands (confirmed below) and die in Phase 8. Task 16 Step 4's verification is
`grep -rn "@tauri-apps\|IS_TAURI" src/`, which matches none of them — so the plan's own check
would report clean while dead UI calling non-existent commands ships.
[api.ts](../../src/lib/api.ts)'s `getServerStatus` / `startServer` / `stopServer` /
`getOptimalWindowSize` are likewise unlisted.

Related factual error: Task 3 Step 5 says `IS_TAURI` "is also used at lines 87 and 759, so it
stays for now". It is not — those are `capabilities.mode` checks. In that file `IS_TAURI`
appears only at the import (line 17) and line 695. Once Task 3 removes the 695 branch the
import is dead and has to go with it.

### [x] I3 — R5's [db_location.rs](../../src-tauri/core/src/db_location.rs) cleanup has no task

[01-task.md R5](./01-task.md#r5--delete-the-desktop-surface) lists "Dead lock-file /
`move_database` logic in [db_location.rs](../../src-tauri/core/src/db_location.rs)". No task
in the plan touches that file. Related loose end: `check_target_has_db` stays dispatched
([dispatcher.rs:479](../../src-tauri/core/src/server/dispatcher.rs)) while its only
consumers — the settings "Change Location" UI and the integration tests Task 13 deletes — go
away. Either remove the arm with the feature or state why it stays.

### [x] I4 — The final npm-script set is never named, and the plan's own I1 check cannot pass

After Tasks 11 and 13 the surviving scripts are at least: `test:backend`, `test:integration`,
`test:integration:tier1/2/3`, `test:integration:server`, `test:integration:server:tier1`,
`test:integration:server:env`, `test:integration:docker`, `test:integration:docker:tier1`,
`test:integration:docker:env`, `test:all`. The job table in
[01-task.md R4](./01-task.md#r4--rewrite-both-pipelines) invokes four things. So most of
those scripts are invoked by no job, and the literal I1 criterion — *"every `test:*` script
is invoked by a job in test.yml, or the script no longer exists"* — fails. Task 17 Step 3 is
where that surfaces, i.e. after the point of no return.

Task 13 also contradicts itself in one step: "remove `test:integration:build`,
`test:integration:tier1/2/3`" followed by a JSON block that redefines tier1/2/3.

**Fix:** write the final script list into the plan and reconcile it with I1 — either delete
the convenience aliases or restate I1 to exempt aliases that only set `TIER` around a script
a job does invoke.

### [x] I5 — Task 14 shrinks `backend-tests` to ubuntu-only; 01-task never asked for that

R4 says "keep `backend-tests`". Task 14 Step 1 adds "shrink the matrix to `ubuntu-latest`
only". That is a real coverage reduction for a crate with platform-conditional code (DB
paths, `hostname`, lock files) that developers still build and run on Windows — the plan's
own Task 12 keeps a `win32` branch in `getBinaryPath()`. Either keep the matrix, or record
the trade-off with `/decision` rather than folding it into a step about deleting Windows
*integration* jobs.

### [x] I6 — Task 18's doc list omits [.claude/skills/](../../.claude/skills/), and `/release` breaks outright

[release-skill/SKILL.md](../../.claude/skills/release-skill/SKILL.md) bumps
`src-tauri/desktop/tauri.conf.json` (step 3 — deleted in Phase 8), runs
`npm run test:integration:tier1` (step 4 — repointed in Task 13), runs `npm run tauri build`
(step 5 — deleted), documents the `TAURI_SIGNING_PRIVATE_KEY` warning as an expected
non-failure, and reports an NSIS installer path (step 7 — gone). After Phase 8 the release
workflow the project uses is broken, and [D2](./01-task.md#resolved-decisions) changes what a
release *is* (ghcr image, no GitHub Release) — that belongs in the skill, not only in an ADR.

[test-update-skill](../../.claude/skills/test-update-skill/SKILL.md) is entirely about testing Tauri
auto-update and becomes dead weight; [code-review-skill](../../.claude/skills/code-review-skill/SKILL.md),
[test-review-skill](../../.claude/skills/test-review-skill/SKILL.md) and
[verify-skill](../../.claude/skills/verify-skill/SKILL.md) also carry Tauri references. All of these fail the acceptance criterion
"`grep -ri tauri` returns only historical references", and Task 17 Step 3's grep catches them
only after Phase 8.

---

## Minor

### [x] M1 — No replacement dev loop

`npm run tauri:dev` is the documented daily workflow ([CLAUDE.md](../../CLAUDE.md), "Common
Commands"). Task 16 deletes the `tauri:*` scripts and nothing defines what replaces them.
[vite.config.ts](../../vite.config.ts) has no `/api/rpc` proxy, so `npm run dev` alone cannot
reach a backend. Add a `server.proxy` entry to `http://localhost:3456` and a documented
two-process loop (`cargo run -p kniha-jazd-web` alongside `npm run dev`), or a single script
that does both. Also: `stage:spa` and `dev:server` are not `tauri:*`-prefixed and so escape
the plan's deletion list, while [scripts/stage-spa.mjs](../../scripts/stage-spa.mjs) — which `stage:spa` calls — is
deleted in Task 15.

### [x] M2 — Task 10's "most likely fix" is already in the file

[ev-vehicle.spec.ts:229](../../tests/integration/specs/existing/ev-vehicle.spec.ts) already
has `await bevBadge.waitForDisplayed({ timeout: 5000 })`. The plan proposes adding exactly
that. Say so, or the implementer "applies" a no-op change and concludes it is fixed. The
TODO's own suspicion — `createBevVehicleViaUI` not completing the creation — is where to
start.

### [x] M3 — Task 13 over-deletes a helper

It says to delete the `Database Move Commands` block "and its `getDbLocation` /
`checkTargetHasDb` helpers". `getDbLocation` is used by three tests that survive
([receipt-settings.spec.ts](../../tests/integration/specs/tier2/receipt-settings.spec.ts)
lines 176, 190, 206). Only `checkTargetHasDb` becomes unused.

### [x] M4 — The new [paths.ts](../../tests/integration/utils/paths.ts) points at the wrong file to keep in sync

Its doc comment says to keep the mount target in sync with
[docker-compose.web.yml](../../docker-compose.web.yml). That file is the production-shaped
deployment compose — [skip.ts](../../tests/integration/utils/skip.ts) says as much — and
should not mount test fixtures. Only [test.yml](../../.github/workflows/test.yml) and the
local `docker run` need the mount.

### [x] M5 — Task 5 turns mock Gemini on for the whole Docker suite

`-e KNIHA_JAZD_MOCK_GEMINI_DIR=/testdata/mocks` applies to every tier, not just the two
newly-unskipped describes. That is a behaviour change for specs that pass today. Worth one
line saying it is intended, and worth watching at the Phase 3 gate.

### [x] M6 — `:ro` on the fixture mount is asserted rather than verified

Receipt scanning appears to read only, so `:ro` looks safe — but the plan states it as a rule
("the container has no business writing into the repo") without naming what breaks if the
pipeline ever moves or rewrites a scanned file. Keep it, and add a line saying a permission
error in Task 6 is the first thing `:ro` would explain.

### [x] M7 — Line-number and claim drift

[export.spec.ts](../../tests/integration/specs/tier1/export.spec.ts)'s skip is at line 27 (plan says 25-29); [backup-restore.spec.ts](../../tests/integration/specs/tier2/backup-restore.spec.ts)'s at 161
(159-161); [receipt-settings.spec.ts](../../tests/integration/specs/tier2/receipt-settings.spec.ts)'
at 158 (154). And [release.yml](../../.github/workflows/release.yml)'s `docker-image` job already declares
`needs: [check-tests, backend-tests]`, so Task 14 Step 2's "repoint `docker-image`'s `needs:`"
is a no-op. None of these block; each costs the implementer a moment.

---

## What was verified and is correct

Recorded so the next pass does not re-check it.

- Task 1's test compiles against reality: `ServerState`'s four fields, `Vehicle::new_ice`,
  `Database::in_memory`, `db.create_vehicle` / `save_settings` / `create_trip` / `get_trip`,
  `Settings::default`, `tempfile`, and the `dispatch_async` / `dispatch_sync` signatures all
  match. `test_state()` exists at
  [dispatcher.rs:903](../../src-tauri/core/src/server/dispatcher.rs).
- `hidden_columns` and `sort_direction` really are consumed by `generate_html`
  ([export.rs:249, 288](../../src-tauri/core/src/export.rs)), and `"time"` really is one of
  the five hideable columns — Task 1's `CAS-MARKER` trick works as described.
- `version.workspace = true` in both `core` and `web`; workspace version `0.43.0` equals
  [package.json](../../package.json)'s. So `env!("CARGO_PKG_VERSION")` in Task 3 reports the
  number the settings page should show.
- The web binary's env contract (`PORT`, `KNIHA_JAZD_DATA_DIR`, `DATABASE_PATH`,
  `STATIC_DIR`) matches Task 12 exactly
  ([web/src/main.rs](../../src-tauri/web/src/main.rs)), and adapter-static outputs to
  `build/`.
- The desktop crate has exactly 2 tests, both in
  [static_dir.rs](../../src-tauri/desktop/src/static_dir.rs) — Task 15's "drops by exactly 2"
  checkpoint is right.
- Diffing all 76 desktop `#[tauri::command]`s against the 79 dispatcher commands leaves only
  `export_to_browser`, `get_optimal_window_size`, `get_server_status`, `start_server`,
  `stop_server`, `move_database`, `reset_database_location` as desktop-only. Every one dies
  legitimately **except** `export_to_browser` — see C1. So R5 loses no other use-case.
- The backup-restore skip really is stale: `restore_backup` is dispatched
  ([dispatcher.rs:766](../../src-tauri/core/src/server/dispatcher.rs)), round-trip tested at
  1068, and `capabilities_handler` reports `restore_backup: true`
  ([server/mod.rs:90](../../src-tauri/core/src/server/mod.rs)). Its skip comment's claims are
  all false.
- `move_database` and `reset_database_location` are genuinely absent from the dispatcher, so
  deleting their integration tests with the feature is I2-legal.
- All 9 skip constructs the task enumerates exist, in the files it names.
- The `sed` line Task 15 removes from [Dockerfile.web](../../Dockerfile.web) matches the
  plan's quoted text exactly.
- The receipts specs' six `invoicesPath` joins are exactly six, at the lines Task 6 implies.
