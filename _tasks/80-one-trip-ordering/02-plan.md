# One Trip Ordering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make one Rust comparator the only thing that decides trip order, so the grid, the row editor and the odometer rewrite can no longer disagree with each other.

**Architecture:** Add `trip_order` to `helpers.rs` and route every trip sort through it. Delete the `(date, odometer)` pre-sort that `statistics.rs` copies in six places. Move the odometer arithmetic out of Svelte: the editor reads the backend's `odometerStart` and the existing preview command returns the resulting odometer, and the whole-year rewrite becomes one RPC command.

**Tech Stack:** Rust (kniha-jazd-core), SvelteKit, WebdriverIO.

**Spec:** [01-task.md](./01-task.md)

## Global Constraints

- Order is `start_datetime`, then `created_at`, then `odometer` ascending, then `id`. Each key applies only when the one before it carries no information.
- The [task 79](../79-odometer-span-inconsistency/) span warnings must still fire on the three 2026 rows after every task. A comparator that silences them is wrong.
- All business logic in Rust ([ADR-008](../../DECISIONS.md)). The frontend displays what the backend returns.
- Slovak UI strings go through i18n. Run `npm run i18n` after editing `src/lib/i18n/{sk,en}/index.ts`.
- Do not write to the production database. Test against a copy (`_tmp/79-local/`).
- Stage only the files of the task you are on. Never `git add -A`.

---

### Task 1: One comparator, used by the three helpers

**Files:**
- Modify: `src-tauri/core/src/commands_internal/helpers.rs:50-95` and `:113-119`
- Test: `src-tauri/core/src/commands_internal/commands_tests.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `pub fn trip_order(a: &Trip, b: &Trip) -> std::cmp::Ordering`, exported from `commands_internal::helpers`.

- [ ] **Step 1: Write the failing test**

Add to `commands_tests.rs`, next to the task 79 tests. `make_trip_at` already exists there.

```rust
#[test]
fn test_trip_order_prefers_created_at_over_odometer() {
    // The 2026-08-19 pair: same datetime, different created_at. The places
    // prove the 4 km hop to the petrol station came first, and created_at
    // agrees. The odometer must not override that.
    let date = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
    let mut first = make_trip_at(date, 15, 0);   // 4 km hop, higher odometer
    first.created_at = Utc::now() - chrono::Duration::seconds(60);
    first.odometer = 69415.0;
    let mut second = make_trip_at(date, 15, 0);  // 352 km leg, lower odometer
    second.created_at = Utc::now();
    second.odometer = 69411.0;

    assert_eq!(trip_order(&first, &second), std::cmp::Ordering::Less);
}

#[test]
fn test_trip_order_falls_back_to_odometer_when_created_at_ties() {
    // The imported rows: one datetime, one identical import timestamp. The
    // odometer is then the only surviving evidence of order.
    let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
    let stamp = Utc::now();
    let mut low = make_trip_at(date, 0, 0);
    low.created_at = stamp;
    low.odometer = 17416.0;
    let mut high = make_trip_at(date, 0, 0);
    high.created_at = stamp;
    high.odometer = 17618.0;

    assert_eq!(trip_order(&low, &high), std::cmp::Ordering::Less);
}

#[test]
fn test_trip_order_is_total() {
    // Same datetime, same created_at, same odometer: id decides, so the
    // result never depends on the DB row order or a stable-sort accident.
    let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
    let stamp = Utc::now();
    let mut a = make_trip_at(date, 0, 0);
    let mut b = make_trip_at(date, 0, 0);
    a.created_at = stamp;
    b.created_at = stamp;
    a.odometer = 100.0;
    b.odometer = 100.0;

    assert_ne!(trip_order(&a, &b), std::cmp::Ordering::Equal);
    assert_eq!(trip_order(&a, &b), a.id.cmp(&b.id));
}

