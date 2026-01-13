**Date:** 2026-01-13
**Subject:** Implementation Plan - Dead Code Cleanup
**Status:** Planning

---

# Phase 1: Remove Suggestion Feature Dead Code

## Context

In v0.12.0, the compensation suggestion feature was simplified:
- **Before:** App suggested specific trips (with route matching, random margins)
- **After:** App only shows "you need X km to compensate"

The suggestion code was never removed. This phase removes it.

## Files to Modify

### 1. `src-tauri/src/suggestions.rs`

**Current state:** Contains suggestion logic that's never called

**Action:** Keep only the module declaration for tests, remove all functions

```rust
// BEFORE (86 lines)
pub struct CompensationSuggestion { ... }
pub fn generate_target_margin() -> f64 { ... }
pub fn find_matching_route(...) -> Option<&Route> { ... }
pub fn build_compensation_suggestion(...) -> CompensationSuggestion { ... }

// AFTER (~5 lines)
//! Compensation trip suggestions
//!
//! Note: Auto-suggestion feature removed in v0.12.0.
//! App now shows "you need X km" without specific trip suggestions.

// Keep test module reference if tests exist for other functions
```

**Checklist:**
- [ ] Delete `CompensationSuggestion` struct
- [ ] Delete `generate_target_margin()`
- [ ] Delete `find_matching_route()`
- [ ] Delete `build_compensation_suggestion()`
- [ ] Add comment explaining why module is minimal
- [ ] Keep `use crate::models::Route;` only if needed elsewhere

### 2. `src-tauri/src/suggestions_tests.rs`

**Action:** Delete or significantly reduce

**Checklist:**
- [ ] Delete tests for removed functions
- [ ] Keep file only if other tests remain
- [ ] If empty, delete file and remove `#[path]` attribute from suggestions.rs

### 3. `src-tauri/src/commands.rs`

**Current state:** Has `get_compensation_suggestion` command (lines 413-431)

**Action:** Remove the command

**Checklist:**
- [ ] Delete `get_compensation_suggestion` function
- [ ] Remove `use crate::suggestions::{build_compensation_suggestion, CompensationSuggestion};`
- [ ] Keep `use crate::suggestions` only if other items needed

### 4. `src-tauri/src/lib.rs`

**Current state:** Registers the command (line 62)

**Action:** Remove registration

**Checklist:**
- [ ] Remove `commands::get_compensation_suggestion` from `invoke_handler`

### 5. `src/lib/api.ts`

**Current state:** Has `getCompensationSuggestion` function (lines 181-186)

**Action:** Remove the function

**Checklist:**
- [ ] Delete `getCompensationSuggestion` function
- [ ] Remove `CompensationSuggestion` from type imports (line 4)

### 6. `src/lib/types.ts`

**Current state:** Has `CompensationSuggestion` interface (line 62)

**Action:** Remove the interface

**Checklist:**
- [ ] Delete `CompensationSuggestion` interface

### 7. `src/lib/i18n/` (sk and en)

**Check:** Are there unused translation keys for suggestions?

**Checklist:**
- [ ] Check `searchingSuggestion` key - still used?
- [ ] Check `bufferNote` key - still used?
- [ ] Remove unused keys if any

## Verification

After changes:

```bash
# Backend compiles without suggestion warnings
cd src-tauri && cargo check 2>&1 | grep -i suggestion

# Frontend type-checks
npm run check

# Tests pass (fewer tests expected)
cd src-tauri && cargo test

# App runs
npm run tauri dev
```

## Rollback

If issues found:
- All changes are deletions
- Git revert is straightforward
- No data migration involved

---

# Phase 2-4: Subsequent Phases

Will be planned after Phase 1 review and completion.

- **Phase 2:** Suppress EV/Route scaffolding
- **Phase 3:** Fix truly dead code
- **Phase 4:** Fix Svelte warnings
