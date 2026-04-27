# Time Inference Toggle Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Make new-row start/end-time auto-fill (origin+destination → infer from last route + jitter) opt-in via a setting (default OFF), with a toast + per-row undo when it fires.

**Architecture:** Setting lives in `LocalSettings` ([local.settings.json](../../)). Backend gates the existing inference command at the boundary (in the public `_internal` function), keeping `compute_inferred_times` pure. Frontend extends the existing toast store with an optional action button; `TripRow.svelte` snapshots pre-inference values and the toast's undo callback closes over them. ADR-008 preserved.

**Tech Stack:** Rust (Tauri backend), SvelteKit + TypeScript (frontend), Vitest/WebdriverIO (integration tests), serde + serde_json (settings persistence).

---

## Task 1: Add `infer_trip_times` field to LocalSettings struct

**Files:**
- Modify: [src-tauri/core/src/settings.rs](../../src-tauri/core/src/settings.rs):27-42
- Modify: [src-tauri/core/src/settings_tests.rs](../../src-tauri/core/src/settings_tests.rs)

**Step 1: Write the failing test**

Append to [settings_tests.rs](../../src-tauri/core/src/settings_tests.rs):

```rust
// infer_trip_times tests
#[test]
fn test_infer_trip_times_default_is_none() {
    let dir = tempdir().unwrap();
    let settings = LocalSettings::load(&dir.path().to_path_buf());
    assert!(settings.infer_trip_times.is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test -p kniha-jazd-core test_infer_trip_times_default_is_none`
Expected: FAIL — compile error `no field 'infer_trip_times' on type 'LocalSettings'`

**Step 3: Add field to LocalSettings**

In [settings.rs](../../src-tauri/core/src/settings.rs), inside the `LocalSettings` struct (line 27-42), add the field after `date_prefill_mode`:

```rust
pub date_prefill_mode: Option<DatePrefillMode>, // existing line — keep
pub infer_trip_times: Option<bool>, // ADD: None = OFF (default)
pub hidden_columns: Option<Vec<String>>, // existing line — keep
```

**Step 4: Update `test_save_preserves_all_fields` to include the new field**

In [settings_tests.rs](../../src-tauri/core/src/settings_tests.rs), inside `test_save_preserves_all_fields` (line 93-125), add to the struct literal (after `date_prefill_mode`):

```rust
infer_trip_times: Some(false),
```

And add an assertion before the closing brace:

```rust
assert_eq!(loaded.infer_trip_times, Some(false));
```

**Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test -p kniha-jazd-core settings`
Expected: PASS — all settings tests green (including new `test_infer_trip_times_default_is_none`).

**Step 6: Commit**

```bash
git add src-tauri/core/src/settings.rs src-tauri/core/src/settings_tests.rs
git commit -m "feat(settings): add infer_trip_times field to LocalSettings"
```

---

## Task 2: Round-trip serialization test for `infer_trip_times`

**Files:**
- Modify: [src-tauri/core/src/settings_tests.rs](../../src-tauri/core/src/settings_tests.rs)

**Step 1: Write the failing test**

Append to [settings_tests.rs](../../src-tauri/core/src/settings_tests.rs):

```rust
#[test]
fn test_infer_trip_times_round_trip_true() {
    let dir = tempdir().unwrap();
    let settings = LocalSettings {
        infer_trip_times: Some(true),
        ..Default::default()
    };
    settings.save(&dir.path().to_path_buf()).unwrap();

    let loaded = LocalSettings::load(&dir.path().to_path_buf());
    assert_eq!(loaded.infer_trip_times, Some(true));
}

