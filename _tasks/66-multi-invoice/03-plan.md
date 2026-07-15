# Multi-Invoice Support Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Date:** 2026-07-13 (revised 2026-07-15 after test review — see [_test-review.md](./_test-review.md))
**Subject:** 1 Fuel + N Other invoices per trip — implementation plan
**Status:** Planning

**Goal:** Allow a trip to carry one Fuel invoice plus any number of Other-cost invoices, with sum-on-assign into `trip.other_costs_eur`, cent-exact money math, and per-type grid warnings.

**Architecture:** Trip stays authoritative (invoices are attached proof; export untouched). ONE atomic SQLite migration rebuilds both tables (removes one-invoice-per-trip constraints); partial unique indexes enforce one-Fuel-per-trip within each store; backend logic enforces it across stores. All new logic in Rust (ADR-008). Design: [02-design.md](./02-design.md).

**Revision note (2026-07-15):** all findings from [_test-review.md](./_test-review.md) are folded in: single atomic migration (C6), corrected rebuild DDL (C3 + DDL-parity minor), `applied_amount_cents` snapshot replacing the `amount_applied` bool (C7), migration data-integrity test task (C1/C2/C5/I5), receipt-path reassignment reversal (C4), compatibility-check redefinition (C8/I1), assignment idempotency (I12), and all remaining Important/Minor items.

**Tech Stack:** Rust (Diesel/SQLite), SvelteKit + TypeScript, WebdriverIO.

**Conventions for the executor:**
- Run backend tests with: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "<filter>"`
- Tests live in `*_tests.rs` / `tests.rs` companion files (`#[path]` pattern), never in source files.
- Commit after each task (plan docs were committed before implementation started).
- DB string values for assignment type are `'Fuel'` / `'Other'` (see `AssignmentType::as_str` in [models.rs](../../src-tauri/core/src/models.rs)).
- The `calculations` module is a DIRECTORY: [calculations/mod.rs](../../src-tauri/core/src/calculations/mod.rs) with tests in [calculations/tests.rs](../../src-tauri/core/src/calculations/tests.rs) wired via `#[path]` at the bottom of mod.rs. There is no `calculations.rs` or `calculations_tests.rs`.

---

### Task 1: Cent-exact money helpers

**Files:**
- Modify: [src-tauri/core/src/calculations/mod.rs](../../src-tauri/core/src/calculations/mod.rs)
- Test: [src-tauri/core/src/calculations/tests.rs](../../src-tauri/core/src/calculations/tests.rs)

**Step 1: Write failing tests** (append to `calculations/tests.rs`):

```rust
// ===== Money math (Task 66: multi-invoice, cent-exact) =====

#[test]
fn test_money_add_float_traps() {
    assert_eq!(money_add(0.1, 0.2), 0.3);
    assert_eq!(money_add(19.99, 5.01), 25.0);
    assert_eq!(money_add(0.0, 12.34), 12.34);
}

#[test]
fn test_money_sub_exact() {
    assert_eq!(money_sub(0.3, 0.1), 0.2);
    assert_eq!(money_sub(25.0, 19.99), 5.01);
}

#[test]
fn test_money_long_chain_roundtrip() {
    // N assigns followed by N unassigns in any order restores the EXACT start.
    let amounts = [4.5, 12.99, 0.01, 33.33, 7.77, 100.0, 0.1, 0.2];
    let start = 55.55_f64;
    let mut total = start;
    for a in amounts { total = money_add(total, a); }
    // subtract in different order
    for a in amounts.iter().rev() { total = money_sub(total, *a); }
    assert_eq!(total, start); // bit-exact, no epsilon
}

#[test]
fn test_money_sub_floors_to_zero() {
    // subtracting more than the total must clamp at 0.0, never go negative
    assert_eq!(money_sub(5.0, 5.01), 0.0);
    assert_eq!(money_sub(0.0, 1.0), 0.0);
}

#[test]
fn test_to_cents_rounds_binary_representation_traps() {
    // 12.345 is stored as 12.34499999… in f64 — the trap is binary
    // representation, not banker's rounding (f64::round is half-away already).
    assert_eq!(to_cents(12.345), 1235);
    assert_eq!(to_cents(12.344999), 1234);
    assert_eq!(to_cents(1.005), 101);   // classic 1.005*100 = 100.49999 trap
}
```

If `tests.rs` doesn't already import the helpers, extend its `use super::*;` scope (helpers live in `mod.rs`, so `use super::*;` covers them).

