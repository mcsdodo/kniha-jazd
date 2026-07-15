# Multi-Invoice Support — Design

**Date:** 2026-07-13
**Status:** Approved (brainstorming session 2026-07-13)

Model: **trip stays authoritative, invoices are proof; 1 Fuel + N Other
invoices per trip; sum-on-assign with manual overwrite + mismatch indicator.**

## 1. Data model & migrations

Two table rebuilds (SQLite cannot drop column constraints in place — use the
create-new → copy → drop-old → rename dance, see
[migrations.md](../../.claude/rules/migrations.md)).

**Both rebuilds live in ONE migration directory** (single `up.sql`, single
transaction). Diesel applies each migration directory in its own transaction
with no outer transaction; two directories would mean the receipts rebuild can
commit while the paperless rebuild fails, leaving a permanently half-migrated
DB that panics on every launch (`Database::new(...).expect(...)` runs before
any backup or compatibility check). One directory makes the whole feature
migration atomic. (Test review finding C6.)

**Rebuild DDL rules** (test review findings C3, iteration-4 minor):

- Recreate the CURRENT live index set: `idx_receipts_status`,
  `idx_receipts_trip`, `idx_receipts_vehicle`, **`idx_receipts_datetime`** —
  NOT the baseline's `idx_receipts_date`, which was dropped by migration
  `2026-02-01-100000_replace_receipt_date_with_datetime`.
- `mismatch_override` is nullable in the live lineage (added via
  `ALTER TABLE … ADD COLUMN mismatch_override INTEGER DEFAULT 0`). The rebuilt
  column stays `NOT NULL DEFAULT 0` (fixes schema.rs drift), but the copy
  SELECT must use `COALESCE(mismatch_override, 0)` so a hand-edited/restored
  DB with a NULL cannot abort the migration.
- Keep baseline column DEFAULTs (`status TEXT NOT NULL DEFAULT 'Pending'`,
  confidence JSON default) for historical DDL parity.

### `receipts`

Drop `trip_id TEXT UNIQUE` (from the
[baseline migration](../../src-tauri/core/migrations/2026-01-08-095218-0000_baseline/up.sql)),
recreate with plain `trip_id TEXT`, then enforce "one Fuel per trip" with a
partial unique index:

```sql
CREATE UNIQUE INDEX idx_receipts_trip_fuel ON receipts(trip_id)
WHERE trip_id IS NOT NULL AND assignment_type = 'Fuel';
```

Other receipts: unlimited per trip. DB values are `'Fuel'` / `'Other'` per
`AssignmentType::as_str` in [models.rs](../../src-tauri/core/src/models.rs).
Recreate the CURRENT live indexes (`idx_receipts_status/trip/vehicle/datetime`
— see the rebuild DDL rules above) after the rebuild.

### `paperless_trip_links`

Currently `trip_id TEXT PRIMARY KEY` + `paperless_document_id UNIQUE`, no type,
no amounts (see
[up.sql](../../src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/up.sql)).
Rebuild as:

```sql
CREATE TABLE paperless_trip_links (
    paperless_document_id INTEGER PRIMARY KEY,  -- one trip per doc (unchanged semantics)
    trip_id TEXT NOT NULL REFERENCES trips(id) ON DELETE CASCADE,
    assignment_type TEXT NOT NULL,              -- 'Fuel' | 'Other'
    amount_eur REAL,                            -- snapshot taken at assign time
    title TEXT,                                 -- snapshot for notes/tooltips
    applied_amount_cents INTEGER,               -- what was added to the trip (NULL = link-only)
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_paperless_links_trip ON paperless_trip_links(trip_id);
CREATE UNIQUE INDEX idx_paperless_links_trip_fuel ON paperless_trip_links(trip_id)
WHERE assignment_type = 'Fuel';
```

The **amount snapshot is load-bearing**: the grid's "sum of Other invoices vs
trip total" check must work offline — `get_trip_grid_data` cannot call the
Paperless server.

