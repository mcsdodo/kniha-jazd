**Date:** 2026-05-21
**Subject:** Datetime Is Order — Implementation Plan
**Status:** Planning

# Datetime Is Order Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use [superpowers:executing-plans](../../CLAUDE.md) to implement this plan task-by-task.

**Goal:** Make `start_datetime` the single source of truth for trip order; drop the `sort_order` column and all manual-reorder UI/API/logic.

**Architecture:** Land changes in a safe order — switch all read queries to `ORDER BY start_datetime DESC, created_at ASC` first (column still present, so tests keep working); then remove writes that mutate `sort_order` (reorder command, insert-at-position); then drop the column + Rust field; then strip the frontend UI; finally an integration test reproduces the original bug scenario and confirms no red rows appear.

**Tech Stack:** Rust (Tauri backend, Diesel/SQLite), TypeScript + Svelte 5, WebdriverIO integration tests.

**Task Reference:** [01-task.md](./01-task.md) — full problem statement, acceptance criteria, surface area.

---

## Task 1: Switch read queries to datetime ordering

Change [db.rs](../../src-tauri/core/src/db.rs) so trips are ordered by `start_datetime DESC, created_at ASC` instead of `sort_order ASC`. Column stays in the table — only the `ORDER BY` clause changes.

**Files:**
- Modify: [src-tauri/core/src/db.rs](../../src-tauri/core/src/db.rs) — `get_trips_for_vehicle`, `get_trips_for_vehicle_in_year`
- Modify: [src-tauri/core/src/db_tests.rs](../../src-tauri/core/src/db_tests.rs) — add new test

**Step 1: Write failing test for chronological ordering**

Add to [db_tests.rs](../../src-tauri/core/src/db_tests.rs):

```rust
#[test]
fn test_get_trips_for_vehicle_returns_chronological_order() {
    let (db, vehicle) = setup_with_vehicle();
    let v_id = vehicle.id.to_string();

    // Insert in arbitrary order, with sort_order intentionally MISMATCHED with dates
    let mut trip_old = Trip::test_ice_trip(NaiveDate::from_ymd_opt(2026, 5, 5).unwrap(), 10.0, None, true);
    trip_old.vehicle_id = vehicle.id;
    trip_old.sort_order = 0; // Lowest sort_order but oldest date

    let mut trip_new = Trip::test_ice_trip(NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(), 10.0, None, true);
    trip_new.vehicle_id = vehicle.id;
    trip_new.sort_order = 2; // Highest sort_order but newest date

    let mut trip_mid = Trip::test_ice_trip(NaiveDate::from_ymd_opt(2026, 5, 12).unwrap(), 10.0, None, true);
    trip_mid.vehicle_id = vehicle.id;
    trip_mid.sort_order = 1;

    db.create_trip(&trip_old).unwrap();
    db.create_trip(&trip_new).unwrap();
    db.create_trip(&trip_mid).unwrap();

    let trips = db.get_trips_for_vehicle(&v_id).unwrap();
    assert_eq!(trips.len(), 3);
    assert_eq!(trips[0].start_datetime.date(), NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(), "newest first");
    assert_eq!(trips[1].start_datetime.date(), NaiveDate::from_ymd_opt(2026, 5, 12).unwrap(), "middle second");
    assert_eq!(trips[2].start_datetime.date(), NaiveDate::from_ymd_opt(2026, 5, 5).unwrap(), "oldest last");
}
```

**Step 2: Run test, verify failure**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "test_get_trips_for_vehicle_returns_chronological_order"
```
Expected: FAIL (current ORDER BY sort_order returns oldest first because sort_order 0 = old here).

**Step 3: Update queries in [db.rs](../../src-tauri/core/src/db.rs)**

Replace `.order(trips::sort_order.asc())` with `.order((trips::start_datetime.desc(), trips::created_at.asc()))` in both [get_trips_for_vehicle](../../src-tauri/core/src/db.rs) and [get_trips_for_vehicle_in_year](../../src-tauri/core/src/db.rs).

**Step 4: Verify test passes + full backend suite still passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```
Expected: All 195+ tests pass. Some pre-existing tests may need updates if they depended on sort_order ordering — fix them by removing sort_order assumptions.

**Step 5: Commit**

```bash
git add src-tauri/core/src/db.rs src-tauri/core/src/db_tests.rs
git commit -m "refactor(trips): order by start_datetime DESC instead of sort_order"
```

