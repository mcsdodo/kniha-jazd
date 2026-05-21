**Date:** 2026-05-21
**Subject:** Datetime Is Order — Implementation Plan
**Status:** Planning

# Datetime Is Order Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use [superpowers:executing-plans](../../CLAUDE.md) to implement this plan task-by-task.

**Goal:** Make `start_datetime` the single source of truth for trip order; drop the `sort_order` column and all manual-reorder UI/API/logic.

**Architecture:** Outside-in TDD. Phase 1 writes all the failing tests — end-to-end integration specs (real UI clicks) and backend unit tests — that describe the target behavior. Phase 2 makes them green in safe order: switch reads to date ordering first (column still present), then drop writes that mutate `sort_order`, then drop the column + Rust field, then strip the frontend UI. Phase 3 verifies the whole stack.

**Tech Stack:** Rust (Tauri backend, Diesel/SQLite), TypeScript + Svelte 5, WebdriverIO integration tests.

**Task Reference:** [01-task.md](./01-task.md) — full problem statement, acceptance criteria, surface area.

**Test discipline:**
- **Every backend behavior change has a failing unit test FIRST.** No exceptions.
- **Every user-visible behavior has a failing E2E spec FIRST.** No exceptions.
- **Removal tasks are tested by verifying the removed thing is gone** (selector returns no element, API call fails, etc.).
- **No "implementation-only" commits.** A commit either adds a test, makes one pass, or removes dead code that no longer has callers.

---

# Phase 1 — Write all failing tests first

## Task 1: Write end-to-end integration spec (all scenarios failing)

Create one E2E spec covering every user-visible scenario this task changes. All scenarios will be failing or skipped initially — they describe the target state.

**Files:**
- Create: [tests/integration/specs/tier2/datetime-is-order.spec.ts](../../tests/integration/specs/tier2/datetime-is-order.spec.ts)

**Step 1: Write the spec skeleton**

Follow the pattern from [tests/integration/specs/tier2/date-prefill.spec.ts](../../tests/integration/specs/tier2/date-prefill.spec.ts) — `waitForAppReady`, `seedVehicle` via IPC, then UI clicks. Cover these scenarios:

```typescript
import { waitForAppReady, navigateTo } from '../../utils/app';
import { waitForTripGrid } from '../../utils/assertions';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, setActiveVehicle } from '../../utils/db';

describe('Tier 2: Datetime is the only order', () => {
  let vehicleId: string;

  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
    const v = await seedVehicle({
      name: 'Test Vehicle', licensePlate: 'TEST001',
      initialOdometer: 50000, tankSizeLiters: 50, tpConsumption: 6.5,
    });
    vehicleId = v.id;
    await setActiveVehicle(vehicleId);
    await navigateTo('trips');
    await waitForTripGrid();
  });

  it('Scenario 1: creating trips out of date order via "+" gives correct chronological grid, no red rows', async () => {
    // Click "Nový záznam" → fill trip for 21.05 → save
    // Click "+" on the 21.05 row → fill trip for 18.05 → save
    // Click "+" on the 18.05 row → fill trip for 20.05 → save
    // Assert visible date column reads: 21.05, 20.05, 18.05 (newest first)
    // Assert: $$('tr.date-warning') has length 0
  });

  it('Scenario 2: editing a trip\'s start datetime moves it to the new chronological position', async () => {
    // Seed 3 trips (5.05, 12.05, 21.05). Open the 12.05 row in edit mode.
    // Change start_datetime to 25.05. Save.
    // Assert visible order is now: 25.05 (was 12.05), 21.05, 5.05
  });

  it('Scenario 3: up/down reorder arrows do not exist on any row', async () => {
    // Seed 3 trips. Assert: $$('button[data-action="move-up"]') and 'move-down' return empty arrays.
    // (Or whatever selector the arrow buttons use today.)
  });

  it('Scenario 4: manual sort mode toggle does not exist in the grid header', async () => {
    // Assert: column headers do not contain a "manual" sort option.
    // The "#" / trip-number column header may still be clickable for asc/desc, but there\'s no "manual" mode.
  });

  it('Scenario 5: deleting a middle trip preserves chronological order', async () => {
    // Seed 3 trips. Click delete on the middle one. Confirm.
    // Assert remaining rows show only newest + oldest dates, in correct order.
  });

  it('Scenario 6: red "date-warning" rows are impossible — even with deliberately out-of-order creation', async () => {
    // Same as Scenario 1, plus assertions that no tr has class containing 'date-warning'.
    // This is the regression guard for the original bug.
  });
});
```

