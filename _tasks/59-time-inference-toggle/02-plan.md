# Time Inference Toggle — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement
> this plan task-by-task.

**Goal:** Make the new-row time inference (origin+destination → auto-fill start/end
with jitter) opt-in via a setting (default OFF), and surface a toast with "Vrátiť"
(undo) action when it does fire.

**Architecture:**

- Backend gate at the **command boundary** (in
  [trips.rs](../../src-tauri/core/src/commands_internal/trips.rs)), not inside the
  pure inference math. Keeps `compute_inferred_times` testable as a pure function.
- Frontend toast extension is additive — existing `toast.success/error/info` calls
  stay untouched. New `toast.withAction(...)` method.
- Per-row undo is **in-memory only** (snapshot taken in
  [TripRow.svelte](../../src/lib/components/TripRow.svelte) before overwrite,
  closure-captured in the toast action handler). No persistence, no history stack.

**Order of work** matches dependency order: data model → backend gate → toast plumbing
→ UI wiring → settings page → tests → docs. TDD throughout: failing test first, then
implementation.

---

## Task 1: Add `infer_trip_times` field to LocalSettings

**Files:**

- Modify: [settings.rs](../../src-tauri/core/src/settings.rs)
- Modify: [settings_tests.rs](../../src-tauri/core/src/settings_tests.rs)

**Steps:**

1. Write a failing test in [settings_tests.rs](../../src-tauri/core/src/settings_tests.rs):
   - `default_infer_trip_times_is_none` — load `LocalSettings` from an empty dir,
     assert `settings.infer_trip_times == None`.
   - `roundtrip_infer_trip_times_some_true` — save with `Some(true)`, load, assert
     `Some(true)`.
   - `roundtrip_infer_trip_times_some_false` — save with `Some(false)`, load, assert
     `Some(false)`.
2. Add field to the `LocalSettings` struct in
   [settings.rs](../../src-tauri/core/src/settings.rs):
   ```rust
   pub infer_trip_times: Option<bool>,  // None = OFF (default)
   ```
3. Run [settings_tests.rs](../../src-tauri/core/src/settings_tests.rs) — tests pass.

**Verification:**

- `cargo test -p kniha-jazd-core settings::` passes (3 new tests + existing).
- Manually inspect a newly-saved [local.settings.json](../../) — field appears as
  `"inferTripTimes":null` after a default save (or absent — both fine since it's
  `Option`).

---

## Task 2: Add backend getter/setter command (internal layer)

**Files:**

- Modify: [settings_cmd.rs](../../src-tauri/core/src/commands_internal/settings_cmd.rs)

**Steps:**

1. Mirror the `date_prefill_mode` pattern in
   [settings_cmd.rs](../../src-tauri/core/src/commands_internal/settings_cmd.rs):
   ```rust
   pub fn get_infer_trip_times_internal(app_dir: &Path) -> Result<bool, String> {
       let settings = LocalSettings::load(app_dir);
       Ok(settings.infer_trip_times.unwrap_or(false))  // default: OFF
   }

   pub fn set_infer_trip_times_internal(app_dir: &Path, enabled: bool) -> Result<(), String> {
       let mut settings = LocalSettings::load(app_dir);
       settings.infer_trip_times = Some(enabled);
       settings.save(app_dir).map_err(|e| e.to_string())
   }
   ```
2. No new tests — covered by Task 1 round-trip tests + the setting's effect is tested
   in Task 3.

**Verification:**

- `cargo build -p kniha-jazd-core` compiles cleanly.

---

## Task 3: Gate inference command on the setting

**Files:**

- Modify: [trips.rs](../../src-tauri/core/src/commands_internal/trips.rs)
- Modify: [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs)

**Steps:**