---

## Task 2: Drop `insert_at_position` from `create_trip`

Remove the bug source: stop letting callers choose `sort_order` for new trips. The field still exists, just defaults to `0` for all new rows.

**Files:**
- Modify: [src-tauri/core/src/commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) — drop `insert_at_position` param + shift logic
- Modify: [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) — drop `insert_at_position` from `create_trip` dispatch arm
- Modify: [src/lib/api.ts](../../src/lib/api.ts) — drop `insertAtPosition` from `createTrip` signature
- Modify: [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte) — drop `insertAtSortOrder` arg from `createTrip` call
- Modify: [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) — update any tests that pass `Some(position)` to `create_trip_internal`

**Step 1: Write test verifying new trips get correct order regardless of creation sequence**

Add to [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs):

```rust
#[test]
fn test_create_trip_orders_by_date_regardless_of_creation_order() {
    let (db, app_state, vehicle) = setup_with_vehicle_and_state();
    // Create in non-chronological order
    create_trip_internal(&db, &app_state, vehicle.id.to_string(),
        "2026-05-21T09:00:00".into(), "2026-05-21T09:30:00".into(),
        "A".into(), "B".into(), 10.0, 10000.0, "test".into(),
        None, None, None, None, None, None, None, None, None).unwrap();
    create_trip_internal(&db, &app_state, vehicle.id.to_string(),
        "2026-05-18T04:30:00".into(), "2026-05-18T08:30:00".into(),
        "A".into(), "B".into(), 370.0, 10370.0, "test".into(),
        None, None, None, None, None, None, None, None, None).unwrap();
    create_trip_internal(&db, &app_state, vehicle.id.to_string(),
        "2026-05-20T16:00:00".into(), "2026-05-20T19:00:00".into(),
        "A".into(), "B".into(), 370.0, 10740.0, "test".into(),
        None, None, None, None, None, None, None, None, None).unwrap();

    let trips = db.get_trips_for_vehicle(&vehicle.id.to_string()).unwrap();
    assert_eq!(trips[0].start_datetime.date(), NaiveDate::from_ymd_opt(2026, 5, 21).unwrap());
    assert_eq!(trips[1].start_datetime.date(), NaiveDate::from_ymd_opt(2026, 5, 20).unwrap());
    assert_eq!(trips[2].start_datetime.date(), NaiveDate::from_ymd_opt(2026, 5, 18).unwrap());
}
```

**Step 2: Run test, verify failure**

(Compilation fails because `create_trip_internal` still requires `insert_at_position` argument — that IS the failure.)

**Step 3: Simplify [create_trip_internal](../../src-tauri/core/src/commands_internal/trips.rs)**

Drop the `insert_at_position` parameter and the `shift_trips_from_position` call. Set `sort_order: 0` unconditionally — its value no longer matters (we never order by it after Task 1, and we'll drop the column in Task 6).

**Step 4: Drop `insertAtPosition` from [server dispatcher](../../src-tauri/core/src/server/dispatcher.rs)**

Remove the field from the `create_trip` `Args` struct.

**Step 5: Drop `insertAtPosition` from [api.ts](../../src/lib/api.ts) `createTrip`**

Remove the parameter and the field passed in `apiCall('create_trip', ...)`.

**Step 6: Drop `insertAtSortOrder` arg from [TripGrid.svelte](../../src/lib/components/TripGrid.svelte) `handleSaveNew`**

Remove the last positional argument to `createTrip(...)`.

**Step 7: Run backend tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```
Expected: pass.

**Step 8: Commit**

```bash
git add src-tauri/core/src/commands_internal/trips.rs src-tauri/core/src/server/dispatcher.rs src/lib/api.ts src/lib/components/TripGrid.svelte src-tauri/core/src/commands_internal/commands_tests.rs
git commit -m "refactor(trips): drop insert_at_position from create_trip"
```

---

## Task 3: Remove `reorder_trip` command and DB method

The arrows are gone (or will be in Task 8); the command has no callers worth preserving.

**Files:**
- Modify: [src-tauri/core/src/commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) — delete `reorder_trip_internal`
- Modify: [src-tauri/core/src/db.rs](../../src-tauri/core/src/db.rs) — delete `reorder_trip` + `shift_trips_from_position`
- Modify: [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) — delete dispatch arm
- Modify: [src/lib/api.ts](../../src/lib/api.ts) — delete `reorderTrip` export
- Modify: [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) — delete tests for `reorder_trip_internal`

**Step 1: Delete `reorder_trip_internal` and its tests**

Grep for any test using it, delete those tests.

**Step 2: Delete `reorder_trip` + `shift_trips_from_position` from [db.rs](../../src-tauri/core/src/db.rs)**

**Step 3: Delete the `reorder_trip` arm from the [server dispatcher](../../src-tauri/core/src/server/dispatcher.rs)**

**Step 4: Delete `reorderTrip` from [api.ts](../../src/lib/api.ts)**

**Step 5: Run backend tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```
Expected: pass.

**Step 6: Commit**

```bash
git commit -m "refactor(trips): remove reorder_trip command and infrastructure"
```

---

## Task 4: Remove `calculate_date_warnings`

The function detected drift that can no longer happen. Remove the function, its tests, and the `date_warnings` field flowing through `TripGridData`.

**Files:**
- Modify: [src-tauri/core/src/commands_internal/statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs) — delete `calculate_date_warnings`, drop `date_warnings` calculation from [get_trip_grid_data](../../src-tauri/core/src/commands_internal/statistics.rs)
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) — drop `date_warnings` from `TripGridData` struct
- Modify: [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) — delete `test_date_warnings_*` tests
- Modify: [src/lib/types.ts](../../src/lib/types.ts) — drop `dateWarnings` field from `TripGridData`
- Modify: [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte) — drop `dateWarnings` Set and reference

