# Plan: Magic String Constants Refactoring

## Status: ✅ COMPLETE (Phase 1)

**Completed:** 2026-01-31

### Summary

Phase 1 complete - constants infrastructure established:

| Component | Status | Details |
|-----------|--------|---------|
| `src/lib/constants.ts` | ✅ Created | All TypeScript constants (17 categories) |
| `src-tauri/src/constants.rs` | ✅ Created | Rust const modules (paths, date_formats, mime_types, env_vars, defaults) |
| `src-tauri/src/models.rs` | ✅ Updated | Added BackupType, AttachmentStatus, Currency, Theme enums |
| Store files updated | ✅ Done | toast.ts, theme.ts, locale.ts now use constants |
| api.ts updated | ✅ Done | ThemeMode re-exported from constants |
| db_location.rs | ✅ Done | Uses paths constants |
| settings.rs | ✅ Done | Uses paths constants |

### Migration Status

New constants are available for use. Existing code can gradually adopt them.
This avoids breaking changes while providing type-safe constants for new code.

---

## Executive Summary

| Language | Total Categories | Already Enum | Needs Refactoring | Priority Items |
|----------|------------------|--------------|-------------------|----------------|
| **Rust** | 25 | 6 | 19 | 4 high priority |
| **JS/TS** | 20 | 0 | 20 | 7 high priority |

---

## Part 1: Rust Backend Findings

### Already Properly Enumerated (No Action Needed)

| Enum | Location | Values |
|------|----------|--------|
| `VehicleType` | models.rs | Ice, Bev, Phev |
| `ReceiptStatus` | models.rs | Pending, Parsed, NeedsReview, Assigned |
| `ConfidenceLevel` | models.rs | Unknown, High, Medium, Low |
| `LockStatus` | db_location.rs | Free, Stale, Locked, Corrupted |
| `FolderStructure` | receipts.rs | Flat, YearBased, Invalid |
| `AppMode` | app_state.rs | Normal, ReadOnly |

### High Priority - Create New Enums

#### 1. BackupType
**Current:** String literals `"manual"`, `"pre-update"`
**Files:** commands.rs (lines 1687, 1690, 1697, 1708, 1764), commands_tests.rs
**Occurrences:** 15+
**Suggested Implementation:**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackupType {
    Manual,
    PreUpdate,
}
```

#### 2. AttachmentStatus
**Current:** String literals `"empty"`, `"matches"`, `"differs"`
**Files:** commands.rs (lines 2699, 2720, 2742, 2759, 2770)
**Occurrences:** 5+
**Suggested Implementation:**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttachmentStatus {
    Empty,
    Matches,
    Differs,
}
```

#### 3. Currency
**Current:** String literals `"EUR"`, `"CZK"`, `"HUF"`, `"PLN"`
**Files:** gemini.rs (line 154, 205), receipts.rs (lines 252, 268), tests
**Occurrences:** 20+
**Suggested Implementation:**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Currency {
    EUR,
    CZK,
    HUF,
    PLN,
}
```

#### 4. Theme
**Current:** String literals `"system"`, `"light"`, `"dark"`
**Files:** commands.rs (lines 3240, 3246), settings.rs (lines 102, 105, 143, 154)
**Occurrences:** 8
**Suggested Implementation:**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}
```

### Medium Priority - Extract Constants

#### 5. Date Format Strings
**Current:** `"%Y-%m-%d"`, `"%Y-%m-%d-%H%M%S"`, `"%d.%m.%Y"`
**Files:** commands.rs, export.rs, db.rs
**Occurrences:** 10+
**Suggested:**
```rust
pub mod date_formats {
    pub const ISO_DATE: &str = "%Y-%m-%d";
    pub const BACKUP_TIMESTAMP: &str = "%Y-%m-%d-%H%M%S";
    pub const DISPLAY_DATE: &str = "%d.%m.%Y";
}
```

