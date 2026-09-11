# Paperless-Only Invoices Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the local receipt + Gemini OCR subsystem and make Paperless-ngx the only invoice source, keeping the app fully usable without invoices.

**Architecture:** The source-agnostic invoice abstraction (`Invoice` trait, `InvoiceRef` enum, inline `InvoiceData`, TS adapters) is deleted. The `receipts` table is dropped by a new migration. Two columns are added to `paperless_trip_links` (`receipt_datetime`, `mismatch_override`) so the grid datetime warnings and override markers keep working. A new async command counts unlinked Paperless fuel docs for the nav badge.

**Tech Stack:** Rust (Axum, Diesel, SQLite), SvelteKit + TypeScript, WebdriverIO.

**Spec:** [_tasks/84-paperless-only-invoices/02-design.md](./02-design.md)

## Global Constraints

- All business logic stays in Rust. The frontend is display-only (ADR-008).
- Historical migrations are never edited. New migrations are forward-only (ADR-012).
- Always add columns with a DEFAULT value.
- Migrations run automatically on server start; a pre-migration backup is written first.
- User-facing strings go through i18n (`src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`); run `npm run i18n` after editing them.
- Use keyboard-typable characters only. No em-dash, curly quotes, or arrows in code, comments, or docs.
- Do not use `git add -A`. Stage only the files named in each task.
- Backend tests: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`.
- Integration tests need `npm run build` and a built `kniha-jazd-web` binary first.

---

## Task 1: Add `receipt_datetime` and `mismatch_override` to `paperless_trip_links`

**Files:**
- Create: `src-tauri/core/migrations/2026-09-11-120000_paperless_only_invoices/up.sql`
- Create: `src-tauri/core/migrations/2026-09-11-120000_paperless_only_invoices/down.sql`
- Modify: `src-tauri/core/src/schema.rs:107-119`
- Modify: `src-tauri/core/src/models.rs:1553-1564` (`PaperlessLink`)
- Modify: `src-tauri/core/src/db.rs:1121-1212` (upsert + selects), `src-tauri/core/src/db.rs:1427-1444` (row tuple + mapper)

**Interfaces:**
- Consumes: nothing.
- Produces: `PaperlessLink { ..., receipt_datetime: Option<NaiveDateTime>, mismatch_override: bool }`; DB columns `paperless_trip_links.receipt_datetime TEXT NULL`, `paperless_trip_links.mismatch_override INTEGER NOT NULL DEFAULT 0`.

- [ ] **Step 1: Write the migration up.sql**

Create `src-tauri/core/migrations/2026-09-11-120000_paperless_only_invoices/up.sql`:

```sql
-- Task 84: Paperless is the only invoice source. Local receipts are removed.
--
-- Two additive columns first, so the link can carry the data the local-receipt
-- grid features used: the invoice datetime (grid datetime warning) and the
-- user-confirmed mismatch flag (grid override marker). Existing rows get
-- NULL / 0, so no false warning appears on upgrade.
ALTER TABLE paperless_trip_links ADD COLUMN receipt_datetime TEXT DEFAULT NULL;
ALTER TABLE paperless_trip_links ADD COLUMN mismatch_override INTEGER NOT NULL DEFAULT 0;

-- Drop the local receipt store. It has foreign keys TO vehicles/trips and no
-- table references it, so the drop is safe with foreign keys on. Indexes drop
-- with the table. This intentionally discards local receipt rows: run
-- scripts/migrate_local_to_paperless.py BEFORE upgrading if you still need them.
DROP TABLE receipts;
```

Create `src-tauri/core/migrations/2026-09-11-120000_paperless_only_invoices/down.sql`:

```sql
-- Forward-only in practice (ADR-012). The dropped receipt rows cannot be
-- restored, so no down migration is provided.
```

- [ ] **Step 2: Run the migration test to verify it fails**

Modify nothing else yet. Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core migration
```

Expected: FAIL to compile or fail at runtime while `schema.rs` still lacks the new columns if the test opens a migrated DB. If it passes, continue; the schema change is compile-checked in Step 4.

- [ ] **Step 3: Update `schema.rs`**

Add the two columns at the END of the `paperless_trip_links` macro (positional `Queryable` binding depends on order):

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
        // Added via migration 2026-09-11-120000_paperless_only_invoices (Task 84)
        receipt_datetime -> Nullable<Text>,
        mismatch_override -> Bool,
    }
}
```

Then remove the `receipts` macro (lines 4-34), the two `receipts` joinables, and `receipts` from `allow_tables_to_appear_in_same_query!`:

```rust
diesel::joinable!(routes -> vehicles (vehicle_id));
diesel::joinable!(trips -> vehicles (vehicle_id));
diesel::joinable!(paperless_trip_links -> trips (trip_id));
diesel::joinable!(trip_routes -> trips (trip_id));

diesel::allow_tables_to_appear_in_same_query!(
    paperless_trip_links, routes, settings, trip_routes, trips, vehicles,
);
```

- [ ] **Step 4: Update `PaperlessLink` and the DB row mapping**

In `src-tauri/core/src/models.rs`:

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
    /// Doc datetime snapshot at assign time (grid datetime warning).
    pub receipt_datetime: Option<chrono::NaiveDateTime>,
    /// User confirmed a data mismatch at assign time (grid override marker).
    pub mismatch_override: bool,
}
```

In `src-tauri/core/src/db.rs`, extend the row tuple and mapper:

```rust
type PaperlessLinkRow = (
    i64, String, String, Option<f64>, Option<String>, Option<i64>, Option<String>, bool,
);

fn paperless_link_from_row(row: PaperlessLinkRow) -> PaperlessLink {
    let (
        paperless_document_id, trip_id, assignment_type, amount_eur, title,
        applied_amount_cents, receipt_datetime, mismatch_override,
    ) = row;
    PaperlessLink {
        paperless_document_id,
        trip_id,
        assignment_type: AssignmentType::from_str(&assignment_type)
            .unwrap_or(AssignmentType::Other),
        amount_eur,
        title,
        applied_amount_cents,
        receipt_datetime: receipt_datetime.as_deref().and_then(|s| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok()
        }),
        mismatch_override,
    }
}
```

- [ ] **Step 5: Update the three selects and the upsert**

In each of `get_paperless_link`, `get_paperless_links_for_trip`, and `list_paperless_links_for_docs`, add the two columns to the `.select((...))` tuple, after `applied_amount_cents`:

