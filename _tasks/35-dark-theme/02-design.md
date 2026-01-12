**Date:** 2026-01-12
**Subject:** Dark theme design
**Status:** Planning

# Dark Theme Design

## Decisions Summary

| Aspect | Decision |
|--------|----------|
| **Trigger** | System preference + manual toggle |
| **Visual style** | Soft dark (#1e1e1e) |
| **Toggle location** | Settings page only |
| **Persistence** | `local.settings.json` (backend) |
| **Implementation** | CSS Custom Properties |

## Architecture

### New Files

**`src/lib/theme.css`** - Design tokens
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

  --border-default: #e0e0e0;
  --border-input: #d5dbdb;

  --accent-primary: #3498db;
  --accent-success: #27ae60;
  --accent-warning: #d39e00;
  --accent-danger: #c0392b;
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

  --border-default: #333333;
  --border-input: #404040;

  /* Brighter accents for dark mode readability */
  --accent-primary: #5dade2;
  --accent-success: #58d68d;
  --accent-warning: #f4d03f;
  --accent-danger: #ec7063;
}
```

**`src/lib/stores/theme.ts`** - Theme state management
```typescript
import { writable } from 'svelte/store';
import { getThemePreference, setThemePreference } from '$lib/api';

type ThemeMode = 'system' | 'light' | 'dark';

function createThemeStore() {
  const { subscribe, set } = writable<ThemeMode>('system');

  return {
    subscribe,
    init: async () => {
      const saved = await getThemePreference();
      set(saved);
      applyTheme(saved);
    },
    change: async (mode: ThemeMode) => {
      await setThemePreference(mode);
      set(mode);
      applyTheme(mode);
    }
  };
}

function applyTheme(mode: ThemeMode) {
  const isDark = mode === 'dark' ||
    (mode === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
  document.documentElement.dataset.theme = isDark ? 'dark' : 'light';
}
```

### Backend Changes

**`src-tauri/src/settings.rs`**
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalSettings {
    pub gemini_api_key: Option<String>,
    pub receipts_folder_path: Option<String>,
    pub theme: Option<String>,  // "system" | "light" | "dark"
}
```

**`src-tauri/src/commands.rs`** - New commands
```rust
#[tauri::command]
pub fn get_theme_preference(state: State<'_, AppState>) -> String {
    state.local_settings.theme.clone().unwrap_or_else(|| "system".to_string())
}

#[tauri::command]
pub fn set_theme_preference(state: State<'_, AppState>, theme: String) -> Result<(), String> {
    // Validate: must be "system", "light", or "dark"
    // Update local_settings.json
}
```

### Theme Application Flow

```
App Startup
    │
    ▼
Load from local.settings.json
    │
    ▼
If "system": detect OS preference
    │
    ▼
Apply: document.documentElement.dataset.theme = "dark" | "light"
```

### UI Changes

**Settings Page** - New "Appearance" section (after Language):
```
┌─────────────────────────────────────────────┐
│  Vzhľad (Appearance)                        │
│  ─────────────────────────────────────────  │
│                                             │
│  Téma:  ○ Podľa systému (System)           │
│         ○ Svetlá (Light)                    │
│         ○ Tmavá (Dark)                      │
│                                             │
└─────────────────────────────────────────────┘
```

**Layout initialization** (`+layout.svelte`):
1. Import `theme.css` globally
2. Call `themeStore.init()` in `onMount`
3. Listen for system preference changes

### Component Migration

Replace hardcoded colors with CSS variables:
```css
/* Before */
background: white;
color: #2c3e50;

/* After */
background: var(--bg-surface);
color: var(--text-primary);
```

**Migration priority:**
1. `+layout.svelte` (header, body)
2. `+page.svelte` (main page)
3. `settings/+page.svelte`
4. `TripGrid.svelte`, `TripRow.svelte`
5. All modals

### i18n Keys

```typescript
settings: {
  appearanceSection: 'Vzhľad',
  themeLabel: 'Téma',
  themeSystem: 'Podľa systému',
  themeLight: 'Svetlá',
  themeDark: 'Tmavá',
}
```

## Out of Scope (YAGNI)

- Animated transitions between themes
- Custom color picker
- Per-component theme overrides
- High contrast mode