**Step 2: Build the debug Tauri app**

```bash
npm run test:integration:build
```
Expected: build succeeds.

**Step 3: Run the new spec — observe failures**

```bash
npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/datetime-is-order.spec.ts
```
Expected: Scenarios 1, 2, 5, 6 may fail or error (functionality changes pending). Scenarios 3, 4 should *fail* — the arrows and manual-sort toggle still exist today. **This failing state is the goal of Phase 1.**

**Step 4: Commit the failing spec**

```bash
git add tests/integration/specs/tier2/datetime-is-order.spec.ts
git commit -m "test(integration): add failing spec for datetime-is-order (task 65)"
```

---

## Task 2: Write failing backend unit tests

Add the unit tests that describe the new ordering contract before changing any production code.

**Files:**
- Modify: [src-tauri/core/src/db_tests.rs](../../src-tauri/core/src/db_tests.rs) — add chronological ordering test
- Modify: [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) — add create-trip-out-of-order test + same-datetime-tiebreaker test

**Step 1: Add `test_get_trips_for_vehicle_returns_chronological_order`**

In [db_tests.rs](../../src-tauri/core/src/db_tests.rs):

```rust
#[test]
fn test_get_trips_for_vehicle_returns_chronological_order() {
    let (db, vehicle) = setup_with_vehicle();
    let v_id = vehicle.id.to_string();

    // sort_order intentionally MISMATCHED with dates to expose the bug
    let mut trip_old = Trip::test_ice_trip(NaiveDate::from_ymd_opt(2026, 5, 5).unwrap(), 10.0, None, true);
    trip_old.vehicle_id = vehicle.id;
    trip_old.sort_order = 0; // lowest sort_order but OLDEST date

    let mut trip_new = Trip::test_ice_trip(NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(), 10.0, None, true);
    trip_new.vehicle_id = vehicle.id;
    trip_new.sort_order = 2; // highest sort_order but NEWEST date

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

**Step 2: Add `test_create_trip_orders_by_date_regardless_of_creation_order`**

In [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs):

```rust
#[test]
fn test_create_trip_orders_by_date_regardless_of_creation_order() {
    let (db, app_state, vehicle) = setup_with_vehicle_and_state();
    let v = vehicle.id.to_string();

    // Create in non-chronological order — exactly the user\'s repro
    create_trip_internal(&db, &app_state, v.clone(),
        "2026-05-21T09:00:00".into(), "2026-05-21T09:30:00".into(),
        "A".into(), "B".into(), 10.0, 10000.0, "test".into(),
        None, None, None, None, None, None, None, None, None).unwrap();
    create_trip_internal(&db, &app_state, v.clone(),
        "2026-05-18T04:30:00".into(), "2026-05-18T08:30:00".into(),
        "A".into(), "B".into(), 370.0, 10370.0, "test".into(),
        None, None, None, None, None, None, None, None, None).unwrap();
    create_trip_internal(&db, &app_state, v.clone(),
        "2026-05-20T16:00:00".into(), "2026-05-20T19:00:00".into(),
        "A".into(), "B".into(), 370.0, 10740.0, "test".into(),
        None, None, None, None, None, None, None, None, None).unwrap();

    let trips = db.get_trips_for_vehicle(&v).unwrap();
    assert_eq!(trips[0].start_datetime.date(), NaiveDate::from_ymd_opt(2026, 5, 21).unwrap());
    assert_eq!(trips[1].start_datetime.date(), NaiveDate::from_ymd_opt(2026, 5, 20).unwrap());
    assert_eq!(trips[2].start_datetime.date(), NaiveDate::from_ymd_opt(2026, 5, 18).unwrap());
}
```

**Step 3: Add `test_trip_numbers_same_datetime_tiebroken_by_created_at`**

In [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs):

```rust
#[test]
fn test_trip_numbers_same_datetime_tiebroken_by_created_at() {
    use chrono::Duration;
    let dt = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap().and_hms_opt(8, 0, 0).unwrap();
    let earlier = Utc::now() - Duration::seconds(60);
    let later = Utc::now();

    // sort_order is DELIBERATELY misleading — test only created_at matters
    let trip_a = Trip { /* ...id A, start_datetime: dt, created_at: earlier, sort_order: 5,...*/ };
    let trip_b = Trip { /* ...id B, start_datetime: dt, created_at: later, sort_order: 0,...*/ };

    let nums = calculate_trip_numbers(&[trip_a.clone(), trip_b.clone()]);
    assert_eq!(nums.get(&trip_a.id.to_string()), Some(&1), "earlier created_at gets #1");
    assert_eq!(nums.get(&trip_b.id.to_string()), Some(&2));
}
```

**Step 4: Run the new tests — verify all three fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core test_get_trips_for_vehicle_returns_chronological_order test_create_trip_orders_by_date_regardless_of_creation_order test_trip_numbers_same_datetime_tiebroken_by_created_at
```
Expected: all three FAIL (current ORDER BY sort_order, current tiebreaker is sort_order, current `create_trip` accepts `insert_at_position`).

