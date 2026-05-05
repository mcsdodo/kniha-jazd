**Date:** 2026-05-04
**Subject:** Unified Invoice Picker (Receipts + Paperless)
**Status:** Planning (deferred — implement after [Task 63](../63-paperless-configurable-fields/))

# Task 64: Unified Invoice Picker

## Problem

Invoice → trip assignment is split into two parallel implementations:

- **Local receipts** use [TripSelectorModal.svelte](../../src/lib/components/TripSelectorModal.svelte) with smart matching: proximity sort, mismatch warnings, Fuel/Other step, override flow.
- **Paperless docs** use a custom inline modal in [src/routes/doklady/+page.svelte](../../src/routes/doklady/+page.svelte) (lines 1081–1126) that just lists every trip for the selected year — no sorting, no matching annotations, no mismatch handling.

This is parallel-implementation drift. From the business perspective both are *invoices*; the source (local OCR'd folder vs Paperless-ngx) is implementation detail. Users assigning a Paperless doc lose the smart matching that local-receipt users have.

## Goal

ONE picker component, ONE compatibility check, ONE assignment flow. Source-discrimination confined to two boundary functions — never leaks into UI, matching logic, or display code.

## User Story

> As a user, regardless of whether my fuel invoice came from a local OCR'd folder or from Paperless-ngx, I want the same smart trip-matching experience: candidate trips sorted by date proximity, exact-match highlighted, mismatch warnings if I pick a trip with different liters/price, and the ability to override.

## Approach (summary)

- Rust trait `Invoice` with object-safe methods (`datetime()`, `liters()`, `total_price_eur()`, `display_name()`, `invoice_ref()`)
- `Receipt` and `PaperlessDoc` both implement `Invoice` (UK→US naming bridged inside the trait impl)
- Generic `check_invoice_trip_compatibility(&dyn Invoice, &Trip)` replaces the receipt-only version
- TS interface `Invoice` mirrors the Rust trait; `ReceiptInvoice`/`PaperlessInvoice` adapters
- Tagged enum `InvoiceRef` at IPC boundary for source dispatch (one Rust match, one TS type guard)
- Two unified Tauri commands replace four

## Acceptance Criteria

- [ ] One [TripSelectorModal.svelte](../../src/lib/components/TripSelectorModal.svelte) component handles both invoice sources
- [ ] Inline Paperless modal in [+page.svelte](../../src/routes/doklady/+page.svelte) is removed
- [ ] Backend: `get_trips_for_invoice_assignment` + `assign_invoice_to_trip` + `unassign_invoice` replace the four old commands
- [ ] Source discrimination only in `load_invoice` / `save_invoice_assignment` (Rust) and `adaptInvoice` (TS)
- [ ] All 12 receipt-side compat tests pass with the renamed function
- [ ] 12 parallel paperless-side compat tests added
- [ ] Integration test for unified flow (assign Paperless doc to trip via shared modal)
- [ ] `npm run test:all` passes

## Design

See [02-design.md](./02-design.md) for full architecture.

## Related

- Builds on [Task 60](../_done/60-paperless-integration/) (Paperless Integration)
- Builds on [Task 62](../62-paperless-toggle/) (Paperless Toggle)
- Independent of (but easier after) [Task 63](../63-paperless-configurable-fields/) (Configurable Paperless Field Names)
