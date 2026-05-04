# Configurable Paperless Field Names — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use the `superpowers:executing-plans` skill (invoke via the `Skill` tool) to implement this plan task-by-task.

**Goal:** Let users configure the three Paperless custom field names (datetime, liters, total) via Settings UI, with current strings as defaults. Existing users see no behavior change.

**Architecture:** Add `PaperlessFieldNames` helper struct (with sensible defaults) in [paperless.rs](../../src-tauri/core/src/paperless.rs). Pass it into `resolve_field_map`. Persist user values via three new `Option<String>` fields on `LocalSettings`, plumbed through the existing `get_paperless_settings` / `save_paperless_settings` commands. Frontend gets three text inputs with default-value placeholders.

**Tech Stack:** Rust (Tauri backend) · TypeScript (SvelteKit frontend) · Slovak/English i18n via `typesafe-i18n` · `wiremock` for HTTP-mock tests · WebdriverIO for integration.

---

## Reference reading before starting

- Design: [02-design.md](./02-design.md)
- Task descriptor: [01-task.md](./01-task.md)
- Current hardcoded strings: [paperless.rs](../../src-tauri/core/src/paperless.rs):60-77
- Settings struct: [settings.rs](../../src-tauri/core/src/settings.rs):41-43
- Existing get/save commands: [integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs):181-231
- Existing TS shape: [types.ts](../../src/lib/types.ts):410-414
- Existing UI section: [+page.svelte](../../src/routes/settings/+page.svelte):1095-1185

---

### Task 1: Add `PaperlessFieldNames` struct + Default impl

**Files:**
- Modify: [src-tauri/core/src/paperless.rs](../../src-tauri/core/src/paperless.rs)
- Test: [src-tauri/core/src/paperless_tests.rs](../../src-tauri/core/src/paperless_tests.rs)

**Step 1: Write failing tests**

Append at the top of [paperless_tests.rs](../../src-tauri/core/src/paperless_tests.rs):

```rust
#[test]
fn paperless_field_names_default_uses_legacy_strings() {
    let n = PaperlessFieldNames::default();
    assert_eq!(n.datetime, "receipt_datetime");
    assert_eq!(n.liters, "litres");
    assert_eq!(n.total, "total_amount");
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test paperless_field_names_default --lib`
Expected: FAIL — `PaperlessFieldNames` not found.

**Step 3: Write minimal implementation**

In [paperless.rs](../../src-tauri/core/src/paperless.rs), insert after the existing `PaperlessFieldMap` struct (around line 31):

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
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test paperless_field_names_default --lib`
Expected: PASS.

**Step 5: Commit**

```bash
git add src-tauri/core/src/paperless.rs src-tauri/core/src/paperless_tests.rs
git commit -m "feat(paperless): add PaperlessFieldNames struct with legacy defaults"
```

---

### Task 2: Add three `Option<String>` fields to `LocalSettings`

**Files:**
- Modify: [src-tauri/core/src/settings.rs](../../src-tauri/core/src/settings.rs):41-43

**Step 1: Run existing settings tests as baseline**

Run: `cd src-tauri && cargo test --lib settings`
Expected: All current settings tests PASS.

**Step 2: Extend struct**

In [settings.rs](../../src-tauri/core/src/settings.rs), under the existing `// Paperless-ngx integration` block (around line 40):

```rust
    // Paperless-ngx integration
    pub paperless_url: Option<String>,
    pub paperless_api_token: Option<String>,
    pub paperless_enabled: Option<bool>,
    // Custom field name overrides — None means "use default"
    pub paperless_field_name_datetime: Option<String>,
    pub paperless_field_name_liters: Option<String>,
    pub paperless_field_name_total: Option<String>,
```

**Step 3: Run all backend tests**

Run: `cd src-tauri && cargo test --lib`
Expected: All PASS (`Option<String>` defaults to `None` via `#[derive(Default)]`, no test fallout).

**Step 4: Commit**

```bash
git add src-tauri/core/src/settings.rs
git commit -m "feat(paperless): add field name overrides to LocalSettings"
```

---

### Task 3: Implement `PaperlessFieldNames::from_settings`

**Files:**
- Modify: [src-tauri/core/src/paperless.rs](../../src-tauri/core/src/paperless.rs)
- Test: [src-tauri/core/src/paperless_tests.rs](../../src-tauri/core/src/paperless_tests.rs)

**Step 1: Write failing tests**

