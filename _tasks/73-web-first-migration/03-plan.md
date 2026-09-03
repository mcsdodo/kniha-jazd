**Date:** 2026-09-03
**Subject:** Web-first migration — implementation plan
**Status:** Planning

# Web-First Migration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this
> plan task-by-task.

**Goal:** Delete the Tauri desktop app and make the Docker container + browser UI the only
thing this project builds, ships, and tests — without losing a single use-case or a single
test that GitHub Actions enforces.

**Architecture:** The [Task 58](../_done/58-tauri-workspace-split/) workspace split already
put every piece of business logic in [kniha-jazd-core](../../src-tauri/core/), reachable
through the RPC dispatcher that the browser UI uses. So this is mostly subtraction. The two
exceptions come first: web-mode export currently drops arguments the desktop export passes,
and the browser has no way to ask what version it is running. Both must close *before*
anything is deleted, and every test fix must go green under Docker *while the Tauri harness
still exists*, so dropped coverage shows up as a red job rather than a deleted file.

**Tech Stack:** Rust (axum, diesel, tokio), SvelteKit 5, WebdriverIO + Chrome, Docker,
GitHub Actions.

**Requirements this implements:** [01-task.md](./01-task.md) R1-R6, decisions D1-D3.
**Evidence for every claim:** [02-research.md](./02-research.md).

---

## Before you start

Read [01-task.md](./01-task.md) — in particular the two **coverage invariants**, which are
the acceptance bar for the whole plan:

- **I1** — every test script in [package.json](../../package.json) is invoked by a job in
  [test.yml](../../.github/workflows/test.yml), or the script does not exist.
- **I2** — every use-case that survives has an end-to-end test. Deleting a *feature* and
  its tests is fine; deleting a *test* for a surviving feature is not.

Per [ADR-003](../../DECISIONS.md#adr-003-test-driven-development) every code change starts
with a failing test. Per [ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication)
no calculation moves to the frontend.

**Commands you will use constantly:**

```bash
# One backend test by name (fast — prefer this while iterating)
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "test_name_filter"

# Whole backend suite
cargo test --manifest-path src-tauri/Cargo.toml --workspace

# One integration spec against a running Docker container
WDIO_EXTERNAL_SERVER=1 WDIO_SERVER_MODE=1 npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/receipts.spec.ts
```

**Do not run the full integration sweep while iterating** — it takes ~10 minutes. Run the
single spec you are fixing. Full sweeps happen at the phase gates, which are marked.

**Branch:** ask the user whether to branch before Task 1. Everything through Phase 7 is
reversible; Phase 8 is not.

---

## Phase 1 — Export parity (R1.1)

The blocking gap. [export_html_internal](../../src-tauri/core/src/commands_internal/export_cmd.rs)
hardcodes `hidden_columns: Vec::new()` and a fixed `SORT_DIRECTION`, so exporting from the
browser silently ignores the user's column visibility and sort choice. Desktop's
`export_to_browser` passes both. Close this before desktop goes away, or the migration is a
regression.

### Task 1: Thread `hidden_columns` and `sort_direction` through `export_html_internal`

**Files:**
- Modify: [src-tauri/core/src/commands_internal/export_cmd.rs](../../src-tauri/core/src/commands_internal/export_cmd.rs)
- Modify: [src-tauri/core/src/server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs) lines 197-218
- Modify: [src-tauri/desktop/src/commands/export_cmd.rs](../../src-tauri/desktop/src/commands/export_cmd.rs) (the `export_html` wrapper — keep it compiling; it dies in Phase 8)

**Step 1: Write the failing test**

Add to the `mod tests` block at the bottom of
[dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs). It seeds the
minimum an export needs (vehicle + settings + one trip) and asserts both arguments survive
the round trip.

```rust
    /// Web-mode export must honour the user's column visibility and sort choice.
    /// Before Task 73 these were hardcoded, so the browser export silently
    /// disagreed with the grid the user was looking at.
    #[tokio::test]
    async fn export_html_honours_hidden_columns_and_sort_direction() {
        use crate::models::{Settings, Trip};
        use chrono::{NaiveDate, Utc};

        let dir = tempfile::tempdir().unwrap();
        let db = std::sync::Arc::new(crate::db::Database::in_memory().unwrap());

        let vehicle =
            crate::models::Vehicle::new_ice("Test".into(), "BA-123AB".into(), 50.0, 6.5, 10_000.0);
        db.create_vehicle(&vehicle).unwrap();
        db.save_settings(&Settings::default()).unwrap();

        // Two trips on different days, so sort direction is observable.
        let mut make_trip = |day: u32, destination: &str, odo: f64| Trip {
            id: uuid::Uuid::new_v4(),
            vehicle_id: vehicle.id,
            start_datetime: NaiveDate::from_ymd_opt(2026, 3, day)
                .unwrap()
                .and_hms_opt(8, 0, 0)
                .unwrap(),
            end_datetime: None,
            origin: "Bratislava".into(),
            destination: destination.into(),
            distance_km: 60.0,
            odometer: odo,
            purpose: "test".into(),
            fuel_liters: None,
            fuel_cost_eur: None,
            full_tank: false,
            energy_kwh: None,
            energy_cost_eur: None,
            full_charge: false,
            soc_override_percent: None,
            other_costs_eur: None,
            other_costs_note: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        db.create_trip(&make_trip(1, "TRNAVA", 10_060.0)).unwrap();
        db.create_trip(&make_trip(2, "KOSICE", 10_120.0)).unwrap();

        let state = ServerState {
            db,
            app_state: std::sync::Arc::new(crate::app_state::AppState::new()),
            app_dir: dir.path().to_path_buf(),
            static_dir: std::env::temp_dir(),
        };

        // "time" is one of the five hideable columns (grep `is_visible("` in
        // export.rs for the full set: time, fuelConsumed, fuelRemaining,
        // otherCosts, otherCostsNote). Give its header a distinctive marker so
        // presence/absence in the HTML is unambiguous.
        let mut labels = serde_json::to_value(sample_export_labels()).unwrap();
        labels["col_time"] = serde_json::json!("CAS-MARKER");

        let visible = dispatch_async(
            "export_html",
            serde_json::json!({
                "vehicleId": vehicle.id.to_string(),
                "year": 2026,
                "labels": labels,
                "hiddenColumns": [],
                "sortDirection": "desc"
            }),
            &state,
        )
        .await
        .unwrap()
        .unwrap();
        let visible = visible.as_str().unwrap();
        assert!(
            visible.contains("CAS-MARKER"),
            "time column should render when not hidden"
        );

        let hidden = dispatch_async(
            "export_html",
            serde_json::json!({
                "vehicleId": vehicle.id.to_string(),
                "year": 2026,
                "labels": labels,
                "hiddenColumns": ["time"],
                "sortDirection": "desc"
            }),
            &state,
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            !hidden.as_str().unwrap().contains("CAS-MARKER"),
            "hiddenColumns was ignored — the time column still rendered"
        );

        // sortDirection: "desc" puts the newest trip first. The `visible` render
        // above already used "desc", so compare against an "asc" render.
        let ascending = dispatch_async(
            "export_html",
            serde_json::json!({
                "vehicleId": vehicle.id.to_string(),
                "year": 2026,
                "labels": labels,
                "hiddenColumns": [],
                "sortDirection": "asc"
            }),
            &state,
        )
        .await
        .unwrap()
        .unwrap();
        let ascending = ascending.as_str().unwrap();

        let desc_kosice = visible.find("KOSICE").expect("KOSICE missing from desc render");
        let desc_trnava = visible.find("TRNAVA").expect("TRNAVA missing from desc render");
        assert!(
            desc_kosice < desc_trnava,
            "sortDirection=desc should put the newer trip (KOSICE) first"
        );

        let asc_kosice = ascending.find("KOSICE").expect("KOSICE missing from asc render");
        let asc_trnava = ascending.find("TRNAVA").expect("TRNAVA missing from asc render");
        assert!(
            asc_trnava < asc_kosice,
            "sortDirection=asc should put the older trip (TRNAVA) first — \
             the argument was ignored"
        );
    }
```

You also need a labels fixture in this test module. Reuse the shape from
[export_tests.rs](../../src-tauri/core/src/export_tests.rs) `sample_labels()` — extract it
to a `pub(crate)` helper rather than copy-pasting 40 lines:

- In [export.rs](../../src-tauri/core/src/export.rs), add
  `#[cfg(test)] pub(crate) fn sample_export_labels() -> ExportLabels` containing the body
  currently in `export_tests.rs::sample_labels()`.
- Change `export_tests.rs::sample_labels()` to call it, or delete it and update its callers.

**Step 2: Run the test to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core export_html_honours_hidden_columns
```

Expected: FAIL to compile — `unknown field 'hiddenColumns'` from `parse_args`, because the
dispatcher's `Args` struct does not have those fields yet.

**Step 3: Widen the internal function**

In [export_cmd.rs](../../src-tauri/core/src/commands_internal/export_cmd.rs), delete the
`SORT_DIRECTION` constant and take both values as parameters:

```rust
pub async fn export_html_internal(
    db: &Database,
    app_dir: &Path,
    vehicle_id: String,
    year: i32,
    labels: ExportLabels,
    hidden_columns: Vec<String>,
    sort_direction: String,
) -> Result<String, String> {
```

Then replace the two hardcoded uses:

```rust
    let rows = route_maps::assemble_export_rows(&grid_data, &sort_direction);
```

```rust
    let export_data = ExportData {
        vehicle,
        settings,
        grid_data,
        year,
        totals,
        labels,
        hidden_columns,
        sort_direction,
        route_maps: map_pages,
    };
```

Note the doc comment above `SORT_DIRECTION` explains why row assembly and `ExportData` must
use the *same* value — keep that invariant, now via the shared parameter. Move that comment
onto the parameter so the reasoning is not lost.

**Step 4: Widen the dispatcher arm**

In [dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs):

```rust
        "export_html" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                year: i32,
                labels: crate::export::ExportLabels,
                #[serde(default)]
                hidden_columns: Vec<String>,
                #[serde(default = "default_sort_direction")]
                sort_direction: String,
            }
            let a: Args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            let result = crate::commands_internal::export_html_internal(
                &state.db,
                &state.app_dir,
                a.vehicle_id,
                a.year,
                a.labels,
                a.hidden_columns,
                a.sort_direction,
            )
            .await;
            Some(result.map(|v| serde_json::to_value(v).unwrap()))
        }