#[test]
fn test_trip_numbers_and_odometer_start_agree_on_a_tied_group() {
    // The bug this task exists for: numbering said one order, the chain said
    // another, so Km pred did not equal the previous row's ODO.
    let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
    let stamp = Utc::now();
    let mut long_leg = make_trip_at(date, 0, 0);
    long_leg.created_at = stamp;
    long_leg.distance_km = 377.0;
    long_leg.odometer = 17416.0;
    let mut short_leg = make_trip_at(date, 0, 0);
    short_leg.created_at = stamp;
    short_leg.distance_km = 202.0;
    short_leg.odometer = 17618.0;

    // Hand them in the order that used to break it: short leg first.
    let trips = vec![short_leg.clone(), long_leg.clone()];
    let numbers = calculate_trip_numbers(&trips);
    let starts = calculate_odometer_start(&trips, 17039.0);

    assert_eq!(numbers.get(&long_leg.id.to_string()), Some(&1));
    assert_eq!(numbers.get(&short_leg.id.to_string()), Some(&2));
    assert_eq!(starts.get(&long_leg.id.to_string()), Some(&17039.0));
    assert_eq!(starts.get(&short_leg.id.to_string()), Some(&17416.0));
}
```

Add `trip_order` to the import list at the top of `commands_tests.rs`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core trip_order`
Expected: FAIL, `no 'trip_order' in 'commands_internal::helpers'`.

- [ ] **Step 3: Write the comparator**

Add to `helpers.rs`, above `calculate_trip_numbers`:

```rust
/// The one order the whole book uses.
///
/// Each key applies only when the one before it carries no information:
///
/// 1. `start_datetime` -- what the driver recorded.
/// 2. `created_at` -- real evidence when it differs. It is what separates the
///    two 2026-08-19 rows correctly, which the places confirm (task 79).
/// 3. `odometer` -- the only surviving evidence for rows imported under one
///    identical timestamp. The odometer only moves forward, so the lower
///    ending value came first.
/// 4. `id` -- makes the order total, so nothing depends on the DB row order
///    or on a stable-sort accident.
///
/// The odometer sits BELOW `created_at` on purpose. Ordering by the odometer
/// first makes the chain agree with itself by construction, which would
/// silence the span warnings that exist to test those very odometers.
pub fn trip_order(a: &Trip, b: &Trip) -> std::cmp::Ordering {
    a.start_datetime
        .cmp(&b.start_datetime)
        .then_with(|| a.created_at.cmp(&b.created_at))
        .then_with(|| {
            a.odometer
                .partial_cmp(&b.odometer)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| a.id.cmp(&b.id))
}
```

Then replace the three sort bodies in `helpers.rs` (`calculate_trip_numbers`, `calculate_odometer_start`, `generate_month_end_rows`) with:

```rust
    sorted.sort_by(|a, b| trip_order(a, b));
```

The old bodies compared `start_datetime.date()` before `start_datetime`, which is redundant: a date comparison is the leading part of a datetime comparison.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core trip_order`
Expected: PASS, 4 tests.

- [ ] **Step 5: Run the whole backend suite**

Run: `npm run test:backend`
Expected: PASS. If a test fails on a changed trip number, read it before changing it: the new number may be the correct one. Record any test you update and why.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/core/src/commands_internal/helpers.rs \
        src-tauri/core/src/commands_internal/commands_tests.rs
git commit -m "refactor(trips): one comparator decides trip order"
```

---

### Task 2: Every trip sort goes through the comparator

**Files:**
- Modify: `src-tauri/core/src/commands_internal/statistics.rs` at lines 66, 220, 281, 336, 435, 752, 1582
- Test: `src-tauri/core/src/commands_internal/commands_tests.rs`

