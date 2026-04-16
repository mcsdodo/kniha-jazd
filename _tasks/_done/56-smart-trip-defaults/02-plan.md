# Smart Trip Defaults — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Auto-clamp ODO regressions and pre-fill start/end times on new trip rows from the most recent matching route.

**Architecture (ADR-008 — all business logic in Rust):**
- Backend (Rust): one Tauri command returns the *final* inferred start/end datetimes for a given `(vehicle_id, origin, destination, row_date)`. Jitter happens in Rust. For testability, split into two Rust functions:
  - `compute_inferred_times(row_date, base_start, base_duration_mins, jitter: &mut dyn Jitter) -> (NaiveDateTime, NaiveDateTime)` — pure, deterministic under a stub jitter trait.
  - `get_inferred_trip_time_for_route(...)` — Tauri command: DB lookup → constructs a real `ThreadRngJitter` → calls the pure helper → returns `Option<{startDatetime, endDatetime}>`.
- Frontend (Svelte): ODO clamp in `handleOdoChange`; on new row with both endpoints filled, `invoke('get_inferred_trip_time_for_route', { ..., rowDate })` and write returned values straight into `formData.startDatetime` / `formData.endDatetime`. No calculation, no jitter, no helper file.

---

## Task 1: Backend — pure `compute_inferred_times` helper with injectable jitter

**Files:**
- Create: `src-tauri/src/calculations/time_inference.rs` (pure logic + `Jitter` trait)
- Modify: `src-tauri/src/calculations/mod.rs` (expose submodule)
- Add tests: co-located `#[cfg(test)] mod tests` in the new file

**Steps:**
1. Define a small trait:
   ```rust
   pub trait Jitter {
       fn minutes(&mut self) -> i64;        // returns value in [-15, 15]
       fn duration_factor(&mut self) -> f64; // returns value in [0.85, 1.15]
   }
   ```
2. Implement `ThreadRngJitter` (uses `rand::thread_rng`) for production.
3. Write failing unit tests using a `StubJitter { minutes, factor }` with fixed return values:
   - `test_inferred_times_applies_max_positive_jitter` — jitter +15 min, factor 1.15 → start and end offsets match expectation exactly.
   - `test_inferred_times_applies_zero_jitter` — 0/1.0 → outputs equal base times on given date.
   - `test_inferred_times_crosses_midnight` — base 23:50 + 30 min duration + max jitter → end_datetime is next calendar day.
   - `test_inferred_times_rounds_duration_to_minute` — verify factor × duration is rounded consistently (pick ceil vs round — document choice).
4. Implement `pub fn compute_inferred_times(row_date: NaiveDate, base_start: NaiveTime, base_duration_mins: i64, jitter: &mut dyn Jitter) -> (NaiveDateTime, NaiveDateTime)` to make tests pass.

**Verification:** `cd src-tauri && cargo test time_inference` — all four tests pass.

---

## Task 2: Backend — Tauri command `get_inferred_trip_time_for_route`

**Files:**
- Modify: `src-tauri/src/models.rs` (add `InferredTripTime { start_datetime: String, end_datetime: String }`)
- Modify or create: `src-tauri/src/commands/trips.rs` (add command)
- Modify: `src-tauri/src/lib.rs` (register command in `invoke_handler`)
- Add tests: `src-tauri/src/commands/commands_tests.rs`

**Steps:**
1. Write failing test `test_inferred_time_returns_none_when_no_match`.
2. Write failing test `test_inferred_time_uses_most_recent_match` — seed 3 trips on same route, different dates; stub jitter returning zeros → assert returned start/end times equal newest trip's HH:MM applied to the supplied `row_date`.
3. Write failing test `test_inferred_time_ignores_null_end_datetime`.
4. Write failing test `test_inferred_time_scoped_to_vehicle` — trips with matching route on a different vehicle must not leak.
5. Signature:
   ```rust
   #[tauri::command]
   pub async fn get_inferred_trip_time_for_route(
       vehicle_id: String,
       origin: String,
       destination: String,
       row_date: String, // "YYYY-MM-DD"
       state: State<'_, AppState>,
   ) -> Result<Option<InferredTripTime>, String>
   ```
