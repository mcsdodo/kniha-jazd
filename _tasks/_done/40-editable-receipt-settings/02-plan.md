# Implementation Plan: Editable Receipt Settings

## Task 1: Backend Commands

**File:** `src-tauri/src/commands.rs`

Add two new Tauri commands:

```rust
#[tauri::command]
pub fn set_gemini_api_key(app_handle: tauri::AppHandle, api_key: String) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);

    // Allow empty string to clear, otherwise store as-is
    settings.gemini_api_key = if api_key.is_empty() { None } else { Some(api_key) };

    let settings_path = app_data_dir.join("local.settings.json");
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, json).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn set_receipts_folder_path(app_handle: tauri::AppHandle, path: String) -> Result<(), String> {
    // Validate path exists and is a directory
    if !path.is_empty() {
        let path_buf = std::path::PathBuf::from(&path);
        if !path_buf.exists() {
            return Err("Priečinok neexistuje".to_string());
        }
        if !path_buf.is_dir() {
            return Err("Cesta nie je priečinok".to_string());
        }
    }

    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.receipts_folder_path = if path.is_empty() { None } else { Some(path) };

    let settings_path = app_data_dir.join("local.settings.json");
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, json).map_err(|e| e.to_string())?;

    Ok(())
}
```

**File:** `src-tauri/src/lib.rs`

Register commands in `invoke_handler`:
```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    commands::set_gemini_api_key,
    commands::set_receipts_folder_path,
])
```

---

## Task 2: Frontend API

**File:** `src/lib/api.ts`

Add new functions after `setAutoCheckUpdates`:

```typescript
// Receipt settings
export async function setGeminiApiKey(apiKey: string): Promise<void> {
    return invoke('set_gemini_api_key', { apiKey });
}

export async function setReceiptsFolderPath(path: string): Promise<void> {
    return invoke('set_receipts_folder_path', { path });
}
```

---

## Task 3: i18n Translations

**File:** `src/lib/i18n/sk/index.ts`

Add to `settings` object:
```typescript
receiptScanningSection: 'Skenovanie dokladov',
geminiApiKey: 'Gemini API kľúč',
geminiApiKeyPlaceholder: 'Zadajte váš API kľúč',
geminiApiKeyHint: 'Potrebný pre rozpoznávanie textu z blokov',
receiptsFolder: 'Priečinok s dokladmi',
receiptsFolderPlaceholder: 'Vyberte priečinok',
receiptsFolderHint: 'Štruktúra: priečinok/ROK/súbory',
browseFolder: 'Vybrať',
showApiKey: 'Zobraziť',
hideApiKey: 'Skryť',
receiptSettingsSaved: 'Nastavenia skenovania uložené',
```

Add/update in `receipts` object:
```typescript
notConfiguredTitle: 'Skenovanie dokladov nie je nakonfigurované',
notConfiguredDescription: 'Pre automatické rozpoznávanie dokladov potrebujete:',
notConfiguredApiKey: 'Gemini API kľúč (pre OCR)',
notConfiguredFolder: 'Priečinok s naskenovanými dokladmi',
goToSettings: 'Prejsť do nastavení',
```

**File:** `src/lib/i18n/en/index.ts`

Same keys with English translations:
```typescript
// settings
receiptScanningSection: 'Receipt Scanning',
geminiApiKey: 'Gemini API Key',
geminiApiKeyPlaceholder: 'Enter your API key',
geminiApiKeyHint: 'Required for receipt text recognition',
receiptsFolder: 'Receipts Folder',
receiptsFolderPlaceholder: 'Select folder',
receiptsFolderHint: 'Structure: folder/YEAR/files',
browseFolder: 'Browse',
showApiKey: 'Show',
hideApiKey: 'Hide',
receiptSettingsSaved: 'Receipt scanning settings saved',

// receipts
notConfiguredTitle: 'Receipt scanning is not configured',
notConfiguredDescription: 'For automatic receipt recognition you need:',
notConfiguredApiKey: 'Gemini API key (for OCR)',
notConfiguredFolder: 'Folder with scanned receipts',
goToSettings: 'Go to Settings',
```

---

## Task 4: Settings Page - Receipt Scanning Section

**File:** `src/routes/settings/+page.svelte`

### 4.1 Add imports and state

```typescript
import { open } from '@tauri-apps/plugin-dialog';
import * as api from '$lib/api';

// Receipt scanning state
let receiptSettings: ReceiptSettings | null = null;
let geminiApiKey = '';
let receiptsFolderPath = '';
let showApiKey = false;
let savingReceiptSettings = false;
```

### 4.2 Load settings on mount

In the async IIFE inside `onMount`:
```typescript
// Load receipt settings
receiptSettings = await api.getReceiptSettings();
if (receiptSettings) {
    geminiApiKey = receiptSettings.geminiApiKey ?? '';
    receiptsFolderPath = receiptSettings.receiptsFolderPath ?? '';
}
```

### 4.3 Add scroll-to-section on mount

```typescript
// Scroll to section if hash present
if (window.location.hash === '#receipt-scanning') {
    setTimeout(() => {
        document.getElementById('receipt-scanning')?.scrollIntoView({ behavior: 'smooth' });
    }, 100);
}
```

### 4.4 Add handler functions

