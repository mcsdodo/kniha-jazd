**Date:** 2026-05-04
**Subject:** Paperless enable/disable toggle
**Status:** Planning

# 61 — Paperless Enable/Disable Toggle

## Goal

Add a toggle to the Paperless-ngx settings section that lets the user switch between Paperless mode and local-files mode **without losing their saved URL and token**.

## Design

### Backend

**`LocalSettings`** ([src-tauri/core/src/settings.rs](../../src-tauri/core/src/settings.rs))
- Add `paperless_enabled: Option<bool>`
- `None` = backward-compat default: treated as `true` when URL + token are both set (mirrors `auto_check_updates`)

**`determine_invoice_source_mode`** ([src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs))
- Before: Paperless when `url.is_some() && token.is_some()`
- After: Paperless when `enabled (with None→true fallback) && url.is_some() && token.is_some()`

**`save_paperless_settings_internal`** ([src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs))
- Add `enabled: Option<bool>` parameter
- `None` = keep existing value (same pattern as `url` / `token`)

**`PaperlessSettingsResponse`** ([src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs))
- Add `enabled: bool` field (resolved value, never None)

### Frontend

**`PaperlessSettings` type** ([src/lib/types.ts](../../src/lib/types.ts))
- Add `enabled: boolean`

**`savePaperlessSettings`** ([src/lib/api.ts](../../src/lib/api.ts))
- Add `enabled: boolean | null` parameter (`null` = keep existing)

**Settings page** ([src/routes/settings/+page.svelte](../../src/routes/settings/+page.svelte))
- Toggle switch at the top of the Paperless section (above the URL field)
- Disabled (greyed out + tooltip) when no URL + token are saved — can't enable without credentials
- On toggle: calls `savePaperlessSettings(null, null, newValue)` immediately

### Tests

**Backend unit tests** ([src-tauri/core/src/commands_internal/integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs))
- `enabled=false` with valid URL + token → `Local`
- `enabled=None` with valid URL + token → `Paperless` (backward compat)
- `enabled=true` with missing URL → `Local`

**Integration test** ([tests/integration/specs/tier2/paperless-integration.spec.ts](../../tests/integration/specs/tier2/paperless-integration.spec.ts))
- Toggle off → Doklady page shows local receipts view
- Toggle back on → Doklady switches to Paperless mode

### i18n

New strings needed in [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts) (and [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts)):
- `paperless.enableToggle` — toggle label, e.g. "Povoliť Paperless-ngx"
- `paperless.enableToggleDisabledHint` — tooltip when disabled, e.g. "Najprv nastav URL a token"