1. Write failing tests in
   [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs):
   - `inference_returns_none_when_setting_disabled` — seed a matching trip; with
     `infer_trip_times: Some(false)` saved to `app_dir`, command returns `Ok(None)`.
   - `inference_returns_none_when_setting_unset` — seed a matching trip; with no
     `infer_trip_times` field saved, command returns `Ok(None)` (default-OFF).
   - `inference_returns_times_when_setting_enabled` — seed a matching trip; with
     `infer_trip_times: Some(true)`, command returns `Ok(Some(...))`.
2. Modify `get_inferred_trip_time_for_route_internal` in
   [trips.rs](../../src-tauri/core/src/commands_internal/trips.rs):
   - Add `app_dir: &Path` parameter.
   - At the top, load `LocalSettings::load(app_dir)`. If
     `settings.infer_trip_times.unwrap_or(false) == false`, return `Ok(None)` early.
   - Existing logic runs only when enabled.
3. Update the Tauri wrapper in
   [desktop/src/commands/trips_cmd.rs](../../src-tauri/desktop/src/commands/) (or
   wherever the wrapper for this command currently lives — locate via
   `rg get_inferred_trip_time_for_route` in [src-tauri/desktop](../../src-tauri/desktop/))
   to pass `app_dir` from the Tauri `AppHandle`.
4. Run the new tests — they pass.

**Verification:**

- `cargo test -p kniha-jazd-core commands_internal::` passes (3 new tests + existing).
- The pure helpers `compute_inferred_times` and `inferred_trip_time_for_route` are
  unmodified — verify by `git diff` showing no changes to
  [time_inference.rs](../../src-tauri/core/src/calculations/time_inference.rs) or to
  the inner function in [trips.rs](../../src-tauri/core/src/commands_internal/trips.rs).

---

## Task 4: Expose the setting via Tauri commands (desktop wrapper)

**Files:**

- Modify: [settings_cmd.rs (desktop)](../../src-tauri/desktop/src/commands/settings_cmd.rs)
- Modify: [lib.rs (desktop)](../../src-tauri/desktop/src/lib.rs)

**Steps:**

1. Add Tauri command wrappers in
   [desktop/src/commands/settings_cmd.rs](../../src-tauri/desktop/src/commands/settings_cmd.rs)
   following the existing `set_date_prefill_mode` pattern:
   ```rust
   #[tauri::command]
   pub fn get_infer_trip_times(app: tauri::AppHandle) -> Result<bool, String> {
       let app_dir = /* resolve via app handle, like existing commands */;
       core::commands_internal::settings_cmd::get_infer_trip_times_internal(&app_dir)
   }

   #[tauri::command]
   pub fn set_infer_trip_times(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
       let app_dir = /* resolve via app handle */;
       core::commands_internal::settings_cmd::set_infer_trip_times_internal(&app_dir, enabled)
   }
   ```
2. Register both commands in the `invoke_handler!` macro in
   [lib.rs (desktop)](../../src-tauri/desktop/src/lib.rs).

**Verification:**

- `cargo build -p kniha-jazd-desktop` compiles.
- `npm run tauri dev` starts without runtime panics on Tauri command registration.

---

## Task 5: Frontend API binding

**Files:**

- Modify: [api.ts](../../src/lib/api.ts)

**Steps:**

1. Add two thin wrappers in [api.ts](../../src/lib/api.ts) next to the existing
   `getDatePrefillMode` / `setDatePrefillMode` functions (~line 419):
   ```ts
   export async function getInferTripTimes(): Promise<boolean> {
       return apiCall<boolean>('get_infer_trip_times');
   }

   export async function setInferTripTimes(enabled: boolean): Promise<void> {
       return apiCall('set_infer_trip_times', { enabled });
   }
   ```

**Verification:**

- `npm run check` (svelte-check) passes — no TypeScript errors.

---

## Task 6: Extend toast store + component to support action buttons

**Files:**

- Modify: [toast.ts](../../src/lib/stores/toast.ts)
- Modify: [Toast.svelte](../../src/lib/components/Toast.svelte)

