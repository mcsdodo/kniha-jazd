**Date:** 2026-04-27
**Subject:** Paperless-ngx integration — design
**Status:** Planning

Companion to [01-task.md](./01-task.md). The original seed is in
[description.md](./description.md).

## Architecture

The [doklady page](../../src/routes/doklady/+page.svelte) becomes a **mode switch**
driven by whether `LocalSettings.paperless_url` and `paperless_api_token` (in
[src-tauri/core/src/settings.rs](../../src-tauri/core/src/settings.rs)) are populated.
Two parallel attachment worlds:

```
Paperless OFF (today)              Paperless ON (new)
─────────────────────              ─────────────────────────────────────
SQLite `receipts` table            Live GET /api/documents/?tags__id__in=...
   ↓                                  ↓
doklady page (read)                doklady page (read)
   ↓                                  ↓
trip assignment writes             trip assignment writes
  receipts.trip_id                   paperless_trip_links row
```

The `receipts` table is frozen but intact when Paperless is on. Toggling Paperless
off restores the local-receipts view exactly. The `trips` table is never modified.

The mode switch and all data-shaping live in Rust (ADR-008, see
[DECISIONS.md](../../DECISIONS.md)). The frontend calls a single command and renders
rows; it does not branch on raw settings.

## Schema change — one new table

Migration folder:
[src-tauri/core/migrations/2026-04-27-100000_add_paperless_trip_links/](../../src-tauri/core/migrations/)
(to be created).

```sql
-- up.sql
CREATE TABLE paperless_trip_links (
    trip_id TEXT PRIMARY KEY,
    paperless_document_id INTEGER NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (trip_id) REFERENCES trips(id)
);
CREATE INDEX idx_paperless_links_doc
    ON paperless_trip_links(paperless_document_id);
```

```sql
-- down.sql
DROP INDEX IF EXISTS idx_paperless_links_doc;
DROP TABLE IF EXISTS paperless_trip_links;
```

This mirrors the dependent-side `trip_id UNIQUE` pattern of the existing `receipts`
table — symmetric, 1:1 with a trip, and physically separate from the OCR-laden
`receipts` schema (which carries 20+ fields with no Paperless analogue).

## Settings — mirror HomeAssistant integration

Two new fields on `LocalSettings`
([src-tauri/core/src/settings.rs](../../src-tauri/core/src/settings.rs)):

```rust
pub paperless_url: Option<String>,         // e.g. "https://documents.lacny.me"
pub paperless_api_token: Option<String>,   // PAT, plaintext (matches HA)
```

UI in [src/routes/settings/+page.svelte](../../src/routes/settings/+page.svelte):
a new section copy-paste of the HA section, same debounced auto-save (800ms), same
IDLE / TESTING / CONNECTED / DISCONNECTED status indicator pattern.

### Test-connection command

Added to
[src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs):

```rust
pub async fn test_paperless_connection(
    app_handle: tauri::AppHandle,
) -> Result<bool, String> {
    // GET {url}/api/ui_settings/   -- always 2xx for valid token, lightweight
    // Header: Authorization: Token {pat}   -- DRF token auth, NOT Bearer
    // Timeout: 5s (matches HA)
}
```

> ⚠️ **Critical divergence from HA:** Paperless-ngx uses Django REST Framework token
> auth, so the header is `Authorization: Token <PAT>`, **not** `Bearer`. This is
> covered by a unit test to prevent regression.

## Custom-field name → ID resolution

Paperless API returns custom fields by **numeric ID**, not name. Document JSON looks
like:

```json
{
  "id": 42,
  "title": "Shell 2026-04-15",
  "tags": [3, 7],
  "custom_fields": [
    { "field": 1, "value": 65.40 },
    { "field": 2, "value": "2026-04-15T14:30" }
  ]
}
```

Resolution at runtime (in new module
[src-tauri/core/src/paperless.rs](../../src-tauri/core/src/paperless.rs)):