**Step 2: Run — expect FAIL** (functions don't exist):
`cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "money"`

**Step 3: Implement** in `calculations/mod.rs` (all `pub` — the DB layer, statistics, and invoice compatibility call them):

```rust
// ===== Cent-exact money math (Task 66: multi-invoice) =====
// HARD requirement: repeated assign/unassign cycles on other_costs_eur must be
// bit-exact. All EUR add/subtract goes through integer cents — never raw f64 +/-.

/// EUR → integer cents. Uses an epsilon nudge before rounding so values that
/// are conceptually exact 2-dp money (but stored as f64 like 1.005 → 100.4999…)
/// round to the intended cent.
pub fn to_cents(eur: f64) -> i64 {
    (eur * 100.0 + if eur >= 0.0 { 1e-6 } else { -1e-6 }).round() as i64
}

pub fn from_cents(cents: i64) -> f64 {
    cents as f64 / 100.0
}

/// Exact addition of two EUR amounts.
pub fn money_add(a: f64, b: f64) -> f64 {
    from_cents(to_cents(a) + to_cents(b))
}

/// Exact subtraction, clamped at 0.0 (money on a trip can never go negative).
pub fn money_sub(a: f64, b: f64) -> f64 {
    from_cents((to_cents(a) - to_cents(b)).max(0))
}
```

**Step 4: Run — expect PASS** (same command).

**Step 5: Commit**
```bash
git add src-tauri/core/src/calculations/mod.rs src-tauri/core/src/calculations/tests.rs
git commit -m "feat(calculations): cent-exact money helpers for multi-invoice sums"
```

---

### Task 2: ONE atomic migration — rebuild `receipts` + `paperless_trip_links`

> **Why one directory (test review C6):** Diesel runs each migration directory
> in its own transaction with no outer transaction. Two directories would let
> the receipts rebuild commit while the paperless rebuild fails, leaving a
> half-migrated production DB that panics on every launch (`Database::new`
> `.expect()` fires before any backup or compatibility check). One directory =
> one transaction = atomic.

**Files:**
- Create: `src-tauri/core/migrations/2026-07-15-100000_multi_invoice/up.sql`
- Create: `src-tauri/core/migrations/2026-07-15-100000_multi_invoice/down.sql`
- Modify: [src-tauri/core/src/schema.rs](../../src-tauri/core/src/schema.rs) (receipts block ~line 5; paperless_trip_links block ~line 108)

**Step 1:** Verify current live DDL (source of truth):
```bash
sqlite3 "$APPDATA/com.notavailable.kniha-jazd.dev/kniha-jazd.db" "SELECT sql FROM sqlite_master WHERE tbl_name IN ('receipts','paperless_trip_links');"
```
The rebuild below reproduces the live schema (baseline + all ALTERs, column order as SQLite appends them) EXCEPT the constraints being removed. Live receipts indexes today are `idx_receipts_status`, `idx_receipts_trip`, `idx_receipts_vehicle`, **`idx_receipts_datetime`** (`idx_receipts_date` was dropped by `2026-02-01-100000_replace_receipt_date_with_datetime` — do NOT recreate the dead name; test review C3a). If live DDL differs from this plan, follow live DDL.

**Step 2: Write `up.sql`:**

```sql
-- Multi-invoice (Task 66): allow 1 Fuel + N Other invoices per trip.
-- BOTH rebuilds live in this single migration so the change is atomic —
-- a partial failure must roll back everything (see _tasks/66 test review C6).
--
-- NOTE: table rebuilds hold an exclusive lock for the whole copy. An external
-- connection open during the upgrade (DB browser, sqlite3 CLI) can cause
-- SQLITE_BUSY and abort startup — close external tools before updating.

-- ============================================================
-- Part 1: receipts — drop trip_id UNIQUE, add applied_amount_cents
-- ============================================================
-- applied_amount_cents: the exact amount (in cents) this receipt added to
-- trip.other_costs_eur at assign time. NULL = nothing applied (link-only or
-- legacy). Unassign subtracts THIS value, never the live total_price_eur,
-- which the user may edit after assigning (test review C7).
CREATE TABLE receipts_new (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT,
    trip_id TEXT,
    file_path TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    scanned_at TEXT NOT NULL,
    liters REAL,
    total_price_eur REAL,
    station_name TEXT,
    station_address TEXT,
    source_year INTEGER,
    status TEXT NOT NULL DEFAULT 'Pending',
    confidence TEXT NOT NULL DEFAULT '{"liters":"Unknown","totalPrice":"Unknown","date":"Unknown"}',
    raw_ocr_text TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    vendor_name TEXT,
    cost_description TEXT,
    original_amount REAL DEFAULT NULL,
    original_currency TEXT DEFAULT NULL,
    receipt_datetime TEXT DEFAULT NULL,
    assignment_type TEXT DEFAULT NULL,
    mismatch_override INTEGER NOT NULL DEFAULT 0,
    applied_amount_cents INTEGER,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id),
    FOREIGN KEY (trip_id) REFERENCES trips(id)
);

-- COALESCE on mismatch_override: the live column is nullable (added via
-- ALTER ... DEFAULT 0 without NOT NULL); a hand-edited/restored DB with a
-- NULL must not abort the migration (test review C3b). The rebuilt column is
-- NOT NULL, which also fixes the schema.rs drift.
INSERT INTO receipts_new (
    id, vehicle_id, trip_id, file_path, file_name, scanned_at, liters,
    total_price_eur, station_name, station_address, source_year, status,
    confidence, raw_ocr_text, error_message, created_at, updated_at,
    vendor_name, cost_description, original_amount, original_currency,
    receipt_datetime, assignment_type, mismatch_override, applied_amount_cents
)
SELECT
    id, vehicle_id, trip_id, file_path, file_name, scanned_at, liters,
    total_price_eur, station_name, station_address, source_year, status,
    confidence, raw_ocr_text, error_message, created_at, updated_at,
    vendor_name, cost_description, original_amount, original_currency,
    receipt_datetime, assignment_type, COALESCE(mismatch_override, 0),
    NULL  -- legacy assignments never subtract on unassign (today's behavior)
FROM receipts;

DROP TABLE receipts;
ALTER TABLE receipts_new RENAME TO receipts;

CREATE INDEX idx_receipts_status ON receipts(status);
CREATE INDEX idx_receipts_trip ON receipts(trip_id);
CREATE INDEX idx_receipts_vehicle ON receipts(vehicle_id);
CREATE INDEX idx_receipts_datetime ON receipts(receipt_datetime);
-- One Fuel receipt per trip; Other receipts unlimited.
CREATE UNIQUE INDEX idx_receipts_trip_fuel ON receipts(trip_id)
WHERE trip_id IS NOT NULL AND assignment_type = 'Fuel';

-- ============================================================
-- Part 2: paperless_trip_links — new PK, type + amount snapshots
-- ============================================================
-- New PK: paperless_document_id (one trip per doc — unchanged semantics).
-- assignment_type + amount_eur/title snapshots taken at assign time so the
-- grid's sum-mismatch check works offline (grid never calls Paperless).
CREATE TABLE paperless_trip_links_new (
    paperless_document_id INTEGER PRIMARY KEY,
    trip_id TEXT NOT NULL,
    assignment_type TEXT NOT NULL,
    amount_eur REAL,
    title TEXT,
    applied_amount_cents INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (trip_id) REFERENCES trips(id) ON DELETE CASCADE
);

-- Backfill heuristic (docs' data lives on the Paperless server, not locally):
-- 'Fuel' if the linked trip has fuel and no Fuel receipt already attached,
-- else 'Other'. NULL fuel_liters compares falsy -> 'Other'. amount_eur stays
-- NULL = unknown -> that trip is excluded from the sum-mismatch check (no
-- false warnings). applied_amount_cents = NULL (legacy links never subtract
-- on unassign). NOTE: this runs AFTER Part 1, reading the rebuilt receipts
-- table — same transaction, same data.
INSERT INTO paperless_trip_links_new (
    paperless_document_id, trip_id, assignment_type, amount_eur, title,
    applied_amount_cents, created_at, updated_at
)
SELECT
    l.paperless_document_id,
    l.trip_id,
    CASE WHEN EXISTS (
            SELECT 1 FROM trips t
            WHERE t.id = l.trip_id AND t.fuel_liters > 0
         )
         AND NOT EXISTS (
            SELECT 1 FROM receipts r
            WHERE r.trip_id = l.trip_id AND r.assignment_type = 'Fuel'
         )
    THEN 'Fuel' ELSE 'Other' END,
    NULL,
    NULL,
    NULL,
    l.created_at,
    l.updated_at
FROM paperless_trip_links l;

DROP TABLE paperless_trip_links;
ALTER TABLE paperless_trip_links_new RENAME TO paperless_trip_links;

CREATE INDEX idx_paperless_links_trip ON paperless_trip_links(trip_id);
CREATE UNIQUE INDEX idx_paperless_links_trip_fuel ON paperless_trip_links(trip_id)
WHERE assignment_type = 'Fuel';
```

**Step 3: Write `down.sql`** — reverse both rebuilds (paperless first, then receipts):

```sql
-- ============================================================
-- WARNING — LOSSY, NEVER EXECUTED IN PRACTICE (forward-only, ADR-012;
-- no diesel CLI revert exists in this repo or CI).
-- * paperless: collapsing to trip_id PRIMARY KEY keeps ONE arbitrary link
--   per trip (MIN(doc_id)) and discards the rest + all snapshots.
-- * receipts: restoring trip_id UNIQUE FAILS OUTRIGHT if any trip holds
--   more than one receipt; applied_amount_cents is discarded.
-- ============================================================
```
…followed by the two reverse rebuilds: paperless back to `trip_id TEXT PRIMARY KEY, paperless_document_id INTEGER NOT NULL UNIQUE, created_at, updated_at` + `idx_paperless_links_doc` (use `GROUP BY trip_id` with `MIN(paperless_document_id)`), receipts back to `trip_id TEXT UNIQUE` without `applied_amount_cents` (recreate the four live indexes, no partial index).

**Step 4: Update `schema.rs`:**

receipts block — after `mismatch_override`:
```rust
        // Added via migration 2026-07-15-100000_multi_invoice
        applied_amount_cents -> Nullable<BigInt>,
```
(`mismatch_override -> Integer` stays — the rebuild makes the column genuinely NOT NULL, fixing the previous drift.)

paperless_trip_links block — replace entirely:
```rust
// Rebuilt via migration 2026-07-15-100000_multi_invoice (Task 66)
diesel::table! {
    paperless_trip_links (paperless_document_id) {
        paperless_document_id -> BigInt,
        trip_id -> Text,
        assignment_type -> Text,
        amount_eur -> Nullable<Double>,
        title -> Nullable<Text>,
        applied_amount_cents -> Nullable<BigInt>,
        created_at -> Text,
        updated_at -> Text,
    }
}
```

**Step 5: Run backend tests — expect COMPILE FAILURES** in Receipt/paperless row mapping (struct/schema mismatch). That's the trigger for Task 3. Do NOT commit yet — Task 3 makes the build green; commit there.

---

### Task 3: Model + DB plumbing (receipts snapshot, paperless links CRUD, per-type coverage)

**Files:**
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) — `Receipt` (~line 505), `ReceiptRow` (~line 893), `NewReceiptRow` (~line 920), `From<ReceiptRow>` (~line 1116), any `Receipt { ... }` literal in test helpers; new `PaperlessLink` + `TripInvoiceCoverage` structs
- Modify: [src-tauri/core/src/db.rs](../../src-tauri/core/src/db.rs) — `insert_receipt` (~line 622), `update_receipt` (~line 700), `unassign_receipt` (~line 746), paperless link functions (~lines 858–958)
- Test: [src-tauri/core/src/db_tests.rs](../../src-tauri/core/src/db_tests.rs)