```typescript
async function handleBrowseFolder() {
    const selected = await open({ directory: true });
    if (selected && typeof selected === 'string') {
        receiptsFolderPath = selected;
    }
}

async function handleSaveReceiptSettings() {
    savingReceiptSettings = true;
    try {
        await api.setGeminiApiKey(geminiApiKey);
        await api.setReceiptsFolderPath(receiptsFolderPath);
        toast.success($LL.settings.receiptSettingsSaved());
    } catch (error) {
        console.error('Failed to save receipt settings:', error);
        toast.error(String(error));
    } finally {
        savingReceiptSettings = false;
    }
}
```

### 4.5 Add section HTML (after Company Settings, before Backups)

```svelte
<!-- Receipt Scanning Section -->
<section class="settings-section" id="receipt-scanning">
    <h2>{$LL.settings.receiptScanningSection()}</h2>
    <div class="section-content">
        <div class="form-group">
            <label for="gemini-api-key">{$LL.settings.geminiApiKey()}</label>
            <div class="input-with-toggle">
                {#if showApiKey}
                    <input
                        type="text"
                        id="gemini-api-key"
                        bind:value={geminiApiKey}
                        placeholder={$LL.settings.geminiApiKeyPlaceholder()}
                    />
                {:else}
                    <input
                        type="password"
                        id="gemini-api-key"
                        bind:value={geminiApiKey}
                        placeholder={$LL.settings.geminiApiKeyPlaceholder()}
                    />
                {/if}
                <button
                    type="button"
                    class="toggle-visibility"
                    on:click={() => showApiKey = !showApiKey}
                    title={showApiKey ? $LL.settings.hideApiKey() : $LL.settings.showApiKey()}
                >
                    {showApiKey ? '🙈' : '👁'}
                </button>
            </div>
            <small class="hint">{$LL.settings.geminiApiKeyHint()}</small>
        </div>

        <div class="form-group">
            <label for="receipts-folder">{$LL.settings.receiptsFolder()}</label>
            <div class="input-with-button">
                <input
                    type="text"
                    id="receipts-folder"
                    bind:value={receiptsFolderPath}
                    placeholder={$LL.settings.receiptsFolderPlaceholder()}
                />
                <button class="button-small" on:click={handleBrowseFolder}>
                    {$LL.settings.browseFolder()}
                </button>
            </div>
            <small class="hint">{$LL.settings.receiptsFolderHint()}</small>
        </div>

        <button class="button" on:click={handleSaveReceiptSettings} disabled={savingReceiptSettings}>
            {$LL.settings.saveSettings()}
        </button>
    </div>
</section>
```

### 4.6 Add CSS styles

```css
.input-with-toggle {
    display: flex;
    gap: 0.5rem;
}

.input-with-toggle input {
    flex: 1;
}

.toggle-visibility {
    padding: 0.75rem;
    background: var(--btn-secondary-bg);
    border: 1px solid var(--border-input);
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
}

.toggle-visibility:hover {
    background: var(--btn-secondary-hover);
}

.input-with-button {
    display: flex;
    gap: 0.5rem;
}

.input-with-button input {
    flex: 1;
}
```

---

## Task 5: Doklady Page - Simplified Notice

**File:** `src/routes/doklady/+page.svelte`

### 5.1 Add navigation import

```typescript
import { goto } from '$app/navigation';
```

### 5.2 Replace config-warning block

Replace the entire `{#if !isConfigured}` block with:

```svelte
{#if !isConfigured}
    <div class="config-notice">
        <div class="notice-title">⚠ {$LL.receipts.notConfiguredTitle()}</div>
        <p>{$LL.receipts.notConfiguredDescription()}</p>
        <ul>
            <li>{$LL.receipts.notConfiguredApiKey()}</li>
            <li>{$LL.receipts.notConfiguredFolder()}</li>
        </ul>
        <button class="button" on:click={() => goto('/settings#receipt-scanning')}>
            {$LL.receipts.goToSettings()}
        </button>
    </div>
{/if}
```

### 5.3 Update CSS

Replace `.config-warning` styles with:

```css
.config-notice {
    background: var(--warning-bg);
    border: 1px solid var(--warning-border);
    padding: 1.5rem;
    border-radius: 8px;
    margin-bottom: 1.5rem;
}

.config-notice .notice-title {
    font-weight: 600;
    font-size: 1.1rem;
    color: var(--text-primary);
    margin-bottom: 0.75rem;
}

.config-notice p {
    margin: 0.5rem 0;
    color: var(--text-primary);
}

.config-notice ul {
    margin: 0.75rem 0;
    padding-left: 1.5rem;
}

.config-notice li {
    color: var(--text-secondary);
    margin: 0.25rem 0;
}

.config-notice .button {
    margin-top: 1rem;
}
```

### 5.4 Remove unused code

- Remove `configFolderPath` state variable
- Remove `appDataDir` import and usage
- Remove old `.config-warning`, `.config-sample`, `.config-note`, `.config-path-btn` styles

---

## Task 6: Testing

1. Run backend tests: `cd src-tauri && cargo test`
2. Manual testing:
   - Open settings → verify new section appears
   - Enter API key → toggle visibility works
   - Browse folder → native dialog opens
   - Save → values persist after app restart
   - Navigate to doklady when not configured → notice appears
   - Click "Prejsť do nastavení" → navigates and scrolls to section

---

## Task 7: Changelog

Run `/changelog` to update CHANGELOG.md with:
- feat: add UI for editing receipt scanning settings (API key + folder picker)
