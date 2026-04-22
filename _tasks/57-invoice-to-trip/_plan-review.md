# Plan Review — Invoice-to-Trip (Task 57)

**Date:** 2026-04-22
**Reviewed file:** `_tasks/57-invoice-to-trip/02-plan.md`
**Recommendation:** Needs Revisions (3 Critical, 7 Important, 5 Minor)

---

## Summary

The plan is well-structured and strongly aligned with ADR-008 (all logic in Rust, frontend displays only). It reuses an existing Jitter pattern from task 56 and breaks work into testable, independently shippable tasks. However, several concrete gaps would stall an implementer:

- The pure function's **input shape for receipts** (raw `Receipt` vs a normalized view) conflicts with the described "Receipt date-only → end at 12:00" case — `receipt_datetime` is already a `NaiveDateTime`, so date-only vs. full-time is not distinguishable from the struct alone.
- **Task 1 doesn't cover the year-scope requirement** — all other similar lookups in the codebase are vehicle + year scoped (see `get_trips_for_vehicle_in_year`). Using "most recent across all history" may pick a destination from 2022 and surprise users.
- **No story for the new `other_costs` fields** — the receipt may contain `total_price_eur` along with non-fuel costs (car wash on same receipt), but the plan pre-fills `fuel_cost` only. This is fine for the typical case but should be stated as an explicit non-goal.

