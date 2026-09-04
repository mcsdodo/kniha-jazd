---
paths:
  - "tests/integration/**/*.ts"
  - "tests/integration/**/*.js"
---

# Integration Test Rules

Lessons learned from debugging flaky integration tests. Follow these patterns to avoid common pitfalls.

## Purpose

**Integration Tests (WebdriverIO + Chrome) - UI flow verification (152 tests):**
- `tests/integration/` - Full app E2E tests via WebDriver protocol, driving a real
  browser against the `kniha-jazd-web` HTTP server
- **Purpose**: Verify UI correctly invokes backend and displays results
- **NOT for**: Re-testing calculation logic (that's backend's job - see `.claude/rules/rust-backend.md`)
- **Tiered execution**: Tier 1 (`tier1` + `existing`, 48 tests) for quick checks, all
  tiers on CI; the `env` suite runs separately because its fixture env vars pin
  settings app-wide
- DB seeding via `POST /api/rpc` (no direct DB access)
- CI: Linux only - the Docker image is built once, then Chrome drives it

**Two ways to run** ([wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts)):

| Mode | Trigger | Server | Port |
|------|---------|--------|------|
| Spawned | default | WDIO launches `src-tauri/target/debug/kniha-jazd-web` with a temp `KNIHA_JAZD_DATA_DIR` and `STATIC_DIR=build/` | 3457 |
| External | `WDIO_EXTERNAL_SERVER=1` | already-running container | 3456 |

Spawned mode therefore needs both artifacts built first:

```bash
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
```

### Start the external container the way CI does

CI runs the container with `--network=host`. That is not cosmetic: several specs
start a mock HTTP server in the **test process**, bound to `127.0.0.1`, and hand the
backend its URL over RPC. The backend then has to reach back out to the host.

- On Linux (CI) `--network=host` puts the container in the host's network namespace,
  so `127.0.0.1` is the same stack and the callback works.
- On Docker Desktop for Windows/macOS it does **not**, so developers publish a port
  (`-p 3456:3456`) instead — and then the container's `127.0.0.1` is the container
  itself. The mock server is unreachable.

The symptom is a spec failing on a connection the *backend* could not make, which
reads exactly like an application bug. `paperless-integration.spec.ts` is the one
that bites: it fails locally under published ports and passes in CI, every time.

If a Docker-mode spec fails on an unreachable host service, check this before
debugging the code. Two ways out: run that spec in spawned mode (the backend is a
host process, so loopback just works), or accept the local failure and rely on CI —
but say which you did, rather than reporting the suite as green.

**Remember:** Integration tests = "Does the UI work?"

## WebDriverIO Integration Tests

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
- **Browser versions:** the Chrome build on the runner may differ from yours
- **Server mode:** CI runs against the container (`WDIO_EXTERNAL_SERVER=1`) on a
  bind-mounted `/data`, locally you get a fresh temp data dir per run
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

**Problem:** Test isolation depends on the data directory the server was started
with. Spawned mode points `KNIHA_JAZD_DATA_DIR` and `DATABASE_PATH` at a fresh temp
folder; Docker mode uses the bind-mounted `/data`. A command that resolves its own
path instead of using the server's data directory will read and write somewhere the
test never looks.

**Solution:** Resolve receipts/backups/DB paths from the data directory the server
was configured with, never from a hardcoded or platform-derived location.

**Lesson:** When adding new commands that read/write to app data, grep for existing
patterns and use the same helper.

## Settings Pinned by the Environment

WebdriverIO auto-loads the repo's `.env`. A real `PAPERLESS_API_TOKEN` or
`GEMINI_API_KEY` there would pin those settings in the spawned server and make the
setter guards reject writes, failing specs with "... is managed by the ...
environment variable". `wdio.server.conf.ts` blanks every overridable variable
(`SCRUBBED_ENV`) for normal runs; the `env` suite (`WDIO_ENV_PINNED=1`,
`npm run test:integration:docker:env`) deliberately sets them instead.

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

## Waiting for the Server

**Problem:** The suite starts before the backend is listening, and the first
navigation fails.

**Solution:** `wdio.server.conf.ts` polls `GET /health` for up to 30s in `onPrepare`
before any spec runs - in both spawned and external mode. If a spec still races the
server, the fix belongs in that poll, not in a `browser.pause()` inside the spec.
