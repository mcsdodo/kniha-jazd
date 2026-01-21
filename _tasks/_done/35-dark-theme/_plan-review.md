# Plan Review: Dark Theme

**Reviewed:** 2026-01-12
**Plan:** 03-plan.md
**Status:** Approved (revisions applied)

## Summary

The plan is well-structured with clear tasks and verification steps. However, there are critical issues with the state management pattern that could cause bugs, plus a missing component that needs theme migration.

## Findings

### Critical

- [x] **Task 2: AppState pattern doesn't exist** - The plan references `state: State<'_, AppState>` with `state.local_settings.lock()`, but the actual codebase doesn't use an `AppState` struct with `local_settings`. The `LocalSettings` struct is loaded once in `lib.rs` setup but not managed via Tauri state. The commands need to either: (a) load/save directly from file each time, or (b) add proper `AppState` management to `lib.rs`. This will cause compilation failures as-written.

- [x] **Task 5: Theme store listener leak** - The `init()` function adds an event listener to `window.matchMedia` but never removes it. Unlike Svelte store subscriptions, this listener persists forever. While not immediately breaking, it's incorrect for a store pattern and could cause issues if `init()` is called multiple times.

### Important

- [x] **Task 1: Test location incorrect** - Plan says add test to `src-tauri/src/settings.rs`, but existing tests are already in the same file (lines 30-68). This is correct placement, but the task description says "Add to `src-tauri/src/settings.rs` tests" which could be interpreted as a separate file. Should clarify: "Add to the existing `#[cfg(test)]` module in `settings.rs`".

- [x] **Task 12: Missing component** - `ReceiptIndicator.svelte` is not listed in Task 12 but exists in `src/lib/components/` and likely has hardcoded colors that need migration.

- [x] **Task 8: Subscription memory leak** - Settings page subscribes to `themeStore` inside async IIFE in `onMount` but never unsubscribes. Should use returned unsubscribe function like the existing `localeStore` pattern (see line 77 of settings page).

- [x] **Task 6: onMount ordering concern** - Plan says to add `await themeStore.init()` "early, before other async operations" but doesn't specify exact location. The existing `onMount` starts with `localeStore.init()` synchronously. Theme init should come after locale init but before the async vehicle loading to prevent flash of wrong theme.

- [x] **i18n location ambiguity** - Task 7 says "Add to `settings` section" but doesn't specify exact line numbers. The `settings` object in sk/index.ts ends at line 207. New keys should be added before the closing brace, after `language: 'Jazyk aplikacie'`.

### Minor

- [x] **Task 4: CSS variable naming** - `--text-on-header-muted` uses different naming convention than design doc (`rgba(255, 255, 255, 0.7)` vs explicit variable). This is fine but should be consistent - design doc should be updated if variable name changes.

- [x] **Task 9: Incomplete migration list** - The layout has more hardcoded colors than listed. For example, line 267 has `rgba(255, 255, 255, 0.1)` for hover states. Plan says "keep" but doesn't explain why (transparency colors on header work in both themes). A brief note would help implementer understand the decision.

- [x] **Verification gaps** - Most tasks have basic verification but Task 12 (migrate all components) lacks specific checklist. Suggest: "Verify each component by toggling theme and checking: backgrounds, text colors, borders, button states, hover effects".

- [x] **Commit message consistency** - Tasks 9-13 use `feat()` prefix but they're style migrations, not new features. Consider `style()` or `refactor()` prefix for consistency with conventional commits.

## Recommendation

~~Fix the critical `AppState` issue before implementation - this will cause compile errors. The simplest fix is to make the theme commands load/save directly from file without state management (similar to how `LocalSettings::load` already works). Also add `ReceiptIndicator.svelte` to Task 12's component list.~~

## Resolution

**Resolved:** 2026-01-12

All 11 findings addressed in `03-plan.md`:

| Category | Findings | Status |
|----------|----------|--------|
| Critical | 2 | ✅ All fixed |
| Important | 5 | ✅ All fixed |
| Minor | 4 | ✅ All fixed |

**Key changes:**
- Task 2: Replaced `AppState` pattern with direct `LocalSettings::load()` calls
- Task 5: Added `destroy()` method and cleanup tracking for `matchMedia` listener
- Task 8: Added `onDestroy` with proper unsubscribe pattern
- Task 12: Added `ReceiptIndicator.svelte` and detailed verification checklist
- Tasks 9-13: Changed commit prefix from `feat()` to `style()` for migrations

Suggested fix for Task 2:

```rust
#[tauri::command]
pub fn get_theme_preference(app_handle: tauri::AppHandle) -> Result<String, String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_data_dir);
    Ok(settings.theme.unwrap_or_else(|| "system".to_string()))
}

#[tauri::command]
pub fn set_theme_preference(app_handle: tauri::AppHandle, theme: String) -> Result<(), String> {
    // Validate
    if !["system", "light", "dark"].contains(&theme.as_str()) {
        return Err(format!("Invalid theme: {}. Must be system, light, or dark", theme));
    }

    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut settings = LocalSettings::load(&app_data_dir);
    settings.theme = Some(theme);

    // Save
    let settings_path = app_data_dir.join("local.settings.json");
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&settings_path, json).map_err(|e| e.to_string())?;

    Ok(())
}
```

This requires adding a `save()` method to `LocalSettings` or implementing save inline as shown.