```

Add near the top of the file:

```rust
/// Older callers omit `sortDirection`; keep their behaviour (oldest first).
fn default_sort_direction() -> String {
    "asc".to_string()
}
```

The `#[serde(default)]` attributes matter: they keep the argument contract backward
compatible, so a stale frontend bundle in a browser cache does not start failing exports.

**Step 5: Fix the desktop wrapper so the workspace still compiles**

In [desktop/src/commands/export_cmd.rs](../../src-tauri/desktop/src/commands/export_cmd.rs),
the `export_html` command delegates to the internal. Give it the two new parameters and pass
them through:

```rust
#[tauri::command]
pub async fn export_html(
    app: tauri::AppHandle,
    db: State<'_, Arc<Database>>,
    vehicle_id: String,
    year: i32,
    labels: ExportLabels,
    hidden_columns: Vec<String>,
    sort_direction: String,
) -> Result<String, String> {
    let app_data_dir = get_app_data_dir(&app)?;
    inner::export_html_internal(
        &db,
        &app_data_dir,
        vehicle_id,
        year,
        labels,
        hidden_columns,
        sort_direction,
    )
    .await
}
```

**Step 6: Run the test to verify it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core export_html_honours_hidden_columns
```

Expected: PASS, 1 test.

**Step 7: Run the whole backend suite**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --workspace
```

Expected: PASS. If `export_tests.rs` fails, it is the `sample_labels` extraction from Step 1
— fix the call sites, not the assertions.

**Step 8: Commit**

```bash
git add src-tauri/core/src/commands_internal/export_cmd.rs \
        src-tauri/core/src/server/dispatcher_async.rs \
        src-tauri/core/src/export.rs \
        src-tauri/core/src/export_tests.rs \
        src-tauri/desktop/src/commands/export_cmd.rs
git commit -m "fix(export): honour hidden columns and sort direction in web mode"
```

### Task 2: Pass the arguments from the browser

**Files:**
- Modify: [src/lib/api.ts](../../src/lib/api.ts) lines 255-261
- Modify: [src/routes/+page.svelte](../../src/routes/+page.svelte) lines ~180-195

**Step 1: Widen the api wrapper**

```ts
// Export - returns HTML string (used in server/browser mode)
export async function exportHtml(
	vehicleId: string,
	year: number,
	labels: ExportLabels,
	hiddenColumns: string[],
	sortDirection: string
): Promise<string> {
	return await apiCall('export_html', { vehicleId, year, labels, hiddenColumns, sortDirection });
}
```

**Step 2: Pass them at the call site**

In [+page.svelte](../../src/routes/+page.svelte), the `else` branch of `handleExport`
currently drops both. `currentHiddenColumns` (line 122) and `exportSortDirection` (line 26)
are already in scope:

```ts
			} else {
				const html = await exportHtml(
					$activeVehicleStore.id,
					$selectedYearStore,
					labels,
					currentHiddenColumns,
					exportSortDirection
				);
				const blob = new Blob([html], { type: 'text/html' });
				const url = URL.createObjectURL(blob);
				window.open(url, '_blank');
			}
```