**Step 5: Commit failing tests**

```bash
git add src-tauri/core/src/db_tests.rs src-tauri/core/src/commands_internal/commands_tests.rs
git commit -m "test(trips): add failing unit tests for datetime ordering"
```

---

# Phase 2 — Implement until tests pass

## Task 3: Make `get_trips_*` order by datetime (test #1 → green)

**Files:**
- Modify: [src-tauri/core/src/db.rs](../../src-tauri/core/src/db.rs)

**Step 1: Update queries**

In `get_trips_for_vehicle` and `get_trips_for_vehicle_in_year`, replace `.order(trips::sort_order.asc())` with:

```rust
.order((trips::start_datetime.desc(), trips::created_at.asc()))
```

**Step 2: Run the targeted test**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core test_get_trips_for_vehicle_returns_chronological_order
```
Expected: PASS.

**Step 3: Run the full backend suite**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```
Expected: all pass. Fix any pre-existing test whose assertion broke (none expected — most tests use single-vehicle, ascending-date fixtures where the two orderings coincide).

**Step 4: Commit**

```bash
git add src-tauri/core/src/db.rs
git commit -m "refactor(db): order trips by start_datetime DESC, created_at ASC"
```

---

## Task 4: Drop `insert_at_position` from `create_trip` (test #2 → green)

**Files:**
- Modify: [src-tauri/core/src/commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) — remove `insert_at_position` param + shift logic; set `sort_order: 0` unconditionally
- Modify: [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) — remove `insert_at_position` from `Args`
- Modify: [src/lib/api.ts](../../src/lib/api.ts) — drop `insertAtPosition` from `createTrip` signature + body
- Modify: [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte) — drop final `insertAtSortOrder` arg from `createTrip` call (state variable can still live; will be removed in Task 9)

**Step 1: Simplify [create_trip_internal](../../src-tauri/core/src/commands_internal/trips.rs)**

Drop the `insert_at_position` parameter. Remove the `if let Some(position) = insert_at_position { ... } else { ... }` branch and `shift_trips_from_position` call. Replace with `let sort_order = 0;` (value is irrelevant now — Task 8 drops the column).

**Step 2: Update [server dispatcher](../../src-tauri/core/src/server/dispatcher.rs)**

Remove `insert_at_position` from the `create_trip` arm's `Args` struct and the call site.

**Step 3: Update [api.ts](../../src/lib/api.ts)**

Remove `insertAtPosition` parameter and field.

**Step 4: Update [TripGrid.svelte](../../src/lib/components/TripGrid.svelte) `handleSaveNew`**

Remove the trailing `insertAtSortOrder` argument from the `await createTrip(...)` call.

**Step 5: Run the targeted test**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core test_create_trip_orders_by_date_regardless_of_creation_order
```
Expected: PASS.

**Step 6: Run backend suite + frontend typecheck**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core && npm run check
```
Expected: pass. Update any test in [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) that called `create_trip_internal(..., Some(N))` — remove the `Some(N)` arg.

**Step 7: Commit**

```bash
git commit -m "refactor(trips): drop insert_at_position from create_trip"
```

---

## Task 5: Remove `reorder_trip` command and DB method

Pure removal. The test is "no callers remain and the command no longer dispatches."

