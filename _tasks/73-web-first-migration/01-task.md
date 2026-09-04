**Date:** 2026-09-03
**Subject:** Retire the Tauri desktop app; make the web/Docker deployment the only target
**Status:** Planning

# Task 73: Web-First Migration

## Context

[ADR-024](../../DECISIONS.md#adr-024-homelab-server-is-the-canonical-deployment-desktop-becomes-a-browser-client)
already named the always-on Docker deployment canonical and demoted the desktop app to
"a browser client that keeps working". This task finishes that move: the desktop app is
**removed**, not demoted, and the Docker container + browser UI become the only thing
built, shipped, and tested.

The structural groundwork from [Task 58](../_done/58-tauri-workspace-split/) makes this
cheap: [kniha-jazd-core](../../src-tauri/core/) holds 509 tests and no Tauri dependency,
while [kniha-jazd-desktop](../../src-tauri/desktop/) holds 2. See
[02-research.md](./02-research.md) for the full coupling inventory.

## User story

> As the sole operator of this logbook, I run one always-on instance on my homelab and
> reach it from a browser on any device. I do not install anything. I do not want CI
> spending 15 minutes per run proving a desktop build I never launch still works.

## Goals

1. **One deployment artifact:** `ghcr.io/mcsdodo/kniha-jazd-web:vX.Y.Z`. No installers,
   no updater, no code signing.
2. **No functional regression vs. today's desktop app.** Web-mode export currently
   ignores column visibility and sort direction — that must be fixed *before* desktop
   goes away, not after.
3. **Docker + Chrome is the primary test harness.** Every integration spec that is
   skipped in Docker mode today either runs, or has a written reason why it cannot.
4. **Faster, less brittle CI.** Drop the Windows/WebView2/tauri-driver path entirely.

## Non-goals

- Adding authentication. [ADR-017](../../DECISIONS.md#adr-017-lan-only-cors-without-authentication)
  and ADR-024's tailnet-trust model stand unchanged.
- Reviving Playwright as a second browser harness (see [D1](#resolved-decisions)).
- Reworking the receipts pipeline. ADR-024 already routes intake through Paperless.

## Coverage invariants

Two rules the migration must not weaken. They are invariants, not goals — a step that
breaks either is not done.

**I1 — Every test runs in GitHub Actions.** Not "can be run locally", not "runs in the
Docker job we happen to have". Every npm test script must be invoked by
[test.yml](../../.github/workflows/test.yml).

**I2 — Every surviving use-case has an end-to-end test** exercising frontend through
backend. Deleting a *feature* (and its tests) is allowed; deleting a *test* for a feature
that survives is not.

Today's baseline already violates I1 in one place, and the migration would newly violate
I2 in two — see [R2](#r2--restore-and-complete-test-coverage). These are the reason R2
grew beyond "unskip the Docker skips".

## Requirements

### R1 — Close functional gaps (must land before deletion)

| # | Gap | Evidence |
|---|-----|----------|
| R1.1 | Web export ignores `hidden_columns` and `sort_direction`, **and drops the "Prvý záznam" opening row** | [export_cmd.rs](../../src-tauri/core/src/commands_internal/export_cmd.rs) lines 50, 61-62 hardcode `Vec::new()` and a fixed `SORT_DIRECTION`; desktop [export_cmd.rs](../../src-tauri/desktop/src/commands/export_cmd.rs) passes both through **and prepends a synthetic `Uuid::nil()` row carrying `year_start_odometer`**. All three differences are documented at [route_maps_tests.rs:491](../../src-tauri/core/src/commands_internal/route_maps_tests.rs) |
| R1.2 | App version is invisible in web mode | [settings/+page.svelte](../../src/routes/settings/+page.svelte) line 695 gates `getVersion()` on `IS_TAURI`; no `get_app_version` RPC exists |
| R1.3 | Receipt processing progress events have no web equivalent | [receipts_cmd.rs](../../src-tauri/desktop/src/commands/receipts_cmd.rs) line 174 emits via `app.emit`; the RPC path returns only the final `SyncResult` |

R1.1 and R1.2 are blocking. **R1.3 is an accepted loss** per
[D3](#resolved-decisions) — no replacement is built.

### R2 — Restore and complete test coverage

#### R2.a — Mode-conditional skips (the obvious set)

Of 141 integration `it`s, roughly 14 plus the 8 env-pinned ones do not run in Docker:

- [tier1/export.spec.ts](../../tests/integration/specs/tier1/export.spec.ts) (2) —
  skipped in *all* server mode, unconditionally
- [tier2/receipts.spec.ts](../../tests/integration/specs/tier2/receipts.spec.ts) —
  "Mismatch Detection E2E" and "Multi-Currency Receipts"
- [tier2/multi-invoice.spec.ts](../../tests/integration/specs/tier2/multi-invoice.spec.ts) —
  whole spec
- [env/env-managed-settings.spec.ts](../../tests/integration/specs/env/env-managed-settings.spec.ts)
  (8) — [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) throws on
  `WDIO_ENV_PINNED=1 + WDIO_EXTERNAL_SERVER=1`
- [tier2/backup-restore.spec.ts](../../tests/integration/specs/tier2/backup-restore.spec.ts) —
  "Backup Restoration"

The last one is a **stale skip**: `restore_backup` is dispatched at
[dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) line 766, round-trip
tested at line 1068, and `capabilities_handler` in
[server/mod.rs](../../src-tauri/core/src/server/mod.rs) reports `restore_backup: true`.
Delete the skip.

The receipts/multi-invoice skips are a mount problem, not a capability problem — the
container cannot see [tests/integration/data/](../../tests/integration/data/). Mount it
and add a host-to-container path mapping helper alongside
[utils/skip.ts](../../tests/integration/utils/skip.ts).

#### R2.b — The env suite has never run in CI (violates I1 today)

`test:integration:server:env` in [package.json](../../package.json) appears in **no
workflow**. Grepping [test.yml](../../.github/workflows/test.yml) and
[release.yml](../../.github/workflows/release.yml) for `WDIO_ENV_PINNED` returns nothing.
So the 8 tests in
[env-managed-settings.spec.ts](../../tests/integration/specs/env/env-managed-settings.spec.ts)
— covering env-pinned settings ([Task 68](../_done/68-env-managed-settings-ui/)) and the
PIN-gated reveal ([Task 69](../_done/69-pin-gated-secret-reveal/)) — are local-only today.

This is a pre-existing hole, not one the migration creates, but the migration is what
makes it cheap to close: a second container started with the fixture env vars is a
handful of lines in the Docker job, whereas the spawned-Tauri path needed a whole
separate wdio invocation. Add it as its own CI job.

#### R2.c — Server-mode tests skipped for flakiness (would violate I2)

[receipt-settings.spec.ts](../../tests/integration/specs/tier2/receipt-settings.spec.ts)
line 154, "should show a placeholder instead of the saved API key", calls `this.skip()`
when `WDIO_SERVER_MODE=1`. The stated reason is **latency, not capability**:

> the settings page runs ~10 sequential RPC calls before loading receipt settings, which
> is flaky over HTTP

The use-case survives the migration, and it is currently covered *only* by the Tauri
harness. Deleting desktop drops it. This needs a real fix — replace the `browser.pause()`
chain with a `waitUntil` on the settled field — not an unskip.

#### R2.d — Pre-existing unconditional skip

[ev-vehicle.spec.ts](../../tests/integration/specs/existing/ev-vehicle.spec.ts) line 217
has `it.skip('should show BEV badge in vehicle list')` with a TODO about the badge not
reliably appearing after creation. Mode-independent, so the migration neither causes nor
fixes it — but afterwards there is only one harness to repair it in, which makes this the
cheapest moment to close it.

#### R2.e — Dead test scripts

Two `test:*` scripts exist that no job invokes and no test file backs. Per
[I1](#coverage-invariants) both must go, not be wired up:

- **Playwright** — [D1](#resolved-decisions): delete
  [tests/e2e/](../../tests/e2e/), [playwright.config.ts](../../playwright.config.ts),
  the `test:e2e` / `test:e2e:ui` scripts, and `@playwright/test`. Also remove the
  stale `playwright-report/` and `test-results/` output directories.
- **vitest** — [vitest.config.ts](../../vitest.config.ts) includes
  `src/**/*.{test,spec}.{js,ts}` and **no such file exists**, so `test:run` is a genuine
  no-op behind `--passWithNoTests`. Per
  [ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication) the
  frontend holds no logic to unit-test, so there is nothing to fill it with.
  Recommendation: delete `test`, `test:run`, [vitest.config.ts](../../vitest.config.ts),
  and the `vitest` dependency. **Flag on review** — this is the one deletion in the task
  that removes a capability rather than dead weight, and it is a one-line reversal if a
  frontend unit layer is ever wanted.

Note `test:all` chains `test:backend && test:run && test:integration` — it needs
rewriting whichever way the vitest call goes.

### R3 — Repoint the local test loop at the web binary

`getBinaryPath()` in [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts)
spawns `kniha-jazd-desktop.exe`. Point it at
[kniha-jazd-web](../../src-tauri/web/src/main.rs) driven by `PORT` / `STATIC_DIR` /
`KNIHA_JAZD_DATA_DIR`. The local loop becomes
`npm run build && cargo build -p kniha-jazd-web` — no Tauri debug build, no
tauri-driver. Then delete [wdio.conf.ts](../../tests/integration/wdio.conf.ts).

### R4 — Rewrite both pipelines

[test.yml](../../.github/workflows/test.yml) — remove `integration-build`,
`integration-tests` (3-tier Windows matrix), `integration-test-server`. Keep
`backend-tests`, `integration-build-docker`, `integration-test-docker`. This deletes the
EdgeDriver version-chasing block (lines 151-190) and the `windows-2022` pin, whose own
comment records jobs hanging "for hours via retries".

**Add** `integration-test-docker-env` — the R2.b job, a second container started with the
`ENV_PINNED_FIXTURE` variables, running the `env/` suite. Without it I1 stays violated.

Resulting job set, which must account for **every** test script in
[package.json](../../package.json):

| Job | Runs | Covers |
|---|---|---|
| `backend-tests` | `cargo test --workspace` | 509 core tests |
| `integration-build-docker` | image build | — |
| `integration-test-docker` (3 tiers) | `test:integration:docker` | 141 e2e |
| `integration-test-docker-env` | env-pinned run | 8 e2e |

That is the complete set. `test:e2e` and `test:run` do not appear because
[R2.e](#r2e--dead-test-scripts) deletes both scripts — a test script that exists but no
job invokes is exactly the I1 violation R2.b documents.

[release.yml](../../.github/workflows/release.yml) — per [D2](#resolved-decisions),
remove the 3-platform `build` matrix and `tauri-action`, the release-notes extraction step
(it reads the version from `src-tauri/desktop/tauri.conf.json`), the duplicated
`integration-build` / `integration-tests` jobs, and the `TAURI_SIGNING_*` env wiring.
`docker-image` becomes the only publish step, and a `v*` tag produces **no GitHub Release
at all** — just the ghcr image.

### R5 — Delete the desktop surface

- [src-tauri/desktop/](../../src-tauri/desktop/) in full (~2,000 lines: 1,517 in
  [commands/](../../src-tauri/desktop/src/commands/), plus
  [lib.rs](../../src-tauri/desktop/src/lib.rs),
  [static_dir.rs](../../src-tauri/desktop/src/static_dir.rs), icons, capabilities, both
  `tauri.conf*.json`, `gen/schemas`)
- Workspace members in [Cargo.toml](../../src-tauri/Cargo.toml) drop to
  `["core", "web"]`; the `sed` hack in [Dockerfile.web](../../Dockerfile.web) that strips
  `desktop` becomes unnecessary
- `.tauri-keys/`, [scripts/stage-spa.mjs](../../scripts/stage-spa.mjs), the `tauri:*`
  scripts in [package.json](../../package.json)
- Frontend (~800 lines): [stores/update.ts](../../src/lib/stores/update.ts),
  [components/UpdateModal.svelte](../../src/lib/components/UpdateModal.svelte),
  [lib/open-external.ts](../../src/lib/open-external.ts), and every `IS_TAURI` branch in
  [api-adapter.ts](../../src/lib/api-adapter.ts),
  [+layout.svelte](../../src/routes/+layout.svelte),
  [settings/+page.svelte](../../src/routes/settings/+page.svelte),
  [doklady/+page.svelte](../../src/routes/doklady/+page.svelte)
- 7 `@tauri-apps/*` npm dependencies
- [stores/capabilities.ts](../../src/lib/stores/capabilities.ts) collapses — every
  feature flag becomes a constant
- Dead lock-file / `move_database` logic in
  [db_location.rs](../../src-tauri/core/src/db_location.rs)

### R6 — Documentation

New ADR superseding [ADR-001](../../DECISIONS.md#adr-001-desktop-app-with-tauri--sveltekit)
and extending ADR-024, recording D1-D3 and the two consequences worth stating outright:
GitHub Releases stop entirely, and folder-scanned receipts survive as an unmaintained path
rather than the intake channel.

A [CHANGELOG.md](../../CHANGELOG.md) entry is the **only** user-facing announcement
([D2](#resolved-decisions)) — it must say plainly that the desktop app is discontinued,
that no further installers or auto-updates will be published, and that the browser UI at
the homelab URL replaces it.

Then: [CLAUDE.md](../../CLAUDE.md) (18 Tauri mentions),
[ARCHITECTURE.md](../../ARCHITECTURE.md), [README.md](../../README.md) +
[README.en.md](../../README.en.md),
[rules/integration-tests.md](../../.claude/rules/integration-tests.md),
[rules/rust-backend.md](../../.claude/rules/rust-backend.md),
[rules/svelte-frontend.md](../../.claude/rules/svelte-frontend.md),
[CHANGELOG.md](../../CHANGELOG.md),
[docs/features/server-mode.md](../../docs/features/server-mode.md),
[docs/features/move-database.md](../../docs/features/move-database.md) (feature
disappears).

## Acceptance criteria

- [ ] `cargo test --workspace` green with members `["core", "web"]`
- [ ] Full Docker integration sweep green across all 3 tiers, with the R2 specs running
- [ ] **I1:** every `test:*` script in [package.json](../../package.json) is invoked by a
      job in [test.yml](../../.github/workflows/test.yml), or the script no longer exists.
      Verify by diffing the script list against `grep "run: npm run"` on the workflow.
- [ ] **I2:** `grep -rnE "describeNotIn[A-Za-z]+\(|it\.skip\(|this\.skip\(\)" tests/integration/specs/`
      returns nothing. All 9 current skip constructs are either fixed (R2.a, R2.c, R2.d),
      removed with their feature (`move_database`, desktop reveal), or made moot because
      only one mode remains (`describeNotInTauriMode` in
      [route-map.spec.ts](../../tests/integration/specs/tier2/route-map.spec.ts)).
      [utils/skip.ts](../../tests/integration/utils/skip.ts) is deleted.
- [ ] The 8 `env/` tests run in CI for the first time (R2.b)
- [ ] No `@tauri-apps` package in [package.json](../../package.json); `npm run check` clean
- [ ] `grep -ri tauri` returns only historical references
      ([DECISIONS.md](../../DECISIONS.md), [CHANGELOG.md](../../CHANGELOG.md),
      [_tasks/_done/](../_done/))
- [ ] Export from the browser honours hidden columns and sort direction
- [ ] [test.yml](../../.github/workflows/test.yml) and
      [release.yml](../../.github/workflows/release.yml) contain no Windows or macOS runner
- [ ] A `v*` tag publishes only the ghcr image and creates **no GitHub Release**
- [ ] [CHANGELOG.md](../../CHANGELOG.md) announces the desktop app as discontinued
- [ ] `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_KEY_PASSWORD` deleted from the
      repository secrets (manual, outside the diff)

## Sequencing

Steps 1-4 are reversible and keep the desktop app working throughout. Step 6 is the
point of no return.

1. R1.1 export parity (backend test first, per
   [ADR-003](../../DECISIONS.md#adr-003-test-driven-development))
2. R1.2 `get_app_version` RPC
3. R2.a un-skips + test-data mount + path helper
4. R2.b env suite as its own Docker CI job — closes the standing I1 violation, and does
   so *before* anything is deleted, so the suite is proven green on the harness that will
   survive
5. R2.c de-flake the receipt-settings placeholder test; R2.d the BEV badge; R2.e decide
   vitest. All three must be green in Docker mode **before** step 7 removes the Tauri
   harness that currently covers R2.c.
6. R3 repoint [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts), delete
   [wdio.conf.ts](../../tests/integration/wdio.conf.ts)
7. R4 rewrite workflows; verify a full green Docker sweep
8. R5 delete desktop crate + frontend Tauri surface
9. R6 documentation and the new ADR

The ordering is load-bearing: steps 3-5 must all be green under Docker while the Tauri
harness still exists, so any coverage the migration would silently drop shows up as a red
job rather than as a deleted file.

## Resolved decisions

Settled 2026-09-03. These are requirements now, not questions.

**D1 — Playwright: drop it.** Delete [tests/e2e/](../../tests/e2e/) (3 specs), the
`test:e2e` / `test:e2e:ui` scripts, [playwright.config.ts](../../playwright.config.ts),
and the `@playwright/test` dependency. It has been commented out of CI since it was
written ([test.yml](../../.github/workflows/test.yml) lines 394-407) and WDIO covers the
same flows against a real backend. Removing it satisfies I1 by subtraction rather than by
adding a second browser harness.

**D2 — Desktop is dropped outright.** No final release, no updater, **no GitHub release
artifacts at all**. A `v*` tag publishes the ghcr image and nothing else. Concretely:

- The `build` job and `tauri-action` are removed from
  [release.yml](../../.github/workflows/release.yml); so is the release-notes extraction
  step that reads `src-tauri/desktop/tauri.conf.json`. `docker-image` is the whole
  workflow.
- `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_KEY_PASSWORD` repo secrets become dead and
  should be deleted from GitHub settings (manual step — note it in the task close-out).
- `.tauri-keys/` is deleted.
- Already-installed desktop copies are **not** migrated or redirected. They keep polling
  an updater endpoint that will never serve another release, which is harmless. They are
  simply obsolete.
- The obsolescence is announced **in [CHANGELOG.md](../../CHANGELOG.md) only** — no
  in-app notice, no migration prompt, no final desktop build carrying a farewell message.

**D3 — Paperless-only from now on.** R1.3 is an accepted loss. The
`receipt-processing-progress` emit path disappears with
[receipts_cmd.rs](../../src-tauri/desktop/src/commands/receipts_cmd.rs); no SSE or
polling replacement is built. `process_pending_receipts` in web mode returns only its
final `SyncResult`, and the listener in
[doklady/+page.svelte](../../src/routes/doklady/+page.svelte) is deleted along with its
`IS_TAURI` guard. This follows
[ADR-024](../../DECISIONS.md#adr-024-homelab-server-is-the-canonical-deployment-desktop-becomes-a-browser-client)
point 4, which already made Paperless the sole intake channel.

One consequence to record in the new ADR: with folder-scanned receipts no longer the
intake path, the local-receipt scanning UI stays functional but unmaintained. Do not
delete it in this task — that is a separate decision about a feature, not about a
deployment target.

## Related

- [02-research.md](./02-research.md) — coupling inventory with file references
- [ADR-024](../../DECISIONS.md#adr-024-homelab-server-is-the-canonical-deployment-desktop-becomes-a-browser-client) — homelab as canonical deployment
- [ADR-018](../../DECISIONS.md#adr-018-workspace-members-over-feature-flags) — the workspace split this builds on
- [Task 58](../_done/58-tauri-workspace-split/), [Task 55](../_done/55-server-mode/),
  [Task 67](../_done/67-online-always-on-runner/)
- [Task 41](../41-integration-test-speedup/) — overlaps R2/R3; the IPC-reset work lands
  in the same harness
- [_TECH_DEBT/07](../_TECH_DEBT/07-integration-db-reset-broken.md) —
  [wdio.conf.ts](../../tests/integration/wdio.conf.ts) cleanup bug, moot once that file
  is deleted