```rust
.select((
    p::paperless_document_id,
    p::trip_id,
    p::assignment_type,
    p::amount_eur,
    p::title,
    p::applied_amount_cents,
    p::receipt_datetime,
    p::mismatch_override,
))
```

In `upsert_paperless_link`, add the two values after `applied_amount_cents`:

```rust
p::receipt_datetime.eq(link.receipt_datetime.map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string())),
p::mismatch_override.eq(link.mismatch_override),
```

- [ ] **Step 6: Add `get_all_paperless_links`**

Add next to the other link readers in `src-tauri/core/src/db.rs`:

```rust
/// All link rows (grid datetime warnings read the whole set once).
pub fn get_all_paperless_links(&self) -> QueryResult<Vec<PaperlessLink>> {
    use crate::schema::paperless_trip_links::dsl as p;
    let conn = &mut *self.conn.lock().unwrap();
    p::paperless_trip_links
        .select((
            p::paperless_document_id,
            p::trip_id,
            p::assignment_type,
            p::amount_eur,
            p::title,
            p::applied_amount_cents,
            p::receipt_datetime,
            p::mismatch_override,
        ))
        .load::<PaperlessLinkRow>(conn)
        .map(|rows| rows.into_iter().map(paperless_link_from_row).collect())
}
```

(Use the real alias `use crate::schema::paperless_trip_links::dsl as p;`.)

- [ ] **Step 7: Fix every `PaperlessLink { ... }` construction**

Find them:

```bash
grep -rn "PaperlessLink {" src-tauri/core/src
```

Add `receipt_datetime` and `mismatch_override` to each literal (test fixtures use `None` / `false` for now).

- [ ] **Step 8: Run the backend tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core`
Expected: PASS (receipt tests still present and green).

- [ ] **Step 9: Commit**

```bash
git add src-tauri/core/migrations/2026-09-11-120000_paperless_only_invoices/ \
  src-tauri/core/src/schema.rs src-tauri/core/src/models.rs src-tauri/core/src/db.rs
git commit -m "feat(84): add receipt_datetime and mismatch_override to paperless_trip_links"
```

---

## Task 2: Persist the override and datetime snapshot on assign; add the revert command

**Files:**
- Modify: `src-tauri/core/src/commands_internal/invoices.rs:111-201`
- Modify: `src-tauri/core/src/server/dispatcher.rs` (`revert_receipt_override` block, ~680-693)
- Test: `src-tauri/core/src/commands_internal/invoices_tests.rs`

**Interfaces:**
- Consumes: `PaperlessLink.receipt_datetime`, `PaperlessLink.mismatch_override` (Task 1).
- Produces: `revert_paperless_override(db, app_state, doc_id) -> Result<(), String>`; RPC `revert_paperless_override` with arg `{ docId }`.

- [ ] **Step 1: Write the failing backend test**

In `src-tauri/core/src/commands_internal/invoices_tests.rs`, add a test that assigns a Paperless doc with `mismatch_override = true` and asserts the stored link carries the datetime and flag. Use the existing test DB setup helper in that file.

```rust
#[test]
fn assign_paperless_persists_datetime_and_override() {
    let (db, app_state, trip_id, vehicle_id) = /* existing fixture */;
    let dt = chrono::NaiveDateTime::parse_from_str("2026-05-04T13:24:14", "%Y-%m-%dT%H:%M:%S").unwrap();
    let doc = crate::paperless::PaperlessDoc {
        id: 435,
        title: "Fuel".into(),
        tag_ids: vec![],
        created: chrono::NaiveDate::from_ymd_opt(2026, 5, 4).unwrap(),
        total_amount: Some(63.34),
        litres: Some(40.0),
        receipt_datetime: Some(dt),
    };
    assign_invoice_to_trip_internal(
        &db, &app_state, &InvoiceRef::Paperless(435), Some(&doc),
        &trip_id, &vehicle_id, AssignmentType::Fuel, true,
    )
    .unwrap();
    let link = db.get_paperless_link(435).unwrap().unwrap();
    assert_eq!(link.receipt_datetime, Some(dt));
    assert!(link.mismatch_override);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core assign_paperless_persists_datetime_and_override`
Expected: FAIL (link has `receipt_datetime == None`, `mismatch_override == false`).

- [ ] **Step 3: Persist the fields in the Paperless arm**

In `src-tauri/core/src/commands_internal/invoices.rs`, replace the link literal and the `let _ = mismatch_override;` line:

```rust
let link = crate::models::PaperlessLink {
    paperless_document_id: *id,
    trip_id: trip_id.to_string(),
    assignment_type,
    amount_eur: doc.total_amount,
    title: Some(doc.title.clone()),
    applied_amount_cents,
    receipt_datetime: doc.receipt_datetime,
    mismatch_override,
};
db.upsert_paperless_link(&link).map_err(|e| e.to_string())?;
Ok(())
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core assign_paperless_persists_datetime_and_override`
Expected: PASS.

- [ ] **Step 5: Write the failing revert test**

```rust
#[test]
fn revert_paperless_override_clears_the_flag() {
    let (db, app_state, trip_id, vehicle_id) = /* existing fixture */;
    let doc = /* same doc as above, mismatch_override true on assign */;
    assign_invoice_to_trip_internal(
        &db, &app_state, &InvoiceRef::Paperless(435), Some(&doc),
        &trip_id, &vehicle_id, AssignmentType::Fuel, true,
    )
    .unwrap();
    revert_paperless_override_internal(&db, &app_state, 435).unwrap();
    let link = db.get_paperless_link(435).unwrap().unwrap();
    assert!(!link.mismatch_override);
}
```

- [ ] **Step 6: Run to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core revert_paperless_override_clears_the_flag`
Expected: FAIL (`revert_paperless_override_internal` not defined).

- [ ] **Step 7: Implement the revert command**

Add to `src-tauri/core/src/commands_internal/invoices.rs`:

```rust
/// Clear the user-confirmed mismatch flag on a Paperless link.
pub fn revert_paperless_override_internal(
    db: &Database,
    app_state: &AppState,
    doc_id: i64,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.set_paperless_override(doc_id, false).map_err(|e| e.to_string())
}
```

Add to `src-tauri/core/src/db.rs` next to the link writers:

```rust
/// Set the mismatch_override flag for one link.
pub fn set_paperless_override(&self, doc_id: i64, value: bool) -> QueryResult<()> {
    use crate::schema::paperless_trip_links::dsl as p;
    let conn = &mut *self.conn.lock().unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    diesel::update(p::paperless_trip_links.filter(p::paperless_document_id.eq(doc_id)))
        .set((p::mismatch_override.eq(value), p::updated_at.eq(now)))
        .execute(conn)
        .map(|_| ())
}
```

- [ ] **Step 8: Run the test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core revert_paperless_override_clears_the_flag`
Expected: PASS.

- [ ] **Step 9: Register the RPC command**

In `src-tauri/core/src/server/dispatcher.rs`, replace the `revert_receipt_override` block with:

```rust
"revert_paperless_override" => {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        doc_id: i64,
    }
    let a: Args = parse_args(args)?;
    crate::commands_internal::invoices::revert_paperless_override_internal(
        &state.db,
        &state.app_state,
        a.doc_id,
    )?;
    Ok(serde_json::to_value(()).unwrap())
}
```

- [ ] **Step 10: Run the backend tests and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core`
Expected: PASS.