**Files:**
- Modify: [src-tauri/core/src/commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) — delete `reorder_trip_internal`
- Modify: [src-tauri/core/src/db.rs](../../src-tauri/core/src/db.rs) — delete `reorder_trip` + `shift_trips_from_position`
- Modify: [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) — delete the `"reorder_trip"` dispatch arm
- Modify: [src/lib/api.ts](../../src/lib/api.ts) — delete `reorderTrip`
- Modify: [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) — delete `reorder_trip` tests

**Step 1: Delete `reorder_trip_internal` and any tests referencing it**

```bash
grep -rn "reorder_trip_internal\|fn test_.*reorder" src-tauri/core/src/
```
Delete each match.

**Step 2: Delete `reorder_trip` + `shift_trips_from_position` from [db.rs](../../src-tauri/core/src/db.rs)**

**Step 3: Delete the dispatch arm from [server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs)**

**Step 4: Delete `reorderTrip` from [api.ts](../../src/lib/api.ts)**

**Step 5: Compile + run backend suite**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```
Expected: pass (no callers should remain in code we've already updated; frontend callers in [TripGrid.svelte](../../src/lib/components/TripGrid.svelte) will be removed in Task 9 but won't affect Rust compilation).

**Step 6: Verification grep**

```bash
git grep -nE "reorderTrip|reorder_trip" -- src-tauri/ src/lib/api.ts
```
Expected: no output (frontend callers in TripGrid remain — that's expected, removed in Task 9).

**Step 7: Commit**

```bash
git commit -m "refactor(trips): remove reorder_trip command and DB infrastructure"
```

---

## Task 6: Remove `calculate_date_warnings`

The function detected drift that can no longer happen.

**Files:**
- Modify: [src-tauri/core/src/commands_internal/statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs) — delete `calculate_date_warnings`, drop call site in [get_trip_grid_data](../../src-tauri/core/src/commands_internal/statistics.rs)
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) — drop `date_warnings` from `TripGridData`
- Modify: [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) — delete `test_date_warnings_*` tests (the production behavior they tested is structurally impossible now)
- Modify: [src/lib/types.ts](../../src/lib/types.ts) — drop `dateWarnings` field
- Modify: [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte) — drop `dateWarnings` Set + `hasDateWarning` prop on `<TripRow>`

**Step 1: Delete `calculate_date_warnings` function from [statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs)**

**Step 2: Delete `test_date_warnings_detects_out_of_order` and `test_date_warnings_correct_order_no_warnings`**

Justification (record in commit message): "drift is structurally impossible after Task 3 — sort_order no longer drives order — so the warning has no input to react to."

**Step 3: Drop `date_warnings` field from `TripGridData` struct in [models.rs](../../src-tauri/core/src/models.rs) and all initializers in [statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs) (search for `date_warnings:`)**

**Step 4: Drop `dateWarnings` field from frontend [types.ts](../../src/lib/types.ts)**

**Step 5: Drop `dateWarnings` Set and `hasDateWarning` prop from [TripGrid.svelte](../../src/lib/components/TripGrid.svelte)**

The `hasDateWarning` prop on `<TripRow>` (~line 785) and the `dateWarnings` Set declaration / population (~lines 51, 100, 785) — delete.

**Step 6: Run backend tests + frontend typecheck**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core && npm run check
```
Expected: pass.

**Step 7: Commit**

```bash
git commit -m "refactor(trips): remove calculate_date_warnings (drift no longer possible)"
```

---

## Task 7: Update `calculate_trip_numbers` tiebreaker (test #3 → green)

**Files:**
- Modify: [src-tauri/core/src/commands_internal/helpers.rs](../../src-tauri/core/src/commands_internal/helpers.rs)

**Step 1: Update tiebreaker in [calculate_trip_numbers](../../src-tauri/core/src/commands_internal/helpers.rs) and [calculate_odometer_start](../../src-tauri/core/src/commands_internal/helpers.rs)**

Change `.then_with(|| b.sort_order.cmp(&a.sort_order))` to `.then_with(|| a.created_at.cmp(&b.created_at))`.

**Step 2: Run the targeted test**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core test_trip_numbers_same_datetime_tiebroken_by_created_at
```
Expected: PASS.

**Step 3: Run full backend suite**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```

**Step 4: Commit**