**Step 3: Typecheck**

```bash
npm run check
```

Expected: no new errors. (If i18n types complain, run `npm run i18n` first — nothing else
regenerates `i18n-types.ts`.)

**Step 4: Commit**

```bash
git add src/lib/api.ts src/routes/+page.svelte
git commit -m "fix(export): send hidden columns and sort direction from the browser"
```

> **Note:** the integration test for this lands in Task 7, once `export.spec.ts` can run in
> Docker mode at all. Do not unskip it here — it will fail for the unrelated reason that the
> spec has never run outside Tauri.

---

## Phase 2 — App version over RPC (R1.2)

### Task 3: Add `get_app_version`

**Files:**
- Modify: [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs)
- Modify: [src/lib/api.ts](../../src/lib/api.ts)
- Modify: [src/routes/settings/+page.svelte](../../src/routes/settings/+page.svelte) line ~694

**Step 1: Write the failing test**

In the `mod tests` block of [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs):

```rust
    /// With versioned ghcr tags as the deployment unit, "which image am I running"
    /// is the question the settings page most needs to answer.
    #[test]
    fn get_app_version_reports_the_crate_version() {
        let state = test_state();
        let v = dispatch_sync("get_app_version", json!({}), &state).unwrap();
        assert_eq!(v.as_str().unwrap(), env!("CARGO_PKG_VERSION"));
        assert!(!v.as_str().unwrap().is_empty());
    }
```

**Step 2: Run it, verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core get_app_version_reports
```

Expected: FAIL — the dispatcher returns an unknown-command error and `.unwrap()` panics.

**Step 3: Add the dispatcher arm**

Next to `"get_app_mode"` (line 475):

```rust
        "get_app_version" => Ok(serde_json::to_value(env!("CARGO_PKG_VERSION")).unwrap()),
```

`CARGO_PKG_VERSION` resolves to the workspace version in
[src-tauri/Cargo.toml](../../src-tauri/Cargo.toml), which
[release-skill](../../.claude/skills/) already bumps in lockstep with
[package.json](../../package.json).

**Step 4: Run it, verify it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core get_app_version_reports
```

Expected: PASS.

**Step 5: Wire the frontend**

In [api.ts](../../src/lib/api.ts):

```ts
export async function getAppVersion(): Promise<string> {
	return apiCall<string>('get_app_version');
}
```

In [settings/+page.svelte](../../src/routes/settings/+page.svelte), replace the `IS_TAURI`
branch at line ~694:

```ts
			appVersion = await getAppVersion();
```

Delete the now-unused `getVersion` import from `@tauri-apps/api/app` and, if `IS_TAURI` has
no other use in this file, its import too. Check with
`grep -n "IS_TAURI" src/routes/settings/+page.svelte` — it is also used at lines 87 and 759,
so it stays for now.

**Step 6: Typecheck and commit**

```bash
npm run check
git add src-tauri/core/src/server/dispatcher.rs src/lib/api.ts src/routes/settings/+page.svelte
git commit -m "feat(settings): report app version over RPC in web mode"
```

---

## Phase 3 — Make the Docker harness cover what Tauri covered (R2.a)

Three suites are skipped in Docker because the container cannot see the host's test data.
This phase mounts it and teaches the specs to address it by container path.

### Task 4: Add a host-to-container path helper

**Files:**
- Create: [tests/integration/utils/paths.ts](../../tests/integration/utils/paths.ts)

**Step 1: Write the helper**

```ts
/**
 * Resolve a path under tests/integration/data/ into whatever the *backend* can see.
 *
 * The specs hand absolute paths to the backend over RPC (receipts folder scanning,
 * Gemini mock JSON). In spawned-server mode the backend is a local process and sees
 * host paths. In Docker mode it is a container that only sees what we mounted, so the
 * same logical location has a different absolute path on the other side of the RPC.
 *
 * Keep the mount target in sync with the `-v` flag in the `Start container` step of
 * .github/workflows/test.yml and with docker-compose.web.yml.
 */
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

/** Where tests/integration/data/ is mounted inside the container. */
export const CONTAINER_DATA_ROOT = '/testdata';

const IS_DOCKER_MODE = process.env.WDIO_EXTERNAL_SERVER === '1';

/** Host path to tests/integration/data/ — for reads the *test process* performs. */
export const HOST_DATA_ROOT = join(__dirname, '..', 'data');

/**
 * Path to a subdirectory of tests/integration/data/ as the BACKEND sees it.
 * Use for any path sent over RPC. Use HOST_DATA_ROOT for fs calls in the spec itself.
 */
export function backendDataPath(...segments: string[]): string {
  return IS_DOCKER_MODE
    ? [CONTAINER_DATA_ROOT, ...segments].join('/')
    : join(HOST_DATA_ROOT, ...segments);
}
```

**Step 2: Commit**

```bash
git add tests/integration/utils/paths.ts
git commit -m "test(integration): add host-to-container path helper"
```

### Task 5: Mount the test data into the CI container

**Files:**
- Modify: [.github/workflows/test.yml](../../.github/workflows/test.yml) — the
  `Start container` step at line 346

**Step 1: Add the mount and the mock-Gemini env var**

```yaml
      - name: Start container
        run: |
          mkdir -p data
          docker run -d --name kniha-jazd-web \
            --network=host \
            -v "$PWD/data:/data" \
            -v "$PWD/tests/integration/data:/testdata:ro" \
            -e KNIHA_JAZD_DATA_DIR=/data \
            -e DATABASE_PATH=/data/kniha-jazd.db \
            -e KNIHA_JAZD_MOCK_GEMINI_DIR=/testdata/mocks \
            -e PORT=3456 \
            kniha-jazd-web:test
```

`KNIHA_JAZD_MOCK_GEMINI_DIR` is `env_vars::MOCK_GEMINI_DIR` in
[constants.rs](../../src-tauri/core/src/constants.rs) line 72. In spawned-server mode
[wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) sets it in `onPrepare`;
in Docker mode the process is started by CI, so it has to be set here.

Mount read-only (`:ro`) — the container has no business writing into the repo. Task 6 covers
the one spec that writes placeholder files, which it does from the *test* process.

**Step 2: Verify locally before touching specs**

```bash
docker build -f Dockerfile.web -t kniha-jazd-web:test .
mkdir -p data
docker run -d --name kj-test --network=host \
  -v "$PWD/data:/data" -v "$PWD/tests/integration/data:/testdata:ro" \
  -e KNIHA_JAZD_DATA_DIR=/data -e DATABASE_PATH=/data/kniha-jazd.db \
  -e KNIHA_JAZD_MOCK_GEMINI_DIR=/testdata/mocks -e PORT=3456 kniha-jazd-web:test
curl -sf http://localhost:3456/health && echo " healthy"
docker exec kj-test ls /testdata/invoices
```

