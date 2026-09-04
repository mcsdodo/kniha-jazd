# Feature: Settings Architecture

> Dual storage system separating deployment-specific configuration (LocalSettings) from business data (Settings), so credentials and server paths never travel inside the database.

## Overview

The application uses a **two-tier settings architecture** that separates:

1. **LocalSettings** — Deployment-specific configuration stored in a JSON file next to the database
2. **Settings** — Business/company data stored in the SQLite database

The split keeps API keys and server-side filesystem paths out of the database file, so the
database stays portable: it can be copied to another deployment, restored from a backup, or
handed to someone else without carrying a credential or a path that only made sense on the
machine it came from.

## The Two Storage Systems

### LocalSettings (File-based)

Stored as `local.settings.json` in the data directory, which is
`KNIHA_JAZD_DATA_DIR` and defaults to **`/data`** — so in the standard container it is
`/data/local.settings.json`, inside the mounted volume, alongside the database and
`backups/`. There is no per-OS app-data path and no separate dev identifier; both belonged
to the deleted desktop bundle ([ADR-030](../../DECISIONS.md)).

**Fields** — the full list lives on the `LocalSettings` struct in
[settings.rs](../../src-tauri/core/src/settings.rs); the commonly-edited ones are:

| Field | Type | Description |
|-------|------|-------------|
| `gemini_api_key` | `Option<String>` | API key for receipt OCR scanning |
| `receipts_folder_path` | `Option<String>` | Server-side folder path for receipt images |
| `theme` | `Option<String>` | UI theme: `"system"`, `"light"`, or `"dark"` |
| `date_prefill_mode` | `Option<DatePrefillMode>` | New-trip date prefill: `previous` or `today` |
| `infer_trip_times` | `Option<bool>` | Time-inference toggle (`None` = off) |
| `hidden_columns` | `Option<Vec<String>>` | Trip grid columns hidden by the user |
| `custom_db_path` | `Option<String>` | Custom database location |
| `backup_retention` | `Option<BackupRetention>` | Auto-cleanup settings for pre-update backups |
| `ha_url` / `ha_api_token` | `Option<String>` | Home Assistant integration |
| `paperless_url` / `paperless_api_token` / `paperless_enabled` | — | Paperless-ngx integration |
| `paperless_field_name_*` | `Option<String>` | Paperless custom field name overrides |

The struct also still carries `auto_check_updates`, `server_enabled` and `server_port`.
**All three are vestigial**: no command reads or writes them and no code branches on them.
They survive only so an inherited `local.settings.json` from a desktop install still
deserializes. Do not document them as behaviour.

**ReceiptSettings return shape:** the `ReceiptSettings` interface in
[types.ts](../../src/lib/types.ts).

**Notes**:
- The JSON keys are the Rust field names verbatim (snake_case) — `LocalSettings` declares no
  serde rename. `BackupRetention` is the exception: it *is* camelCase, so its nested keys are
  `enabled` and `keepCount`.
- Setting `gemini_api_key` or `receipts_folder_path` to an empty string clears the value.
- `receipts_folder_path` must exist and be a directory **on the server**, not on the
  machine running the browser.

**BackupRetention:** the struct in [settings.rs](../../src-tauri/core/src/settings.rs),
holding `enabled` (bool) and `keep_count` (u32), serialized camelCase.

### Environment-variable overrides (server deployments)

For server/Docker and headless deployments, secrets and integration endpoints can be supplied via environment variables instead of `local.settings.json`:

| Env var | Overrides field |
|---------|-----------------|
| `GEMINI_API_KEY` | `gemini_api_key` |
| `HA_URL` | `ha_url` |
| `HA_API_TOKEN` | `ha_api_token` |
| `PAPERLESS_URL` | `paperless_url` |
| `PAPERLESS_API_TOKEN` | `paperless_api_token` |
| `PAPERLESS_ENABLED` | `paperless_enabled` (truthy: `1`/`true`/`yes`, case-insensitive; any other non-empty value means false) |

**Precedence and behaviour:**

- A set, non-empty env variable wins over the value in `local.settings.json`. Empty or whitespace-only values are treated as unset.
- Env values are never persisted to disk — the JSON file is left untouched.
- When a field is pinned by an env variable, the corresponding setter command refuses the change with an explanatory error ("… is managed by the … environment variable"), so the Settings UI cannot silently diverge from the deployment configuration.
- Behaviour is unchanged when the variables are unset.
- Preferences (theme, hidden columns, date prefill, backup retention, Paperless custom field names, receipts folder) are not overridable — they remain file/UI-managed.

The variable names live in one place — the `env_vars` module in [settings.rs](../../src-tauri/core/src/settings.rs) — and are consumed by `apply_overrides`, the setter guards in [integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs), and the settings responses that ship the name to the UI.

**How the Settings page renders a pinned field** (see [ADR-025](../../DECISIONS.md)):

