**Date:** 2026-05-04
**Subject:** Unified Invoice Picker — Architecture
**Status:** Approved (implementation deferred)

# Design: Unified Invoice Picker

## Context

See [01-task.md](./01-task.md) for problem statement. This document captures the architecture validated through brainstorming on 2026-05-04.

## Architecture Overview

```
                     ┌──────────────────────────┐
                     │    TripSelectorModal     │  ← ONE component
                     │   (operates on Invoice)  │
                     └────────────┬─────────────┘
                                  │ invoice: Invoice (TS interface)
                  ┌───────────────┴────────────────┐
                  ▼                                ▼
        ┌─────────────────┐              ┌─────────────────┐
        │ ReceiptInvoice  │              │ PaperlessInvoice│
        │  (TS adapter)   │              │  (TS adapter)   │
        └────────┬────────┘              └────────┬────────┘
                 └────────────┬───────────────────┘
                              ▼
                  invoice_ref: { source, id }
                              │
                  ┌───────────▼────────────┐
                  │  get_trips_for_invoice │  ← unified backend cmd
                  │       _assignment      │
                  └───────────┬────────────┘
                              ▼
                  ┌───────────────────────┐
                  │  load source-specific │   match invoice_ref
                  │  data → impl Invoice  │
                  └───────────┬───────────┘
                              ▼
            check_invoice_trip_compatibility(&dyn Invoice)
                  (single Rust source of truth)
```

## Design Patterns

- **Adapter Pattern** — each source has one adapter (Rust `impl Invoice for ...`; TS `class XInvoice implements Invoice`). The adapter is the *only* place that touches source-specific field names.
- **Polymorphism** — Rust trait, TS interface. Everywhere except the boundary, code consumes `Invoice` and never the concrete type.
- **Boundary Pattern** — source discrimination allowed in exactly three Rust matches (`load_invoice`, `save_invoice_assignment`, `unassign_invoice`) and one TS type guard (`adaptInvoice`). Outside these, source-checking is forbidden.
- **Open/Closed Principle** — adding a third invoice source = one trait impl + one match arm in each boundary function.

## Rust Trait + Types

New file [src-tauri/core/src/invoice.rs](../../src-tauri/core/src/invoice.rs):

```rust
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// Tagged reference used at the IPC boundary and DB lookup.
/// Serializes to: { "source": "receipt", "id": "uuid" }
///              or { "source": "paperless", "id": 12345 }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source", content = "id", rename_all = "lowercase")]
pub enum InvoiceRef {
    Receipt(String),    // UUID string
    Paperless(i64),     // Paperless document ID
}

/// Source-agnostic view of an invoice.
/// All matching, sorting, and display code consumes this — never the concrete types.
pub trait Invoice {
    fn datetime(&self) -> Option<NaiveDateTime>;
    fn liters(&self) -> Option<f64>;
    fn total_price_eur(&self) -> Option<f64>;
    fn display_name(&self) -> &str;
    fn invoice_ref(&self) -> InvoiceRef;
}
```

### Adapter implementations

[models.rs](../../src-tauri/core/src/models.rs) — alongside `Receipt`:

```rust
impl Invoice for Receipt {
    fn datetime(&self) -> Option<NaiveDateTime> { self.receipt_datetime }
    fn liters(&self) -> Option<f64> { self.liters }
    fn total_price_eur(&self) -> Option<f64> { self.total_price_eur }
    fn display_name(&self) -> &str { &self.file_name }
    fn invoice_ref(&self) -> InvoiceRef { InvoiceRef::Receipt(self.id.clone()) }
}
```

[paperless.rs](../../src-tauri/core/src/paperless.rs) — alongside `PaperlessDoc`:

```rust
impl Invoice for PaperlessDoc {
    fn datetime(&self) -> Option<NaiveDateTime> { self.receipt_datetime }
    fn liters(&self) -> Option<f64> { self.litres }                  // bridge UK→US
    fn total_price_eur(&self) -> Option<f64> { self.total_amount }   // bridge name
    fn display_name(&self) -> &str { &self.title }
    fn invoice_ref(&self) -> InvoiceRef { InvoiceRef::Paperless(self.id) }
}
```