```bash
git add src-tauri/core/src/commands_internal/invoices.rs src-tauri/core/src/db.rs \
  src-tauri/core/src/commands_internal/invoices_tests.rs src-tauri/core/src/server/dispatcher.rs
git commit -m "feat(84): persist paperless override + datetime, add revert command"
```

---

## Task 3: Port the grid datetime and override warning functions to Paperless links

**Files:**
- Modify: `src-tauri/core/src/commands_internal/statistics.rs:399-433`, `:1409-1465`
- Modify: `src-tauri/core/src/db.rs` (`get_trip_invoice_coverage`: delete the receipt half at ~1233-1259)
- Test: `src-tauri/core/src/commands_internal/statistics_tests.rs` (add) or the existing statistics test module

**Interfaces:**
- Consumes: `db.get_all_paperless_links()` (Task 1).
- Produces: `calculate_invoice_datetime_warnings(&[Trip], &[PaperlessLink]) -> (HashSet<String>, HashSet<String>)`, `calculate_invoice_override_warnings(&[Trip], &[PaperlessLink]) -> (HashSet<String>, HashSet<String>)`.

- [ ] **Step 1: Write the failing tests**

Add tests with a `Trip` and `PaperlessLink` fixtures:

```rust
#[test]
fn datetime_warning_flags_link_outside_trip_range() {
    let trip = trip(/* start 08:00, end 09:00 */);
    let link = link("Fuel", Some("2026-05-04T20:00:00"), false);
    let (fuel, other) = calculate_invoice_datetime_warnings(&[trip.clone()], &[link]);
    assert!(fuel.contains(&trip.id.to_string()));
    assert!(other.is_empty());
}

#[test]
fn override_warning_flags_only_true_links() {
    let trip = trip(/* ... */);
    let link = link("Other", Some("2026-05-04T08:30:00"), true);
    let (fuel, other) = calculate_invoice_override_warnings(&[trip.clone()], &[link]);
    assert!(other.contains(&trip.id.to_string()));
    assert!(fuel.is_empty());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core datetime_warning_flags override_warning_flags`
Expected: FAIL (functions not defined).

- [ ] **Step 3: Rewrite the two functions**

Replace `calculate_receipt_datetime_warnings` and `calculate_receipt_mismatch_overrides` in `statistics.rs`:

```rust
/// Find trips with an assigned invoice whose datetime is outside the trip's
/// [start, end] range. The datetime is the assign-time snapshot on the link.
pub fn calculate_invoice_datetime_warnings(
    trips: &[Trip],
    links: &[PaperlessLink],
) -> (HashSet<String>, HashSet<String>) {
    let mut fuel = HashSet::new();
    let mut other = HashSet::new();
    for trip in trips {
        for link in links.iter().filter(|l| l.trip_id == trip.id.to_string()) {
            let Some(dt) = link.receipt_datetime else { continue };
            if is_datetime_in_trip_range(dt, trip) {
                continue;
            }
            match link.assignment_type {
                AssignmentType::Other => other.insert(trip.id.to_string()),
                AssignmentType::Fuel => fuel.insert(trip.id.to_string()),
            };
        }
    }
    (fuel, other)
}

/// Find trips with an assigned invoice whose mismatch the user confirmed.
pub fn calculate_invoice_override_warnings(
    trips: &[Trip],
    links: &[PaperlessLink],
) -> (HashSet<String>, HashSet<String>) {
    let mut fuel = HashSet::new();
    let mut other = HashSet::new();
    for trip in trips {
        for link in links.iter().filter(|l| l.trip_id == trip.id.to_string()) {
            if !link.mismatch_override {
                continue;
            }
            match link.assignment_type {
                AssignmentType::Other => other.insert(trip.id.to_string()),
                AssignmentType::Fuel => fuel.insert(trip.id.to_string()),
            };
        }
    }
    (fuel, other)
}
```

Import `PaperlessLink` at the top of `statistics.rs`.

- [ ] **Step 4: Fix the call site**

In `build_trip_grid_data`, replace the `db.get_all_receipts()` line with:

```rust
let links = db.get_all_paperless_links().map_err(|e| e.to_string())?;
```

and the two call pairs:

```rust
let (fuel_datetime_warnings, other_datetime_warnings) =
    calculate_invoice_datetime_warnings(&trips, &links);
let (fuel_mismatch_overrides, other_mismatch_overrides) =
    calculate_invoice_override_warnings(&trips, &links);
```

- [ ] **Step 5: Delete the receipt half of `get_trip_invoice_coverage`**

In `db.rs`, remove the `use crate::schema::receipts::dsl as r;` line and the whole `// Assigned local receipts` block (the `receipt_rows` load and its `for` loop). Keep the Paperless block unchanged. Update the doc comment to say "Paperless links only".