```bash
git commit -m "refactor(trips): use created_at as same-datetime tiebreaker"
```

---

## Task 8: Drop `sort_order` column + Rust field

Now safe — no code depends on `sort_order` meaningfully.

**Files:**
- Create: [src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/up.sql](../../src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/up.sql)
- Create: [src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/down.sql](../../src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/down.sql)
- Modify: [src-tauri/core/src/schema.rs](../../src-tauri/core/src/schema.rs) — drop line in `trips` table macro
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) — drop `sort_order` from `Trip`, `TripRow`, `NewTrip`, From impls, and `test_ice_trip` helper
- Modify: [src-tauri/core/src/db_tests.rs](../../src-tauri/core/src/db_tests.rs), [invoice_tests.rs](../../src-tauri/core/src/invoice_tests.rs), [export_tests.rs](../../src-tauri/core/src/export_tests.rs), [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs), [calculations/tests.rs](../../src-tauri/core/src/calculations/tests.rs) — remove `sort_order: 0,` from fixtures
- Modify: [src/lib/types.ts](../../src/lib/types.ts), [src/lib/constants.ts](../../src/lib/constants.ts), [src/routes/+page.svelte](../../src/routes/+page.svelte), [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte), [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte) — drop `sortOrder` references
- Modify: [tests/integration/utils/db.ts](../../tests/integration/utils/db.ts), [tests/integration/fixtures/trips.ts](../../tests/integration/fixtures/trips.ts), [tests/integration/fixtures/types.ts](../../tests/integration/fixtures/types.ts) — drop `sort_order` from fixtures

**Step 1: Create migration**

[up.sql](../../src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/up.sql):
```sql
ALTER TABLE trips DROP COLUMN sort_order;
```

[down.sql](../../src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/down.sql):
```sql
ALTER TABLE trips ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
```

**Step 2: Drop the line from [schema.rs](../../src-tauri/core/src/schema.rs)**

Remove `sort_order -> Integer,` from the `trips` table macro.

**Step 3: Drop the field from all Rust structs in [models.rs](../../src-tauri/core/src/models.rs)**

`Trip`, `TripRow`, `NewTrip`, `From<TripRow> for Trip`, `From<Trip> for NewTrip`, `Trip::test_ice_trip`.

**Step 4: `cargo check` and walk every error**

```bash
cargo check --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```
Fix each error by removing the `sort_order` reference. Most are `sort_order: 0,` in test fixtures.

**Step 5: Run full backend suite**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```
Expected: all pass.

**Step 6: Drop `sortOrder` from frontend types**

- [types.ts](../../src/lib/types.ts) — remove from `Trip`
- [constants.ts](../../src/lib/constants.ts) — remove any mock data fields
- [+page.svelte](../../src/routes/+page.svelte), [TripGrid.svelte](../../src/lib/components/TripGrid.svelte), [TripRow.svelte](../../src/lib/components/TripRow.svelte) — remove all references

```bash
git grep -n "sortOrder" -- src/
```
Walk through and remove each.

**Step 7: Drop `sort_order` from integration fixtures**

[tests/integration/utils/db.ts](../../tests/integration/utils/db.ts), [tests/integration/fixtures/trips.ts](../../tests/integration/fixtures/trips.ts), [tests/integration/fixtures/types.ts](../../tests/integration/fixtures/types.ts).

```bash
git grep -n "sort_order\|sortOrder" -- tests/
```

**Step 8: Frontend typecheck**

```bash
npm run check
```
Expected: no errors.

**Step 9: Verification grep**

```bash
git grep -nE "sort_order|sortOrder" -- src-tauri/ src/ tests/
```
Expected: NO output anywhere.

**Step 10: Commit**

```bash
git commit -m "feat(db): drop trips.sort_order column and Rust/TS field"
```

---

## Task 9: Remove manual sort mode + up/down arrows + date-warning CSS (E2E scenarios 3, 4 → green)

This is the visible UI change. After this task, integration scenarios 3 (no arrows) and 4 (no manual sort toggle) pass.

**Files:**
- Modify: [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte) — drop `SortColumn` type, `sortColumn` prop, sort toggle UI, `reorderDisabled`, `handleMoveUp`, `handleMoveDown`, `handleInsertAbove` cleanup
- Modify: [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte) — drop arrow buttons + props + CSS

**Step 1: Strip manual sort from [TripGrid.svelte](../../src/lib/components/TripGrid.svelte)**

- Remove `type SortColumn = 'manual' | 'tripNumber';` and the `sortColumn` `let` / `export let`.
- Replace conditional sort with single sort: chronological descending by `startDatetime` (with `createdAt` ASC tiebreaker if available, else `id`).
- Remove the `sortable` class and the `.sort-indicator` (▲/▼) span from the `#` / trip-number `<th>` — the column is no longer clickable to toggle direction. There is one order (newest first) and no user-facing way to change it.
- Remove `toggleSort` function (or its body) — no longer reachable.
- Remove `reorderDisabled` reactive and any consumers.
- Remove `handleMoveUp` / `handleMoveDown` functions (lines ~365–399) and the `reorderTrip` import.
- Simplify `handleInsertAbove(targetTrip)` to only pre-fill the date — no more `insertAtSortOrder` state.
- Remove `insertAtSortOrder` state variable and all references; replace template guards with a `newRowAnchorId: string | null` (the trip whose row to render the form above).

