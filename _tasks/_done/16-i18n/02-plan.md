# Implementation Plan: i18n

## Phase 1: Setup typesafe-i18n

- [ ] Install typesafe-i18n: `npm install typesafe-i18n`
- [ ] Run init: `npx typesafe-i18n --setup-auto`
- [ ] Configure `.typesafe-i18n.json` for Slovak as base language
- [ ] Create initial `src/lib/i18n/sk/index.ts` with empty structure
- [ ] Verify generator works: `npm run typesafe-i18n`

## Phase 2: Extract strings from components

Extract all hardcoded Slovak text and organize by feature:

### Common strings
- [ ] Buttons: Uložiť, Zrušiť, Vymazať, Pridať, Upraviť
- [ ] States: Načítavam..., Exportujem...
- [ ] Units: km, L, L/100km, €

### Pages
- [ ] `+page.svelte` (home) - vehicle info, stats labels
- [ ] `+layout.svelte` - nav items, year picker
- [ ] `settings/+page.svelte` - all settings labels and buttons
- [ ] `doklady/+page.svelte` - receipts page labels

### Components
- [ ] `TripGrid.svelte` - column headers, tooltips
- [ ] `TripRow.svelte` - row labels, actions
- [ ] `CompensationBanner.svelte` - warning messages
- [ ] `Autocomplete.svelte` - placeholder text
- [ ] `ReceiptPicker.svelte` - receipt labels
- [ ] `TripSelectorModal.svelte` - modal text

### Stores (toast/confirm messages)
- [ ] `toast.ts` - success/error messages
- [ ] `confirm.ts` - confirmation dialogs

## Phase 3: Create English translations

- [ ] Create `src/lib/i18n/en/index.ts`
- [ ] Translate all keys from Slovak to English
- [ ] Verify no missing keys (TypeScript will catch this)

## Phase 4: Language persistence & detection

- [ ] Add `get_setting` / `set_setting` Tauri commands (if not exist)
- [ ] Create `src/lib/stores/locale.ts` store
- [ ] Add language detection in `+layout.svelte` on mount
- [ ] Add language selector dropdown to Settings page
- [ ] Wire up save to settings table on change

## Phase 5: Backend error keys

- [ ] Audit Rust code for user-facing error messages
- [ ] Replace Slovak text with error keys (e.g., `error.trip.invalid_date`)
- [ ] Add `errors` section to both translation files
- [ ] Update frontend error handlers to use `$LL.errors[key]()`

## Phase 6: PDF export translations

- [ ] Define `ExportLabels` struct in `export.rs`
- [ ] Update `open_export_preview` command to accept labels parameter
- [ ] Add `export` section to translations with all PDF text
- [ ] Update frontend to pass translated labels when calling export

## Testing

- [ ] Verify all pages render correctly in Slovak
- [ ] Switch to English, verify all text updates
- [ ] Restart app, verify language preference persists
- [ ] Test PDF export in both languages
- [ ] Verify error messages appear translated

## Files to modify

**New files:**
- `src/lib/i18n/sk/index.ts`
- `src/lib/i18n/en/index.ts`
- `src/lib/i18n/i18n-types.ts` (generated)
- `src/lib/i18n/i18n-util.ts` (generated)
- `src/lib/stores/locale.ts`
- `.typesafe-i18n.json`

**Modified files:**
- `src/routes/+layout.svelte` - language init
- `src/routes/+page.svelte` - use $LL
- `src/routes/settings/+page.svelte` - language selector + use $LL
- `src/routes/doklady/+page.svelte` - use $LL
- All components in `src/lib/components/` - use $LL
- `src/lib/stores/toast.ts` - use $LL for messages
- `src/lib/stores/confirm.ts` - use $LL for messages
- `src/lib/api.ts` - pass labels to export
- `src-tauri/src/export.rs` - accept labels struct
- `src-tauri/src/commands.rs` - error keys
- `src-tauri/src/db.rs` - get_setting/set_setting if needed