- [ ] **Step 6: Run the tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core`
Expected: PASS (receipt-specific tests may now fail to compile; they are removed in Task 8. If several receipt tests block compilation, delete them in this task and record it in the commit message.)

- [ ] **Step 7: Commit**

```bash
git add src-tauri/core/src/commands_internal/statistics.rs src-tauri/core/src/db.rs
git commit -m "refactor(84): drive grid datetime + override warnings from paperless links"
```

---

## Task 4: Delete the source-agnostic invoice abstraction

**Files:**
- Rewrite: `src-tauri/core/src/invoice.rs`
- Modify: `src-tauri/core/src/commands_internal/invoices.rs:1-60, 83-201, 325-349`
- Move: `TripForAssignment` from `src-tauri/core/src/commands_internal/receipts_cmd.rs:504-517` to `invoices.rs`
- Test: `src-tauri/core/src/invoice_tests.rs` (rewrite fixtures)

**Interfaces:**
- Consumes: `PaperlessLink`, `db.get_trip_invoice_coverage()`.
- Produces: `check_paperless_trip_compatibility(doc: &PaperlessDoc, trip: &Trip, coverage: &TripInvoiceCoverage) -> CompatibilityResult`; `TripForAssignment` now lives in `invoices.rs`.

- [ ] **Step 1: Move `TripForAssignment`**

Cut the struct from `receipts_cmd.rs` and paste it into `invoices.rs` (keep the same fields and serde attributes).

- [ ] **Step 2: Replace `invoice.rs`**

Delete the `Invoice` trait, `InvoiceRef`, `InvoiceData`, and `PaperlessInvoiceView`. Keep `CompatibilityResult`, `is_same_date`, `get_datetime_mismatch_type`, and change the compat check to take a `&PaperlessDoc`:

```rust
//! Paperless invoice compatibility check (Task 84).
//!
//! The source-agnostic trait and enum were removed when local receipts were
//! deleted: Paperless is the only source.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::calculations::to_cents;
use crate::commands_internal::statistics::is_datetime_in_trip_range;
use crate::models::{AssignmentType, AttachmentStatus, Trip, TripInvoiceCoverage};
use crate::paperless::PaperlessDoc;

pub struct CompatibilityResult {
    pub can_attach: bool,
    pub status: String,
    pub mismatch_reason: Option<String>,
}

pub fn check_paperless_trip_compatibility(
    doc: &PaperlessDoc,
    trip: &Trip,
    coverage: &TripInvoiceCoverage,
) -> CompatibilityResult {
    let is_fuel = doc.litres.is_some();
    // ...body copied from check_invoice_trip_compatibility, replacing:
    //   invoice.datetime()      -> doc.receipt_datetime
    //   invoice.liters()        -> doc.litres
    //   invoice.total_price_eur()-> doc.total_amount
    //   invoice.assignment_type()-> doc.litres.map(...)  (.is_none() => Other)
    // Keep the existing fuel/other branches verbatim.
    # unimplemented!()
}
```

(For the `assignment_type` branch, use `Some(AssignmentType::Fuel)` when `doc.litres.is_some()`, else `Some(AssignmentType::Other)`.)

- [ ] **Step 3: Update `invoices.rs` commands**

- `get_trips_for_paperless_assignment_internal(db, doc: &PaperlessDoc, vehicle_id, year)` builds `PaperlessInvoiceView` no more; call `check_paperless_trip_compatibility(doc, &trip, trip_coverage)`.
- `assign_paperless_invoice_internal(db, app_state, doc: &PaperlessDoc, trip_id, vehicle_id, assignment_type, mismatch_override)` replaces `assign_invoice_to_trip_internal`. Remove the `match invoice_ref` and take `doc` directly. Body is the current `InvoiceRef::Paperless` arm.
- `unassign_paperless_invoice_internal(db, app_state, doc_id: i64)` is the current `InvoiceRef::Paperless` arm.

- [ ] **Step 4: Rewrite `invoice_tests.rs` fixtures**

Replace the `Receipt` fixture with a `PaperlessDoc` fixture and call `check_paperless_trip_compatibility`. Keep every test case that covers the fuel/other branch logic.

- [ ] **Step 5: Run the backend tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core`
Expected: failures only in receipt modules that are deleted in Task 7. The invoice tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/core/src/invoice.rs src-tauri/core/src/invoice_tests.rs \
  src-tauri/core/src/commands_internal/invoices.rs src-tauri/core/src/commands_internal/receipts_cmd.rs
git commit -m "refactor(84): collapse invoice abstraction to paperless-only"
```

---

## Task 5: Add the unlinked-fuel count command and its RPC

**Files:**
- Modify: `src-tauri/core/src/commands_internal/paperless_cmd.rs`
- Modify: `src-tauri/core/src/server/dispatcher_async.rs` (after `get_paperless_invoices`, ~150)
- Test: `src-tauri/core/src/commands_internal/paperless_cmd_tests.rs`

**Interfaces:**
- Consumes: `get_paperless_invoices_internal`.
- Produces: `count_unlinked_fuel(rows: &[PaperlessInvoiceRow]) -> i64`; `count_unlinked_paperless_fuel_invoices_internal(app_dir, db, vehicle_id, year) -> Result<i64, PaperlessError>`; RPC `count_unlinked_paperless_fuel_invoices` with `{ vehicleId, year }`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn count_unlinked_fuel_counts_only_unlinked_fuel_rows() {
    let rows = vec![
        row(AssignmentType::Fuel, None),
        row(AssignmentType::Fuel, Some("trip-1")),
        row(AssignmentType::Other, None),
    ];
    assert_eq!(count_unlinked_fuel(&rows), 1);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core count_unlinked_fuel_counts_only_unlinked_fuel_rows`
Expected: FAIL (function not defined).

- [ ] **Step 3: Implement the pure counter and wrapper**

In `src-tauri/core/src/commands_internal/paperless_cmd.rs`:

```rust
/// Count fuel-tagged docs that are not linked to a trip yet.
pub fn count_unlinked_fuel(rows: &[PaperlessInvoiceRow]) -> i64 {
    rows.iter()
        .filter(|r| r.assignment_type == AssignmentType::Fuel && r.trip_id.is_none())
        .count() as i64
}

/// Nav badge count for the active vehicle and year.
pub async fn count_unlinked_paperless_fuel_invoices_internal(
    app_dir: &Path,
    db: &Database,
    vehicle_id: &str,
    year: i32,
) -> Result<i64, PaperlessError> {
    let rows = get_paperless_invoices_internal(app_dir, db, vehicle_id, year).await?;
    Ok(count_unlinked_fuel(&rows))
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core count_unlinked_fuel_counts_only_unlinked_fuel_rows`
Expected: PASS.

- [ ] **Step 5: Register the async RPC command**

In `src-tauri/core/src/server/dispatcher_async.rs` after `list_paperless_custom_fields`:

```rust
"count_unlinked_paperless_fuel_invoices" => {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Args { vehicle_id: String, year: i32 }
    let a: Args = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return Some(Err(e)),
    };
    let result = crate::commands_internal::paperless_cmd::count_unlinked_paperless_fuel_invoices_internal(
        &state.app_dir, &state.db, &a.vehicle_id, a.year,
    ).await;
    Some(result.map(|v| serde_json::to_value(v).unwrap()).map_err(|e| e.to_string()))
}
```