**Interfaces:**
- Consumes: `trip_order` from Task 1.
- Produces: no new API. `build_trip_grid_data` returns an `odometer_start` map that always chains from `trip_numbers`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_grid_chain_reads_correctly_in_trip_number_order() {
    // Walking the rows in the order the grid displays them, every row's
    // start must equal the previous row's stored odometer. This held only
    // by accident before: numbering and the chain used different sorts.
    let date = NaiveDate::from_ymd_opt(2024, 1, 4).unwrap();
    let stamp = Utc::now();
    let mut a = make_trip_at(date, 0, 0);
    a.created_at = stamp;
    a.distance_km = 377.0;
    a.odometer = 17416.0;
    let mut b = make_trip_at(date, 0, 0);
    b.created_at = stamp;
    b.distance_km = 202.0;
    b.odometer = 17618.0;
    let mut c = make_trip_at(date.succ_opt().unwrap(), 8, 0);
    c.created_at = stamp;
    c.distance_km = 168.0;
    c.odometer = 17786.0;

    let trips = vec![b.clone(), c.clone(), a.clone()];
    let numbers = calculate_trip_numbers(&trips);
    let starts = calculate_odometer_start(&trips, 17039.0);

    let mut ordered: Vec<_> = trips.iter().collect();
    ordered.sort_by_key(|t| numbers[&t.id.to_string()]);

    let mut prev = 17039.0;
    for trip in ordered {
        assert_eq!(
            starts[&trip.id.to_string()], prev,
            "row {} starts where the previous row ended",
            numbers[&trip.id.to_string()]
        );
        prev = trip.odometer;
    }
}
```

- [ ] **Step 2: Run it to verify it passes already**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core grid_chain_reads`
Expected: PASS after Task 1. This test guards the invariant; it is not the driver for this task. Keep it.

- [ ] **Step 3: Replace the six `(date, odometer)` sorts**

At `statistics.rs` lines 66, 220, 281, 435, 752 and 1582 the body is:

```rust
    trips.sort_by(|a, b| {
        a.start_datetime
            .date()
            .cmp(&b.start_datetime.date())
            .then_with(|| {
                a.odometer
                    .partial_cmp(&b.odometer)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
```

Replace each with:

```rust
    trips.sort_by(|a, b| trip_order(a, b));
```

At line 435 the variable is `chronological`, built as `trips.clone()`. Keep the clone (the unsorted `trips` is still returned in the payload) and sort it with `trip_order`. At line 336, inside `get_year_start_odometer`, replace the `(date, datetime, created_at)` body with the same call, so the year carryover uses the same last row the grid shows.

Import `trip_order` at the top of `statistics.rs`:

```rust
use super::{calculate_odometer_start, calculate_trip_numbers, generate_month_end_rows, trip_order};
```

- [ ] **Step 4: Run the whole backend suite**

Run: `npm run test:backend`
Expected: PASS.

- [ ] **Step 5: Measure the change against a copy of the production book**

```bash
docker rm -f kniha-jazd-local 2>/dev/null
docker build -f Dockerfile.web -t kniha-jazd-web:local .
docker run -d --name kniha-jazd-local -p 3456:3456 -v "$PWD/_tmp/79-local/data:/data" kniha-jazd-web:local
sleep 4
for Y in 2023 2024 2025 2026; do
  curl -s -X POST http://127.0.0.1:3456/api/rpc -H 'Content-Type: application/json' \
    -d "{\"command\":\"get_trip_grid_data\",\"args\":{\"vehicleId\":\"c5c0b5d8-abaf-4e21-b6d4-fb289c26e854\",\"year\":$Y}}" \
    | python3 -c "
import json,sys
d=json.load(sys.stdin); nums=d['tripNumbers']; back=d['odometerStart']
ts=sorted(d['trips'], key=lambda t: nums[t['id']]); prev=d['yearStartOdometer']; bad=0
for t in ts:
    if abs(prev-back[t['id']])>0.001: bad+=1
    prev=t['odometer']
print('$Y chain breaks', bad, '| span warnings', len(d['odometerSpanWarnings']))
"
done
```

Expected: chain breaks **0** for every year. Span warnings **3** for 2026 (the task 79 rows). Any other year's span count is the honest state of that year's data, not a regression.

Stop if 2026 does not still report 3. That means the comparator is ordering by the odometer where it must not.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/core/src/commands_internal/statistics.rs \
        src-tauri/core/src/commands_internal/commands_tests.rs