**Step 1: Delete `calculate_date_warnings` and `test_date_warnings_*` tests**

Find and delete both `test_date_warnings_detects_out_of_order` and `test_date_warnings_correct_order_no_warnings` in [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs).

**Step 2: Drop `date_warnings` from [TripGridData](../../src-tauri/core/src/models.rs) struct**

**Step 3: Drop `date_warnings` HashMap building in [get_trip_grid_data](../../src-tauri/core/src/commands_internal/statistics.rs)**

Also remove the `HashSet::new()` initializer where TripGridData is built without trips.

**Step 4: Drop `dateWarnings` from frontend [types.ts](../../src/lib/types.ts) and [TripGrid.svelte](../../src/lib/components/TripGrid.svelte)**

In TripGrid.svelte, also remove the `hasDateWarning={dateWarnings.has(trip.id)}` prop on `<TripRow>`.

**Step 5: Run backend tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```
Expected: pass (after deleting the date-warning tests).

**Step 6: Commit**

```bash
git commit -m "refactor(trips): remove calculate_date_warnings (drift no longer possible)"
```

---

## Task 5: Update `calculate_trip_numbers` tiebreaker

Replace the `sort_order DESC` tiebreaker in [calculate_trip_numbers](../../src-tauri/core/src/commands_internal/helpers.rs) with `created_at ASC` (same trip date+time, older creation = earlier trip number).

**Files:**
- Modify: [src-tauri/core/src/commands_internal/helpers.rs](../../src-tauri/core/src/commands_internal/helpers.rs) — change tiebreaker in `calculate_trip_numbers` and `calculate_odometer_start`
- Modify: [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) — update [test_trip_numbers_same_datetime_by_sort_order](../../src-tauri/core/src/commands_internal/commands_tests.rs)

**Step 1: Write failing test for new tiebreaker**

Update the existing `test_trip_numbers_same_datetime_by_sort_order` to:

```rust
#[test]
fn test_trip_numbers_same_datetime_tiebroken_by_created_at() {
    // Two trips at identical start_datetime, different created_at
    let dt = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap().and_hms_opt(8, 0, 0).unwrap();
    let earlier_created = Utc::now() - Duration::seconds(60);
    let later_created = Utc::now();

    let trip_a = Trip { /* ...id A, created_at: earlier_created, sort_order: 5...*/ };
    let trip_b = Trip { /* ...id B, created_at: later_created, sort_order: 0...*/ };

    let nums = calculate_trip_numbers(&[trip_a.clone(), trip_b.clone()]);
    assert_eq!(nums.get(&trip_a.id.to_string()), Some(&1), "earlier created_at gets #1");
    assert_eq!(nums.get(&trip_b.id.to_string()), Some(&2));
}
```

**Step 2: Run test, verify failure**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "test_trip_numbers_same_datetime"
```

