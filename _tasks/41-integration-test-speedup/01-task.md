# Task: Speed Up Integration Tests via IPC Database Reset

## Problem

Integration tests have ~4-5 seconds overhead **per test** due to:
1. 800ms pause for pending operations
2. ~500ms file deletion (with retry logic for locked SQLite DB)
3. ~2-3s `browser.refresh()` to reload the app
4. ~1s wait for DOM + Tauri IPC bridge readiness

For 39 Tier 1 tests, this is **~195 seconds** of pure overhead (vs ~6 minutes actual test execution).

## Current Approach

```
beforeTest:
  1. Pause 800ms
  2. Delete kniha-jazd.db file (with 3 retries)
  3. Delete WAL/SHM journal files
  4. browser.refresh()
  5. Wait for DOM (h1 visible)
  6. Wait for Tauri IPC bridge
```

## Proposed Solution

Replace file deletion + refresh with:
1. **Tauri IPC command** (`reset_test_database`) that truncates all tables
2. **JavaScript function** that resets all Svelte stores
3. **Skip `browser.refresh()`** - keep app running, just clear state

```
beforeTest (optimized):
  1. Invoke 'reset_test_database' via IPC (~50ms)
  2. Call resetStores() in browser (~50ms)
  3. Set locale in localStorage
  4. Pause 200ms for reactivity
```

## Expected Savings

| Test Count | Current Overhead | Optimized Overhead | Savings |
|------------|------------------|-------------------|---------|
| Tier 1 (39) | ~195s | ~12s | **~183s (~3 min)** |
| All (103) | ~515s | ~31s | **~484s (~8 min)** |

## Scope

- **In scope:** Test infrastructure changes, new Tauri command (test-only), store reset mechanism
- **Out of scope:** Production code changes (except guarded test helpers)

## Acceptance Criteria

- [ ] Integration tests pass without `browser.refresh()` between tests
- [ ] Test isolation maintained (no state leakage between tests)
- [ ] At least 50% reduction in per-test overhead
- [ ] Test-only code guarded by env var (not in production builds)