Expected: `healthy`, then `invoice-czk.pdf` and `invoice.pdf`.

Leave the container running — the next tasks test against it. Tear down with
`docker rm -f kj-test` when the phase is done.

**Step 3: Commit**

```bash
git add .github/workflows/test.yml
git commit -m "ci(docker): mount integration test data into the container"
```

### Task 6: Unskip the receipts and multi-invoice suites

**Files:**
- Modify: [tests/integration/specs/tier2/receipts.spec.ts](../../tests/integration/specs/tier2/receipts.spec.ts) lines 95, 350 and the six `invoicesPath` joins
- Modify: [tests/integration/specs/tier2/multi-invoice.spec.ts](../../tests/integration/specs/tier2/multi-invoice.spec.ts) line 104

**Step 1: Swap the path construction**

In [receipts.spec.ts](../../tests/integration/specs/tier2/receipts.spec.ts) there are six
occurrences of:

```ts
      const invoicesPath = join(__dirname, '..', '..', 'data', 'invoices');
```

Replace each with:

```ts
      const invoicesPath = backendDataPath('invoices');
```

and add the import:

```ts
import { backendDataPath } from '../../utils/paths';
```

Do the same for any equivalent joins in
[multi-invoice.spec.ts](../../tests/integration/specs/tier2/multi-invoice.spec.ts). Note
line 121 there does `rmSync(join(dataDir, 'seeded-receipts'), ...)` — that is the *test
process* cleaning up, so it must keep using a host path. Only paths sent over RPC change.

**Step 2: Change the describe wrappers**

Replace `describeNotInDockerMode(` with plain `describe(` in all three places (receipts.spec
lines 95 and 350, multi-invoice.spec line 104) and drop the now-unused imports.

**Step 3: Run the two specs against the container**

```bash
WDIO_EXTERNAL_SERVER=1 WDIO_SERVER_MODE=1 npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/receipts.spec.ts \
  --spec tests/integration/specs/tier2/multi-invoice.spec.ts
```

Expected: PASS, no skipped suites in the reporter output.

If receipt *seeding* fails, check whether the spec writes placeholder files into
`tests/integration/data/` — those writes happen host-side and must not go through
`backendDataPath`.

**Step 4: Commit**

```bash
git add tests/integration/specs/tier2/receipts.spec.ts \
        tests/integration/specs/tier2/multi-invoice.spec.ts
git commit -m "test(integration): run receipts and multi-invoice suites in Docker mode"
```

### Task 7: Unskip export and backup restoration

**Files:**
- Modify: [tests/integration/specs/tier1/export.spec.ts](../../tests/integration/specs/tier1/export.spec.ts) lines 25-29
- Modify: [tests/integration/specs/tier2/backup-restore.spec.ts](../../tests/integration/specs/tier2/backup-restore.spec.ts) lines 159-161

**Step 1: Delete both skips**

In [export.spec.ts](../../tests/integration/specs/tier1/export.spec.ts), remove the whole
`before` hook:

```ts
  before(function () {
    if (process.env.WDIO_SERVER_MODE === '1') {
      this.skip();
    }
  });
```

In [backup-restore.spec.ts](../../tests/integration/specs/tier2/backup-restore.spec.ts),
change `describeNotInServerMode('Backup Restoration', ...)` to `describe(...)` and delete
the stale comment above it — its three claims are all false (see
[02-research.md §3.4](./02-research.md)). Drop the unused import if nothing else in the file
uses it.

**Step 2: Run both specs**

```bash
WDIO_EXTERNAL_SERVER=1 WDIO_SERVER_MODE=1 npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier1/export.spec.ts \
  --spec tests/integration/specs/tier2/backup-restore.spec.ts
```

Expected: PASS. The export spec exercises Phase 1's work end-to-end — it uses
`window.open()`, so if it fails on a popup, add `--disable-popup-blocking` to the Chrome
args in [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) rather than
weakening the assertion.

**Step 3: Commit**

```bash
git add tests/integration/specs/tier1/export.spec.ts \
        tests/integration/specs/tier2/backup-restore.spec.ts
git commit -m "test(integration): run export and backup restore in server mode"
```

> **Phase gate.** Run the full Docker sweep before moving on:
> ```bash
> for t in 1 2 3; do TIER=$t PARALLEL_TIERS=true npm run test:integration:docker || break; done
> ```
> Expected: all three tiers green. This is the first point where Docker mode covers what
> Tauri mode covers, minus the two flaky specs in Phase 5.

---

## Phase 4 — Run the env suite in CI (R2.b)

`test:integration:server:env` is invoked by **no workflow** — the 8 tests covering
env-pinned settings ([Task 68](../_done/68-env-managed-settings-ui/)) and the PIN reveal
([Task 69](../_done/69-pin-gated-secret-reveal/)) have never been enforced by a pipeline.
This is the standing I1 violation.

### Task 8: Add the `integration-test-docker-env` job

**Files:**
- Modify: [tests/integration/wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) — the `ENV_PINNED && EXTERNAL_SERVER` guard in `onPrepare`
- Modify: [package.json](../../package.json)
- Modify: [.github/workflows/test.yml](../../.github/workflows/test.yml)

**Step 1: Remove the guard that forbids the combination**

The guard exists because the fixture variables must be present when the *server process*
starts, and a spawned-Tauri run could not retrofit them. A container started with `-e` flags
can. Delete:

```ts
    if (ENV_PINNED && EXTERNAL_SERVER) {
      throw new Error(
        'WDIO_ENV_PINNED=1 cannot run against an external server — the fixture ' +
          'variables must be present when the process starts. Use the spawned-Tauri run.'
      );
    }
```

Replace it with a comment recording the new contract:

```ts
    // ENV_PINNED + EXTERNAL_SERVER is valid: CI starts a second container with
    // ENV_PINNED_FIXTURE passed as -e flags. The values below must stay in sync
    // with the `Start env-pinned container` step in .github/workflows/test.yml.
```

**Step 2: Add the npm script**

```json
		"test:integration:docker:env": "set WDIO_EXTERNAL_SERVER=1&& set WDIO_SERVER_MODE=1&& set WDIO_ENV_PINNED=1&& wdio run tests/integration/wdio.server.conf.ts",
```

**Step 3: Add the CI job**

After `integration-test-docker` in [test.yml](../../.github/workflows/test.yml):