**Step 3: Update [calculate_trip_numbers](../../src-tauri/core/src/commands_internal/helpers.rs) tiebreaker**

Change `b.sort_order.cmp(&a.sort_order)` to `a.created_at.cmp(&b.created_at)`. Apply the same change in [calculate_odometer_start](../../src-tauri/core/src/commands_internal/helpers.rs).

**Step 4: Run tests, verify pass**

**Step 5: Commit**

```bash
git commit -m "refactor(trips): use created_at as same-datetime tiebreaker"
```

---

## Task 6: Migration + drop `sort_order` field

Now that no code reads or writes `sort_order` meaningfully, drop it from the schema and Rust types.

**Files:**
- Create: [src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/up.sql](../../src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/up.sql)
- Create: [src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/down.sql](../../src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/down.sql)
- Modify: [src-tauri/core/src/schema.rs](../../src-tauri/core/src/schema.rs) — drop `sort_order` line from `trips` table macro
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) — drop `sort_order` from `Trip`, `TripRow`, `NewTrip`, and `From` impls
- Modify: [src-tauri/core/src/db_tests.rs](../../src-tauri/core/src/db_tests.rs), [invoice_tests.rs](../../src-tauri/core/src/invoice_tests.rs), [export_tests.rs](../../src-tauri/core/src/export_tests.rs), [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs), [calculations/tests.rs](../../src-tauri/core/src/calculations/tests.rs) — remove `sort_order: 0,` lines from fixtures
- Modify: [src/lib/types.ts](../../src/lib/types.ts) — drop `sortOrder` from `Trip`
- Modify: [src/lib/constants.ts](../../src/lib/constants.ts) — drop `sortOrder` references
- Modify: [src/routes/+page.svelte](../../src/routes/+page.svelte), [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte), [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte) — remove `sortOrder` references
- Modify: [tests/integration/utils/db.ts](../../tests/integration/utils/db.ts), [tests/integration/fixtures/trips.ts](../../tests/integration/fixtures/trips.ts), [tests/integration/fixtures/types.ts](../../tests/integration/fixtures/types.ts) — drop `sort_order` from fixtures

**Step 1: Create the migration**

[up.sql](../../src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/up.sql):
```sql
ALTER TABLE trips DROP COLUMN sort_order;
```

[down.sql](../../src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/down.sql):
```sql
ALTER TABLE trips ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
```

(SQLite 3.35+ supports `DROP COLUMN`. Tauri ships a recent enough version.)

**Step 2: Drop `sort_order` from [schema.rs](../../src-tauri/core/src/schema.rs)**

Remove the `sort_order -> Integer,` line from the `trips` table macro.

**Step 3: Drop `sort_order` from [Trip and related structs](../../src-tauri/core/src/models.rs)**

Remove the field from `Trip`, `TripRow`, `NewTrip`, and from the `From<TripRow> for Trip` + reverse impls. Also drop `sort_order: 0,` from `Trip::test_ice_trip`.

**Step 4: Compile and fix every remaining reference**

```bash
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```

Walk through each compile error and remove `sort_order` references. Most are test fixtures — see file list above. After all errors fixed:

