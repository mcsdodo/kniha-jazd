---
globs:
  - "tests/integration/**/*.ts"
  - "tests/integration/**/*.js"
---

# Integration Test Rules

Lessons learned from debugging flaky integration tests. Follow these patterns to avoid common pitfalls.

## Purpose

**Integration Tests (WebdriverIO + tauri-driver) - UI flow verification (61 tests):**
- `tests/integration/` - Full app E2E tests via WebDriver protocol
- **Purpose**: Verify UI correctly invokes backend and displays results
- **NOT for**: Re-testing calculation logic (that's backend's job - see `.claude/rules/rust-backend.md`)
- **Tiered execution**: Tier 1 (39 tests) for PRs, all tiers for main
- Runs against debug build of Tauri app
- DB seeding via Tauri IPC (no direct DB access)
- CI: Windows only (tauri-driver limitation)

**Remember:** Integration tests = "Does the UI work?"

## WebDriverIO + Tauri Integration Tests

### Date Inputs - Use Atomic Setting

**Problem:** `setValue()` doesn't work reliably with `<input type="date">` elements. The browser may auto-format/validate dates differently, resulting in wrong values (e.g., "2026-01-02" instead of "2026-03-15").

**Solution:** Use `browser.execute()` for atomic value setting:

```typescript
// ❌ BAD - unreliable with date inputs
await dateInput.setValue(`${year}-03-15`);

// ✅ GOOD - atomic setting with proper events
await browser.execute((sel: string, newValue: string) => {
  const input = document.querySelector(sel) as HTMLInputElement;
  if (input) {
    input.value = newValue;
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new Event('change', { bubbles: true }));
  }
}, '[data-testid="trip-date"]', `${year}-03-15`);
```

### Number Inputs with Auto-Calculation

**Problem:** `setValue()` types characters one by one, firing multiple input events. If the field triggers auto-calculation (like KM → ODO), intermediate values corrupt the result.

**Solution:** Same atomic pattern:

```typescript
// ❌ BAD - fires multiple input events
await distanceInput.setValue('50');

// ✅ GOOD - single atomic update
await browser.execute((sel: string, newValue: string) => {
  const input = document.querySelector(sel) as HTMLInputElement;
  if (input) {
    input.value = newValue;
    input.dispatchEvent(new Event('input', { bubbles: true }));
  }
}, '[data-testid="trip-distance"]', '50');
```

### Verify Field Values Before Submission

**Problem:** Tests can fail after submission without knowing which field was wrong.

**Solution:** Add assertions before triggering save:

```typescript
// Verify critical fields before submission
const distanceValue = await distanceInput.getValue();
const odoValue = await odoInput.getValue();
expect(distanceValue).toBe('50');
expect(odoValue).toBe('50150');

// Then submit
await browser.keys('Enter');
```

### Check for Error Toasts

**Problem:** Async save operations can fail silently if the UI updates before the error is visible.

**Solution:** Explicitly check for error toasts:

```typescript
await browser.pause(700); // Wait for save to complete

const toastError = await $('.toast-error');
expect(await toastError.isExisting()).toBe(false);
```

### Local vs CI Differences

Tests may pass locally but fail in CI due to:
- **Browser versions:** WebView2 versions differ between local and CI
- **Timing:** CI runners may be slower
- **Screen resolution:** Can affect click coordinates

**Mitigation strategies:**
1. Use explicit waits instead of fixed pauses where possible
2. Use `waitForDisplayed()` before interacting with elements
3. Prefer keyboard navigation (`Tab`, `Enter`) over clicks for form submission
4. Use atomic value setting instead of `setValue()`

## Test Structure Best Practices

### Seed Data Isolation

Each test should seed its own data to avoid interference:

```typescript
it('should do something', async () => {
  // Create isolated test data
  const vehicle = await seedVehicle({ name: 'Test Vehicle', ... });
  await seedTrip({ vehicleId: vehicle.id, ... });
  await setActiveVehicle(vehicle.id);

  // Now test...
});
```

### Wait for UI State

After navigation or data changes, wait for the expected UI state:

```typescript
await navigateTo('trips');
await waitForTripGrid();
await browser.pause(500); // Allow Svelte reactivity to settle
```

## Debugging Flaky Tests

1. **Add diagnostic logging:** `console.log()` values to CI output
2. **Check ALL field values:** The failing field might not be the one you expect
3. **Look for async timing:** UI might update before async operation completes
4. **Compare with passing tests:** What patterns do they use that you're missing?

## Environment Variable Consistency

**Problem:** Test isolation uses `KNIHA_JAZD_DATA_DIR` env var to point to a temp directory. But if some commands use `get_app_data_dir()` (respects env var) and others use `app_handle.path().app_data_dir()` (ignores env var), data gets written/read from different locations.

**Solution:** Always use the same helper function for resolving paths:

```rust
// ❌ BAD - ignores KNIHA_JAZD_DATA_DIR
let app_data_dir = app_handle.path().app_data_dir()?;

// ✅ GOOD - respects env var for test isolation
let app_data_dir = get_app_data_dir(&app_handle)?;
```

**Lesson:** When adding new commands that read/write to app data, grep for existing patterns and use the same helper.

## SvelteKit Component Caching

**Problem:** Navigating to the same route doesn't remount the component. `onMount` only fires on first mount. If a test:
1. Is already on `/settings`
2. Saves data via IPC
3. Navigates to `/settings` again
4. Expects UI to show new data

...it will fail because `onMount` doesn't re-run.

**Solution:** Navigate away first to force a fresh mount:

```typescript
// ❌ BAD - component may be cached
await setGeminiApiKey(testApiKey);
await navigateTo('settings');
const value = await apiKeyInput.getValue(); // Empty!

// ✅ GOOD - force remount by navigating away first
await setGeminiApiKey(testApiKey);
await navigateTo('trips');      // Navigate away
await navigateTo('settings');   // Now onMount runs fresh
const value = await apiKeyInput.getValue(); // Has value!
```

## File System Sync in CI

**Problem:** `browser.pause(100)` after writing a file isn't enough. The OS may buffer writes, and the next read may see stale data.

**Solution:** Use `sync_all()` in Rust to guarantee disk flush:

```rust
// In settings.rs save():
file.write_all(json.as_bytes())?;
file.sync_all()?;  // Force flush to disk
```

## Null vs Empty String

**Problem:** Rust's `Option<String>` serializes to `null` in JSON when `None`, not `""`.

```rust
settings.gemini_api_key = if api_key.is_empty() {
    None  // Becomes null in JSON
} else {
    Some(api_key)
};
```

**Solution:** Test for `null` when checking "cleared" state:

```typescript
// ❌ BAD
expect(cleanSettings?.geminiApiKey).toBe('');

// ✅ GOOD
expect(cleanSettings?.geminiApiKey).toBeNull();
```

## WebDriver Version Matching

**Problem:** Edge WebDriver version must match WebView2 version exactly. CI may have different versions than local.

**Solution:** Dynamically detect WebView2 version from registry (see `.github/workflows/test.yml`):

```powershell
# Read actual WebView2 version from registry
$webviewVersion = (Get-ItemProperty -Path 'HKLM:\SOFTWARE\...\WebView2').pv
# Download matching Edge WebDriver
```
