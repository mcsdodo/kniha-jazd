# Testing Guidelines

Lessons learned from debugging flaky integration tests. Follow these patterns to avoid common pitfalls.

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