- [ ] **Step 6: Run the tests and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core`
Expected: PASS.

```bash
git add src-tauri/core/src/commands_internal/paperless_cmd.rs \
  src-tauri/core/src/commands_internal/paperless_cmd_tests.rs \
  src-tauri/core/src/server/dispatcher_async.rs
git commit -m "feat(84): add unlinked paperless fuel invoice count command"
```

---

## Task 6: Remove the receipt RPC commands, image route, and Gemini reveal field

**Files:**
- Modify: `src-tauri/core/src/server/dispatcher.rs:620-780` (remove receipt blocks; update shared blocks)
- Modify: `src-tauri/core/src/server/dispatcher_async.rs:31-56, 158-197`
- Modify: `src-tauri/core/src/server/mod.rs:102-132, 197`
- Modify: `src-tauri/core/src/commands_internal/reveal.rs:22,30,38`

**Interfaces:**
- Consumes: `get_trips_for_paperless_assignment_internal`, `assign_paperless_invoice_internal`, `unassign_paperless_invoice_internal` (Task 4).
- Produces: no receipt RPCs.

- [ ] **Step 1: Delete the sync receipt commands**

Remove these dispatcher arms: `get_receipts`, `get_receipts_for_vehicle`, `get_unassigned_receipts`, `update_receipt`, `delete_receipt`, `verify_receipts`, `get_receipt_settings`, `set_gemini_api_key`, `set_receipts_folder_path`, `scan_receipts`. (`revert_receipt_override` was already replaced in Task 2.)

- [ ] **Step 2: Rewrite the shared sync commands**

Replace `get_trips_for_invoice_assignment` and `unassign_invoice`:

```rust
"get_trips_for_paperless_assignment" => {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        doc_id: i64,
        vehicle_id: String,
        year: i32,
    }
    let a: Args = parse_args(args)?;
    let doc = crate::commands_internal::paperless_cmd::fetch_paperless_doc_by_id(
        &state.app_dir, a.doc_id,
    )
    .map_err(|e| e.to_string())?;
    let v = crate::commands_internal::invoices::get_trips_for_paperless_assignment_internal(
        &state.db, &doc, &a.vehicle_id, a.year,
    )?;
    Ok(serde_json::to_value(v).unwrap())
}
"unassign_paperless_invoice" => {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Args { doc_id: i64 }
    let a: Args = parse_args(args)?;
    crate::commands_internal::invoices::unassign_paperless_invoice_internal(
        &state.db, &state.app_state, a.doc_id,
    )?;
    Ok(serde_json::to_value(()).unwrap())
}
```

Note: `fetch_paperless_doc_by_id` is async. Move `get_trips_for_paperless_assignment` to `dispatcher_async.rs` instead, next to `assign_invoice_to_trip`, because it now needs the async fetch.

- [ ] **Step 3: Rewrite the async assign command**

In `dispatcher_async.rs`, replace `assign_invoice_to_trip`:

```rust
"assign_paperless_invoice" => {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        doc_id: i64,
        trip_id: String,
        vehicle_id: String,
        assignment_type: crate::models::AssignmentType,
        mismatch_override: bool,
    }
    let a: Args = match parse_args(args) {
        Ok(a) => a,
        Err(e) => return Some(Err(e)),
    };
    let doc = match crate::commands_internal::paperless_cmd::fetch_paperless_doc_by_id(
        &state.app_dir, a.doc_id,
    ).await {
        Ok(doc) => doc,
        Err(e) => return Some(Err(e.to_string())),
    };
    let result = crate::commands_internal::invoices::assign_paperless_invoice_internal(
        &state.db, &state.app_state, &doc, &a.trip_id, &a.vehicle_id,
        a.assignment_type, a.mismatch_override,
    );
    Some(result.map(|_| serde_json::to_value(()).unwrap()))
}
```

Remove the async `sync_receipts`, `process_pending_receipts`, and `reprocess_receipt` arms.

- [ ] **Step 4: Remove the receipt image route**

In `src-tauri/core/src/server/mod.rs`, delete the `receipt_image_handler` function and the route line `GET /api/receipts/{id}/image`.

- [ ] **Step 5: Remove the Gemini reveal variant**

In `src-tauri/core/src/commands_internal/reveal.rs`, delete the `SecretField::GeminiApiKey` variant and its match arms (`label`, `value`). Keep the HA and Paperless fields.

- [ ] **Step 6: Build**

Run: `cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web`
Expected: FAIL if any deleted function is still referenced. Follow the compiler errors and delete the remaining references (they are all receipt modules due in Task 7).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/core/src/server/dispatcher.rs src-tauri/core/src/server/dispatcher_async.rs \
  src-tauri/core/src/server/mod.rs src-tauri/core/src/commands_internal/reveal.rs
git commit -m "refactor(84): drop receipt RPCs, image route, and gemini reveal field"
```

---

## Task 7: Delete the receipt and Gemini modules, settings, models, and drop the table

**Files:**
- Delete: `src-tauri/core/src/receipts.rs`, `src-tauri/core/src/receipts_tests.rs`,
  `src-tauri/core/src/gemini.rs`, `src-tauri/core/src/gemini_tests.rs`,
  `src-tauri/core/src/commands_internal/receipts_cmd.rs`,
  `src-tauri/core/src/commands_internal/receipts_cmd_tests.rs`
- Modify: `src-tauri/core/src/lib.rs:15,20`, `src-tauri/core/src/commands_internal/mod.rs:27`
- Modify: `src-tauri/core/src/models.rs` (delete receipt types)
- Modify: `src-tauri/core/src/settings.rs`
- Modify: `src-tauri/core/src/constants.rs` (`MOCK_GEMINI_DIR`)
- Modify: `src-tauri/core/src/db.rs` (delete receipt functions)
- Modify: `local.settings.json.sample`

**Interfaces:**
- Consumes: no receipt code remains.
- Produces: no `receipts` table, no Gemini module.

- [ ] **Step 1: Delete the modules and their registrations**

```bash
git rm src-tauri/core/src/receipts.rs src-tauri/core/src/receipts_tests.rs \
  src-tauri/core/src/gemini.rs src-tauri/core/src/gemini_tests.rs \
  src-tauri/core/src/commands_internal/receipts_cmd.rs \
  src-tauri/core/src/commands_internal/receipts_cmd_tests.rs
```

Remove `pub mod gemini;` and `pub mod receipts;` from `lib.rs`, and `pub mod receipts_cmd;` from `commands_internal/mod.rs`.

- [ ] **Step 2: Delete the receipt models**

