**Date:** 2026-09-11
**Subject:** Remove the local invoice source; Paperless-ngx is the only invoice source
**Status:** Planning

# 84 -- Paperless-Only Invoices: Design

## Goal

One invoice source: Paperless-ngx. Delete the local receipt and Gemini OCR
subsystem. Keep the app fully usable without invoices when Paperless is not
configured.

## Target Architecture

```
┌────────────────────────────────────────────────────┐
│              SvelteKit Frontend                     │
│  Doklady page: Paperless rows only                  │
│  TripSelectorModal consumes a Paperless row         │
└──────────────────────┬─────────────────────────────┘
                       │ POST /api/rpc
┌──────────────────────▼─────────────────────────────┐
│  kniha-jazd-web (Axum)                              │
├────────────────────────────────────────────────────┤
│  kniha-jazd-core                                    │
│  paperless.rs    commands_internal/paperless_cmd.rs │
│  commands_internal/invoices.rs (paperless only)     │
│  db.rs: paperless_trip_links only                   │
└──────────────────────┬─────────────────────────────┘
                       │
                 SQLite (paperless_trip_links)
                 + live Paperless-ngx API
```

The source-agnostic layer is deleted. The code no longer asks "which source?".

## What Is Deleted, What Stays

### Backend, deleted

| Item | Path |
|---|---|
| Folder scan + Gemini application | `src-tauri/core/src/receipts.rs` |
| Gemini OCR client | `src-tauri/core/src/gemini.rs` |
| Receipt commands | `src-tauri/core/src/commands_internal/receipts_cmd.rs` |
| Receipt DB functions | `src-tauri/core/src/db.rs` (the receipt block, `create_receipt` to `get_receipts_for_vehicle`) |
| Receipt models | `src-tauri/core/src/models.rs` (`Receipt`, `ReceiptRow`, `ReceiptStatus`, `Currency`, ...) |
| Receipt schema | `src-tauri/core/src/schema.rs` (macro, joinables, `allow_tables` entry) |
| Receipt image route | `src-tauri/core/src/server/mod.rs` (`GET /api/receipts/{id}/image`) |
| Gemini reveal field | `src-tauri/core/src/commands_internal/reveal.rs` (`SecretField::GeminiApiKey`) |
| Settings fields | `src-tauri/core/src/settings.rs` (`gemini_api_key`, `receipts_folder_path`, `GEMINI_API_KEY`) |
| `Invoice` trait + `InvoiceRef` | `src-tauri/core/src/invoice.rs` |

### Backend, simplified

| Item | Change |
|---|---|
| `invoice.rs` | Becomes a Paperless compat module. Keeps `check_invoice_trip_compatibility` (renamed to `check_paperless_trip_compatibility`) and the range helpers. Deletes the trait, enum, and `PaperlessInvoiceView`. |
| `commands_internal/invoices.rs` | Keeps the assign/unassign/apply-other logic for Paperless only. Moves `TripForAssignment` here from `receipts_cmd.rs`. |
| `db.rs` | `get_trip_invoice_coverage` keeps only the `paperless_trip_links` half. Adds `get_all_paperless_links`. |
| `invoice_tests.rs` | Rewritten to drive the compat check with a Paperless test double, not a `Receipt` fixture. |

### Backend, kept unchanged

Paperless client and commands, `paperless_trip_links` CRUD, assign/unassign
rules, sum-on-assign, one-fuel-per-trip, coverage, missing-invoice warnings,
other-sum mismatch, read-only gating.

### Frontend, deleted

`ReceiptEditModal.svelte`, `ReceiptIndicator.svelte`, `stores/receipts.ts`, the
`ReceiptInvoice`/`PaperlessInvoice`/`adaptInvoice` layer in `src/lib/invoice.ts`,
receipt types/api wrappers/constants, the local branch and handlers in
`src/routes/doklady/+page.svelte`, and the Settings "Skenovanie dokladov" section.

### Frontend, changed

`TripSelectorModal.svelte` takes a concrete `PaperlessInvoiceRow` instead of the
`Invoice` interface. `+layout.svelte` uses the new invoice indicator.

## Capability Gap: What Is Lost With Gemini OCR

Paperless already provides: its own OCR, document storage, tags (`fuel`/`car`),
`total_amount`/`litres`/`receipt_datetime` custom fields, open-in-Paperless, live
fetch, assign/unassign, amount/title snapshots, sum-on-assign, one-fuel-per-trip,
coverage, and missing-invoice warnings.

