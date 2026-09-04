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

The blocking gap, and it is **three** differences, not two. The repo already documents them,
in the doc comment at
[route_maps_tests.rs:491](../../src-tauri/core/src/commands_internal/route_maps_tests.rs):

> The two export paths differ: desktop prepends a synthetic "Prvý záznam" row **and**
> honours the user's sort direction, server mode does neither.

Add hidden columns to that list. So
[export_html_internal](../../src-tauri/core/src/commands_internal/export_cmd.rs) must gain
all three, or Phase 8 makes the printed logbook permanently lose its year-opening odometer
baseline — a row the on-screen grid still renders
([TripGrid.svelte:436](../../src/lib/components/TripGrid.svelte), `FIRST_RECORD_ID`), so the
export would stop matching the screen. That is exactly the regression
[Goal 2](./01-task.md#goals) forbids.

### Task 1: Bring `export_html_internal` to parity with the desktop export

**Files:**
- Modify: [src-tauri/core/src/commands_internal/export_cmd.rs](../../src-tauri/core/src/commands_internal/export_cmd.rs)
- Modify: [src-tauri/core/src/server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs) lines 197-218
- Modify: [src-tauri/core/src/commands_internal/route_maps_tests.rs](../../src-tauri/core/src/commands_internal/route_maps_tests.rs) line 491 (the doc comment above goes stale the moment this lands)
- Modify: [src-tauri/desktop/src/commands/export_cmd.rs](../../src-tauri/desktop/src/commands/export_cmd.rs) (the `export_html` wrapper — keep it compiling; it dies in Phase 8)

> **Open question for the user — do not decide silently.** Desktop pushes the first record
> *unconditionally*; core returns `Err("No trips found for this year")` for an empty year.
> Once core is the only path, exporting an empty year either produces a one-row document
> containing nothing but the synthetic placeholder (desktop's behaviour) or shows an error
> toast (core's). This plan **keeps the error**, on the grounds that a document whose only
> row is a placeholder is not useful output. Flag it at the Phase 1 gate; it is a one-line
> change either way.

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

        // The synthetic year-opening row. Desktop prepends it; web mode never did,
        // so after the migration the printed logbook would silently lose the
        // baseline odometer the on-screen grid still shows.
        assert!(
            ascending.contains("Prvý záznam"),
            "the synthetic first-record row is missing from the web export"
        );
        assert!(
            ascending.contains("10000"),
            "the first-record row should carry year_start_odometer (10000)"
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

**Step 3b: Prepend the synthetic first record**

Port the block from
[desktop/src/commands/export_cmd.rs](../../src-tauri/desktop/src/commands/export_cmd.rs).
**Order matters** — it must land after `build_trip_grid_data` and before `ExportTotals` and
`assemble_export_rows`, because the row participates in numbering. Make `grid_data` `mut`:

```rust
    let mut grid_data = statistics::build_trip_grid_data(db, &vehicle_id, year)?;

    if grid_data.trips.is_empty() {
        return Err("No trips found for this year".to_string());
    }

    // Synthetic year-opening row: carries the odometer the year started at, so the
    // printed logbook shows the same baseline the on-screen grid does
    // (TripGrid.svelte FIRST_RECORD_ID). Uuid::nil() is the marker export.rs keys
    // its special-case rendering off (`is_first_record`).
    let first_record_date =
        chrono::NaiveDate::from_ymd_opt(year, 1, 1).ok_or_else(|| "Invalid year".to_string())?;
    let first_record = crate::models::Trip {
        id: uuid::Uuid::nil(),
        vehicle_id: vehicle.id,
        start_datetime: first_record_date.and_hms_opt(0, 0, 0).unwrap(),
        end_datetime: None,
        origin: "-".to_string(),
        destination: "-".to_string(),
        distance_km: 0.0,
        odometer: grid_data.year_start_odometer,
        purpose: "Prvý záznam".to_string(),
        fuel_liters: None,
        fuel_cost_eur: None,
        full_tank: true,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    grid_data.trips.push(first_record);
    grid_data
        .fuel_remaining
        .insert(uuid::Uuid::nil().to_string(), grid_data.year_start_fuel);
    grid_data.trip_numbers.insert(uuid::Uuid::nil().to_string(), 0);
    grid_data
        .odometer_start
        .insert(uuid::Uuid::nil().to_string(), grid_data.year_start_odometer);
```

Two things to watch:

- `vehicle.id` is read here but `vehicle` is moved into `ExportData` later — read the id
  before the move, or clone it.
- `ExportTotals::calculate` already excludes dummy rows
  (`test_export_totals_excludes_dummy_rows` in
  [export_tests.rs](../../src-tauri/core/src/export_tests.rs)), so the totals are unaffected
  by the extra row. Do not "fix" them.

**Step 3c: Update the doc comment that now describes the old world**

[route_maps_tests.rs:491](../../src-tauri/core/src/commands_internal/route_maps_tests.rs)
says "desktop prepends a synthetic 'Prvý záznam' row and honours the user's sort direction,
server mode does neither". After this task both paths do both. Rewrite it to say the paths
now agree, and keep the test — its point (both cite the same record number for the same
trip) is exactly what protects this change.

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

Delete the now-unused `getVersion` import from `@tauri-apps/api/app`, **and the `IS_TAURI`
import at line 17** — line 695 is its only use in this file. The desktop branches at lines
87 and 759 are `$capabilities.mode === 'desktop'` checks, not `IS_TAURI`; they are Task 16's
problem. Verify with `grep -n "IS_TAURI" src/routes/settings/+page.svelte` — expect no hits
after this step.

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
 * Keep the mount targets in sync with the `-v` flags in the `Start container` step of
 * .github/workflows/test.yml and the local `docker run` in 03-plan.md. NOT with
 * docker-compose.web.yml — that is the production-shaped deployment file and has no
 * business mounting test fixtures.
 *
 * There are TWO mappings, and conflating them is the main way to waste an hour here:
 *
 *   fixtures  read-only   repo tests/integration/data  ->  /testdata
 *   workdir   read-write  host $PWD/data               ->  /data
 *
 * Fixtures are committed inputs (invoice PDFs, Gemini mock JSON). The workdir is where
 * the running instance keeps its database and where a spec that *creates* files for the
 * backend to find must write.
 */
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const IS_DOCKER_MODE = process.env.WDIO_EXTERNAL_SERVER === '1';

/** Where tests/integration/data/ is mounted inside the container (read-only). */
export const CONTAINER_FIXTURE_ROOT = '/testdata';

/** Where the writable data dir is mounted inside the container. */
export const CONTAINER_WORK_ROOT = '/data';

/** Host path to tests/integration/data/ — for reads the *test process* performs. */
export const HOST_FIXTURE_ROOT = join(__dirname, '..', 'data');

/**
 * Host path to the writable data dir the backend also sees.
 * Docker: the bind-mount source. Spawned mode: the temp dir wdio exported.
 */
export function hostWorkDir(): string {
  if (IS_DOCKER_MODE) return join(process.cwd(), 'data');
  const dir = process.env.KNIHA_JAZD_DATA_DIR;
  if (!dir) {
    throw new Error('KNIHA_JAZD_DATA_DIR not set — spawned-server mode should export it');
  }
  return dir;
}

/**
 * A committed fixture path as the BACKEND sees it.
 * Use for any fixture path sent over RPC; use HOST_FIXTURE_ROOT for fs calls in the spec.
 */
export function backendFixturePath(...segments: string[]): string {
  return IS_DOCKER_MODE
    ? [CONTAINER_FIXTURE_ROOT, ...segments].join('/')
    : join(HOST_FIXTURE_ROOT, ...segments);
}

/**
 * A path under the writable work dir as the BACKEND sees it.
 * Pair every call with hostWorkDir() for the write the test process performs.
 */
export function backendWorkPath(...segments: string[]): string {
  return IS_DOCKER_MODE
    ? [CONTAINER_WORK_ROOT, ...segments].join('/')
    : join(hostWorkDir(), ...segments);
}
```

The names are deliberately not `backendDataPath` — "data" is the word both mappings would
claim, and the whole point is to keep them apart.

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

Two things this step changes that are worth stating rather than discovering:

- **`:ro` on the fixture mount.** The container has no business writing into the repo, and
  receipt scanning appears to read only. If Task 6 hits a *permission* error rather than a
  not-found, `:ro` is the explanation and it means the pipeline rewrites files it scans —
  new information, worth reporting rather than silently dropping the flag.
- **`KNIHA_JAZD_MOCK_GEMINI_DIR` now applies to the whole Docker suite**, not just the two
  newly-unskipped describes. That is intended — it matches what
  [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) already does in
  `onPrepare` for spawned mode — but it is a behaviour change for specs that pass today.
  Watch for it at the Phase 3 gate.

Task 6b covers the separate, writable mapping that `seedReceipt` needs; the read-only
fixture mount does not serve that purpose.

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

### Task 6: Unskip the receipts suites (fixture paths only)

[receipts.spec.ts](../../tests/integration/specs/tier2/receipts.spec.ts) is the easy half:
it reads committed fixtures and hands their paths to the backend. Pure search-and-replace.
[multi-invoice.spec.ts](../../tests/integration/specs/tier2/multi-invoice.spec.ts) is
**not** — it is Task 6b, and its blocker is somewhere else entirely.

**Files:**
- Modify: [tests/integration/specs/tier2/receipts.spec.ts](../../tests/integration/specs/tier2/receipts.spec.ts) lines 95, 350 and the six `invoicesPath` joins

**Step 1: Swap the path construction**

There are exactly six occurrences of:

```ts
      const invoicesPath = join(__dirname, '..', '..', 'data', 'invoices');
```

at lines 148, 227, 296, 384, 432, 489. Replace each with:

```ts
      const invoicesPath = backendFixturePath('invoices');
```

and add the import:

```ts
import { backendFixturePath } from '../../utils/paths';
```

Drop the now-unused `join` / `__dirname` scaffolding if nothing else in the file uses it.

**Step 2: Change the describe wrappers**

Replace `describeNotInDockerMode(` with plain `describe(` at lines 95 and 350, and drop the
unused import.

**Step 3: Run the spec against the container**

```bash
WDIO_EXTERNAL_SERVER=1 WDIO_SERVER_MODE=1 npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/receipts.spec.ts
```

Expected: PASS, no skipped suites in the reporter output.

If a scan fails with a permission error rather than "not found", the `:ro` on the fixture
mount is the first thing to suspect — it means the receipts pipeline writes to files it
scans, which the mount forbids. That would be new information; report it rather than
silently dropping `:ro`.

**Step 4: Commit**

```bash
git add tests/integration/specs/tier2/receipts.spec.ts
git commit -m "test(integration): run the receipts suite in Docker mode"
```

### Task 6b: Make `seedReceipt` work across the container boundary

**This is a helper redesign, not a rename.**
[multi-invoice.spec.ts](../../tests/integration/specs/tier2/multi-invoice.spec.ts) contains
**no** `data/invoices` joins. It seeds through `seedReceipt`
([utils/db.ts:681](../../tests/integration/utils/db.ts)), which:

1. reads `getTestDataDir()` = `process.env.KNIHA_JAZD_DATA_DIR` — **unset in the test
   process** under Docker — and throws before doing anything else;
2. writes a placeholder file from the test process into `<dataDir>/seeded-receipts`;
3. hands the backend that same absolute path and scans it.

So it needs a location that is *writable by the test process* and *readable by the backend*
— the read-only fixture mount cannot provide that. The helper's own doc comment says so
("needs a filesystem shared between the test runner and the backend"), as does the spec
header.

**Files:**
- Modify: [tests/integration/utils/db.ts](../../tests/integration/utils/db.ts) — `getTestDataDir`, `seedReceipt`
- Modify: [tests/integration/specs/tier2/multi-invoice.spec.ts](../../tests/integration/specs/tier2/multi-invoice.spec.ts) lines 104, 119-121

**Step 1: Point the helper at the mapped work dir**

`$PWD/data` ↔ `/data` is already bind-mounted read-write by the CI `docker run`, so it is the
shared filesystem. In [db.ts](../../tests/integration/utils/db.ts), replace the
`getTestDataDir()` call inside `seedReceipt` with the pair from Task 4:

```ts
import { hostWorkDir, backendWorkPath } from './paths';

// ... inside seedReceipt:
  // Write host-side, tell the backend the path IT sees. In spawned mode the two
  // are identical; in Docker they are the two ends of the -v $PWD/data:/data mount.
  const seedDirHost = join(hostWorkDir(), 'seeded-receipts');
  const seedDirBackend = backendWorkPath('seeded-receipts');
  mkdirSync(seedDirHost, { recursive: true });
  writeFileSync(join(seedDirHost, fileName), PLACEHOLDER_BYTES);
  await rpc('set_receipts_folder_path', { path: seedDirBackend });
```

Delete the `KNIHA_JAZD_DATA_DIR not set` throw and update the doc comment — in particular
the line telling callers to wrap in `describeNotInDockerMode`, which is about to be false.

**Step 2: Fix the cleanup that silently no-ops**

The `after()` hook at
[multi-invoice.spec.ts:119-121](../../tests/integration/specs/tier2/multi-invoice.spec.ts)
is guarded by `if (dataDir)`. In Docker `dataDir` is empty, so cleanup skips and seeded
receipts persist in the container volume — exactly the cross-spec poisoning the hook's own
comment warns about, and the failure mode
[_TECH_DEBT/07](../_TECH_DEBT/07-integration-db-reset-broken.md) documents. Point it at
`hostWorkDir()` and drop the guard.

**Step 3: Unskip and run**

Change `describeNotInDockerMode(` to `describe(` at line 104.

```bash
WDIO_EXTERNAL_SERVER=1 WDIO_SERVER_MODE=1 npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/multi-invoice.spec.ts
```

Then immediately re-run the spec that the leak used to poison, to prove the cleanup works:

```bash
WDIO_EXTERNAL_SERVER=1 WDIO_SERVER_MODE=1 npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/multi-invoice.spec.ts \
  --spec tests/integration/specs/tier2/receipts.spec.ts
```

Expected: both PASS in that order. If receipts fails only in the pair, the cleanup is still
not running — check `hostWorkDir()` resolves to the same directory the `-v` flag names.

**Step 4: Commit**

```bash
git add tests/integration/utils/db.ts tests/integration/utils/paths.ts \
        tests/integration/specs/tier2/multi-invoice.spec.ts
git commit -m "test(integration): seed receipts across the container boundary"
```

### Task 7: Unskip export and backup restoration

**Files:**
- Modify: [tests/integration/specs/tier1/export.spec.ts](../../tests/integration/specs/tier1/export.spec.ts) lines 25-29 (the `this.skip()` is at 27)
- Modify: [tests/integration/specs/tier2/backup-restore.spec.ts](../../tests/integration/specs/tier2/backup-restore.spec.ts) line 161 (comment above at 159-160)

**Step 1: Delete both skips**

In [export.spec.ts](../../tests/integration/specs/tier1/export.spec.ts), remove the whole
`before` hook (the `this.skip()` is at line 27):

```ts
  before(function () {
    if (process.env.WDIO_SERVER_MODE === '1') {
      this.skip();
    }
  });
```

In [backup-restore.spec.ts](../../tests/integration/specs/tier2/backup-restore.spec.ts) line
161, change `describeNotInServerMode('Backup Restoration', ...)` to `describe(...)` and
delete the stale comment above it — its three claims are all false (see
[02-research.md §3.4](./02-research.md)). Drop the unused import if nothing else in the file
uses it.

**Step 1b: Give the export spec assertions worth unskipping**

Unskipping alone buys a green tick, not coverage.
[export.spec.ts](../../tests/integration/specs/tier1/export.spec.ts) as written asserts only
that city names, licence plate and company name appear — nothing about hidden columns, sort
order, or the first-record row, which is the entire subject of Phase 1. Worse, it is
structurally toothless: every assertion sits inside
`if (handles.length > originalHandles.length)`, the `else` branch only asserts when the URL
happens to contain `export` or `blob`, and a missing export button hits
`console.log('Export button not found, skipping test'); return;`. If the export window never
opens, the test passes having asserted nothing.

Under [I2](./01-task.md#coverage-invariants) the export-argument use-case must have real
end-to-end coverage once desktop is gone. So:

1. **Delete the silent-pass escapes.** A missing export button is a failure — replace the
   `console.log`/`return` with a `waitForDisplayed` that throws. Replace the
   `if (handles.length > ...)` guard with a `waitUntil` on the new window handle.
2. **Add a hidden-column assertion.** Hide a column through the grid UI (or seed it via
   `set_hidden_columns`), export, and assert the corresponding header string is absent from
   the exported document — the end-to-end mirror of Task 1's `CAS-MARKER` unit test.
3. **Add a sort-order assertion.** Flip the sort control, export, and assert two seeded
   destinations appear in the expected order.
4. **Add a first-record assertion.** Assert `Prvý záznam` is present — the C1 regression
   this phase exists to prevent, checked where a user would see it.

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

      # The env: block is NOT redundant with the npm script. The scripts use
      # Windows `set X=Y&&`, which under sh sets positional parameters and exports
      # nothing — so on ubuntu the wdio config would see EXTERNAL_SERVER=false,
      # run all four tier globs instead of ./specs/env/**, and try to spawn a
      # binary CI never built. The existing docker job passes them the same way.
      - name: Run env-pinned integration tests
        env:
          WDIO_SERVER_MODE: '1'
          WDIO_EXTERNAL_SERVER: '1'
          WDIO_ENV_PINNED: '1'
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
curl -sf http://localhost:3456/health
WDIO_SERVER_MODE=1 WDIO_EXTERNAL_SERVER=1 WDIO_ENV_PINNED=1 \
  npx wdio run tests/integration/wdio.server.conf.ts
```

Expected: 8 tests PASS, and the reporter lists specs from `specs/env/` **only**. If it lists
tier specs instead, the environment did not reach the config — that is the same failure mode
the `env:` block above prevents in CI (`set X=Y&&` is a no-op under Git Bash too).

Tear down with `docker rm -f kj-env`.

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
- Modify: [tests/integration/specs/tier2/receipt-settings.spec.ts](../../tests/integration/specs/tier2/receipt-settings.spec.ts) lines 154-173 (the `this.skip()` is at 158)

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

**Step 2: Note what is already there**

The spec **already** does `await bevBadge.waitForDisplayed({ timeout: 5000 })` at line 229.
So "add a wait" is a no-op — do not apply it and conclude the test is fixed. Raising the
timeout is equally unlikely to help: a test that waits longer for something that never
happens is worse than a skip.

The TODO's own suspicion is the place to start: `createBevVehicleViaUI` may not be
completing the creation at all. Verify the vehicle exists over RPC before asserting on the
badge — that splits "creation failed" from "list did not re-render", which are different
bugs with different fixes.

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
guards a feature that disappears in Phase 8 — **delete that whole describe block**, per I2
(deleting a feature's tests with the feature is allowed). Of its two local helpers only
`checkTargetHasDb` becomes unused; **keep `getDbLocation`**, which three surviving tests
still call (lines 176, 190, 206).

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

Now collapse the script list. There are currently three families (`test:integration:*`,
`:server:*`, `:docker:*`) because there were three harnesses; after this task there is one
backend and two ways to reach it (spawned locally, or a container). Write the **final** set
explicitly — this is what [I1](./01-task.md#coverage-invariants) is checked against in
Task 17:

```json
		"test:backend": "cargo test --manifest-path src-tauri/Cargo.toml --workspace",
		"test:integration": "wdio run tests/integration/wdio.server.conf.ts",
		"test:integration:tier1": "set TIER=1&& npm run test:integration",
		"test:integration:tier2": "set TIER=2&& set PARALLEL_TIERS=true&& npm run test:integration",
		"test:integration:tier3": "set TIER=3&& set PARALLEL_TIERS=true&& npm run test:integration",
		"test:integration:docker": "set WDIO_EXTERNAL_SERVER=1&& npm run test:integration",
		"test:integration:docker:env": "set WDIO_ENV_PINNED=1&& npm run test:integration:docker",
		"test:all": "npm run test:backend && npm run test:integration"
```

Delete `test:integration:build`, `test:integration:server`,
`test:integration:server:tier1`, `test:integration:server:env`,
`test:integration:docker:tier1` — the `:server:` family *is* `test:integration` now, and the
tier variants are covered by the `TIER` aliases.

`WDIO_SERVER_MODE` disappears from the scripts entirely: with one harness there is no other
mode to distinguish. Remove the reads of it in
[wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts),
[utils/app.ts](../../tests/integration/utils/app.ts) and
[utils/db.ts](../../tests/integration/utils/db.ts) as part of Step 2.

> **I1 note.** `test:integration:tier1/2/3` are aliases that only set `TIER` around
> `test:integration`, which CI does invoke. They are not separately-invoked suites, so they
> satisfy I1 by delegation. State this in [01-task.md](./01-task.md) when you get to Task 18
> — otherwise Task 17's literal check ("every `test:*` script is invoked by a job") fails on
> a technicality, *after* the point of no return.

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

**Keep the `backend-tests` matrix on all three platforms.** [R4](./01-task.md#r4--rewrite-both-pipelines)
says "keep `backend-tests`", and the crate still has platform-conditional code — DB path
resolution, `hostname`, the `#[cfg(unix)]` / `#[cfg(not(unix))]` shutdown handler in
[web/src/main.rs](../../src-tauri/web/src/main.rs) — that developers build and run on
Windows. Task 12 keeps a `win32` branch in `getBinaryPath()` for exactly that reason.
Shrinking the matrix is a coverage reduction, not a Tauri cleanup; if it is wanted, it is a
separate decision to record with `/decision`, not a line item inside a step about deleting
Windows *integration* jobs.

Do drop the `Install Linux dependencies` step — WebKitGTK was for Tauri, and the web binary
links none of it.

Final job list: `check-changes`, `backend-tests` (3 platforms),
`integration-build-docker`, `integration-test-docker` (3 tiers),
`integration-test-docker-env`.

**Step 2: Strip release.yml**

Delete the `build` matrix job, the `Extract release notes from CHANGELOG` step (it reads
`src-tauri/desktop/tauri.conf.json`, which is about to not exist), and the duplicated
`integration-build` / `integration-tests` jobs. Per
[D2](./01-task.md#resolved-decisions) a `v*` tag now produces **no GitHub Release** — only
the ghcr push.

`docker-image` already declares `needs: [check-tests, backend-tests]`, so its `needs:` needs
no change once the jobs around it are gone — just confirm both still exist.

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
- Modify: [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml), [Dockerfile.web](../../Dockerfile.web), [package.json](../../package.json), [vite.config.ts](../../vite.config.ts)
- Delete: `.tauri-keys/`, [scripts/stage-spa.mjs](../../scripts/stage-spa.mjs)

**Step 0: Replace the dev loop before deleting it**

`npm run tauri:dev` is the documented daily workflow ([CLAUDE.md](../../CLAUDE.md), "Common
Commands"). Deleting it without a replacement leaves the project with no way to run the app
locally — [vite.config.ts](../../vite.config.ts) has no proxy, so `npm run dev` alone cannot
reach a backend.

Add one to [vite.config.ts](../../vite.config.ts):

```ts
export default defineConfig({
	plugins: [sveltekit()],
	server: {
		proxy: {
			'/api': 'http://localhost:3456'
		}
	}
});
```

Then the loop is two processes: `cargo run --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web`
(with `STATIC_DIR` unset — vite serves the UI) alongside `npm run dev`. Document it in
[CLAUDE.md](../../CLAUDE.md) in Task 18.

Note `stage:spa` and `dev:server` are **not** `tauri:*`-prefixed and so escape a naive
deletion pass — `stage:spa` calls
[scripts/stage-spa.mjs](../../scripts/stage-spa.mjs), deleted in Step 1, and `dev:server`
launches the Tauri shell. Both go. Remove `tauri:prebuild` and the `tauri` passthrough too.

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

**Step 2b: Prune the now-dead database-location machinery**

[R5](./01-task.md#r5--delete-the-desktop-surface) lists this and no other task covers it.
[db_location.rs](../../src-tauri/core/src/db_location.rs) exists to support a feature
[ADR-024](../../DECISIONS.md#adr-024-homelab-server-is-the-canonical-deployment-desktop-becomes-a-browser-client)
retired: one `/data` volume means no custom paths and no multi-PC lock dance.

Work outward from the compiler rather than deleting by eye:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --workspace 2>&1 | grep "never used"
```

`resolve_db_paths` is still used by the web binary's path resolution — check before
removing anything. `acquire_lock` / `check_lock` / `LockStatus` had exactly one caller
(desktop `lib.rs`) and should now be dead.

Also decide `check_target_has_db`: it stays dispatched at
[dispatcher.rs:479](../../src-tauri/core/src/server/dispatcher.rs) while both its consumers
disappear — the settings "Change Location" UI (Task 16) and the integration tests (Task 13).
Remove the arm with the feature, or leave a comment saying why it stays. Do not leave it
undecided.

**Step 3: Commit**

```bash
git add -u && git add src-tauri/Cargo.toml Dockerfile.web
git commit -m "refactor: delete the Tauri desktop crate and dead db-location code"
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
- [api.ts](../../src/lib/api.ts) — remove `openExportPreview`, the `revealItemInDir` branch
  (line ~227), and the four desktop-only command wrappers that now call nothing:
  `getOptimalWindowSize` (line 331), `getServerStatus` (505), `startServer` (509),
  `stopServer` (513)

**Step 2b: Remove the `$capabilities.mode === 'desktop'` branches**

The frontend gates desktop behaviour **three** ways, not two: `IS_TAURI`,
`$capabilities.features.*`, and `$capabilities.mode`. The third is invisible to a grep for
either of the others, so handle it explicitly. All three sites are in
[settings/+page.svelte](../../src/routes/settings/+page.svelte):

- **line 87** — the desktop `revealSecret` path. Keep the *server* branch: that is the
  PIN-gated flow from [Task 69](../_done/69-pin-gated-secret-reveal/), which survives and is
  covered by the env suite.
- **line 759** — the `getServerStatus` load. Delete.
- **line 1531** — the **entire Server Mode section**: port input, start/stop button, server
  URL display, error line. It is driven by `startServer` / `stopServer` / `getServerStatus`,
  which are desktop-only commands the web dispatcher never had. Delete the section and its
  `settings.serverMode*` i18n keys from both
  [sk](../../src/lib/i18n/sk/index.ts) and [en](../../src/lib/i18n/en/index.ts), then run
  `npm run i18n` — nothing else regenerates `i18n-types.ts`.

A container that *is* the server has no UI for starting one.

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
npm run i18n
npm run check
grep -rn "@tauri-apps\|IS_TAURI\|capabilities.mode" src/
grep -rn "getServerStatus\|startServer\|stopServer\|getOptimalWindowSize" src/
```

Expected: `npm run check` clean, both greps return nothing. The `capabilities.mode` and
command-name greps matter — a check for `@tauri-apps|IS_TAURI` alone would report clean
while dead UI calling non-existent commands ships.

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

**Step 3: Repair the skills — `/release` is broken, not merely stale**

This is not documentation polish. After Phase 8
[release-skill/SKILL.md](../../.claude/skills/release-skill/SKILL.md) instructs an agent to:

- bump `"version"` in `src-tauri/desktop/tauri.conf.json` (step 3) — file deleted
- run `npm run test:integration:tier1` (step 4) — repointed in Task 13
- run `npm run tauri build` (step 5) — script and toolchain deleted
- treat a missing `TAURI_SIGNING_PRIVATE_KEY` warning as expected
- report an NSIS installer path (step 7) — no installer exists

And [D2](./01-task.md#resolved-decisions) changes what a release *is*: bump
[package.json](../../package.json) + the workspace
[Cargo.toml](../../src-tauri/Cargo.toml), tag, push, let CI publish the ghcr image, no
GitHub Release. Rewrite the skill to that flow — an ADR describing it is not enough, because
`/release` is what actually runs.

Then: delete [test-update-skill](../../.claude/skills/test-update-skill/) outright (it is
entirely about testing Tauri auto-update), and clear the Tauri references from
[code-review-skill](../../.claude/skills/code-review-skill/SKILL.md),
[test-review-skill](../../.claude/skills/test-review-skill/SKILL.md) and
[verify-skill](../../.claude/skills/verify-skill/SKILL.md). Drop `/test-update` from the
skills table in [CLAUDE.md](../../CLAUDE.md).

**Step 4: The rest**

[CLAUDE.md](../../CLAUDE.md) carries 18 Tauri mentions — the architecture diagram, the
"Common Commands" block, the test commands, and the database-location section (the custom
location / lock-file feature is gone). [ARCHITECTURE.md](../../ARCHITECTURE.md) and both
READMEs describe a desktop app in their opening lines.
[docs/features/move-database.md](../../docs/features/move-database.md) documents a deleted
feature — delete the file and remove it from any index.

Also fold in the I1 alias note from Task 13: record in
[01-task.md](./01-task.md#coverage-invariants) that `test:integration:tier1/2/3` satisfy I1
by delegating to a script CI invokes, so the criterion reads as intended.

**Step 5: Commit**

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
