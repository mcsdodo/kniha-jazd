# Test Coverage Review (Pre-Implementation)

**Target:** _tasks/66-multi-invoice — planned test coverage in [03-plan.md](./03-plan.md)
**Reference:** [01-task.md](./01-task.md), [02-design.md](./02-design.md)
**Started:** 2026-07-14
**Status:** In Progress
**Focus:** Migration data integrity (HARD requirement: the two table-rebuild
migrations MUST NOT lose or corrupt any existing data and MUST complete
successfully on real production DBs), plus completeness/edge cases of all
planned tests.

**Context:** Implementation has NOT started — this reviews the *planned* tests
in the plan against the design and the existing codebase, so gaps get fixed in
the plan before code is written. Note: the repo currently has NO
data-preserving migration tests (migrations only run against fresh in-memory
DBs in `db_tests.rs`), so every migration-safety guarantee must be added by
this task's tests.

**Baseline:** 360 backend tests, all passing (`cargo test -p kniha-jazd-core`, 2026-07-14)

## Iteration 1

Verification notes: migrations are embedded and run per-migration inside a
transaction with no `metadata.toml` overrides
([db.rs](../../src-tauri/core/src/db.rs) lines 24, 59); there is **no automated
backup before migration** (backups are manual/pre-update only,
[backup.rs](../../src-tauri/core/src/commands_internal/backup.rs)); **no
`PRAGMA foreign_keys` anywhere in Rust source** — FK enforcement is OFF, so the
rebuild dance cannot fail on FKs, but orphaned `receipts.trip_id` rows are
possible in production ([db.rs](../../src-tauri/core/src/db.rs) lines 413–419 —
`delete_trip` clears paperless links but never nulls `receipts.trip_id`). The
plan's proposed `to_cents`/`money_add`/`money_sub` implementation was compiled
and run: **every proposed assertion passes**, and the bit-exact `f64`
assertions are sound because `from_cents` is correctly-rounded division of
exact small integers.

### New Coverage Gaps — Critical

- **[C1] ZERO migration data-integrity tests planned** — the user's top
  requirement has no executable test. [02-design.md](./02-design.md) line 167
  lists "migration backfill heuristic" as a test, but no plan task implements
  it; Task 2 Step 5 / Task 4 Step 4 only run the suite to provoke compile
  errors; all existing tests use fresh `Database::in_memory()`
  ([db_tests.rs](../../src-tauri/core/src/db_tests.rs) lines 8–12, 406–423). A
  failed migration panics at startup with no auto-backup — tests are the only
  safety net. **Add (new Task 4.5):** helper `open_db_migrated_to(version)`
  (apply embedded migrations up to but excluding the Task-66 ones, seed legacy
  rows via raw SQL, run the rest), then:
  `test_receipts_migration_preserves_every_row_and_column` (~6 receipts:
  assigned Fuel, assigned Other, unassigned, all-optional-NULL, orphaned
  trip_id, non-ASCII text; assert row count + every column value +
  `amount_applied = 0`),
  `test_paperless_links_migration_preserves_rows`,
  `test_receipts_migration_recreates_indexes`.
- **[C2] Backfill heuristic untested against real pre-state data.** The
  `CASE WHEN EXISTS…` backfill ([03-plan.md](./03-plan.md) lines 295–312) is
  the only data *transformation* in the feature and has no test. Edge cases:
  trip with `fuel_liters > 0` AND a Fuel receipt (must become `'Other'`),
  `fuel_liters` NULL (SQL falsy → `'Other'`), `fuel_liters = 0`, orphaned
  link/trip. **Add:** `test_backfill_fuel_when_trip_fueled_and_no_fuel_receipt`,
  `test_backfill_other_when_fuel_receipt_already_attached`,
  `test_backfill_other_when_fuel_liters_null_or_zero`.
- **[C3] Plan's rebuild DDL diverges from the real migrated schema — two
  defects that would corrupt or abort the migration:**
  (a) migration `2026-02-01-100000_replace_receipt_date_with_datetime`
  **dropped** `idx_receipts_date` and created `idx_receipts_datetime`; the
  plan's up.sql recreates the stale name and never recreates
  `idx_receipts_datetime`.
  (b) production `mismatch_override` is nullable
  (`2026-02-03-100000_receipt_assignment_type`); the plan declares it
  `NOT NULL` and copies verbatim — any NULL row aborts the whole migration and
  bricks startup. **Fix plan:** `COALESCE(mismatch_override, 0)` in the copy
  SELECT; correct index names. **Add:**
  `test_receipts_migration_tolerates_null_mismatch_override`.