**Steps:**

1. Extend the `Toast` interface in [toast.ts](../../src/lib/stores/toast.ts):
   ```ts
   export interface Toast {
       id: number;
       message: string;
       type: ToastType;
       action?: { label: string; onClick: () => void };
   }
   ```
2. Add `withAction` method to the store:
   ```ts
   function withAction(
       message: string,
       actionLabel: string,
       onAction: () => void,
       duration = 6000
   ) {
       const id = nextId++;
       update((toasts) => [...toasts, {
           id,
           message,
           type: TOAST_TYPES.INFO,
           action: { label: actionLabel, onClick: () => { onAction(); dismiss(id); } }
       }]);
       if (duration > 0) setTimeout(() => dismiss(id), duration);
   }
   ```
   Expose in returned object: `withAction`.
3. Update [Toast.svelte](../../src/lib/components/Toast.svelte) to render the action
   button when `t.action` is set, and stop the action click from bubbling up to the
   toast's own dismiss handler:
   ```svelte
   {#if t.action}
       <button
           class="toast-action"
           on:click|stopPropagation={t.action.onClick}
       >
           {t.action.label}
       </button>
   {/if}
   ```
   Add minimal styling (small button, themed, right-aligned within the toast).

**Verification:**

- Existing `success/error/info` toasts still render correctly (no regression).
- Manual smoke: open dev tools, run
  `toast.withAction('hello', 'Undo', () => console.log('undone'))` — toast appears,
  clicking "Undo" logs and dismisses. Clicking the body of the toast (away from the
  button) also dismisses.

---

## Task 7: Wire up inference toast + undo in TripRow

**Files:**

- Modify: [TripRow.svelte](../../src/lib/components/TripRow.svelte)

**Steps:**

1. In `tryInferTimes()` (around line 170 of
   [TripRow.svelte](../../src/lib/components/TripRow.svelte)), capture the pre-overwrite
   values **before** the assignment on lines 184-185:
   ```ts
   const previousStart = formData.startDatetime;
   const previousEnd = formData.endDatetime;
   const previousInferredKey = inferredKey;  // for full revert
   formData.startDatetime = result.startDatetime.slice(0, 16);
   formData.endDatetime = result.endDatetime.slice(0, 16);
   ```
2. Show toast with undo callback:
   ```ts
   toast.withAction(
       $LL.trips.timeInferenceApplied(),
       $LL.trips.timeInferenceUndo(),
       () => {
           formData.startDatetime = previousStart;
           formData.endDatetime = previousEnd;
           inferredKey = '';  // allow re-trigger
       }
   );
   ```
3. Import `toast` from [toast.ts](../../src/lib/stores/toast.ts) at the top of the
   `<script>` block if not already imported.

**Verification:**

- `npm run tauri dev`, enable the setting (Task 9), pick a route with prior history,
  observe: times change + toast appears with "Vrátiť" button.
- Click "Vrátiť": times revert to whatever was there before. Re-pick destination:
  inference fires again (because `inferredKey` was cleared).
- Click the body of the toast (not the button): toast dismisses without undoing.

---

## Task 8: i18n strings

**Files:**

- Modify: [sk/index.ts](../../src/lib/i18n/sk/index.ts)
- Modify: [en/index.ts](../../src/lib/i18n/en/index.ts)
- Modify: [i18n-types.ts](../../src/lib/i18n/i18n-types.ts) (regenerated; or run
  the typesafe-i18n generator if the project uses it)

**Steps:**

1. Add to the `trips` namespace (alongside `datePrefillTooltip` etc.):
   - `timeInferenceApplied` — "Časy doplnené z poslednej trasy" / "Times filled from
     last route"
   - `timeInferenceUndo` — "Vrátiť" / "Undo"
