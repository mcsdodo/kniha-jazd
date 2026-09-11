# Tech Debt: Paperless-Only Invoice Capability Gaps

**Date:** 2026-09-11
**Priority:** Low
**Effort:** High (1-3d, one item at a time)
**Component:** `src-tauri/core/src/paperless.rs`, `src-tauri/core/src/commands_internal/invoices.rs`, `src/routes/doklady/+page.svelte`
**Status:** Open

## Problem

Task 84 removed the local receipt folder and the Gemini OCR subsystem. Paperless-ngx
is now the only invoice source. Paperless supplies its own OCR, document storage, the
`fuel`/`car` tags, the `total_amount`/`litres`/`receipt_datetime` custom fields,
open-in-Paperless, live fetch, assign/unassign, amount/title snapshots, sum-on-assign,
one-fuel-per-trip, coverage, and missing-invoice warnings.

Six capabilities of the deleted Gemini path have no Paperless-side implementation yet.
Task 84 recorded them as accepted losses. This item tracks them so the loss stays
visible and each gap has a named Paperless path if the user wants it back.

## Gap Items

Each item names the lost capability, the Paperless-side implementation path, and the
current behaviour.

### 1. Per-field OCR confidence and the `NeedsReview` queue

- **Lost:** Gemini returned a confidence level per field. Low confidence or an unknown
  value moved the receipt into a review queue before assignment.
- **Paperless-side path:** read the confidence Paperless stores for its own OCR, or add
  a custom field that carries the score the user wants to gate on.
- **Current:** no confidence is read or shown. Every document is assumed usable.
- **Component:** `src-tauri/core/src/paperless.rs`, `src/routes/doklady/+page.svelte`.

### 2. Foreign-currency capture and manual EUR conversion

- **Lost:** `original_amount` and `original_currency` stored the raw OCR values for
  CZK/HUF/PLN and flagged the receipt for manual conversion to EUR.
- **Paperless-side path:** add `original_amount` and `original_currency` custom fields
  to the document type, read them on fetch, and reuse the manual EUR entry flow.
- **Current:** only `total_amount` (EUR) is read. A foreign-currency document has no
  place to store its original value.

### 3. In-app edit of extracted values

- **Lost:** the receipt edit modal wrote corrected OCR values back to the local row.
- **Paperless-side path:** write the corrected custom fields through the Paperless API
  (`PATCH /api/documents/{id}/`), then re-fetch the document.
- **Current:** extracted values are read-only in kniha-jazd. The user edits in Paperless
  and the next fetch picks the change up.

### 4. In-app reprocess (re-OCR)

- **Lost:** a per-receipt reprocess action re-ran Gemini on the stored image.
- **Paperless-side path:** call the Paperless redo-OCR endpoint for the document and
  poll until the task completes.
- **Current:** reprocessing is a Paperless action, not a kniha-jazd action.

### 5. Station name/address, raw OCR text, and status lifecycle

- **Lost:** `station_name`, `station_address`, the raw OCR text, and the
  `Pending -> Parsed -> NeedsReview -> Assigned` status plus its year-mismatch warning.
- **Paperless-side path:** map `correspondent` and custom fields for the station data;
  read the document's own processing state for the lifecycle; keep the raw text in a
  custom field only if a feature needs it.
- **Current:** the station text is unavailable; the local status lifecycle is gone, and
  assignment state lives on the `paperless_trip_links` row.

### 6. Full `verify_receipts` detail

- **Lost:** `verify_receipts` returned per-receipt mismatch reasons (`date`, `liters`,
  `price`, `noFuelTripFound`, `dateMismatch`, `noOtherCostMatch`, and so on).
- **Paperless-side path:** compute the same verdicts from the coverage data plus the
  amount/title/datetime snapshots already stored on the link.
- **Current:** the grid warnings cover assignment health, but there is no per-document
  verification report with a mismatch reason list.

## Impact

- The user cannot see OCR quality, correct a bad extraction in the app, or re-run OCR
  from the app.
- Foreign-currency documents cannot store their original amount.
- Station metadata and a document-level verification report are unavailable.

None of these gaps blocks the core logbook or the legal calculations. They reduce the
richness of the Doklady page compared with the deleted local path.

## Root Cause

The deleted path owned the image, the OCR call, and the extracted data, so it could
offer edit, reprocess, confidence, and status for free. Paperless owns all four now;
kniha-jazd is a display and assignment client. Closing each gap means either writing
to the Paperless API or extending the Paperless document type with custom fields, which
is a larger feature than Task 84's scope.

## Recommended Solution

Treat each gap as an independent feature. Build them on demand, in this order:

1. In-app edit (item 3) and reprocess (item 4): both are Paperless API writes and need
   no schema change.
2. Foreign-currency fields (item 2): add the two Paperless custom fields and read them.
3. Verification report (item 6): pure computation over existing link snapshots.
4. Confidence queue (item 1) and station/raw-text fields (item 5): the largest, because
   they depend on what Paperless chooses to expose.

The paths in each item above are the starting point. Confirm the exact Paperless API
response shape against the live Paperless version before building.

## Alternative Options

1. **Restore the local receipt path.** Rejected by Task 84: it duplicates Paperless and
   the user chose a single source.
2. **Do nothing.** Accepted for now. The gaps are listed and deferred, not forgotten.

## Related

- [Task 84](../84-paperless-only-invoices/) -- removed the local receipt path and the
  Gemini OCR subsystem, and recorded these gaps.
- [02-design.md](../84-paperless-only-invoices/02-design.md) -- the Capability Gap
  table these items come from.
- [docs/features/paperless-integration.md](../../docs/features/paperless-integration.md)
  -- the surviving invoice source.

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-09-11 | Created item | Task 84 accepted the loss of the Gemini OCR capabilities and deferred their Paperless-side implementations |
