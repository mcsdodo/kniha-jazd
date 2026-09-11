**Date:** 2026-09-11
**Subject:** Remove the local invoice source; Paperless-ngx is the only invoice source
**Status:** Planning

# 84 -- Paperless-Only Invoices

## Goal

Remove the local receipt and Gemini OCR subsystem from the whole codebase. Make
Paperless-ngx the only invoice source. Keep the app fully usable without invoices
when Paperless is not configured.

This closes the decision left open by [ADR-030](../../DECISIONS.md) (line 369):
"Folder-scanned receipts survive as an unmaintained path ... Deleting the folder
path is a separate decision about a feature." This is that decision.

## User Story

As the operator, I want one invoice source instead of two, so that the codebase
holds no dead OCR path, the Settings page holds no unused Gemini fields, and the
Doklady page has one clear flow. If I have not configured Paperless, the Doklady
page tells me to configure it and the rest of the app still works.

## Requirements

1. Paperless-ngx is the only invoice source. The Doklady page never falls back to
   local receipts.
2. When Paperless is not configured, the Doklady page shows a setup empty state
   that links to Settings. Trips, consumption, and margin work as before.
3. The source-agnostic invoice abstraction is deleted: the Rust `Invoice` trait,
   the `InvoiceRef` enum, the inline `InvoiceData` indirection, and the TS
   `ReceiptInvoice`/`PaperlessInvoice`/`adaptInvoice` layer. The picker consumes a
   concrete Paperless row.
4. The `receipts` table is dropped by a new migration. Existing local receipt rows
   are gone after the upgrade.
5. `scripts/migrate_local_to_paperless.py` stays in the repo and is documented as a
   pre-upgrade step, because it can only read the table while the table exists.
6. The Gemini OCR path is removed: `gemini.rs`, `GEMINI_API_KEY`, the Settings
   "Skenovanie dokladov" section, and the mock-Gemini test wiring.
7. The nav badge is ported: it counts unlinked Paperless fuel documents for the
   active vehicle and selected year.
8. Trip-grid datetime warnings and mismatch-override persistence are ported to
   Paperless by snapshotting `receipt_datetime` and `mismatch_override` on
   `paperless_trip_links` at assign time.
9. Every capability that local receipts provided and Paperless does not is stated
   in `02-design.md` as an explicit loss, with a note on how it could be added on
   the Paperless side.

## Out of Scope

The following are accepted losses for now. Each is recorded as a tech-debt entry
during implementation:

- Per-field OCR confidence and the `NeedsReview` queue.
- Foreign-currency capture (`original_amount` + `original_currency`) and the manual
  EUR conversion step.
- In-app edit of extracted invoice values.
- In-app reprocess (re-OCR).
- Station name/address, raw OCR text, and the receipt status lifecycle.
- The full `verify_receipts` mismatch-reason report (the nav badge is ported).

## Technical Notes

- [ADR-008](../../DECISIONS.md) applies: all business logic stays in Rust; the
  frontend stays display-only.
- Historical migrations are never edited. The `receipts` drop is a new migration
  that runs after `2026-07-15-100000_multi_invoice`.
- `LocalSettings` ignores unknown JSON fields, so an old `local.settings.json` that
  still holds `gemini_api_key` loads without error. The fields are still removed
  from the struct.
- The datetime warning uses the assign-time snapshot. If the user edits the date in
  Paperless later, the grid does not notice. This matches the existing amount
  snapshot behavior.
- No calculation is duplicated in the frontend.

## Related

- [ADR-030](../../DECISIONS.md) -- the desktop app was deleted; Paperless became the
  supported intake channel.
- [docs/features/paperless-integration.md](../../docs/features/paperless-integration.md)
- [docs/features/receipt-scanning.md](../../docs/features/receipt-scanning.md) --
  deleted by this task.
- [02-design.md](./02-design.md) -- full design and capability-gap analysis.