**Step 5: Run full backend suite**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```
Expected: all pass.

**Step 6: Frontend type cleanup**

Drop `sortOrder` from `Trip` in [types.ts](../../src/lib/types.ts), drop any references in [constants.ts](../../src/lib/constants.ts), [+page.svelte](../../src/routes/+page.svelte), [TripGrid.svelte](../../src/lib/components/TripGrid.svelte), [TripRow.svelte](../../src/lib/components/TripRow.svelte).

**Step 7: Frontend typecheck**

```bash
npm run check
```
Expected: no errors.

**Step 8: Integration test fixture cleanup**

Drop `sort_order` from [tests/integration/utils/db.ts](../../tests/integration/utils/db.ts), [tests/integration/fixtures/trips.ts](../../tests/integration/fixtures/trips.ts), [tests/integration/fixtures/types.ts](../../tests/integration/fixtures/types.ts).

**Step 9: Commit**

```bash
git commit -m "feat(db): drop trips.sort_order column"
```

---

## Task 7: Remove manual sort mode from grid

Single chronological view only.

**Files:**
- Modify: [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte) — drop `SortColumn` type, `sortColumn` prop, sort toggling logic, `reorderDisabled`, `handleMoveUp`, `handleMoveDown`, `sortedTrips` simplification

**Step 1: Remove the `SortColumn` type and `sortColumn` prop**

Replace conditional sort logic with single sort: `[...trips].sort((a, b) => b.startDatetime.localeCompare(a.startDatetime))`.

**Step 2: Remove `handleMoveUp` and `handleMoveDown` functions**

Delete the functions (lines 365–399) and the `reorderTrip` import.

**Step 3: Remove `reorderDisabled` reactive statement and its consumers**

**Step 4: Simplify `handleInsertAbove`**

Keep only the date pre-fill — drop `insertAtSortOrder` state. Replace the variable with a `newRowInsertedNear: Trip | null` ref used purely for placing the form row in the template.

**Step 5: Update template — drop sort column toggle UI from `<th>` headers**

**Step 6: Frontend typecheck + dev smoke**

```bash
npm run check
npm run tauri:dev
```
Open the app, navigate to the trips grid, verify it renders and is sorted by date.

**Step 7: Commit**

```bash
git commit -m "refactor(trips): drop manual sort mode from grid"
```

---

## Task 8: Remove up/down arrows from row

**Files:**
- Modify: [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte) — drop arrow buttons + props + CSS
- Modify: [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte) — drop `canMoveUp` / `canMoveDown` / `onMoveUp` / `onMoveDown` props passed to `<TripRow>`

**Step 1: Drop `onMoveUp`, `onMoveDown`, `canMoveUp`, `canMoveDown` props from [TripRow.svelte](../../src/lib/components/TripRow.svelte)**

**Step 2: Drop the two `<button>` elements with `on:click={onMoveUp}` / `on:click={onMoveDown}` from the template (around lines 720–740)**

**Step 3: Drop `hasDateWarning` prop and `tr.date-warning` + `tr.date-warning:hover` + `tr.date-warning.consumption-warning` CSS rules**

**Step 4: Drop the props from the `<TripRow>` usage in [TripGrid.svelte](../../src/lib/components/TripGrid.svelte)**

**Step 5: Drop `errorMoveTrip` from i18n if still referenced**

```bash
grep -r errorMoveTrip src/
```
If only in i18n files, remove. If still consumed somewhere, leave that for Task 9.

**Step 6: Frontend typecheck + dev smoke**

```bash
npm run check && npm run tauri:dev
```
Verify rows render with no arrow buttons and no red date-warning rows.

**Step 7: Commit**

```bash
git commit -m "refactor(trips): drop up/down arrows and date-warning row styling"
```

---

## Task 9: i18n cleanup

Remove all i18n keys that no longer have consumers.

**Files:**
- Modify: [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts)
- Modify: [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts)
- Modify: [src/lib/i18n/i18n-types.ts](../../src/lib/i18n/i18n-types.ts) (auto-generated — regen or hand-edit)

**Step 1: Find obsolete keys**

```bash
grep -rE "trips\.(moveUp|moveDown|legend\.dateWarning)|toast\.errorMoveTrip" src/
```

**Step 2: Remove matching entries from sk/en `index.ts`**

**Step 3: Regenerate or hand-edit [i18n-types.ts](../../src/lib/i18n/i18n-types.ts)**

If typesafe-i18n watch is running, the file regenerates automatically. Otherwise:

```bash
npx typesafe-i18n --no-watch
```

**Step 4: Frontend typecheck**

```bash
npm run check
```

**Step 5: Commit**

```bash
git commit -m "chore(i18n): remove obsolete reorder/date-warning strings"
```

---

## Task 10: Integration test reproducing the user's scenario

Lock in the fix with an integration test that creates trips out of date order via the `+` button and verifies no red rows + correct date display.

**Files:**
- Create: [tests/integration/specs/tier2/datetime-is-order.spec.ts](../../tests/integration/specs/tier2/datetime-is-order.spec.ts)

**Step 1: Write the spec**

```typescript
import { browser } from '@wdio/globals';
import { resetDb, seedVehicle } from '../../utils/db';
import { TripGrid } from '../../pages/trip-grid';

