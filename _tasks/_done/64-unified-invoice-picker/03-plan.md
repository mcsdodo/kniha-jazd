**Date:** 2026-05-04
**Subject:** Unified Invoice Picker — Implementation Plan
**Status:** Planning

# Unified Invoice Picker Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Route Paperless invoices through the same [TripSelectorModal](../../src/lib/components/TripSelectorModal.svelte) as local receipts so users get proximity-sorted trip lists, mismatch warnings, Fuel/Other selection, and override flow regardless of invoice source.

**Architecture:** A new Rust `Invoice` trait + `InvoiceRef` tagged enum unifies the compatibility check. `Receipt` and `PaperlessDoc` implement the trait. A new `InvoiceData` DTO carries the inline fields (datetime/liters/price/title) so backend never has to fetch Paperless docs by id (the `paperless_trip_links` table doesn't cache doc data — frontend always has the row already from `get_paperless_invoices`). Frontend gets a parallel `Invoice` interface plus `ReceiptInvoice`/`PaperlessInvoice` adapters; [TripSelectorModal](../../src/lib/components/TripSelectorModal.svelte) becomes invoice-source-agnostic.

**Tech Stack:** Rust (Tauri backend, Diesel/SQLite), TypeScript + Svelte 5 (frontend), WebdriverIO (integration tests).

**Design Reference:** [02-design.md](./02-design.md) — read this before starting; the design enumerates the boundary contract and the exact trait shape. Original task description in [01-task.md](./01-task.md).

**Deviation from design:** The design doc's `load_invoice(db, &InvoiceRef) -> Box<dyn Invoice>` cannot be implemented for Paperless without either a new doc-cache table or an async network call per modal-open. The plan instead carries **inline invoice data** through the IPC boundary alongside `InvoiceRef`. Source dispatch still lives in exactly the spots the design names (Tauri command body and one TS adapter); the trait, compat check, and frontend remain source-agnostic.

---

## Task 1: Create Invoice trait + InvoiceRef + InvoiceData

**Files:**
- Create: [src-tauri/core/src/invoice.rs](../../src-tauri/core/src/invoice.rs)
- Modify: [src-tauri/core/src/lib.rs](../../src-tauri/core/src/lib.rs) (register the new module)

**Step 1: Create the invoice module**

Create [src-tauri/core/src/invoice.rs](../../src-tauri/core/src/invoice.rs):

```rust
//! Source-agnostic invoice abstraction (Task 64).
//!
//! Both local receipts and Paperless documents are *invoices* from the user's
//! perspective. This module provides the trait, IPC boundary types, and compat
//! check that the unified picker uses. Source-specific dispatch is confined to
//! the Tauri command boundary (see desktop/src/commands/invoices.rs).

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::models::AssignmentType;

/// Tagged reference used at the IPC boundary.
/// Serializes to `{ "source": "receipt", "id": "uuid" }`
/// or            `{ "source": "paperless", "id": 12345 }`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "source", content = "id", rename_all = "lowercase")]
pub enum InvoiceRef {
    Receipt(String), // UUID string
    Paperless(i64),  // Paperless document ID
}

/// Inline invoice payload sent by the frontend alongside the InvoiceRef.
/// For Receipt: backend ignores this and loads from DB by id.
/// For Paperless: backend uses these fields directly (paperless_trip_links has no doc data).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceData {
    pub datetime: Option<NaiveDateTime>,
    pub liters: Option<f64>,
    pub total_price_eur: Option<f64>,
    pub title: String,
    pub assignment_type: AssignmentType,
}

/// Source-agnostic view of an invoice.
/// All matching, sorting, and display code consumes this — never the concrete types.
pub trait Invoice {
    fn datetime(&self) -> Option<NaiveDateTime>;
    fn liters(&self) -> Option<f64>;
    fn total_price_eur(&self) -> Option<f64>;
    fn display_name(&self) -> &str;
    fn invoice_ref(&self) -> InvoiceRef;
    fn assignment_type(&self) -> Option<AssignmentType>;
}

/// Adapter for Paperless invoices when only the inline `InvoiceData` is available.
/// Used at the IPC boundary to give the compat check an `&dyn Invoice` for paperless docs.
pub struct PaperlessInvoiceView<'a> {
    pub id: i64,
    pub data: &'a InvoiceData,
}

impl<'a> Invoice for PaperlessInvoiceView<'a> {
    fn datetime(&self) -> Option<NaiveDateTime> { self.data.datetime }
    fn liters(&self) -> Option<f64> { self.data.liters }
    fn total_price_eur(&self) -> Option<f64> { self.data.total_price_eur }
    fn display_name(&self) -> &str { &self.data.title }
    fn invoice_ref(&self) -> InvoiceRef { InvoiceRef::Paperless(self.id) }
    fn assignment_type(&self) -> Option<AssignmentType> { Some(self.data.assignment_type) }
}

#[cfg(test)]
#[path = "invoice_tests.rs"]
mod tests;
```

**Step 2: Register the module**

In [src-tauri/core/src/lib.rs](../../src-tauri/core/src/lib.rs) find the list of `pub mod ...;` lines and add `pub mod invoice;` alphabetically (after `gemini` / before `models`).

**Step 3: Compile**

Run: `cd src-tauri && cargo build -p kniha-jazd-core`
Expected: success (only the new module — no consumers yet).

**Step 4: Commit**

```bash
git add src-tauri/core/src/invoice.rs src-tauri/core/src/lib.rs
git commit -m "feat(invoice): add Invoice trait + InvoiceRef + InvoiceData (Task 64)"
```

---

## Task 2: Implement Invoice for Receipt

**Files:**
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) (add impl block at bottom or near `Receipt`)
- Create: [src-tauri/core/src/invoice_tests.rs](../../src-tauri/core/src/invoice_tests.rs)

**Step 1: Write a failing test**

Create [src-tauri/core/src/invoice_tests.rs](../../src-tauri/core/src/invoice_tests.rs):

```rust
//! Tests for the Invoice trait (Task 64).
use super::*;
use chrono::NaiveDate;
use uuid::Uuid;
use crate::models::{AssignmentType, Receipt};

fn make_receipt() -> Receipt {
    Receipt {
        id: Uuid::nil(),
        file_name: "test.jpg".into(),
        file_path: "/x/test.jpg".into(),
        receipt_datetime: Some(NaiveDate::from_ymd_opt(2026, 5, 4).unwrap()
            .and_hms_opt(13, 24, 14).unwrap()),
        liters: Some(40.5),
        total_price_eur: Some(58.20),
        // Fill remaining fields with reasonable defaults — copy a real test factory if needed
        ..crate::db_tests::stub_receipt()
    }
}

#[test]
fn receipt_implements_invoice_trait_with_correct_field_mapping() {
    let r = make_receipt();
    let inv: &dyn Invoice = &r;
    assert_eq!(inv.datetime(), r.receipt_datetime);
    assert_eq!(inv.liters(), Some(40.5));
    assert_eq!(inv.total_price_eur(), Some(58.20));
    assert_eq!(inv.display_name(), "test.jpg");
    assert_eq!(inv.invoice_ref(), InvoiceRef::Receipt(Uuid::nil().to_string()));
}
```

If `db_tests::stub_receipt()` doesn't exist, look for any existing receipt factory in [db_tests.rs](../../src-tauri/core/src/db_tests.rs) (search for `Receipt {` literal) and call that instead. If no factory exists, build the Receipt struct literal inline — every field must be filled to compile.

Run: `cd src-tauri && cargo test -p kniha-jazd-core invoice::tests::receipt_implements_invoice_trait`
Expected: FAIL (no `Invoice for Receipt` impl yet).

**Step 2: Implement Invoice for Receipt**

In [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs), after the `Receipt` struct definition, add:

```rust
impl crate::invoice::Invoice for Receipt {
    fn datetime(&self) -> Option<chrono::NaiveDateTime> {
        self.receipt_datetime
    }
    fn liters(&self) -> Option<f64> {
        self.liters
    }
    fn total_price_eur(&self) -> Option<f64> {
        self.total_price_eur
    }
    fn display_name(&self) -> &str {
        &self.file_name
    }
    fn invoice_ref(&self) -> crate::invoice::InvoiceRef {
        crate::invoice::InvoiceRef::Receipt(self.id.to_string())
    }
    fn assignment_type(&self) -> Option<AssignmentType> {
        self.assignment_type
    }
}
```

**Step 3: Run test**

Run: `cd src-tauri && cargo test -p kniha-jazd-core invoice::tests::receipt_implements_invoice_trait`
Expected: PASS.

**Step 4: Commit**

```bash
git add src-tauri/core/src/models.rs src-tauri/core/src/invoice_tests.rs
git commit -m "feat(invoice): impl Invoice for Receipt (Task 64)"
```

---

## Task 3: Implement Invoice for PaperlessDoc

**Files:**
- Modify: [src-tauri/core/src/paperless.rs](../../src-tauri/core/src/paperless.rs)
- Modify: [src-tauri/core/src/invoice_tests.rs](../../src-tauri/core/src/invoice_tests.rs)

**Step 1: Write a failing test**

Append to [src-tauri/core/src/invoice_tests.rs](../../src-tauri/core/src/invoice_tests.rs):

```rust
use crate::paperless::PaperlessDoc;

#[test]
fn paperless_doc_implements_invoice_trait_with_uk_us_field_bridge() {
    let doc = PaperlessDoc {
        id: 435,
        title: "Tank Mol Bratislava".into(),
        tag_ids: vec![51], // fuel
        created: chrono::NaiveDate::from_ymd_opt(2026, 5, 4).unwrap(),
        total_amount: Some(58.20),  // UK→US bridge
        litres: Some(40.5),         // UK→US bridge
        receipt_datetime: chrono::NaiveDate::from_ymd_opt(2026, 5, 4).unwrap()
            .and_hms_opt(13, 24, 14),
    };
    let inv: &dyn Invoice = &doc;
    assert_eq!(inv.liters(), Some(40.5));
    assert_eq!(inv.total_price_eur(), Some(58.20));
    assert_eq!(inv.display_name(), "Tank Mol Bratislava");
    assert_eq!(inv.invoice_ref(), InvoiceRef::Paperless(435));
}
```

Run: `cd src-tauri && cargo test -p kniha-jazd-core invoice::tests::paperless_doc_implements_invoice`
Expected: FAIL (no impl yet).

**Step 2: Implement Invoice for PaperlessDoc**

In [src-tauri/core/src/paperless.rs](../../src-tauri/core/src/paperless.rs), after the `PaperlessDoc` struct, add:

```rust
impl crate::invoice::Invoice for PaperlessDoc {
    fn datetime(&self) -> Option<chrono::NaiveDateTime> {
        self.receipt_datetime
    }
    fn liters(&self) -> Option<f64> {
        self.litres // UK→US naming bridge
    }
    fn total_price_eur(&self) -> Option<f64> {
        self.total_amount // bridge: total_amount → total_price_eur
    }
    fn display_name(&self) -> &str {
        &self.title
    }
    fn invoice_ref(&self) -> crate::invoice::InvoiceRef {
        crate::invoice::InvoiceRef::Paperless(self.id)
    }
    fn assignment_type(&self) -> Option<crate::models::AssignmentType> {
        // PaperlessDoc itself doesn't carry assignment_type — derived from tags by caller.
        // Returning None is fine; compat check derives "is_fuel" from liters.is_some().
        None
    }
}
```

**Step 3: Run test**

Run: `cd src-tauri && cargo test -p kniha-jazd-core invoice::tests::paperless_doc_implements_invoice`
Expected: PASS.

**Step 4: Verify InvoiceRef serde shape**

Append to [invoice_tests.rs](../../src-tauri/core/src/invoice_tests.rs):

```rust
#[test]
fn invoice_ref_serde_shape_matches_design() {
    let r = InvoiceRef::Receipt("abc-123".into());
    let json = serde_json::to_string(&r).unwrap();
    assert_eq!(json, r#"{"source":"receipt","id":"abc-123"}"#);

    let p = InvoiceRef::Paperless(435);
    let json = serde_json::to_string(&p).unwrap();
    assert_eq!(json, r#"{"source":"paperless","id":435}"#);

    let round: InvoiceRef = serde_json::from_str(&json).unwrap();
    assert_eq!(round, p);
}
```

Run: `cd src-tauri && cargo test -p kniha-jazd-core invoice::tests::invoice_ref_serde_shape`
Expected: PASS.

**Step 5: Commit**

```bash
git add src-tauri/core/src/paperless.rs src-tauri/core/src/invoice_tests.rs
git commit -m "feat(invoice): impl Invoice for PaperlessDoc + serde round-trip test (Task 64)"
```

---

## Task 4: Move check_receipt_trip_compatibility into invoice.rs as check_invoice_trip_compatibility

This is the surgical refactor: same body, takes `&dyn Invoice` instead of `&Receipt`. The 12 existing receipt-side tests in [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) must keep passing — they're our regression net.

**Files:**
- Modify: [src-tauri/core/src/invoice.rs](../../src-tauri/core/src/invoice.rs) (add fn here)
- Modify: [src-tauri/core/src/commands_internal/receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs) (delete the moved fn, keep `CompatibilityResult` if used)

**Step 1: Move the function body**

Open [src-tauri/core/src/commands_internal/receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs) at line 480 and read `check_receipt_trip_compatibility` plus the helpers it calls (`is_same_date`, `get_datetime_mismatch_type`, `is_datetime_in_trip_range` — note: `is_datetime_in_trip_range` is in [statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs), **leave it where it is**, just import it).

Append to [src-tauri/core/src/invoice.rs](../../src-tauri/core/src/invoice.rs):