**Step 1: Write failing tests** (append to `db_tests.rs`; adapt helper names to those present):

```rust
#[test]
fn test_receipt_applied_amount_cents_roundtrip() {
    let (db, _dir) = test_db();
    let vehicle = insert_test_vehicle(&db);
    let mut receipt = make_test_receipt(&vehicle);
    receipt.applied_amount_cents = Some(501);
    db.insert_receipt(&receipt).unwrap();
    let loaded = db.get_receipt_by_id(&receipt.id.to_string()).unwrap().unwrap();
    assert_eq!(loaded.applied_amount_cents, Some(501));

    // unassign clears it
    db.unassign_receipt(&receipt.id.to_string()).unwrap();
    let loaded = db.get_receipt_by_id(&receipt.id.to_string()).unwrap().unwrap();
    assert_eq!(loaded.applied_amount_cents, None);
}

#[test]
fn test_two_other_links_one_trip_allowed() { /* upsert two Other docs on one trip; both persist */ }

#[test]
fn test_second_fuel_link_same_trip_rejected() { /* upsert 2nd Fuel doc on same trip -> Err (unique index) */ }

#[test]
fn test_upsert_link_reassign_moves_doc() { /* doc linked to trip A, upsert to trip B -> only B row remains */ }

#[test]
fn test_invoice_coverage_per_type_and_sum() {
    // trip with 1 Fuel receipt + 2 Other links (amounts 5.00, 7.50)
    // -> coverage: has_fuel=true, has_other=true, other_sum_cents=1250, has_unknown_amount=false
    // trip with 1 Other link amount NULL -> has_unknown_amount=true
    // trip with 1 Other RECEIPT with total_price_eur NULL -> has_unknown_amount=true (test review I3)
}
```

**Step 2: Run — expect FAIL/compile errors.**

**Step 3: Implement:**

`models.rs`:
- `Receipt`: add `pub applied_amount_cents: Option<i64>,` next to `mismatch_override`. Plumb through `ReceiptRow` / `NewReceiptRow` / `From<ReceiptRow>`. Fix every struct-literal compile error in tests with `applied_amount_cents: None`.
- New structs:
```rust
/// A paperless doc→trip link with assignment snapshots (Task 66).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperlessLink {
    pub paperless_document_id: i64,
    pub trip_id: String,
    pub assignment_type: AssignmentType,
    pub amount_eur: Option<f64>,
    pub title: Option<String>,
    /// Cents actually added to trip.other_costs_eur at assign time; None = link-only.
    pub applied_amount_cents: Option<i64>,
}

/// Per-trip invoice coverage for grid indicators (Task 66).
#[derive(Debug, Clone, Default)]
pub struct TripInvoiceCoverage {
    pub has_fuel: bool,
    pub has_other: bool,
    /// Sum of Other invoice amounts in integer cents (receipts' live
    /// total_price_eur + paperless amount_eur snapshots).
    pub other_sum_cents: i64,
    /// True if any Other invoice has an unknown (NULL) amount -> skip sum-mismatch check.
    pub has_unknown_amount: bool,
}
```

