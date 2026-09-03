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
- Reviving Playwright as a second browser harness (see [Open decisions](#open-decisions)).
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
| R1.1 | Web export ignores `hidden_columns` and `sort_direction` | [export_cmd.rs](../../src-tauri/core/src/commands_internal/export_cmd.rs) lines 50, 61-62 hardcode `Vec::new()` and a fixed `SORT_DIRECTION`; desktop [export_cmd.rs](../../src-tauri/desktop/src/commands/export_cmd.rs) passes both through |
| R1.2 | App version is invisible in web mode | [settings/+page.svelte](../../src/routes/settings/+page.svelte) line 695 gates `getVersion()` on `IS_TAURI`; no `get_app_version` RPC exists |
| R1.3 | Receipt processing progress events have no web equivalent | [receipts_cmd.rs](../../src-tauri/desktop/src/commands/receipts_cmd.rs) line 174 emits via `app.emit`; the RPC path returns only the final `SyncResult` |

R1.1 and R1.2 are blocking. R1.3 is a deliberate-loss candidate — decide and record it.

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

#### R2.e — Frontend unit layer

[vitest.config.ts](../../vitest.config.ts) includes `src/**/*.{test,spec}.{js,ts}` and
**no such file exists** — `test:run` runs with `--passWithNoTests` and is a genuine
no-op. Not a coverage hole to fill (per
[ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication) the frontend
holds no logic to unit-test); noted so nobody mistakes its absence from CI for an
oversight. Either wire it in as a cheap guard or delete the script — do not leave it
ambiguous.

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
| (per R2.e) | `test:run` or the script is deleted | vitest |
| (per D1) | Playwright in CI, or `tests/e2e/` is deleted | — |

The last two rows are decisions, not optional work: a test script that exists but no job
invokes is exactly the I1 violation R2.b documents.

[release.yml](../../.github/workflows/release.yml) — remove the 3-platform `build` matrix
and `tauri-action`, its duplicated integration jobs, and the `TAURI_SIGNING_*` secrets.
`docker-image` becomes the only publish step.

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
and extending ADR-024. Then: [CLAUDE.md](../../CLAUDE.md) (18 Tauri mentions),
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
- [ ] A `v*` tag publishes only the ghcr image

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

## Open decisions

**D1 — Playwright.** [tests/e2e/](../../tests/e2e/) (3 specs) has been commented out of
CI since it was written ([test.yml](../../.github/workflows/test.yml) lines 394-407).
Web-first makes it viable again, but WDIO already covers those flows against a real
backend. Recommendation: delete the directory and drop `@playwright/test` rather than
maintain two browser harnesses.

**D2 — Existing desktop installs.** ADR-024 says they "keep working but point at the
server URL". Options: (a) ship one final desktop release hardcoded to the server URL,
(b) stop publishing and use a browser bookmark. Recommendation: (b) — this is a
single-operator deployment.

**D3 — R1.3 receipt progress events.** Accept the loss, or add SSE/polling? Accepting is
consistent with ADR-024's Paperless-only intake.

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