```rust
struct PaperlessFieldMap {
    total_amount_id: i64,
    receipt_datetime_id: i64,
}
// Populated by GET /api/custom_fields/ on the first sync of the session,
// cached in AppState. If a name isn't found, return a structured error to the UI:
//   "Custom field 'receipt_datetime' not found in Paperless. Create it as a
//    string field."
```

Field names are hardcoded (`total_amount`, `receipt_datetime`) — the user does not
type them into Settings; they only configure URL and PAT.

## Doklady page — Paperless mode

Single Tauri command for the page to call:

```rust
get_paperless_invoices(vehicle_id: &str, year: i32)
    -> Result<Vec<PaperlessInvoiceRow>, String>
```

Backend flow (in new module
[src-tauri/core/src/commands_internal/paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs)):

1. Resolve custom-field IDs (cached after first call).
2. Resolve tag IDs for `fuel` and `car` via `/api/tags/?name__iexact=...` (cached).
3. `GET /api/documents/?tags__id__in={fuel_id},{car_id}&page_size=100`, follow
   `next` URL until exhausted.
4. For each doc, extract `id`, `title`, `created`, custom fields by resolved IDs,
   tag set.
5. Map tags → `AssignmentType` (`fuel` → Fuel; `car` → Other; both → Fuel priority).
6. Filter to year using `receipt_datetime` if present, else fallback to `created`.
7. LEFT JOIN with `paperless_trip_links` to know which docs are already assigned.
8. Return a flat `PaperlessInvoiceRow` shape close enough to the existing `Receipt`
   shape (in [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs)) that
   the Svelte grid can render with minimal branching.

`PaperlessInvoiceRow` (illustrative — added to
[src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs)):

```rust
pub struct PaperlessInvoiceRow {
    pub paperless_document_id: i64,
    pub title: String,
    pub paperless_url: String,            // {base}/documents/{id}/
    pub total_price_eur: Option<f64>,
    pub receipt_datetime: Option<NaiveDateTime>,
    pub assignment_type: AssignmentType,  // Fuel | Other
    pub trip_id: Option<String>,          // from paperless_trip_links
}
```

### Buttons in Paperless mode

| Button | Behavior |
|---|---|
| **Open** | Opens `{paperless_url}/documents/{doc_id}/` in default browser |
| **Assign** | Existing `TripSelectorModal`; on confirm calls `assign_paperless_doc_to_trip(doc_id, trip_id)` (UPSERT into `paperless_trip_links`) |
| **Unassign** | `unassign_paperless_doc(doc_id)` → DELETE FROM `paperless_trip_links` WHERE `paperless_document_id = ?` |
| **Edit / Reprocess / Remove** | **Hidden** (read-only against Paperless) |

Empty-`receipt_datetime` rows render with `?` in the date column; assignment still
works manually.

## Sync UX

- **Live fetch on page open** — one API round-trip behind the existing skeleton
  loader. No SQLite caching of Paperless documents themselves.
- **"Refresh from Paperless"** button in the page toolbar for explicit re-pull
  mid-session.
- Failure state: banner *"Paperless nedostupný — skontroluj nastavenia"* with a
  retry button. Assignment writes are blocked while offline (toast on attempt).
- Custom-field-missing state: banner pointing to the offending field name and the
  Paperless settings page.

## Trip assignment — backend writes

Two new commands, both gated by `check_read_only!(app_state)`:

```rust
assign_paperless_doc_to_trip(doc_id: i64, trip_id: &str) -> Result<(), String>
unassign_paperless_doc(doc_id: i64) -> Result<(), String>
```

UPSERT semantics: assigning a doc to a trip that already has a different doc linked
replaces the link (1:1 invariant via `trip_id PRIMARY KEY` and
`paperless_document_id UNIQUE`).

### Edge case: trip has both a local receipt AND a Paperless link

Allowed by the schema — both rows can coexist. In Paperless mode, the Paperless link
"wins" for display. In local mode, the local receipt "wins". This is informational,
not corrupting; nothing forces the user to clear one when adding the other.

## Mode-switch query — single source of truth