Append to [paperless_tests.rs](../../src-tauri/core/src/paperless_tests.rs):

```rust
#[test]
fn paperless_field_names_from_settings_uses_defaults_when_none() {
    let s = crate::settings::LocalSettings::default();
    let n = PaperlessFieldNames::from_settings(&s);
    assert_eq!(n.datetime, "receipt_datetime");
    assert_eq!(n.liters, "litres");
    assert_eq!(n.total, "total_amount");
}

#[test]
fn paperless_field_names_from_settings_uses_defaults_when_empty_strings() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_field_name_datetime = Some("".to_string());
    s.paperless_field_name_liters = Some("   ".to_string());
    s.paperless_field_name_total = Some("\t".to_string());
    let n = PaperlessFieldNames::from_settings(&s);
    assert_eq!(n.datetime, "receipt_datetime");
    assert_eq!(n.liters, "litres");
    assert_eq!(n.total, "total_amount");
}

#[test]
fn paperless_field_names_from_settings_uses_custom_when_set() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_field_name_datetime = Some("Dátum dokladu".to_string());
    s.paperless_field_name_liters = Some("Litre".to_string());
    s.paperless_field_name_total = Some("Suma".to_string());
    let n = PaperlessFieldNames::from_settings(&s);
    assert_eq!(n.datetime, "Dátum dokladu");
    assert_eq!(n.liters, "Litre");
    assert_eq!(n.total, "Suma");
}
```

**Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test paperless_field_names_from_settings --lib`
Expected: FAIL — `from_settings` not implemented.

**Step 3: Write the implementation**

In [paperless.rs](../../src-tauri/core/src/paperless.rs), add an `impl PaperlessFieldNames` block after the `Default` impl from Task 1:

```rust
impl PaperlessFieldNames {
    /// Resolve from LocalSettings: empty/whitespace/None → fall back to default.
    pub fn from_settings(s: &crate::settings::LocalSettings) -> Self {
        let d = Self::default();
        let pick = |opt: &Option<String>, default: String| -> String {
            opt.as_ref()
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .map(|v| v.to_string())
                .unwrap_or(default)
        };
        Self {
            datetime: pick(&s.paperless_field_name_datetime, d.datetime),
            liters:   pick(&s.paperless_field_name_liters,   d.liters),
            total:    pick(&s.paperless_field_name_total,    d.total),
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test paperless_field_names --lib`
Expected: PASS (all 4 tests).

**Step 5: Commit**

```bash
git add src-tauri/core/src/paperless.rs src-tauri/core/src/paperless_tests.rs
git commit -m "feat(paperless): resolve PaperlessFieldNames from LocalSettings"
```

---

### Task 4: Refactor `resolve_field_map` to accept `&PaperlessFieldNames`

**Files:**
- Modify: [src-tauri/core/src/paperless.rs](../../src-tauri/core/src/paperless.rs):60-77
- Modify: [src-tauri/core/src/paperless_tests.rs](../../src-tauri/core/src/paperless_tests.rs):49, 65
- Modify: [src-tauri/core/src/commands_internal/paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs):51

**Step 1: Write failing test for custom names**

Append to [paperless_tests.rs](../../src-tauri/core/src/paperless_tests.rs):

```rust
#[tokio::test]
async fn resolve_field_map_uses_custom_names_when_provided() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/custom_fields/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"id": 11, "name": "Suma",          "data_type": "float"},
                {"id": 12, "name": "Litre",         "data_type": "float"},
                {"id": 13, "name": "Dátum dokladu", "data_type": "string"},
            ]
        })))
        .mount(&mock).await;

    let client = PaperlessClient::new(mock.uri(), "tok".into());
    let names = PaperlessFieldNames {
        datetime: "Dátum dokladu".into(),
        liters:   "Litre".into(),
        total:    "Suma".into(),
    };
    let map = client.resolve_field_map(&names).await.unwrap();
    assert_eq!(map.total_amount_id, 11);
    assert_eq!(map.litres_id, 12);
    assert_eq!(map.receipt_datetime_id, 13);
}
```

**Step 2: Run to verify fail**

Run: `cd src-tauri && cargo test resolve_field_map_uses_custom_names --lib`
Expected: FAIL (compile error — `resolve_field_map` takes no args).

**Step 3: Modify `resolve_field_map` signature**

In [paperless.rs](../../src-tauri/core/src/paperless.rs):60-77, change to:

```rust
    pub async fn resolve_field_map(
        &self,
        names: &PaperlessFieldNames,
    ) -> Result<PaperlessFieldMap, PaperlessError> {
        #[derive(Deserialize)] struct Field { id: i64, name: String }
        #[derive(Deserialize)] struct Resp { results: Vec<Field> }

        let url = format!("{}/api/custom_fields/?page_size=200", self.base_url);
        let resp = self.http.get(&url).header("Authorization", self.auth()).send().await?;
        if !resp.status().is_success() { return Err(PaperlessError::Http(resp.status().as_u16())); }
        let body: Resp = resp.json().await.map_err(|e| PaperlessError::Parse(e.to_string()))?;

        let find = |n: &str| body.results.iter().find(|f| f.name == n).map(|f| f.id)
            .ok_or_else(|| PaperlessError::CustomFieldNotFound(n.to_string()));

        Ok(PaperlessFieldMap {
            total_amount_id:     find(&names.total)?,
            litres_id:           find(&names.liters)?,
            receipt_datetime_id: find(&names.datetime)?,
        })
    }