- The input is **disabled** and carries a badge with the variable's name, plus an "env-managed" hint. Marking is per-field, so pinning `HA_URL` while leaving `HA_API_TOKEN` in the file leaves the token editable.
- The page sends `null` for pinned fields when saving, so a pinned URL doesn't block a token edit.
- **Connection status still runs** — a fully env-configured integration shows its ✓/✗ indicator as usual.
- Disabling is UX only; the setter guards remain the enforcement boundary, since a browser client can call `/api/rpc` directly.

## Reading a secret back: PIN-gated reveal

No settings command returns a credential. `get_ha_settings` / `get_paperless_settings` report `hasToken`, and `get_receipt_settings` reports `hasGeminiApiKey` — the values themselves leave the backend only through `reveal_secret`, and only under the rules in [ADR-027](../../DECISIONS.md).

Every caller reaches `reveal_secret` through the HTTP dispatcher, so the PIN is always
required: the client must send the value of `KNIHA_JAZD_REVEAL_PIN` on **every** reveal —
no session, no caching.

- With `KNIHA_JAZD_REVEAL_PIN` unset, reveal is disabled on the server (the server still starts normally).
- Five consecutive wrong PINs lock reveal out for 60s, escalating to 5/15/60 minutes. The counter is global, not per-IP.
- The `field` argument is a closed enum (`geminiApiKey`, `haApiToken`, `paperlessApiToken`), so the command can't be aimed at other settings.
- **Why this exists:** the LAN/tailnet trust model in [ADR-017](../../DECISIONS.md) is enforced by a CORS allowlist, and CORS only constrains browsers — a direct HTTP client reaches every RPC command regardless.

Consequently the Gemini key field is **write-only** in the UI, like the HA and Paperless tokens: it shows `********` when a key is stored, and leaving it blank means "unchanged", not "clear it".

**Testing note:** WebdriverIO auto-loads the repo's `.env` file, so a developer with a real `PAPERLESS_API_TOKEN` there would pin that setting inside the app under test and make setter specs fail. [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) blanks the six variables before launching the server; the dedicated `npm run test:integration:docker:env` run re-applies fixture values on top to exercise the pinned UI ([env-managed-settings.spec.ts](../../tests/integration/specs/env/env-managed-settings.spec.ts)).

**Consumption vs. setter rule:** Code that *reads* configuration goes through `LocalSettings::load_effective()` in [settings.rs](../../src-tauri/core/src/settings.rs), which layers env overrides on top of the file. Setter commands use plain `load()` so they read and write only the on-disk file — combined with the env-pinned guard above, this keeps env values out of the persisted JSON.

### Settings (Database)

Stored in the `settings` table of `kniha-jazd.db`:

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique identifier |
| `company_name` | `String` | Company name printed in the HTML export header |
| `company_ico` | `String` | Company identification number (IČO) |
| `buffer_trip_purpose` | `String` | Default purpose text for buffer trips |
| `updated_at` | `DateTime<Utc>` | Last modification timestamp |

**Defaults:** the `Default` impl for `Settings` in
[models.rs](../../src-tauri/core/src/models.rs). `buffer_trip_purpose` defaults to
"sluzobna cesta" (service trip); other string fields default to empty.

## Why the Split?

The separation exists for **three key reasons**:

### 1. API Keys Don't Travel

API keys (like Gemini) are personal credentials that shouldn't be shared when syncing the database across computers. Each user/machine needs their own key.

### 2. Paths Are Deployment-Specific

File paths (like the receipts folder) belong to the machine running the server, not to the
database. A path baked into a shared database would be wrong for every other deployment
that opened it.

### 3. Preferences Are Not Business Data

Theme, hidden columns and date-prefill mode are UI preferences. They have no place in a
record that has to satisfy a tax audit.

> **Historical note:** the split was originally designed for one database shared between
> several desktop PCs over Google Drive or a NAS. That scenario is gone — one container owns
> one database file — but the split still earns its keep: it keeps credentials out of the
> file users copy around, and keeps the DB portable between deployments.

## Technical Implementation

### Loading Settings

**LocalSettings loading:** `LocalSettings::load()` in
[settings.rs](../../src-tauri/core/src/settings.rs) reads `local.settings.json` from the
data directory, falling back to defaults if the file is missing or malformed.
`LocalSettings::load_effective()` layers env overrides on top — see the consumption-vs-setter
rule above.

**Settings loading:** `Database::get_settings()` in
[db.rs](../../src-tauri/core/src/db.rs) queries the `settings` table and converts the row to
a domain model using Diesel.

### Saving Settings

**LocalSettings saving:** `LocalSettings::save()` writes pretty-printed JSON with
`sync_all()`, so the data is flushed to disk before the call returns.

**Settings saving:** `Database::save_settings()` uses an upsert pattern — checks whether a
settings row exists, then updates or inserts.

Write commands fail in read-only mode with a user-facing error — see [read-only-mode.md](./read-only-mode.md).

### Frontend Integration

The Settings UI ([settings/+page.svelte](../../src/routes/settings/+page.svelte)) loads both
setting types and presents them in a unified interface. Its `onMount()` subscribes to the
locale and theme stores, then sequentially awaits `getSettings()`, `loadBackups()`,
`loadRetentionSettings()`, `checkVehiclesWithTrips()`, `getAppVersion()`,
`getInferTripTimes()` and `getReceiptSettings()`.