```rust
pub fn get_invoice_source_mode(settings: &LocalSettings) -> InvoiceSourceMode {
    match (&settings.paperless_url, &settings.paperless_api_token) {
        (Some(url), Some(tok)) if !url.is_empty() && !tok.is_empty()
            => InvoiceSourceMode::Paperless,
        _ => InvoiceSourceMode::Local,
    }
}
```

The frontend calls a single `get_invoice_source_mode` Tauri command on doklady page
load and chooses which renderer to mount based on the answer. It never inspects raw
settings.

## Error handling — full matrix

| Scenario | Behavior |
|---|---|
| Paperless URL invalid in settings | Test button shows DISCONNECTED, tooltip = parse error |
| 401 on test or sync | DISCONNECTED with "Neplatný token" |
| Network unreachable mid-session | Doklady banner + retry; assignment writes blocked with toast |
| Custom field name not found in Paperless | Sync error: "Vytvor custom field `<name>` v Paperless" |
| Tag `fuel` or `car` not found in Paperless | Sync error: "Pridaj tag `<name>` v Paperless a označ ním invoice" |
| Doc has both `fuel` and `car` tags | Treated as Fuel (priority order in Rust) |
| Doc has neither tag | Not returned (server-side filter excludes) |
| Trip already linked to a Paperless doc, user assigns a different one | Old link deleted, new one created (UPSERT) |
| Read-only mode | Both assign/unassign commands fail fast via `check_read_only!` |

## HTTP client — reqwest, mirroring HA

- Uses `reqwest` (already in dependency tree per HA).
- 5-second timeout for both test-connection and document fetch.
- Pagination: `page_size=100`, follow the `next` URL until null.
- Connection pool: default reqwest pooling — no special tuning needed for a desktop
  app's volume.

## Testing strategy

### Backend unit tests

New file:
[src-tauri/core/src/paperless_tests.rs](../../src-tauri/core/src/paperless_tests.rs)
(to be created)

- **Custom-field name → ID resolution** with stub `/api/custom_fields/` JSON.
- **Document parsing**: tag → AssignmentType mapping (fuel only, car only, both,
  neither).
- **Year filtering**: `receipt_datetime` present, missing (fallback to `created`),
  string parsing of ISO-8601.
- **Mode-switch query**: empty url, empty token, both populated, both empty.

Extensions to existing test files:

- [src-tauri/core/src/commands_tests.rs](../../src-tauri/core/src/commands_tests.rs):
  `assign_paperless_doc_to_trip` UPSERT semantics; `unassign_paperless_doc` happy +
  idempotent (delete-when-missing is a no-op); `check_read_only!` gating on both.
- [src-tauri/core/src/integrations_tests.rs](../../src-tauri/core/src/integrations_tests.rs)
  (or the equivalent file for integration command tests):
  `test_paperless_connection` happy (200), 401, timeout, network error — using a
  mock HTTP server. **Auth header is `Authorization: Token <PAT>`** — explicit
  assertion to prevent regression to Bearer.

### Integration test (Tier 2, WebdriverIO)

New file:
[tests/integration/specs/tier2/paperless-integration.spec.ts](../../tests/integration/specs/tier2/paperless-integration.spec.ts)
(to be created)

1. Open Settings, enter URL + PAT, click Test → assert CONNECTED indicator.
2. Navigate to Doklady → assert rows from mocked Paperless API render with title and
   total.
3. Click Assign on a row → pick a trip → assert `paperless_trip_links` row exists
   (via a debug query command if one exists, or by re-rendering and asserting the
   row shows as assigned).
4. Refresh page → row still shown as assigned.
5. Clear Paperless URL in Settings → Doklady reverts to local-receipt view; the
   previously assigned row no longer appears (but the link row remains in DB,
   dormant, ready if Paperless is re-enabled).

The mock Paperless server is launched in the test harness (mirroring how the
existing HA integration test mocks HA endpoints). Mock server helper:
[tests/integration/specs/_helpers/mock-paperless-server.ts](../../tests/integration/specs/_helpers/mock-paperless-server.ts)
(to be created).

## Files to add / change

### Rust backend

