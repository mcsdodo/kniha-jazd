**Date:** 2026-05-04
**Subject:** Configurable Paperless Field Names — Design
**Status:** Approved

# Design: Configurable Paperless Custom Field Names

See [01-task.md](./01-task.md) for problem statement.

## Approach

A small, contained change. Replace three hardcoded strings in [paperless.rs](../../src-tauri/core/src/paperless.rs):72-76 with values pulled from settings. Defaults preserve current behavior.

## Settings additions

In [settings.rs](../../src-tauri/core/src/settings.rs), three new optional fields on `LocalSettings`:

```rust
pub struct LocalSettings {
    // ... existing fields ...

    // Paperless-ngx integration
    pub paperless_url: Option<String>,
    pub paperless_api_token: Option<String>,
    pub paperless_enabled: Option<bool>,
    // NEW — custom field names (None → use default)
    pub paperless_field_name_datetime: Option<String>,
    pub paperless_field_name_liters: Option<String>,
    pub paperless_field_name_total: Option<String>,

    // ... rest ...
}
```

`Option<String>` mirrors how url/token are stored — `None` means "use default", non-empty Some means "use this value".

## New helper struct

In [paperless.rs](../../src-tauri/core/src/paperless.rs), introduce a small struct that captures the three field names:

```rust
#[derive(Debug, Clone)]
pub struct PaperlessFieldNames {
    pub datetime: String,
    pub liters: String,
    pub total: String,
}

impl Default for PaperlessFieldNames {
    fn default() -> Self {
        Self {
            datetime: "receipt_datetime".to_string(),
            liters: "litres".to_string(),
            total: "total_amount".to_string(),
        }
    }
}

impl PaperlessFieldNames {
    /// Build from a LocalSettings — empty/missing values fall back to defaults.
    pub fn from_settings(s: &LocalSettings) -> Self {
        let d = Self::default();
        Self {
            datetime: s.paperless_field_name_datetime.clone()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or(d.datetime),
            liters: s.paperless_field_name_liters.clone()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or(d.liters),
            total: s.paperless_field_name_total.clone()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or(d.total),
        }
    }
}
```

## Modified `resolve_field_map`

The existing function in [paperless.rs](../../src-tauri/core/src/paperless.rs):60-77 grows one parameter:

```rust
pub async fn resolve_field_map(
    &self,
    names: &PaperlessFieldNames,
) -> Result<PaperlessFieldMap, PaperlessError> {
    // ... HTTP fetch unchanged ...

    let find = |n: &str| body.results.iter().find(|f| f.name == n).map(|f| f.id)
        .ok_or_else(|| PaperlessError::CustomFieldNotFound(n.to_string()));

    Ok(PaperlessFieldMap {
        total_amount_id:    find(&names.total)?,
        litres_id:          find(&names.liters)?,
        receipt_datetime_id: find(&names.datetime)?,
    })
}
```

Callers updated to pass `&PaperlessFieldNames::from_settings(&settings)`.

## Backend response surface

Extend `PaperlessSettingsResponse` in [integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs):181-198 so the frontend can render current values:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperlessSettingsResponse {
    pub url: Option<String>,
    pub has_token: bool,
    pub enabled: bool,
    // NEW — resolved values (defaults applied)
    pub field_name_datetime: String,
    pub field_name_liters: String,
    pub field_name_total: String,
}