- **[C4] Receipt reassignment never reverses the old trip's sum.** Task 6
  rule 5 handles reversal only for Paperless; the receipt path has no old-trip
  handling and no test — moving an applied Other receipt from trip A to B
  leaves A inflated forever. **Add rule + test:**
  `test_receipt_reassign_reverses_old_trip_sum`.
- **[C5] No fresh-vs-migrated schema identity test.** Drift already exists
  (C3a; [schema.rs](../../src-tauri/core/src/schema.rs) line 30 declares
  `mismatch_override -> Integer` non-null vs nullable DB column). **Add:**
  `test_migrated_schema_identical_to_fresh_schema` (normalized
  `sqlite_master` diff, assert empty).

### New Coverage Gaps — Important

- **[I1] `can_attach` not updated for the new Fuel cardinality rule** —
  `check_invoice_trip_compatibility` returns `can_attach: true` in every
  branch; Task 8 removes frontend gating citing it as authoritative. **Add:**
  plan step + `test_trips_for_fuel_invoice_assignment_excludes_covered_trip`
  (both sources), or an explicit error-on-assign UX decision asserted in the
  integration spec.
- **[I2] Note appending/stripping (Requirement 3) has no test.** **Add:**
  `test_other_assign_appends_note_and_unassign_strips_it` (user-edited note
  left untouched).
- **[I3] NULL-amount Other *receipt* untested** (link-only + poisons sum
  check). **Add:** `test_assign_other_receipt_null_price_is_link_only`; extend
  `test_invoice_coverage_per_type_and_sum` with NULL-price Other receipt →
  `has_unknown_amount = true`.
- **[I4] Reassigning a link-only paperless doc must NOT reverse old trip.**
  **Add:** `test_paperless_reassign_link_only_does_not_touch_old_trip`.
- **[I5] No "no false warnings after upgrade" test on migrated data.**
  **Add:** `test_migrated_db_produces_no_sum_mismatch_warnings` (backfilled
  NULL-amount link + populated `other_costs_eur` → absent from
  `other_sum_mismatches` AND `missing_other_invoices`).
- **[I6] Unassign-to-zero must store `None`, not `Some(0.0)` — untested.**
  **Add:** `test_unassign_last_applied_other_resets_costs_to_none`.
- **[I7] Plan doesn't name the existing tests it will break:**
  `paperless_link_unique_trip_invariant`
  ([db_tests.rs](../../src-tauri/core/src/db_tests.rs) line 479) asserts the
  removed one-doc-per-trip invariant;
  `paperless_link_upsert_creates_then_replaces` (line 430) uses the old API.
  List them in Task 5 with new expected semantics.

### New Coverage Gaps — Minor

- down.sql failure modes undocumented (receipts down.sql fails outright once
  N receipts share a trip; paperless down.sql silently discards N−1 links) —
  document in SQL comments, no test.
- Non-finite/negative money inputs: `to_cents(NAN) == 0`, huge values saturate
  silently. Consider `test_assign_rejects_non_finite_or_negative_amount` or
  document the trust boundary.
- Guard tolerance contradiction: [01-task.md](./01-task.md) says "±0.01",
  [02-design.md](./02-design.md) says cent-exact. Pin with one test
  (12.34 vs 12.335 → guard does NOT trigger).
- Misleading comment in proposed `test_to_cents_rounds_half_away`
  ("banker's-rounding trap" — actually a binary-representation trap).
- Integration spec step 5 re-asserts backend subtraction — phrase as display
  verification only.
- i18n: integration spec should assert actual Slovak tooltip text, not a key.

### Test Quality Issues

- Proposed bit-exact `f64` equality assertions verified SOUND (compiled & run).
- Proposed `to_cents` implementation passes all its own proposed assertions.

### Coverage Assessment

Reviewed: migrations (deep, incl. real DDL history diff), money helpers
(executed), assignment semantics, coverage/indicator logic, integration spec,
existing-test impact. Remaining: frontend prop wiring depth, export
non-regression, concurrency/HTTP-server angles.

## Iteration 2

Verification notes: full-corpus grep confirms **nothing references `receipts`**
(no `REFERENCES receipts`, no views, no triggers in any migration) — the
DROP/RENAME dance cannot break other objects; `legacy_alter_table` concern is
moot. Desktop app and embedded HTTP server share one `Arc<Database>` with a
single `Mutex<SqliteConnection>`
([server/mod.rs](../../src-tauri/core/src/server/mod.rs) lines 28–33); an old
binary opening a newer DB already lands in read-only mode — no new in-process
concurrency gap.

### New Coverage Gaps — Critical

