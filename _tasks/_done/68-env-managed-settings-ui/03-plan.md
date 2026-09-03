**Date:** 2026-08-08
**Subject:** Settings UI must show which values come from environment variables
**Status:** Complete

# Plan — Env-managed settings in the Settings UI

Task: [01-task.md](./01-task.md) · Design: [02-design.md](./02-design.md)

Each step is independently verifiable. Backend steps are test-first per the TDD rule
in [CLAUDE.md](../../CLAUDE.md).

---

## Step 1 — Backend: env var name constants

**Files:** [settings.rs](../../src-tauri/core/src/settings.rs)

Add `pub mod env_vars` with the six `&str` constants and replace every string
literal in `apply_overrides`, the setter guards, and
[test_env](../../src-tauri/core/src/settings.rs)'s `scrub_ambient_env` list.

Pure refactor — no behavior change. **Verify:**
`cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core` stays green
(the existing env tests in
[settings_tests.rs](../../src-tauri/core/src/settings_tests.rs) already cover the
override matrix).

---

## Step 2 — Backend: response flags (test-first)

**Files:**
[integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs),
[integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs),
[receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs),
[receipts_cmd_tests.rs](../../src-tauri/core/src/commands_internal/receipts_cmd_tests.rs)

### 2a. Write failing tests

In [integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs)
(reuse the existing `test_env::with_env_vars` / `test_env::lock` helpers):

| Test | Asserts |
|------|---------|
| `get_ha_settings_flags_env_pinned_fields` | with `HA_URL` + `HA_API_TOKEN` set: `url_from_env`, `token_from_env` true, `token_env_value == Some("env-token")` |
| `get_ha_settings_no_env_leaves_flags_false` | file-only token: both flags false, `token_env_value` is `None`, `has_token` still true |
| `get_ha_settings_pins_url_only` | only `HA_URL` set: `url_from_env` true, `token_from_env` false — the mixed case goal 3 calls out |
| `get_paperless_settings_flags_env_pinned_fields` | all three Paperless vars set: three flags true, `token_env_value` populated |
| `get_paperless_settings_no_env_hides_token_value` | file token: `has_token` true, `token_env_value` `None` |

In [receipts_cmd_tests.rs](../../src-tauri/core/src/commands_internal/receipts_cmd_tests.rs):

| Test | Asserts |
|------|---------|
| `get_receipt_settings_flags_env_pinned_key` | `GEMINI_API_KEY` set → `gemini_api_key_from_env` true and `gemini_api_key` carries the env value |
| `get_receipt_settings_file_key_not_flagged` | file-only key → flag false (this is the case the old `is_some()` flag got wrong) |

### 2b. Implement

Add the fields described in [02-design.md](./02-design.md) and populate them from
`LocalSettings::env_pinned(env_vars::…)`. Remove `gemini_api_key_from_override` and
`receipts_folder_from_override`.

**Verify:** `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core`

---

## Step 3 — Frontend types and API wrappers

**Files:** [types.ts](../../src/lib/types.ts), [api.ts](../../src/lib/api.ts)

Mirror the new fields in `HaSettings`, `PaperlessSettings`, `ReceiptSettings`
(camelCase: `urlFromEnv`, `tokenFromEnv`, `tokenEnvValue`, `enabledFromEnv`,
`geminiApiKeyFromEnv`); drop the two removed `*FromOverride` fields.

**Verify:** `npm run check` reports no new errors.

---

## Step 4 — Settings page: disable, mark, and stop saving pinned fields

**Files:** [+page.svelte](../../src/routes/settings/+page.svelte),
[sk/index.ts](../../src/lib/i18n/sk/index.ts),
[en/index.ts](../../src/lib/i18n/en/index.ts)

1. Add the i18n keys from [02-design.md](./02-design.md) to **both** locales.
2. Add the `*FromEnv` state variables; populate them in `onMount` alongside the
   existing loads. When a token is pinned, seed the bound value *and* its
   `initial*` twin from `tokenEnvValue`.