From `models.rs`, delete: `ReceiptStatus`, `ConfidenceLevel`, `FieldConfidence`,
`Receipt`, `MismatchReason`, `Currency`, `ReceiptVerification`, `VerificationResult`,
`ReceiptRow`, `NewReceiptRow`, `From<ReceiptRow> for Receipt`, `impl Receipt`, and
`impl Invoice for Receipt`. Keep `AssignmentType`, `AttachmentStatus`,
`PaperlessInvoiceRow`, `PaperlessLink`, `TripInvoiceCoverage`.

- [ ] **Step 3: Delete the receipt DB functions**

From `db.rs`, delete: `create_receipt`, `get_all_receipts`, `get_unassigned_receipts`,
`get_pending_receipts`, `update_receipt`, `delete_receipt`, `unassign_receipt`,
`revert_receipt_override`, `get_receipt_by_file_path`, `get_receipt_by_id`,
`get_receipts_for_year`, `get_receipts_for_vehicle`, the receipts unassign step in
`delete_vehicle`, and the `Receipt`/`ReceiptRow` imports. Also delete the
`seed_minimal_receipt`/`snapshot_receipts` test helpers and receipt migration tests
in `migration_tests.rs`, and the receipt cases in `db_tests.rs`.

- [ ] **Step 4: Remove the Gemini settings**

In `settings.rs`: delete `gemini_api_key`, `receipts_folder_path`,
`env_vars::GEMINI_API_KEY`, its `apply_overrides` block, and update `env_vars::ALL`
to `[&str; 5]` without `GEMINI_API_KEY`. In `constants.rs`, delete
`MOCK_GEMINI_DIR`. Update `local.settings.json.sample` to an empty object `{}`.
Scrub the live key from `local.settings.json` (gitignored) and
`data/local.settings.json`.

- [ ] **Step 5: Build and run the backend tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
Expected: PASS. Fix any remaining compile references to deleted items.

- [ ] **Step 6: Verify the schema no longer mentions receipts**

Run: `grep -rn "receipts" src-tauri/core/src/schema.rs src-tauri/core/src/db.rs`
Expected: no match (the only `receipt_datetime` hits are the paperless column).

- [ ] **Step 7: Commit**

```bash
git add -u src-tauri/core/src local.settings.json.sample
git commit -m "feat(84): delete local receipt + gemini OCR subsystem and drop receipts table"
```

---

## Task 8: Clean the backend test fixtures and helpers

**Files:**
- Modify: `src-tauri/core/src/commands_tests.rs` (delete receipt-only tests)
- Modify: `src-tauri/core/src/commands_internal/invoices_tests.rs` (delete the `InvoiceRef` receipt cases)
- Modify: `src-tauri/core/src/export_tests.rs` (already holds the new grid fields)
- Modify: `src-tauri/core/src/invoice_tests.rs` (Paperless double, from Task 4)

**Interfaces:**
- Consumes: no receipt types.
- Produces: a backend suite with no receipt references.

- [ ] **Step 1: Find every remaining receipt reference in the backend**

Run:

```bash
grep -rnE "Receipt|receipt|gemini|Gemini" src-tauri/core/src --include=*.rs
```

Expected: only `receipt_datetime` on the paperless link, and comment/test text that names Paperless docs.

- [ ] **Step 2: Delete the receipt-only tests**

Remove `make_receipt_with_datetime_assigned` and the `calculate_receipt_datetime_warnings` and `calculate_receipt_mismatch_overrides` tests. Keep `calculate_missing_receipts`, `calculate_other_sum_mismatches`, and `calculate_other_invoice_sums` tests.

- [ ] **Step 3: Update `invoices_tests.rs`**

Replace any `InvoiceRef::Receipt` case with a `PaperlessDoc` case calling `assign_paperless_invoice_internal` / `unassign_paperless_invoice_internal`.

- [ ] **Step 4: Run the backend tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/core/src/commands_tests.rs src-tauri/core/src/commands_internal/invoices_tests.rs \
  src-tauri/core/src/export_tests.rs
git commit -m "test(84): prune local receipt backend tests"
```

---

## Task 9: Simplify the frontend picker and rebuild Doklady Paperless-only

**Files:**
- Modify: `src/lib/api.ts:527-562`
- Modify: `src/lib/types.ts` (delete `InvoiceRef`, `InvoiceData`, `InvoiceSourceMode`)
- Delete: `src/lib/invoice.ts`
- Modify: `src/lib/components/TripSelectorModal.svelte`
- Modify: `src/routes/doklady/+page.svelte`
- Create: `src/lib/components/InvoiceIndicator.svelte`
- Modify: `src/routes/+layout.svelte:14,129`

**Interfaces:**
- Consumes: RPC commands `get_trips_for_paperless_assignment`, `assign_paperless_invoice`, `unassign_paperless_invoice`, `revert_paperless_override`, `count_unlinked_paperless_fuel_invoices` (Tasks 2, 4, 5, 6).
- Produces: a Doklady page with no local branch; `InvoiceIndicator`.

- [ ] **Step 1: Rewrite the API wrappers**

In `src/lib/api.ts`:

```ts
export async function countUnlinkedPaperlessFuelInvoices(
	vehicleId: string,
	year: number,
): Promise<number> {
	return apiCall<number>('count_unlinked_paperless_fuel_invoices', { vehicleId, year });
}

export async function getTripsForPaperlessAssignment(
	docId: number,
	vehicleId: string,
	year: number,
): Promise<TripForAssignment[]> {
	return await apiCall('get_trips_for_paperless_assignment', { docId, vehicleId, year });
}

export async function assignPaperlessInvoice(
	docId: number,
	tripId: string,
	vehicleId: string,
	assignmentType: 'Fuel' | 'Other',
	mismatchOverride = false,
): Promise<void> {
	return await apiCall('assign_paperless_invoice', {
		docId, tripId, vehicleId, assignmentType, mismatchOverride,
	});
}

export async function unassignPaperlessInvoice(docId: number): Promise<void> {
	return await apiCall('unassign_paperless_invoice', { docId });
}