- **[C6] Two migration directories = two transactions = an untested
  half-migrated state that crash-loops the app.** Diesel applies each
  migration dir in its own transaction, and `Database::new` panics on failure
  ([db.rs](../../src-tauri/core/src/db.rs) lines 59–60; same path in
  [desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs) line 82 and
  [web/src/main.rs](../../src-tauri/web/src/main.rs) line 38, both *before*
  any backup or compatibility check). If the receipts rebuild commits and the
  paperless rebuild fails, the production DB is permanently half-migrated and
  **every subsequent launch panics before the UI exists**. **Fix plan:** merge
  both rebuilds into ONE migration directory (single transaction; the
  backfill's `receipts.assignment_type` dependency is satisfied — column
  exists since 2026-02-03). **Add:**
  `test_multi_invoice_migration_is_single_atomic_unit` (legacy-seeded DB →
  run pending → both tables rebuilt; exactly one new row in
  `__diesel_schema_migrations`).
- **[C7] Receipts have no applied-amount snapshot — editing a receipt's price
  after assignment corrupts the trip total on unassign.** Paperless links get
  an `amount_eur` snapshot but the receipt side stores only the
  `amount_applied` bool; Task 6 rule 4 subtracts the **live**
  `total_price_eur`, and `update_receipt_internal` is a raw passthrough
  ([receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs)
  lines 126–133). Assign 5.00 → edit to 7.00 → unassign subtracts 7.00 →
  total permanently off by 2.00 (clamping can silently floor to 0).
  **Fix design:** snapshot the applied amount (e.g. applied cents instead of a
  bool, mirroring paperless). **Add:**
  `test_unassign_after_receipt_price_edit_subtracts_originally_applied_amount`.

### New Coverage Gaps — Important

- **[I8] Datetime-warning/override loops only see the FIRST receipt per trip.**
  Both use `.find(|r| r.trip_id == Some(trip.id))`
  ([statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs)
  lines 1250, 1272) — with N receipts, an out-of-range second Other receipt is
  invisible; Task 7's planned single-receipt test would pass against the bug.
  **Add:** `test_datetime_warning_fires_for_second_other_receipt` + mirror for
  mismatch overrides.
- **[I9] Silent behavior change in the missing-invoice predicate unpinned.**
  Today `is_some()` ([statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs)
  line 1233); Task 7 switches to `> 0` — a trip with `Some(0.0)` flips from
  flagged to unflagged, no test uses zero. **Add:**
  `test_zero_value_costs_not_flagged_missing` (pin intended semantics).
- **[I10] Unassigning a receipt whose trip was deleted must not fail.**
  `delete_trip` leaves `receipts.trip_id` orphaned; naive
  `ok_or("Trip not found")?` in the new unassign logic makes orphans
  permanently un-unassignable. **Add:**
  `test_unassign_orphaned_receipt_succeeds_without_trip`. (Coverage/mismatch
  warnings key on live trips only — no test needed there.)
- **[I11] Task 9 integration spec is not executable as written.** There is
  **no `seedReceipt` helper**
  ([tests/integration/utils/db.ts](../../tests/integration/utils/db.ts) has
  only vehicle/trip/settings seeders; the `seedReceipt` in
  [tests/integration/README.md](../../tests/integration/README.md) line 167 is
  stale). Receipts are only creatable via the Docker-skipped scan+mock-Gemini
  flow with two mock files, neither suitable for two distinct Other receipts.
  **Fix plan:** add Other-cost mock JSONs (accepting Docker-skip) or a real
  `seedReceipt` helper. Spec content itself is adequate.
- **[I12] Assign is not idempotent — re-assign double-adds.**
  `assign_receipt_to_trip_internal` never checks `receipt.trip_id`; Task 6
  pseudo-code has no already-assigned pre-check; command reachable without UI
  gating via the HTTP RPC server. Same-doc-same-trip paperless re-upsert adds
  again; type change Fuel→Other adds without reversing. **Add:**
  `test_assign_same_receipt_same_trip_twice_adds_once`,
  `test_paperless_reupsert_same_trip_does_not_double_add`,
  `test_reassign_receipt_with_new_type_reverses_old_contribution`.

### New Coverage Gaps — Minor

- `test_unassign_fuel_receipt_never_touches_other_costs` — proves Fuel unassign
  keys on assignment type, not just `amount_applied`.
- No `busy_timeout`/WAL anywhere; table rebuilds lengthen the exclusive-lock
  window — an external connection (DB browser, query-sqlite-db skill) during
  upgrade → `SQLITE_BUSY` → startup panic. Document or set `busy_timeout`;
  no test proposed.

### Explicit no-new-gap areas