Source struct field names (`litres`, `total_amount`) are **not** renamed — the trait impl is the single naming bridge.

### Boundary functions

In [invoice.rs](../../src-tauri/core/src/invoice.rs):

```rust
pub fn load_invoice(db: &Database, r: &InvoiceRef) -> Result<Box<dyn Invoice>, String> {
    match r {
        InvoiceRef::Receipt(id) => {
            let receipt = db.get_receipt_by_id(id).map_err(|e| e.to_string())?
                .ok_or("Receipt not found")?;
            Ok(Box::new(receipt))
        }
        InvoiceRef::Paperless(id) => {
            let doc = db.get_paperless_doc_by_id(*id).map_err(|e| e.to_string())?
                .ok_or("Paperless doc not found")?;
            Ok(Box::new(doc))
        }
    }
}

pub fn save_invoice_assignment(
    db: &Database, r: &InvoiceRef, trip_id: &str,
    assignment_type: AssignmentType, mismatch_override: bool,
) -> Result<(), String> {
    match r {
        InvoiceRef::Receipt(id)   => db.assign_receipt(id, trip_id, assignment_type, mismatch_override),
        InvoiceRef::Paperless(id) => db.assign_paperless(*id, trip_id, assignment_type, mismatch_override),
    }
}
```

### Refactored compat check

In [invoice.rs](../../src-tauri/core/src/invoice.rs):

```rust
pub fn check_invoice_trip_compatibility(
    invoice: &dyn Invoice,
    trip: &Trip,
) -> CompatibilityResult {
    let is_fuel = invoice.liters().is_some();
    // body identical to existing fn — every `receipt.X` becomes `invoice.X()`
    // no logic changes, just access pattern; existing 12 tests catch regressions
}
```

The current `check_receipt_trip_compatibility` lives in [receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs) at line 480. It moves to the new module, gets renamed, and trades field access for trait method calls.

Trait object-safety: every method takes `&self` only, returns concrete types, no generics, no `Self`. `Box<dyn Invoice>` works.

## TypeScript Interface + Adapters

New file [src/lib/invoice.ts](../../src/lib/invoice.ts):

```ts
import type { Receipt, PaperlessInvoiceRow } from '$lib/types';

export type InvoiceRef =
    | { source: 'receipt'; id: string }
    | { source: 'paperless'; id: number };

export interface Invoice {
    getDateTime(): string | null;
    getLiters(): number | null;
    getPrice(): number | null;
    getDisplayName(): string;
    getRef(): InvoiceRef;
}

class ReceiptInvoice implements Invoice {
    constructor(private r: Receipt) {}
    getDateTime() { return this.r.receiptDatetime; }
    getLiters() { return this.r.liters; }
    getPrice() { return this.r.totalPriceEur; }
    getDisplayName() { return this.r.fileName; }
    getRef(): InvoiceRef { return { source: 'receipt', id: this.r.id }; }
}

class PaperlessInvoice implements Invoice {
    constructor(private p: PaperlessInvoiceRow) {}
    getDateTime() { return this.p.receiptDatetime; }
    getLiters() { return this.p.liters; }
    getPrice() { return this.p.totalPriceEur; }
    getDisplayName() { return this.p.title; }
    getRef(): InvoiceRef { return { source: 'paperless', id: this.p.paperlessDocumentId }; }
}

/** The ONE type guard. Source-checking elsewhere is a smell. */
export function adaptInvoice(source: Receipt | PaperlessInvoiceRow): Invoice {
    return 'paperlessDocumentId' in source
        ? new PaperlessInvoice(source)
        : new ReceiptInvoice(source);
}
```

## Backend Endpoints

Two unified Tauri commands replace four:

| Today | After |
|-------|-------|
| `get_trips_for_receipt_assignment` | `get_trips_for_invoice_assignment(invoice_ref, vehicle_id, year)` |
| `get_trips_for_year` (used by Paperless picker) | (same as above for Paperless) |
| `assign_receipt_to_trip` | `assign_invoice_to_trip(invoice_ref, trip_id, vehicle_id, assignment_type, mismatch_override)` |
| `assign_paperless_doc_to_trip` | (same as above for Paperless) |
| `unassign_receipt` | `unassign_invoice(invoice_ref)` |
| `unassign_paperless_doc` | (same as above for Paperless) |

Old commands deleted in the same PR — Tauri commands are internal IPC, not a public API.

## Frontend Changes

| File | Change |
|------|--------|
| [src/lib/invoice.ts](../../src/lib/invoice.ts) | **NEW** — Invoice interface, adapters, factory |
| [src/lib/types.ts](../../src/lib/types.ts) | Add `InvoiceRef` type |
| [src/lib/api.ts](../../src/lib/api.ts) | Replace 6 functions with 3 unified ones |
| [src/lib/components/TripSelectorModal.svelte](../../src/lib/components/TripSelectorModal.svelte) | Props `receipt` → `invoice`; field access via methods; one API rename |
| [src/routes/doklady/+page.svelte](../../src/routes/doklady/+page.svelte) | Delete inline Paperless modal (~46 lines); merge handlers; both buttons set `invoiceToAssign` |

### UX implications

- Paperless docs gain Fuel/Other step (already correct — same business artifact)
- Paperless docs gain mismatch override flow
- Local receipts retain their existing behavior
- Single Slovak i18n key set under `tripSelector.*` (inline-modal-only keys removed)

## Migration Sequence (single PR, ordered commits)

1. Backend trait + types (`Invoice`, `InvoiceRef`, two `impl` blocks). `cargo test` passes.
2. Refactor compat check: `check_receipt_trip_compatibility` → `check_invoice_trip_compatibility`. Update 12 tests mechanically.
3. Add boundary functions: `load_invoice`, `save_invoice_assignment`, `unassign_invoice`. Tests for both arms.
4. Add unified Tauri commands. 12 paperless-side compat tests + 6 boundary tests.
5. Delete old backend commands (compile errors in Svelte calls expected, fixed in step 6).
6. Frontend types & adapter ([invoice.ts](../../src/lib/invoice.ts), `InvoiceRef` in [types.ts](../../src/lib/types.ts)).
7. Refactor [TripSelectorModal.svelte](../../src/lib/components/TripSelectorModal.svelte) — props rename, method calls, API rename.
8. Rewrite [+page.svelte](../../src/routes/doklady/+page.svelte) — delete inline modal, merge handlers.
9. Rewrite [tests/integration/specs/tier2/paperless-integration.spec.ts](../../tests/integration/specs/tier2/paperless-integration.spec.ts) for unified Paperless flow.
10. Verify: `npm run test:all`.

## Risks

| Risk | Mitigation |
|------|------------|
| Refactored compat check has subtle regressions | 12 existing receipt tests catch behavior changes; run before & after |
| Object-safety violation if trait grows | Keep trait minimal; add lint test that constructs `Box<dyn Invoice>` |
| `InvoiceRef` serde shape mismatch (Rust ↔ TS) | Round-trip test serializing both variants against fixed JSON |
| Paperless toggle-off mode breaks | When disabled, paperless rows aren't rendered → assign button never shown → `InvoiceRef::Paperless` never sent. Local-only path unchanged |

## Out of Scope

- DB schema changes (the [receipts](../../src-tauri/core/migrations/) and [paperless_documents](../../src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/) tables stay independent)
- Unifying the underlying storage structures (`Receipt` and `PaperlessDoc` remain distinct in DB)
- Paperless sync logic (custom field parsing, fetch loop) — only the assignment surface changes
- Tauri command security/permissions