3. Mark up the five controls — Gemini key, HA URL, HA token, Paperless URL,
   Paperless token, Paperless enabled checkbox — with `disabled`, an env badge next
   to the label carrying the variable name, and the `envManaged` hint. Add
   `data-test` attributes (`ha-url-env-badge`, `ha-token-env-badge`,
   `paperless-url-env-badge`, `paperless-token-env-badge`, `gemini-key-env-badge`)
   for the integration spec.
4. Leave the eye buttons enabled.
5. In `saveHaSettingsNow` / `savePaperlessSettingsNow`, pass `null` for pinned
   fields and return early when every field in the section is pinned.
6. CSS for `.env-badge` and a muted style for disabled inputs.

**Verify:** `npm run check`; manual run with `set HA_URL=… & set HA_API_TOKEN=… &
npm run tauri:dev` — fields disabled, badge shows the variable name, eye reveals the
env token, no error toast on click, and the ✓ status still appears.

---

## Step 5 — Receipts folder editor by capability

**Files:** [+page.svelte](../../src/routes/settings/+page.svelte), both i18n locales

Wrap the "Zmeniť" button in `{#if $capabilities.features.fileDialogs}` and add an
`{:else}` branch rendering a debounced text input bound to `receiptsFolderPath`
(saved via the existing `saveReceiptSettingsNow`) with the
`receiptsFolderServerHint` hint. First real use of the `fileDialogs` capability.

**Verify:** `npm run check`; the field is a text input when the page is opened over
HTTP in server mode and a dialog button on desktop.

---

## Step 6 — Integration test for the env-pinned UI

**Files:** [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts),
`tests/integration/specs/env/env-managed-settings.spec.ts`,
[package.json](../../package.json)

The pinned state has to exist *before* the server process starts, so it needs its
own run rather than a case inside
[settings.spec.ts](../../tests/integration/specs/tier2/settings.spec.ts) — a suite
that pins `GEMINI_API_KEY` would break that spec's key-editing test.

1. In [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts): when
   `WDIO_ENV_PINNED=1`, `getSpecs()` returns only `./specs/env/**/*.spec.ts`, and
   `onPrepare` adds the fixture variables (`HA_URL`, `HA_API_TOKEN`,
   `PAPERLESS_URL`, `PAPERLESS_API_TOKEN`, `PAPERLESS_ENABLED`) to the spawned
   process env. Make the default (no-TIER) spec list enumerate the tier folders
   explicitly so `./specs/env/**` is never swept into a normal run. Skip the suite
   under `WDIO_EXTERNAL_SERVER=1`, where the container's env can't be set from here.
2. Spec assertions: inputs disabled, badges present and naming the right variable,
   the HA/Paperless status blocks still render, and typing into a pinned field
   produces no error toast.
3. `package.json`: `test:integration:server:env` script.

**Verify:** `npm run test:integration:server:env` passes; a normal
`npm run test:integration:server` run does **not** pick up the env specs.

---

## Step 7 — Documentation

- [settings-architecture.md](../../docs/features/settings-architecture.md) — describe
  the UI behavior for pinned fields and the new response fields.
- [server-mode.md](../../docs/features/server-mode.md) — replace "the Settings UI
  refuses to edit it and returns an explanatory error" with the disabled-and-badged
  behavior.
- [CHANGELOG.md](../../CHANGELOG.md) — user-visible entry under `[Unreleased]`.
- [DECISIONS.md](../../DECISIONS.md) — ADR for echoing env-pinned secrets back to the
  client (why pinned-only, why not variable-name-only).
- `docs/features/` — no new feature doc; this extends an existing one.

**Verify:** `/verify` (tests + git status + changelog check).

---

## Risks

- **Secret over the wire.** Mitigated by pinned-only exposure; recorded as an ADR.
- **`token_env_value` accidentally populated from the file.** Directly covered by
  `get_ha_settings_no_env_leaves_flags_false` and
  `get_paperless_settings_no_env_hides_token_value`.
- **Env spec leaking into normal runs.** The default spec list stops using a
  catch-all glob; Step 6's verification checks this explicitly.
