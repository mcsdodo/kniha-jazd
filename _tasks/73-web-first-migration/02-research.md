**Date:** 2026-09-03
**Subject:** Desktop/web coupling inventory — what actually blocks retiring Tauri
**Status:** Complete

# Research: Desktop-to-Web Coupling Inventory

Findings behind [01-task.md](./01-task.md). Every claim below was verified against the
tree at commit `2018243`. Numbers are counts at that commit, not estimates.

---

## 1. Backend: the split already did the work

[Task 58](../_done/58-tauri-workspace-split/) left three crates
([Cargo.toml](../../src-tauri/Cargo.toml), members `["core", "desktop", "web"]`):

| Crate | Role | `#[test]` count |
|---|---|---|
| [kniha-jazd-core](../../src-tauri/core/) | DB, calculations, HTTP server, dispatcher | **509** |
| [kniha-jazd-desktop](../../src-tauri/desktop/) | Tauri shell + thin command wrappers | **2** ([static_dir.rs](../../src-tauri/desktop/src/static_dir.rs)) |
| [kniha-jazd-web](../../src-tauri/web/) | Headless binary, one file | 0 |

Deleting the desktop crate therefore costs 2 tests. All business-logic coverage lives in
core and is already exercised by the web binary.

### 1.1 Command parity

Comparing the `generate_handler!` list in
[desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs) (lines 221-329) against the
match arms in [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) and
[dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs):

- Desktop registers **82** commands; the web dispatcher handles **79**.
- **Desktop-only (7):** `export_to_browser`, `get_optimal_window_size`,
  `get_server_status`, `start_server`, `stop_server`, `move_database`,
  `reset_database_location`.
- **Web-only (4):** `generate_route`, `get_trip_route`, `save_trip_route`,
  `delete_trip_route` — the route-map commands from
  [Task 70](../_done/70-route-map-integration/), which desktop never got.

