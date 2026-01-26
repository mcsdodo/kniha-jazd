# Design: Configurable Date Prefill

## UI Component

**Segmented Toggle** placed next to "Nový záznam" button in trip grid header:

```
[+ Nový záznam]    [ +1 | Dnes ]
                      ↑ active side highlighted
```

- Click either side to switch modes
- Active option has accent background (matches app theme)
- Inactive option is subtle/muted
- Tooltip on hover: "Predvyplnený dátum pre nový záznam"

## Component Architecture

**New reusable component:** `src/lib/components/SegmentedToggle.svelte`

```svelte
<SegmentedToggle
  options={[
    { value: 'previous', label: '+1' },
    { value: 'today', label: 'Dnes' }
  ]}
  value={datePrefillMode}
  on:change={(e) => handleDatePrefillChange(e.detail)}
  size="small"
/>
```

**Props:**
- `options` - array of `{ value: string, label: string }`
- `value` - currently selected value
- `size` (optional) - `'small' | 'default'` for different contexts

**Events:**
- `change` - dispatches selected value when user clicks

## Data Persistence

### Rust Enum (`src-tauri/src/settings.rs`)

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DatePrefillMode {
    #[default]
    Previous,  // serializes as "previous"
    Today,     // serializes as "today"
}
```

### LocalSettings Extension

```rust
pub struct LocalSettings {
    // ... existing fields ...
    pub date_prefill_mode: Option<DatePrefillMode>,
}
```

### Tauri Commands (`src-tauri/src/commands.rs`)

```rust
#[tauri::command]
pub fn get_date_prefill_mode(app_handle: tauri::AppHandle) -> Result<DatePrefillMode, String>

#[tauri::command]
pub fn set_date_prefill_mode(app_handle: tauri::AppHandle, mode: DatePrefillMode) -> Result<(), String>
```

Note: Uses `AppHandle` (not `AppState`) to access app data directory for LocalSettings. Follows existing pattern from `get_theme_preference`/`set_theme_preference`.

## TypeScript Types

**In `src/lib/types.ts`:**

```typescript
export const DatePrefillMode = {
    Previous: 'previous',
    Today: 'today',
} as const;

export type DatePrefillMode = typeof DatePrefillMode[keyof typeof DatePrefillMode];
```

## Frontend Integration

**TripGrid.svelte changes:**

1. Load setting on mount
2. Add toggle next to "Nový záznam" button
3. Update `defaultNewDate` reactive block to respect mode
4. Save on toggle change (immediate, no debounce needed)

## Translations

**Slovak (`sk/index.ts`):**
```typescript
tripGrid: {
    datePrefillPrevious: '+1',
    datePrefillToday: 'Dnes',
    datePrefillTooltip: 'Predvyplnený dátum pre nový záznam',
}
```

**English (`en/index.ts`):**
```typescript
tripGrid: {
    datePrefillPrevious: '+1',
    datePrefillToday: 'Today',
    datePrefillTooltip: 'Prefilled date for new entry',
}
```

## Edge Cases

1. **No trips exist:** Both modes return today's date (no "previous" to add to)
2. **Insert between trips:** Uses existing behavior (date of trip below), not affected by this setting
3. **Year change:** Setting is global, not per-year
