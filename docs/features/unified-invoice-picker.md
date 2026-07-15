# Feature: Unified Invoice Picker

> One [TripSelectorModal](../../src/lib/components/TripSelectorModal.svelte) handles trip assignment for **both** local OCR'd receipts AND Paperless-ngx documents — same proximity sort, same mismatch detection, same Fuel/Other selection, same override flow.

## User Flow

Identical for both invoice sources. The user clicks **Priradiť k jazde** on any invoice card; the modal opens with:

1. **Trip list sorted by date proximity** to the invoice's `receipt_datetime`. Trips on the same day are visually highlighted.
2. **Match indicator per trip** — `✓ matches` (exact compat), `~ matches_date` (same day, time outside trip range), or `⚠ differs` (data conflict, hover for details). Since multi-invoice support ([multi-invoice.md](./multi-invoice.md)): for an **Other** invoice, a trip that already carries at least one Other invoice skips the amount comparison entirely (`✓ matches` — the new amount will be summed on assign, there is nothing to match against); with zero Other invoices attached, the amount comparison is **cent-exact** (replacing the earlier ±0.01 epsilon), so the picker verdict always agrees with the assign-time double-count guard. For a **Fuel** invoice, trips that already have a Fuel invoice (either source) are greyed out (`can_attach = false`, reason „Jazda už má doklad o tankovaní").
3. **Click a trip → step 2: Fuel/Other selection.** Default is pre-picked from the invoice's nature (fuel docs default to Fuel; non-fuel default to Other).
4. **Confirm.** If the trip already had data (fuel_liters / other_costs_eur) that conflicts with the invoice, a warning + override prompt is shown; otherwise a single confirm button. On confirm, an empty trip auto-populates fuel_liters / fuel_cost_eur (or other_costs_eur / other_costs_note) from the invoice; an Other invoice assigned to a trip with existing other costs adds its amount to the total (sum-on-assign, see [multi-invoice.md](./multi-invoice.md)).

Before this feature: local receipts had this flow, but Paperless docs got a flat alphabetical trip list with no smart matching.

## Architecture

```
                     ┌──────────────────────────┐
                     │    TripSelectorModal     │  ← ONE component
                     │   (consumes Invoice)     │
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
                  invoice.getRef() / invoice.getData()
                              │
                  ┌───────────▼────────────┐
                  │  get_trips_for_invoice │  ← unified backend cmd
                  │       _assignment      │
                  └───────────┬────────────┘
                              ▼
            check_invoice_trip_compatibility(&dyn Invoice, &Trip)
                  (single Rust source of truth)
```

**Source dispatch is confined to two boundary functions** — never leaks into UI, matching logic, or display code:

| Layer | Boundary | Where |
|---|---|---|
| Rust IPC entry | `match InvoiceRef { Receipt(id) => …, Paperless(id) => … }` | [commands_internal/invoices.rs](../../src-tauri/core/src/commands_internal/invoices.rs) |
| TS adapter factory | `'paperlessDocumentId' in source ? PaperlessInvoice : ReceiptInvoice` | [src/lib/invoice.ts](../../src/lib/invoice.ts) |

Outside these two spots, code consumes `&dyn Invoice` (Rust) or `Invoice` (TS) and never inspects the source.

## Backend (Rust)

| Module | Purpose |
|---|---|
| [invoice.rs](../../src-tauri/core/src/invoice.rs) | `Invoice` trait, `InvoiceRef` tagged enum, `InvoiceData` IPC payload, `PaperlessInvoiceView` adapter, and the single `check_invoice_trip_compatibility` compat check |
| [models.rs](../../src-tauri/core/src/models.rs) | `impl Invoice for Receipt` (delegates to existing fields) |
| [paperless.rs](../../src-tauri/core/src/paperless.rs) | `impl Invoice for PaperlessDoc` with UK→US naming bridge (`litres` → `liters()`, `total_amount` → `total_price_eur()`) |
| [commands_internal/invoices.rs](../../src-tauri/core/src/commands_internal/invoices.rs) | `get_trips_for_invoice_assignment_internal`, `assign_invoice_to_trip_internal`, `unassign_invoice_internal` (the source-dispatch boundary) |
| [desktop/src/commands/invoices.rs](../../src-tauri/desktop/src/commands/invoices.rs) | `#[tauri::command]` wrappers exposing the three IPC endpoints |

## Frontend (TS / Svelte)

| File | Purpose |
|---|---|
| [src/lib/invoice.ts](../../src/lib/invoice.ts) | `Invoice` interface, `ReceiptInvoice` + `PaperlessInvoice` adapter classes, `adaptInvoice(source)` factory |
| [src/lib/types.ts](../../src/lib/types.ts) | `InvoiceRef` tagged-union type, `InvoiceData` IPC payload type |
| [src/lib/api.ts](../../src/lib/api.ts) | Three unified API fns: `getTripsForInvoiceAssignment`, `assignInvoiceToTrip`, `unassignInvoice` |
| [src/lib/components/TripSelectorModal.svelte](../../src/lib/components/TripSelectorModal.svelte) | Modal — props is now `invoice: Invoice` (was `receipt: Receipt`) |
| [src/routes/doklady/+page.svelte](../../src/routes/doklady/+page.svelte) | Both invoice cards (local + Paperless) call `adaptInvoice(source)` and feed the same modal |

## Data Flow (any invoice source)

```
User clicks "Priradiť k jazde" on an invoice card
    ↓
+page.svelte: invoiceToAssign = adaptInvoice(receipt | paperlessRow)
    ↓
TripSelectorModal mounts with invoice prop
    ↓
loadTrips() calls getTripsForInvoiceAssignment(invoice.getRef(), invoice.getData(), ...)
    ↓
Backend dispatch (invoices.rs):
    • Receipt(id)    → load receipt from DB  → annotate trips with compat status
    • Paperless(id)  → use inline InvoiceData → annotate trips with compat status
    ↓
Trips returned with attachmentStatus + mismatchReason per trip
    ↓
Frontend sorts by date proximity, highlights nearby trips
    ↓
User picks a trip → Fuel/Other step → Confirm
    ↓
assignInvoiceToTrip(invoice.getRef(), invoice.getData(), tripId, ...)
    ↓
Backend dispatch (invoices.rs):
    • Receipt(id)    → assign_receipt_to_trip_internal (existing flow, populates trip)
    • Paperless(id)  → populate trip from inline data (mirrors receipt flow), upsert paperless_trip_links row
```

## Key Decision: Inline `InvoiceData` at the IPC Boundary

The original [02-design.md](../../_tasks/_done/64-unified-invoice-picker/02-design.md) proposed `load_invoice(db, &InvoiceRef) -> Box<dyn Invoice>` to centralize loading. This required `db.get_paperless_doc_by_id(*id)` — but [paperless_trip_links](../../src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/up.sql) caches only `(trip_id, doc_id)`, no doc data. Implementing the design as written would mean either:

- A new `paperless_documents_cache` table (significant scope creep), or
- An async Paperless API call per modal-open (network round-trip in the UI hot path)

**Decision:** the frontend, which already has the full Paperless row from `get_paperless_invoices`, sends an inline `InvoiceData` payload alongside the `InvoiceRef`. Receipts ignore it (backend loads from DB by ID); Paperless docs use it directly via `PaperlessInvoiceView<'a>`. The trait, single compat check, and source-agnostic frontend goals are preserved. See [ADR-020](../../DECISIONS.md).

## Related

- [Task 64](../../_tasks/_done/64-unified-invoice-picker/) — original planning docs.
- [paperless-integration.md](./paperless-integration.md) — Paperless-ngx invoice source.
- [receipt-scanning.md](./receipt-scanning.md) — Local OCR'd receipt source.
- [multi-invoice.md](./multi-invoice.md) — Task 66: 1 Fuel + N Other invoices per trip; redefined the compatibility check and `can_attach` described above.
- [ADR-008](../../DECISIONS.md) — frontend-display-only constraint that drove the trait abstraction.
- [ADR-020](../../DECISIONS.md) — inline-data deviation from the original design.
