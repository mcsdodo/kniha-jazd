# Design — Env-managed settings in the Settings UI

Goals and background: [01-task.md](./01-task.md).

## Guiding principle

ADR-008: the backend decides, the frontend displays. The frontend must never call
`process.env`, never hardcode variable names, and never infer "this looks
env-managed". Every command that returns a settings block also returns, per field,
whether that field is pinned and (for secrets) the effective value to show.

## Backend

### 1. Name the variables once

[settings.rs](../../src-tauri/core/src/settings.rs) currently repeats the six env
var names as string literals in `apply_overrides`, in every setter guard, and in the
test scrubber. Introduce a `pub mod env_vars` with one `pub const` per variable and
use it everywhere. This is what lets the response structs ship the variable *name*
to the UI without the frontend hardcoding it.

```
env_vars::GEMINI_API_KEY      = "GEMINI_API_KEY"
env_vars::HA_URL              = "HA_URL"
env_vars::HA_API_TOKEN        = "HA_API_TOKEN"
env_vars::PAPERLESS_URL       = "PAPERLESS_URL"
env_vars::PAPERLESS_API_TOKEN = "PAPERLESS_API_TOKEN"
env_vars::PAPERLESS_ENABLED   = "PAPERLESS_ENABLED"
```

### 2. Extend the response structs

[integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs):

`HaSettingsResponse` gains
- `url_from_env: bool` — `env_pinned(HA_URL)`
- `token_from_env: bool` — `env_pinned(HA_API_TOKEN)`
- `token_env_value: Option<String>` — the effective token, populated **only** when
  `token_from_env` is true; `None` otherwise

`PaperlessSettingsResponse` gains the same three plus
- `enabled_from_env: bool` — `env_pinned(PAPERLESS_ENABLED)`

[receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs):

`ReceiptSettings` — the two `*_from_override` flags are replaced, not extended.
`gemini_api_key_from_override` was `local.gemini_api_key.is_some()`, which answers
"is a key configured", not "is it env-pinned"; `receipts_folder_from_override` is
meaningless because the receipts folder has no env variable at all. Both are unused
by the frontend, so removing them is free.

- **remove** `gemini_api_key_from_override`, `receipts_folder_from_override`
- **add** `gemini_api_key_from_env: bool` — `env_pinned(GEMINI_API_KEY)`

`gemini_api_key` is already returned in full by `get_receipt_settings_internal`, so
no separate `*_env_value` field is needed — the existing field already carries the
effective (env-overridden) value.

Why `token_env_value` is separate from `has_token` rather than just always returning
the token: file-stored tokens keep today's write-only behavior (`hasToken: bool`, UI
shows `********`). Only env-pinned tokens — where the operator already controls the
deployment and set the variable themselves — are echoed back. See the security note
in [01-task.md](./01-task.md).

### 3. Do not change the setter guards

`save_ha_settings_internal` / `save_paperless_settings_internal` /
`set_gemini_api_key_internal` keep refusing env-pinned writes. The disabled inputs
are UX; the guards remain the enforcement boundary (a browser client can call
`/api/rpc` directly). The guards only get the `env_vars::` constants substituted for
their string literals.

### 4. Server dispatcher

[dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) forwards to the same
`_internal` functions and serializes whatever they return, so the new fields reach
the web client with no dispatcher change. Verified for `get_ha_settings`,
`get_paperless_settings`, `get_receipt_settings`.

## Frontend

### State

Per pinned field the page holds a boolean; the values themselves reuse the existing
variables so the rest of the page (validation, connection test, `haConfigured`)
keeps working unchanged:

```
haUrlFromEnv, haTokenFromEnv
paperlessUrlFromEnv, paperlessTokenFromEnv, paperlessEnabledFromEnv
geminiKeyFromEnv
```

On load, when a token is env-pinned, seed **both** `haApiToken` and
`initialHaApiToken` from `tokenEnvValue`. This is what makes the eye icon reveal the
real value — the existing `type={show ? 'text' : 'password'}` binding then does the
work with no new markup — and seeding `initial*` too keeps the change-detection
guard in `saveHaSettingsNow` from firing.