pub fn get_paperless_settings_internal(app_dir: &Path) -> Result<PaperlessSettingsResponse, String> {
    let settings = LocalSettings::load(app_dir);
    let names = PaperlessFieldNames::from_settings(&settings);
    Ok(PaperlessSettingsResponse {
        url: settings.paperless_url,
        has_token: settings.paperless_api_token.as_deref().is_some_and(|t| !t.trim().is_empty()),
        enabled: settings.paperless_enabled.unwrap_or(true),
        field_name_datetime: names.datetime,
        field_name_liters: names.liters,
        field_name_total: names.total,
    })
}
```

The frontend always receives valid (non-empty) values. The "empty → default" logic is centralized in `from_settings`, never duplicated in TS.

## Save command

Extend `save_paperless_settings_internal` in [integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs):200-231 with three optional params, following the same `Option<String>` pattern as url/token (None = keep existing, empty string = clear/use default, value = set):

```rust
pub fn save_paperless_settings_internal(
    app_dir: &Path,
    app_state: &AppState,
    url: Option<String>,
    token: Option<String>,
    enabled: Option<bool>,
    field_name_datetime: Option<String>,
    field_name_liters: Option<String>,
    field_name_total: Option<String>,
) -> Result<(), String> {
    check_read_only!(app_state);
    // ... existing url/token validation ...

    let mut settings = LocalSettings::load(app_dir);
    // ... existing url/token/enabled writes ...

    if let Some(v) = field_name_datetime {
        let v = v.trim().to_string();
        settings.paperless_field_name_datetime = if v.is_empty() { None } else { Some(v) };
    }
    if let Some(v) = field_name_liters {
        let v = v.trim().to_string();
        settings.paperless_field_name_liters = if v.is_empty() { None } else { Some(v) };
    }
    if let Some(v) = field_name_total {
        let v = v.trim().to_string();
        settings.paperless_field_name_total = if v.is_empty() { None } else { Some(v) };
    }

    settings.save(app_dir).map_err(|e| e.to_string())
}
```

## Frontend

### TS type

In [types.ts](../../src/lib/types.ts), extend `PaperlessSettings`:

```ts
export interface PaperlessSettings {
    url: string | null;
    hasToken: boolean;
    enabled: boolean;
    fieldNameDatetime: string;   // NEW — always populated (default if not set)
    fieldNameLiters: string;     // NEW
    fieldNameTotal: string;      // NEW
}
```

### API call

In [api.ts](../../src/lib/api.ts), `savePaperlessSettings` gains three optional params (mirrors url/token pattern):

```ts
export async function savePaperlessSettings(
    url: string | null,
    token: string | null,
    enabled: boolean | null,
    fieldNameDatetime: string | null,
    fieldNameLiters: string | null,
    fieldNameTotal: string | null,
): Promise<void>
```

### Settings UI

In [settings/+page.svelte](../../src/routes/settings/+page.svelte), three text inputs in the Paperless section, shown only when the URL+token are configured (consistent with other Paperless-only inputs):

```svelte
<label>
    {$LL.settings.paperless.fieldNameDatetime()}
    <input type="text" bind:value={fieldNameDatetime} placeholder="receipt_datetime" />
</label>
<label>
    {$LL.settings.paperless.fieldNameLiters()}
    <input type="text" bind:value={fieldNameLiters} placeholder="litres" />
</label>
<label>
    {$LL.settings.paperless.fieldNameTotal()}
    <input type="text" bind:value={fieldNameTotal} placeholder="total_amount" />
</label>
```

The placeholder shows the default; empty input means "use default".

### i18n keys

In [sk/index.ts](../../src/lib/i18n/sk/index.ts) and [en/index.ts](../../src/lib/i18n/en/index.ts), add to the `settings.paperless` namespace:

| Key | SK | EN |
|-----|----|----|
| `fieldNameDatetime` | "Názov vlastného poľa pre dátum/čas" | "Custom field name for datetime" |
| `fieldNameLiters` | "Názov vlastného poľa pre litre" | "Custom field name for liters" |
| `fieldNameTotal` | "Názov vlastného poľa pre sumu" | "Custom field name for total amount" |
| `fieldNameHint` | "Necháj prázdne pre predvolenú hodnotu" | "Leave empty to use default" |

## Test plan

### Backend

In [paperless_tests.rs](../../src-tauri/core/src/paperless_tests.rs):

- `test_resolve_field_map_uses_provided_names` — pass non-default `PaperlessFieldNames`, verify lookup uses them
- `test_field_names_default_values` — verify `PaperlessFieldNames::default()` returns the legacy strings
- `test_field_names_from_settings_falls_back_to_default_when_empty` — empty/None settings → defaults
- `test_field_names_from_settings_uses_custom_when_set` — custom values override defaults

In [integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs):

- `test_save_paperless_settings_persists_field_names`
- `test_get_paperless_settings_returns_resolved_field_names_with_defaults`
- `test_get_paperless_settings_returns_custom_field_names_when_set`

### Integration

Update [paperless-integration.spec.ts](../../tests/integration/specs/tier2/paperless-integration.spec.ts):

- Save custom field names → verify saved settings reflect them
- Mock Paperless server returns docs with custom-named fields → integration loads invoices correctly

## Out of Scope

- Validating that the field names exist on the user's Paperless server before saving (would require a roundtrip test which adds complexity; the existing "test connection" button doesn't validate field names today either — failure surfaces at first sync)
- Auto-detecting field names from Paperless API (could be a future feature; for now we trust user input)
- Migration of existing users — they keep working because defaults match the previously-hardcoded strings exactly
