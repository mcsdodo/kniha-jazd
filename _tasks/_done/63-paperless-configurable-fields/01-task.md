**Date:** 2026-05-04
**Subject:** Configurable Paperless Custom Field Names
**Status:** Planning

# Task 63: Configurable Paperless Custom Field Names

## Problem

Paperless-ngx integration looks up three custom fields by **hardcoded** English/UK strings in [paperless.rs](../../src-tauri/core/src/paperless.rs):72-76:

```rust
Ok(PaperlessFieldMap {
    total_amount_id: find("total_amount")?,
    litres_id: find("litres")?,
    receipt_datetime_id: find("receipt_datetime")?,
})
```

This forces every Paperless user to name their custom fields with these exact strings. Users in Slovak/Czech/German/etc. environments — or simply users with different naming conventions — get a `PaperlessError::CustomFieldNotFound` and integration setup fails.

## Goal

Let users configure the three custom field names via the Paperless settings UI. Current strings (`"receipt_datetime"`, `"litres"`, `"total_amount"`) are the defaults — existing users see no behavior change.

## User Story

> As a user, I want my Paperless custom fields to be named in my own language or convention (e.g., "Suma", "Litre", "Dátum dokladu"), and have the integration find them — without forcing me to rename fields I already use elsewhere.

## Acceptance Criteria

- [ ] Three new text inputs in the Paperless section of [settings/+page.svelte](../../src/routes/settings/+page.svelte): datetime field name, liters field name, total amount field name
- [ ] Each input is **prefilled** with the current default (`"receipt_datetime"`, `"litres"`, `"total_amount"`)
- [ ] Settings are persisted via the existing `save_paperless_settings` mechanism in [integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs):200-231
- [ ] [resolve_field_map](../../src-tauri/core/src/paperless.rs):60-77 uses configured names instead of hardcoded strings
- [ ] Empty/missing settings fall back to defaults — no breakage for existing users
- [ ] Backend unit test: lookup with custom name finds the field
- [ ] Backend unit test: missing/empty setting falls back to default
- [ ] Round-trip test: save custom names → load returns same names
- [ ] `cargo test` and `npm run test:all` pass

## Files Affected

| File | Change |
|------|--------|
| [src-tauri/core/src/settings.rs](../../src-tauri/core/src/settings.rs) | Add 3 `paperless_field_name_*` fields to `LocalSettings` |
| [src-tauri/core/src/paperless.rs](../../src-tauri/core/src/paperless.rs) | Replace hardcoded strings; `resolve_field_map` accepts a `PaperlessFieldNames` struct |
| [src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs) | Extend `PaperlessSettingsResponse`, `save_paperless_settings_internal` |
| [src-tauri/core/src/commands_internal/integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs) | Round-trip + defaults tests |
| [src-tauri/core/src/paperless_tests.rs](../../src-tauri/core/src/paperless_tests.rs) | Custom-name lookup tests |
| [src-tauri/desktop/src/commands/integrations.rs](../../src-tauri/desktop/src/commands/integrations.rs) | Pass new params through Tauri wrapper |
| [src/lib/types.ts](../../src/lib/types.ts) | Extend `PaperlessSettings` type |
| [src/lib/api.ts](../../src/lib/api.ts) | Pass new fields through `savePaperlessSettings` |
| [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts) | Slovak labels |
| [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts) | English labels |
| [src/lib/i18n/i18n-types.ts](../../src/lib/i18n/i18n-types.ts) | Generated types for new keys |
| [src/routes/settings/+page.svelte](../../src/routes/settings/+page.svelte) | 3 new text inputs in Paperless section |
| [tests/integration/specs/tier2/paperless-integration.spec.ts](../../tests/integration/specs/tier2/paperless-integration.spec.ts) | Custom field-name path |

## Design

See [02-design.md](./02-design.md).

## Related

- Builds on [Task 60](../_done/60-paperless-integration/) (Paperless Integration)
- Builds on [Task 62](../62-paperless-toggle/) (Paperless Toggle)
- Independent of [Task 64](../64-unified-invoice-picker/) (Unified Invoice Picker — follow-up)
