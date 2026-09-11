# Feature: Unified Invoice Picker

> One [TripSelectorModal](../../src/lib/components/TripSelectorModal.svelte) handles trip assignment for Paperless-ngx documents -- proximity sort, mismatch detection, Fuel/Other selection, and the override flow.

Paperless-ngx is the only invoice source. The local OCR'd receipt source and the
source-agnostic `Invoice` abstraction were removed in
[Task 84](../../_tasks/84-paperless-only-invoices/). The picker now takes a concrete
`PaperlessInvoiceRow`; there is no `adaptInvoice` factory and no `ReceiptInvoice` adapter.

## User Flow

The user clicks **Priradiť k jazde** on a Paperless invoice card; the modal opens with:

1. **Trip list sorted by date proximity** to the invoice's `receipt_datetime`. Trips on the same day are visually highlighted.
2. **Match indicator per trip** -- `✓ matches` (exact compat), `~ matches_date` (same day, time outside trip range), or `⚠ differs` (data conflict, hover for details). Since multi-invoice support ([multi-invoice.md](./multi-invoice.md)): for an **Other** invoice, a trip that already carries at least one Other invoice skips the amount comparison entirely (`✓ matches` -- the new amount will be summed on assign, there is nothing to match against); with zero Other invoices attached, the amount comparison is **cent-exact** (replacing the earlier ±0.01 epsilon), so the picker verdict always agrees with the assign-time double-count guard. For a **Fuel** invoice, trips that already have a Fuel invoice are greyed out (`can_attach = false`, reason "Jazda už má doklad o tankovaní").
3. **Click a trip -> step 2: Fuel/Other selection.** Default is pre-picked from the invoice's nature (fuel docs default to Fuel; non-fuel default to Other).
4. **Confirm.** If the trip already had data (fuel_liters / other_costs_eur) that conflicts with the invoice, a warning + override prompt is shown; otherwise a single confirm button. On confirm, an empty trip auto-populates fuel_liters / fuel_cost_eur (or other_costs_eur / other_costs_note) from the invoice; an Other invoice assigned to a trip with existing other costs adds its amount to the total (sum-on-assign, see [multi-invoice.md](./multi-invoice.md)). The override flag is persisted on the link (`mismatch_override`), so the "Potvrdené" marker survives a reload and can be reverted.

## Architecture

```
                     +--------------------------+
                     |    TripSelectorModal     |  <- ONE component
                     | (consumes a Paperless    |
                     |  InvoiceRow + inline     |
                     |  InvoiceData)            |
                     +------------+-------------+
                                  |
                   get_trips_for_paperless_assignment(docId, doc, ...)
                                  |
                                  v
                   +------------------------------+
                   | commands_internal/invoices.rs |
                   +--------------+---------------+
                                  v
             check_paperless_trip_compatibility(&PaperlessDoc, &Trip)
                   (single Rust source of truth)
```

There is no source dispatch left. The frontend sends the Paperless document id plus an
inline `InvoiceData` payload, and the backend runs the one compatibility check.

## Backend (Rust)

| Module | Purpose |
|---|---|
| [invoice.rs](../../src-tauri/core/src/invoice.rs) | `check_paperless_trip_compatibility`, `InvoiceData` RPC payload, and the range helpers |
| [paperless.rs](../../src-tauri/core/src/paperless.rs) | Paperless-ngx client and document model with the UK->US naming bridge (`litres` -> `liters`, `total_amount` -> `total_price_eur`) |
| [commands_internal/invoices.rs](../../src-tauri/core/src/commands_internal/invoices.rs) | `get_trips_for_paperless_assignment_internal`, `assign_paperless_invoice_internal`, `unassign_paperless_invoice_internal`, `revert_paperless_override_internal` |
| [commands_internal/paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs) | `count_unlinked_paperless_fuel_invoices_internal` (the nav badge) |
| [server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs) | Registers the assignment commands (they call the Paperless API) |

## Frontend (TS / Svelte)

| File | Purpose |
|---|---|
| [src/lib/types.ts](../../src/lib/types.ts) | `PaperlessInvoiceRow` and `InvoiceData` RPC payload type |
| [src/lib/api.ts](../../src/lib/api.ts) | API fns: `getTripsForPaperlessAssignment`, `assignPaperlessInvoice`, `unassignPaperlessInvoice` |
| [src/lib/components/TripSelectorModal.svelte](../../src/lib/components/TripSelectorModal.svelte) | Modal -- consumes a `PaperlessInvoiceRow` |
| [src/routes/doklady/+page.svelte](../../src/routes/doklady/+page.svelte) | Paperless cards feed the modal directly |

## Data Flow

```
User clicks "Priradiť k jazde" on a Paperless card
    |
    v
+page.svelte: passes the PaperlessInvoiceRow + inline InvoiceData
    |
    v
TripSelectorModal mounts
    |
    v
loadTrips() calls getTripsForPaperlessAssignment(docId, doc, ...)
    |
    v
Backend (invoices.rs): annotate trips with compat status
    |
    v
Trips returned with attachmentStatus + mismatchReason per trip
    |
    v
Frontend sorts by date proximity, highlights nearby trips
    |
    v
User picks a trip -> Fuel/Other step -> Confirm (override prompt if mismatch)
    |
    v
assignPaperlessInvoice(docId, doc, tripId, ..., mismatch_override)
    |
    v
Backend (invoices.rs): populate the trip from the inline data (if empty),
sum the amount for an Other invoice, and upsert the paperless_trip_links row
with the amount/title/datetime snapshots and the override flag
```

## Key Decision: Inline `InvoiceData` at the IPC Boundary

The original [02-design.md](../../_tasks/_done/64-unified-invoice-picker/02-design.md)
proposed `load_invoice(db, &InvoiceRef) -> Box<dyn Invoice>` to centralize loading. This
required `db.get_paperless_doc_by_id(*id)` -- but
[paperless_trip_links](../../src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/up.sql)
caches only `(trip_id, doc_id)`, no doc data. Implementing the design as written would
mean either a new `paperless_documents_cache` table (significant scope creep) or an
async Paperless API call per modal-open (network round-trip in the UI hot path).

**Decision:** the frontend, which already has the full Paperless row from
`get_paperless_invoices`, sends an inline `InvoiceData` payload alongside the document
id. The backend uses it directly for the compatibility check and the assign. The single
compat check and the source-agnostic frontend goals are preserved. There is only one
source now, so the payload is always Paperless data. See [ADR-020](../../DECISIONS.md).

## Related

- [Task 64](../../_tasks/_done/64-unified-invoice-picker/) -- original planning docs.
- [Task 84](../../_tasks/84-paperless-only-invoices/) -- removed the local receipt source
  and collapsed the abstraction to Paperless-only.
- [paperless-integration.md](./paperless-integration.md) -- the only invoice source.
- [multi-invoice.md](./multi-invoice.md) -- Task 66: 1 Fuel + N Other invoices per trip; redefined the compatibility check and `can_attach` described above.
- [ADR-008](../../DECISIONS.md) -- frontend-display-only constraint.
- [ADR-020](../../DECISIONS.md) -- inline-data deviation from the original design.
