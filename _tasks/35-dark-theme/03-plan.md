# Dark Theme Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add dark theme support with system preference detection and manual toggle on Settings page.

**Architecture:** CSS Custom Properties define color tokens, `data-theme` attribute on `<html>` toggles palettes, theme preference stored in `local.settings.json` via Rust backend.

**Tech Stack:** Svelte 5, Tauri 2, Rust, CSS Custom Properties

---

## Task 1: Backend - Extend LocalSettings

**Files:**
- Modify: `src-tauri/src/settings.rs:8-12`

**Step 1: Add theme field to LocalSettings struct**

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalSettings {
    pub gemini_api_key: Option<String>,
    pub receipts_folder_path: Option<String>,
    pub theme: Option<String>,  // "system" | "light" | "dark"
}
```

**Step 2: Run existing tests to verify no regression**

Run: `cd src-tauri && cargo test settings`
Expected: All 3 tests PASS

**Step 3: Add test for theme field**

Add to `src-tauri/src/settings.rs` tests:
```rust
#[test]
fn test_load_with_theme() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("local.settings.json");
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(b"{\"theme\": \"dark\"}").unwrap();

    let settings = LocalSettings::load(&dir.path().to_path_buf());
    assert_eq!(settings.theme, Some("dark".to_string()));
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test test_load_with_theme`
Expected: PASS (serde handles it automatically)

**Step 5: Commit**

```bash
git add src-tauri/src/settings.rs
git commit -m "feat(settings): add theme field to LocalSettings"
```

---

## Task 2: Backend - Add Theme Commands

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`

**Step 1: Add get_theme_preference command**

Add to `src-tauri/src/commands.rs`:
```rust
#[tauri::command]
pub fn get_theme_preference(state: State<'_, AppState>) -> String {
    state
        .local_settings
        .lock()
        .unwrap()
        .theme
        .clone()
        .unwrap_or_else(|| "system".to_string())
}
```

**Step 2: Add set_theme_preference command**

Add to `src-tauri/src/commands.rs`:
```rust
#[tauri::command]
pub fn set_theme_preference(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    theme: String,
) -> Result<(), String> {
    // Validate
    if !["system", "light", "dark"].contains(&theme.as_str()) {
        return Err(format!("Invalid theme: {}. Must be system, light, or dark", theme));
    }

    // Update in-memory state
    {
        let mut settings = state.local_settings.lock().unwrap();
        settings.theme = Some(theme.clone());
    }

    // Persist to file
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let settings_path = app_data_dir.join("local.settings.json");

    let settings = state.local_settings.lock().unwrap();
    let json = serde_json::to_string_pretty(&*settings)
        .map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, json)
        .map_err(|e| e.to_string())?;

    Ok(())
}
```

**Step 3: Register commands in main.rs**

Find the `invoke_handler` in `src-tauri/src/main.rs` and add:
```rust
commands::get_theme_preference,
commands::set_theme_preference,
```

**Step 4: Run cargo check**

Run: `cd src-tauri && cargo check`
Expected: No errors

**Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(commands): add theme preference get/set commands"
```

---

## Task 3: Frontend - API Functions

**Files:**
- Modify: `src/lib/api.ts`

**Step 1: Add theme API functions**

Add to `src/lib/api.ts`:
```typescript
export type ThemeMode = 'system' | 'light' | 'dark';

export async function getThemePreference(): Promise<ThemeMode> {
    return invoke<string>('get_theme_preference') as Promise<ThemeMode>;
}

export async function setThemePreference(theme: ThemeMode): Promise<void> {
    return invoke('set_theme_preference', { theme });
}
```

**Step 2: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(api): add theme preference functions"
```

---

## Task 4: Frontend - Theme CSS Variables

**Files:**
- Create: `src/lib/theme.css`
- Modify: `src/routes/+layout.svelte`

**Step 1: Create theme.css with CSS custom properties**

