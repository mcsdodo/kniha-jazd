**Date:** 2026-05-04
**Subject:** Paperless enable/disable toggle
**Status:** Planning

# Task 62: Paperless Enable/Disable Toggle

## Problem

Once a user configures Paperless-ngx (URL + token), there is no way to switch back to local receipts mode without clearing their credentials. Clearing the URL or token field in Settings silently does nothing because the frontend converts empty strings to `null` before saving, and the backend treats `null` as "keep existing value."

## Goal

Add a toggle switch to the Paperless-ngx section in Settings that lets the user enable or disable the integration globally, **without losing their saved URL and token**.

## User Story

> As a user, I want to temporarily switch back to local receipts without losing my Paperless credentials, so I can use the app from a device without Paperless access and switch back easily later.

## Acceptance Criteria

- [ ] A toggle switch appears at the top of the Paperless-ngx settings section
- [ ] The toggle is disabled (with tooltip) when no URL + token are saved yet
- [ ] Toggling off → Doklady page shows local receipts view
- [ ] Toggling on → Doklady page loads Paperless invoices
- [ ] Saved URL and token are **not** cleared when toggling off
- [ ] Existing users who already have URL + token set are unaffected (stay in Paperless mode)
- [ ] All backend unit tests pass
- [ ] Integration test verifies toggle-off → local mode, toggle-on → Paperless mode

## Design

See [01-design.md](./01-design.md) for architecture decisions.

## Implementation Plan

See [02-plan.md](./02-plan.md) for step-by-step TDD implementation.

## Files Affected

| File | Change |
|------|--------|
| [src-tauri/core/src/settings.rs](../../src-tauri/core/src/settings.rs) | Add `paperless_enabled: Option<bool>` field |
| [src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs) | Update mode logic, extend save fn + response struct |
| [src-tauri/core/src/commands_internal/integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs) | Add 5 new unit tests |
| [src-tauri/desktop/src/commands/integrations.rs](../../src-tauri/desktop/src/commands/integrations.rs) | Add `enabled` param to Tauri wrapper |
| [src/lib/types.ts](../../src/lib/types.ts) | Add `enabled: boolean` to `PaperlessSettings` |
| [src/lib/api.ts](../../src/lib/api.ts) | Add `enabled` param to `savePaperlessSettings` |
| [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts) | Add `enableToggle` + `enableToggleDisabledHint` |
| [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts) | Same in English |
| [src/lib/i18n/i18n-types.ts](../../src/lib/i18n/i18n-types.ts) | Add types for new i18n keys |
| [src/routes/settings/+page.svelte](../../src/routes/settings/+page.svelte) | Add toggle switch UI + state + handler |
| [tests/integration/specs/tier2/paperless-integration.spec.ts](../../tests/integration/specs/tier2/paperless-integration.spec.ts) | Update to test enabled/disabled toggle |