2. Add to the `settings` namespace:
   - `inferTripTimesLabel` — "Automaticky vyplniť časy podľa poslednej trasy" /
     "Auto-fill times from last route"
   - `inferTripTimesDescription` — short description in both languages explaining
     the behavior + jitter
3. Regenerate [i18n-types.ts](../../src/lib/i18n/i18n-types.ts) per the project's
   typesafe-i18n setup.

**Verification:**

- `npm run check` passes — no missing-key TS errors.
- UI shows correct strings in Slovak (default) and English.

---

## Task 9: Settings page toggle

**Files:**

- Modify: [settings/+page.svelte](../../src/routes/settings/+page.svelte)

**Steps:**

1. Add a checkbox/switch toggle in the settings page UI, in the trip-entry section.
   Wire to:
   - On mount: `inferTripTimes = await getInferTripTimes();`
   - On change: `await setInferTripTimes(newValue); inferTripTimes = newValue;`
2. Use `$LL.settings.inferTripTimesLabel` for the label and
   `$LL.settings.inferTripTimesDescription` for tooltip / helper text.

**Verification:**

- Toggle visible in the settings page.
- Toggling and reloading the app preserves state (round-trips through
  [local.settings.json](../../)).

---

## Task 10: Integration test (Tier 2)

**Files:**

- Create: [tests/integration/specs/tier2/time-inference-toggle.spec.ts](../../tests/integration/specs/tier2/time-inference-toggle.spec.ts)

**Steps:**

1. Test 1: with the setting OFF (default), seed a matching historical trip, then on a
   new row enter `startDatetime`, then enter origin + destination. Assert
   `startDatetime` and `endDatetime` are **unchanged**, and no toast appears.
2. Test 2: enable the setting via the settings page, repeat the flow. Assert the
   times **change**, a toast with text containing "Vrátiť" is visible.
3. Test 3: continuing from Test 2, capture the inferred values, click the "Vrátiť"
   action, assert the original values are restored.

**Verification:**

- `npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/time-inference-toggle.spec.ts`
  passes (focused run per
  [CLAUDE.md](../../CLAUDE.md) "iteration strategy").
- Final full sweep: `npm run test:integration` passes.

---

## Task 11: Documentation

**Files:**

- Modify: [CHANGELOG.md](../../CHANGELOG.md)
- Modify: [DECISIONS.md](../../DECISIONS.md)

**Steps:**

1. Run [/changelog](../../.claude/skills/changelog-skill/) to add an entry under
   [Unreleased]:
   - **Changed (BREAKING for users who relied on auto-fill):** Auto-fill of trip
     start/end times from the last matching route is now **off by default**. Enable
     it under Settings → Trip entry → "Auto-fill times from last route". When on, a
     toast notifies you and offers an undo action.
2. Run [/decision](../../.claude/skills/decision-skill/) to record a BIZ entry:
   - **Decision:** Default `infer_trip_times` to OFF.
   - **Why:** Existing behavior silently overwrote user-typed start/end times,
     surprising users. Default-OFF eliminates the surprise. The toast (which only
     appears when the setting is ON) makes the feature discoverable for users who
     opt in.
   - **Alternatives considered:** Default-ON with toast — preserves current behavior
     and leans on discoverability, but still surprises users at least once.

**Verification:**

- [CHANGELOG.md](../../CHANGELOG.md) shows the entry.
- [DECISIONS.md](../../DECISIONS.md) shows the BIZ entry.
- Run [/verify](../../.claude/skills/verify-skill/) before final commit.

---

## Final verification

- `npm run test:backend` — all backend tests pass (existing 195 + new ones from
  Tasks 1, 3).
- `npm run test:integration:tier1` quick check, then full
  `npm run test:integration` for the new spec.
- `npm run check` — no TS errors.
- Manual smoke per Tasks 7 and 9 verifications.
- Run [/verify](../../.claude/skills/verify-skill/) to confirm git status is clean,
  changelog updated, decision recorded.