```rust
use crate::commands_internal::statistics::is_datetime_in_trip_range;
use crate::models::{AttachmentStatus, Trip};

/// Compat check result.
pub struct CompatibilityResult {
    pub can_attach: bool,
    pub status: String,
    pub mismatch_reason: Option<String>,
}

fn is_same_date(dt: NaiveDateTime, trip: &Trip) -> bool {
    dt.date() == trip.start_datetime.date()
}

fn get_datetime_mismatch_type(dt: Option<NaiveDateTime>, trip: &Trip) -> Option<&'static str> {
    match dt {
        Some(d) if is_datetime_in_trip_range(d, trip) => None,
        Some(d) if is_same_date(d, trip) => Some("time"),
        Some(_) => Some("date"),
        None => Some("date"),
    }
}

/// Check if invoice data matches trip's existing data.
/// Returns compatibility result with detailed mismatch reason.
/// Handles both FUEL invoices (has liters) and OTHER cost invoices (no liters).
pub fn check_invoice_trip_compatibility(
    invoice: &dyn Invoice,
    trip: &Trip,
) -> CompatibilityResult {
    let is_fuel = invoice.liters().is_some();

    if is_fuel {
        let trip_has_fuel = trip.fuel_liters.map(|l| l > 0.0).unwrap_or(false);
        if !trip_has_fuel {
            let status = match invoice.datetime() {
                Some(dt) if is_datetime_in_trip_range(dt, trip) => AttachmentStatus::Matches,
                Some(dt) if is_same_date(dt, trip) => AttachmentStatus::MatchesDate,
                _ => AttachmentStatus::Empty,
            };
            return CompatibilityResult {
                can_attach: true,
                status: status.as_str().to_string(),
                mismatch_reason: None,
            };
        }
        let r_liters = invoice.liters().unwrap();
        let r_price = invoice.total_price_eur().unwrap_or(0.0);
        let datetime_mismatch = get_datetime_mismatch_type(invoice.datetime(), trip);
        let liters_match = trip.fuel_liters.map(|fl| (fl - r_liters).abs() < 0.01).unwrap_or(false);
        let price_match = trip.fuel_cost_eur.map(|fc| (fc - r_price).abs() < 0.01).unwrap_or(false);

        if datetime_mismatch.is_none() && liters_match && price_match {
            return CompatibilityResult {
                can_attach: true,
                status: AttachmentStatus::Matches.as_str().to_string(),
                mismatch_reason: None,
            };
        }
        let dt_type = datetime_mismatch.unwrap_or("date");
        let mismatch = match (datetime_mismatch.is_some(), liters_match, price_match) {
            (false, false, false) => "liters_and_price",
            (false, false, true) => "liters",
            (false, true, false) => "price",
            (false, true, true) => unreachable!(),
            (true, false, false) => match dt_type { "time" => "time_and_liters_and_price", _ => "all" },
            (true, false, true)  => match dt_type { "time" => "time_and_liters", _ => "date_and_liters" },
            (true, true, false)  => match dt_type { "time" => "time_and_price", _ => "date_and_price" },
            (true, true, true)   => dt_type,
        };
        CompatibilityResult {
            can_attach: true,
            status: AttachmentStatus::Differs.as_str().to_string(),
            mismatch_reason: Some(mismatch.to_string()),
        }
    } else {
        // OTHER cost invoice — body identical to existing fn, but accessed via invoice methods
        let trip_has_other_costs = trip.other_costs_eur.map(|c| c > 0.0).unwrap_or(false);
        if !trip_has_other_costs {
            let status = match invoice.datetime() {
                Some(dt) if is_datetime_in_trip_range(dt, trip) => AttachmentStatus::Matches,
                Some(dt) if is_same_date(dt, trip) => AttachmentStatus::MatchesDate,
                _ => AttachmentStatus::Empty,
            };
            return CompatibilityResult {
                can_attach: true,
                status: status.as_str().to_string(),
                mismatch_reason: None,
            };
        }
        if let Some(r_price) = invoice.total_price_eur() {
            let datetime_mismatch = get_datetime_mismatch_type(invoice.datetime(), trip);
            let price_match = trip.other_costs_eur.map(|tc| (tc - r_price).abs() < 0.01).unwrap_or(false);
            if datetime_mismatch.is_none() && price_match {
                return CompatibilityResult {
                    can_attach: true,
                    status: AttachmentStatus::Matches.as_str().to_string(),
                    mismatch_reason: None,
                };
            }
            let dt_type = datetime_mismatch.unwrap_or("date");
            let mismatch = match (datetime_mismatch.is_some(), price_match) {
                (false, false) => "price",
                (false, true) => unreachable!(),
                (true, false) => match dt_type { "time" => "time_and_price", _ => "date_and_price" },
                (true, true)  => dt_type,
            };
            CompatibilityResult {
                can_attach: true,
                status: AttachmentStatus::Differs.as_str().to_string(),
                mismatch_reason: Some(mismatch.to_string()),
            }
        } else {
            CompatibilityResult {
                can_attach: true,
                status: AttachmentStatus::Empty.as_str().to_string(),
                mismatch_reason: None,
            }
        }
    }
}
```

**Step 2: Update receipts_cmd.rs to call the new fn**

In [src-tauri/core/src/commands_internal/receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs):
1. Delete the `fn check_receipt_trip_compatibility` body (lines ~480–609), the local `CompatibilityResult` struct (~450), and the helpers `is_same_date` and `get_datetime_mismatch_type`.
2. Inside `get_trips_for_receipt_assignment_internal`, change the call from `check_receipt_trip_compatibility(&receipt, &trip)` to `crate::invoice::check_invoice_trip_compatibility(&receipt as &dyn crate::invoice::Invoice, &trip)`.
3. Add `use crate::invoice::Invoice;` near the existing `use` block if needed (or qualify inline as above).

**Step 3: Compile + run all backend tests**

Run: `cd src-tauri && cargo test -p kniha-jazd-core`
Expected: all 195 backend tests pass — the 12 receipt-side compat tests in [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) exercise this path and should keep passing unchanged.

If any test fails, the move was not behavior-preserving — diff the original body in git history and align.

**Step 4: Commit**

```bash
git add src-tauri/core/src/invoice.rs src-tauri/core/src/commands_internal/receipts_cmd.rs
git commit -m "refactor(invoice): move compat check into invoice.rs, take &dyn Invoice (Task 64)"
```

---

## Task 5: Add Paperless-side compat tests

The 12 existing receipt tests prove the function works for receipts. Mirror them for Paperless to lock in the trait abstraction.

**Files:**
- Modify: [src-tauri/core/src/invoice_tests.rs](../../src-tauri/core/src/invoice_tests.rs)

**Step 1: Identify the receipt-side test patterns**

Read [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) lines 2256–2585 — the `test_get_trips_for_receipt_assignment_*` tests. Pick the 12 cases (empty trip / matches / mismatches: liters / price / date / time / combinations).

**Step 2: Write 12 Paperless-side parallel tests**

For each receipt-side case in [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs), write a parallel test in [invoice_tests.rs](../../src-tauri/core/src/invoice_tests.rs) that constructs a `PaperlessDoc` with equivalent data and asserts `check_invoice_trip_compatibility(&doc, &trip)` returns the same `CompatibilityResult`.

Example (the rest follow the same shape):

```rust
use crate::models::Trip;
use crate::invoice::check_invoice_trip_compatibility;
use crate::paperless::PaperlessDoc;
use chrono::{NaiveDate, NaiveDateTime};

fn fuel_doc(dt: NaiveDateTime, liters: f64, price: f64) -> PaperlessDoc {
    PaperlessDoc {
        id: 1,
        title: "test".into(),
        tag_ids: vec![51],
        created: dt.date(),
        total_amount: Some(price),
        litres: Some(liters),
        receipt_datetime: Some(dt),
    }
}

fn empty_trip(start: NaiveDateTime, end: NaiveDateTime) -> Trip {
    // Use the same factory pattern receipts tests use, or build inline.
    // See commands_tests.rs for an existing helper.
    crate::db_tests::stub_trip_with_datetime(start, end)
}

#[test]
fn paperless_compat_empty_trip_inside_time_range_matches() {
    let dt = NaiveDate::from_ymd_opt(2026, 5, 4).unwrap().and_hms_opt(13, 24, 14).unwrap();
    let doc = fuel_doc(dt, 40.5, 58.20);
    let trip = empty_trip(
        NaiveDate::from_ymd_opt(2026, 5, 4).unwrap().and_hms_opt(13, 0, 0).unwrap(),
        NaiveDate::from_ymd_opt(2026, 5, 4).unwrap().and_hms_opt(14, 0, 0).unwrap(),
    );
    let result = check_invoice_trip_compatibility(&doc, &trip);
    assert_eq!(result.status, "matches");
    assert_eq!(result.mismatch_reason, None);
    assert!(result.can_attach);
}

// ...11 more parallel cases — see commands_tests.rs:2256-2585 for the full set.
```

If `db_tests::stub_trip_with_datetime` doesn't exist, look for the trip factory used by the existing receipt-compat tests and reuse it (search `Trip {` in [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) near line 2256). Either reuse, expose via `pub(crate)`, or duplicate.

**Step 3: Run all 12 new tests**

Run: `cd src-tauri && cargo test -p kniha-jazd-core invoice::tests::paperless_compat`
Expected: 12 tests PASS.

**Step 4: Commit**