#### 6. File Names and Extensions
**Current:** `"kniha-jazd.db"`, `"kniha-jazd.lock"`, `"backups"`, `"local.settings.json"`
**Files:** db_location.rs, commands.rs, settings.rs
**Occurrences:** 15+
**Suggested:**
```rust
pub mod paths {
    pub const DB_FILENAME: &str = "kniha-jazd.db";
    pub const LOCK_FILENAME: &str = "kniha-jazd.lock";
    pub const BACKUPS_DIR: &str = "backups";
    pub const SETTINGS_FILENAME: &str = "local.settings.json";
    pub const BACKUP_PREFIX: &str = "kniha-jazd-backup-";
}
```

#### 7. MIME Types
**Current:** `"image/jpeg"`, `"image/png"`, `"image/webp"`, `"application/pdf"`
**Files:** gemini.rs
**Occurrences:** 4
**Suggested:**
```rust
pub mod mime_types {
    pub const JPEG: &str = "image/jpeg";
    pub const PNG: &str = "image/png";
    pub const WEBP: &str = "image/webp";
    pub const PDF: &str = "application/pdf";
}
```

### Low Priority - Document Only

| Category | Examples | Notes |
|----------|----------|-------|
| Environment Variables | `KNIHA_JAZD_DATA_DIR`, `KNIHA_JAZD_MOCK_GEMINI_DIR` | Already constants in code |
| SQL Patterns | `strftime('%Y', date)` | Necessary for raw queries |
| API Endpoints | Gemini URL | Dynamic with API key |
| Serde Field Names | `generationConfig`, `responseMimeType` | JSON API contract |
| Error Messages | Slovak user-facing messages | Dynamic content |

---

## Part 2: TypeScript/Svelte Frontend Findings

### High Priority - Create Const Objects

#### 1. VehicleTypes
**Current:** `'Ice'`, `'Bev'`, `'Phev'`
**Files:** TripGrid.svelte, TripRow.svelte, VehicleModal.svelte, settings/+page.svelte
**Occurrences:** 19
**Suggested:**
```typescript
// src/lib/constants.ts
export const VEHICLE_TYPES = {
    ICE: 'Ice',
    BEV: 'Bev',
    PHEV: 'Phev',
} as const;
export type VehicleType = typeof VEHICLE_TYPES[keyof typeof VEHICLE_TYPES];
```

#### 2. ToastTypes
**Current:** `'success'`, `'error'`, `'info'`
**Files:** toast.ts, doklady/+page.svelte, settings/+page.svelte
**Occurrences:** 32+
**Suggested:**
```typescript
export const TOAST_TYPES = {
    SUCCESS: 'success',
    ERROR: 'error',
    INFO: 'info',
} as const;
export type ToastType = typeof TOAST_TYPES[keyof typeof TOAST_TYPES];
```

#### 3. ReceiptStatus
**Current:** `'Pending'`, `'Parsed'`, `'NeedsReview'`, `'Assigned'`
**Files:** doklady/+page.svelte, types.ts
**Occurrences:** 6
**Suggested:**
```typescript
export const RECEIPT_STATUS = {
    PENDING: 'Pending',
    PARSED: 'Parsed',
    NEEDS_REVIEW: 'NeedsReview',
    ASSIGNED: 'Assigned',
} as const;
```

#### 4. ReceiptFilters
**Current:** `'all'`, `'unassigned'`, `'needs_review'`, `'fuel'`, `'other'`
**Files:** doklady/+page.svelte
**Occurrences:** 13
**Suggested:**
```typescript
export const RECEIPT_FILTERS = {
    ALL: 'all',
    UNASSIGNED: 'unassigned',
    NEEDS_REVIEW: 'needs_review',
} as const;

export const RECEIPT_TYPE_FILTERS = {
    ALL: 'all',
    FUEL: 'fuel',
    OTHER: 'other',
} as const;
```

#### 5. ConfidenceLevels
**Current:** `'High'`, `'Medium'`, `'Low'`, `'Unknown'`
**Files:** doklady/+page.svelte, types.ts
**Occurrences:** 6
**Suggested:**
```typescript
export const CONFIDENCE_LEVELS = {
    HIGH: 'High',
    MEDIUM: 'Medium',
    LOW: 'Low',
    UNKNOWN: 'Unknown',
} as const;
```