```

**Step 4: Update existing test calls to pass defaults**

In [paperless_tests.rs](../../src-tauri/core/src/paperless_tests.rs):

- Line 49: `let map = client.resolve_field_map().await.unwrap();` → `let map = client.resolve_field_map(&PaperlessFieldNames::default()).await.unwrap();`
- Line 65: same pattern, replace `client.resolve_field_map().await.unwrap_err()` with `client.resolve_field_map(&PaperlessFieldNames::default()).await.unwrap_err()`

**Step 5: Update production caller**

In [paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs):51:

```rust
// Before:
let fmap = client.resolve_field_map().await?;

// After (caller must have a `LocalSettings` already in scope; if not, load it):
let fmap = client.resolve_field_map(&PaperlessFieldNames::from_settings(&settings)).await?;
```

If `settings` is not in scope, add `let settings = crate::settings::LocalSettings::load(app_dir);` before this line. Check the function signature in [paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs) for what `app_dir` is named in this context — likely passed in via the function args.

Add import at top of [paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs):

```rust
use crate::paperless::PaperlessFieldNames;
```

**Step 6: Run all paperless tests**

Run: `cd src-tauri && cargo test paperless --lib`
Expected: All PASS, including the new `resolve_field_map_uses_custom_names_when_provided`.

**Step 7: Run full backend test suite**

Run: `cd src-tauri && cargo test --lib`
Expected: All PASS.

**Step 8: Commit**

```bash
git add src-tauri/core/src/paperless.rs src-tauri/core/src/paperless_tests.rs src-tauri/core/src/commands_internal/paperless_cmd.rs
git commit -m "refactor(paperless): resolve_field_map takes PaperlessFieldNames param"
```

---

### Task 5: Extend `PaperlessSettingsResponse` and `get_paperless_settings_internal`

**Files:**
- Modify: [src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs):181-198
- Test: [src-tauri/core/src/commands_internal/integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs)

**Step 1: Write failing tests**

Append to [integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs):

```rust
#[test]
fn get_paperless_settings_returns_default_field_names_when_unset() {
    let dir = tempfile::tempdir().unwrap();
    let r = get_paperless_settings_internal(&dir.path().to_path_buf()).unwrap();
    assert_eq!(r.field_name_datetime, "receipt_datetime");
    assert_eq!(r.field_name_liters, "litres");
    assert_eq!(r.field_name_total, "total_amount");
}