git commit -m "refactor(trips): route every trip sort through trip_order"
```

---

### Task 3: The odometer rewrite moves to Rust

**Files:**
- Modify: `src-tauri/core/src/commands_internal/trips.rs`
- Modify: `src-tauri/core/src/server/dispatcher.rs:176` (add an arm beside `update_trip`)
- Modify: `src/lib/api.ts`, `src/lib/components/TripGrid.svelte:312, 353, 443-461`
- Test: `src-tauri/core/src/commands_internal/commands_tests.rs`

**Interfaces:**
- Consumes: `trip_order` from Task 1.
- Produces: `recalculate_odometers_internal(db, app_state, vehicle_id: String, year: i32) -> Result<usize, String>`, returning how many rows it changed. RPC command name `recalculate_odometers`, frontend `recalculateOdometers(vehicleId, year): Promise<number>`.

- [ ] **Step 1: Write the failing test**

```rust
/// A vehicle whose odometer starts at a known value. `setup_db_with_vehicle`
/// (commands_tests.rs:2618) hardcodes 0.0, and this task needs a real start.
fn setup_db_with_start_odometer(initial: f64) -> (Database, Vehicle) {
    let db = Database::in_memory().unwrap();
    let mut vehicle =
        Vehicle::new("Order Car".to_string(), "BA999XY".to_string(), 66.0, 5.1, initial);
    vehicle.initial_odometer = initial;
    db.create_vehicle(&vehicle).unwrap();
    (db, vehicle)
}

fn seed_chain_trip(db: &Database, vehicle_id: Uuid, day: u32, km: f64, odo: f64) -> Uuid {
    let date = NaiveDate::from_ymd_opt(2026, 3, day).unwrap();
    let mut trip = make_trip_detailed(date, km, None, false);
    trip.vehicle_id = vehicle_id;
    trip.odometer = odo;
    db.create_trip(&trip).unwrap();
    trip.id
}

#[test]
fn test_recalculate_odometers_rewrites_only_rows_that_move() {
    // The frontend loop rewrote a whole year on every save. The command
    // writes a row only when its value actually changes, and it walks the
    // canonical order rather than the DB order.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50999.0); // wrong
    let c = seed_chain_trip(&db, vehicle.id, 3, 30.0, 50150.0);

    let changed =
        recalculate_odometers_internal(&db, &app_state, vehicle.id.to_string(), 2026).unwrap();

    assert_eq!(changed, 2, "only b and c move");
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().odometer, 50050.0);
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().odometer, 50120.0);
    assert_eq!(db.get_trip(&c.to_string()).unwrap().unwrap().odometer, 50150.0);
}

#[test]
fn test_recalculate_odometers_is_read_only_guarded() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    app_state.set_read_only("newer migrations".to_string());

    let result = recalculate_odometers_internal(&db, &app_state, vehicle.id.to_string(), 2026);

    assert!(result.is_err(), "a write command must respect read-only mode");
}
```

`make_trip_detailed` (commands_tests.rs:808) and `make_trip_at` (added by task 79)
already exist. Check the exact name of the read-only setter on `AppState`
([app_state.rs](../../src-tauri/core/src/app_state.rs)) before writing the second
test; the constructor is `AppState::new()`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core recalculate_odometers`
Expected: FAIL, function not found.

- [ ] **Step 3: Write the command**

Add to `trips.rs`:

```rust
/// Rewrite the year's odometers so each row starts where the previous one
/// ended, walking `trip_order`. Returns the number of rows changed.
///
/// This replaces a loop that ran in the browser after every save, wrote rows
/// the user never touched, and walked the DB order rather than the canonical
/// one (task 80).
pub fn recalculate_odometers_internal(
    db: &Database,
    app_state: &AppState,
    vehicle_id: String,
    year: i32,
) -> Result<usize, String> {
    check_read_only!(app_state);

    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    let mut trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;
    trips.sort_by(|a, b| trip_order(a, b));

    let mut running = get_year_start_odometer(db, &vehicle_id, year, vehicle.initial_odometer)?;
    let mut changed = 0usize;

    for trip in trips.iter_mut() {
        running += trip.distance_km;
        if (trip.odometer - running).abs() > 0.001 {
            trip.odometer = running;
            trip.updated_at = Utc::now();
            db.update_trip(trip).map_err(|e| e.to_string())?;
            changed += 1;
        }
    }

    Ok(changed)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core recalculate_odometers`
Expected: PASS, 2 tests.

- [ ] **Step 5: Add the RPC arm**

In `dispatcher.rs`, beside the `"update_trip"` arm:

```rust
        "recalculate_odometers" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                year: i32,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::recalculate_odometers_internal(
                &state.db,
                &state.app_state,
                a.vehicle_id,
                a.year,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
```

Export `recalculate_odometers_internal` from `commands_internal/mod.rs` next to `update_trip_internal`.

- [ ] **Step 6: Replace the frontend loop**

In `src/lib/api.ts`, beside `updateTrip`:

```typescript
export async function recalculateOdometers(vehicleId: string, year: number): Promise<number> {
	return await apiCall('recalculate_odometers', { vehicleId, year });
}
```

In `TripGrid.svelte`, delete `recalculateAllOdo` (lines 443-461) and its two call sites at 312 and 353, replacing each with:

```svelte
			await recalculateOdometers(vehicleId, year);
```

Add `recalculateOdometers` to the `$lib/api` import on line 4.

- [ ] **Step 7: Rebuild and run the integration suite**

```bash
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
xvfb-run -a -s "-screen 0 1280x1024x24" npm run test:integration
```

Expected: 33 spec files pass. `km-odo-bidirectional.spec.ts` and `trip-management.spec.ts` are the ones this task can break.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/core/src/commands_internal/trips.rs \
        src-tauri/core/src/commands_internal/mod.rs \
        src-tauri/core/src/commands_internal/commands_tests.rs \
        src-tauri/core/src/server/dispatcher.rs \
        src/lib/api.ts src/lib/components/TripGrid.svelte
git commit -m "refactor(trips): recalculate odometers in Rust, not in the browser"
```

---

### Task 4: The editor reads the odometer, it does not compute it

**Files:**
- Modify: `src-tauri/core/src/models.rs:887-898` (`PreviewResult`)
- Modify: `src-tauri/core/src/commands_internal/statistics.rs` (the preview command around line 752)
- Modify: `src/lib/components/TripRow.svelte:218, 243, 321, 339-350, 415-425`
- Modify: `src/lib/components/TripGrid.svelte:717, 750, 822`
- Test: `src-tauri/core/src/commands_internal/commands_tests.rs`, `tests/integration/specs/tier1/km-odo-bidirectional.spec.ts`

**Interfaces:**
- Consumes: `trip_order` (Task 1), the `odometer_start` map from `build_trip_grid_data`.
- Produces: `PreviewResult` gains `odometer_start: f64` and `odometer: f64`. The frontend prop `previousOdometer` is deleted.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_preview_returns_the_row_odometer() {
    // The editor stops computing odo = previous + km. The preview command
    // already runs on every km change, so it returns the answer instead.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);

    let preview = preview_trip_calculation_internal(
        &db,
        vehicle.id.to_string(),
        2026,
        70,    // distance_km is i32 on this command
        None,  // fuel_liters
        false, // full_tank
        None,  // insert_at_trip_id
        None,  // editing_trip_id
    )
    .unwrap();

    assert_eq!(preview.odometer_start, 50050.0);
    assert_eq!(preview.odometer, 50120.0);
}
```