```yaml
  # Env-pinned settings suite — needs the fixture variables present when the
  # server process starts, so it gets its own container. Never ran in CI before
  # Task 73; see _tasks/73-web-first-migration/02-research.md section 3.2.
  integration-test-docker-env:
    name: Integration Tests (Docker/Chrome - Env-Pinned)
    needs: [check-changes, integration-build-docker]
    if: needs.check-changes.outputs.run_tests == 'true'
    runs-on: ubuntu-latest

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: 'npm'

      - name: Setup Chrome
        uses: browser-actions/setup-chrome@v1
        with:
          chrome-version: stable

      - name: Install npm dependencies
        run: npm ci

      - name: Download Docker image
        uses: actions/download-artifact@v4
        with:
          name: docker-image
          path: /tmp

      - name: Load Docker image
        run: docker load -i /tmp/kniha-jazd-web.tar

      # Values mirror ENV_PINNED_FIXTURE in tests/integration/wdio.server.conf.ts.
      # The hosts do not have to resolve — the assertions only check the fields are pinned.
      - name: Start env-pinned container
        run: |
          mkdir -p data-env
          docker run -d --name kniha-jazd-web-env \
            --network=host \
            -v "$PWD/data-env:/data" \
            -v "$PWD/tests/integration/data:/testdata:ro" \
            -e KNIHA_JAZD_DATA_DIR=/data \
            -e DATABASE_PATH=/data/kniha-jazd.db \
            -e KNIHA_JAZD_MOCK_GEMINI_DIR=/testdata/mocks \
            -e PORT=3456 \
            -e HA_URL=http://env-pinned-ha.test:8123 \
            -e HA_API_TOKEN=env-pinned-ha-token \
            -e PAPERLESS_URL=https://env-pinned-paperless.test \
            -e PAPERLESS_API_TOKEN=env-pinned-paperless-token \
            -e PAPERLESS_ENABLED=true \
            -e KNIHA_JAZD_REVEAL_PIN=4269 \
            kniha-jazd-web:test

      - name: Wait for server health
        run: |
          for i in {1..60}; do
            if curl -sf http://localhost:3456/health >/dev/null; then
              echo "Server is healthy"
              exit 0
            fi
            sleep 1
          done
          echo "::error::Server did not become healthy within 60s"
          docker logs kniha-jazd-web-env
          exit 1

      - name: Run env-pinned integration tests
        run: npm run test:integration:docker:env

      - name: Show container logs on failure
        if: failure()
        run: docker logs kniha-jazd-web-env || true

      - name: Tear down container
        if: always()
        run: docker rm -f kniha-jazd-web-env || true

      - name: Upload test screenshots
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: integration-test-screenshots-docker-env
          path: tests/integration/screenshots/
          if-no-files-found: ignore
```

**Step 4: Verify locally**

```bash
docker rm -f kj-test 2>/dev/null
docker run -d --name kj-env --network=host \
  -v "$PWD/data-env:/data" -v "$PWD/tests/integration/data:/testdata:ro" \
  -e KNIHA_JAZD_DATA_DIR=/data -e DATABASE_PATH=/data/kniha-jazd.db \
  -e PORT=3456 -e HA_URL=http://env-pinned-ha.test:8123 -e HA_API_TOKEN=env-pinned-ha-token \
  -e PAPERLESS_URL=https://env-pinned-paperless.test -e PAPERLESS_API_TOKEN=env-pinned-paperless-token \
  -e PAPERLESS_ENABLED=true -e KNIHA_JAZD_REVEAL_PIN=4269 kniha-jazd-web:test
curl -sf http://localhost:3456/health && npm run test:integration:docker:env
```

Expected: 8 tests PASS. Tear down with `docker rm -f kj-env`.

**Step 5: Commit**

```bash
git add tests/integration/wdio.server.conf.ts package.json .github/workflows/test.yml
git commit -m "ci(test): run the env-pinned settings suite in GitHub Actions"
```

---

## Phase 5 — Fix the flaky skips (R2.c, R2.d)

Both must be green **before** Phase 8 deletes the Tauri harness. R2.c in particular is
covered *only* by Tauri today, so deleting first would drop a live use-case silently.

### Task 9: De-flake the API-key placeholder test

**Files:**
- Modify: [tests/integration/specs/tier2/receipt-settings.spec.ts](../../tests/integration/specs/tier2/receipt-settings.spec.ts) lines 154-173

**Step 1: Replace the pause chain with a wait on the settled state**

The spec skips in server mode because it uses fixed `browser.pause()` calls to wait for ~10
sequential RPC round-trips. Waiting on the observable outcome instead is both faster and
mode-independent:

```ts
    it('should show a placeholder instead of the saved API key', async function () {
      await setGeminiApiKey('test-display-key');
      await navigateTo('trips');
      await navigateTo('settings');

      const apiKeyInput = await $(ReceiptSettings.geminiApiKeyInput);
      // The settings page issues ~10 sequential RPCs before receipt settings land,
      // which is slow over HTTP — wait for the field to settle rather than guessing
      // a duration. Write-only: the field stays empty and advertises the stored key.
      await browser.waitUntil(
        async () => (await apiKeyInput.getAttribute('placeholder')) === '********',
        { timeout: 15000, timeoutMsg: 'receipt settings never loaded into the API key field' }
      );
      expect(await apiKeyInput.getValue()).toBe('');

      await setGeminiApiKey('');
    });
```

**Step 2: Run it in both modes**

```bash
# Docker (the mode that was skipped)
WDIO_EXTERNAL_SERVER=1 WDIO_SERVER_MODE=1 npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/receipt-settings.spec.ts

# Tauri (must not regress while it still exists)
npx wdio run tests/integration/wdio.conf.ts \
  --spec tests/integration/specs/tier2/receipt-settings.spec.ts
```

Expected: PASS in both, no skip reported.

Run the Docker one **three times**. A test that passes once is not proof a flakiness fix
worked.

**Step 3: Commit**

```bash
git add tests/integration/specs/tier2/receipt-settings.spec.ts
git commit -m "test(settings): wait for settled state instead of skipping in server mode"
```

### Task 10: Fix the BEV badge test

**Files:**
- Modify: [tests/integration/specs/existing/ev-vehicle.spec.ts](../../tests/integration/specs/existing/ev-vehicle.spec.ts) lines 213-230

**Step 1: Diagnose before changing**

The TODO says the badge exists in the UI but `createBevVehicleViaUI` does not reliably make
it visible. **Do not guess.** Use `superpowers:systematic-debugging`: unskip it, run it,
read the actual failure.

```bash
# Change it.skip( to it( first, then:
WDIO_EXTERNAL_SERVER=1 WDIO_SERVER_MODE=1 npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/existing/ev-vehicle.spec.ts
```

**Step 2: Most likely fix**

The badge renders as `class="badge type-bev"`. If the failure is a race between vehicle
creation and list re-render, wait on the badge rather than asserting immediately:

```ts
    const badge = await $('.badge.type-bev');
    await badge.waitForDisplayed({
      timeout: 10000,
      timeoutMsg: 'BEV badge did not appear in the vehicle list after creation'
    });
```