#[test]
fn get_paperless_settings_returns_custom_field_names_when_set() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = LocalSettings::default();
    s.paperless_field_name_datetime = Some("Dátum dokladu".to_string());
    s.paperless_field_name_liters = Some("Litre".to_string());
    s.paperless_field_name_total = Some("Suma".to_string());
    s.save(dir.path()).unwrap();

    let r = get_paperless_settings_internal(&dir.path().to_path_buf()).unwrap();
    assert_eq!(r.field_name_datetime, "Dátum dokladu");
    assert_eq!(r.field_name_liters, "Litre");
    assert_eq!(r.field_name_total, "Suma");
}
```

**Step 2: Run to verify fail**

Run: `cd src-tauri && cargo test get_paperless_settings_returns --lib`
Expected: FAIL — fields don't exist on `PaperlessSettingsResponse`.

**Step 3: Extend `PaperlessSettingsResponse`**

In [integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs):181-198:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperlessSettingsResponse {
    pub url: Option<String>,
    pub has_token: bool,
    pub enabled: bool,
    // NEW — resolved values (defaults applied when settings are None/empty)
    pub field_name_datetime: String,
    pub field_name_liters: String,
    pub field_name_total: String,
}

pub fn get_paperless_settings_internal(app_dir: &Path) -> Result<PaperlessSettingsResponse, String> {
    use crate::paperless::PaperlessFieldNames;

    let settings = LocalSettings::load(app_dir);
    let names = PaperlessFieldNames::from_settings(&settings);
    let enabled = settings.paperless_enabled.unwrap_or(true);
    Ok(PaperlessSettingsResponse {
        url: settings.paperless_url,
        has_token: settings
            .paperless_api_token
            .as_deref()
            .is_some_and(|t| !t.trim().is_empty()),
        enabled,
        field_name_datetime: names.datetime,
        field_name_liters: names.liters,
        field_name_total: names.total,
    })
}
```

**Step 4: Run tests to verify pass**

Run: `cd src-tauri && cargo test get_paperless_settings_returns --lib`
Expected: PASS.

**Step 5: Run all integrations tests**

Run: `cd src-tauri && cargo test --lib integrations_tests`
Expected: All PASS (existing tests unaffected — they don't assert on the new fields).

**Step 6: Commit**

```bash
git add src-tauri/core/src/commands_internal/integrations.rs src-tauri/core/src/commands_internal/integrations_tests.rs
git commit -m "feat(paperless): expose resolved field names via get_paperless_settings"
```

---

### Task 6: Extend `save_paperless_settings_internal` with three new params

**Files:**
- Modify: [src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs):200-231
- Test: [src-tauri/core/src/commands_internal/integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs)

**Step 1: Write failing tests**

Append to [integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs):

```rust
#[test]
fn save_paperless_settings_persists_custom_field_names() {
    let dir = tempfile::tempdir().unwrap();
    let app_state = AppState::default();

    save_paperless_settings_internal(
        &dir.path().to_path_buf(),
        &app_state,
        Some("https://paperless.example.com".to_string()),
        Some("token123".to_string()),
        Some(true),
        Some("Dátum dokladu".to_string()),
        Some("Litre".to_string()),
        Some("Suma".to_string()),
    ).unwrap();

    let loaded = LocalSettings::load(dir.path());
    assert_eq!(loaded.paperless_field_name_datetime.as_deref(), Some("Dátum dokladu"));
    assert_eq!(loaded.paperless_field_name_liters.as_deref(), Some("Litre"));
    assert_eq!(loaded.paperless_field_name_total.as_deref(), Some("Suma"));
}

#[test]
fn save_paperless_settings_empty_field_name_clears_to_use_default() {
    let dir = tempfile::tempdir().unwrap();
    let app_state = AppState::default();

    // First save custom values
    save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        None, None, None,
        Some("custom_dt".to_string()),
        Some("custom_lt".to_string()),
        Some("custom_tt".to_string()),
    ).unwrap();

    // Then clear with empty strings
    save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        None, None, None,
        Some("".to_string()),
        Some("".to_string()),
        Some("".to_string()),
    ).unwrap();

    let loaded = LocalSettings::load(dir.path());
    assert_eq!(loaded.paperless_field_name_datetime, None);
    assert_eq!(loaded.paperless_field_name_liters, None);
    assert_eq!(loaded.paperless_field_name_total, None);
}

#[test]
fn save_paperless_settings_none_field_name_keeps_existing() {
    let dir = tempfile::tempdir().unwrap();
    let app_state = AppState::default();

    save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        None, None, None,
        Some("existing_dt".to_string()),
        Some("existing_lt".to_string()),
        Some("existing_tt".to_string()),
    ).unwrap();

    // Update only enabled, leave field names as None
    save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        None, None, Some(false),
        None, None, None,
    ).unwrap();

    let loaded = LocalSettings::load(dir.path());
    assert_eq!(loaded.paperless_field_name_datetime.as_deref(), Some("existing_dt"));
    assert_eq!(loaded.paperless_field_name_liters.as_deref(), Some("existing_lt"));
    assert_eq!(loaded.paperless_field_name_total.as_deref(), Some("existing_tt"));
    assert_eq!(loaded.paperless_enabled, Some(false));
}
```

**Step 2: Run to verify fail**

Run: `cd src-tauri && cargo test save_paperless_settings_persists_custom --lib`
Expected: FAIL (compile error — too many args).

**Step 3: Extend `save_paperless_settings_internal`**

In [integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs):200-231 — change signature and append new field handling:

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
    if let Some(ref url_str) = url {
        if !url_str.is_empty() {
            if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
                return Err("URL must start with http:// or https://".to_string());
            }
            if url::Url::parse(url_str).is_err() {
                return Err("Invalid URL format".to_string());
            }
        }
    }
    let mut settings = LocalSettings::load(app_dir);
    if let Some(u) = url {
        let u = u.trim().to_string();
        settings.paperless_url = if u.is_empty() { None } else { Some(u) };
    }
    if let Some(t) = token {
        let t = t.trim().to_string();
        settings.paperless_api_token = if t.is_empty() { None } else { Some(t) };
    }
    if let Some(e) = enabled {
        settings.paperless_enabled = Some(e);
    }
    // Field name overrides — empty string clears (back to default), None keeps existing
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