The signature is `preview_trip_calculation_internal(db, vehicle_id, year, distance_km: i32, fuel_liters, full_tank, insert_at_trip_id, editing_trip_id)` ([statistics.rs:1477](../../src-tauri/core/src/commands_internal/statistics.rs)). It takes no `AppState`, because it writes nothing.

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core preview_returns_the_row_odometer`
Expected: FAIL, `no field 'odometer_start' on type 'PreviewResult'`.

- [ ] **Step 3: Extend `PreviewResult` and the command**

In `models.rs`:

```rust
    /// True if rate is estimated (no full-tank fill-up yet in this period)
    pub is_estimated_rate: bool,
    /// Odometer this row starts from, derived from the canonical order
    pub odometer_start: f64,
    /// Odometer this row ends at: `odometer_start + km` (task 80: the editor
    /// must not do this arithmetic itself, see ADR-008)
    pub odometer: f64,
}
```

In the preview command, after the trips are sorted with `trip_order`, compute the anchor the same way `calculate_odometer_start` does and fill both fields. Every existing construction of `PreviewResult`, including test fixtures, needs the two new fields; the compiler lists them.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `npm run test:backend`
Expected: PASS.

- [ ] **Step 5: Delete the frontend arithmetic**

In `TripRow.svelte`:
- Replace `formData.odometer = previousOdometer + roundedKm;` (line 218) and `formData.odometer = previousOdometer + (formData.distanceKm ?? 0);` (line 243) and `formData.odometer = previousOdometer + km;` (line 321) with assignments from `previewData.odometer`, applied when the preview returns and `manualOdoEdit` is false.
- Replace the clamp in `handleOdoBlur` (lines 339-350) and the belt-and-braces clamp in `handleSave` (lines 415-425) with a comparison against `previewData.odometerStart`.
- Delete the `previousOdometer` prop (line 17) and its three bindings in `TripGrid.svelte` (717, 750, 822).

Keep the two-way behaviour: typing km fills ODO, typing ODO fills km. The difference is that the anchor now comes from the backend rather than from a display neighbour.

- [ ] **Step 6: Extend the integration spec**

Add to `tests/integration/specs/tier1/km-odo-bidirectional.spec.ts` a case that edits a row inside a tied group and asserts the ODO the editor offers equals the Km pred the grid shows for that row. That is the exact discrepancy this task removes: 34 rows in 2023, 36 in 2024, 19 in 2025.

- [ ] **Step 7: Rebuild and run the integration suite**

```bash
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
xvfb-run -a -s "-screen 0 1280x1024x24" npm run test:integration
```

Expected: 33 spec files pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/core/src/models.rs \
        src-tauri/core/src/commands_internal/statistics.rs \
        src-tauri/core/src/commands_internal/commands_tests.rs \
        src/lib/components/TripRow.svelte src/lib/components/TripGrid.svelte \
        tests/integration/specs/tier1/km-odo-bidirectional.spec.ts
git commit -m "refactor(trips): the row editor reads the odometer from the backend"
```

---

### Task 5: Verify against the real book, then document

**Files:**
- Modify: `CHANGELOG.md`, `DECISIONS.md`, `_tasks/index.md`
- Modify: `_tasks/79-odometer-span-inconsistency/01-task.md` (cross-link)
- Create: `_tasks/80-one-trip-ordering/03-status.md`

- [ ] **Step 1: Re-measure the whole book**

Run the Step 5 script from Task 2 again, on a **fresh** copy of the production database:

```bash
scp root@192.168.0.112:'~/kniha-jazd/data/kniha-jazd.db' _tmp/80-verify/data/kniha-jazd.db
```

Record, for each year: chain breaks, span warnings, and the trip numbers that changed. Expected: 0 chain breaks everywhere, 3 span warnings in 2026.

- [ ] **Step 2: Write down the renumbering**

In `03-status.md`, list every trip whose number changed, with its date and route. This is the legal "Poradové číslo jazdy" column. The user approves this list before anything runs against the production book.

- [ ] **Step 3: Add the ADR**

One entry in `DECISIONS.md`, newest first, numbered after ADR-043: the comparator, the four keys, and the measured reason the odometer sits below `created_at` (65 warnings vs 1 vs 4, and the one group of 31 where the two disagree, settled by the place chain 2 links to 0).

- [ ] **Step 4: Changelog**

A Slovak entry under `### Opravené`: the Km pred column now always follows the previous row, and the odometer recalculation happens in the backend.

- [ ] **Step 5: Update the task index and cross-link task 79**

Move task 80 to ✅ in `_tasks/index.md` and note in task 79 that its open tie-break question is answered here.

- [ ] **Step 6: Commit**

```bash
git add CHANGELOG.md DECISIONS.md _tasks/index.md \
        _tasks/79-odometer-span-inconsistency/01-task.md \
        _tasks/80-one-trip-ordering/03-status.md
git commit -m "docs: record the trip ordering decision and its effect on the book"
```

---

## Notes for the executor

- **Do not "fix" a span warning by changing the order.** If a row still warns after Task 2, that is the data, and it belongs to [task 79](../79-odometer-span-inconsistency/).
- The three 2026 rows are wrong in the production book and are not corrected by this plan.
- `_tmp/` is gitignored. Database copies stay there.
