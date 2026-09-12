# Feature: Paperless-ngx Integration

> Paperless-ngx is the only invoice source. When it is configured, the Doklady page reads invoices directly from a [Paperless-ngx](https://docs.paperless-ngx.com/) instance.

The local OCR scanning pipeline and the `receipts` table were removed in
[Task 84](../../_tasks/84-paperless-only-invoices/). Without Paperless configured the app
stays fully usable: the Doklady page shows a setup empty state and the logbook,
calculations, and export are unaffected.

## User Flow

1. **Configure Paperless in Settings.** Open *Nastavenia -> Paperless-ngx*, enter the instance URL (e.g. `https://documents.example.com`) and a Paperless API token. Auto-saves after 800ms; status indicator transitions through `IDLE -> TESTING -> CONNECTED` (or `DISCONNECTED` with a tooltip on hover).
2. **Open Doklady.** If Paperless is not configured, the page shows a setup empty state with a link to Settings. If it is configured, the page live-fetches documents tagged with `fuel` or `car`. A skeleton loader appears during the round-trip.
3. **Read-only display per row.** Each Paperless row shows: title, date (from the `receipt_datetime` custom field; falls back to `?` if missing), assignment chip (Fuel/Other), total amount, liters (only for fuel rows), and an assigned-trip indicator if linked.
4. **Two action buttons:**
   - **Otvoriť v Paperless** -- opens `{paperless_url}/documents/{doc_id}/` in the user's default browser (their browser stays logged in via cookies).
   - **Priradiť k jazde** -- opens the [unified TripSelectorModal](./unified-invoice-picker.md): proximity sort, mismatch warnings, and Fuel/Other selection. On confirm, an UPSERT row goes into [paperless_trip_links](../../src-tauri/core/migrations/2026-07-15-100000_multi_invoice/up.sql) (with type/amount/title/datetime snapshots and the override flag), and if the trip's fuel/other-costs side was empty it auto-populates from the doc's inline data. A trip can carry one Fuel doc plus any number of Other docs -- see [multi-invoice.md](./multi-invoice.md).
5. **Edit / Reprocess / Remove are hidden** -- Paperless is the source of truth for these documents; the app does not modify them. These are listed as deferred capability gaps in [_TECH_DEBT/09](../../_tasks/_TECH_DEBT/09-paperless-ocr-capability-gaps.md).
6. **Refresh from Paperless** button in the toolbar forces a fresh fetch (no client cache of documents themselves).

## Tag to Assignment Mapping

| Paperless tag | App `AssignmentType` |
|---|---|
| `fuel` | Fuel |
| `car` | Other |
| both | Fuel (priority) |
| neither | Logged warning + Other (server-side filter should make this unreachable) |

## Custom Fields (User-Created in Paperless)

| Custom field name | Type | Required for |
|---|---|---|
| `total_amount` | float | All invoices |
| `litres` (British spelling) | float | Fuel invoices only |
| `receipt_datetime` | string (ISO-8601, no TZ) | All invoices (optional per-doc; missing -> `?` in display) |

If any required field is missing in Paperless, the sync surfaces a structured error (`Vytvor custom field "<name>" v Paperless`) -- see [PaperlessError::CustomFieldNotFound](../../src-tauri/core/src/paperless.rs).

## Technical Implementation

### Backend (Rust)

| Module | Purpose |
|---|---|
| [paperless.rs](../../src-tauri/core/src/paperless.rs) | HTTP client (`PaperlessClient`), structured errors (`PaperlessError`), tag/field ID resolution, document fetch + parse with pagination |
| [commands_internal/paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs) | `get_paperless_invoices_internal` (composes client + DB join), `list_paperless_custom_fields_internal`, `count_unlinked_paperless_fuel_invoices_internal` (nav badge) |
| [commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs) | Settings I/O (`get_paperless_settings_internal`, `save_paperless_settings_internal`), connection test (`test_paperless_connection_internal`) |
| [commands_internal/invoices.rs](../../src-tauri/core/src/commands_internal/invoices.rs) | Trip-link persistence: `assign_paperless_invoice_internal`, `unassign_paperless_invoice_internal`, `revert_paperless_override_internal` |
| [db.rs](../../src-tauri/core/src/db.rs) | UPSERT/CRUD for `paperless_trip_links`: `upsert_paperless_link` (keyed on `paperless_document_id`), `delete_paperless_link_for_doc`, `get_paperless_link`, `get_paperless_links_for_trip`, `list_paperless_links_for_docs`, `get_all_paperless_links` |

### Data Flow

```
User opens Doklady page
    |
    v
Frontend probes whether Paperless is configured
    |
    v
configured? -- no --> setup empty state with a link to Settings
    |
   yes
    |
    v
Frontend calls get_paperless_invoices(vehicle_id, year)
    |
    v
Backend: PaperlessClient.resolve_tag_id("fuel" / "car")     <- cached after first session call
        + PaperlessClient.resolve_field_map()               <- cached after first session call
        + PaperlessClient.fetch_invoice_documents(...)      <- live, paginated
    |
    v
Filter by year (receipt_datetime if present, else `created`)
    |
    v
LEFT JOIN with paperless_trip_links (via list_paperless_links_for_docs)
    |
    v
Map each PaperlessDoc -> PaperlessInvoiceRow (incl. assignment_type and trip_id)
    |
    v
Frontend renders rows; user clicks Assign -> assign_paperless_invoice -> UPSERT
(see [unified-invoice-picker.md](./unified-invoice-picker.md) for the full picker flow)
```

### Schema Addition

One table, `paperless_trip_links` (introduced by [2026-05-03-100000_add_paperless_trip_links](../../src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/up.sql), rebuilt for multi-invoice support by [2026-07-15-100000_multi_invoice](../../src-tauri/core/migrations/2026-07-15-100000_multi_invoice/up.sql), extended by [2026-09-11-120000_paperless_only_invoice_columns](../../src-tauri/core/migrations/2026-09-11-120000_paperless_only_invoice_columns/up.sql)):

| Column | Type | Notes |
|---|---|---|
| `paperless_document_id` | INTEGER PRIMARY KEY | One trip per doc; a trip can carry many docs |
| `trip_id` | TEXT NOT NULL | FK to `trips(id)` ON DELETE CASCADE, indexed |
| `assignment_type` | TEXT NOT NULL | `'Fuel'` / `'Other'`; partial unique index enforces one Fuel link per trip |
| `amount_eur`, `title` | REAL / TEXT | Snapshots taken at assign time so the grid's sum check works offline |
| `applied_amount_cents` | INTEGER | Exact cents added to the trip at assign; NULL = link-only |
| `receipt_datetime` | TEXT | Assign-time datetime snapshot for the grid datetime warning |
| `mismatch_override` | INTEGER NOT NULL DEFAULT 0 | User-confirmed mismatch flag, revertible |
| `created_at`, `updated_at` | TEXT | ISO-8601 timestamps |

The original `trip_id PRIMARY KEY` shape ([ADR-019](../../DECISIONS.md)) mirrored the
one-invoice-per-trip rule of the former `receipts` table; that constraint was dropped when
trips gained 1 Fuel + N Other invoice support -- see [multi-invoice.md](./multi-invoice.md)
and BIZ-023 in [DECISIONS.md](../../DECISIONS.md).

## Design Decisions

- **Why a separate link table instead of storing documents?** -- Paperless owns its documents and their OCR. kniha-jazd stores only the trip-link row, so the database stays small and its backups stay textual. The removed local receipts have no import path into Paperless: the upgrade discards every receipt row that was not already assigned to a trip. Upload the documents to Paperless yourself, or export the table before you upgrade -- see the upgrade rule in [server-mode.md](./server-mode.md).
- **Why `Authorization: Token <PAT>` instead of `Bearer`?** -- Paperless-ngx uses Django REST Framework token auth, not OAuth2. See [BIZ-015](../../DECISIONS.md).
- **Why hardcode the custom-field names (`total_amount`, `litres`, `receipt_datetime`)?** -- They're load-bearing for parsing the document JSON. Surfacing them as user-configurable Settings would multiply test combinations without solving any real problem (the user controls naming on the Paperless side). Empty-name settings still allow a per-field override.
- **Why not cache Paperless documents in SQLite?** -- Document state lives in Paperless. The kniha-jazd app caches only the trip-link row. A sync of >100 docs that exceeds the 5s timeout per page is a known design ceiling for v1; bumping the timeout or implementing per-page caching is deferred.

## Out of Scope (deferred)

- Best-effort fuzzy matching of pre-existing local receipts to Paperless documents.
- Bulk multi-select assign / unassign UI.
- Storing Paperless documents themselves in SQLite (only the trip-link row is persisted).
- Paperless tags beyond `fuel` and `car`.
- Encrypting the PAT in the OS keyring (matches HA's plaintext-JSON convention; revisit globally with HA).
- Multi-vehicle scoping -- see [BIZ-016](../../DECISIONS.md).
- The six Gemini-path capabilities lost in Task 84 -- see [_TECH_DEBT/09-paperless-ocr-capability-gaps.md](../../_tasks/_TECH_DEBT/09-paperless-ocr-capability-gaps.md).

## Related

- [Task 60](../../_tasks/_done/60-paperless-integration/) -- original planning docs.
- [Task 84](../../_tasks/84-paperless-only-invoices/) -- removed the local receipt source; Paperless is the only invoice source.
- [unified-invoice-picker.md](./unified-invoice-picker.md) -- the trip-assignment flow for Paperless docs.
- [multi-invoice.md](./multi-invoice.md) -- 1 Fuel + N Other invoices per trip (link-table rebuild, amount snapshots, sum-on-assign).
- [ADR-008 in DECISIONS.md](../../DECISIONS.md) -- frontend-display-only constraint.
- [ADR-019, BIZ-015, BIZ-016 in DECISIONS.md](../../DECISIONS.md) -- schema, auth header, and v1-scope decisions.
- [ADR-020 in DECISIONS.md](../../DECISIONS.md) -- inline-data boundary contract.