**Step 4: Run tests**

Run: `cd src-tauri && cargo test save_paperless_settings --lib`
Expected: PASS — including the 3 new tests.

The pre-existing `save_paperless_settings_*` tests will fail to compile — they pass too few args. Update each call site with three trailing `None` to preserve their semantics. Use `cargo test` output to find them.

**Step 5: Run all backend tests**

Run: `cd src-tauri && cargo test --lib`
Expected: All PASS.

**Step 6: Commit**

```bash
git add src-tauri/core/src/commands_internal/integrations.rs src-tauri/core/src/commands_internal/integrations_tests.rs
git commit -m "feat(paperless): save_paperless_settings accepts field name overrides"
```

---

### Task 7: Update Tauri desktop wrapper

**Files:**
- Modify: [src-tauri/desktop/src/commands/integrations.rs](../../src-tauri/desktop/src/commands/integrations.rs):136-144

**Step 1: Inspect current wrapper**

Read [integrations.rs](../../src-tauri/desktop/src/commands/integrations.rs):136-144 — note that it forwards `url`, `token`, `enabled` to `save_paperless_settings_internal`.

**Step 2: Modify wrapper signature**

```rust
#[tauri::command]
pub fn save_paperless_settings(
    app: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
    url: Option<String>,
    token: Option<String>,
    enabled: Option<bool>,
    field_name_datetime: Option<String>,
    field_name_liters: Option<String>,
    field_name_total: Option<String>,
) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    inner::save_paperless_settings_internal(
        &app_data_dir,
        &app_state,
        url,
        token,
        enabled,
        field_name_datetime,
        field_name_liters,
        field_name_total,
    )
}
```

(Adapt to the actual signature pattern in the file — match imports and `AppState` import path.)

**Step 3: Compile**

Run: `cd src-tauri && cargo build`
Expected: SUCCESS.

**Step 4: Commit**

```bash
git add src-tauri/desktop/src/commands/integrations.rs
git commit -m "feat(paperless): plumb field name overrides through Tauri wrapper"
```

---

### Task 8: Update server dispatcher

**Files:**
- Modify: [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs):808-825

**Step 1: Inspect current dispatcher arm**

Read [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs):808-825 — note the inline `Args` struct deserializes camelCase JSON.

**Step 2: Extend `Args` and forward all fields**

In [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs):808-825:

```rust
"save_paperless_settings" => {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        url: Option<String>,
        token: Option<String>,
        enabled: Option<bool>,
        field_name_datetime: Option<String>,
        field_name_liters: Option<String>,
        field_name_total: Option<String>,
    }
    let a: Args = parse_args(args)?;
    crate::commands_internal::integrations::save_paperless_settings_internal(
        &state.app_dir,
        &state.app_state,
        a.url,
        a.token,
        a.enabled,
        a.field_name_datetime,
        a.field_name_liters,
        a.field_name_total,
    )?;
    Ok(serde_json::to_value(()).unwrap())
}
```

**Step 3: Compile**

Run: `cd src-tauri && cargo build`
Expected: SUCCESS.

**Step 4: Compile-time tests as smoke check**

Run: `cd src-tauri && cargo test --lib`
Expected: All PASS.

**Step 5: Commit**

```bash
git add src-tauri/core/src/server/dispatcher.rs
git commit -m "feat(paperless): server dispatcher forwards field name args"
```

