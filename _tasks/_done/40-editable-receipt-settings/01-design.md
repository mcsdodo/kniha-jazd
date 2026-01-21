# Editable Receipt Settings via UI

## Overview

Make `gemini_api_key` and `receipts_folder_path` in `local.settings.json` editable via the UI instead of requiring manual JSON editing.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Storage | `local.settings.json` | Already used for theme, auto-update; keeps machine-specific settings separate from DB |
| Folder picker | Text input + Browse button | Flexible: native dialog + manual entry for power users |
| API key input | Password with show/hide toggle | Balances security (shoulder surfing) with usability (verify paste) |
| Not-configured notice | Brief explanation + settings link | Helps users understand requirements without being overly technical |
| Settings placement | After Company Settings, before Backups | Logical grouping with other configuration sections |

## Components

### Backend (Rust)

**New commands in `commands.rs`:**
```rust
#[tauri::command]
pub fn set_gemini_api_key(app_handle: AppHandle, api_key: String) -> Result<(), String>

#[tauri::command]
pub fn set_receipts_folder_path(app_handle: AppHandle, path: String) -> Result<(), String>
```

Pattern: Load `LocalSettings` → modify field → write back to JSON file.

Validation:
- `set_gemini_api_key`: Allow empty string to clear
- `set_receipts_folder_path`: Validate path exists and is a directory

### Frontend API

**New functions in `api.ts`:**
```typescript
export async function setGeminiApiKey(apiKey: string): Promise<void>
export async function setReceiptsFolderPath(path: string): Promise<void>
```

### Settings Page UI

New section "Skenovanie dokladov" with:
- Password input + eye toggle for API key
- Text input + "Vybrať" button for folder (uses `@tauri-apps/plugin-dialog`)
- Save button with toast feedback
- Scroll-to-section support via URL hash `#receipt-scanning`

### Doklady Page

Simplified not-configured state:
- Warning icon + title
- Bullet list: API key requirement, folder requirement
- "Prejsť do nastavení" button → `/settings#receipt-scanning`

## Files to Modify

1. `src-tauri/src/commands.rs` — new commands
2. `src-tauri/src/lib.rs` — register commands
3. `src/lib/api.ts` — new API functions
4. `src/routes/settings/+page.svelte` — new section + scroll handling
5. `src/routes/doklady/+page.svelte` — simplified not-configured state
6. `src/lib/i18n/sk/index.ts` — Slovak translations
7. `src/lib/i18n/en/index.ts` — English translations

## i18n Keys

**Slovak (primary):**
- `receiptScanningSection`: 'Skenovanie dokladov'
- `geminiApiKey`: 'Gemini API kľúč'
- `geminiApiKeyHint`: 'Potrebný pre rozpoznávanie textu z blokov'
- `receiptsFolder`: 'Priečinok s dokladmi'
- `receiptsFolderHint`: 'Štruktúra: priečinok/ROK/súbory'
- `browseFolder`: 'Vybrať'
- `showApiKey`: 'Zobraziť' / `hideApiKey`: 'Skryť'
- `notConfiguredTitle`: 'Skenovanie dokladov nie je nakonfigurované'
- `goToSettings`: 'Prejsť do nastavení'