DROP/RENAME referencing objects (none exist); in-process concurrent access
during migration (single Mutex'd connection); statistics year/vehicle
filtering; NULL-amount-only trips (covered by I5); backup info inspection
(schema-agnostic COUNT queries).

## Iteration 3

Verification notes: `get_trip_ids_with_invoice` has exactly one production
caller ([statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs)
line 444, handled by Task 7). [export.rs](../../src-tauri/core/src/export.rs)
reads trip fields only and formats money with `{:.2}` — no float-noise leak.
Both assign entry points (desktop command and
[dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs)
lines 128–138) fetch the Paperless doc backend-side — Task 6 rule 5 holds on
the HTTP path too.

### New Coverage Gaps — Critical

- **[C8] The invoice-picker compatibility check false-flags every 2nd+ Other
  invoice — the feature's headline flow — and no plan task touches it.**
  `check_invoice_trip_compatibility`'s Other branch
  ([invoice.rs](../../src-tauri/core/src/invoice.rs) lines 142–176) compares
  the invoice amount against the **whole** `trip.other_costs_eur` with ±0.01.
  Once a trip carries one applied Other (5.00), attaching the next (7.50)
  returns `status="differs"` →
  [TripSelectorModal.svelte](../../src/lib/components/TripSelectorModal.svelte)
  (lines 242–292) shows ⚠ and pushes the mismatch-confirm flow, setting
  `mismatch_override=true`, which Task 7 then surfaces permanently. Also the
  picker's ±0.01 and Task 6's cent-exact guard disagree on borderline values
  (12.34 vs 12.335: picker "matches", backend adds → 24.68). `invoice.rs` /
  `invoice_tests.rs` appear in no task's file list. **Fix plan:** redefine the
  Other branch under multi-invoice (compare against remainder, or `Matches`
  when coverage shows existing Others). **Add:**
  `test_compatibility_second_other_invoice_not_flagged_as_price_mismatch`,
  `test_compatibility_other_uses_cent_exact_not_epsilon`.

### New Coverage Gaps — Important

- **[I13] A whole breaking test file is unlisted + two paperless DB APIs with
  unspecified fate.**
  [invoices_tests.rs](../../src-tauri/core/src/commands_internal/invoices_tests.rs)
  calls the old `upsert_paperless_link(&trip_id, 435)` signature (line 222)
  and `get_paperless_link_for_doc` (lines 156, 224) — Task 5's caller-grep
  only hunts `get_paperless_link_for_trip`. Fate of
  `get_paperless_link_for_doc` ([db.rs](../../src-tauri/core/src/db.rs)
  line 894) and `list_paperless_links_for_docs` (line 914; production caller
  [paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs)
  line 75) unstated. **Fix plan:** add `invoices_tests.rs` to Task 5/6 lists;
  enumerate the paperless DB API end-state.
- **[I14] Unassign-after-manual-overwrite silently destroys the manual
  value.** Other 5.01 applied (total 15.01) → user hand-edits to 3.00 (Req 4)
  → unassign → `money_sub(3.00, 5.01)` clamps to `None`, erasing 3.00. Not
  covered by C7 (price edit) or I6 (exact zero). **Add:**
  `test_unassign_applied_other_after_manual_overwrite` pinning intended
  semantics (+ grid `missing_other_invoices` follow-through if clamp-to-None
  is intended).
- **[I15] Task 1's files don't exist** — actual layout is
  `src-tauri/core/src/calculations/{mod.rs, tests.rs, …}` (no
  `calculations.rs` / `calculations_tests.rs`; stale names come from
  [.claude/rules/rust-backend.md](../../.claude/rules/rust-backend.md)). An
  executor following Task 1 verbatim creates orphan files that never compile.
  **Fix plan:** target `calculations/mod.rs` + `calculations/tests.rs`;
  `to_cents` must be `pub`-exported (Tasks 5 and 7 call it).

### New Coverage Gaps — Minor

- Task 10's doc-sweep grep provably misses the actual stale docs:
  [paperless-integration.md](../../docs/features/paperless-integration.md)
  lines 91–94 (documents the old PK/UNIQUE shape + ADR-019 symmetry) and
  [unified-invoice-picker.md](../../docs/features/unified-invoice-picker.md)
  line 10 (match-indicator semantics C8 changes). List both + an ADR-019
  supersede note explicitly in Task 10.
- Integration fixture type goes stale:
  [tests/integration/fixtures/types.ts](../../tests/integration/fixtures/types.ts)
  line 187 mirrors `missingReceipts`; update alongside
  [src/lib/types.ts](../../src/lib/types.ts) lines 158–160.

### Explicit no-new-gap areas

Export & reporting (trip-fields-only, 2-dp formatting); frontend/TS contract
beyond C8/fixture-type (all consumers in Task 8 scope); suggestions/HA/other
modules (zero receipt/`other_costs_eur` reads, no caching); Requirement 4
"not prevented" half (covered by Task 9 step 4).