Plus smaller issues below (destination picker UX, route-history key scoping, alias normalization, integration test seed data, and the Clock trait's under-specified scope).

---

## Findings

### Critical

- [ ] **C1 — `build_trip_draft` signature cannot distinguish date-only receipts from full-datetime receipts.**
  The plan's Logic step 1 says "If receipt has time → jitter; if date-only → 12:00 + warning." But `Receipt.receipt_datetime` in `src-tauri/src/models.rs:504` is `Option<NaiveDateTime>`. Parsing is done in `From<ReceiptRow>` at `models.rs:1174` using `"%Y-%m-%dT%H:%M:%S"` format — if the OCR ever stored "date-only", it would fail to parse and become `None`, not a `NaiveDateTime` at 00:00:00. Clarify the behaviour explicitly:
  - Either: pass separate `receipt_date: NaiveDate` + `receipt_time: Option<NaiveTime>` into the pure function so date-only is representable.
  - Or: inspect what `gemini.rs` actually produces for date-only receipts today and document it (is it `None` or `T00:00:00`?). Then state the convention.
  Without this, Task 3's test *"Receipt date-only → end at 12:00, end_time_approximate = true"* cannot be written against a concrete input value.

- [ ] **C2 — Task 1 (previous trip destination) must be scoped by year to match existing patterns.**
  Every other "most recent per vehicle" lookup in the codebase (e.g., `get_trips_for_vehicle_in_year` in `db.rs`, the year picker in the UI) is year-scoped. If a user switches to 2026 and adds the very first row, we should pre-fill from "most recent 2026 trip", not leak a 2023 destination. Either:
  - Add a `year` parameter to `find_previous_trip_destination(vehicle_id, year)` and fall back to prior years only if the current year has no trips, OR
  - Explicitly document in Task 1 that cross-year carryover is the intended behaviour (mirroring how odometer carries over at year start).
  The plan is silent, which will be a surprise either way. Decide and state it.

- [ ] **C3 — Task 4 pre-assumes a ReceiptCard component that does not exist as a separate file.**
  `src/lib/components/ReceiptCard.svelte` does not exist. Receipt cards are rendered inline in `src/routes/doklady/+page.svelte` (see `class="receipt-card"` at lines 618 and 757 — there are TWO card markups: one for unassigned receipts, one for assigned). The plan's "Modify: receipt card component (likely `src/lib/components/ReceiptCard.svelte` or similar)" will force the implementer to make an unplanned decision: extract a component first, or add the button inline to both card templates. Pick one now:
  - Recommended: add the button inline under both card blocks in `+page.svelte` (smallest diff, matches existing style). Or
  - Extract `ReceiptCard.svelte` as a separate prep step (larger refactor, out of scope for this task).
  Without this decision, Task 4's "Files" block is misleading and estimation will be wrong.

### Important

- [ ] **I1 — Alias lookup normalization is mentioned but not defined.**
  Task 2 says "normalize_location util as routes" but `normalize_location` (`db.rs:40`) only trims/collapses whitespace — it does NOT lowercase, and OCR'd station addresses commonly vary in case (`"OMV Bratislava"` vs `"OMV BRATISLAVA"`), diacritics, and punctuation (`"Bratislava, Einsteinova 19"` vs `"Einsteinova 19, Bratislava"`). Either:
  - Accept that users re-pick on case/punctuation variants (state this explicitly — it's a legit conservative choice), OR
  - Define a dedicated station normalizer (e.g., uppercase + strip `,`/`.`/whitespace) in Task 2 and test it.
  The current wording ("same normalize_location util as routes") suggests the first but without acknowledging the UX consequence. Decide and document.

- [ ] **I2 — Route-history distance lookup uses the alias-resolved destination, but origin may not match previous trip exactly.**
  Task 3 Logic step 3 says distance lookup is `(vehicle_id, origin, alias_destination)`. Origin comes from "previous trip's destination" (Task 1). In practice:
  - Previous trip destination = "Budapest" (user-typed free text)
  - Alias resolution for new station = "Budapest, Váci út 12" (user chose invoice value on first use)
  → `routes` lookup `(vehicle_id, "Budapest", "Budapest, Váci út 12")` misses even though semantically the legs are identical.
  This matches what the task doc warned about ("OCR station strings rarely match user's historical free-text destinations exactly"), but the plan doesn't handle the symmetric origin problem. Either:
  - Accept the miss on first-ever invoice for a given (origin, station) pair — user types distance manually, and next time the destination is already the alias so origin matches (chicken-and-egg resolved by usage).
  - Or mention this known limitation in the plan's Out of Scope.

- [ ] **I3 — `Clock` trait is introduced but never used in the pure function's logic.**
  Task 3 says "Define `TripDraft` struct + `Clock` trait (if not already present)." But the Logic steps 1–7 don't reference current time anywhere — `end_datetime` comes from `receipt_datetime`, `start_datetime` is derived from end. What is `Clock` actually for? If it's for `created_at` on an alias upsert, that's in Task 4's wrapper, not the pure helper. Either:
  - Remove `Clock` from the pure function's signature.
  - Or specify exactly where it is consumed (e.g., to stamp "approximate" warnings based on receipt age? Currently not described.)
  Leaving an unused parameter is a code-review rejection waiting to happen.

- [ ] **I4 — Task 5 integration tests need receipt fixtures, which the plan doesn't mention.**
  Scenario "Upload fuel invoice → OCR completes → button visible" requires either (a) a real fuel invoice image and a running Gemini key, or (b) a seeded `Receipt` in the DB with the parsed fields already set. The existing integration test suite uses seeded DB records (see `.claude/rules/integration-tests.md`). The plan should specify: "Tests seed a parsed Receipt directly into the DB (no OCR roundtrip), verify the UI flow starting from the receipt card."

- [ ] **I5 — "Smoke test" for Task 4 is not a real verification step.**
  Task 4's Verification is "Manual UI test — upload a fuel invoice → click button → dialog opens...". This is not automatable and leaves Task 4 un-verifiable in CI. Either:
  - Promote the smoke test into a tier-1 integration test here (but that duplicates Task 5).
  - Or reword to: "Task 4 verification deferred to Task 5 integration tests; developer does a manual dev-build smoke-test before committing Task 4."
  As written, an implementer following the plan will commit Task 4 based on a subjective "it worked on my machine".

### Minor

- [ ] **M1 — Migration filename placeholder `{next_number}` should be a concrete date.**
  Existing migrations use `YYYY-MM-DD-HHMMSS_description` format (`src-tauri/migrations/2026-02-12-100000_add_vehicle_ha_fuel_level_sensor`). Tell the implementer to use `2026-04-DD-100000_create_fuel_station_aliases` or similar. Saves them a trip to grep existing filenames.

- [ ] **M2 — Task ordering diagram says "Tasks 2 and 3 can be developed in parallel" but Task 3's pure function test for alias hits needs Task 2's data shape.**
  Fine as-is because Task 3 tests pass `Option<String>` directly (not hitting Task 2's function), but worth clarifying: "Task 3's pure function tests pass `alias_lookup` as a stubbed `Option<String>`; only the Task 4 wiring requires Task 2 fully implemented."

- [ ] **M3 — `link_receipt_to_trip` command is described in Task 4 but no unit test is listed for the receipt-already-linked guard.**
  Task 4 step 1 says "with Rust unit tests" but doesn't enumerate cases. Should include: linking an already-linked receipt (should it error, overwrite, or be idempotent?). Current receipt assignment flow in `receipts_cmd.rs:395` uses explicit `assign_receipt_to_trip` with `assignment_type` — this new command needs a clear spec on whether it's a thin wrapper over that or a new code path (I'd expect: reuses `assign_receipt_to_trip` with `AssignmentType::Fuel`).

---

## Round 2 Additional Findings

- [ ] **I6 — `link_receipt_to_trip` should re-use existing `assign_receipt_to_trip_internal` — plan should say so explicitly.**
  `assign_receipt_to_trip_internal` in `receipts_cmd.rs:392` already handles: populating fuel fields on the trip, setting `trip_id`, `vehicle_id`, `assignment_type`, and `mismatch_override`. Introducing a parallel `link_receipt_to_trip` command risks two code paths that diverge over time. Recommendation: the new flow just calls `assign_receipt_to_trip` with `assignment_type = "Fuel"` and `mismatch_override = false`. Task 4's "Steps" should make this reuse explicit rather than implying a new, thin Tauri command. If the plan authors intended a new command for some reason (e.g., to skip the fuel-population because the new trip already has fuel from the dialog), state that rationale.

- [ ] **I7 — `receipts.trip_id` is `UNIQUE NOT NULL` in the schema — dialog must handle duplicate receipt assignment.**
  Per `migrations/2026-01-08-095218-0000_baseline/up.sql:73`: `trip_id TEXT UNIQUE`. If the user opens the dialog, backgrounds the app, the receipt is auto-matched by some other flow, and they click Save: `create_trip` succeeds but `link_receipt_to_trip` would fail with a constraint violation. Plan should specify:
  - Pre-check in `prepare_trip_from_receipt`: if `receipt.trip_id.is_some()` return an error (UI shouldn't even open the dialog — the button is hidden per req 9, but there's a TOCTOU window).
  - Post-check in save flow: if linking fails due to UNIQUE constraint, refund by deleting the just-created trip (transactional), or surface a clear error and leave the trip created.
  Task 4 doesn't mention either path. Pick one and document it.

- [ ] **M5 — `prepare_trip_from_receipt` must validate receipt's `vehicle_id` is set before returning a draft.**
  `Receipt.vehicle_id: Option<Uuid>` (`models.rs:495`) — a newly-scanned receipt may have no vehicle assigned yet. Requirement 2 says the button only shows when "receipt is FUEL with a known vehicle", but the backend command should defensively reject `vehicle_id = None` with a clear error rather than silently building a draft with `vehicle_id = ""`. Add to Task 3 steps: "Error if `receipt.vehicle_id.is_none()` — the UI should not invoke this command without an assigned vehicle."

- [ ] **M4 — Route `find_or_create_route` auto-fires on `create_trip` — good, but plan doesn't note the consequence for alias-fed destinations.**
  `commands/trips.rs:124` calls `db.find_or_create_route(vehicle_id, origin, destination, distance_km)` on every trip create. After the first invoice-to-trip flow at a given station, a `routes` row will exist for `(vehicle_id, previous_destination, alias_destination)`. Subsequent invoices from the same station will hit the route lookup in `build_trip_draft` (Task 3 Logic step 3) and auto-fill distance. This is the correct behaviour — worth mentioning in the plan as a beneficial side effect, and in the feature doc (Task 6) as part of the "self-learning" story.

---

## What's Good

- **ADR-008 alignment is strong**: single `prepare_trip_from_receipt` command, pure helper with injectable jitter following the task 56 pattern already in `src-tauri/src/calculations/time_inference.rs`. Reusing the `Jitter` trait is the right call.
- **Task 1 as a standalone ship** is a smart risk-reduction move; it also gets production signal on the origin-prefill idea before the bigger flow depends on it.
- **Explicit out-of-scope section** (no Google Maps, no alias editing v1, no leg-2 automation) prevents scope creep during execution.
- **Test scenarios in Task 3** cover the speed heuristic thresholds at realistic values (65 km → 100 km/h; 30 km → 40 km/h), which is exactly where boundary bugs hide.
- **Task 6 prescribes `/decision` for the three BIZ calls** (speed thresholds, jitter window, alias scope) — matches project conventions.

---

## Recommendation

**Needs Revisions** before implementation. Critical items C1–C3 block coding (ambiguous function signature, undefined year-scope semantics, missing file location for UI change). Important items I1–I5 would cause review churn. Minor items are polish.

Once C1–C3 are resolved with explicit decisions in the plan, the implementation path is clear and the plan is otherwise solid.