It does **not** fetch the database location — `getDbLocation()` has no caller in the
frontend. The `get_db_location` command still exists on the backend and is reachable over
RPC, but nothing in the UI displays it, and there is no "Change location" flow: moving the
database is now an operator action on the host volume.

**Auto-save with debouncing:** a local `debounce()` helper wraps `saveCompanySettingsNow`,
`saveReceiptSettingsNow`, `saveHaSettingsNow` and `savePaperlessSettingsNow`, all at 800ms,
to prevent excessive writes while typing.

## RPC Commands

### LocalSettings Commands

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `get_theme_preference` | — | `String` | Get theme ("system", "light", "dark") |
| `set_theme_preference` | `theme` | `()` | Set theme preference |
| `get_date_prefill_mode` | — | `DatePrefillMode` | Get new-trip date prefill mode |
| `set_date_prefill_mode` | `mode` | `()` | Set new-trip date prefill mode |
| `get_hidden_columns` | — | `Vec<String>` | Get hidden trip grid columns |
| `set_hidden_columns` | `columns` | `()` | Set hidden trip grid columns |
| `get_infer_trip_times` | — | `bool` | Get the time-inference toggle |
| `set_infer_trip_times` | `enabled` | `()` | Set the time-inference toggle |
| `get_receipt_settings` | — | `ReceiptSettings` | Get folder path plus "is a key configured / is it env-pinned" flags |
| `set_gemini_api_key` | `key` | `()` | Set Gemini API key |
| `set_receipts_folder_path` | `path` | `()` | Set receipts folder |
| `get_backup_retention` | — | `BackupRetention?` | Get cleanup settings |
| `set_backup_retention` | `retention` | `()` | Set cleanup settings |
| `reveal_secret` | `field`, `pin` | `String` | PIN-gated read of one credential |
| `get_db_location` | — | `DbLocationInfo` | Database path, custom-path flag, backups path — **no frontend caller** |
| `get_app_mode` | — | `AppModeInfo` | Read-only state (see [read-only-mode.md](./read-only-mode.md)) |
| `get_app_version` | — | `String` | The `CARGO_PKG_VERSION` of the running binary |

There are no `get_auto_check_updates` / `set_auto_check_updates` commands — they went with
the updater. Home Assistant and Paperless settings have their own commands
(`get_ha_settings`, `save_ha_settings`, `get_paperless_settings`, `save_paperless_settings`).

### Database Settings Commands

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `get_settings` | — | `Settings?` | Load company settings |
| `save_settings` | `name`, `ico`, `purpose` | `Settings` | Save company settings |

## Key Files

| File | Purpose |
|------|---------|
| [settings.rs](../../src-tauri/core/src/settings.rs) | `LocalSettings` struct, load/save methods, env overrides, `BackupRetention` |
| [models.rs](../../src-tauri/core/src/models.rs) | `Settings` struct definition with defaults |
| [commands_internal/settings_cmd.rs](../../src-tauri/core/src/commands_internal/settings_cmd.rs) | Settings commands for both setting types |
| [commands_internal/reveal.rs](../../src-tauri/core/src/commands_internal/reveal.rs) | PIN-gated `reveal_secret` |
| [db.rs](../../src-tauri/core/src/db.rs) | Database operations for `Settings` |
| [+page.svelte](../../src/routes/settings/+page.svelte) | Unified settings UI |
| [api.ts](../../src/lib/api.ts) | TypeScript API wrappers |
| [types.ts](../../src/lib/types.ts) | TypeScript interfaces (`Settings`, `ReceiptSettings`) |

## Design Decisions

### Why JSON for LocalSettings?

1. **Survives upgrades** — it lives in the mounted `/data` volume, so pulling a new image
   leaves it untouched
2. **Human-readable** — an operator with shell access to the volume can edit it directly
3. **No migration needed** — new fields with `Option<T>` are backward compatible, which is
   also why fields deleted from the UI can stay in the struct without breaking old files

### Why Database for Settings?

1. **Travels with data** — Company info is tied to the vehicle/trip data
2. **Consistent** — Same ACID guarantees as other business data
3. **Single source** — No sync conflicts between files

### Unified UI Despite Split Storage

The user sees one Settings page, unaware of the underlying split. This provides:
- Simple mental model for users
- All settings in one place
- Transparent save/load behavior

### Sample `local.settings.json`

```json
{
    "gemini_api_key": "YOUR_API_KEY_HERE",
    "receipts_folder_path": "/data/receipts",
    "theme": "dark",
    "date_prefill_mode": "previous",
    "hidden_columns": ["time", "fuelConsumed"],
    "ha_url": "http://homeassistant.local:8123",
    "ha_api_token": "eyJhbGciOiJIUzI1NiIs...",
    "backup_retention": {
        "enabled": true,
        "keepCount": 3
    }
}
```

Note the casing: top-level keys are snake_case (the Rust field names), while
`backup_retention`'s own keys are camelCase because that struct declares
`#[serde(rename_all = "camelCase")]`.
