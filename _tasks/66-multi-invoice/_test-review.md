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