#[test]
fn test_infer_trip_times_round_trip_false() {
    let dir = tempdir().unwrap();
    let settings = LocalSettings {
        infer_trip_times: Some(false),
        ..Default::default()
    };
    settings.save(&dir.path().to_path_buf()).unwrap();

    let loaded = LocalSettings::load(&dir.path().to_path_buf());
    assert_eq!(loaded.infer_trip_times, Some(false));
}
```

**Step 2: Run tests to verify they pass**

Run: `cd src-tauri && cargo test -p kniha-jazd-core test_infer_trip_times_round_trip`
Expected: PASS — serde round-trip works (no implementation change needed; field was added in Task 1).

**Step 3: Commit**

```bash
git add src-tauri/core/src/settings_tests.rs
git commit -m "test(settings): round-trip tests for infer_trip_times"
```

---

## Task 3: Internal getter `get_infer_trip_times_internal`

**Files:**
- Modify: [src-tauri/core/src/commands_internal/settings_cmd.rs](../../src-tauri/core/src/commands_internal/settings_cmd.rs):137-157

**Step 1: Write the failing test**

Append to [settings_tests.rs](../../src-tauri/core/src/settings_tests.rs):

```rust
#[test]
fn test_get_infer_trip_times_internal_default_is_false() {
    use crate::commands_internal::settings_cmd::get_infer_trip_times_internal;
    let dir = tempdir().unwrap();
    let result = get_infer_trip_times_internal(&dir.path().to_path_buf()).unwrap();
    assert!(!result, "default must be OFF");
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test -p kniha-jazd-core test_get_infer_trip_times_internal_default`
Expected: FAIL — compile error `unresolved import 'crate::commands_internal::settings_cmd::get_infer_trip_times_internal'`.

**Step 3: Implement the internal getter**

In [settings_cmd.rs](../../src-tauri/core/src/commands_internal/settings_cmd.rs), append after the `Hidden Columns` block (after line 173):

```rust
// ============================================================================
// Time Inference Toggle
// ============================================================================

pub fn get_infer_trip_times_internal(app_dir: &Path) -> Result<bool, String> {
    let settings = LocalSettings::load(app_dir);
    // Default OFF: None and Some(false) both mean disabled.
    Ok(settings.infer_trip_times.unwrap_or(false))
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test -p kniha-jazd-core test_get_infer_trip_times_internal_default`
Expected: PASS.

**Step 5: Commit**

```bash
git add src-tauri/core/src/commands_internal/settings_cmd.rs src-tauri/core/src/settings_tests.rs
git commit -m "feat(settings): add get_infer_trip_times_internal (default OFF)"
```

---

## Task 4: Internal setter `set_infer_trip_times_internal`

**Files:**
- Modify: [src-tauri/core/src/commands_internal/settings_cmd.rs](../../src-tauri/core/src/commands_internal/settings_cmd.rs)

**Step 1: Write the failing test**

Append to [settings_tests.rs](../../src-tauri/core/src/settings_tests.rs):

```rust
#[test]
fn test_set_infer_trip_times_internal_round_trip() {
    use crate::commands_internal::settings_cmd::{
        get_infer_trip_times_internal, set_infer_trip_times_internal,
    };
    let dir = tempdir().unwrap();
    let app_dir = dir.path().to_path_buf();

    set_infer_trip_times_internal(&app_dir, true).unwrap();
    assert!(get_infer_trip_times_internal(&app_dir).unwrap());

    set_infer_trip_times_internal(&app_dir, false).unwrap();
    assert!(!get_infer_trip_times_internal(&app_dir).unwrap());
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test -p kniha-jazd-core test_set_infer_trip_times_internal_round_trip`
Expected: FAIL — compile error `unresolved import 'set_infer_trip_times_internal'`.

**Step 3: Implement the setter**

In [settings_cmd.rs](../../src-tauri/core/src/commands_internal/settings_cmd.rs), inside the `Time Inference Toggle` block from Task 3, append:

```rust
pub fn set_infer_trip_times_internal(app_dir: &Path, enabled: bool) -> Result<(), String> {
    let mut settings = LocalSettings::load(app_dir);
    settings.infer_trip_times = Some(enabled);
    settings.save(app_dir).map_err(|e| e.to_string())
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test -p kniha-jazd-core test_set_infer_trip_times_internal_round_trip`
Expected: PASS.

**Step 5: Commit**

```bash
git add src-tauri/core/src/commands_internal/settings_cmd.rs src-tauri/core/src/settings_tests.rs
git commit -m "feat(settings): add set_infer_trip_times_internal"
```

---

## Task 5: Gate `get_inferred_trip_time_for_route_internal` on the setting

The current public command always runs inference. We change its signature to accept `app_dir: &Path` and short-circuit to `Ok(None)` when the setting is OFF. The pure helpers `compute_inferred_times` and `inferred_trip_time_for_route` (the test seam taking `Jitter`) stay unchanged.

**Files:**
- Modify: [src-tauri/core/src/commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs):230-241
- Modify: [src-tauri/desktop/src/commands/trips.rs](../../src-tauri/desktop/src/commands/trips.rs):153-160 (caller)
- Modify: [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs)

**Step 1: Write three failing tests**

Locate the existing `inferred_trip_time_for_route` test in [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) (search for `inferred_trip_time_for_route` — there's at least one existing test around line 4100). Append these three tests in the same module:

```rust
#[test]
fn test_inference_command_returns_none_when_setting_unset() {
    use crate::commands_internal::trips::get_inferred_trip_time_for_route_internal;
    let (db, _temp_db) = test_db_with_completed_trip(); // helper exists in this module
    let dir = tempfile::tempdir().unwrap();
    let app_dir = dir.path().to_path_buf();
    // No infer_trip_times in local.settings.json → default OFF.

    let result = get_inferred_trip_time_for_route_internal(
        &db,
        &app_dir,
        "vehicle-1".to_string(),
        "Bratislava".to_string(),
        "Žilina".to_string(),
        "2026-04-27".to_string(),
    )
    .unwrap();

    assert!(result.is_none(), "default-OFF must short-circuit to None");
}

#[test]
fn test_inference_command_returns_none_when_setting_disabled() {
    use crate::commands_internal::settings_cmd::set_infer_trip_times_internal;
    use crate::commands_internal::trips::get_inferred_trip_time_for_route_internal;
    let (db, _temp_db) = test_db_with_completed_trip();
    let dir = tempfile::tempdir().unwrap();
    let app_dir = dir.path().to_path_buf();
    set_infer_trip_times_internal(&app_dir, false).unwrap();

    let result = get_inferred_trip_time_for_route_internal(
        &db,
        &app_dir,
        "vehicle-1".to_string(),
        "Bratislava".to_string(),
        "Žilina".to_string(),
        "2026-04-27".to_string(),
    )
    .unwrap();

    assert!(result.is_none(), "Some(false) must short-circuit to None");
}

#[test]
fn test_inference_command_returns_some_when_setting_enabled() {
    use crate::commands_internal::settings_cmd::set_infer_trip_times_internal;
    use crate::commands_internal::trips::get_inferred_trip_time_for_route_internal;
    let (db, _temp_db) = test_db_with_completed_trip();
    let dir = tempfile::tempdir().unwrap();
    let app_dir = dir.path().to_path_buf();
    set_infer_trip_times_internal(&app_dir, true).unwrap();

    let result = get_inferred_trip_time_for_route_internal(
        &db,
        &app_dir,
        "vehicle-1".to_string(),
        "Bratislava".to_string(),
        "Žilina".to_string(),
        "2026-04-27".to_string(),
    )
    .unwrap();

    assert!(result.is_some(), "Some(true) must allow inference to run");
}
```

> **NOTE:** `test_db_with_completed_trip()` is a placeholder for whatever helper already exists in [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) for seeding a DB with one matching trip on the route Bratislava→Žilina for vehicle "vehicle-1". If no such helper exists, **first** locate the existing inference test (`rg "inferred_trip_time_for_route" src-tauri/core/src/commands_internal/commands_tests.rs`) and reuse its setup pattern. Adjust the route/vehicle strings to match.

**Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test -p kniha-jazd-core test_inference_command`
Expected: FAIL — compile error: `function takes 5 arguments but 6 were supplied` (the new `app_dir` arg).

**Step 3: Update internal function signature + add the gate**

In [trips.rs](../../src-tauri/core/src/commands_internal/trips.rs), modify `get_inferred_trip_time_for_route_internal` (line 230-241) to:

```rust
pub fn get_inferred_trip_time_for_route_internal(
    db: &Database,
    app_dir: &Path,                              // NEW
    vehicle_id: String,
    origin: String,
    destination: String,
    row_date: String,
) -> Result<Option<InferredTripTime>, String> {
    // Gate on setting — default OFF.
    use crate::settings::LocalSettings;
    let settings = LocalSettings::load(app_dir);
    if !settings.infer_trip_times.unwrap_or(false) {
        return Ok(None);
    }

    let row_date = NaiveDate::parse_from_str(&row_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid row_date (expected YYYY-MM-DD): {}", e))?;
    let mut jitter = ThreadRngJitter;
    inferred_trip_time_for_route(db, &mut jitter, &vehicle_id, &origin, &destination, row_date)
}
```

Add `use std::path::Path;` at the top of the file if not already imported.

**Step 4: Update the desktop wrapper to pass `app_dir`**

In [src-tauri/desktop/src/commands/trips.rs](../../src-tauri/desktop/src/commands/trips.rs):153-160, update the wrapper to take `app: tauri::AppHandle` and derive `app_dir`:

```rust
#[tauri::command]
pub fn get_inferred_trip_time_for_route(
    app: tauri::AppHandle,                       // NEW
    db: tauri::State<std::sync::Arc<kniha_jazd_core::db::Database>>,
    vehicle_id: String,
    origin: String,
    destination: String,
    row_date: String,
) -> Result<Option<kniha_jazd_core::commands_internal::trips::InferredTripTime>, String> {
    let app_dir = super::get_app_data_dir(&app)?;  // mirrors theme/auto-update wrappers
    inner::get_inferred_trip_time_for_route_internal(
        &db,
        &app_dir,
        vehicle_id,
        origin,
        destination,
        row_date,
    )
}
```

**Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test -p kniha-jazd-core test_inference_command`
Expected: PASS — three new tests green.

Then run the full backend suite to catch any regression:

Run: `cd src-tauri && cargo test`
Expected: PASS — all tests green (existing inference tests still work because they use the test seam `inferred_trip_time_for_route`, not the gated public command).

**Step 6: Commit**

```bash
git add src-tauri/core/src/commands_internal/trips.rs \
        src-tauri/core/src/commands_internal/commands_tests.rs \
        src-tauri/desktop/src/commands/trips.rs
git commit -m "feat(trips): gate route-time inference on infer_trip_times setting"
```

---

## Task 6: Tauri command wrappers for `get/set_infer_trip_times`

**Files:**
- Modify: [src-tauri/desktop/src/commands/settings_cmd.rs](../../src-tauri/desktop/src/commands/settings_cmd.rs):109-124 (insert after Hidden Columns)
- Modify: [src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs) (invoke_handler)

**Step 1: Add Tauri command wrappers**

In [settings_cmd.rs (desktop)](../../src-tauri/desktop/src/commands/settings_cmd.rs), append after the `Hidden Columns` block (after line 124):

```rust
// ============================================================================
// Time Inference Toggle
// ============================================================================

#[tauri::command]
pub fn get_infer_trip_times(app: tauri::AppHandle) -> Result<bool, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::get_infer_trip_times_internal(&app_dir)
}

#[tauri::command]
pub fn set_infer_trip_times(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::set_infer_trip_times_internal(&app_dir, enabled)
}
```

**Step 2: Register both commands in `invoke_handler!`**

In [lib.rs (desktop)](../../src-tauri/desktop/src/lib.rs), find the `invoke_handler!` / `tauri::generate_handler!` block (search for `commands::get_date_prefill_mode`) and add two lines next to it:

```rust
commands::get_date_prefill_mode,
commands::set_date_prefill_mode,
commands::get_infer_trip_times,    // NEW
commands::set_infer_trip_times,    // NEW
```

**Step 3: Verify the desktop crate compiles**

Run: `cd src-tauri && cargo build -p kniha-jazd-desktop`
Expected: PASS — clean build.

**Step 4: Commit**

```bash
git add src-tauri/desktop/src/commands/settings_cmd.rs src-tauri/desktop/src/lib.rs
git commit -m "feat(settings): expose infer_trip_times via Tauri commands"
```

---

## Task 7: Frontend `api.ts` bindings

**Files:**
- Modify: [src/lib/api.ts](../../src/lib/api.ts):419-425 (insert after `setDatePrefillMode`)

**Step 1: Add the two thin wrappers**

In [api.ts](../../src/lib/api.ts), append after `setDatePrefillMode` (around line 425):

```ts
export async function getInferTripTimes(): Promise<boolean> {
    return apiCall<boolean>('get_infer_trip_times');
}

export async function setInferTripTimes(enabled: boolean): Promise<void> {
    return apiCall('set_infer_trip_times', { enabled });
}
```

**Step 2: Run typecheck to verify compilation**

Run: `npm run check`
Expected: PASS — no TS errors.

**Step 3: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(api): bindings for get/setInferTripTimes"
```

---

## Task 8: i18n keys

**Files:**
- Modify: [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts) (add to `trips` and `settings` namespaces)
- Modify: [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts) (mirror)
- Modify: [src/lib/i18n/i18n-types.ts](../../src/lib/i18n/i18n-types.ts) (regenerated)

**Step 1: Add Slovak keys**

In [sk/index.ts](../../src/lib/i18n/sk/index.ts), add to the `trips` namespace (next to `datePrefillTooltip` near line 114):

```ts
timeInferenceApplied: 'Časy doplnené z poslednej trasy',
timeInferenceUndo: 'Vrátiť',
```

And add to the `settings` namespace:

```ts
inferTripTimesLabel: 'Automaticky vyplniť časy podľa poslednej trasy',
inferTripTimesDescription: 'Pri novom zázname po výbere trasy doplní začiatok a koniec z poslednej rovnakej cesty (s malou náhodnou odchýlkou).',
```

**Step 2: Add English keys (mirror)**

In [en/index.ts](../../src/lib/i18n/en/index.ts), add the same keys with English text:

```ts
// in trips namespace:
timeInferenceApplied: 'Times filled from last route',
timeInferenceUndo: 'Undo',

// in settings namespace:
inferTripTimesLabel: 'Auto-fill times from last route',
inferTripTimesDescription: 'When creating a new entry, fill start/end times from the most recent matching route (with a small random offset).',
```

**Step 3: Regenerate `i18n-types.ts`**

If the project uses `typesafe-i18n` (it does — see [i18n-types.ts](../../src/lib/i18n/i18n-types.ts)), run the generator. Otherwise add the type entries manually mirroring `datePrefillPrevious` etc.

Run: `npm run typesafe-i18n` (if defined) or `npx typesafe-i18n --no-watch`
If neither works, edit [i18n-types.ts](../../src/lib/i18n/i18n-types.ts) by hand to add the four new keys to the `trips` and `settings` namespaces (mirror existing entries like `datePrefillPrevious` for shape).

**Step 4: Verify typecheck**

Run: `npm run check`
Expected: PASS — `$LL.trips.timeInferenceApplied`, `$LL.trips.timeInferenceUndo`, `$LL.settings.inferTripTimesLabel`, `$LL.settings.inferTripTimesDescription` are all typed.

**Step 5: Commit**

```bash
git add src/lib/i18n/
git commit -m "i18n: add keys for time-inference toast and settings toggle"
```

---

## Task 9: Toast store — add `withAction` method

**Files:**
- Modify: [src/lib/stores/toast.ts](../../src/lib/stores/toast.ts):6-40

**Step 1: Extend the `Toast` interface**

In [toast.ts](../../src/lib/stores/toast.ts), update the interface (line 6-10):

```ts
export interface Toast {
    id: number;
    message: string;
    type: ToastType;
    action?: { label: string; onClick: () => void };
}
```

**Step 2: Add the `withAction` method to the store**

Inside `createToastStore` (after `dismiss`, before `return`):

```ts
function withAction(
    message: string,
    actionLabel: string,
    onAction: () => void,
    duration = 6000
) {
    const id = nextId++;
    update((toasts) => [
        ...toasts,
        {
            id,
            message,
            type: TOAST_TYPES.INFO,
            action: {
                label: actionLabel,
                onClick: () => {
                    onAction();
                    dismiss(id);
                },
            },
        },
    ]);
    if (duration > 0) {
        setTimeout(() => dismiss(id), duration);
    }
}
```

Add `withAction` to the returned object alongside `success/error/info/dismiss`:

```ts
return {
    subscribe,
    success: (message: string) => show(message, TOAST_TYPES.SUCCESS),
    error: (message: string) => show(message, TOAST_TYPES.ERROR, 6000),
    info: (message: string) => show(message, TOAST_TYPES.INFO),
    withAction,        // NEW
    dismiss,
};
```

**Step 3: Verify typecheck**

Run: `npm run check`
Expected: PASS — no errors. Existing `toast.success/error/info` callers unaffected.

**Step 4: Commit**

```bash
git add src/lib/stores/toast.ts
git commit -m "feat(toast): add withAction method for actionable toasts"
```

---

## Task 10: Toast component — render action button

**Files:**
- Modify: [src/lib/components/Toast.svelte](../../src/lib/components/Toast.svelte)

**Step 1: Render the action button when present**

In [Toast.svelte](../../src/lib/components/Toast.svelte), inside the `{#each $toast as t}` block, add the button after `<span class="toast-message">`:

```svelte
<span class="toast-message">{t.message}</span>
{#if t.action}
    <button
        class="toast-action"
        on:click|stopPropagation={t.action.onClick}
    >
        {t.action.label}
    </button>
{/if}
```

`stopPropagation` is required so clicking the action does NOT also fire the toast's whole-element `on:click` dismiss handler.

**Step 2: Add styles**

Append inside the `<style>` block:

```css
.toast-action {
    background: transparent;
    border: 1px solid currentColor;
    color: inherit;
    padding: 0.25rem 0.6rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 600;
    margin-left: 0.5rem;
}

.toast-action:hover {
    background: rgba(255, 255, 255, 0.15);
}
```

**Step 3: Manual smoke test**

Run: `npm run tauri dev`
In dev tools console, run:

```js
window.__toastSmokeTest = (() => {
    import('/src/lib/stores/toast.ts').then(({ toast }) => {
        toast.withAction('Hello', 'Undo', () => console.log('undone'));
    });
})();
```

Expected: A toast appears with an "Undo" button. Clicking the button logs "undone" and dismisses. Clicking the body of the toast (away from the button) dismisses without logging.

**Step 4: Commit**

```bash
git add src/lib/components/Toast.svelte
git commit -m "feat(toast): render action button when toast has action"
```

---

## Task 11: Settings page toggle UI

**Files:**
- Modify: [src/routes/settings/+page.svelte](../../src/routes/settings/+page.svelte)

**Step 1: Wire load + save in the settings page**

Locate the existing settings page logic (search for `getDatePrefillMode` import or `auto_check_updates` toggle for pattern). At the top of the `<script>` block:

```ts
import { getInferTripTimes, setInferTripTimes } from '$lib/api';

let inferTripTimes = $state(false);

onMount(async () => {
    inferTripTimes = await getInferTripTimes();
    // … existing onMount code
});

async function handleInferTripTimesChange(enabled: boolean) {
    inferTripTimes = enabled;
    await setInferTripTimes(enabled);
}
```

(Adjust to the project's actual state pattern — Svelte 5 `$state` if used, otherwise plain `let inferTripTimes = false`.)

**Step 2: Add the toggle UI in a "Trip entry" section**

In the page template, in the trip-entry section (or create one if none exists, near other trip preferences):

```svelte
<section class="settings-section">
    <h3>{$LL.settings.inferTripTimesLabel()}</h3>
    <p class="settings-description">{$LL.settings.inferTripTimesDescription()}</p>
    <label class="toggle">
        <input
            type="checkbox"
            checked={inferTripTimes}
            on:change={(e) => handleInferTripTimesChange(e.currentTarget.checked)}
        />
        <span>{inferTripTimes ? $LL.common.on() : $LL.common.off()}</span>
    </label>
</section>
```

(Use whatever toggle pattern the existing page uses — if it has a `<Toggle>` component, use that.)

**Step 3: Manual smoke test**

Run: `npm run tauri dev`
- Open settings page → toggle is visible, defaults to **off**.
- Enable it → reload app → toggle is still on (round-tripped through [local.settings.json](../../)).
- Disable it → reload → still off.

**Step 4: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(settings): add UI toggle for time-inference auto-fill"
```

---

## Task 12: Integration test (Tier 2) — written BEFORE TripRow wiring

This test will fail until Task 13 wires up the toast/undo. The failing test drives that implementation.

**Files:**
- Create: [tests/integration/specs/tier2/time-inference-toggle.spec.ts](../../tests/integration/specs/tier2/time-inference-toggle.spec.ts)

**Step 1: Write the failing integration test**

Read [tests/integration/specs/tier2/date-prefill.spec.ts](../../tests/integration/specs/tier2/date-prefill.spec.ts) first to learn the test patterns (selectors, helpers, page-object usage). Then create [time-inference-toggle.spec.ts](../../tests/integration/specs/tier2/time-inference-toggle.spec.ts):

```ts
import { describe, it, before } from 'mocha';
import { expect } from 'chai';
import { setupApp, addTrip, openNewRow, fillField, getFieldValue } from '../../helpers/app';
// (import names above are placeholders — match what date-prefill.spec.ts actually imports)

describe('time inference toggle', () => {
    before(async () => {
        await setupApp();
        // Seed: one completed trip Bratislava → Žilina with known times,
        // matching a vehicle that's preselected.
        await addTrip({
            origin: 'Bratislava',
            destination: 'Žilina',
            startDatetime: '2026-04-26T08:00',
            endDatetime: '2026-04-26T10:00',
            // …other required fields per the test helper
        });
    });

    it('does NOT overwrite times when setting is OFF (default)', async () => {
        await openNewRow();
        await fillField('startDatetime', '2026-04-27T07:30');
        await fillField('endDatetime', '2026-04-27T09:30');
        await fillField('origin', 'Bratislava');
        await fillField('destination', 'Žilina');

        const start = await getFieldValue('startDatetime');
        const end = await getFieldValue('endDatetime');
        expect(start).to.equal('2026-04-27T07:30');
        expect(end).to.equal('2026-04-27T09:30');

        // No toast should appear.
        const toasts = await $$('.toast');
        expect(toasts.length).to.equal(0);
    });

    it('overwrites times AND shows toast with Undo when setting is ON', async () => {
        // Enable the setting via the settings page.
        await navigateToSettings();
        await toggleSetting('infer-trip-times', true);
        await navigateBack();

        await openNewRow();
        await fillField('startDatetime', '2026-04-27T07:30');
        await fillField('endDatetime', '2026-04-27T09:30');
        await fillField('origin', 'Bratislava');
        await fillField('destination', 'Žilina');

        // Times should now reflect the historical 08:00/10:00 ± jitter.
        const start = await getFieldValue('startDatetime');
        expect(start).to.not.equal('2026-04-27T07:30');

        // Toast with Vrátiť/Undo button should appear.
        const toast = await $('.toast .toast-action');
        await toast.waitForDisplayed({ timeout: 2000 });
        expect(await toast.getText()).to.match(/Vrátiť|Undo/);
    });

    it('Undo restores the original typed values', async () => {
        // Continuing from previous test — toast still displayed.
        const undoBtn = await $('.toast .toast-action');
        await undoBtn.click();

        const start = await getFieldValue('startDatetime');
        const end = await getFieldValue('endDatetime');
        expect(start).to.equal('2026-04-27T07:30');
        expect(end).to.equal('2026-04-27T09:30');
    });
});
```

> **NOTE:** Helper names (`setupApp`, `addTrip`, `openNewRow`, `fillField`, `navigateToSettings`, `toggleSetting`) are placeholders — read the actual helpers in [tests/integration/helpers/](../../tests/integration/helpers/) and use what's there. If a helper doesn't exist (e.g. `toggleSetting`), prefer raw WebdriverIO selectors over inventing new helpers — keep this PR focused.

**Step 2: Run the spec and verify it fails**

Run: `npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/time-inference-toggle.spec.ts`
Expected: FAIL — Test 1 likely passes (default-OFF works thanks to backend gate from Task 5), but Test 2 and Test 3 fail because:
- No toast appears yet (TripRow doesn't call `toast.withAction`)
- No undo wiring yet

**Step 3: Commit the failing spec**

```bash
git add tests/integration/specs/tier2/time-inference-toggle.spec.ts
git commit -m "test(integration): time-inference toggle + toast + undo (failing)"
```

---

## Task 13: Wire toast + undo into TripRow

**Files:**
- Modify: [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte):170-191

**Step 1: Import `toast` store**

At the top of the `<script>` block in [TripRow.svelte](../../src/lib/components/TripRow.svelte):

```ts
import { toast } from '$lib/stores/toast';
```

(Skip if already imported.)

**Step 2: Snapshot pre-overwrite values + show toast**

Replace the body of the `if (result)` block inside `tryInferTimes()` (currently lines 182-186):

```ts
if (result) {
    // Snapshot pre-overwrite values so undo can restore them.
    const previousStart = formData.startDatetime;
    const previousEnd = formData.endDatetime;

    // Apply the inferred times.
    formData.startDatetime = result.startDatetime.slice(0, 16);
    formData.endDatetime = result.endDatetime.slice(0, 16);

    // Toast with undo action.
    toast.withAction(
        $LL.trips.timeInferenceApplied(),
        $LL.trips.timeInferenceUndo(),
        () => {
            formData.startDatetime = previousStart;
            formData.endDatetime = previousEnd;
            inferredKey = ''; // allow re-trigger if user changes their mind
        }
    );
}
```

**Step 3: Run the integration test to verify it passes**

Run: `npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/time-inference-toggle.spec.ts`
Expected: PASS — all three test cases green.

**Step 4: Run full backend + integration suite (regression check)**

Run: `cd src-tauri && cargo test`
Expected: PASS — all backend tests still green.

Run: `npm run test:integration:tier1`
Expected: PASS — Tier 1 specs still green.

**Step 5: Commit**

```bash
git add src/lib/components/TripRow.svelte
git commit -m "feat(trips): toast + undo for route-based time inference"
```

---

## Task 14: Documentation — changelog + decision

**Files:**
- Modify: [CHANGELOG.md](../../CHANGELOG.md)
- Modify: [DECISIONS.md](../../DECISIONS.md)

**Step 1: Add `[Unreleased]` changelog entry via the skill**

Run: `/changelog`
Add under the [Unreleased] section a **Changed** entry:

```markdown
### Changed
- Auto-fill of new-trip start/end times from the most recent matching route is now **off by default**. Enable it under Settings → Trip entry → "Auto-fill times from last route". When enabled, a toast notifies you and offers an undo action.
```

**Step 2: Add a BIZ decision entry via the skill**

Run: `/decision`
Provide:
- **Title:** Default `infer_trip_times` to OFF
- **Context:** Existing route-based time inference was silently overwriting user-typed start/end values, surprising users who hadn't read the source.
- **Decision:** Default the new `infer_trip_times` setting to OFF (treating `None` as `false`). Toast appears only when ON, making the feature discoverable for users who opt in.
- **Alternatives considered:**
  - Default ON with toast — preserves prior behavior + leans on discoverability, but still surprises users at least once.
  - Default ON without toast — current state; rejected as user-hostile.
- **Trade-offs:** Existing users who relied on the auto-fill lose it silently after upgrade. Mitigation: changelog entry + the in-app discovery path via the toast once they enable it.

**Step 3: Run `/verify` to confirm the work is complete**

Run: `/verify`
Expected: clean git status (or only the docs updates), changelog updated, decision recorded.

**Step 4: Commit**

```bash
git add CHANGELOG.md DECISIONS.md
git commit -m "docs: changelog + BIZ decision for default-OFF time inference"
```

---

## Final verification

Run the full test gauntlet before declaring done:

```bash
cd src-tauri && cargo test
cd .. && npm run check
npm run test:integration:tier1
npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/time-inference-toggle.spec.ts
```

Expected: all PASS.

Manual smoke checklist:
- Toggle off (default fresh install): origin+destination on a new row leaves typed times alone. No toast.
- Toggle on: same flow overwrites times. Toast appears with "Vrátiť" button. Click → times restored. Re-pick destination → inference fires again (because `inferredKey` was cleared).
- Settings page persists state across app restart.

---

## Execution Handoff

**Two execution options:**

**1. Subagent-Driven (this session)** — I dispatch a fresh subagent per task and review between tasks. Good for fast iteration.

**2. Parallel Session (separate)** — open a new session with `superpowers:executing-plans`, batch execution with checkpoints. Good when the plan can run mostly unattended.

Which approach?