`db.rs`:
- `insert_receipt` / `update_receipt`: write `applied_amount_cents`; `unassign_receipt`: also `.set(receipts::applied_amount_cents.eq(None::<i64>))`.
- `upsert_paperless_link(&self, link: &PaperlessLink) -> QueryResult<()>` — keyed on `paperless_document_id` only (delete prior row for THIS doc, insert new). **Remove** the delete-by-trip_id statement (that's the single-invoice rule being removed).
- `get_paperless_link(&self, doc_id: i64) -> QueryResult<Option<PaperlessLink>>` — full row; **subsumes `get_paperless_link_for_doc`, which is deleted** (update its callers).
- `get_paperless_links_for_trip(&self, trip_id: &str) -> QueryResult<Vec<PaperlessLink>>` — replaces `get_paperless_link_for_trip` (deleted). Find all callers: `grep -rn "get_paperless_link_for_trip\|get_paperless_link_for_doc" src-tauri/`.
- `list_paperless_links_for_docs` (~line 914; production caller [paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs) ~line 75): **survives**, now returning the new columns — update its row mapping to `PaperlessLink`.
- Replace `get_trip_ids_with_invoice() -> HashSet<String>` with:
```rust
/// Per-type invoice coverage for every trip that has at least one invoice.
/// Union of local receipts and paperless links; sums use integer cents.
pub fn get_trip_invoice_coverage(&self) -> QueryResult<HashMap<String, TripInvoiceCoverage>>
```
  Loop over assigned receipts (`trip_id IS NOT NULL`) and all paperless links; per trip set `has_fuel`/`has_other`, accumulate `other_sum_cents += to_cents(amount)` for Other rows with known amounts, set `has_unknown_amount` when an Other amount is `None` (either source — receipts included, test review I3).

**Step 3b: Update the existing tests that break (update, NOT delete — test review I7/I13):**
- [db_tests.rs](../../src-tauri/core/src/db_tests.rs) `paperless_link_unique_trip_invariant` (~line 479) — the invariant INVERTS: two docs on one trip now both persist (rename accordingly).
- [db_tests.rs](../../src-tauri/core/src/db_tests.rs) `paperless_link_upsert_creates_then_replaces` (~line 430) — new `PaperlessLink`-struct signature; "replaces" now means same-doc re-upsert, not same-trip.
- [db_tests.rs](../../src-tauri/core/src/db_tests.rs) `get_trip_ids_with_invoice_unions_receipts_and_paperless` (~line 515) — same scenario, assert per-type flags on the new coverage API.
- [invoices_tests.rs](../../src-tauri/core/src/commands_internal/invoices_tests.rs) `assign_paperless_populates_trip_fuel_when_empty` (~line 222, old two-arg `upsert_paperless_link`) and `unassign_dispatches_paperless_source` (~lines 156/224, `get_paperless_link_for_doc`) — port to the new API, same behavioral assertions.

**Step 4: Run full backend suite — expect PASS:**
`cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core`
(`commands_internal` callers may need minimal stub-fixes to compile; real logic lands in Task 5.)

**Step 5: Commit** (Tasks 2+3 together — migration + plumbing):
```bash
git add src-tauri/core/migrations/2026-07-15-100000_multi_invoice/ src-tauri/core/src/schema.rs src-tauri/core/src/models.rs src-tauri/core/src/db.rs src-tauri/core/src/db_tests.rs src-tauri/core/src/commands_internal/invoices_tests.rs
git commit -m "feat(db): 1 Fuel + N Other invoices per trip; atomic rebuild migration with amount snapshots"
```

---

### Task 4: Migration data-integrity tests (test review C1/C2/C3b/C5/C6/I5)

> **This task IS the user's hard requirement** ("the data migration MUST NOT
> fuckup the data and be successful"). The repo has never had data-preserving
> migration tests — every existing test runs the full fresh chain on
> `Database::in_memory()`.

**Files:**
- Modify: [src-tauri/core/src/db.rs](../../src-tauri/core/src/db.rs) — add a `#[cfg(test)] pub(crate)` harness helper + `#[path = "migration_tests.rs"]` test module wiring (next to the existing `#[path = "db_tests.rs"]` at ~line 962)
- Create: `src-tauri/core/src/migration_tests.rs`

**Step 1: Implement the harness** (in `db.rs`, `#[cfg(test)]` so both `migration_tests.rs` and `commands_tests.rs` can use it):

```rust
#[cfg(test)]
pub(crate) const MULTI_INVOICE_VERSION: &str = "2026-07-15";

/// Test harness: open an in-memory DB migrated only UP TO (excluding) the
/// multi-invoice migration, so tests can seed legacy-shaped rows and then
/// run the remaining migrations against real data.
///
/// Implementation notes (verified against diesel_migrations 2.2):
/// - MigrationSource::migrations() order is NOT contractually sorted —
///   sort by name().version() before replaying.
/// - version() is the directory name up to the FIRST underscore
///   (e.g. "2026-01-09-100000-add"), so filter with a lexical
///   `version.to_string() < MULTI_INVOICE_VERSION` — never string-match
///   full directory names.
#[cfg(test)]
pub(crate) fn open_db_legacy() -> Database { /* replay filtered, sorted migrations via MigrationHarness::run_migration */ }

/// Run the remaining (multi-invoice) migrations on a legacy DB.
#[cfg(test)]
pub(crate) fn migrate_to_current(db: &Database) { /* run_pending_migrations */ }
```

**Step 2: Write the tests** (in `migration_tests.rs`; seed via `diesel::sql_query` raw SQL — the legacy schema has no Rust structs anymore):

```rust
#[test]
fn test_receipts_migration_preserves_every_row_and_column() {
    // Seed ~6 legacy receipts covering: assigned Fuel, assigned Other,
    // unassigned, all-optional-columns-NULL, orphaned trip_id (trip deleted),
    // non-ASCII text in raw_ocr_text. Snapshot all column values pre-migration
    // (SELECT * via sql_query into a raw-row struct), migrate, then assert
    // row count AND every column value byte-identical, and
    // applied_amount_cents IS NULL for all rows.
}

#[test]
fn test_paperless_links_migration_preserves_rows() {
    // doc ids, trip ids, created_at/updated_at preserved;
    // amount_eur IS NULL, title IS NULL, applied_amount_cents IS NULL.
}

#[test]
fn test_receipts_migration_recreates_indexes() {
    // Post-migration index set on receipts ==
    // {idx_receipts_status, idx_receipts_trip, idx_receipts_vehicle,
    //  idx_receipts_datetime, idx_receipts_trip_fuel}
    // — catches the dead-idx_receipts_date / dropped-datetime drift (C3a).
}

#[test]
fn test_receipts_migration_tolerates_null_mismatch_override() {
    // Seed one legacy receipt with mismatch_override = NULL (hand-edited DB).
    // Migration succeeds; value becomes 0. (C3b — without COALESCE this
    // aborts the whole migration and bricks startup.)
}

#[test]
fn test_backfill_fuel_when_trip_fueled_and_no_fuel_receipt() {
    // legacy link on trip with fuel_liters > 0, no Fuel receipt -> 'Fuel'
}

#[test]
fn test_backfill_other_when_fuel_receipt_already_attached() {
    // legacy link on trip with fuel_liters > 0 AND a Fuel receipt -> 'Other'
    // (also proves no cross-source double-Fuel state after upgrade)
}

#[test]
fn test_backfill_other_when_fuel_liters_null_or_zero() {
    // NULL and 0 both -> 'Other' (SQL NULL > 0 is falsy)
}

#[test]
fn test_multi_invoice_migration_is_single_atomic_unit() {
    // After migrate_to_current: BOTH tables rebuilt AND exactly ONE new row
    // in __diesel_schema_migrations (C6 — two directories would be two
    // transactions and a half-migrated crash-loop risk).
}

#[test]
fn test_migrated_schema_identical_to_fresh_schema() {
    // Normalize (lowercase, collapse whitespace) SELECT type, name, sql FROM
    // sqlite_master (tables + indexes, skip sqlite_* internals) for
    // (a) Database::in_memory() fresh chain, (b) legacy->migrated DB.
    // Assert identical sets (C5 — catches schema.rs/DDL drift forever).
}
```

Plus, in [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) (uses the same harness; needs statistics fns):

```rust
#[test]
fn test_migrated_db_produces_no_false_warnings() {
    // Legacy DB: trip with other_costs_eur = 12.50 + one legacy paperless
    // link (backfills to Other, amount NULL). After migration + coverage:
    // trip absent from other_sum_mismatches (unknown amount excluded) AND
    // absent from missing_other_invoices (has_other = true). (I5 — "don't
    // scare users after update".)
}
```

**Step 3: Run — expect PASS** (these run against the Task 2 SQL; any failure here is a REAL migration bug — fix the SQL, not the test).

**Step 4: Commit:**
```bash
git add src-tauri/core/src/db.rs src-tauri/core/src/migration_tests.rs src-tauri/core/src/commands_internal/commands_tests.rs
git commit -m "test(db): migration data-integrity suite for multi-invoice rebuild"
```

---

### Task 5: Assignment semantics — cross-source Fuel uniqueness, sum-on-assign, snapshots, idempotency

**Files:**
- Modify: [src-tauri/core/src/commands_internal/receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs) (`assign_receipt_to_trip_internal`, ~line 353)
- Modify: [src-tauri/core/src/commands_internal/invoices.rs](../../src-tauri/core/src/commands_internal/invoices.rs) (`assign_invoice_to_trip_internal` ~line 70, `unassign_invoice_internal`)
- Test: [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) (extend the Task-51 assignment block, ~line 1500+)

**Step 1: Write failing tests** (one per use-case; reuse `make_trip_for_assignment` / `make_receipt_with_datetime_assigned` helpers):

```rust
#[test]
fn test_assign_fuel_and_other_to_same_trip_succeeds() { /* fuel receipt then other receipt; both linked */ }

#[test]
fn test_second_fuel_receipt_same_trip_rejected() { /* clear error, first link intact */ }

#[test]
fn test_second_fuel_cross_source_rejected() {
    // fuel receipt assigned -> paperless Fuel assign rejected; and vice versa
}

#[test]
fn test_other_sum_on_assign_adds_exactly() {
    // trip other_costs 10.00, assign second Other 5.01 -> 15.01 (cent-exact, bit-equal)
}

#[test]
fn test_other_unassign_subtracts_only_if_applied() {
    // invoice that added 5.01 -> unassign -> back to exactly 10.00
    // link-only invoice (applied_amount_cents = None) -> unassign -> total unchanged
}

#[test]
fn test_unassign_after_receipt_price_edit_subtracts_originally_applied_amount() {
    // assign Other 5.00 (applied) -> edit receipt total_price_eur to 7.00 ->
    // unassign -> subtracts 5.00 (the snapshot), NOT 7.00 (test review C7)
}

#[test]
fn test_double_count_guard_links_only() {
    // trip other_costs=12.34 (manual), zero Other invoices, assign invoice amount 12.34
    // -> link-only, total stays 12.34, receipt.applied_amount_cents == None
}

#[test]
fn test_double_count_guard_is_cent_exact() {
    // Pin: comparison is to_cents(total) == to_cents(amount), no ±0.01 epsilon.
    // Discriminating value: trip other_costs=12.34, invoice 12.3345 —
    // old ±0.01 epsilon would say "equal" (diff 0.0055), cent-exact says
    // 1234 != 1233 -> guard does NOT trigger, amount IS added -> 24.67.
    // And the exact case still guards: 12.34 vs 12.34 -> link-only.
}

#[test]
fn test_first_other_populates_empty_trip() { /* existing behavior kept; applied_amount_cents == Some(cents) */ }

#[test]
fn test_assign_same_receipt_same_trip_twice_adds_once() {
    // second call is a no-op (I12) — total unchanged, no double note
}

#[test]
fn test_paperless_reupsert_same_trip_does_not_double_add() { /* I12, paperless path */ }

#[test]
fn test_receipt_reassign_reverses_old_trip_sum() {
    // applied Other receipt moved trip A -> trip B: A restored exactly,
    // B increased, snapshot updated (test review C4)
}

#[test]
fn test_reassign_receipt_with_new_type_reverses_old_contribution() {
    // Other (applied) re-assigned as Fuel -> old contribution reversed first (I12)
}

#[test]
fn test_paperless_assign_stores_snapshots() {
    // assign paperless Other -> link row has assignment_type/amount_eur/title
    // from backend-fetched doc + applied_amount_cents set when applied
}

#[test]
fn test_paperless_reassign_reverses_old_trip_sum() {
    // Other doc applied to trip A, reassign to trip B -> A restored exactly, B increased
}

#[test]
fn test_paperless_reassign_link_only_does_not_touch_old_trip() {
    // doc linked via double-count guard (applied None), reassign -> A's total untouched (I4)
}

#[test]
fn test_assign_other_receipt_null_price_is_link_only() {
    // total_price_eur = None -> link-only, trip total unchanged, applied None (I3)
}

#[test]
fn test_unassign_last_applied_other_resets_costs_to_none() {
    // populate-from-empty then unassign -> other_costs_eur == None, not Some(0.0) (I6)
}

#[test]
fn test_unassign_applied_other_after_manual_overwrite() {
    // applied 5.01 (total 15.01) -> user hand-edits total to 3.00 -> unassign
    // -> money_sub(3.00, 5.01) clamps -> other_costs_eur == None (pinned
    // semantics, I14: contribution is removed; divergence was already flagged)
}

#[test]
fn test_unassign_orphaned_receipt_succeeds_without_trip() {
    // trip deleted (receipts.trip_id orphaned) -> unassign clears link,
    // no error, no trip mutation attempted (I10)
}

#[test]
fn test_unassign_fuel_receipt_never_touches_other_costs() {
    // Fuel unassign keys on assignment type, not just the snapshot (minor)
}

#[test]
fn test_assign_rejects_non_finite_or_negative_amount() {
    // invoice amount NaN / -5.0 -> clear error, nothing linked, nothing added
    // (to_cents(NAN) == 0 silently — validate at the boundary instead)
}
```

**Step 2: Run — expect FAIL:**
`cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "assign"`

**Step 3: Implement.** Shared rules (both source paths, dispatch at the boundary per Task-64 pattern):

1. **Validation:** invoice amount, when present, must be finite and `>= 0.0` — else error before any mutation.
2. **Idempotency pre-check:** invoice already assigned to the SAME trip with the same type → no-op `Ok`. Already assigned elsewhere (or same trip, different type) → **reverse the old contribution first** (subtract `applied_amount_cents` from the old trip via `money_sub`, clear the old snapshot) — BOTH sources (C4/I12), then proceed as a fresh assign.
3. **Fuel pre-check (cross-source):** before linking a Fuel invoice, reject if the trip's coverage shows `has_fuel` in either store: error `"Trip already has a fuel invoice"` (i18n frontend-side). Fuel populate-if-empty unchanged.
4. **Other:**
   ```text
   amount = invoice amount (receipt total_price_eur / paperless doc.total_amount)
   if amount is None:                          link-only; applied = None      # I3
   elif no existing Other invoice on trip and to_cents(total or 0) == to_cents(amount):
                                               link-only; applied = None      # double-count guard, cent-exact
   elif total empty/0 and no existing Other:   populate; applied = Some(to_cents(amount))
   else: trip.other_costs_eur = money_add(total, amount); append note;
                                               applied = Some(to_cents(amount))
   ```
5. **Unassign (both paths):** if link is Other AND `applied_amount_cents` is `Some(c)` → `trip.other_costs_eur = money_sub(total, from_cents(c))`; result `0` → `None`. Trip missing (orphan) → skip the subtract, still clear the link (I10). Fuel unassign never touches `other_costs_eur`. Note handling: strip the appended segment if trivially identifiable, else leave the note untouched.
6. **Paperless assign:** build `PaperlessLink` from the **backend-fetched doc** (`doc.total_amount`, `doc.title`), never caller data.

**Step 4: Run full backend suite — expect PASS.** Existing single-invoice tests (e.g. `test_reassign_invoice_to_different_trip`, `test_assign_other_to_trip_with_existing_other_costs_allowed`) must be **updated to the new rules, not deleted**.

**Step 5: Note-handling test** (I2, part of this task's suite):

```rust
#[test]
fn test_other_assign_appends_note_and_unassign_strips_it() {
    // assign appends invoice note segment to other_costs_note;
    // unassign removes exactly that segment; a user-edited note is left untouched
}
```

**Step 6: Commit:**
```bash
git add src-tauri/core/src/commands_internal/ src-tauri/core/src/db.rs
git commit -m "feat(invoices): 1 Fuel + N Other per trip; cent-exact snapshot-based sum-on-assign/unassign"
```

---

### Task 6: Compatibility check + `can_attach` for multi-invoice (test review C8/I1)

> Without this task the feature's headline flow breaks: once a trip carries
> one applied Other invoice, the picker false-flags every further Other
> invoice as a price mismatch and pushes users through the mismatch-confirm
> flow, permanently polluting `other_mismatch_overrides`.

**Files:**
- Modify: [src-tauri/core/src/invoice.rs](../../src-tauri/core/src/invoice.rs) (`check_invoice_trip_compatibility`, Other branch ~lines 142–176)
- Modify: [src-tauri/core/src/commands_internal/invoices.rs](../../src-tauri/core/src/commands_internal/invoices.rs) (`get_trips_for_invoice_assignment_internal` ~line 55 — pass coverage in)
- Modify: [src-tauri/core/src/commands_internal/receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs) (picker caller ~line 470)
- Test: [src-tauri/core/src/invoice_tests.rs](../../src-tauri/core/src/invoice_tests.rs)

**Step 1: Write failing tests:**

```rust
#[test]
fn test_compatibility_second_other_invoice_not_flagged_as_price_mismatch() {
    // trip already has >=1 Other invoice attached -> amount comparison skipped,
    // status == Matches regardless of other_costs_eur vs invoice amount
}

#[test]
fn test_compatibility_other_uses_cent_exact_not_epsilon() {
    // zero Other invoices: comparison via to_cents — verdict must agree with
    // the Task 5 double-count guard on borderline values (12.34 vs 12.3345:
    // epsilon would say Matches, cent-exact says Differs — picker and assign
    // must give the same answer)
}

#[test]
fn test_trips_for_fuel_invoice_assignment_excludes_covered_trip() {
    // trip with a Fuel invoice (either source) -> can_attach == false in the
    // picker list, for both the receipt picker and the paperless picker (I1)
}
```

**Step 2: Run — expect FAIL.**

**Step 3: Implement** per the design ([02-design.md](./02-design.md) § compatibility): Other branch takes the trip's `TripInvoiceCoverage`; `has_other` → skip amount check (`Matches`); zero Others → cent-exact compare. Fuel: coverage `has_fuel` → `can_attach = false`. Both picker entry points fetch coverage once (`db.get_trip_invoice_coverage()`) and pass per-trip entries down.

**Step 4: Run — expect PASS.** Update existing `invoice_tests.rs` epsilon-based Other tests to the new semantics (update, don't delete).

**Step 5: Commit:**
```bash
git add src-tauri/core/src/invoice.rs src-tauri/core/src/invoice_tests.rs src-tauri/core/src/commands_internal/
git commit -m "feat(invoices): multi-invoice-aware compatibility check and can_attach"
```

---

### Task 7: Per-type grid indicators + sum-mismatch warning

**Files:**
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) (`TripGridData`, ~line 373–379)
- Modify: [src-tauri/core/src/commands_internal/statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs) (~lines 393–401, 442–452, 559–567, `calculate_missing_receipts` ~line 1226, `calculate_receipt_datetime_warnings` ~line 1242, `calculate_receipt_mismatch_overrides` ~line 1266)
- Test: [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) (missing-invoice block ~line 74+)

**Step 1: Write failing tests:**

```rust
#[test]
fn test_missing_fuel_invoice_flagged_per_type() {
    // trip has fuel cost + other cost; only Other invoice attached
    // -> in missing_fuel_invoices, NOT in missing_other_invoices
}

#[test]
fn test_missing_other_invoice_flagged_per_type() { /* mirror case */ }

#[test]
fn test_zero_value_costs_not_flagged_missing() {
    // other_costs_eur = Some(0.0) / fuel_liters = Some(0.0) -> NOT flagged
    // (pins the is_some() -> "> 0" predicate change, test review I9)
}

#[test]
fn test_other_sum_mismatch_flagged() {
    // other_costs_eur = 20.00, attached Others sum 15.01 -> trip in other_sum_mismatches
}

#[test]
fn test_other_sum_match_not_flagged() { /* 15.01 vs 15.01 -> absent */ }

#[test]
fn test_other_sum_unknown_amount_skips_check() {
    // one Other link with NULL amount -> trip absent even though partial sum differs
}

#[test]
fn test_datetime_warnings_type_scoped() {
    // fuel receipt out of range -> fuel_datetime_warnings only
}

#[test]
fn test_datetime_warning_fires_for_second_other_receipt() {
    // trip with in-range Fuel receipt + out-of-range Other receipt ->
    // trip in other_datetime_warnings (kills the `.find()` first-receipt-only
    // bug, test review I8)
}

#[test]
fn test_mismatch_override_recognized_on_second_receipt() { /* I8 mirror for overrides */ }
```

**Step 2: Run — expect FAIL.**

**Step 3: Implement:**

`TripGridData` — replace the three shared-warning fields:
```rust
    // Shared warnings (per assignment type since Task 66)
    /// Trip IDs with fuel cost but no Fuel invoice attached
    pub missing_fuel_invoices: HashSet<String>,
    /// Trip IDs with other costs but no Other invoice attached
    pub missing_other_invoices: HashSet<String>,
    /// Trip IDs where other_costs_eur != sum of attached Other invoice amounts (cent-exact)
    pub other_sum_mismatches: HashSet<String>,
    /// Trip IDs where an assigned Fuel invoice datetime is outside trip range
    pub fuel_datetime_warnings: HashSet<String>,
    /// Trip IDs where an assigned Other invoice datetime is outside trip range
    pub other_datetime_warnings: HashSet<String>,
    /// Trip IDs where user confirmed a mismatch (per type)
    pub fuel_mismatch_overrides: HashSet<String>,
    pub other_mismatch_overrides: HashSet<String>,
```

`statistics.rs`:
- `calculate_missing_receipts(trips, coverage: &HashMap<String, TripInvoiceCoverage>) -> (HashSet<String>, HashSet<String>)` — fuel-missing when `trip.fuel_liters` is `Some(v) if v > 0.0` and `!coverage.has_fuel`; other-missing when `trip.other_costs_eur` is `Some(v) if v > 0.0` and `!coverage.has_other`.
- New `calculate_other_sum_mismatches(trips, coverage) -> HashSet<String>`: flag when `coverage.has_other && !coverage.has_unknown_amount && to_cents(trip.other_costs_eur.unwrap_or(0.0)) != coverage.other_sum_cents`.
- `calculate_receipt_datetime_warnings` / `calculate_receipt_mismatch_overrides`: split output per `receipt.assignment_type` AND replace the `.find(|r| …)` first-match lookups with **iteration over ALL receipts of the trip** (`.filter(…)`) — I8.
- Wire it all in `get_trip_grid_data` (uses `db.get_trip_invoice_coverage()` from Task 3).

**Step 4: Run full backend suite — expect PASS** (update all `TripGridData` literals, ~line 393 default block included).

**Step 5: Commit:**
```bash
git add src-tauri/core/src/models.rs src-tauri/core/src/commands_internal/
git commit -m "feat(grid): per-type missing-invoice indicators + Other sum-mismatch warning"
```

---

### Task 8: Frontend — per-type warning rendering + i18n

**Files:**
- Modify: [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte) (~lines 450–453, 721–722)
- Modify: [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte) (props ~lines 64–66; fuel column ~line 655; other-costs column ~line 704)
- Modify: [src/lib/types.ts](../../src/lib/types.ts) (~lines 158–160 — the `TripGridData` mirror; find any others with `grep -rn "missingReceipts" src/`)
- Modify: [tests/integration/fixtures/types.ts](../../tests/integration/fixtures/types.ts) (~line 187 — stale `missingReceipts` mirror; update in the same commit so it can't silently lie)
- Modify: [src/lib/i18n](../../src/lib/i18n/) Slovak + English dictionaries
- Check: [src/lib/components/TripSelectorModal.svelte](../../src/lib/components/TripSelectorModal.svelte) and [src/routes/doklady/+page.svelte](../../src/routes/doklady/+page.svelte) — remove any "trip already has an invoice" gating for Other assignments (backend `can_attach` from Task 6 is authoritative and now actually implements the rule)

**Steps:**
1. Replace `hasMatchingReceipt`/`hasReceiptDatetimeWarning`/`hasReceiptMismatchOverride` props with per-type variants: `hasMatchingFuelInvoice`, `hasMatchingOtherInvoice`, `otherSumMismatch`, `fuelDatetimeWarning`, `otherDatetimeWarning`, `fuelMismatchOverride`, `otherMismatchOverride`. Fuel column renders fuel flags; other-costs column renders other flags + NEW sum-mismatch ⚠ with tooltip showing `other_costs_eur` vs invoice sum.
2. Wire from `gridData.missingFuelInvoices` / `missingOtherInvoices` / `otherSumMismatches` / etc. (camelCase serde output).
3. New i18n keys (SK first, EN mirror), e.g. `trips.legend.missingFuelInvoice` ("Chýba doklad o tankovaní"), `trips.legend.missingOtherInvoice` ("Chýba doklad k iným nákladom"), `trips.legend.otherSumMismatch` ("Suma iných nákladov nesedí so súčtom dokladov ({total} € vs {sum} €)").
4. Legend/counters in TripGrid (~line 450) updated to the new sets.
5. `npm run lint && npm run format` — fix findings.
6. Manual smoke: `npm run tauri:dev`, verify indicators on a trip with fuel+other.

**Commit:**
```bash
git add src/lib/ src/routes/ tests/integration/fixtures/types.ts
git commit -m "feat(ui): per-type invoice warnings incl. Other sum-mismatch"
```

---

### Task 9: Integration test (UI flow) + `seedReceipt` helper

**Files:**
- Modify: [tests/integration/utils/db.ts](../../tests/integration/utils/db.ts) — **add a `seedReceipt` helper; none exists today** (the `seedReceipt` mentioned in [tests/integration/README.md](../../tests/integration/README.md) ~line 167 is stale — test review I11). Follow the existing `seedTrip` pattern (direct DB insert of a completed/processed receipt row with explicit `assignment_type`, `total_price_eur`, `receipt_datetime`). Fix the README mention while here.
- Create: `tests/integration/specs/tier2/multi-invoice.spec.ts` (copy structure from an existing tier2 spec, e.g. [legal-compliance.spec.ts](../../tests/integration/specs/tier2/legal-compliance.spec.ts))

**Scenario (ONE spec — do not re-test backend math):**
1. Seed trip + two Other receipts + one Fuel receipt via `seedReceipt`.
2. Assign Fuel receipt → assign both Other receipts to the same trip via the picker — all succeed, no mismatch-confirm dialog appears for the second Other (C8 regression guard at UI level).
3. Grid **displays** the summed other-costs total (display verification — the arithmetic itself is proven in backend tests).
4. Hand-edit `other_costs_eur` to a different value → sum-mismatch ⚠ appears in the other-costs column, tooltip shows the actual Slovak text (assert visible translated text, not an i18n key).
5. Unassign one Other receipt → grid displays the reduced total; ⚠ state updates.

**Run (focused, debug build required):**
```bash
npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/multi-invoice.spec.ts
```
Expected: PASS. Then full sweep before claiming done: `npm run test:all`.

**Commit:**
```bash
git add tests/integration/
git commit -m "test(integration): multi-invoice assign/sum/mismatch flow + seedReceipt helper"
```

---

### Task 10: Documentation

**Files:**
- Create: [docs/features/multi-invoice.md](../../docs/features/multi-invoice.md) — per use-case user flow (fuel+parking ride; multiple Others; manual overwrite → warning; double-count guard; unassign behavior incl. after price edit) + technical notes (schema, `applied_amount_cents`, cent-exact math, backfill heuristic, atomic migration). Template: [docs/CLAUDE.md](../../docs/CLAUDE.md).
- Modify: [CHANGELOG.md](../../CHANGELOG.md) `[Unreleased]` — user-visible entry (via `/changelog`).
- Modify: [DECISIONS.md](../../DECISIONS.md) — BIZ entry (via `/decision`): cardinality 1 Fuel + N Other; sum-on-assign with manual overwrite + surfaced mismatch; cent-exact money math; `applied_amount_cents` unassign rule; **ADR-019 supersede note** (paperless link table shape changed).
- Sweep stale docs: `grep -rn -i "one receipt\|single receipt\|jeden doklad" docs/ README.md README.en.md` — **plus these known hits the grep misses** (test review, iteration 3): [docs/features/paperless-integration.md](../../docs/features/paperless-integration.md) (~lines 91–94: old `trip_id PRIMARY KEY`/`UNIQUE` shape + ADR-019 symmetry) and [docs/features/unified-invoice-picker.md](../../docs/features/unified-invoice-picker.md) (~line 10: match-indicator semantics changed by Task 6).
- Update [_tasks/index.md](../index.md): task 66 status → complete.

**Verification:** `/verify` skill — tests green, changelog updated, git status clean.

**Commit:**
```bash
git add docs/ CHANGELOG.md DECISIONS.md README.md README.en.md _tasks/index.md
git commit -m "docs: multi-invoice feature doc, changelog, decisions"
```

---

## Execution order & dependencies

```
Task 1 (money helpers)             — independent
Task 2+3 (atomic migration + plumbing) — commit together
Task 4 (migration integrity tests) — after 2+3 (tests the real SQL)
Task 5 (assign/unassign logic)     — after 1, 3
Task 6 (compatibility/can_attach)  — after 3, 5 (uses coverage + guard semantics)
Task 7 (grid indicators)           — after 3, 5
Task 8 (frontend)                  — after 6, 7
Task 9 (integration test)          — after 8
Task 10 (docs)                     — last
```

## Out of scope (explicitly)

- Export changes — export reads trip fields, which stay authoritative
  (verified: [export.rs](../../src-tauri/core/src/export.rs) reads no receipt
  or link tables and formats money with `{:.2}`).
- Deriving `other_costs_eur` from invoices (rejected in brainstorming — manual
  entry without documents must keep working).
- `mismatch_override` for Paperless links (pre-existing gap, unchanged).
- N Fuel invoices per trip (a trip has exactly one fill-up by design).
- Pre-migration backend backup + read-only backup fix — shipped separately
  (2026-07-15) from the backup-failure investigation; complements this
  migration but is not part of task 66.