### Disabling and marking

Each pinned control gets `disabled={…FromEnv}` and a marker beside its label:

```
Home Assistant token   [HA_API_TOKEN]        ← badge, monospace, muted background
┌────────────────────────────────┬───┐
│ ••••••••••••••••••••••••       │ 👁 │      ← disabled, eye still works
└────────────────────────────────┴───┘
Spravované premennou prostredia.
```

The badge text is the variable name straight from the backend response, so a reader
can map a field to the variable to change without consulting docs. The eye button
itself stays enabled — revealing is a read, not an edit.

A shared `EnvBadge` component is deliberately **not** introduced: it would be a
one-element wrapper (`<span class="env-badge">{name}</span>`) used in five places
inside a single file. A local snippet plus one CSS rule is less machinery.

### Save paths

Two changes, both in the "don't send what you can't change" direction:

1. `saveHaSettingsNow` / `savePaperlessSettingsNow` send `null` for env-pinned
   fields instead of the current value. This fixes the bug where editing the token
   raised a *URL* error, and means an operator who pins only `HA_URL` can still edit
   the token from the UI.
2. If every field in a section is pinned, the save function returns early.

### Receipts folder

Gate on the existing (currently unused)
[capabilities](../../src/lib/stores/capabilities.ts) flag rather than on `IS_TAURI`,
because the server already publishes `features.file_dialogs` over
`/api/capabilities`:

- `features.fileDialogs === true` (desktop) — unchanged: read-only path display plus
  the native "Zmeniť" dialog button.
- `features.fileDialogs === false` (server/web) — the "Zmeniť" button is hidden and
  the path becomes a debounced text input, saved through the existing
  `set_receipts_folder_path`. The path is a path **on the server**, so a hint says so.

This answers the "is the receipts folder relevant to the web app at all?" question:
the value is (receipt scanning reads it server-side), only the native-dialog editor
isn't.

### i18n

New keys in both [sk](../../src/lib/i18n/sk/index.ts) and
[en](../../src/lib/i18n/en/index.ts):

| Key | sk | en |
|-----|----|----|
| `settings.envManaged` | `Spravované premennou prostredia.` | `Managed by an environment variable.` |
| `settings.envManagedTitle` | `Táto hodnota je nastavená premennou prostredia {name} a nedá sa tu zmeniť.` | `This value is set by the {name} environment variable and cannot be changed here.` |
| `settings.receiptsFolderServerHint` | `Cesta k priečinku na serveri.` | `Folder path on the server.` |

## Test ownership

| Use-case | Backend unit test | Integration test |
|----------|-------------------|------------------|
| `env_pinned` per variable | ✅ exists ([settings_tests.rs](../../src-tauri/core/src/settings_tests.rs)) | ❌ |
| Response carries correct `*FromEnv` flags | ✅ new | ❌ |
| `tokenEnvValue` present only when pinned | ✅ new | ❌ |
| Setter still refuses pinned writes | ✅ exists | ❌ |
| Input disabled + badge visible when pinned | ❌ | ✅ new |
| Connection status still appears when env-configured | ❌ | ✅ new |
| Receipts folder editor swaps by capability | ❌ | ✅ new (server-mode spec) |

Integration coverage goes in server mode, where env vars can be injected into the
spawned process — see [03-plan.md](./03-plan.md) for the mechanism.

## Rejected alternatives

- **Reveal the variable name instead of the value.** Safer (no secret crosses the
  IPC boundary) and was the initial recommendation, but the user chose the value:
  on a homelab box the operator wants to confirm *which* token is live, not be told
  the name of a variable they just typed into a compose file. The name is still
  shown, in the badge.
- **Hide env-managed sections entirely.** Loses the connection status, which goal 4
  explicitly requires, and hides the fact that the integration is configured at all.
- **A generic `SettingsField` component wrapping label + input + badge.** The five
  call sites differ (password + eye, plain text, checkbox, path display); the wrapper
  would need enough slots and props to cost more than it saves.
