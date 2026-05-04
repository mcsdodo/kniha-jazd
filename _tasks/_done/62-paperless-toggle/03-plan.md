**Date:** 2026-05-04
**Subject:** Paperless enable/disable toggle — implementation plan
**Status:** Planning

# Paperless Enable/Disable Toggle Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `paperless_enabled` boolean to settings so users can switch Paperless on/off without losing their saved URL and token.

**Architecture:** Add `paperless_enabled: Option<bool>` to `LocalSettings`; update `get_invoice_source_mode_from_settings` to require it; extend `save_paperless_settings_internal` and the Tauri wrapper with an `enabled` param; add a toggle switch in the Settings UI.

**Tech Stack:** Rust (Tauri core), SvelteKit/TypeScript, typesafe-i18n, WebdriverIO integration tests.

---

### Task 1: Backend — failing tests for `enabled` flag in `InvoiceSourceMode`

**Files:**
- Modify: [src-tauri/core/src/commands_internal/integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs)

**Step 1: Write the three failing tests**

Append to the end of [integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs):

```rust
#[test]
fn invoice_source_mode_is_local_when_disabled_even_with_credentials() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x".into());
    s.paperless_api_token = Some("t".into());
    s.paperless_enabled = Some(false);
    assert_eq!(get_invoice_source_mode_from_settings(&s), InvoiceSourceMode::Local);
}

#[test]
fn invoice_source_mode_is_paperless_when_enabled_true_with_credentials() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x".into());
    s.paperless_api_token = Some("t".into());
    s.paperless_enabled = Some(true);
    assert_eq!(get_invoice_source_mode_from_settings(&s), InvoiceSourceMode::Paperless);
}

#[test]
fn invoice_source_mode_is_paperless_when_enabled_none_with_credentials_backward_compat() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x".into());
    s.paperless_api_token = Some("t".into());
    // None means "not explicitly set" — treat as enabled for backward compat
    s.paperless_enabled = None;
    assert_eq!(get_invoice_source_mode_from_settings(&s), InvoiceSourceMode::Paperless);
}
```

**Step 2: Run tests to verify they fail**

```bash
cd src-tauri && cargo test invoice_source_mode_is_local_when_disabled
```

Expected: `FAILED` — field `paperless_enabled` does not exist on `LocalSettings`

**Step 3: Add `paperless_enabled` field to `LocalSettings`**

In [src-tauri/core/src/settings.rs](../../src-tauri/core/src/settings.rs), after line 42 (`pub paperless_api_token: Option<String>,`):

```rust
    pub paperless_enabled: Option<bool>,  // None = backward-compat (true when url+token set)
```

**Step 4: Update `get_invoice_source_mode_from_settings`**

In [src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs), replace lines 256–261:

```rust
pub fn get_invoice_source_mode_from_settings(s: &LocalSettings) -> InvoiceSourceMode {
    let enabled = s.paperless_enabled.unwrap_or(true);
    match (&s.paperless_url, &s.paperless_api_token) {
        (Some(u), Some(t)) if enabled && !u.trim().is_empty() && !t.trim().is_empty() => {
            InvoiceSourceMode::Paperless
        }
        _ => InvoiceSourceMode::Local,
    }
}
```

**Step 5: Run all backend tests to verify they pass**

```bash
cd src-tauri && cargo test
```

Expected: All tests pass (195 + 3 new = 198 total).

**Step 6: Commit**

```bash
git add src-tauri/core/src/settings.rs src-tauri/core/src/commands_internal/integrations.rs src-tauri/core/src/commands_internal/integrations_tests.rs
git commit -m "feat(paperless): add paperless_enabled field + update InvoiceSourceMode logic"
```

---

### Task 2: Backend — extend `save_paperless_settings` and `PaperlessSettingsResponse`

**Files:**
- Modify: [src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs)
- Modify: [src-tauri/core/src/commands_internal/integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs)

**Step 1: Write two failing tests**

Append to [integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs):