**Step 2: Strip arrows from [TripRow.svelte](../../src/lib/components/TripRow.svelte)**

- Remove `onMoveUp`, `onMoveDown`, `canMoveUp`, `canMoveDown` props.
- Remove the two `<button>` elements with `on:click={onMoveUp}` / `on:click={onMoveDown}` (around lines 720–740).
- Remove `hasDateWarning` prop.
- Remove CSS rules: `tr.date-warning`, `tr.date-warning:hover:not(.editing)`, `tr.date-warning.consumption-warning`, and any other selectors involving `date-warning`.

**Step 3: Frontend typecheck**

```bash
npm run check
```
Expected: pass.

**Step 4: Build + run the relevant E2E scenarios**

```bash
npm run test:integration:build
npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/datetime-is-order.spec.ts
```
Expected: Scenarios 3 (no arrows) and 4 (no manual sort) PASS. Scenarios 1, 2, 5, 6 should also pass at this point (everything they depend on is implemented).

**Step 5: Commit**

```bash
git commit -m "refactor(trips): drop manual sort mode, arrows, and date-warning styling"
```

---

## Task 10: i18n cleanup

Remove orphaned translation keys (typecheck enforces this — `npm run check` fails until they're gone or the `LL.<key>()` consumers are removed in Task 9).

**Files:**
- Modify: [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts)
- Modify: [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts)
- Modify: [src/lib/i18n/i18n-types.ts](../../src/lib/i18n/i18n-types.ts) (auto-regen)

**Step 1: Find obsolete keys**

```bash
git grep -nE "trips\.(moveUp|moveDown|legend\.dateWarning|legend\.highConsumption.*date)|toast\.errorMoveTrip" -- src/
```
The ones with consumers in `src/lib/components/` were removed in Task 9; the keys in `i18n/*` are now orphans.

**Step 2: Remove the matching entries from [sk/index.ts](../../src/lib/i18n/sk/index.ts) and [en/index.ts](../../src/lib/i18n/en/index.ts)**

**Step 3: Regenerate types**

```bash
npx typesafe-i18n --no-watch
```

**Step 4: Frontend typecheck**

```bash
npm run check
```
Expected: pass.

**Step 5: Commit**

```bash
git commit -m "chore(i18n): remove obsolete reorder/date-warning strings"
```

---

# Phase 3 — Verify everything

## Task 11: Full-suite verification + docs + archive

**Files:**
- Modify: [CHANGELOG.md](../../CHANGELOG.md)
- Modify: [DECISIONS.md](../../DECISIONS.md) (via [/decision](../../.claude/skills/decision-skill))
- Move: [_tasks/65-datetime-is-order/](.) → [_tasks/_done/65-datetime-is-order/](../_done/)
- Modify: [_tasks/index.md](../index.md)

**Step 1: Run full backend test suite**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```
Expected: all pass.

**Step 2: Run full integration suite**

```bash
npm run test:integration:build
npm run test:integration
```
Expected: all pass — including all 6 scenarios in [datetime-is-order.spec.ts](../../tests/integration/specs/tier2/datetime-is-order.spec.ts).

**Step 3: Manual smoke test in dev mode**

```bash
npm run tauri:dev
```
With the user's existing DB (the one with the corrupted sort_orders):
- Grid renders trips in correct chronological order (newest first)
- No red rows visible
- No up/down arrow buttons in any row
- No sort mode toggle in column headers
- Click "+" on any row → new-trip form appears above that row, date pre-filled
- Save a trip with a past date → it appears in the correct chronological position
- Edit a trip's start_datetime → on save, the trip moves to the new chronological position

**Step 4: Verification grep — nothing should match**

```bash
git grep -nE "sort_order|sortOrder|reorder_trip|reorderTrip|calculate_date_warnings|dateWarning|date-warning" -- src-tauri/ src/ tests/ docs/
```
Expected: no output (or only matches inside this `02-plan.md`).

**Step 4a: Fix stale documentation in [docs/features/multi-year-state.md](../../docs/features/multi-year-state.md)**

If the grep above matches lines in `docs/features/multi-year-state.md` (it will — there are stale references to `reorder_trip` / `shift_trips_from_position` from before Task 65), update those lines to reflect the current code (datetime-based ordering, no reorder command).

**Step 5: Update [CHANGELOG.md](../../CHANGELOG.md)**

Under `[Unreleased]`:
```markdown
### Changed
- Trip order is now derived purely from `start_datetime` — manual reordering removed. New trips appear at the correct chronological position automatically.

### Fixed
- "Date warning" red rows no longer appear for trips created out of chronological order via the `+` button (the underlying `sort_order` field was dropped — drift is structurally impossible).

### Removed
- Manual sort mode toggle in the trips grid.
- Up/down reorder arrows on each trip row.
- `reorder_trip` Tauri command (was unused after manual reordering was removed).
```

**Step 6: Record ADR via [/decision](../../.claude/skills/decision-skill)**

Add an entry to [DECISIONS.md](../../DECISIONS.md):
> **ADR-NNN: Datetime is the only source of trip order.** Manual `sort_order` field dropped. Rationale: a separate "manual order" field can drift from `start_datetime`, producing confusing "date warning" red rows for users with chronologically valid data. Editing a trip's `start_datetime` is now the only way to change its position. Tiebreaker for identical datetimes: `created_at` ASC. See task [65-datetime-is-order](../_tasks/_done/65-datetime-is-order/).

**Step 7: Commit docs**

```bash
git add CHANGELOG.md DECISIONS.md
git commit -m "docs: changelog + ADR for datetime-is-order"
```

**Step 8: Move task folder to `_done/` and update [index.md](../index.md)**

```bash
git mv _tasks/65-datetime-is-order _tasks/_done/65-datetime-is-order
```

Edit [_tasks/index.md](../index.md): move row from Active to Completed table.

```bash
git add _tasks/index.md
git commit -m "docs: archive task 65 (datetime-is-order) as complete"
```

---

## Verification Checklist (final gate)

Tests:
- [ ] All 3 backend unit tests added in Task 2 pass
- [ ] All 6 E2E scenarios in [datetime-is-order.spec.ts](../../tests/integration/specs/tier2/datetime-is-order.spec.ts) pass
- [ ] `cargo test -p kniha-jazd-core` passes
- [ ] `npm run check` passes
- [ ] `npm run test:integration` passes
- [ ] `npm run test:all` passes

Code removal verified by grep:
- [ ] No `sort_order` / `sortOrder` references in code
- [ ] No `reorderTrip` / `reorder_trip` references
- [ ] No `calculate_date_warnings` references
- [ ] No `dateWarning` / `date-warning` references

Manual smoke:
- [ ] Existing user data renders without red rows
- [ ] `+` button on any row creates a trip in the correct chronological position
- [ ] Editing a trip's datetime moves it to the correct position
- [ ] No arrow buttons visible
- [ ] No manual sort toggle visible

Docs:
- [ ] [CHANGELOG.md](../../CHANGELOG.md) updated
- [ ] [DECISIONS.md](../../DECISIONS.md) has new ADR
- [ ] Task moved to `_done/` and [index.md](../index.md) updated

---

## Rollback plan

The migration's [down.sql](../../src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/down.sql) re-adds `sort_order INTEGER NOT NULL DEFAULT 0`. Mid-rollout: `git revert` the head commit + `diesel migration revert`. Manual sort_order data is permanently lost (acceptable: no code depended on it after Task 3).