export async function revertPaperlessOverride(docId: number): Promise<void> {
	return await apiCall('revert_paperless_override', { docId });
}
```

Delete `getInvoiceSourceMode`, `getTripsForInvoiceAssignment`, `assignInvoiceToTrip`, `unassignInvoice`, and the receipt wrappers.

- [ ] **Step 2: Delete `invoice.ts` and unused types**

```bash
git rm src/lib/invoice.ts
```

From `types.ts`, delete `InvoiceRef`, `InvoiceData`, `InvoiceSourceMode`, and the receipt types (`Receipt`, `ReceiptSettings`, `ScanResult`, `SyncResult`, `ReceiptVerification`, `ReceiptDisplayState`, `ReceiptStatus`, `ReceiptCurrency`, ...). Keep `PaperlessInvoiceRow`, `PaperlessCustomFieldInfo`, and `TripForAssignment`.

- [ ] **Step 3: Update `TripSelectorModal.svelte`**

Change the `invoice` prop to `invoice: PaperlessInvoiceRow`. Replace `invoice.getRef()` / `invoice.getData()` calls with `invoice.paperlessDocumentId` and inline values. Call `getTripsForPaperlessAssignment(invoice.paperlessDocumentId, vehicleId, year)` and `assignPaperlessInvoice(...)`; pass `mismatchOverride` from the existing override prompt.

- [ ] **Step 4: Rebuild the Doklady page**

In `src/routes/doklady/+page.svelte`, remove the local branch, receipt handlers, `ReceiptEditModal`, and receipt-only helpers. `loadInvoices` becomes:

```ts
async function loadInvoices() {
	loading = true;
	paperlessError = null;
	try {
		const vehicle = $activeVehicleStore;
		if (!vehicle) {
			paperlessRows = [];
			return;
		}
		paperlessRows = await api.getPaperlessInvoices(vehicle.id, $selectedYearStore);
	} catch (error) {
		paperlessError = String(error);
		paperlessRows = [];
	} finally {
		loading = false;
	}
}
```

Add the setup empty state when Paperless is not configured (treat a `NotConfigured` error as the empty state, not a toast). Set `needsPaperlessSetup = true` in the `catch` block when the error is `NotConfigured`:

```ts
let needsPaperlessSetup = $state(false);
// in loadInvoices catch:
needsPaperlessSetup = paperlessError.includes('NotConfigured') || paperlessError.includes('not configured');
```

```svelte
{#if !paperlessLoading && needsPaperlessSetup}
	<div class="empty-state">
		<p>{$LL.doklady.paperless.notConfigured()}</p>
		<a href="/settings">{$LL.doklady.paperless.openSettings()}</a>
	</div>
{/if}
```

Remove `verifyReceipts`, `scanReceipts`, `syncReceipts`, `processPendingReceipts`, `reprocessReceipt`, `updateReceipt`, `deleteReceipt`, `revertReceiptOverride`, and the receipt-only modal handlers.

- [ ] **Step 5: Replace the nav indicator**

Create `src/lib/components/InvoiceIndicator.svelte`:

```svelte
<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import LL from '$lib/i18n/i18n-svelte';

	let count = $state(0);
	let loading = $state(true);

	onMount(() => {
		loadCount();
		const interval = setInterval(loadCount, 300000); // 5 min: hits the Paperless API
		return () => clearInterval(interval);
	});

	$effect(() => {
		const _vehicle = $activeVehicleStore;
		const _year = $selectedYearStore;
		loadCount();
	});

	async function loadCount() {
		const vehicle = $activeVehicleStore;
		if (!vehicle) {
			count = 0;
			loading = false;
			return;
		}
		try {
			count = await api.countUnlinkedPaperlessFuelInvoices(vehicle.id, $selectedYearStore);
		} catch {
			count = 0;
		} finally {
			loading = false;
		}
	}
</script>

{#if !loading && count > 0}
	<span class="badge" title={$LL.doklady.paperless.unlinkedBadge()}>{count}</span>
{/if}
```

Copy the `<style>` badge block from `ReceiptIndicator.svelte`. In `+layout.svelte`, import `InvoiceIndicator` and use it in place of `ReceiptIndicator`.

- [ ] **Step 6: Delete the receipt components and store**

```bash
git rm src/lib/components/ReceiptIndicator.svelte \
  src/lib/components/ReceiptEditModal.svelte \
  src/lib/stores/receipts.ts
```

Remove `triggerReceiptRefresh` and `receiptRefreshTrigger` imports from `TripGrid.svelte` and any remaining file.

- [ ] **Step 7: Update i18n and check**

Add to `src/lib/i18n/sk/index.ts` and `src/lib/i18n/en/index.ts`:

```ts
doklady: {
	paperless: {
		notConfigured: () => 'Paperless-ngx nie je nastavený.',
		openSettings: () => 'Otvoriť nastavenia',
		unlinkedBadge: () => 'Nepriradené doklady',
		// ...existing paperless keys stay...
	},
},
```

Then run: `npm run i18n && npm run check`
Expected: no errors.

- [ ] **Step 8: Build**

Run: `npm run build`
Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add src/lib/api.ts src/lib/types.ts src/lib/components/TripSelectorModal.svelte \
  src/lib/components/InvoiceIndicator.svelte src/routes/doklady/+page.svelte \
  src/routes/+layout.svelte src/lib/components/TripGrid.svelte \
  src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts src/lib/i18n/i18n-types.ts
git add -u src/lib
git commit -m "feat(84): make the doklady page paperless-only, simplify the picker"
```

---

## Task 10: Remove the receipt settings section and remaining receipt strings

**Files:**
- Modify: `src/routes/settings/+page.svelte` (receipt section 1045-1103, state 48-52,114-115, autosave 219-248, load 553-562)
- Modify: `src/lib/api.ts` (delete `getReceiptSettings`, `setGeminiApiKey`, `setReceiptsFolderPath`)
- Modify: `src/lib/constants.ts` (`RECEIPT_STATUS`, `RECEIPT_FILTERS`, `RECEIPT_TYPE_FILTERS`)
- Modify: `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

**Interfaces:**
- Consumes: nothing.
- Produces: no receipt UI or strings.

- [ ] **Step 1: Delete the Settings section**

Remove the "Skenovanie dokladov" / "Receipt Scanning" block, the `receiptsFolderPath` and `geminiApiKey` state, their autosave and load code, and the now-unused imports.

- [ ] **Step 2: Delete receipt API wrappers and constants**

Remove `getReceiptSettings`, `setGeminiApiKey`, `setReceiptsFolderPath` from `api.ts`. Remove `RECEIPT_STATUS`, `RECEIPT_FILTERS`, `RECEIPT_TYPE_FILTERS` from `constants.ts`.

- [ ] **Step 3: Delete the receipt i18n namespaces**

From both `sk/index.ts` and `en/index.ts`, delete the `receipts` and `receiptEdit` namespaces, the `settings.receiptScanning*`/`geminiApiKey*`/`receiptsFolder*` keys, and the receipt-only `toast.*` and `confirm.*` keys. Keep `tripSelector`, `paperless`, and `doklady.paperless`.

- [ ] **Step 4: Regenerate and check**

Run: `npm run i18n && npm run check`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/routes/settings/+page.svelte src/lib/api.ts src/lib/constants.ts \
  src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts src/lib/i18n/i18n-types.ts
git commit -m "chore(84): remove receipt settings UI and i18n namespaces"
```

---

## Task 11: Remove the local-receipt integration tests and mock wiring

**Files:**
- Delete: `tests/integration/specs/tier2/receipts.spec.ts`, `tests/integration/specs/tier2/receipt-settings.spec.ts`
- Modify: `tests/integration/specs/tier2/multi-invoice.spec.ts` (drop receipt seeding)
- Modify: `tests/integration/specs/env/env-managed-settings.spec.ts` (drop Gemini)
- Delete: `tests/integration/data/mocks/`, `tests/integration/data/invoices/`
- Modify: `tests/integration/fixtures/receipts.ts`, `fixtures/scenarios.ts`, `fixtures/types.ts`
- Modify: `tests/integration/utils/db.ts`, `utils/assertions.ts`, `utils/language.ts`
- Modify: `tests/integration/wdio.server.conf.ts:264-265,308`
- Modify: `.github/workflows/test.yml:150,237`

**Interfaces:**
- Consumes: no receipt backend.
- Produces: an integration suite that does not know about receipts or Gemini.

- [ ] **Step 1: Delete the specs and fixtures**

```bash
git rm tests/integration/specs/tier2/receipts.spec.ts \
  tests/integration/specs/tier2/receipt-settings.spec.ts \
  tests/integration/fixtures/receipts.ts
git rm -r tests/integration/data/mocks tests/integration/data/invoices
```

- [ ] **Step 2: Remove the receipt test helpers**

Delete `triggerReceiptScan`, `syncReceipts`, `reprocessReceipt`, `deleteReceipt`, `getReceipts`, `getReceiptsForVehicle`, `setReceiptsFolderPath`, `updateReceipt`, and `seedReceipt` from `utils/db.ts`; the doklady receipt selectors from `utils/assertions.ts`; and the receipt translations from `utils/language.ts`.

- [ ] **Step 3: Fix the mixed specs and fixtures**

In `multi-invoice.spec.ts`, replace receipt seeding with Paperless mock rows. In `env-managed-settings.spec.ts`, remove the Gemini env badge assertions. In `fixtures/scenarios.ts` and `fixtures/types.ts`, remove receipt imports and fields.

- [ ] **Step 4: Remove the Gemini mock wiring**

Delete `KNIHA_JAZD_MOCK_GEMINI_DIR` from `wdio.server.conf.ts` and from both CI jobs in `test.yml`.

- [ ] **Step 5: Build and run a focused spec**

```bash
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/paperless-integration.spec.ts
```

Expected: PASS.

- [ ] **Step 6: Add the Doklady empty-state spec**

Add a test to `paperless-integration.spec.ts` (or a new `doklady-paperless-only.spec.ts`) that clears the Paperless settings and asserts the setup empty state appears.

- [ ] **Step 7: Commit**

```bash
git add tests/integration .github/workflows/test.yml
git commit -m "test(84): remove local receipt + gemini integration coverage"
```

---

## Task 12: Docs, changelog, decisions, and tech debt

**Files:**
- Delete: `docs/features/receipt-scanning.md`
- Modify: `docs/features/multi-invoice.md`, `docs/features/unified-invoice-picker.md`,
  `docs/features/settings-architecture.md`, `docs/features/server-mode.md`,
  `docs/features/read-only-mode.md`, `ARCHITECTURE.md`, `README.md`, `README.en.md`
- Modify: `CHANGELOG.md`, `DECISIONS.md`
- Create: `_tasks/_TECH_DEBT/07-paperless-ocr-capability-gaps.md`

**Interfaces:**
- Consumes: the finished implementation.
- Produces: accurate docs and a recorded set of explicit losses.

- [ ] **Step 1: Delete and edit the feature docs**

```bash
git rm docs/features/receipt-scanning.md
```

In the mixed docs, remove the local-receipt path, the `ReceiptInvoice` adapter, and the mode switch. State that Paperless is the only source.

- [ ] **Step 2: Update the top-level docs**

Remove the receipt/OCR sections from `README.md`, `README.en.md`, and `ARCHITECTURE.md`. Add the upgrade rule: run `scripts/migrate_local_to_paperless.py` before upgrading to this release, because the upgrade drops the `receipts` table.

- [ ] **Step 3: Add the tech-debt entry**

Create `_tasks/_TECH_DEBT/07-paperless-ocr-capability-gaps.md` listing gap items 1 to 6 from `02-design.md`, each with its Paperless-side implementation path.

- [ ] **Step 4: Add the CHANGELOG entry**

Under `[Unreleased]`, add a Removed section: local receipt scanning, Gemini OCR, and the local invoice fallback; Paperless-ngx is the only invoice source.

- [ ] **Step 5: Add the superseding ADR**

Add an ADR to `DECISIONS.md` that supersedes the local-receipt parts of ADR-010,
ADR-019 to ADR-021, BIZ-015/016, and closes the ADR-030 line 369 note. Record the
capability losses and the ported features.

- [ ] **Step 6: Update the task index**

In `_tasks/index.md`, mark task 84 as implemented or archive it to `_done/` per the
normal completion flow.

- [ ] **Step 7: Run the full verification**

```bash
npm run test:backend
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
npm run test:integration
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add docs ARCHITECTURE.md README.md README.en.md CHANGELOG.md DECISIONS.md \
  _tasks/_TECH_DEBT/07-paperless-ocr-capability-gaps.md _tasks/index.md
git rm docs/features/receipt-scanning.md
git commit -m "docs(84): document paperless-only invoices and the capability losses"
```

---

## Self-Review Checklist

- [ ] Every requirement in `01-task.md` maps to a task above.
- [ ] The `receipts` table drop and the two link columns are one migration, applied atomically.
- [ ] `get_trip_invoice_coverage` no longer reads `receipts`.
- [ ] The nav badge command, the datetime warning port, and the override port all have a failing test before implementation.
- [ ] No task leaves a dangling reference to a deleted type or RPC.
- [ ] `npm run i18n` runs after every i18n edit.
- [ ] The destructive migration is last in the code order.