Create `src/lib/theme.css`:
```css
:root {
    /* Light theme (default) */
    --bg-body: #f5f5f5;
    --bg-surface: #ffffff;
    --bg-surface-alt: #fafafa;
    --bg-header: #2c3e50;

    --text-primary: #2c3e50;
    --text-secondary: #7f8c8d;
    --text-muted: #95a5a6;
    --text-on-header: #ffffff;
    --text-on-header-muted: rgba(255, 255, 255, 0.7);

    --border-default: #e0e0e0;
    --border-input: #d5dbdb;
    --shadow-default: rgba(0, 0, 0, 0.1);

    --accent-primary: #3498db;
    --accent-primary-hover: #2980b9;
    --accent-success: #27ae60;
    --accent-success-hover: #219653;
    --accent-warning: #d39e00;
    --accent-danger: #c0392b;
    --accent-danger-bg: #fee;

    /* Form elements */
    --input-bg: #ffffff;
    --input-focus-shadow: rgba(52, 152, 219, 0.1);

    /* Buttons */
    --btn-secondary-bg: #ecf0f1;
    --btn-secondary-hover: #d5dbdb;
    --btn-primary-light-bg: #d4edda;
    --btn-primary-light-color: #155724;
    --btn-primary-light-hover: #c3e6cb;
}

[data-theme="dark"] {
    --bg-body: #121212;
    --bg-surface: #1e1e1e;
    --bg-surface-alt: #252525;
    --bg-header: #1e1e1e;

    --text-primary: #e0e0e0;
    --text-secondary: #a0a0a0;
    --text-muted: #707070;
    --text-on-header: #e0e0e0;
    --text-on-header-muted: rgba(255, 255, 255, 0.6);

    --border-default: #333333;
    --border-input: #404040;
    --shadow-default: rgba(0, 0, 0, 0.3);

    /* Brighter accents for dark mode */
    --accent-primary: #5dade2;
    --accent-primary-hover: #3498db;
    --accent-success: #58d68d;
    --accent-success-hover: #27ae60;
    --accent-warning: #f4d03f;
    --accent-danger: #ec7063;
    --accent-danger-bg: #3d2020;

    /* Form elements */
    --input-bg: #2a2a2a;
    --input-focus-shadow: rgba(93, 173, 226, 0.2);

    /* Buttons */
    --btn-secondary-bg: #333333;
    --btn-secondary-hover: #404040;
    --btn-primary-light-bg: #1e3a1e;
    --btn-primary-light-color: #58d68d;
    --btn-primary-light-hover: #2a4a2a;
}
```

**Step 2: Import theme.css in layout**

Add to top of `src/routes/+layout.svelte` `<script>` section:
```typescript
import '$lib/theme.css';
```

**Step 3: Commit**

```bash
git add src/lib/theme.css src/routes/+layout.svelte
git commit -m "feat(theme): add CSS custom properties for light/dark themes"
```

---

## Task 5: Frontend - Theme Store

**Files:**
- Create: `src/lib/stores/theme.ts`

**Step 1: Create theme store**

Create `src/lib/stores/theme.ts`:
```typescript
import { writable } from 'svelte/store';
import { getThemePreference, setThemePreference, type ThemeMode } from '$lib/api';

function createThemeStore() {
    const { subscribe, set } = writable<ThemeMode>('system');

    function applyTheme(mode: ThemeMode) {
        const isDark =
            mode === 'dark' ||
            (mode === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
        document.documentElement.dataset.theme = isDark ? 'dark' : 'light';
    }

    return {
        subscribe,
        init: async () => {
            const saved = await getThemePreference();
            set(saved);
            applyTheme(saved);

            // Listen for system preference changes
            window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
                // Re-apply if in system mode
                getThemePreference().then((current) => {
                    if (current === 'system') {
                        applyTheme('system');
                    }
                });
            });
        },
        change: async (mode: ThemeMode) => {
            await setThemePreference(mode);
            set(mode);
            applyTheme(mode);
        }
    };
}

export const themeStore = createThemeStore();
```

**Step 2: Commit**

```bash
git add src/lib/stores/theme.ts
git commit -m "feat(stores): add theme store with system preference detection"
```

---

## Task 6: Frontend - Initialize Theme in Layout

**Files:**
- Modify: `src/routes/+layout.svelte`

**Step 1: Import and initialize theme store**

Add to imports in `src/routes/+layout.svelte`:
```typescript
import { themeStore } from '$lib/stores/theme';
```

Add to `onMount` function (early, before other async operations):
```typescript
// Initialize theme first to avoid flash
await themeStore.init();
```

**Step 2: Test manually**

Run: `npm run tauri dev`
Expected: App loads without errors. Theme defaults to light (no data-theme yet, but CSS variables work).

**Step 3: Commit**

```bash
git add src/routes/+layout.svelte
git commit -m "feat(layout): initialize theme store on app startup"
```

---

## Task 7: i18n - Add Theme Labels

**Files:**
- Modify: `src/lib/i18n/sk/index.ts`
- Modify: `src/lib/i18n/en/index.ts`

**Step 1: Add Slovak translations**

Add to `settings` section in `src/lib/i18n/sk/index.ts`:
```typescript
appearanceSection: 'Vzhľad',
themeLabel: 'Téma',
themeSystem: 'Podľa systému',
themeLight: 'Svetlá',
themeDark: 'Tmavá',
```

**Step 2: Add English translations**