**Backfill for existing links** (no type/amount available offline):
`assignment_type = 'Fuel'` if the linked trip has `fuel_liters > 0` AND no Fuel
receipt already attached, else `'Other'`; `amount_eur = NULL` (unknown →
excluded from the sum check, so no false warnings). This is a heuristic —
existing datasets are small enough to eyeball after migration.

## 2. Money arithmetic — no floating-point errors (HARD requirement)

All EUR add/subtract goes through integer-cent helpers in
[calculations/mod.rs](../../src-tauri/core/src/calculations/mod.rs) (the
`calculations` module is a directory; `to_cents` must be `pub` — the DB layer
and statistics call it):

```rust
/// Exact money math: convert to integer cents, operate, convert back.
/// Guarantees repeated assign/unassign cycles return the exact original value.
fn to_cents(eur: f64) -> i64 { (eur * 100.0).round() as i64 }
fn from_cents(c: i64) -> f64 { c as f64 / 100.0 }
pub fn money_add(a: f64, b: f64) -> f64 { from_cents(to_cents(a) + to_cents(b)) }
pub fn money_sub(a: f64, b: f64) -> f64 { from_cents(to_cents(a) - to_cents(b)) }
```

Rules:
- Every mutation of `other_costs_eur` (assign-add, unassign-subtract) uses
  these helpers — never raw `+`/`-` on `f64`.
- Subtract flooring: result `<= 0` cents → store `None` (not `Some(0.0)`,
  not a tiny negative).
- Sum-mismatch comparison is done **in cents** (`to_cents(total) !=
  to_cents(sum)`), replacing `±0.01` epsilon comparisons for this feature.
- Unit tests must prove: N assigns followed by N unassigns in any order
  restores the exact starting value, including classic float traps
  (`0.1 + 0.2`, `19.99 + 5.01`, long chains).

## 3. Assignment semantics (backend)

In [invoices.rs](../../src-tauri/core/src/commands_internal/invoices.rs) /
[receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs):

- **Fuel:** max 1 per trip **across both sources**.
  `assign_invoice_to_trip_internal` rejects with a clear error if the trip
  already has a Fuel invoice in *either* store (the DB indexes only guard
  within each table). Populate-if-empty behavior unchanged.
- **Other — sum-on-assign:** assigning adds `amount` to `other_costs_eur`
  (via `money_add`) and appends the note; unassigning subtracts the linked
  invoice's amount (receipt: `total_price_eur`; paperless: `amount_eur`
  snapshot; `NULL` snapshot → subtract nothing).
