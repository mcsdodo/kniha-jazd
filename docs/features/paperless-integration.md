# Feature: Paperless-ngx Integration

> Optional alternative invoice source: when configured, the Doklady page reads invoices directly from a [Paperless-ngx](https://docs.paperless-ngx.com/) instance instead of the local OCR scanning pipeline.

## User Flow

1. **Configure Paperless in Settings.** Open *Nastavenia → Paperless-ngx*, enter the instance URL (e.g. `https://documents.example.com`) and a Paperless API token. Auto-saves after 800ms; status indicator transitions through `IDLE → TESTING → CONNECTED` (or `DISCONNECTED` with a tooltip on hover).
2. **Open Doklady.** When Paperless is configured, the Doklady page lives-fetches documents tagged with `fuel` or `car` from Paperless instead of scanning the local folder. Skeleton loader appears during the round-trip.
3. **Read-only display per row.** Each Paperless row shows: title, date (from the `receipt_datetime` custom field; falls back to `?` if missing), assignment chip (Fuel/Other), total amount, liters (only for fuel rows), and an assigned-trip indicator if linked.
4. **Two action buttons in Paperless mode:**
   - **Otvoriť v Paperless** — opens `{paperless_url}/documents/{doc_id}/` in the user's default browser (their browser stays logged in via cookies).
   - **Priradiť k jazde** — opens the [unified TripSelectorModal](./unified-invoice-picker.md): same proximity sort, mismatch warnings, and Fuel/Other selection as local receipts. On confirm, an UPSERT row goes into [paperless_trip_links](../../src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/up.sql), and if the trip's fuel/other-costs side was empty it auto-populates from the doc's inline data.
5. **Edit / Reprocess / Remove are hidden** — Paperless is the source of truth for these documents; the desktop app does not modify them.
6. **Refresh from Paperless** button in the toolbar forces a fresh fetch (no client cache of documents themselves).
7. **Toggle off restores local view.** Clearing the Paperless URL in Settings reverts the Doklady page to the local-receipts grid; existing local receipts and their assignments are preserved untouched.

## Tag → Assignment Mapping

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
| `receipt_datetime` | string (ISO-8601, no TZ) | All invoices (optional per-doc; missing → `?` in display) |

If any required field is missing in Paperless, the sync surfaces a structured error (`Vytvor custom field "<name>" v Paperless`) — see [PaperlessError::CustomFieldNotFound](../../src-tauri/core/src/paperless.rs).

## Technical Implementation

### Mode-Switch Pattern

Frontend asks the backend "are we in Paperless mode?" and renders the appropriate branch. It never inspects raw settings — see [ADR-008](../../DECISIONS.md) (no calculation/conditional logic in frontend).

```
get_invoice_source_mode (Tauri command)
    ↓
LocalSettings { paperless_url, paperless_api_token }
    ↓
{ Local, Paperless }
    ↓
Frontend mounts the matching renderer
```

### Backend (Rust)

| Module | Purpose |
|---|---|
| [paperless.rs](../../src-tauri/core/src/paperless.rs) | HTTP client (`PaperlessClient`), structured errors (`PaperlessError`), tag/field ID resolution, document fetch + parse with pagination, `impl Invoice for PaperlessDoc` |
| [commands_internal/paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs) | `get_paperless_invoices_internal` (composes client + DB join), `list_paperless_custom_fields_internal`. Trip-link persistence is handled by the [unified invoice commands](./unified-invoice-picker.md), not by Paperless-specific functions. |
| [commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs) | Settings I/O (`get_paperless_settings_internal`, `save_paperless_settings_internal`), connection test (`test_paperless_connection_internal`), mode probe (`get_invoice_source_mode_internal`) |
| [db.rs](../../src-tauri/core/src/db.rs) | UPSERT/CRUD for `paperless_trip_links`: `upsert_paperless_link`, `delete_paperless_link_for_doc`, `get_paperless_link_for_doc`, `get_paperless_link_for_trip`, `list_paperless_links_for_docs` |

### Data Flow (Paperless mode)

```
User opens Doklady page
    ↓
Frontend calls get_invoice_source_mode → "Paperless"
    ↓
Frontend calls get_paperless_invoices(vehicle_id, year)
    ↓
Backend: PaperlessClient.resolve_tag_id("fuel" / "car")     ← cached after first session call
        + PaperlessClient.resolve_field_map()               ← cached after first session call
        + PaperlessClient.fetch_invoice_documents(...)      ← live, paginated
    ↓
Filter by year (receipt_datetime if present, else `created`)
    ↓
LEFT JOIN with paperless_trip_links (via list_paperless_links_for_docs)
    ↓
Map each PaperlessDoc → PaperlessInvoiceRow (incl. assignment_type and trip_id)
    ↓
Frontend renders rows; user clicks Assign → adaptInvoice(row) → assign_invoice_to_trip → UPSERT
(see [unified-invoice-picker.md](./unified-invoice-picker.md) for the full picker flow)
```

### Schema Addition

One new table, [paperless_trip_links](../../src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/up.sql):

| Column | Type | Notes |
|---|---|---|
| `trip_id` | TEXT PRIMARY KEY | FK to `trips(id)` |
| `paperless_document_id` | INTEGER NOT NULL UNIQUE | Indexed for join in `list_paperless_links_for_docs` |
| `created_at`, `updated_at` | TEXT | ISO-8601 timestamps |

Symmetric with the dependent-side `trip_id UNIQUE` pattern of the existing `receipts` table — see [ADR-019](../../DECISIONS.md).

## Design Decisions

- **Why a separate table instead of adding `paperless_document_id` to `receipts`?** — Receipts and Paperless documents are independent attachment paths. The `receipts` table carries 20+ OCR-specific fields (path, currency, OCR confidence, etc.) that have no Paperless analogue, and we want toggling Paperless on/off to be byte-for-byte non-destructive to the local view.
- **Why `Authorization: Token <PAT>` instead of `Bearer`?** — Paperless-ngx uses Django REST Framework token auth, not OAuth2. See [BIZ-015](../../DECISIONS.md).
- **Why hardcode the custom-field names (`total_amount`, `litres`, `receipt_datetime`)?** — They're load-bearing for parsing the document JSON. Surfacing them as user-configurable Settings would multiply test combinations without solving any real problem (the user controls naming on the Paperless side).
- **Why not cache Paperless documents in SQLite?** — Document state lives in Paperless. The kniha-jazd app caches only the trip-link row. A sync of >100 docs that exceeds the 5s timeout per page is a known design ceiling for v1; bumping the timeout or implementing per-page caching is deferred.

## Out of Scope (deferred)

- Migrating local receipts INTO Paperless (one-way bridge only — Paperless is read).
- Best-effort fuzzy matching of pre-existing local receipts to Paperless documents.
- Bulk multi-select assign / unassign UI.
- Storing Paperless documents themselves in SQLite (only the trip-link row is persisted).
- Paperless tags beyond `fuel` and `car`.
- Encrypting the PAT in the OS keyring (matches HA's plaintext-JSON convention; revisit globally with HA).
- Multi-vehicle scoping — see [BIZ-016](../../DECISIONS.md).

## Related

- [Task 60](../../_tasks/_done/60-paperless-integration/) — original planning docs.
- [unified-invoice-picker.md](./unified-invoice-picker.md) — Task 64 unified the trip-assignment flow across Paperless docs and local receipts (one modal, one compat check).
- [ADR-008 in DECISIONS.md](../../DECISIONS.md) — frontend-display-only constraint that drove the `get_invoice_source_mode` mode-switch command.
- [ADR-019, BIZ-015, BIZ-016 in DECISIONS.md](../../DECISIONS.md) — schema, auth header, and v1-scope decisions.
- [ADR-020, ADR-021 in DECISIONS.md](../../DECISIONS.md) — unified-picker boundary contract and `mismatch_override` semantics.