If instead the helper itself is not completing the creation, fix the helper — a test that
waits longer for something that never happens is worse than a skip.

**Step 3: Run three times, then commit**

```bash
git add tests/integration/specs/existing/ev-vehicle.spec.ts
git commit -m "test(vehicles): un-skip and stabilise the BEV badge assertion"
```

> If this one resists after a reasonable attempt, **stop and report** rather than sinking
> the migration into it. It is the only item in this plan that is not on the critical path:
> flag it, leave the skip with an updated comment pointing at what you found, and carry on.
> Say so explicitly in the phase summary — do not quietly leave it looking done.

---

## Phase 6 — Delete the dead test scripts (R2.e, D1)

### Task 11: Remove Playwright and the empty vitest setup

**Files:**
- Delete: [tests/e2e/](../../tests/e2e/), [playwright.config.ts](../../playwright.config.ts), `playwright-report/`, `test-results/`
- Delete: [vitest.config.ts](../../vitest.config.ts)
- Modify: [package.json](../../package.json)
- Modify: [.github/workflows/test.yml](../../.github/workflows/test.yml) lines 394-407 (the commented-out block)

**Step 1: Confirm vitest really matches nothing**

```bash
npx vitest run --passWithNoTests
find src -name "*.test.ts" -o -name "*.spec.ts" | wc -l
```

Expected: `0`. If this prints anything else, **stop** — the premise for deleting the vitest
setup is wrong and this needs re-deciding with the user.

**Step 2: Delete**

```bash
git rm -r tests/e2e playwright.config.ts vitest.config.ts
rm -rf playwright-report test-results
```

Remove from [package.json](../../package.json): the `test`, `test:run`, `test:e2e`,
`test:e2e:ui` scripts and the `@playwright/test` + `vitest` devDependencies. Rewrite
`test:all`:

```json
		"test:all": "npm run test:backend && npm run test:integration"
```

Delete the commented-out `e2e-tests` job block from
[test.yml](../../.github/workflows/test.yml).

**Step 3: Verify**

```bash
npm install
npm run check
grep -n '"test' package.json
```

Expected: only `test:backend`, `test:integration*`, `test:all` remain. Cross-check each
against `grep "run: npm run" .github/workflows/test.yml` — this is the I1 check.

**Step 4: Commit**

```bash
git add -u && git add package.json package-lock.json .github/workflows/test.yml
git commit -m "test: drop Playwright and the empty vitest setup (D1)"
```

---

## Phase 7 — Point the local harness at the web binary (R3)

### Task 12: Spawn `kniha-jazd-web` instead of the Tauri binary

**Files:**
- Modify: [tests/integration/wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) — `getBinaryPath()` and the `spawn` call in `onPrepare`

**Step 1: Repoint the binary path**

```ts
/**
 * Path to the headless web server binary. CI can override via KJ_WEB_BINARY.
 */
function getBinaryPath(): string {
  if (process.env.KJ_WEB_BINARY) {
    return process.env.KJ_WEB_BINARY;
  }
  const base = join(__dirname, '../../src-tauri/target/debug');
  return process.platform === 'win32'
    ? join(base, 'kniha-jazd-web.exe')
    : join(base, 'kniha-jazd-web');
}
```

**Step 2: Repoint the spawn env**

The web binary reads `PORT` / `STATIC_DIR` / `KNIHA_JAZD_DATA_DIR` / `DATABASE_PATH` (see
[web/src/main.rs](../../src-tauri/web/src/main.rs)), not `KNIHA_JAZD_SERVER_AUTOSTART` /
`KNIHA_JAZD_SERVER_PORT`:

```ts
    serverProcess = spawn(binaryPath, [], {
      env: {
        ...process.env,
        KNIHA_JAZD_DATA_DIR: testDataDir,
        DATABASE_PATH: join(testDataDir, 'kniha-jazd.db'),
        STATIC_DIR: join(__dirname, '../../build'),
        PORT: String(SERVER_PORT),
        KNIHA_JAZD_MOCK_GEMINI_DIR: join(__dirname, 'data', 'mocks'),
        ...SCRUBBED_ENV,
        ...(ENV_PINNED ? ENV_PINNED_FIXTURE : {}),
      },
      stdio: 'ignore',
    });
```

Rename the `tauriProcess` variable to `serverProcess` throughout, and update the log lines
and comments that say "Tauri binary".

**Step 3: Verify the loop works**

```bash
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
TIER=1 npm run test:integration:server
```

Expected: tier 1 PASS. This is the new local loop — no Tauri debug build, no tauri-driver.

**Step 4: Commit**

```bash
git add tests/integration/wdio.server.conf.ts
git commit -m "test(integration): spawn the web binary instead of the Tauri shell"
```

### Task 13: Delete the Tauri wdio config

**Files:**
- Delete: [tests/integration/wdio.conf.ts](../../tests/integration/wdio.conf.ts)
- Modify: [package.json](../../package.json)
- Modify: [tests/integration/utils/skip.ts](../../tests/integration/utils/skip.ts) → delete
- Modify: [tests/integration/specs/tier2/route-map.spec.ts](../../tests/integration/specs/tier2/route-map.spec.ts) line 140
- Modify: [tests/integration/specs/tier2/receipt-settings.spec.ts](../../tests/integration/specs/tier2/receipt-settings.spec.ts) line 241
- Modify: [tests/integration/utils/app.ts](../../tests/integration/utils/app.ts), [tests/integration/utils/db.ts](../../tests/integration/utils/db.ts)

**Step 1: Remove the last skip helpers**

Only one mode remains, so `describeNotInTauriMode` in
[route-map.spec.ts](../../tests/integration/specs/tier2/route-map.spec.ts) becomes plain
`describe(`. `describeNotInServerMode('Database Move Commands', ...)` in
[receipt-settings.spec.ts](../../tests/integration/specs/tier2/receipt-settings.spec.ts)
guards a feature that disappears in Phase 8 — **delete that whole describe block and its
`getDbLocation`/`checkTargetHasDb` helpers**, per I2 (deleting a feature's tests with the
feature is allowed).

Then delete [utils/skip.ts](../../tests/integration/utils/skip.ts).

**Step 2: Collapse the dual-mode branches in the utils**

In [utils/app.ts](../../tests/integration/utils/app.ts), drop the `isServerMode` check and
the Tauri IPC-bridge wait — DOM ready is the only condition now. In
[utils/db.ts](../../tests/integration/utils/db.ts), delete the `IS_SERVER_MODE` branch in
`invokeTauri` so it always uses HTTP RPC, and rename it to `rpc` (update call sites with a
project-wide find/replace; verify with
`grep -rn "invokeTauri" tests/`). Remove the `__TAURI__` global declarations.