```bash
git add src-tauri/core/src/invoice_tests.rs
git commit -m "test(invoice): add 12 Paperless-side compat tests (Task 64)"
```

---

## Task 6: Add unified `_internal` functions for trip lookup, assign, and unassign

**Files:**
- Create: [src-tauri/core/src/commands_internal/invoices.rs](../../src-tauri/core/src/commands_internal/invoices.rs)
- Modify: [src-tauri/core/src/commands_internal/mod.rs](../../src-tauri/core/src/commands_internal/mod.rs)

**Step 1: Create the unified internal commands module**

Create [src-tauri/core/src/commands_internal/invoices.rs](../../src-tauri/core/src/commands_internal/invoices.rs):

```rust
//! Unified invoice command implementations (Task 64).
//!
//! Source dispatch confined to the three boundary functions here.
//! Beyond these, code consumes `&dyn Invoice` and never inspects the source.

use uuid::Uuid;

use crate::app_state::AppState;
use crate::check_read_only;
use crate::db::Database;
use crate::invoice::{
    check_invoice_trip_compatibility, Invoice, InvoiceData, InvoiceRef, PaperlessInvoiceView,
};
use crate::models::{AssignmentType, Receipt, Trip};

use super::receipts_cmd::TripForAssignment;

/// Get trips annotated with attachment status for a given invoice.
/// For Receipt: backend loads from DB by id (ignores `data`).
/// For Paperless: backend uses `data` directly (the inline doc payload from the frontend).
pub fn get_trips_for_invoice_assignment_internal(
    db: &Database,
    invoice_ref: &InvoiceRef,
    data: Option<&InvoiceData>,
    vehicle_id: &str,
    year: i32,
) -> Result<Vec<TripForAssignment>, String> {
    let trips = db
        .get_trips_for_vehicle_in_year(vehicle_id, year)
        .map_err(|e| e.to_string())?;

    match invoice_ref {
        InvoiceRef::Receipt(id) => {
            let receipt = db
                .get_receipt_by_id(id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Receipt not found".to_string())?;
            Ok(annotate_trips(&receipt, trips))
        }
        InvoiceRef::Paperless(id) => {
            let data = data.ok_or_else(|| {
                "InvoiceData required for Paperless invoices".to_string()
            })?;
            let view = PaperlessInvoiceView { id: *id, data };
            Ok(annotate_trips(&view, trips))
        }
    }
}

fn annotate_trips(invoice: &dyn Invoice, trips: Vec<Trip>) -> Vec<TripForAssignment> {
    trips
        .into_iter()
        .map(|trip| {
            let compat = check_invoice_trip_compatibility(invoice, &trip);
            TripForAssignment {
                trip,
                can_attach: compat.can_attach,
                attachment_status: compat.status,
                mismatch_reason: compat.mismatch_reason,
            }
        })
        .collect()
}

/// Assign an invoice to a trip.
/// For Receipt: delegates to existing receipt-assignment logic (populates trip.fuel_* / other_costs_*).
/// For Paperless: populates trip fuel/other_costs from inline data when trip is empty, then upserts the link.
#[allow(clippy::too_many_arguments)]
pub fn assign_invoice_to_trip_internal(
    db: &Database,
    app_state: &AppState,
    invoice_ref: &InvoiceRef,
    data: Option<&InvoiceData>,
    trip_id: &str,
    vehicle_id: &str,
    assignment_type: AssignmentType,
    mismatch_override: bool,
) -> Result<(), String> {
    check_read_only!(app_state);

    match invoice_ref {
        InvoiceRef::Receipt(id) => {
            super::receipts_cmd::assign_receipt_to_trip_internal(
                db,
                id,
                trip_id,
                vehicle_id,
                assignment_type.as_str(),
                mismatch_override,
            )
            .map(|_| ())
        }
        InvoiceRef::Paperless(id) => {
            let data = data.ok_or_else(|| {
                "InvoiceData required for Paperless invoices".to_string()
            })?;
            let _vehicle_uuid =
                Uuid::parse_str(vehicle_id).map_err(|e| format!("Invalid vehicle ID: {}", e))?;

            let trip = db
                .get_trip(trip_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Trip not found".to_string())?;

            // Populate trip data from invoice when trip side is empty (mirror receipt behavior)
            match assignment_type {
                AssignmentType::Fuel => {
                    let trip_has_fuel = trip.fuel_liters.map(|l| l > 0.0).unwrap_or(false);
                    if !trip_has_fuel {
                        let mut updated = trip.clone();
                        updated.fuel_liters = data.liters;
                        updated.fuel_cost_eur = data.total_price_eur;
                        updated.full_tank = true;
                        db.update_trip(&updated).map_err(|e| e.to_string())?;
                    }
                }
                AssignmentType::Other => {
                    let trip_has_other = trip.other_costs_eur.map(|c| c > 0.0).unwrap_or(false);
                    if !trip_has_other {
                        let mut updated = trip.clone();
                        updated.other_costs_eur = data.total_price_eur;
                        updated.other_costs_note = Some(data.title.clone());
                        db.update_trip(&updated).map_err(|e| e.to_string())?;
                    }
                }
            }

            db.upsert_paperless_link(trip_id, *id)
                .map_err(|e| e.to_string())?;
            // mismatch_override is currently ignored for Paperless — `paperless_trip_links`
            // has no override column. If users need this for Paperless, extend the schema
            // in a follow-up task.
            let _ = mismatch_override;
            Ok(())
        }
    }
}

/// Unassign an invoice from its trip.
pub fn unassign_invoice_internal(
    db: &Database,
    app_state: &AppState,
    invoice_ref: &InvoiceRef,
) -> Result<(), String> {
    check_read_only!(app_state);
    match invoice_ref {
        InvoiceRef::Receipt(id) => {
            super::receipts_cmd::unassign_receipt_internal(db, app_state, id.clone())
        }
        InvoiceRef::Paperless(id) => db
            .delete_paperless_link_for_doc(*id)
            .map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
#[path = "invoices_tests.rs"]
mod tests;
```

**Step 2: Register the module**

In [src-tauri/core/src/commands_internal/mod.rs](../../src-tauri/core/src/commands_internal/mod.rs) add `pub mod invoices;` alphabetically.

**Step 3: Compile**

Run: `cd src-tauri && cargo build -p kniha-jazd-core`
Expected: success.