describe('Datetime is the only order', () => {
    beforeEach(async () => {
        await resetDb();
        await seedVehicle({ tpConsumption: 5.10 });
    });

    it('orders trips by date regardless of creation order, no red rows', async () => {
        const grid = new TripGrid();
        await grid.open();

        // Create #66 (21.05), then #64 (18.05), then #65 (20.05) via "+" button
        await grid.addTripViaNewButton({ startDate: '2026-05-21', startTime: '09:00', endTime: '09:30', distanceKm: 4, purpose: 'navsteva banky' });
        await grid.addTripViaPlusOnRow(0, { startDate: '2026-05-18', startTime: '04:30', endTime: '08:30', distanceKm: 370, purpose: 'rokovanie' });
        await grid.addTripViaPlusOnRow(0, { startDate: '2026-05-20', startTime: '16:00', endTime: '19:00', distanceKm: 370, purpose: 'navrat' });

        // Verify date ordering (newest at top)
        const dates = await grid.getVisibleDates();
        expect(dates.slice(0, 3)).toEqual(['21.05.', '20.05.', '18.05.']);

        // Verify no red rows
        const redRowCount = await grid.countRowsWithClass('date-warning');
        expect(redRowCount).toBe(0);
    });
});
```

(Helper methods on `TripGrid` page object may need adding — check [tests/integration/pages/](../../tests/integration/pages/) for existing patterns.)

**Step 2: Run the spec**

```bash
npm run test:integration:build
npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/datetime-is-order.spec.ts
```
Expected: pass.

**Step 3: Commit**

```bash
git commit -m "test(integration): repro datetime-is-order bug scenario"
```

---

## Task 11: Final verification + docs

**Files:**
- Modify: [CHANGELOG.md](../../CHANGELOG.md) — add entry under `[Unreleased]`
- Modify: [DECISIONS.md](../../DECISIONS.md) — add ADR for "datetime is the only order"

**Step 1: Run full backend test suite**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```
Expected: all 195+ tests pass (minus removed `test_date_warnings_*` and `reorder` tests).

**Step 2: Run full integration test suite**

```bash
npm run test:integration:build
npm run test:integration
```
Expected: all pass.

**Step 3: Manual smoke test in dev mode**

```bash
npm run tauri:dev
```
- Open app, navigate to trips grid → renders correctly, no red rows on the user's existing data
- Click "Nový záznam" → can add a trip
- Click "+" on a row → can add a trip with the clicked row's date pre-filled
- Edit a trip's date → trip moves to correct chronological position on save

**Step 4: Update [CHANGELOG.md](../../CHANGELOG.md)**

Add under `[Unreleased]`:
```markdown
### Changed
- Trip order is now derived purely from `start_datetime` — manual reordering removed. New trips appear at the correct chronological position automatically.

### Fixed
- "Date warning" red rows no longer appear for trips created out of chronological order via the `+` button.
```

**Step 5: Run `/decision` skill to record ADR**

Use [.claude/skills/decision-skill](../../.claude/skills/decision-skill) to add an ADR entry to [DECISIONS.md](../../DECISIONS.md) recording: "ADR-NNN: Datetime is the only source of trip order. Manual sort_order removed."

**Step 6: Final commit**

```bash
git commit -m "docs: add changelog + ADR for datetime-is-order"
```

**Step 7: Move task folder to `_done/` and update [index.md](../index.md)**

```bash
git mv _tasks/65-datetime-is-order _tasks/_done/65-datetime-is-order
```
Update [_tasks/index.md](../index.md): move row from Active to Completed.

```bash
git add _tasks/index.md
git commit -m "docs: archive task 65 (datetime-is-order) as complete"
```

---

## Verification Checklist (final gate)

- [ ] `cargo test -p kniha-jazd-core` passes
- [ ] `npm run check` passes
- [ ] `npm run test:integration` passes
- [ ] Manual smoke: existing user data renders without red rows
- [ ] Manual smoke: `+` button on any row creates a trip in the correct chronological position
- [ ] Manual smoke: editing a trip's datetime moves it to the correct position
- [ ] [CHANGELOG.md](../../CHANGELOG.md) updated
- [ ] [DECISIONS.md](../../DECISIONS.md) has new ADR
- [ ] No `sort_order` references in `git grep sort_order src-tauri/ src/ tests/`
- [ ] No `reorderTrip` / `reorder_trip` references in `git grep`
- [ ] No `calculate_date_warnings` references in `git grep`
- [ ] No `dateWarning` references in `git grep`

---

## Rollback plan

The migration's [down.sql](../../src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/down.sql) re-adds the column with default `0`. If a rollback is needed mid-rollout, restore the previous commit and run `diesel migration revert` — but note that manual sort_order data is permanently lost (which is fine: no code depended on it after Task 1).