#### 6. ThemeModes
**Current:** `'system'`, `'light'`, `'dark'`
**Files:** theme.ts, settings/+page.svelte, api.ts
**Occurrences:** 15
**Suggested:**
```typescript
export const THEME_MODES = {
    SYSTEM: 'system',
    LIGHT: 'light',
    DARK: 'dark',
} as const;
```

#### 7. BackupSteps
**Current:** `'pending'`, `'in-progress'`, `'done'`, `'failed'`, `'skipped'`
**Files:** update.ts, UpdateModal.svelte (DUPLICATED type definition!)
**Occurrences:** 10
**Suggested:**
```typescript
export const BACKUP_STEPS = {
    PENDING: 'pending',
    IN_PROGRESS: 'in-progress',
    DONE: 'done',
    FAILED: 'failed',
    SKIPPED: 'skipped',
} as const;
export type BackupStep = typeof BACKUP_STEPS[keyof typeof BACKUP_STEPS];
```

### Medium Priority - Extract Constants

#### 8. SortOptions
**Current:** `'manual'`, `'date'`, `'asc'`, `'desc'`
**Files:** TripGrid.svelte, +page.svelte
**Occurrences:** 13
**Suggested:**
```typescript
export const SORT_COLUMNS = {
    MANUAL: 'manual',
    DATE: 'date',
} as const;

export const SORT_DIRECTIONS = {
    ASC: 'asc',
    DESC: 'desc',
} as const;
```

#### 9. AttachmentStatus
**Current:** `'empty'`, `'matches'`, `'differs'`
**Files:** TripSelectorModal.svelte, types.ts
**Occurrences:** 4
**Suggested:**
```typescript
export const ATTACHMENT_STATUS = {
    EMPTY: 'empty',
    MATCHES: 'matches',
    DIFFERS: 'differs',
} as const;
```

#### 10. MismatchReasons
**Current:** `'date'`, `'liters'`, `'price'`, `'none'`, etc.
**Files:** TripSelectorModal.svelte, doklady/+page.svelte, types.ts
**Occurrences:** 12
**Suggested:**
```typescript
export const MISMATCH_REASONS = {
    NONE: 'none',
    DATE: 'date',
    LITERS: 'liters',
    PRICE: 'price',
    LITERS_AND_PRICE: 'liters_and_price',
    DATE_AND_LITERS: 'date_and_liters',
    DATE_AND_PRICE: 'date_and_price',
    ALL: 'all',
} as const;
```

#### 11. Currencies
**Current:** `'EUR'`, `'CZK'`, `'HUF'`, `'PLN'`
**Files:** doklady/+page.svelte, ReceiptEditModal.svelte, types.ts
**Occurrences:** 3
**Suggested:**
```typescript
export const CURRENCIES = {
    EUR: 'EUR',
    CZK: 'CZK',
    HUF: 'HUF',
    PLN: 'PLN',
} as const;
export const PRIMARY_CURRENCY = CURRENCIES.EUR;
```

#### 12. Locales
**Current:** `'sk'`, `'en'`, `'sk-SK'`, `'en-US'`
**Files:** locale.ts, various components with toLocaleString
**Occurrences:** 10
**Suggested:**
```typescript
export const LOCALES = {
    SK: 'sk',
    EN: 'en',
} as const;

export const LOCALE_CODES = {
    SK: 'sk-SK',
    EN: 'en-US',
} as const;
```

#### 13. KeyboardKeys
**Current:** `'Escape'`, `'Enter'`, `'Tab'`, `'ArrowDown'`, `'ArrowUp'`
**Files:** Multiple modal and input components
**Occurrences:** 12
**Suggested:**
```typescript
export const KEYBOARD_KEYS = {
    ESCAPE: 'Escape',
    ENTER: 'Enter',
    TAB: 'Tab',
    ARROW_DOWN: 'ArrowDown',
    ARROW_UP: 'ArrowUp',
} as const;
```