**Step 4: Commit (no tests yet — they're the next step)**

```bash
git add src-tauri/core/src/commands_internal/invoices.rs src-tauri/core/src/commands_internal/mod.rs
git commit -m "feat(invoices): add unified _internal functions for invoice assignment (Task 64)"
```

---

## Task 7: Boundary tests for unified `_internal` functions

**Files:**
- Create: [src-tauri/core/src/commands_internal/invoices_tests.rs](../../src-tauri/core/src/commands_internal/invoices_tests.rs)

**Step 1: Write 6 boundary tests**

```rust
//! Boundary tests for invoices commands (Task 64).
use super::*;
use crate::app_state::AppState;
use crate::db::Database;
use crate::db_tests;
use crate::invoice::{InvoiceData, InvoiceRef};
use crate::models::AssignmentType;
use chrono::NaiveDate;

#[test]
fn get_trips_dispatches_receipt_path_loads_from_db() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    let trip = db_tests::seed_test_trip(&db, &v.id.to_string());
    let receipt_id = db_tests::seed_fuel_receipt(&db, &v.id, /* liters */ 40.0, /* price */ 58.0);

    let result = get_trips_for_invoice_assignment_internal(
        &db,
        &InvoiceRef::Receipt(receipt_id.to_string()),
        None,                       // ignored for Receipt path
        &v.id.to_string(),
        2026,
    ).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].trip.id, trip);
}

#[test]
fn get_trips_dispatches_paperless_path_uses_inline_data() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    db_tests::seed_test_trip(&db, &v.id.to_string());
    let data = InvoiceData {
        datetime: NaiveDate::from_ymd_opt(2026, 5, 4).unwrap().and_hms_opt(13, 24, 14),
        liters: Some(40.5), total_price_eur: Some(58.20),
        title: "Doc 435".into(), assignment_type: AssignmentType::Fuel,
    };
    let result = get_trips_for_invoice_assignment_internal(
        &db, &InvoiceRef::Paperless(435), Some(&data),
        &v.id.to_string(), 2026,
    ).unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn get_trips_paperless_without_data_errors() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    let err = get_trips_for_invoice_assignment_internal(
        &db, &InvoiceRef::Paperless(435), None, &v.id.to_string(), 2026,
    ).unwrap_err();
    assert!(err.to_lowercase().contains("invoicedata required"));
}

#[test]
fn assign_paperless_populates_trip_fuel_when_empty() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    let data = InvoiceData {
        datetime: NaiveDate::from_ymd_opt(2026, 5, 4).unwrap().and_hms_opt(13, 24, 14),
        liters: Some(40.5), total_price_eur: Some(58.20),
        title: "Doc".into(), assignment_type: AssignmentType::Fuel,
    };
    assign_invoice_to_trip_internal(
        &db, &app_state, &InvoiceRef::Paperless(435), Some(&data),
        &trip_id, &v.id.to_string(), AssignmentType::Fuel, false,
    ).unwrap();
    let trip = db.get_trip(&trip_id).unwrap().unwrap();
    assert_eq!(trip.fuel_liters, Some(40.5));
    assert_eq!(trip.fuel_cost_eur, Some(58.20));
    assert_eq!(db.get_paperless_link_for_doc(435).unwrap(), Some(trip_id));
}

#[test]
fn assign_invoice_blocked_when_read_only() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    app_state.enable_read_only("test");
    let data = InvoiceData {
        datetime: None, liters: None, total_price_eur: None,
        title: "X".into(), assignment_type: AssignmentType::Other,
    };
    let err = assign_invoice_to_trip_internal(
        &db, &app_state, &InvoiceRef::Paperless(435), Some(&data),
        &trip_id, &v.id.to_string(), AssignmentType::Other, false,
    ).unwrap_err();
    assert!(err.to_lowercase().contains("read") || err.to_lowercase().contains("čítanie"));
}

#[test]
fn unassign_dispatches_both_sources() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    db.upsert_paperless_link(&trip_id, 435).unwrap();
    unassign_invoice_internal(&db, &app_state, &InvoiceRef::Paperless(435)).unwrap();
    assert_eq!(db.get_paperless_link_for_doc(435).unwrap(), None);
}
```

If `db_tests::seed_fuel_receipt` doesn't exist, look for an equivalent helper or build a Receipt inline using `db.create_receipt`. The seed helpers must match what the existing tests use — search [db_tests.rs](../../src-tauri/core/src/db_tests.rs) first.

**Step 2: Run boundary tests**

Run: `cd src-tauri && cargo test -p kniha-jazd-core commands_internal::invoices::tests`
Expected: 6 tests PASS.

**Step 3: Run all backend tests**

Run: `cd src-tauri && cargo test -p kniha-jazd-core`
Expected: ALL tests PASS — no regressions.

**Step 4: Commit**

```bash
git add src-tauri/core/src/commands_internal/invoices_tests.rs
git commit -m "test(invoices): boundary tests for unified _internal functions (Task 64)"
```

---

## Task 8: Wire unified Tauri commands

**Files:**
- Create: [src-tauri/desktop/src/commands/invoices.rs](../../src-tauri/desktop/src/commands/invoices.rs)
- Modify: [src-tauri/desktop/src/commands/mod.rs](../../src-tauri/desktop/src/commands/mod.rs) (export)
- Modify: [src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs) (register commands)
- Modify: [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) (server-mode dispatch — sync commands)

**Step 1: Create the desktop wrapper**

Create [src-tauri/desktop/src/commands/invoices.rs](../../src-tauri/desktop/src/commands/invoices.rs):

```rust
//! Tauri command wrappers for unified invoice assignment (Task 64).

use std::sync::Arc;
use tauri::State;

use kniha_jazd_core::app_state::AppState;
use kniha_jazd_core::commands_internal::invoices as inner;
use kniha_jazd_core::commands_internal::receipts_cmd::TripForAssignment;
use kniha_jazd_core::db::Database;
use kniha_jazd_core::invoice::{InvoiceData, InvoiceRef};
use kniha_jazd_core::models::AssignmentType;

#[tauri::command]
pub fn get_trips_for_invoice_assignment(
    db: State<'_, Arc<Database>>,
    invoice_ref: InvoiceRef,
    invoice_data: Option<InvoiceData>,
    vehicle_id: String,
    year: i32,
) -> Result<Vec<TripForAssignment>, String> {
    inner::get_trips_for_invoice_assignment_internal(
        &db, &invoice_ref, invoice_data.as_ref(), &vehicle_id, year,
    )
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn assign_invoice_to_trip(
    db: State<'_, Arc<Database>>,
    app_state: State<'_, Arc<AppState>>,
    invoice_ref: InvoiceRef,
    invoice_data: Option<InvoiceData>,
    trip_id: String,
    vehicle_id: String,
    assignment_type: AssignmentType,
    mismatch_override: bool,
) -> Result<(), String> {
    inner::assign_invoice_to_trip_internal(
        &db, &app_state, &invoice_ref, invoice_data.as_ref(),
        &trip_id, &vehicle_id, assignment_type, mismatch_override,
    )
}

#[tauri::command]
pub fn unassign_invoice(
    db: State<'_, Arc<Database>>,
    app_state: State<'_, Arc<AppState>>,
    invoice_ref: InvoiceRef,
) -> Result<(), String> {
    inner::unassign_invoice_internal(&db, &app_state, &invoice_ref)
}
```

**Step 2: Register module**

In [src-tauri/desktop/src/commands/mod.rs](../../src-tauri/desktop/src/commands/mod.rs) add `pub mod invoices;` and `pub use invoices::*;`.

**Step 3: Register Tauri commands**

In [src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs) find `invoke_handler` and add the three new commands. Keep the four old ones for now — they'll be deleted in Task 12 once the frontend stops calling them.

```rust
// inside invoke_handler! macro:
commands::get_trips_for_invoice_assignment,
commands::assign_invoice_to_trip,
commands::unassign_invoice,
```

**Step 4: Register server-mode dispatch**

Read [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) lines 543–610 (the existing receipt-assignment dispatch entries). Add three parallel branches for the new commands following the same pattern. Use `serde_json::from_value` to parse `InvoiceRef` and `InvoiceData` from the request body.

```rust
"get_trips_for_invoice_assignment" => {
    let invoice_ref: InvoiceRef = serde_json::from_value(args.get("invoiceRef")
        .cloned().ok_or("missing invoiceRef")?).map_err(|e| e.to_string())?;
    let invoice_data: Option<InvoiceData> = match args.get("invoiceData") {
        Some(v) if !v.is_null() => Some(serde_json::from_value(v.clone()).map_err(|e| e.to_string())?),
        _ => None,
    };
    let vehicle_id = args["vehicleId"].as_str().ok_or("missing vehicleId")?.to_string();
    let year = args["year"].as_i64().ok_or("missing year")? as i32;
    let v = crate::commands_internal::invoices::get_trips_for_invoice_assignment_internal(
        &db, &invoice_ref, invoice_data.as_ref(), &vehicle_id, year,
    ).map_err(...);
    serde_json::to_value(v)?
}
// ...mirror for assign_invoice_to_trip and unassign_invoice
```

(Match the exact error-handling pattern in the surrounding dispatcher entries — don't invent a new style.)

**Step 5: Compile + run all backend tests**

Run: `cd src-tauri && cargo build && cargo test`
Expected: build succeeds; tests pass.

**Step 6: Commit**

```bash
git add src-tauri/desktop/src/commands/invoices.rs \
        src-tauri/desktop/src/commands/mod.rs \
        src-tauri/desktop/src/lib.rs \
        src-tauri/core/src/server/dispatcher.rs
git commit -m "feat(invoices): wire unified Tauri commands + server dispatch (Task 64)"
```

---

## Task 9: Frontend Invoice abstraction

**Files:**
- Create: [src/lib/invoice.ts](../../src/lib/invoice.ts)
- Modify: [src/lib/types.ts](../../src/lib/types.ts) (add `InvoiceRef` + `InvoiceData`)
- Modify: [src/lib/api.ts](../../src/lib/api.ts) (add unified API fns)

**Step 1: Add types to types.ts**

Append to [src/lib/types.ts](../../src/lib/types.ts):

```ts
export type InvoiceRef =
    | { source: 'receipt'; id: string }
    | { source: 'paperless'; id: number };

export interface InvoiceData {
    datetime: string | null;          // ISO-8601 NaiveDateTime
    liters: number | null;
    totalPriceEur: number | null;
    title: string;
    assignmentType: 'Fuel' | 'Other';
}
```

**Step 2: Create the adapter module**

Create [src/lib/invoice.ts](../../src/lib/invoice.ts):

```ts
import type { Receipt, PaperlessInvoiceRow, InvoiceRef, InvoiceData } from './types';

export interface Invoice {
    getDateTime(): string | null;
    getLiters(): number | null;
    getPrice(): number | null;
    getDisplayName(): string;
    getRef(): InvoiceRef;
    /** Inline payload sent to backend with InvoiceRef. Receipts return null (backend loads from DB). */
    getData(): InvoiceData | null;
    /** For UI: pre-selected assignment type ("Fuel" if liters > 0, else "Other"). */
    looksLikeFuel(): boolean;
    /** Source-specific extras the modal still needs (e.g. mismatch tooltips). */
    getRaw(): Receipt | PaperlessInvoiceRow;
}

class ReceiptInvoice implements Invoice {
    constructor(private r: Receipt) {}
    getDateTime() { return this.r.receiptDatetime; }
    getLiters()   { return this.r.liters; }
    getPrice()    { return this.r.totalPriceEur; }
    getDisplayName() { return this.r.fileName; }
    getRef(): InvoiceRef { return { source: 'receipt', id: this.r.id }; }
    getData() { return null; }
    looksLikeFuel() { return this.r.liters !== null && this.r.liters > 0; }
    getRaw() { return this.r; }
}

class PaperlessInvoice implements Invoice {
    constructor(private p: PaperlessInvoiceRow) {}
    getDateTime() { return this.p.receiptDatetime; }
    getLiters()   { return this.p.liters; }
    getPrice()    { return this.p.totalPriceEur; }
    getDisplayName() { return this.p.title; }
    getRef(): InvoiceRef { return { source: 'paperless', id: this.p.paperlessDocumentId }; }
    getData(): InvoiceData {
        return {
            datetime: this.p.receiptDatetime,
            liters: this.p.liters,
            totalPriceEur: this.p.totalPriceEur,
            title: this.p.title,
            assignmentType: this.p.assignmentType,
        };
    }
    looksLikeFuel() { return this.p.assignmentType === 'Fuel'; }
    getRaw() { return this.p; }
}

/** The ONE type guard. Source-checking elsewhere is a smell. */
export function adaptInvoice(source: Receipt | PaperlessInvoiceRow): Invoice {
    return 'paperlessDocumentId' in source
        ? new PaperlessInvoice(source)
        : new ReceiptInvoice(source);
}
```

**Step 3: Add unified API functions**

Append to [src/lib/api.ts](../../src/lib/api.ts):

```ts
import type { InvoiceRef, InvoiceData } from './types';

export async function getTripsForInvoiceAssignment(
    invoiceRef: InvoiceRef,
    invoiceData: InvoiceData | null,
    vehicleId: string,
    year: number,
): Promise<TripForAssignment[]> {
    return await apiCall('get_trips_for_invoice_assignment', {
        invoiceRef, invoiceData, vehicleId, year,
    });
}

export async function assignInvoiceToTrip(
    invoiceRef: InvoiceRef,
    invoiceData: InvoiceData | null,
    tripId: string,
    vehicleId: string,
    assignmentType: 'Fuel' | 'Other',
    mismatchOverride: boolean = false,
): Promise<void> {
    return await apiCall('assign_invoice_to_trip', {
        invoiceRef, invoiceData, tripId, vehicleId, assignmentType, mismatchOverride,
    });
}

export async function unassignInvoice(invoiceRef: InvoiceRef): Promise<void> {
    return await apiCall('unassign_invoice', { invoiceRef });
}
```

(Don't delete the old [api.ts](../../src/lib/api.ts) functions yet — components still call them. Removed in Task 12.)

**Step 4: Type-check + smoke-test**

Run: `npm run check`
Expected: zero errors.

**Step 5: Commit**

```bash
git add src/lib/invoice.ts src/lib/types.ts src/lib/api.ts
git commit -m "feat(invoice): TS Invoice interface + adapters + unified api fns (Task 64)"
```

---

## Task 10: Refactor TripSelectorModal to consume Invoice

**Files:**
- Modify: [src/lib/components/TripSelectorModal.svelte](../../src/lib/components/TripSelectorModal.svelte)

**Step 1: Refactor props + handlers**

Open [src/lib/components/TripSelectorModal.svelte](../../src/lib/components/TripSelectorModal.svelte). Make these surgical changes — avoid rewrites that produce churn the reviewer can't track:

```ts
// OLD:
import type { Trip, Receipt, TripForAssignment, MismatchReason, AssignmentType } from '$lib/types';
import { getTripsForReceiptAssignment } from '$lib/api';

interface Props {
    receipt: Receipt;
    onSelect: (result: AssignmentResult) => void;
    onClose: () => void;
}
let { receipt, onSelect, onClose }: Props = $props();

// NEW:
import type { Trip, TripForAssignment, MismatchReason, AssignmentType } from '$lib/types';
import { getTripsForInvoiceAssignment } from '$lib/api';
import type { Invoice } from '$lib/invoice';

interface Props {
    invoice: Invoice;
    onSelect: (result: AssignmentResult) => void;
    onClose: () => void;
}
let { invoice, onSelect, onClose }: Props = $props();
```

**Step 2: Update field accesses**

Replace `receipt.receiptDatetime` → `invoice.getDateTime()`, `receipt.liters` → `invoice.getLiters()`, `receipt.totalPriceEur` → `invoice.getPrice()`, `receipt.fileName` → `invoice.getDisplayName()`. The `receipt-info` block becomes:

```svelte
<div class="receipt-info">
    <span class="file-name">{invoice.getDisplayName()}</span>
    <span class="separator">|</span>
    <span>{invoice.getLiters()?.toFixed(2) ?? '??'} L</span>
    <span class="separator">|</span>
    <span>{invoice.getPrice()?.toFixed(2) ?? '??'} EUR</span>
    {#if invoice.getDateTime()}
        <span class="separator">|</span>
        <span>{formatDate(invoice.getDateTime()!)}</span>
    {/if}
</div>
```

Replace `looksLikeFuel = $derived(receipt.liters !== null && receipt.liters > 0)` with `looksLikeFuel = $derived(invoice.looksLikeFuel())`.

**Step 3: Update API call**

```ts
async function loadTrips() {
    const vehicle = $activeVehicleStore;
    if (!vehicle) { error = $LL.tripSelector.noVehicleSelected(); loading = false; return; }
    loading = true;
    try {
        const items = await getTripsForInvoiceAssignment(
            invoice.getRef(), invoice.getData(),
            vehicle.id, $selectedYearStore,
        );
        tripItems = items.sort((a, b) => {
            const aDiff = dateProximity(getTripDate(a.trip), invoice.getDateTime());
            const bDiff = dateProximity(getTripDate(b.trip), invoice.getDateTime());
            return aDiff - bDiff;
        });
    } catch (e) {
        console.error('Failed to load trips:', e);
        error = $LL.tripSelector.loadError();
    } finally { loading = false; }
}
```

Update `getMismatchDetailTooltip` similarly — it was reading `receipt.liters`, `receipt.totalPriceEur`, etc. Use `invoice.getLiters()`, `invoice.getPrice()`, `invoice.getDateTime()`.

**Step 4: Type-check**

Run: `npm run check`
Expected: zero errors in [TripSelectorModal.svelte](../../src/lib/components/TripSelectorModal.svelte). Errors in [+page.svelte](../../src/routes/doklady/+page.svelte) are expected and fixed in Task 11.

**Step 5: Commit**

```bash
git add src/lib/components/TripSelectorModal.svelte
git commit -m "refactor(TripSelectorModal): consume Invoice interface, drop Receipt coupling (Task 64)"
```

---

## Task 11: Refactor doklady/+page.svelte — delete inline modal, route both flows through TripSelectorModal

**Files:**
- Modify: [src/routes/doklady/+page.svelte](../../src/routes/doklady/+page.svelte)

**Step 1: Update state + handlers**

Replace `receiptToAssign` with a unified `invoiceToAssign: Invoice | null`, drop `paperlessTripPickerDocId` and `paperlessTrips`.

```ts
import { adaptInvoice, type Invoice } from '$lib/invoice';
import { unassignInvoice, assignInvoiceToTrip } from '$lib/api';

let invoiceToAssign = $state<Invoice | null>(null);
// REMOVE: let receiptToAssign = $state<Receipt | null>(null);
// REMOVE: let paperlessTripPickerDocId = $state<number | null>(null);
// REMOVE: let paperlessTrips = $state<Trip[]>([]);
```

In `loadPaperlessRows` remove the line `paperlessTrips = await api.getTripsForYear(...)` — no longer needed (the modal fetches trips itself).

**Step 2: Update click handlers**

```ts
function handleAssignClick(receipt: Receipt) {
    if (!$activeVehicleStore) { toast.error($LL.toast.errorSelectVehicleFirst()); return; }
    invoiceToAssign = adaptInvoice(receipt);
}
function handleAssignPaperlessClick(row: PaperlessInvoiceRow) {
    if (!$activeVehicleStore) { toast.error($LL.toast.errorSelectVehicleFirst()); return; }
    invoiceToAssign = adaptInvoice(row);
}
```

In the Paperless row template change the assign button:

```svelte
<button class="button-small primary" data-test="assign-btn"
    onclick={() => handleAssignPaperlessClick(row)}>
    {$LL.receipts.assignToTrip()}
</button>
```

**Step 3: Unify the assignment handler**

Replace `handleAssignToTrip` (receipt-only) and `handleAssignPaperless` (Paperless-only) with one:

```ts
async function handleAssignInvoice(result: { trip: Trip; assignmentType: 'Fuel' | 'Other'; mismatchOverride: boolean }) {
    if (!invoiceToAssign || !$activeVehicleStore) return;
    try {
        await assignInvoiceToTrip(
            invoiceToAssign.getRef(),
            invoiceToAssign.getData(),
            result.trip.id,
            $activeVehicleStore.id,
            result.assignmentType,
            result.mismatchOverride,
        );
        await refreshReceiptData();
        invoiceToAssign = null;
        toast.success($LL.toast.receiptAssigned());
    } catch (error) {
        console.error('Failed to assign invoice:', error);
        toast.error($LL.toast.errorAssignReceipt({ error: String(error) }));
    }
}

async function handleUnassignInvoice(invoice: Invoice) {
    try {
        await unassignInvoice(invoice.getRef());
        await refreshReceiptData();
        toast.success($LL.toast.receiptUnassigned());
    } catch (error) {
        console.error('Failed to unassign invoice:', error);
        toast.error($LL.toast.errorUnassignReceipt());
    }
}
```

Update the existing receipt-side unassign confirm modal to call `handleUnassignInvoice(adaptInvoice(receiptToUnassign))`. Update Paperless row's "Unassign" button onClick from `handleUnassignPaperless` to `handleUnassignInvoice(adaptInvoice(row))`.

**Step 4: Delete the inline Paperless modal**

Delete the entire `{#if paperlessTripPickerDocId !== null}` block (lines ~1081–1126 of the current file).

Delete the modal-related CSS (`.modal-overlay`, `.modal`, `.trip-list`, `.trip-item`, `.modal-actions` near the bottom) — no other element on this page uses them. (Verify by searching the file for these class names.)

**Step 5: Update the TripSelectorModal invocation**

Replace the `{#if receiptToAssign}` block:

```svelte
{#if invoiceToAssign}
    <TripSelectorModal
        invoice={invoiceToAssign}
        onSelect={handleAssignInvoice}
        onClose={() => (invoiceToAssign = null)}
    />
{/if}
```

**Step 6: Type-check**

Run: `npm run check`
Expected: zero errors.

**Step 7: Commit**

```bash
git add src/routes/doklady/+page.svelte
git commit -m "refactor(doklady): unify invoice assignment via TripSelectorModal (Task 64)"
```

---

## Task 12: Delete obsolete backend commands

**Files:**
- Modify: [src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs)
- Modify: [src-tauri/desktop/src/commands/receipts_cmd.rs](../../src-tauri/desktop/src/commands/receipts_cmd.rs)
- Modify: [src-tauri/desktop/src/commands/integrations.rs](../../src-tauri/desktop/src/commands/integrations.rs)
- Modify: [src-tauri/core/src/commands_internal/receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs)
- Modify: [src-tauri/core/src/commands_internal/paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs)
- Modify: [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs)
- Modify: [src-tauri/core/src/server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs)
- Modify: [src/lib/api.ts](../../src/lib/api.ts)

**Step 1: Confirm no frontend references remain**

```bash
grep -rn "assign_receipt_to_trip\|get_trips_for_receipt_assignment\|unassign_receipt\|assign_paperless_doc_to_trip\|unassign_paperless_doc" src/
```

Expected: only matches inside [src/lib/api.ts](../../src/lib/api.ts) (the old api fns we'll delete).

**Step 2: Delete obsolete frontend api functions**

Delete from [src/lib/api.ts](../../src/lib/api.ts):
- `unassignReceipt`
- `assignReceiptToTrip`
- `getTripsForReceiptAssignment`
- `assignPaperlessDocToTrip`
- `unassignPaperlessDoc`

Run: `npm run check`
Expected: zero errors (the [+page.svelte](../../src/routes/doklady/+page.svelte) and [TripSelectorModal.svelte](../../src/lib/components/TripSelectorModal.svelte) already use the unified fns).

**Step 3: Delete Tauri command wrappers**

Remove from [src-tauri/desktop/src/commands/receipts_cmd.rs](../../src-tauri/desktop/src/commands/receipts_cmd.rs):
- `pub fn unassign_receipt`
- `pub fn assign_receipt_to_trip`
- `pub fn get_trips_for_receipt_assignment`

Remove from [src-tauri/desktop/src/commands/integrations.rs](../../src-tauri/desktop/src/commands/integrations.rs):
- `pub fn assign_paperless_doc_to_trip`
- `pub fn unassign_paperless_doc`

Remove from [src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs) `invoke_handler`:
- `commands::unassign_receipt`
- `commands::assign_receipt_to_trip`
- `commands::get_trips_for_receipt_assignment`
- `commands::assign_paperless_doc_to_trip`
- `commands::unassign_paperless_doc`

**Step 4: Delete obsolete dispatcher entries**

Remove from [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs):
- `"unassign_receipt" => { ... }`
- `"assign_receipt_to_trip" => { ... }`
- `"get_trips_for_receipt_assignment" => { ... }`

Remove from [src-tauri/core/src/server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs):
- `"assign_paperless_doc_to_trip" => { ... }`
- `"unassign_paperless_doc" => { ... }`

**Step 5: Delete obsolete _internal functions**

In [src-tauri/core/src/commands_internal/receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs):
- KEEP `assign_receipt_to_trip_internal` (called by `invoices::assign_invoice_to_trip_internal`)
- KEEP `unassign_receipt_internal` (called by `invoices::unassign_invoice_internal`)
- KEEP `get_trips_for_receipt_assignment_internal` IF still called by tests; otherwise delete. (Search: `rg get_trips_for_receipt_assignment_internal src-tauri/`)
- DELETE the receipt-specific helpers IF moved to [invoice.rs](../../src-tauri/core/src/invoice.rs) already — confirm no leftovers.

In [src-tauri/core/src/commands_internal/paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs):
- DELETE `assign_paperless_doc_to_trip_internal`
- DELETE `unassign_paperless_doc_internal`
- KEEP `get_paperless_invoices_internal` and `list_paperless_custom_fields_internal` (still used)

**Step 6: Update tests that called deleted functions**

In [src-tauri/core/src/commands_internal/paperless_cmd_tests.rs](../../src-tauri/core/src/commands_internal/paperless_cmd_tests.rs):
- Update `assign_paperless_doc_blocked_when_read_only`, `assign_paperless_doc_persists_link`, `unassign_paperless_doc_removes_link` to call `invoices::assign_invoice_to_trip_internal` / `unassign_invoice_internal` with `InvoiceRef::Paperless(435)`.
- OR: move these tests to [invoices_tests.rs](../../src-tauri/core/src/commands_internal/invoices_tests.rs) if cleaner.

**Step 7: Compile + test**

Run: `cd src-tauri && cargo build && cargo test`
Expected: all tests pass, no warnings about unused functions.

**Step 8: Commit**

```bash
git add -u
git commit -m "refactor: delete obsolete receipt+paperless commands superseded by unified invoice cmds (Task 64)"
```

---

## Task 13: Rewrite Paperless integration test for unified flow

**Files:**
- Modify: [tests/integration/specs/tier2/paperless-integration.spec.ts](../../tests/integration/specs/tier2/paperless-integration.spec.ts) (or whatever the existing Paperless integration spec is named)

**Step 1: Locate the spec**

```bash
ls tests/integration/specs/tier2/ | grep paperless
```

**Step 2: Update the assign-flow assertions**

The unified picker shows trip-list items annotated with match status. Update:
- `data-test="paperless-trip-item"` → `data-test="trip-item"` (or whatever [TripSelectorModal.svelte](../../src/lib/components/TripSelectorModal.svelte) uses; check the file)
- Assert that trips appear sorted by date proximity to the doc's `receipt_datetime`
- Assert the Fuel/Other step appears after picking a trip
- Assert the assignment persists after confirm

Read [.claude/rules/integration-tests.md](../../.claude/rules/integration-tests.md) before writing — that file holds the project's integration-test conventions.

**Step 3: Run the focused spec (Tier 2)**

```bash
WDIO_SERVER_MODE=1 npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/paperless-integration.spec.ts
```

Expected: PASS.

**Step 4: Commit**

```bash
git add tests/integration/specs/tier2/paperless-integration.spec.ts
git commit -m "test(integration): paperless invoice picker uses unified flow (Task 64)"
```

---

## Task 14: Final verification + docs

**Files:**
- Modify: [CHANGELOG.md](../../CHANGELOG.md)
- Modify: [_tasks/index.md](../index.md) (move row from Active to Completed)

**Step 1: Run full test suite**

```bash
npm run test:all
```

Expected: all backend tests pass; all integration tests pass.

**Step 2: Smoke-test in dev mode**

```bash
npm run tauri:dev
```

Manually:
- Toggle Paperless mode ON in Settings, verify rows render
- Click "Assign to trip" on a Paperless row → unified [TripSelectorModal](../../src/lib/components/TripSelectorModal.svelte) opens
- Verify trip list is sorted by date proximity to the doc's `receipt_datetime`
- Verify trips with same date as the doc are highlighted
- Pick a trip with empty fuel data → Fuel/Other step → confirm Fuel
- Re-fetch `paperlessRows` and verify `tripId` is set
- Verify trip table shows `fuel_liters` populated from the Paperless doc

If any of these fail, do NOT mark task complete — go back and fix.

**Step 3: Update CHANGELOG**

Add to `[Unreleased]` in [CHANGELOG.md](../../CHANGELOG.md):

```markdown
### Changed
- Unified the trip-picker for invoice assignment: both local receipts and Paperless documents now use the same modal with proximity sort, mismatch warnings, Fuel/Other step, and override flow (Task 64).
```

**Step 4: Update task index**

In [_tasks/index.md](../index.md) move the Task 64 row from "Active Tasks" to "Completed Tasks", flip the icon to ✅.

**Step 5: Final commit**

```bash
git add CHANGELOG.md _tasks/index.md
git commit -m "docs: changelog + task-index for unified invoice picker (Task 64)"
```

**Step 6: Move task folder to _done**

```bash
git mv _tasks/64-unified-invoice-picker _tasks/_done/64-unified-invoice-picker
git commit -m "chore(tasks): archive Task 64 (unified invoice picker)"
```

Update the link in [_tasks/index.md](../index.md) from `_tasks/64-unified-invoice-picker/` to `_tasks/_done/64-unified-invoice-picker/`.

---

## Acceptance Criteria (from [01-task.md](./01-task.md))

- [ ] One [TripSelectorModal.svelte](../../src/lib/components/TripSelectorModal.svelte) handles both invoice sources
- [ ] Inline Paperless modal in [+page.svelte](../../src/routes/doklady/+page.svelte) is removed
- [ ] Backend: 3 unified commands replace the original 5 (note: design said 4 — re-counted: `get_trips_for_receipt_assignment`, `assign_receipt_to_trip`, `unassign_receipt`, `assign_paperless_doc_to_trip`, `unassign_paperless_doc` = 5; `get_trips_for_year` is kept since it has other callers besides the picker)
- [ ] Source discrimination only in [commands_internal/invoices.rs](../../src-tauri/core/src/commands_internal/invoices.rs) (Rust) and `adaptInvoice` (TS)
- [ ] All 12 receipt-side compat tests pass with the renamed function
- [ ] 12 parallel paperless-side compat tests added
- [ ] Integration test for unified flow passes
- [ ] `npm run test:all` passes

## Risks (from [02-design.md](./02-design.md))

| Risk | Mitigation in Plan |
|------|--------------------|
| Refactored compat check has subtle regressions | Task 4 reuses the existing 12 receipt tests as the regression net |
| Object-safety violation if trait grows | Trait kept minimal (Task 1); all methods take `&self`, return concrete types |
| `InvoiceRef` serde shape mismatch | Task 3 step 4 round-trips both variants against fixed JSON |
| Paperless toggle-off mode breaks | Paperless rows aren't rendered when toggled off → assign button never shown → `InvoiceRef::Paperless` never sent. Local-only path unchanged |
| Loss of mismatch_override for Paperless | Documented in Task 6 — `paperless_trip_links` has no override column. Override flag is accepted but ignored for Paperless. Schema extension is a follow-up if users need it. |