```rust
#[test]
fn save_paperless_settings_persists_enabled_flag() {
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        Some("https://x.example".into()),
        Some("tok".into()),
        Some(false),
    ).unwrap();

    let loaded = crate::settings::LocalSettings::load(&dir.path().to_path_buf());
    assert_eq!(loaded.paperless_enabled, Some(false));
}

#[test]
fn get_paperless_settings_returns_enabled_field() {
    let dir = tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x.example".into());
    s.paperless_api_token = Some("tok".into());
    s.paperless_enabled = Some(false);
    s.save(&dir.path().to_path_buf()).unwrap();

    let r = get_paperless_settings_internal(&dir.path().to_path_buf()).unwrap();
    assert!(!r.enabled);
}
```

**Step 2: Run tests to verify they fail**

```bash
cd src-tauri && cargo test save_paperless_settings_persists_enabled_flag
```

Expected: `FAILED` — `save_paperless_settings_internal` does not accept an `enabled` argument

**Step 3: Add `enabled: bool` to `PaperlessSettingsResponse`**

In [integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs), replace lines 179–184:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperlessSettingsResponse {
    pub url: Option<String>,
    pub has_token: bool,
    pub enabled: bool,
}
```

**Step 4: Update `get_paperless_settings_internal`**

Replace lines 186–195:

```rust
pub fn get_paperless_settings_internal(app_dir: &Path) -> Result<PaperlessSettingsResponse, String> {
    let settings = LocalSettings::load(app_dir);
    let enabled = settings.paperless_enabled.unwrap_or(true);
    Ok(PaperlessSettingsResponse {
        url: settings.paperless_url,
        has_token: settings
            .paperless_api_token
            .as_deref()
            .is_some_and(|t| !t.trim().is_empty()),
        enabled,
    })
}
```

**Step 5: Add `enabled: Option<bool>` to `save_paperless_settings_internal`**

Replace lines 197–224:

```rust
pub fn save_paperless_settings_internal(
    app_dir: &Path,
    app_state: &AppState,
    url: Option<String>,
    token: Option<String>,
    enabled: Option<bool>,
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
    settings.save(app_dir).map_err(|e| e.to_string())
}
```

**Step 6: Fix the existing test that calls `save_paperless_settings_internal` without `enabled`**

The existing tests in [integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs) call `save_paperless_settings_internal` with 4 args. Add `None` as the 5th arg to each existing call site:

- `save_paperless_settings_persists_url_and_token` — add `, None` before `.unwrap()`
- `save_paperless_settings_none_args_preserves_existing` — both call sites
- `save_paperless_settings_rejects_invalid_url` — add `, None`
- `save_paperless_settings_blocked_by_read_only` — add `, None`

**Step 7: Run all backend tests**

```bash
cd src-tauri && cargo test
```

Expected: All tests pass.

**Step 8: Commit**

```bash
git add src-tauri/core/src/commands_internal/integrations.rs src-tauri/core/src/commands_internal/integrations_tests.rs
git commit -m "feat(paperless): extend save/response structs with enabled field"
```

---

### Task 3: Update Tauri command wrapper

**Files:**
- Modify: [src-tauri/desktop/src/commands/integrations.rs](../../src-tauri/desktop/src/commands/integrations.rs)

**Step 1: Add `enabled` param to the `save_paperless_settings` Tauri command**

Replace lines 135–144 in [src-tauri/desktop/src/commands/integrations.rs](../../src-tauri/desktop/src/commands/integrations.rs):

```rust
#[tauri::command]
pub fn save_paperless_settings(
    app_handle: tauri::AppHandle,
    app_state: State<Arc<AppState>>,
    url: Option<String>,
    token: Option<String>,
    enabled: Option<bool>,
) -> Result<(), String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    inner::save_paperless_settings_internal(&app_data_dir, &app_state, url, token, enabled)
}
```

**Step 2: Verify it compiles**

```bash
cd src-tauri && cargo build -p kniha-jazd-desktop 2>&1 | tail -5
```

Expected: `Finished` with no errors.

**Step 3: Commit**

```bash
git add src-tauri/desktop/src/commands/integrations.rs
git commit -m "feat(paperless): pass enabled param through Tauri command wrapper"
```

---

### Task 4: Frontend types and API

**Files:**
- Modify: [src/lib/types.ts](../../src/lib/types.ts)
- Modify: [src/lib/api.ts](../../src/lib/api.ts)

**Step 1: Add `enabled` to `PaperlessSettings` type**

In [src/lib/types.ts](../../src/lib/types.ts), replace lines 410–413:

```typescript
export interface PaperlessSettings {
    url: string | null;
    hasToken: boolean;
    enabled: boolean;
}
```

**Step 2: Add `enabled` param to `savePaperlessSettings`**

In [src/lib/api.ts](../../src/lib/api.ts), replace lines 561–564:

```typescript
// null = keep existing value, '' (empty string) = clear the value, null = keep existing for enabled
export async function savePaperlessSettings(url: string | null, token: string | null, enabled: boolean | null = null): Promise<void> {
    return apiCall('save_paperless_settings', { url, token, enabled });
}
```

**Step 3: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts
git commit -m "feat(paperless): add enabled field to frontend types and API"
```