Add to `settings` section in `src/lib/i18n/en/index.ts`:
```typescript
appearanceSection: 'Appearance',
themeLabel: 'Theme',
themeSystem: 'System default',
themeLight: 'Light',
themeDark: 'Dark',
```

**Step 3: Commit**

```bash
git add src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts
git commit -m "feat(i18n): add theme setting translations"
```

---

## Task 8: Settings Page - Add Theme Toggle

**Files:**
- Modify: `src/routes/settings/+page.svelte`

**Step 1: Import theme store and type**

Add to imports:
```typescript
import { themeStore } from '$lib/stores/theme';
import type { ThemeMode } from '$lib/api';
```

**Step 2: Add theme state**

Add to state variables:
```typescript
let selectedTheme: ThemeMode = 'system';
```

**Step 3: Initialize theme in onMount**

Add to the async IIFE in `onMount`:
```typescript
// Load theme preference
themeStore.subscribe((theme) => {
    selectedTheme = theme;
});
```

**Step 4: Add theme change handler**

Add function:
```typescript
async function handleThemeChange(theme: ThemeMode) {
    selectedTheme = theme;
    await themeStore.change(theme);
}
```

**Step 5: Add Appearance section after Language section**

Add after the Language section closing `</section>`:
```svelte
<!-- Appearance Section -->
<section class="settings-section">
    <h2>{$LL.settings.appearanceSection()}</h2>
    <div class="section-content">
        <div class="form-group">
            <label>{$LL.settings.themeLabel()}</label>
            <div class="theme-options">
                <label class="theme-option">
                    <input
                        type="radio"
                        name="theme"
                        value="system"
                        checked={selectedTheme === 'system'}
                        on:change={() => handleThemeChange('system')}
                    />
                    <span>{$LL.settings.themeSystem()}</span>
                </label>
                <label class="theme-option">
                    <input
                        type="radio"
                        name="theme"
                        value="light"
                        checked={selectedTheme === 'light'}
                        on:change={() => handleThemeChange('light')}
                    />
                    <span>{$LL.settings.themeLight()}</span>
                </label>
                <label class="theme-option">
                    <input
                        type="radio"
                        name="theme"
                        value="dark"
                        checked={selectedTheme === 'dark'}
                        on:change={() => handleThemeChange('dark')}
                    />
                    <span>{$LL.settings.themeDark()}</span>
                </label>
            </div>
        </div>
    </div>
</section>
```

**Step 6: Add styles for theme options**

Add to `<style>` section:
```css
.theme-options {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.theme-option {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    padding: 0.5rem;
    border-radius: 4px;
    transition: background-color 0.2s;
}

.theme-option:hover {
    background-color: var(--bg-surface-alt);
}

.theme-option input[type="radio"] {
    width: 18px;
    height: 18px;
    cursor: pointer;
}

.theme-option span {
    color: var(--text-primary);
}
```

**Step 7: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(settings): add theme toggle with radio buttons"
```

---

## Task 9: Migrate Layout Styles

**Files:**
- Modify: `src/routes/+layout.svelte`

**Step 1: Replace hardcoded colors with CSS variables**

Update the `<style>` section, replacing:

| Before | After |
|--------|-------|
| `background-color: #f5f5f5` | `background-color: var(--bg-body)` |
| `background-color: #2c3e50` | `background-color: var(--bg-header)` |
| `color: white` (in header) | `color: var(--text-on-header)` |
| `rgba(255, 255, 255, 0.7)` | `var(--text-on-header-muted)` |
| `rgba(255, 255, 255, 0.1)` | `rgba(255, 255, 255, 0.1)` (keep) |
| `rgba(255, 255, 255, 0.2)` | `rgba(255, 255, 255, 0.2)` (keep) |
| `background-color: white` (select) | `background-color: var(--input-bg)` |
| `border: 1px solid #ddd` | `border: 1px solid var(--border-input)` |
| `border-color: #3498db` | `border-color: var(--accent-primary)` |
| `rgba(52, 152, 219, 0.1)` | `var(--input-focus-shadow)` |
| `rgba(0, 0, 0, 0.1)` (shadow) | `var(--shadow-default)` |

**Step 2: Test both themes**

Run: `npm run tauri dev`
1. Go to Settings > Appearance
2. Switch to Dark - header and body should change
3. Switch to Light - should revert
4. Switch to System - should follow OS preference

**Step 3: Commit**

```bash
git add src/routes/+layout.svelte
git commit -m "feat(layout): migrate to CSS variables for theming"
```

---

## Task 10: Migrate Main Page Styles

**Files:**
- Modify: `src/routes/+page.svelte`

**Step 1: Replace hardcoded colors**