- [src-tauri/core/migrations/](../../src-tauri/core/migrations/) — new folder
  `2026-04-27-100000_add_paperless_trip_links/` with `up.sql` + `down.sql`
- [src-tauri/core/src/schema.rs](../../src-tauri/core/src/schema.rs) *(regenerated by
  Diesel)*
- [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) *(new
  `PaperlessTripLink` struct; `PaperlessInvoiceRow` row shape)*
- [src-tauri/core/src/settings.rs](../../src-tauri/core/src/settings.rs) *(two new
  fields on `LocalSettings`)*
- [src-tauri/core/src/paperless.rs](../../src-tauri/core/src/paperless.rs)
  *(new module — HTTP client, parsing, field resolution)*
- [src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs)
  *(extend with `test_paperless_connection`, `get_invoice_source_mode`)*
- [src-tauri/core/src/commands_internal/paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs)
  *(new — `get_paperless_invoices`, `assign_paperless_doc_to_trip`,
  `unassign_paperless_doc`)*
- [src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs) *(register new
  Tauri commands)*

### Frontend

- [src/routes/settings/+page.svelte](../../src/routes/settings/+page.svelte) *(new
  Paperless section, mirrors HA)*
- [src/routes/doklady/+page.svelte](../../src/routes/doklady/+page.svelte)
  *(mode-aware rendering, button hiding)*
- [src/lib/api.ts](../../src/lib/api.ts) *(new wrappers for the new commands)*
- [src/lib/types.ts](../../src/lib/types.ts) *(`PaperlessSettings`,
  `PaperlessInvoiceRow`, `InvoiceSourceMode`)*
- [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts) and
  [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts) *(new keys:
  `settings.paperless.*`, `doklady.paperless.*`, error messages)*

### Tests

- [src-tauri/core/src/paperless_tests.rs](../../src-tauri/core/src/paperless_tests.rs)
- [tests/integration/specs/tier2/paperless-integration.spec.ts](../../tests/integration/specs/tier2/paperless-integration.spec.ts)
- [tests/integration/specs/_helpers/mock-paperless-server.ts](../../tests/integration/specs/_helpers/mock-paperless-server.ts)

### Docs

- [docs/features/paperless-integration.md](../../docs/features/paperless-integration.md)
  *(after completion)*
- [CHANGELOG.md](../../CHANGELOG.md) *(Unreleased)*
- [DECISIONS.md](../../DECISIONS.md) *(ADR for D-symmetric link table; ADR/BIZ for
  auth header divergence from HA)*

## Defaults locked in (sign-off recap)

| Default | Value |
|---|---|
| Auth header | `Authorization: Token <PAT>` (Paperless DRF, **not** Bearer) |
| PAT storage | Plaintext in `LocalSettings` JSON (mirrors HA convention) |
| Test endpoint | `GET /api/ui_settings/` (lightweight, always 2xx for valid token) |
| Custom field names (hardcoded) | `total_amount`, `receipt_datetime` |
| Custom field date type | `string`, ISO-8601 (e.g. `2026-04-15T14:30`) |
| Empty `receipt_datetime` | Row shown with `?`; assignment still works |
| Tag mapping | `fuel` → Fuel, `car` → Other, both → Fuel (priority) |
| Server-side tag filter | `?tags__id__in={fuel_id},{car_id}` |
| Page size | 100 (max), follow `next` until exhausted |
| Sync trigger | Live fetch on page open + manual "Refresh from Paperless" button |
| Year scoping | `receipt_datetime` year, fallback to `created` year |
| Paperless docs cached locally | **No** — only the `paperless_trip_links` row is persisted |
| HTTP timeout | 5s (matches HA) |
| HTTP client | `reqwest` (matches HA) |

## Out of scope (for later iterations)

- Encrypting the PAT in OS keyring (revisit globally once HA also moves).
- Best-effort fuzzy matching of pre-existing local receipts to Paperless documents.
- Bulk assign/unassign UI.
- Other Paperless tags beyond `fuel` / `car`.
- Migration tooling that pushes existing local receipts up to Paperless.
- Storing Paperless documents themselves in SQLite for offline use.