---

### Task 9: Frontend — extend `PaperlessSettings` TS interface

**Files:**
- Modify: [src/lib/types.ts](../../src/lib/types.ts):410-414

**Step 1: Modify type**

In [types.ts](../../src/lib/types.ts):410-414:

```ts
export interface PaperlessSettings {
    url: string | null;
    hasToken: boolean;
    enabled: boolean;
    // NEW — always populated server-side (defaults applied)
    fieldNameDatetime: string;
    fieldNameLiters: string;
    fieldNameTotal: string;
}
```

**Step 2: Type check**

Run: `npm run check` (or `npx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json`)
Expected: Errors flagged in callers that read `PaperlessSettings` (this is fine — they'll be fixed in next tasks).

**Step 3: Commit**

```bash
git add src/lib/types.ts
git commit -m "feat(paperless): add field name fields to PaperlessSettings type"
```

---

### Task 10: Frontend — extend `savePaperlessSettings` API call

**Files:**
- Modify: [src/lib/api.ts](../../src/lib/api.ts):562

**Step 1: Modify signature**

In [api.ts](../../src/lib/api.ts):562:

```ts
export async function savePaperlessSettings(
    url: string | null,
    token: string | null,
    enabled: boolean | null = null,
    fieldNameDatetime: string | null = null,
    fieldNameLiters: string | null = null,
    fieldNameTotal: string | null = null,
): Promise<void> {
    return apiCall('save_paperless_settings', {
        url,
        token,
        enabled,
        fieldNameDatetime,
        fieldNameLiters,
        fieldNameTotal,
    });
}
```

The defaults (`= null`) keep all existing call sites compiling — they'll just send `None` for the three new fields, preserving "keep existing" semantics.

**Step 2: Type check**

Run: `npm run check`
Expected: No new errors from this change. Pre-existing call sites in [+page.svelte](../../src/routes/settings/+page.svelte) still work.

**Step 3: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(paperless): savePaperlessSettings accepts optional field name overrides"
```

---

### Task 11: Add i18n strings (Slovak + English)

**Files:**
- Modify: [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts)
- Modify: [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts)
- Regenerate: [src/lib/i18n/i18n-types.ts](../../src/lib/i18n/i18n-types.ts)

**Step 1: Add Slovak keys**

In [sk/index.ts](../../src/lib/i18n/sk/index.ts), inside the existing `paperless: { ... }` block (line ~684), add a nested `customFields` group:

```ts
        customFields: {
            sectionTitle: 'Vlastné polia',
            sectionDescription: 'Názvy vlastných polí v Paperless. Necháj prázdne pre predvolenú hodnotu.',
            datetime: 'Pole pre dátum/čas',
            liters: 'Pole pre litre',
            total: 'Pole pre sumu',
            placeholderDatetime: 'receipt_datetime',
            placeholderLiters: 'litres',
            placeholderTotal: 'total_amount',
        },
```

**Step 2: Add English keys**

In [en/index.ts](../../src/lib/i18n/en/index.ts), inside the matching `paperless: { ... }` block (line ~710), add the same group with English translations:

```ts
        customFields: {
            sectionTitle: 'Custom fields',
            sectionDescription: 'Names of custom fields in Paperless. Leave empty to use the default.',
            datetime: 'Datetime field',
            liters: 'Liters field',
            total: 'Total amount field',
            placeholderDatetime: 'receipt_datetime',
            placeholderLiters: 'litres',
            placeholderTotal: 'total_amount',
        },
```

**Step 3: Regenerate types**

Run: `npm run i18n:generate` (or the project's typesafe-i18n generator command — check the [package.json](../../package.json) `scripts` block).

If the project uses watch mode for i18n type generation, this runs automatically. Otherwise, the generated `i18n-types.ts` reflects the new keys.

**Step 4: Type check**

Run: `npm run check`
Expected: PASS (new keys typed correctly).

**Step 5: Commit**

```bash
git add src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts src/lib/i18n/i18n-types.ts
git commit -m "feat(i18n): add custom-fields keys for Paperless settings"
```

---

### Task 12: Add three text inputs to Settings UI

**Files:**
- Modify: [src/routes/settings/+page.svelte](../../src/routes/settings/+page.svelte)

**Step 1: Add state variables**

Near the existing Paperless state block (around line 88-103):

```ts
let paperlessFieldDatetime = '';
let paperlessFieldLiters = '';
let paperlessFieldTotal = '';
let initialPaperlessFieldDatetime = '';
let initialPaperlessFieldLiters = '';
let initialPaperlessFieldTotal = '';
```

**Step 2: Hydrate from `getPaperlessSettings`**

In the load block where existing paperless fields are populated (around line 561-566):

```ts
const paperlessSettings = await getPaperlessSettings();
paperlessUrl = paperlessSettings.url || '';
paperlessHasToken = paperlessSettings.hasToken;
paperlessEnabled = paperlessSettings.enabled;
paperlessFieldDatetime = paperlessSettings.fieldNameDatetime;
paperlessFieldLiters = paperlessSettings.fieldNameLiters;
paperlessFieldTotal = paperlessSettings.fieldNameTotal;
initialPaperlessUrl = paperlessUrl;
initialPaperlessFieldDatetime = paperlessFieldDatetime;
initialPaperlessFieldLiters = paperlessFieldLiters;
initialPaperlessFieldTotal = paperlessFieldTotal;
```

**Step 3: Add a debounced save handler for field names**

Below the existing `savePaperlessSettingsNow` function (around line 290):

```ts
async function savePaperlessFieldNamesNow() {
    if (
        paperlessFieldDatetime === initialPaperlessFieldDatetime &&
        paperlessFieldLiters === initialPaperlessFieldLiters &&
        paperlessFieldTotal === initialPaperlessFieldTotal
    ) {
        return;
    }
    try {
        // Send empty strings as "" so the backend interprets them as "use default";
        // do NOT send null because that means "keep existing".
        await savePaperlessSettings(
            null, null, null,
            paperlessFieldDatetime,
            paperlessFieldLiters,
            paperlessFieldTotal,
        );
        initialPaperlessFieldDatetime = paperlessFieldDatetime;
        initialPaperlessFieldLiters = paperlessFieldLiters;
        initialPaperlessFieldTotal = paperlessFieldTotal;
        toast.success($LL.toast.settingsSaved());
    } catch (error) {
        toast.error($LL.toast.errorSavingSettings({ error: String(error) }));
    }
}

const debouncedSavePaperlessFieldNames = debounce(savePaperlessFieldNamesNow, 800);
```

**Step 4: Add UI section**

Inside the existing Paperless section (after the existing token input, around line 1162), add:

```svelte
<div class="form-group" class:disabled={!paperlessEnabled}>
    <h3>{$LL.paperless.customFields.sectionTitle()}</h3>
    <p class="hint">{$LL.paperless.customFields.sectionDescription()}</p>

    <div class="form-row">
        <label for="paperless-field-datetime">{$LL.paperless.customFields.datetime()}</label>
        <input
            type="text"
            id="paperless-field-datetime"
            data-test="paperless-field-datetime"
            bind:value={paperlessFieldDatetime}
            placeholder={$LL.paperless.customFields.placeholderDatetime()}
            on:input={debouncedSavePaperlessFieldNames}
            on:blur={savePaperlessFieldNamesNow}
            disabled={!paperlessEnabled}
        />
    </div>
    <div class="form-row">
        <label for="paperless-field-liters">{$LL.paperless.customFields.liters()}</label>
        <input
            type="text"
            id="paperless-field-liters"
            data-test="paperless-field-liters"
            bind:value={paperlessFieldLiters}
            placeholder={$LL.paperless.customFields.placeholderLiters()}
            on:input={debouncedSavePaperlessFieldNames}
            on:blur={savePaperlessFieldNamesNow}
            disabled={!paperlessEnabled}
        />
    </div>
    <div class="form-row">
        <label for="paperless-field-total">{$LL.paperless.customFields.total()}</label>
        <input
            type="text"
            id="paperless-field-total"
            data-test="paperless-field-total"
            bind:value={paperlessFieldTotal}
            placeholder={$LL.paperless.customFields.placeholderTotal()}
            on:input={debouncedSavePaperlessFieldNames}
            on:blur={savePaperlessFieldNamesNow}
            disabled={!paperlessEnabled}
        />
    </div>
</div>
```

**Step 5: Type check**

Run: `npm run check`
Expected: PASS.

**Step 6: Visual smoke test**

Run: `npm run tauri:dev`

Navigate to Settings → Paperless. Verify:
1. Three new inputs appear, prefilled with current defaults (or saved custom values)
2. Editing an input → settings save (toast appears)
3. Clearing an input and blurring → backend stores `None`, but the placeholder shows the default

**Step 7: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(paperless): three text inputs for custom field names in Settings"
```

---

### Task 13: Update integration test for custom field names

**Files:**
- Modify: [tests/integration/specs/tier2/paperless-integration.spec.ts](../../tests/integration/specs/tier2/paperless-integration.spec.ts)

**Step 1: Inspect existing spec**

Read [paperless-integration.spec.ts](../../tests/integration/specs/tier2/paperless-integration.spec.ts) to understand the mock-server fixture pattern.

**Step 2: Add test case**

Append a new test:

```ts
it('persists custom field names and uses them when fetching invoices', async () => {
    await openSettings();
    await fillPaperlessConfig({ url: MOCK_URL, token: 'test-token' });

    // Set custom field names
    await $('[data-test="paperless-field-datetime"]').setValue('Dátum');
    await $('[data-test="paperless-field-liters"]').setValue('Objem');
    await $('[data-test="paperless-field-total"]').setValue('Suma');
    await $('[data-test="paperless-field-total"]').click(); // blur trigger

    await waitForToast('settings saved');

    // Reload page → verify persisted
    await browser.refresh();
    await openSettings();
    await expect($('[data-test="paperless-field-datetime"]')).toHaveValue('Dátum');
    await expect($('[data-test="paperless-field-liters"]')).toHaveValue('Objem');
    await expect($('[data-test="paperless-field-total"]')).toHaveValue('Suma');
});
```

(Adapt selectors and helpers to match the existing spec's style — use whatever `openSettings`/`waitForToast` pattern the file already uses.)

**Step 3: Build debug binary if needed**

Run: `npm run test:integration:build` (per [CLAUDE.md](../../CLAUDE.md))

**Step 4: Run focused integration test**

Run:
```bash
npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/paperless-integration.spec.ts
```
Expected: PASS.

**Step 5: Commit**

```bash
git add tests/integration/specs/tier2/paperless-integration.spec.ts
git commit -m "test(paperless): integration test for custom field names persistence"
```

---

### Task 14: CHANGELOG entry + final verification

**Files:**
- Modify: [CHANGELOG.md](../../CHANGELOG.md)

**Step 1: Add entry to `[Unreleased]`**

In [CHANGELOG.md](../../CHANGELOG.md) under `## [Unreleased]` → `### Added`:

```markdown
- Configurable Paperless custom field names — Settings UI now lets users override the names of the three Paperless custom fields the integration looks up (datetime, liters, total amount). Defaults preserve previous behavior; existing users see no change.
```

**Step 2: Run full test suite**

Run: `npm run test:all`
Expected: All Rust unit tests + integration tests PASS.

**Step 3: Manual end-to-end smoke**

Run: `npm run tauri:dev`

1. Settings → Paperless → set custom field names ("Dátum", "Objem", "Suma")
2. Configure your Paperless server (or mock) to have custom fields with those names
3. Verify Doklady page loads invoices using the custom-named fields
4. Clear one input → verify falls back to default
5. Reload app → verify settings persist

**Step 4: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): note configurable Paperless field names"
```

**Step 5: Move task to done (post-merge)**

When the PR for this work merges:

```bash
git mv _tasks/63-paperless-configurable-fields _tasks/_done/63-paperless-configurable-fields
# Then update _tasks/index.md — move task 63 from Active to Completed
```

---

## Rollback strategy

Each task is a single small commit. If anything goes wrong mid-implementation:

```bash
git log --oneline | head -20    # Find the last good commit
git reset --hard <good-commit>  # ⚠️ only if no other unrelated work is in progress
```

For a partial revert (e.g., Tauri wrapper changes break dev mode but backend is good), revert specific commits:

```bash
git revert <commit-sha>
```

## Definition of Done

- [ ] All 14 tasks committed sequentially
- [ ] `cd src-tauri && cargo test --lib` passes (zero failures, all new tests counted)
- [ ] `npm run check` passes (no new TypeScript errors)
- [ ] Focused integration spec passes: `npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/paperless-integration.spec.ts`
- [ ] Full integration suite passes: `npm run test:integration:tier1` (and full test:integration before merge)
- [ ] Manual smoke verifies UI flow end-to-end
- [ ] [CHANGELOG.md](../../CHANGELOG.md) updated
- [ ] No regressions in unrelated areas (run `git diff main` and skim)