| Before | After |
|--------|-------|
| `background: white` | `background: var(--bg-surface)` |
| `rgba(0, 0, 0, 0.1)` | `var(--shadow-default)` |
| `color: #2c3e50` | `color: var(--text-primary)` |
| `color: #7f8c8d` | `color: var(--text-secondary)` |
| `background-color: #27ae60` | `background-color: var(--accent-success)` |
| `background-color: #219653` | `background-color: var(--accent-success-hover)` |
| `background-color: #bdc3c7` | `background-color: var(--text-muted)` |
| `color: #d39e00` | `color: var(--accent-warning)` |
| `background-color: #3498db` | `background-color: var(--accent-primary)` |
| `background-color: #2980b9` | `background-color: var(--accent-primary-hover)` |

**Step 2: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat(home): migrate to CSS variables for theming"
```

---

## Task 11: Migrate Settings Page Styles

**Files:**
- Modify: `src/routes/settings/+page.svelte`

**Step 1: Replace hardcoded colors**

Same pattern as Task 10. Key replacements:
- `background: white` → `var(--bg-surface)`
- `background: #fafafa` → `var(--bg-surface-alt)`
- `color: #2c3e50` → `var(--text-primary)`
- `color: #7f8c8d` → `var(--text-secondary)`
- `border: 1px solid #e0e0e0` → `border: 1px solid var(--border-default)`
- `background-color: #3498db` → `var(--accent-primary)`
- `background-color: #ecf0f1` → `var(--btn-secondary-bg)`
- `background-color: #fee` → `var(--accent-danger-bg)`
- `color: #c0392b` → `var(--accent-danger)`

**Step 2: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(settings): migrate to CSS variables for theming"
```

---

## Task 12: Migrate Remaining Components

**Files:**
- Modify: `src/lib/components/TripGrid.svelte`
- Modify: `src/lib/components/TripRow.svelte`
- Modify: `src/lib/components/VehicleModal.svelte`
- Modify: `src/lib/components/ConfirmModal.svelte`
- Modify: `src/lib/components/Toast.svelte`
- Modify: `src/lib/components/CompensationBanner.svelte`
- Modify: `src/lib/components/Autocomplete.svelte`
- Modify: `src/lib/components/TripSelectorModal.svelte`

**Step 1: Apply same color mapping to all components**

Use the established variable mapping from Tasks 9-11.

**Step 2: Test all components in both themes**

Run: `npm run tauri dev`
- Add/edit trips
- Open modals
- Test toasts
- Check receipts page

**Step 3: Commit**

```bash
git add src/lib/components/
git commit -m "feat(components): migrate all components to CSS variables"
```

---

## Task 13: Migrate Receipts Page

**Files:**
- Modify: `src/routes/doklady/+page.svelte`

**Step 1: Replace hardcoded colors**

Same pattern as other pages.

**Step 2: Commit**

```bash
git add src/routes/doklady/+page.svelte
git commit -m "feat(receipts): migrate to CSS variables for theming"
```

---

## Task 14: Final Testing & Cleanup

**Step 1: Full theme test**

Run: `npm run tauri dev`
1. Test all three theme modes
2. Navigate through all pages
3. Open all modals
4. Verify no hardcoded colors remain visible

**Step 2: Test persistence**

1. Set theme to Dark
2. Close app completely
3. Reopen app
4. Verify theme is still Dark

**Step 3: Test system preference**

1. Set theme to System
2. Change OS to dark mode
3. Verify app switches automatically

**Step 4: Run all tests**

Run: `npm run test:backend`
Expected: All tests pass

**Step 5: Update changelog**

Run: `/changelog`

**Step 6: Final commit**

```bash
git add CHANGELOG.md
git commit -m "docs: add dark theme to changelog"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Backend - Extend LocalSettings | settings.rs |
| 2 | Backend - Add Theme Commands | commands.rs, main.rs |
| 3 | Frontend - API Functions | api.ts |
| 4 | Frontend - Theme CSS Variables | theme.css, +layout.svelte |
| 5 | Frontend - Theme Store | theme.ts |
| 6 | Frontend - Initialize Theme | +layout.svelte |
| 7 | i18n - Add Theme Labels | sk/index.ts, en/index.ts |
| 8 | Settings Page - Theme Toggle | settings/+page.svelte |
| 9 | Migrate Layout Styles | +layout.svelte |
| 10 | Migrate Main Page | +page.svelte |
| 11 | Migrate Settings Page | settings/+page.svelte |
| 12 | Migrate Components | components/*.svelte |
| 13 | Migrate Receipts Page | doklady/+page.svelte |
| 14 | Final Testing & Cleanup | CHANGELOG.md |