| # | Capability lost | Paperless-side path (not built now) | Decision |
|---|---|---|---|
| 1 | Per-field OCR confidence + `NeedsReview` queue | Read Paperless's own confidence or add a custom field | Accept loss; tech-debt |
| 2 | Foreign-currency capture + manual EUR conversion | Add `original_amount`/`original_currency` custom fields | Accept loss; tech-debt |
| 3 | In-app edit of extracted values | Write to the Paperless API | Accept loss; tech-debt |
| 4 | In-app reprocess (re-OCR) | Call the Paperless redo-OCR endpoint | Accept loss; tech-debt |
| 5 | Station name/address, raw OCR text, status lifecycle | `correspondent` + custom fields | Accept loss; tech-debt |
| 6 | Full `verify_receipts` detail | Compute from coverage + link snapshots | Accept loss; tech-debt |
| 7 | Nav badge (unmatched invoices) | -- | **Ported** (unlinked Paperless fuel docs) |
| 8 | Trip-grid datetime warnings | Snapshot `receipt_datetime` on the link | **Ported** |
| 9 | Mismatch-override persistence | `mismatch_override` column on the link | **Ported** |

Items 1 to 6 are recorded in `_tasks/_TECH_DEBT/` during implementation. Items 7
to 9 are built in this task.

## Schema Changes

One new migration, `2026-09-11-120000_paperless_only_invoices` (name and timestamp
fixed at implementation).

### Add two columns to `paperless_trip_links`

```sql
ALTER TABLE paperless_trip_links ADD COLUMN receipt_datetime TEXT DEFAULT NULL;
ALTER TABLE paperless_trip_links ADD COLUMN mismatch_override INTEGER NOT NULL DEFAULT 0;
```

`ALTER TABLE ADD COLUMN` is enough in SQLite: both columns are nullable or have a
constant default. No table rebuild is needed. Existing rows get NULL and 0, so no
false warning appears on upgrade.

### Drop `receipts`

```sql
DROP TABLE receipts;
```

The indexes drop with the table. The table has foreign keys to `vehicles` and
`trips`, and no table references `receipts`, so the drop is safe with foreign keys
on. This migration runs after `2026-07-15-100000_multi_invoice`, which already
rebuilt both tables.

The whole migration is one folder, so it is atomic.

## Command Surface

### Removed RPC commands

Sync dispatcher (`src-tauri/core/src/server/dispatcher.rs`): `get_receipts`,
`get_receipts_for_vehicle`, `get_unassigned_receipts`, `update_receipt`,
`delete_receipt`, `revert_receipt_override`, `verify_receipts`,
`get_receipt_settings`, `set_gemini_api_key`, `set_receipts_folder_path`,
`scan_receipts`.

Async dispatcher (`src-tauri/core/src/server/dispatcher_async.rs`):
`sync_receipts`, `process_pending_receipts`, `reprocess_receipt`.

### Changed RPC commands

| Before | After |
|---|---|
| `get_trips_for_invoice_assignment(invoice_ref, data, ...)` | `get_trips_for_paperless_assignment(docId, data, ...)` |
| `assign_invoice_to_trip(invoice_ref, doc, ..., mismatch_override)` | `assign_paperless_invoice(docId, doc, ..., mismatch_override)` (override now persisted) |
| `unassign_invoice(invoice_ref)` | `unassign_paperless_invoice(docId)` |
| `get_invoice_source_mode` | Removed; the page probes Paperless configured state |
| `revert_receipt_override(receipt_id)` | `revert_paperless_override(docId)` |

### Added RPC commands

`count_unlinked_paperless_fuel_invoices(vehicleId, year)` -- returns an integer for
the nav badge.

## Nav Badge

`ReceiptIndicator.svelte` becomes `InvoiceIndicator.svelte`. It calls the new count
command for the active vehicle and selected year, and shows a badge when the count
is above zero. It links to `/doklady`.

The old indicator polled every 30 seconds against local SQLite. The new count hits
the Paperless API, so the poll drops to every 5 minutes and also refreshes on
vehicle and year change. This bounds the network cost.

## Datetime Warnings and Mismatch Override

At assign time in `commands_internal/invoices.rs`, the link now stores:

- `receipt_datetime` = `doc.receipt_datetime`
- `mismatch_override` = the parameter (today it is discarded at line 198)

`db.get_all_paperless_links` loads every link for the year. The two functions in
`commands_internal/statistics.rs` are rewritten to take links instead of receipts:

- `calculate_receipt_datetime_warnings` -> `calculate_invoice_datetime_warnings`
- `calculate_receipt_mismatch_overrides` -> `calculate_invoice_override_warnings`

The split by `AssignmentType` stays. The `TripGridData` fields
(`fuelDatetimeWarnings`, `otherDatetimeWarnings`, `fuelMismatchOverrides`,
`otherMismatchOverrides`) and the `TripRow.svelte` markup stay unchanged.

`revert_receipt_override` becomes `revert_paperless_override`, which sets
`mismatch_override = 0` on the link row.

Known limitation: the warning uses the assign-time snapshot. A later date edit in
Paperless does not update the grid. This matches the amount snapshot behavior.

## Doklady Page

```
onMount / vehicle or year change
    ↓
call get_paperless_settings (or a small configured probe)
    ↓
configured? ── no ──> setup empty state with a link to Settings
    │
   yes
    ↓
load Paperless rows for vehicle + year
    ↓
render rows, assign, unassign, open, revert override
```

No local fallback. No silent `Local` mode.

## Testing

### Removed

- Backend: `receipts_tests.rs`, `gemini_tests.rs`,
  `commands_internal/receipts_cmd_tests.rs`, and all receipt cases in `db_tests.rs`,
  `commands_tests.rs`, and `migration_tests.rs`.
- Integration: `tests/integration/specs/tier2/receipts.spec.ts` and
  `receipt-settings.spec.ts`; receipt fixtures, `seedReceipt`, and the receipt
  helpers in `utils/db.ts`; the Gemini mock files in `data/mocks` and
  `data/invoices`; `KNIHA_JAZD_MOCK_GEMINI_DIR` in the wdio config and CI.

### Added or changed

- Backend: `count_unlinked_paperless_fuel_invoices` count logic.
- Backend: rewritten `invoice_tests.rs` over a Paperless double, including the
  datetime-warning and override-warning functions.
- Integration: Doklady unconfigured empty state.
- Integration: assign a Paperless doc with a mismatch and confirm the override
  marker appears; then revert it.

### Kept

The Paperless integration spec and the shared missing-invoice and other-sum tests.

## Documentation

- Delete `docs/features/receipt-scanning.md`.
- Edit `multi-invoice.md`, `unified-invoice-picker.md`, `settings-architecture.md`,
  `server-mode.md`, `read-only-mode.md`, `ARCHITECTURE.md`, `README.md`,
  `README.en.md`.
- Add a superseding ADR to `DECISIONS.md`.
- Add a `CHANGELOG.md` Unreleased entry.
- Add tech-debt entries for gap items 1 to 6.
- Update `.claude/rules/integration-tests.md` (Gemini mock dir).

## Implementation Order

1. Move `TripForAssignment` into `invoices.rs`; add the Paperless link columns and
   the concrete types.
2. Simplify the invoice commands and the picker; persist the override; rewrite the
   warning functions.
3. Add the count command and the new indicator.
4. Remove the receipt RPCs, image route, settings, and reveal variant.
5. Remove the backend modules, DB functions, models, schema, and Gemini; fix the
   tests.
6. Rewrite the Doklady page Paperless-only; remove the frontend components, types,
   and i18n namespaces.
7. Add the drop migration.
8. Remove the test fixtures, the mock wiring, and update the wdio/CI config.
9. Update the docs, CHANGELOG, DECISIONS, and tech-debt entries.

The migration is last in the code order because it is destructive. The code must
compile against the new schema before the table disappears.

## Decision Log

| Decision | Why |
|---|---|
| Delete the abstraction, do not keep it | One implementation is not an extension point, it is unused indirection. |
| Drop the table, do not keep dead data | The user chose a clean end state. Backups cover the DB. The migration script is the pre-upgrade path. |
| Port the badge, datetime warnings, and override | They are cheap to port and users rely on them. |
| Accept the other six gaps | They need new Paperless custom fields or API writes, which is a larger feature. They are listed and deferred. |
| Keep the migration script | It is the only way to move old local receipts into Paperless, and only before the upgrade. |