Every desktop-only command is either meaningless when the server *is* the app
(`start_server`, `get_server_status`, `get_optimal_window_size`) or obsoleted by
[ADR-024](../../DECISIONS.md#adr-024-homelab-server-is-the-canonical-deployment-desktop-becomes-a-browser-client)'s
single `/data` volume (`move_database`, `reset_database_location`). Only
`export_to_browser` has a web counterpart worth preserving — see
[section 2](#2-real-functional-gaps).

### 1.2 What the desktop crate actually contains

~2,000 lines total: 1,517 across
[commands/](../../src-tauri/desktop/src/commands/) (11 files, nearly all thin
`_internal` delegators per
[ADR-016](../../DECISIONS.md#adr-016-_internal-extraction-pattern-for-command-reuse)),
plus [lib.rs](../../src-tauri/desktop/src/lib.rs) (setup, lock-file handling, server
autostart), [static_dir.rs](../../src-tauri/desktop/src/static_dir.rs), 16 icons,
[capabilities/default.json](../../src-tauri/desktop/capabilities/default.json), two
`tauri.conf*.json`, and generated `gen/schemas`.

[Dockerfile.web](../../Dockerfile.web) already `sed`s `desktop` out of the workspace
members list at build time — that hack disappears with the crate.

---

## 2. Real functional gaps

These need code, not deletion. They are the reason the migration cannot start with
step "delete the desktop crate".

### 2.1 Export drops hidden columns and sort direction

Desktop [export_to_browser](../../src-tauri/desktop/src/commands/export_cmd.rs) takes
`sort_direction` and `hidden_columns` from the caller and threads them into
`ExportData`. The web path calls `export_html_internal` in
[core/src/commands_internal/export_cmd.rs](../../src-tauri/core/src/commands_internal/export_cmd.rs),
which hardcodes:

```rust
let rows = route_maps::assemble_export_rows(&grid_data, SORT_DIRECTION);  // line 50
// ...
hidden_columns: Vec::new(),                                                // line 61
sort_direction: SORT_DIRECTION.to_string(),                                // line 62
```

The frontend branch is in [routes/+page.svelte](../../src/routes/+page.svelte) around
line 180: with `openExternal` it calls `openExportPreview` (full fidelity), otherwise
`exportHtml` (degraded). So **web export is strictly worse than desktop export today**.
Route maps *are* included on both paths.

**Correction (2026-09-04, from the [plan review](./_plan-review.md) C1).** This section
originally said "only the column/sort arguments are lost". That was wrong — the gap is
**three** differences, and the third changes what the printed legal document says. Desktop's
`export_to_browser` also prepends a synthetic `Uuid::nil()` trip with purpose
`"Prvý záznam"` carrying `year_start_odometer`, and injects the matching
`fuel_remaining` / `trip_numbers` / `odometer_start` entries.
`export_html_internal` never does. The rendering machinery is already in core
([export.rs:332](../../src-tauri/core/src/export.rs), `is_first_record`) — only the caller
that builds the row is missing, and the on-screen grid still renders it
([TripGrid.svelte:436](../../src/lib/components/TripGrid.svelte), `FIRST_RECORD_ID`).

The divergence was documented in the repo all along, at
[route_maps_tests.rs:491](../../src-tauri/core/src/commands_internal/route_maps_tests.rs):
"desktop prepends a synthetic 'Prvý záznam' row **and** honours the user's sort direction,
server mode does neither."

Fix: widen `export_html_internal`'s signature, prepend the synthetic first record, add the
two arguments to the `export_html` dispatcher arm, and drop the branch in `+page.svelte`.
See [03-plan.md](./03-plan.md) Task 1.

### 2.2 No app version in web mode

[settings/+page.svelte](../../src/routes/settings/+page.svelte) line 695:

```ts
// Load app version (Tauri only — getVersion() from @tauri-apps/api throws in web/server mode)
if (IS_TAURI) {
    appVersion = await getVersion();
}
```

No `get_app_version` arm exists in either dispatcher. With versioned ghcr tags being the
deployment mechanism, "which image is this" becomes the question you most want the
settings page to answer.

Fix: one dispatcher arm returning `env!("CARGO_PKG_VERSION")`.

### 2.3 Receipt progress events are Tauri-only

[receipts_cmd.rs](../../src-tauri/desktop/src/commands/receipts_cmd.rs) line 174 emits
`receipt-processing-progress` per iteration; the doc comment says this wrapper keeps its
own body precisely because the framework-free internal version cannot emit. The listener
is in [doklady/+page.svelte](../../src/routes/doklady/+page.svelte) line 56, already
gated on `IS_TAURI`. In web mode the user sees nothing until `sync_receipts` returns.

ADR-024 point 4 makes Paperless the sole intake channel going forward, so this is an
accepted loss — D3 in [01-task.md](./01-task.md) settles it: no SSE or polling replacement
is built.

### 2.4 Gaps that are already handled

- **File dialogs** — receipts folder picker is gated on
  `capabilities.features.fileDialogs`; web mode shows a text input. Fine.
- **Secret reveal** — the PIN flow from [Task 69](../_done/69-pin-gated-secret-reveal/)
  is the server-mode path and already works.
- **Backup restore** — works in server mode; see [section 3.4](#34-one-stale-skip).

---

## 3. Test harness

### 3.1 Current state

141 integration `it`s across
[tests/integration/specs/](../../tests/integration/specs/) (tier1: 36, tier2: 88,
tier3: 8, existing: 9; the 8 in `env/` run separately and are not in that 141). Two wdio
configs:

| Config | Driver | Spawns | Used by |
|---|---|---|---|
| [wdio.conf.ts](../../tests/integration/wdio.conf.ts) | tauri-driver + msedgedriver | Tauri debug binary | `test:integration*` |
| [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) | plain Chrome | Tauri binary with `KNIHA_JAZD_SERVER_AUTOSTART=1`, **or** nothing when `WDIO_EXTERNAL_SERVER=1` | `test:integration:server*`, `test:integration:docker*` |

Docker mode already runs **all three tiers** in CI
([test.yml](../../.github/workflows/test.yml) line 303) against real Chrome and the
container on `--network=host`. This is not a pipeline to build; it is one to promote.

Note that the server config still spawns the *Tauri* binary in non-Docker mode —
`getBinaryPath()` returns `kniha-jazd-desktop.exe`. Swapping it for
[kniha-jazd-web](../../src-tauri/web/src/main.rs) removes the Tauri debug build from the
local loop entirely.

### 3.2 What CI actually invokes

`grep -n "run: npm run\|run: cargo\|run: npx" .github/workflows/test.yml` returns exactly
four test commands: `cargo test --workspace`, `test:integration`,
`test:integration:server`, `test:integration:docker`.

Cross-referencing against the `test:*` scripts in [package.json](../../package.json):

| Script | Invoked by CI? |
|---|---|
| `test:backend` / `cargo test --workspace` | ✅ |
| `test:integration` (Tauri) | ✅ 3 tiers |
| `test:integration:server` | ✅ tier 1 only |
| `test:integration:docker` | ✅ 3 tiers |
| `test:integration:server:env` | ❌ **never** — no workflow mentions `WDIO_ENV_PINNED` |
| `test:run` (vitest) | ❌ — but `src/**/*.{test,spec}.{js,ts}` matches zero files, so it is a no-op |
| `test:e2e` (Playwright) | ❌ — commented out at [test.yml](../../.github/workflows/test.yml) lines 394-407 |

So the 8 tests in
[env-managed-settings.spec.ts](../../tests/integration/specs/env/env-managed-settings.spec.ts)
have never been enforced by a pipeline. They cover the env-pinned settings behaviour from
[Task 68](../_done/68-env-managed-settings-ui/) and the PIN reveal from
[Task 69](../_done/69-pin-gated-secret-reveal/) — both shipped features. This predates the
migration; the migration is just the moment it becomes cheap to fix, because a second
container with different env vars is trivial next to a second spawned-Tauri wdio run.

### 3.3 Skips that are not about mode

Two skips in the suite have nothing to do with desktop-vs-web, and one of them is load-bearing
for this migration:

**[receipt-settings.spec.ts](../../tests/integration/specs/tier2/receipt-settings.spec.ts)
line 154** — "should show a placeholder instead of the saved API key" calls `this.skip()`
under `WDIO_SERVER_MODE=1`, with the reason given as latency:

> the settings page runs ~10 sequential RPC calls before loading receipt settings, which
> is flaky over HTTP

The feature survives the migration, so today the **Tauri harness is its only coverage**.
Deleting desktop drops the use-case silently. The spec body is a chain of
`browser.pause(100/300/500)` calls — replacing them with a `waitUntil` on the settled
field is the actual fix.

**[ev-vehicle.spec.ts](../../tests/integration/specs/existing/ev-vehicle.spec.ts) line
217** — `it.skip('should show BEV badge in vehicle list')`, TODO'd as flaky since the
vehicle list does not reliably update after creation via the UI helper. Mode-independent,
pre-existing, and worth closing while there is still a second harness to compare against.

### 3.4 One stale skip

[backup-restore.spec.ts](../../tests/integration/specs/tier2/backup-restore.spec.ts)
line 159 says:

> `restore_backup` replaces the running database file — excluded from server RPC by
> design (see ADR-017 / capabilities.restore_backup=false). Skip in server mode.

That is no longer true on any of its three claims:

- [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) line 766 dispatches
  `restore_backup`
- line 1068 of the same file round-trip tests it (restore, then assert the restored
  vehicle comes back)
- `capabilities_handler` in [server/mod.rs](../../src-tauri/core/src/server/mod.rs)
  returns `"restore_backup": true`

ADR-024 itself cites "restore-backup parity closed the last functional gap for browser
users". The skip should just go.

### 3.5 The rest of the Docker skips

| Spec | Suite | Reason | Fixable how |
|---|---|---|---|
| [export.spec.ts](../../tests/integration/specs/tier1/export.spec.ts) | whole file (2) | `before()` calls `this.skip()` on `WDIO_SERVER_MODE` | Falls out of gap 2.1 — once web export has parity, unskip |
| [receipts.spec.ts](../../tests/integration/specs/tier2/receipts.spec.ts) | "Mismatch Detection E2E", "Multi-Currency Receipts" | `describeNotInDockerMode` — container cannot see host [data/](../../tests/integration/data/) | Mount + path mapping |
| [multi-invoice.spec.ts](../../tests/integration/specs/tier2/multi-invoice.spec.ts) | whole spec (1) | same | same |
| [env-managed-settings.spec.ts](../../tests/integration/specs/env/env-managed-settings.spec.ts) | whole suite (8) | config throws on `WDIO_ENV_PINNED=1 + WDIO_EXTERNAL_SERVER=1` — vars must exist at process start | Second container start with the fixture env |
| [receipt-settings.spec.ts](../../tests/integration/specs/tier2/receipt-settings.spec.ts) | "Database Move Commands" | `describeNotInServerMode` — feature disappears | Delete with `move_database` |

The mount problem is mechanical: specs build absolute host paths
(`join(__dirname, '..', '..', 'data', 'invoices')` at
[receipts.spec.ts](../../tests/integration/specs/tier2/receipts.spec.ts) lines 148, 227,
296, 384, 432, 489) and hand them to the backend over RPC. Mount the directory into the
container and route those joins through a `containerPath()` helper that rewrites the
prefix when `WDIO_EXTERNAL_SERVER=1`. Same for `KNIHA_JAZD_MOCK_GEMINI_DIR`, set in
`onPrepare` to a host path.

The Paperless mock server needs no work — `--network=host` already lets the container
reach it, which is why
[paperless-integration.spec.ts](../../tests/integration/specs/tier2/paperless-integration.spec.ts)
is not skipped.

### 3.6 Playwright

[tests/e2e/](../../tests/e2e/) holds 3 Playwright specs and has been commented out of CI
since it was written ([test.yml](../../.github/workflows/test.yml) lines 394-407, "These
tests need a running Vite dev server which requires complex setup"). It runs frontend-only
against a dev server; WDIO covers the same flows against a real backend. D1 in
[01-task.md](./01-task.md) settles it: the directory, the config, both scripts, and the
`@playwright/test` dependency are deleted.

---

## 4. Pipelines

### 4.1 [test.yml](../../.github/workflows/test.yml)

| Job | Runner | Fate |
|---|---|---|
| `check-changes` | ubuntu | keep |
| `backend-tests` (3-platform matrix) | win/mac/ubuntu | keep, but the matrix can shrink to ubuntu once desktop is gone |
| `integration-build` | windows-latest | **delete** — Tauri debug build |
| `integration-tests` (3 tiers) | windows-2022 | **delete** |
| `integration-test-server` (tier 1) | windows-latest | **delete** — superseded by Docker |
| `integration-build-docker` | ubuntu | keep |
| `integration-test-docker` (3 tiers) | ubuntu | keep — becomes primary |

The `integration-tests` job carries the worst maintenance burden in the repo: a
40-line PowerShell block (lines 151-190) that reads the WebView2 version out of the
registry, fetches a matching msedgedriver, and pins `MSEDGEDRIVER_PATH` — with a comment
explaining that getting this wrong "hangs the job for hours via retries". The runner is
pinned to `windows-2022` because the 2026-07-14 `windows-latest` image shipped WebView2
150, under which msedgedriver could not launch the app at all. All of that disappears.

### 4.2 [release.yml](../../.github/workflows/release.yml)

`build` is a 3-platform `tauri-action` matrix (windows, macos-aarch64, macos-x64)
consuming `TAURI_SIGNING_PRIVATE_KEY` + password, publishing installers to a GitHub
Release. It also duplicates `backend-tests` / `integration-build` / `integration-tests`
from [test.yml](../../.github/workflows/test.yml). All of it goes; `docker-image`
(ghcr push, ADR-024 point 5) becomes the only artifact job.

Note [release.yml](../../.github/workflows/release.yml) reads the version from
`src-tauri/desktop/tauri.conf.json` when extracting changelog notes — that needs
repointing at [package.json](../../package.json) or the workspace
[Cargo.toml](../../src-tauri/Cargo.toml).

---

## 5. Frontend

`@tauri-apps` imports are confined to 8 files
(`grep -rn "@tauri-apps" src/`); [capabilities.ts](../../src/lib/stores/capabilities.ts)
is a ninth touchpoint via `IS_TAURI`:

| File | Uses | Disposition |
|---|---|---|
| [api-adapter.ts](../../src/lib/api-adapter.ts) | `invoke`, defines `IS_TAURI` | Collapses to the `fetch('/api/rpc')` branch |
| [stores/update.ts](../../src/lib/stores/update.ts) (359 lines) | updater, process | Delete |
| [components/UpdateModal.svelte](../../src/lib/components/UpdateModal.svelte) (418 lines) | opener | Delete |
| [lib/open-external.ts](../../src/lib/open-external.ts) (18 lines) | opener | Delete |
| [routes/+layout.svelte](../../src/routes/+layout.svelte) | window sizing (line 141), update banner/modal | Strip both |
| [routes/settings/+page.svelte](../../src/routes/settings/+page.svelte) | dialog, opener, `appDataDir`, `getVersion` | Strip 5 branches |
| [routes/doklady/+page.svelte](../../src/routes/doklady/+page.svelte) | `listen`, `appDataDir`, opener | Strip `onMount` branch (line 56) |
| [lib/api.ts](../../src/lib/api.ts) | `revealItemInDir` (line 227) | Strip branch |
| [stores/capabilities.ts](../../src/lib/stores/capabilities.ts) | `IS_TAURI` | Every flag becomes a constant |

~800 lines removed, plus 7 npm dependencies (`@tauri-apps/api`, `-cli`, and the
dialog/fs/opener/process/updater plugins).

Only 13 `$capabilities` call sites exist in markup, so the flag collapse is small.

---

## 6. Summary

| Dimension | Before | After |
|---|---|---|
| Cargo workspace members | 3 | 2 |
| Rust lines in the desktop path | ~2,000 | 0 |
| Backend tests lost | — | 2 of 511 |
| Frontend lines removed | — | ~800 |
| npm deps removed | — | 7 |
| CI jobs in [test.yml](../../.github/workflows/test.yml) | 7 | 4 |
| CI runners | windows-2022, windows-latest, macos, ubuntu | ubuntu |
| Release artifacts | 3 installers + ghcr image | ghcr image |
| Blocking feature gaps | — | 2 ([2.1](#21-export-drops-hidden-columns-and-sort-direction), [2.2](#22-no-app-version-in-web-mode)) |
| Test scripts CI never invokes | 2 (`:server:env`, `test:e2e`) | 0 |
| e2e tests enforced by a pipeline | 141 | 149+ (the `env/` 8 join) |
| Skip constructs in [specs/](../../tests/integration/specs/) | 9 | 0 |

The conclusion that drove the sequencing in [01-task.md](./01-task.md): the only work
that *must* precede deletion is closing the two functional gaps and proving the Docker
suite covers what the Tauri suite covered.

That second half is larger than it first looks. Unskipping the mode-conditional
`describeNotIn*` blocks is the easy part; the migration also has to absorb
[3.2](#32-what-ci-actually-invokes) (a suite no pipeline has ever run) and
[3.3](#33-skips-that-are-not-about-mode) (a use-case whose only coverage is the harness
being deleted). Everything after that is subtraction.