**Step 3: Delete the config and its scripts**

```bash
git rm tests/integration/wdio.conf.ts tests/integration/utils/skip.ts
```

From [package.json](../../package.json), remove `test:integration:build`,
`test:integration:tier1/2/3`, and repoint `test:integration` at the server config:

```json
		"test:integration": "wdio run tests/integration/wdio.server.conf.ts",
		"test:integration:tier1": "set TIER=1&& npm run test:integration",
		"test:integration:tier2": "set TIER=2&& set PARALLEL_TIERS=true&& npm run test:integration",
		"test:integration:tier3": "set TIER=3&& set PARALLEL_TIERS=true&& npm run test:integration",
```

**Step 4: Full sweep in both Docker and spawned modes**

```bash
npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
for t in 1 2 3; do TIER=$t PARALLEL_TIERS=true npm run test:integration:server || break; done
```

Expected: all green, zero skips.

**Step 5: Commit**

```bash
git add -u && git add package.json
git commit -m "test(integration): retire the tauri-driver harness"
```

> **Phase gate — the last reversible point.** Everything up to here leaves a working desktop
> app. Confirm with the user before Phase 8.

---

## Phase 8 — Delete the desktop app (R5, D2)

### Task 14: Rewrite the workflows

**Files:**
- Modify: [.github/workflows/test.yml](../../.github/workflows/test.yml)
- Modify: [.github/workflows/release.yml](../../.github/workflows/release.yml)

**Step 1: Strip test.yml**

Delete the `integration-build`, `integration-tests`, and `integration-test-server` jobs
entirely — including the 40-line EdgeDriver registry-probing block (lines 151-190) and the
`windows-2022` pin whose comment records jobs hanging "for hours via retries".

Shrink the `backend-tests` matrix to `ubuntu-latest` only, and drop the
`Install Linux dependencies` step (WebKitGTK was for Tauri; the web binary needs none).

Final job list: `check-changes`, `backend-tests`, `integration-build-docker`,
`integration-test-docker` (3 tiers), `integration-test-docker-env`.

**Step 2: Strip release.yml**