#### 14. DownloadEventTypes
**Current:** `'Started'`, `'Progress'`, `'Finished'`
**Files:** update.ts
**Occurrences:** 3
**Suggested:**
```typescript
export const DOWNLOAD_EVENTS = {
    STARTED: 'Started',
    PROGRESS: 'Progress',
    FINISHED: 'Finished',
} as const;
```

### Low Priority - Document Only

| Category | Examples | Notes |
|----------|----------|-------|
| URL Routes | `'/'`, `'/doklady'`, `'/settings'` | SvelteKit routing |
| LocalStorage Keys | `'kniha-jazd-locale'` | Already defined as constants |
| Form Input IDs | `'name'`, `'license-plate'` | HTML attributes |
| ARIA Roles | `'dialog'`, `'alert'` | Standard HTML |
| CSS Classes | `'active'`, `'warning'` | Style-specific |
| Animation Values | `'fly'`, `'duration: 200'` | Svelte transitions |

---

## Implementation Order

### Phase 1: High Priority (Breaking Changes Risk: Low)
1. Create `src/lib/constants.ts` with all JS/TS const objects
2. Create Rust enums in `models.rs` for BackupType, AttachmentStatus
3. Update all imports and usages
4. Run full test suite

### Phase 2: Medium Priority (Some File Changes)
5. Add Rust const modules for date_formats, paths, mime_types
6. Add remaining JS/TS const objects (sorts, keyboard, etc.)
7. Consolidate duplicate type definitions (BackupStep)
8. Run full test suite

### Phase 3: Low Priority (Documentation)
9. Document intentionally-kept string literals
10. Add comments explaining API contract strings
11. Update CLAUDE.md with constants guidance

---

## Files to Modify

### Rust (src-tauri/src/)
| File | Changes |
|------|---------|
| `models.rs` | Add BackupType, AttachmentStatus, Currency, Theme enums |
| `commands.rs` | Use new enums, add const modules |
| `settings.rs` | Use Theme enum |
| `receipts.rs` | Use Currency enum |
| `gemini.rs` | Use Currency enum, add MIME const |
| `db_location.rs` | Add paths const module |
| `export.rs` | Use date_formats const |

### TypeScript (src/)
| File | Changes |
|------|---------|
| `src/lib/constants.ts` | NEW FILE - all const objects |
| `src/lib/types.ts` | Import types from constants |
| `src/lib/stores/toast.ts` | Use TOAST_TYPES |
| `src/lib/stores/theme.ts` | Use THEME_MODES |
| `src/lib/stores/update.ts` | Use BACKUP_STEPS, DOWNLOAD_EVENTS |
| `src/lib/stores/locale.ts` | Use LOCALES |
| `src/lib/components/TripGrid.svelte` | Use VEHICLE_TYPES, SORT_* |
| `src/lib/components/TripRow.svelte` | Use VEHICLE_TYPES |
| `src/lib/components/UpdateModal.svelte` | Remove duplicate BackupStep type |
| `src/lib/components/TripSelectorModal.svelte` | Use ATTACHMENT_STATUS, MISMATCH_REASONS |
| `src/routes/doklady/+page.svelte` | Use RECEIPT_*, CONFIDENCE_LEVELS |
| `src/routes/settings/+page.svelte` | Use THEME_MODES, VEHICLE_TYPES |

---

## Testing Strategy

1. **No behavior changes** - Only refactor, no new functionality
2. **Run existing tests** - All 195 Rust tests + 61 integration tests must pass
3. **Type checking** - `npm run check` must pass with no errors
4. **Manual smoke test** - Verify UI still works correctly

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Serde serialization breaks | High | Test JSON output matches before/after |
| Type inference issues | Medium | Use `as const` assertion consistently |
| Import cycles | Low | Keep constants in dedicated file |
| Missed usages | Medium | Use TypeScript strict mode, Rust compiler |