- **`applied_amount_cents` snapshot (per link, replaces the earlier
  `amount_applied` bool — test review finding C7):** stored on the receipt row
  AND the paperless link as `applied_amount_cents INTEGER` (`NULL` = nothing
  was added to the trip). Set to the invoice amount in cents only when the
  assignment actually added its amount. Unassign subtracts **exactly
  `applied_amount_cents`** — never the live `total_price_eur`, which the user
  can edit after assignment (subtracting the live value would corrupt the trip
  total; the sum-mismatch indicator, which DOES use live values, is what
  surfaces such edits). A link-only assignment (double-count guard below) has
  `NULL` and must not subtract on unassign. Existing rows backfill to `NULL`
  (conservative: unassigning legacy links never mutates trip totals, matching
  today's behavior).
- **Assign is idempotent / no double-add (finding I12):** assigning an
  invoice that is already assigned to the SAME trip is a no-op (no second
  add). Re-assigning to a different trip, or re-assigning with a different
  `assignment_type`, first reverses the old contribution
  (`applied_amount_cents` on the old link) — this applies to BOTH sources,
  receipts included (finding C4), not just paperless.
- **Unassign tolerates orphans (finding I10):** `delete_trip` does not null
  `receipts.trip_id`, so orphaned assigned receipts exist in production.
  Unassigning a receipt whose trip no longer exists must clear the link
  without error (skip the subtract — there is no trip to mutate).
- **Unassign after manual overwrite (finding I14, pinned semantics):**
  unassign always subtracts `applied_amount_cents` via `money_sub` (clamped at
  0 → stored as `None`). If the user manually lowered the total below the
  applied amount, the result is `None` and the grid's missing/mismatch
  indicators surface the state; we do NOT try to preserve the overwritten
  value (the invoice's contribution is being explicitly removed, and the
  divergence was already flagged as a sum-mismatch before the unassign).
- **NULL-amount invoices (both sources — finding I3):** an invoice with no
  amount (`total_price_eur` NULL on receipts, `amount_eur` NULL on paperless)
  is always link-only (`applied_amount_cents = NULL`) and sets the trip's
  `has_unknown_amount`, excluding it from the sum-mismatch check.
- **Double-count guard:** if the trip has **zero Other invoices** and
  `other_costs_eur` already equals the invoice amount (cent-exact),
  assignment is link-only (`amount_applied = false`) — today's "manually
  pre-entered, now attaching proof" case.
- **Manual overwrite stays allowed** — divergence is surfaced (section 4),
  never prevented.
- `upsert_paperless_link` → keyed on `paperless_document_id`; stores
  `assignment_type` + `amount_eur`/`title` snapshots. Reassigning a doc to a
  different trip first reverses its sum contribution on the old trip.
- Unassign is already per-invoice on both paths — no interface change.

## 4. Indicators — per-type, replacing per-trip

`get_trip_ids_with_invoice` in [db.rs](../../src-tauri/core/src/db.rs)
(flat `HashSet<String>`) becomes per-type coverage:
`trip_id → { has_fuel_invoice, has_other_invoice, other_invoice_sum_cents }`.
`calculate_missing_receipts` in
[statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs)
flags per type. Per trip row:

| Indicator | Condition |
|---|---|
| Fuel column ⚠ missing | trip has fuel cost, no Fuel invoice (today's rule, type-scoped) |
| Other column ⚠ missing | trip has other costs, zero Other invoices |
| Other column ⚠ sum-mismatch (NEW) | `other_costs_eur` ≠ sum of attached Other amounts (cent-exact) — tooltip shows both numbers. Trips with any NULL-amount snapshot are **excluded from this check entirely** (sum is unknowable; partial sums would produce false warnings) |
| Datetime-mismatch ⚠ | type-scoped the same way |

All computed in Rust inside `get_trip_grid_data` (ADR-008); frontend renders
flags only.

**Implementation constraints from the test review:**

- `calculate_receipt_datetime_warnings` and
  `calculate_receipt_mismatch_overrides` currently use
  `.find(|r| r.trip_id == Some(trip.id))` — first receipt per trip only.
  Under multi-invoice they MUST iterate **all** receipts of the trip (finding
  I8), else an out-of-range second Other receipt is invisible.
- The missing-invoice predicate changes from `is_some()` to `> 0`: a trip
  with `other_costs_eur = Some(0.0)` or `fuel_liters = Some(0.0)` is
  intentionally NOT flagged as missing an invoice (finding I9 — pinned by a
  dedicated test).

### Trip-picker compatibility check — redefined (finding C8)

`check_invoice_trip_compatibility`'s Other branch today compares the invoice
amount against the **whole** `trip.other_costs_eur` with ±0.01 — under
sum-on-assign, every 2nd Other invoice would false-flag as a price mismatch
and push users through the mismatch-confirm flow (permanently surfacing
`other_mismatch_overrides`). Redefined:

- **Other invoices:** if the trip already has ≥1 Other invoice attached,
  amount comparison is skipped entirely → `Matches` (the new invoice will be
  summed on assign; there is nothing to "match" against). If the trip has
  zero Other invoices, compare **cent-exact** (`to_cents`) against
  `other_costs_eur` — aligned with the double-count guard, replacing ±0.01,
  so picker verdict and assign behavior can never disagree on borderline
  values (e.g. 12.34 vs 12.3345 — epsilon says equal, cent-exact says not).
- **Fuel invoices:** trips that already have a Fuel invoice (either source)
  return `can_attach = false` (finding I1) so the picker greys them out
  instead of erroring after selection. The backend assign pre-check remains
  authoritative.

## 5. Frontend

- [TripRow.svelte](../../src/lib/components/TripRow.svelte): split
  `hasMatchingReceipt` into `hasMatchingFuelInvoice` /
  `hasMatchingOtherInvoice`, add `otherCostsSumMismatch`; each column warns
  independently (today one boolean drives both the fuel column and the
  other-costs column).
- [doklady/+page.svelte](../../src/routes/doklady/+page.svelte): assigned rows
  unchanged (unassign already per-invoice); trip-picker modal drops any
  "trip already has an invoice" special-casing for Other.
- New Slovak i18n strings (missing-per-type + sum-mismatch tooltip) via
  [i18n](../../src/lib/i18n/).

## 6. Testing

TDD backend-first
([commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs),
[db_tests.rs](../../src-tauri/core/src/db_tests.rs),
[invoice_tests.rs](../../src-tauri/core/src/invoice_tests.rs),
[calculations/tests.rs](../../src-tauri/core/src/calculations/tests.rs) —
note: `calculations` is a module directory, there is no `calculations.rs` /
`calculations_tests.rs`):

- money helpers: exact round-trips, float traps, flooring to `None`
- **migration data integrity (findings C1/C2/C5/C6):** a test harness that
  replays embedded migrations up to (excluding) the multi-invoice migration,
  seeds legacy rows via raw SQL, runs the rest, and asserts: every
  pre-existing row/column value survives byte-for-byte; indexes recreated
  correctly; backfill heuristic per edge case (`fuel_liters` NULL/0/positive,
  Fuel receipt present/absent); NULL `mismatch_override` tolerated; the
  migration is one atomic unit; fresh-vs-migrated `sqlite_master` schemas
  identical; migrated data produces no false grid warnings
- assign Fuel + Other to same trip succeeds (both sources)
- second Fuel rejected: receipt→receipt, receipt→paperless, paperless→receipt
- N Other invoices: sum-on-assign, unassign-subtract, double-count guard,
  reassign-to-other-trip reverses contribution (both sources), idempotent
  re-assign, orphaned-trip unassign, unassign-after-manual-overwrite,
  unassign-after-price-edit subtracts the snapshot, NULL-amount link-only,
  Fuel unassign never touches other costs, non-finite/negative amounts
  rejected at assign
- per-type coverage sets + sum-mismatch edge cases (NULL snapshots, manual
  overwrite, zero-value costs, second receipt datetime warnings)
- compatibility check: second Other not false-flagged; cent-exact agreement
  with the double-count guard; Fuel-covered trips `can_attach = false`
- existing single-invoice tests updated (not deleted) to the new rules — in
  BOTH [db_tests.rs](../../src-tauri/core/src/db_tests.rs) AND
  [invoices_tests.rs](../../src-tauri/core/src/commands_internal/invoices_tests.rs)
- ONE integration spec: assign two Others via UI → grid shows summed total;
  hand-edit total → sum-mismatch ⚠ appears (requires a new `seedReceipt`
  helper — none exists today)

[export.rs](../../src-tauri/core/src/export.rs) needs **no change** — it reads
trip fields, which stay authoritative.

## 7. Documentation

- [docs/features/multi-invoice.md](../../docs/features/multi-invoice.md) (new):
  user flow per use-case (fuel+parking ride, multiple Others, manual overwrite
  + warning, double-count guard) + technical notes
- [CHANGELOG.md](../../CHANGELOG.md): user-visible entry
- [DECISIONS.md](../../DECISIONS.md): BIZ entry — sum-on-assign, cardinality
  (1 Fuel + N Other), cent-exact money math
- Sweep existing feature docs for stale "one receipt per trip" statements —
  known hits the generic grep misses (finding, iteration 3):
  [paperless-integration.md](../../docs/features/paperless-integration.md)
  (documents the old `trip_id PRIMARY KEY`/`UNIQUE` shape + ADR-019 symmetry)
  and
  [unified-invoice-picker.md](../../docs/features/unified-invoice-picker.md)
  (match-indicator semantics changed by the compatibility redefinition);
  DECISIONS.md needs an ADR-019 supersede note
- [README.md](../../README.md) / [README.en.md](../../README.en.md) sync if
  they mention invoice limits
