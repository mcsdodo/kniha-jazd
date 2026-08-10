**Date:** 2026-08-08
**Subject:** Settings UI must show which values come from environment variables
**Status:** Complete

## Background

Task 67 (server / always-on runner) added environment-variable overrides for the
integration secrets so the Docker/homelab deployment can be configured without
touching [local.settings.json](../../local.settings.json.sample). The mechanism
lives in [settings.rs](../../src-tauri/core/src/settings.rs):

- `LocalSettings::load_effective()` layers env vars over the file at every
  **consumption** site.
- Setters use plain `load()` + `save()` and refuse to write a field that is
  pinned by an env var (`LocalSettings::env_pinned`), returning e.g.
  *"Home Assistant URL is managed by the HA_URL environment variable"*.

Overridable variables: `GEMINI_API_KEY`, `HA_URL`, `HA_API_TOKEN`,
`PAPERLESS_URL`, `PAPERLESS_API_TOKEN`, `PAPERLESS_ENABLED`.

**The gap:** the Settings page has no idea any of this exists. `get_ha_settings`
returns only `{ url, hasToken }` and `get_paperless_settings` only
`{ url, hasToken, enabled, fieldName* }` — no "this came from the environment"
signal. So on the homelab instance the user sees ordinary, fully-enabled inputs
holding env-provided values. The guard only fires *after* they type: the debounced
auto-save calls the setter, the backend rejects it, and a red error toast appears.

Two concrete bugs fall out of that:

1. **Editing the token reports a URL error.** `saveHaSettingsNow`
   ([+page.svelte:272](../../src/routes/settings/+page.svelte)) always sends both
   fields — `saveHaSettings(haUrl || null, haApiToken || null)`. With `HA_URL` set,
   `url.is_some()` is true on every save, so the URL guard trips first even when the
   user only touched the token. Same shape in the Paperless section.
2. **The one existing "override" flag is wrong and unused.**
   `ReceiptSettings.gemini_api_key_from_override` is computed as
   `local.gemini_api_key.is_some()`
   ([receipts_cmd.rs:46](../../src-tauri/core/src/commands_internal/receipts_cmd.rs))
   — true whenever *any* key exists, file or env — and the frontend never reads it
   (declared only in [types.ts:237](../../src/lib/types.ts)).

A third, separate observation came up while reviewing the page: the **"Priečinok s
dokladmi"** (receipts folder) control is desktop-shaped. Its only affordance is a
"Zmeniť" button that opens a native Tauri directory dialog
([+page.svelte:458](../../src/routes/settings/+page.svelte)), which cannot work in
the browser. The folder path itself *is* still meaningful server-side —
`scan_receipts_internal` / `sync_receipts_internal` read it — so the setting is not
irrelevant to the web app, only its editor is. The
[capabilities store](../../src/lib/stores/capabilities.ts) already carries a
`features.fileDialogs` flag for exactly this case, and it is currently unused.

## User-visible goals

1. **Env-managed settings are not editable.** Any field pinned by an environment
   variable renders disabled in the Settings page. No typing, no debounced save, no
   error toast.

2. **The eye icon reveals the real value.** For an env-pinned token/key, toggling
   the eye shows the actual value taken from the environment variable (not `********`,
   not the stale file value). Decided explicitly with the user over showing the
   variable *name* instead.

3. **It is obvious at a glance what is env-managed and what is editable.** Each
   pinned field carries a visible marker naming the variable it comes from (e.g. a
   small `HA_API_TOKEN` badge next to the label) plus a hint line. Un-pinned fields
   in the same section stay normal and editable — the distinction has to be readable
   field-by-field, not section-by-section, because a deployment can pin `HA_URL` and
   leave the token in the file (or vice versa).

4. **Connection status keeps working.** The "✓ Pripojené" / "✗ Nepripojené" indicator
   for both Home Assistant and Paperless must still appear and still auto-test on
   page load when the values come from the environment. This works today
   ([+page.svelte:636](../../src/routes/settings/+page.svelte),
   [:653](../../src/routes/settings/+page.svelte)) and must not regress when the
   inputs become disabled.

5. **Receipts folder behaves sensibly in the web app.** The native-dialog "Zmeniť"
   button is hidden when `capabilities.features.fileDialogs` is false; in server mode
   the path becomes a plain editable text field instead, so the setting remains
   usable from a browser.

## Non-goals

- Adding new overridable env variables (e.g. a `RECEIPTS_FOLDER` one). Preferences
  — theme, hidden columns, date prefill, backup retention, Paperless custom field
  names — stay file/UI-managed, per the decision recorded in
  [settings-architecture.md](../../docs/features/settings-architecture.md).
- Changing the precedence rules or the `load_effective` / `load` split.
- Reworking the Settings page layout beyond the markers described above.

## Security note

Goal 2 means env-pinned secrets are sent to the frontend. For Home Assistant and
Gemini this is not a new exposure — `get_local_settings_for_ha` already returns
`ha_api_token` in full, and `get_receipt_settings` already returns `gemini_api_key`.
For Paperless it is new. Since server mode serves this page over the LAN, the value
is exposed only for fields that are **env-pinned** (where the operator already
controls the deployment), and file-stored tokens keep today's `hasToken`-only
behavior.

## Related

- Task 67 — [online always-on runner](../67-online-always-on-runner/), which
  introduced the env overrides.
- [docs/features/settings-architecture.md](../../docs/features/settings-architecture.md)
- [docs/features/server-mode.md](../../docs/features/server-mode.md)