6. SQL lookup (most recent match with non-null end): `SELECT start_datetime, end_datetime FROM trips WHERE vehicle_id = ?1 AND origin = ?2 AND destination = ?3 AND end_datetime IS NOT NULL ORDER BY start_datetime DESC LIMIT 1`.
7. Parse `base_start` (HH:MM) and `base_duration_mins`; construct `ThreadRngJitter`; call `compute_inferred_times`; format outputs as ISO `YYYY-MM-DDTHH:MM:SS`.
8. Tests inject a `StubJitter` via a test-only seam (either `#[cfg(test)]` alternate entry point that accepts a `&mut dyn Jitter`, or make the command thin and test the seam directly). Prefer: split command body into `compute_for_route(db, vehicle_id, origin, destination, row_date, jitter) -> Option<InferredTripTime>` and unit-test that.
9. Register command in `lib.rs`.

**Verification:** `cd src-tauri && cargo test get_inferred_trip_time_for_route` — all four tests pass.

---

## Task 3: Frontend — ODO clamp in `handleOdoChange`

**Files:**
- Modify: `src/lib/components/TripRow.svelte` (function `handleOdoChange`, line ~186)

**Steps:**
1. In `handleOdoChange`, after reading `newOdo` from input, add guard:
   ```ts
   if (previousOdometer > 0 && newOdo < previousOdometer) {
       newOdo = previousOdometer + 1;
       // write clamped value back to the input element
   }
   ```
2. Ensure the input element reflects the clamped value (two-way binding or explicit set on `event.target.value`).
3. Preserve the existing auto-calc and manual-edit paths after clamping.

**Verification:** manual smoke-test in dev (`npm run tauri dev`): enter ODO lower than previous row → field snaps to `previous + 1`. Integration test (Task 4) will formalise this.

---

## Task 4: Frontend — wire time inference for new rows

**Files:**
- Modify: `src/lib/components/TripRow.svelte` (invoke the backend command when a new row has both endpoints)

**Steps:**
1. Add a reactive block (or extend the existing origin/destination handler) that, when both `origin` and `destination` are non-empty AND the row is new (`isNew && !trip?.id`), calls:
   ```ts
   const result = await invoke<InferredTripTime | null>('get_inferred_trip_time_for_route', {
       vehicleId, origin, destination, rowDate: extractDate(formData.startDatetime)
   });
   if (result) {
       formData.startDatetime = result.startDatetime;
       formData.endDatetime = result.endDatetime;
   }
   ```
2. Use the row's currently selected date (from `formData.startDatetime` or the prefill logic already in place) as `rowDate`. Do not derive a new date on the frontend.
3. Guard: never invoke this command for existing/saved rows (`trip?.id` set). Also guard against redundant invocations (e.g., debounce or run only when `(origin, destination)` pair actually changes).
4. Add `InferredTripTime` type to `src/lib/types.ts` mirroring the Rust struct.

**Verification:** manual smoke-test in `npm run tauri dev`: create new trip with known route → start/end auto-fill; edit an existing trip's origin/destination → times unchanged. Integration tests (Task 5) formalise this.

---

## Task 5: Integration tests

**Files:**
- Modify: appropriate file under `tests/integration/` (find the trip-grid spec covering new-row entry).

**Steps:**
1. Add test: "ODO below previous row clamps to previous+1" — add a new trip, enter ODO = previousODO − 100, assert field value becomes previousODO + 1.
2. Add test: "time fields auto-fill on known route for new rows" — prior-seed a trip with known origin→destination, add a new trip with same route, assert `start_datetime` and `end_datetime` are non-empty and within expected jitter bounds.
3. Add test: "editing existing row does not re-infer times" — edit an existing trip's origin/destination, assert start/end times remain unchanged.

**Verification:** `npm run test:integration:tier1` includes these and passes.

---

## Task 6: Documentation

**Files:**
- Modify: `CHANGELOG.md` — entry under `[Unreleased]`:
  - *Added:* Time inference from known routes on new trip rows.
  - *Changed:* ODO entered below the previous row's ODO is now auto-clamped to `previous + 1`.
- Modify: `DECISIONS.md` — ADR for the time-inference algorithm: why most-recent + bounded jitter; why jitter stays in Rust (ADR-008 reaffirmation); testability pattern (pure helper + `Jitter` trait).

**Verification:** `/verify` passes; `git diff CHANGELOG.md DECISIONS.md` shows the new entries.

---

## Task 7: Final verification

**Steps:**
1. `npm run test:backend` — 195+ tests still pass, plus new ones from Task 1.
2. `npm run test:integration:tier1` — new integration tests pass.
3. Manual smoke-test in dev build: add a new trip with a known route, confirm time auto-fill; enter a regressing ODO, confirm clamp.

**Verification:** all tests green, manual flows behave as specified in `01-task.md`.