Delete the `build` matrix job, the `Extract release notes from CHANGELOG` step (it reads
`src-tauri/desktop/tauri.conf.json`, which is about to not exist), and the duplicated
`integration-build` / `integration-tests` jobs. Repoint `docker-image`'s `needs:` at what
remains. Per [D2](./01-task.md#resolved-decisions) a `v*` tag now produces **no GitHub
Release** — only the ghcr push.

**Step 3: Validate the YAML before pushing**

```bash
npx --yes yaml-lint .github/workflows/test.yml .github/workflows/release.yml \
  || python -c "import yaml,sys; [yaml.safe_load(open(f)) for f in sys.argv[1:]]" \
     .github/workflows/test.yml .github/workflows/release.yml
```

Expected: no parse errors.

**Step 4: Commit**

```bash
git add .github/workflows/
git commit -m "ci: drop the desktop build and release pipelines (D2)"
```

### Task 15: Delete the desktop crate

**Files:**
- Delete: [src-tauri/desktop/](../../src-tauri/desktop/)
- Modify: [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml), [Dockerfile.web](../../Dockerfile.web)
- Delete: `.tauri-keys/`, [scripts/stage-spa.mjs](../../scripts/stage-spa.mjs)

**Step 1: Delete and shrink the workspace**

```bash
git rm -r src-tauri/desktop scripts/stage-spa.mjs
rm -rf .tauri-keys
```

In [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml):

```toml
members = ["core", "web"]
```

In [Dockerfile.web](../../Dockerfile.web), delete the now-pointless line:

```dockerfile
RUN sed -i 's/members = \["core", "desktop", "web"\]/members = ["core", "web"]/' Cargo.toml
```

**Step 2: Verify the backend still builds and tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --workspace
```

Expected: PASS. Test count drops by exactly 2 (the `static_dir.rs` tests) — anything more
means something was in the desktop crate that should have moved to core first. **Stop and
investigate if the drop is larger.**

**Step 3: Commit**

```bash
git add -u && git add src-tauri/Cargo.toml Dockerfile.web
git commit -m "refactor: delete the Tauri desktop crate"
```

### Task 16: Strip the Tauri surface from the frontend

**Files:**
- Delete: [src/lib/stores/update.ts](../../src/lib/stores/update.ts), [src/lib/components/UpdateModal.svelte](../../src/lib/components/UpdateModal.svelte), [src/lib/open-external.ts](../../src/lib/open-external.ts)
- Modify: [src/lib/api-adapter.ts](../../src/lib/api-adapter.ts), [src/lib/api.ts](../../src/lib/api.ts), [src/routes/+layout.svelte](../../src/routes/+layout.svelte), [src/routes/settings/+page.svelte](../../src/routes/settings/+page.svelte), [src/routes/doklady/+page.svelte](../../src/routes/doklady/+page.svelte), [src/lib/stores/capabilities.ts](../../src/lib/stores/capabilities.ts)
- Modify: [package.json](../../package.json)

**Step 1: Collapse the adapter**

[api-adapter.ts](../../src/lib/api-adapter.ts) becomes:

```ts
export async function apiCall<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const response = await fetch('/api/rpc', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-KJ-Client': '1',
    },
    body: JSON.stringify({ command, args: args ?? {} }),
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(text);
  }
  return response.json();
}
```

**Step 2: Work outward from the compiler**

```bash
git rm src/lib/stores/update.ts src/lib/components/UpdateModal.svelte src/lib/open-external.ts
npm run check
```

Fix each error it reports. The list, for orientation:

- [+layout.svelte](../../src/routes/+layout.svelte) — remove the `updateStore` import,
  the update-check block (lines ~88-99), the banner and `<UpdateModal>` (lines ~189, 247,
  262-271), and the `IS_TAURI` window-sizing block (lines ~141-146) with its
  `getCurrentWindow`/`LogicalSize` import
- [settings/+page.svelte](../../src/routes/settings/+page.svelte) — remove the dialog /
  opener / `appDataDir` imports and the `IS_TAURI` branches at lines 87 and 759
- [doklady/+page.svelte](../../src/routes/doklady/+page.svelte) — remove the `listen`
  import, the `unlistenProgress` state, the `onMount` `IS_TAURI` block (line ~56) and its
  `onDestroy` cleanup. Per [D3](./01-task.md#resolved-decisions) the progress display is an
  accepted loss — delete the UI that consumed it, do not stub it.
- [api.ts](../../src/lib/api.ts) — remove `openExportPreview` and the `revealItemInDir`
  branch (line ~227)

**Step 3: Flatten the capabilities store**

Only server mode exists. [capabilities.ts](../../src/lib/stores/capabilities.ts) keeps
`readOnly` (still meaningful — the `check_read_only!` macro is alive) and loses the feature
flags. Update the 13 `$capabilities.features.*` call sites: `routeMaps` is now always true
(delete the guards in [mapa/+page.svelte](../../src/routes/mapa/+page.svelte) and
[TripRow.svelte](../../src/lib/components/TripRow.svelte)), `fileDialogs` / `updater` /
`openExternal` / `moveDatabase` always false (delete the guarded blocks), `restoreBackup`
always true.

**Step 4: Drop the dependencies**

Remove all 7 `@tauri-apps/*` entries from [package.json](../../package.json), then:

```bash
npm install
npm run check
grep -rn "@tauri-apps\|IS_TAURI" src/
```

Expected: `npm run check` clean, grep returns nothing.

**Step 5: Commit**

```bash
git add -u && git add package.json package-lock.json
git commit -m "refactor(ui): remove the Tauri client surface"
```

### Task 17: Full verification sweep

**Step 1: Everything, from a clean build**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --workspace
npm run build
docker build -f Dockerfile.web -t kniha-jazd-web:test .
```

**Step 2: Full Docker sweep, all tiers plus env**

```bash
docker run -d --name kj-verify --network=host \
  -v "$PWD/data:/data" -v "$PWD/tests/integration/data:/testdata:ro" \
  -e KNIHA_JAZD_DATA_DIR=/data -e DATABASE_PATH=/data/kniha-jazd.db \
  -e KNIHA_JAZD_MOCK_GEMINI_DIR=/testdata/mocks -e PORT=3456 kniha-jazd-web:test
for t in 1 2 3; do TIER=$t PARALLEL_TIERS=true npm run test:integration:docker || break; done
docker rm -f kj-verify
```

**Step 3: Check the invariants mechanically**

```bash
# I1 — every test script has a job
grep -n '"test' package.json
grep -n "run: npm run\|run: cargo" .github/workflows/test.yml

# I2 — no skips left
grep -rnE "describeNotIn[A-Za-z]+\(|it\.skip\(|this\.skip\(\)" tests/integration/specs/

# No Tauri outside history
grep -rin tauri --exclude-dir=node_modules --exclude-dir=.git \
  --exclude=CHANGELOG.md --exclude=DECISIONS.md . | grep -v "_tasks/_done/"
```

Expected: the two greps agree; the skip grep is empty; the Tauri grep returns only
[_tasks/73-web-first-migration/](.) and files Task 18 is about to update.

**Do not proceed to Phase 9 with any of these failing.** Report instead.

---

## Phase 9 — Documentation (R6)

### Task 18: Write the ADR and update the docs

**Files:**
- Modify: [DECISIONS.md](../../DECISIONS.md) — new ADR at the top
- Modify: [CHANGELOG.md](../../CHANGELOG.md), [CLAUDE.md](../../CLAUDE.md), [ARCHITECTURE.md](../../ARCHITECTURE.md), [README.md](../../README.md), [README.en.md](../../README.en.md)
- Modify: [.claude/rules/integration-tests.md](../../.claude/rules/integration-tests.md), [.claude/rules/rust-backend.md](../../.claude/rules/rust-backend.md), [.claude/rules/svelte-frontend.md](../../.claude/rules/svelte-frontend.md)
- Modify: [docs/features/server-mode.md](../../docs/features/server-mode.md), [docs/features/move-database.md](../../docs/features/move-database.md)

**Step 1: The ADR**

Use the `/decision` skill. It supersedes
[ADR-001](../../DECISIONS.md#adr-001-desktop-app-with-tauri--sveltekit) and completes
[ADR-024](../../DECISIONS.md#adr-024-homelab-server-is-the-canonical-deployment-desktop-becomes-a-browser-client).
It must record D1-D3 and two consequences worth stating outright: GitHub Releases stop
entirely, and folder-scanned receipts survive as an unmaintained path rather than the intake
channel.

**Step 2: The CHANGELOG entry**

Per [D2](./01-task.md#resolved-decisions) this is the **only** user-facing announcement. Use
`/changelog`. It must say plainly, in Slovak: the desktop app is discontinued, no further
installers or automatic updates will be published, and the browser UI replaces it.

**Step 3: The rest**

[CLAUDE.md](../../CLAUDE.md) carries 18 Tauri mentions — the architecture diagram, the
"Common Commands" block, the test commands, and the database-location section (the custom
location / lock-file feature is gone). [ARCHITECTURE.md](../../ARCHITECTURE.md) and both
READMEs describe a desktop app in their opening lines.
[docs/features/move-database.md](../../docs/features/move-database.md) documents a deleted
feature — delete the file and remove it from any index.

**Step 4: Commit**

```bash
git add -u
git commit -m "docs: record the web-first migration and retire desktop references"
```

### Task 19: Close out the task

**Step 1: Manual steps the diff cannot cover**

Tell the user, explicitly, that these remain:

- Delete `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_KEY_PASSWORD` from the repository
  secrets in GitHub settings.
- Decide whether to delete or archive the old desktop GitHub Releases (this plan leaves them
  in place — they are history, and removing them breaks nothing).

**Step 2: Move the task folder**

```bash
git mv _tasks/73-web-first-migration _tasks/_done/73-web-first-migration
```

Update [_tasks/index.md](../index.md): move row 73 from Active to Completed with today's
date, and fix the link path to `./_done/73-web-first-migration/`. Update the tech-debt table
— [_TECH_DEBT/07](../_TECH_DEBT/07-integration-db-reset-broken.md) is now moot, since the
`wdio.conf.ts` launcher/worker bug it describes died with the file. Mark it resolved with a
decision-log entry pointing here.

**Step 3: Commit**

```bash
git add -A _tasks/
git commit -m "docs: complete task 73 web-first migration"
```

---

## Execution notes

**Order is load-bearing.** Phases 1-7 leave a working desktop app at every commit. Phase 8
does not. The point of running every test fix under Docker while the Tauri harness still
exists is that a use-case this migration would silently drop shows up as a red job, not as a
deleted file.

**If a phase gate fails, stop and report.** Do not carry a red suite into the next phase —
after Phase 8 there is no second harness to bisect against.

**Two places to expect trouble:**

- **Task 6** — the receipts specs mix paths the *test process* reads (host) with paths the
  *backend* reads (container). Getting one wrong produces a confusing "file not found" from
  the wrong side of the RPC. When in doubt, ask which process opens the file.
- **Task 16** — the frontend strip is wide. Let `npm run check` drive it rather than
  grepping for `IS_TAURI` and deleting by eye; the compiler finds the call sites that a grep
  for the flag will not.

**Related:** [01-task.md](./01-task.md) (requirements, decisions),
[02-research.md](./02-research.md) (evidence),
[executing-plans](../../.claude/skills/) for the task-by-task workflow.
