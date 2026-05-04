**Date:** 2026-04-27
**Subject:** Paperless-ngx integration as alternative invoice source
**Status:** Planning

## Background

The user already runs a [Paperless-ngx](https://docs.paperless-ngx.com/) instance at
[documents.lacny.me](https://documents.lacny.me) and stores all fuel and other-cost
invoices there. Today, kniha-jazd has its own local-receipt OCR pipeline
([src/routes/doklady/+page.svelte](../../src/routes/doklady/+page.svelte)) that scans
a folder, OCRs invoices via Gemini, and lets the user assign each receipt to a trip.
The two systems are duplicate work for documents that already live in Paperless.

This task introduces an optional Paperless integration: when configured, the doklady
page treats Paperless as the source of truth for invoices, while preserving the
existing local-receipt flow when it isn't.

The original seed for this task is in [description.md](./description.md). The design
doc is [02-design.md](./02-design.md).

## User-visible goals

1. **Settings — Paperless section.** A new section in
   [Settings](../../src/routes/settings/+page.svelte) that mirrors the existing
   HomeAssistant integration: configurable URL, configurable PAT, and a
   "Test connection" button with IDLE / TESTING / CONNECTED / DISCONNECTED status.

2. **Doklady page — mode switch.** When Paperless is configured, the
   [doklady page](../../src/routes/doklady/+page.svelte) reads invoices live from
   Paperless (filtered by tag `fuel` or `car`) instead of from the local `receipts`
   table.

3. **Buttons differ per mode.**
   - Paperless **off** (today's behavior): Edit, Reprocess, Remove, Open-local-file,
     Assign-to-trip.
   - Paperless **on**: only **Open-in-Paperless** (browser link to the Paperless
     document) and **Assign-to-trip**. Edit, Reprocess, Remove are **hidden**.

4. **Trip assignment still works** in both modes. In Paperless mode, the link is
   stored in a new `paperless_trip_links` table (separate from the `receipts.trip_id`
   used in local mode).

5. **Non-destructive.** Existing local receipts and their trip assignments stay
   exactly as they are. Toggling Paperless off restores the local view.

## Data model

Two parallel attachment paths, both 1:1 with a trip:

- **Local mode:** `receipts.trip_id REFERENCES trips(id)` *(today's schema, untouched)*
- **Paperless mode:** new table `paperless_trip_links(trip_id, paperless_document_id)`,
  same dependent-side pattern as `receipts`.

The `receipts` table is **frozen but intact** when Paperless is on — re-readable any
time Paperless is disabled. The `trips` table is **never touched**.

## Custom fields in Paperless (user must create)

Documents tagged `fuel` or `car` are expected to have:

| Custom field name | Type | Notes |
|---|---|---|
| `total_amount` | monetary or float | total paid (already exists in user's setup) |
| `receipt_datetime` | string | ISO-8601 datetime, e.g. `2026-04-15T14:30` |

If a document has the `fuel`/`car` tag but no `receipt_datetime`, the row still
appears in the doklady grid with `?` in the date column and remains assignable.

## Tag → assignment mapping

| Paperless tag | AssignmentType in app |
|---|---|
| `fuel` | Fuel |
| `car` | Other |
| both | Fuel (priority) |
| neither | not returned (server-side filter) |

## Configuration mirrors the HomeAssistant pattern

- URL + PAT stored plaintext in `LocalSettings` JSON
  ([src-tauri/core/src/settings.rs](../../src-tauri/core/src/settings.rs)) — same
  convention as `ha_url` / `ha_api_token`.
- "Test connection" calls a Tauri command (added to
  [src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs))
  that hits Paperless with a 5s timeout.
- Auth header is `Authorization: Token <PAT>` *(Paperless DRF token auth — **not**
  Bearer, which is the one place we diverge from the HA pattern).*
- Settings UI mirrors the HA section: debounced auto-save (800ms), connection-status
  indicator (IDLE / TESTING / CONNECTED / DISCONNECTED).

## Architectural constraints (project conventions)

- **ADR-008** *(see [DECISIONS.md](../../DECISIONS.md))*: All conditional and
  business logic stays in Rust. The frontend asks the backend "which mode are we in,
  and what rows do I render?" — it does not branch on raw settings.
- **i18n**: All new UI text added to
  [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts) and
  [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts).
- **Read-only mode**: `assign_paperless_doc_to_trip` and `unassign_paperless_doc` are
  gated by `check_read_only!(app_state)`.
- **Test-driven**: Backend unit tests for tag mapping, custom-field resolution,
  test-connection happy/sad paths; one Tier-2 integration test for the
  Settings → Test → Doklady-render → Assign flow.

## Out of scope

- Migrating existing local receipts into Paperless (one-way bridge only).
- Best-effort fuzzy matching of local receipts to Paperless documents.
- Bulk operations (multi-select assign, etc.).
- Storing Paperless documents themselves in SQLite (we cache only the trip link).
- Other Paperless tags beyond `fuel` and `car`.
- Encrypting the PAT in the OS keyring (matches HA's plaintext-JSON convention; can
  revisit globally later).

## Acceptance criteria

- [ ] [Settings page](../../src/routes/settings/+page.svelte) shows a Paperless
      section with URL, PAT, and Test button.
- [ ] Test button reports CONNECTED for valid URL+PAT, DISCONNECTED otherwise (with
      reason in tooltip on hover).
- [ ] When Paperless is configured and reachable, the
      [doklady page](../../src/routes/doklady/+page.svelte) renders documents from
      Paperless filtered by `fuel`/`car` tags, scoped to the selected vehicle/year.
- [ ] When Paperless is **not** configured, doklady behaves exactly as today.
- [ ] In Paperless mode, **Open** opens the document in the user's default browser at
      the Paperless URL; **Assign** writes a row to `paperless_trip_links`; **Edit**,
      **Reprocess**, **Remove** are not visible.
- [ ] Toggling Paperless off restores the local-receipts view with all prior
      assignments intact.
- [ ] Backend unit tests + one Tier-2 integration test cover the flow.
- [ ] [CHANGELOG.md](../../CHANGELOG.md) entry under Unreleased.
- [ ] [DECISIONS.md](../../DECISIONS.md) entries for: integration FK strategy
      (D-symmetric link table) and auth header divergence from HA.