---

### Task 5: i18n strings

**Files:**
- Modify: [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts)
- Modify: [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts)
- Modify: [src/lib/i18n/i18n-types.ts](../../src/lib/i18n/i18n-types.ts)

**Step 1: Add Slovak strings**

In [sk/index.ts](../../src/lib/i18n/sk/index.ts), after `tokenSet: 'Token je nastavený',` (inside the `paperless:` block):

```typescript
        enableToggle: 'Povoliť Paperless-ngx',
        enableToggleDisabledHint: 'Najprv nastav URL a token',
```

**Step 2: Add English strings**

In [en/index.ts](../../src/lib/i18n/en/index.ts), after `tokenSet: 'Token is set',` (inside the `paperless:` block):

```typescript
        enableToggle: 'Enable Paperless-ngx',
        enableToggleDisabledHint: 'Set URL and token first',
```

**Step 3: Add to `i18n-types.ts` — `BaseTranslation` section**

In [i18n-types.ts](../../src/lib/i18n/i18n-types.ts), find the `paperless:` block in the `BaseTranslation` type (around line 2286). After `tokenSet: string`, add:

```typescript
        /**
         * P​o​v​o​l​i​ť​ ​P​a​p​e​r​l​e​s​s​-​n​g​x
         */
        enableToggle: string
        /**
         * N​a​j​p​r​v​ ​n​a​s​t​a​v​ ​U​R​L​ ​a​ ​t​o​k​e​n
         */
        enableToggleDisabledHint: string
```

**Step 4: Add to `i18n-types.ts` — `TranslationFunctions` section**

In [i18n-types.ts](../../src/lib/i18n/i18n-types.ts), find the `paperless:` block in the `TranslationFunctions` type (around line 4553). After `tokenSet: () => LocalizedString`, add:

```typescript
        /**
         * Povoliť Paperless-ngx
         */
        enableToggle: () => LocalizedString
        /**
         * Najprv nastav URL a token
         */
        enableToggleDisabledHint: () => LocalizedString
```

**Step 5: Verify TypeScript types**

```bash
npm run check 2>&1 | tail -20
```

Expected: No type errors.

**Step 6: Commit**

```bash
git add src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts src/lib/i18n/i18n-types.ts
git commit -m "feat(paperless): add enableToggle i18n strings"
```

---

### Task 6: Settings UI — toggle switch

**Files:**
- Modify: [src/routes/settings/+page.svelte](../../src/routes/settings/+page.svelte)

**Step 1: Add state variable**

In [+page.svelte](../../src/routes/settings/+page.svelte), after the `let paperlessConnectionStatus` declaration (around line 102), add:

```svelte
let paperlessEnabled = false;
```

**Step 2: Load `enabled` in `onMount`**

Find the `onMount` block that loads Paperless settings (around line 542). Replace:

```svelte
const paperlessSettings = await getPaperlessSettings();
paperlessUrl = paperlessSettings.url || '';
paperlessHasToken = paperlessSettings.hasToken;
initialPaperlessUrl = paperlessUrl;
```

with:

```svelte
const paperlessSettings = await getPaperlessSettings();
paperlessUrl = paperlessSettings.url || '';
paperlessHasToken = paperlessSettings.hasToken;
paperlessEnabled = paperlessSettings.enabled;
initialPaperlessUrl = paperlessUrl;
```

**Step 3: Add toggle handler function**

After the `debouncedSavePaperlessSettings` declaration (around line 311), add:

```svelte
async function togglePaperlessEnabled(value: boolean) {
    paperlessEnabled = value;
    try {
        await savePaperlessSettings(null, null, value);
        toast.success($LL.toast.settingsSaved());
        if (value) {
            await testPaperlessConnectionStatus();
        } else {
            paperlessConnectionStatus = PAPERLESS_STATUS.IDLE;
        }
    } catch (error) {
        console.error('Failed to toggle Paperless:', error);
        paperlessEnabled = !value; // revert
        toast.error($LL.toast.errorSaveSettings({ error: String(error) }));
    }
}
```

**Step 4: Add toggle markup to the Paperless section**

In [+page.svelte](../../src/routes/settings/+page.svelte), replace the opening of the Paperless section (line 1078 `<div class="section-content">`):

```svelte
<div class="section-content">
    <p class="hint">{$LL.paperless.description()}</p>

    <div class="form-group toggle-row">
        <label class="toggle-label" for="paperless-enabled">
            {$LL.paperless.enableToggle()}
        </label>
        <button
            type="button"
            id="paperless-enabled"
            data-test="paperless-enabled-toggle"
            role="switch"
            aria-checked={paperlessEnabled}
            class="toggle-switch"
            class:active={paperlessEnabled}
            disabled={!initialPaperlessUrl || !paperlessHasToken}
            title={!initialPaperlessUrl || !paperlessHasToken ? $LL.paperless.enableToggleDisabledHint() : ''}
            on:click={() => togglePaperlessEnabled(!paperlessEnabled)}
        >
            <span class="toggle-thumb"></span>
        </button>
    </div>
```

Remove the duplicate `<p class="hint">` that was above the URL field (the one from the original section, keep only the one above the toggle).

**Step 5: Update existing `savePaperlessSettings` calls to pass `null` for `enabled`**

The `savePaperlessSettingsNow` function calls `savePaperlessSettings(paperlessUrl || null, paperlessApiToken || null)`. Since the new param has a default value of `null`, no change is needed — but verify by running TypeScript check.

**Step 6: Verify TypeScript**

```bash
npm run check 2>&1 | tail -20
```

Expected: No errors.

**Step 7: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(paperless): add enable/disable toggle to settings UI"
```

---

### Task 7: Integration test for toggle

**Files:**
- Modify: [tests/integration/specs/tier2/paperless-integration.spec.ts](../../tests/integration/specs/tier2/paperless-integration.spec.ts)

**Step 1: Update the existing `toggle off restores local mode` test section**

The existing test already has a "toggle off" step that uses `invokeTauri('save_paperless_settings', { url: '', token: '' })`. Replace that section with a toggle using `enabled: false` instead of clearing credentials, so the test verifies the new `enabled` flag:

In the test body, find the part that clears Paperless to verify local mode. Replace the IPC call that clears URL/token:

```typescript
// OLD: clears credentials
await invokeTauri<void>('save_paperless_settings', { url: '', token: '' });

// NEW: disables without clearing credentials
await invokeTauri<void>('save_paperless_settings', { url: null, token: null, enabled: false });
```

Then after navigating to Doklady, verify it shows local mode (not Paperless rows):

```typescript
// Navigate to Doklady — should show local receipts mode, not Paperless rows
await navigateTo('doklady');
await browser.pause(800);

// In local mode the "Refresh from Paperless" button must NOT be present
const refreshBtn = await $('[data-test="paperless-refresh"]');
expect(await refreshBtn.isExisting()).toBe(false);
```

Re-enable to verify switching back:

```typescript
// Re-enable Paperless
await invokeTauri<void>('save_paperless_settings', { url: null, token: null, enabled: true });
await navigateTo('doklady');
await browser.pause(800);

// Now Paperless rows should load again
const rows = await $$('[data-test="paperless-invoice-row"]');
expect(rows.length).toBeGreaterThan(0);
```

**Step 2: Run the focused integration test**

Requires the debug build to already be built (`npm run test:integration:build` if not already done).

```bash
npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/paperless-integration.spec.ts
```

Expected: All tests in this spec pass.

**Step 3: Commit**

```bash
git add tests/integration/specs/tier2/paperless-integration.spec.ts
git commit -m "test(paperless): update integration test to verify enabled/disabled toggle"
```

---

### Task 8: Final verification

**Step 1: Run all backend tests**

```bash
cd src-tauri && cargo test 2>&1 | tail -5
```

Expected: All tests pass.

**Step 2: Run full integration suite**

```bash
npm run test:integration:build
```

Expected: All tiers pass.
