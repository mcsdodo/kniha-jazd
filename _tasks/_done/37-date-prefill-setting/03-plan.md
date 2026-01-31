# Implementation Plan: Configurable Date Prefill

**Status:** Complete
**Completed:** 2026-01-31

## Phase 1: Backend (Rust)

### Step 1.1: Write Failing Backend Tests (TDD)
**File:** `src-tauri/src/settings.rs` (in existing `mod tests`)

- [ ] Test: `test_date_prefill_mode_default` - missing field returns Previous
- [ ] Test: `test_date_prefill_mode_serialization` - "today" JSON → Today variant
- [ ] Test: `test_date_prefill_mode_round_trip` - save Today, load, verify
- [ ] Verify tests fail (enum doesn't exist yet)

### Step 1.2: Add DatePrefillMode Enum
**File:** `src-tauri/src/settings.rs`

- [ ] Add `DatePrefillMode` enum with `Previous` (default) and `Today` variants
- [ ] Add `#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]`
- [ ] Add `#[serde(rename_all = "lowercase")]` for clean JSON
- [ ] Add `date_prefill_mode: Option<DatePrefillMode>` to `LocalSettings` struct
- [ ] Verify tests now pass

### Step 1.3: Add Tauri Commands
**File:** `src-tauri/src/commands.rs`

- [ ] Add `get_date_prefill_mode` command using `tauri::AppHandle` parameter
- [ ] Add `set_date_prefill_mode` command using `tauri::AppHandle` parameter
- [ ] Follow existing pattern from `get_theme_preference`/`set_theme_preference`
- [ ] Note: LocalSettings are per-machine, not database-dependent, so read-only mode doesn't apply

### Step 1.4: Register Commands
**File:** `src-tauri/src/lib.rs`

- [ ] Add `get_date_prefill_mode` to `invoke_handler`
- [ ] Add `set_date_prefill_mode` to `invoke_handler`

### Step 1.5: Run Backend Tests
- [ ] `cd src-tauri && cargo test` - verify all tests pass

## Phase 2: Frontend Types & API

### Step 2.1: Add TypeScript Type
**File:** `src/lib/types.ts`

- [ ] Add `DatePrefillMode` const object with `Previous` and `Today`
- [ ] Add `DatePrefillMode` type export

### Step 2.2: Add API Functions
**File:** `src/lib/api.ts`

- [ ] Add `getDatePrefillMode(): Promise<DatePrefillMode>`
- [ ] Add `setDatePrefillMode(mode: DatePrefillMode): Promise<void>`

## Phase 3: UI Component

### Step 3.1: Create SegmentedToggle Component
**File:** `src/lib/components/SegmentedToggle.svelte` (new)

- [ ] Create component with `options`, `value`, `size` props
- [ ] Implement click handling with `dispatch('change', value)`
- [ ] Style with CSS variables for theme support
- [ ] Support `size="small"` variant

### Step 3.2: Add Translations
**Files:** `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

- [ ] Add `tripGrid.datePrefillPrevious`
- [ ] Add `tripGrid.datePrefillToday`
- [ ] Add `tripGrid.datePrefillTooltip`

## Phase 4: Integration

### Step 4.1: Wire Up TripGrid
**File:** `src/lib/components/TripGrid.svelte`

- [ ] Import `SegmentedToggle` and `DatePrefillMode`
- [ ] Add `datePrefillMode` state variable
- [ ] Load setting in `onMount`
- [ ] Add toggle to grid header (next to "Nový záznam" button)
- [ ] Update `defaultNewDate` reactive block to check mode
- [ ] Add `handlePrefillModeChange` to save on toggle

### Step 4.2: Manual Testing
- [ ] Dev mode: toggle works, date changes appropriately
- [ ] Persist test: change to "Dnes", reload app, verify still "Dnes"
- [ ] Edge case: no trips, both modes show today

## Phase 5: Integration Test

### Step 5.1: Add E2E Test
**File:** `tests/integration/specs/tier2/date-prefill.spec.ts` (new)

- [ ] Test: toggle changes prefilled date
- [ ] Test: setting persists across app reload

### Step 5.2: Run All Tests
- [ ] `npm run test:backend` - Rust tests pass
- [ ] `npm run test:integration:tier1` - Integration tests pass

## Phase 6: Documentation

### Step 6.1: Update Changelog
- [ ] Add entry to CHANGELOG.md [Unreleased] section

## Files Changed Summary

| File | Change Type |
|------|-------------|
| `src-tauri/src/settings.rs` | Modify (enum + tests) |
| `src-tauri/src/commands.rs` | Modify (2 new commands) |
| `src-tauri/src/lib.rs` | Modify (register commands) |
| `src/lib/types.ts` | Modify (add type) |
| `src/lib/api.ts` | Modify (add functions) |
| `src/lib/components/SegmentedToggle.svelte` | **New** |
| `src/lib/components/TripGrid.svelte` | Modify (add toggle) |
| `src/lib/i18n/sk/index.ts` | Modify (add keys) |
| `src/lib/i18n/en/index.ts` | Modify (add keys) |
| `tests/integration/specs/tier2/date-prefill.spec.ts` | **New** |
| `CHANGELOG.md` | Modify |

**Total: 11 files (2 new, 9 modified)**
