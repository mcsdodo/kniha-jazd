**Date:** 2026-04-27
**Subject:** Make new-row time inference (start/end auto-fill from last matching route + jitter) opt-in via settings, with a discoverable toast and per-row undo.
**Status:** Planning

## Goal

Stop silently overwriting user-entered start/end datetimes on new trip rows. Move the
existing route-based time inference behind a setting (default: off) and, when it does
fire, surface it via a non-blocking toast with an "undo" action that restores the
pre-inference values for that row.

## Background / Why

The current behavior ([trips.rs](../../src-tauri/core/src/commands_internal/trips.rs):230
→ [time_inference.rs](../../src-tauri/core/src/calculations/time_inference.rs):39)
auto-fills start/end datetimes on a new row when the user picks origin + destination,
then jitters them by ±15 min / ±15% to avoid machine-identical timestamps. Per
[ADR-008](../../DECISIONS.md) the jitter lives in Rust, which is correct.

Two UX problems remain:

1. **Surprise.** The user fills in start/end first, then origin/destination, and their
   typed times silently disappear. They cannot tell that this is intentional.
2. **No escape hatch.** Even users who knew about the feature have no way to opt out
   (short of code changes), and no way to revert a single inference if it lands badly.

## Requirements

### R1 — Setting (default: OFF)

- Add `infer_trip_times: Option<bool>` to `LocalSettings` in
  [settings.rs](../../src-tauri/core/src/settings.rs).
- **When `None` or `Some(false)`: inference is disabled.** Default-OFF was chosen to
  prevent silent overwrites for new and upgrading users alike.
- Expose via `get_infer_trip_times` / `set_infer_trip_times` Tauri commands, mirroring
  the `date_prefill_mode` pattern in
  [settings_cmd.rs](../../src-tauri/core/src/commands_internal/settings_cmd.rs).
- Toggle in [settings page](../../src/routes/settings/+page.svelte) (not the trip
  grid header), grouped with other trip-entry preferences.

### R2 — Backend gate

- `get_inferred_trip_time_for_route_internal` short-circuits to `Ok(None)` when the
  setting is off — no DB lookup, no jitter computed. Frontend therefore receives
  nothing to apply.
- The pure inference helpers (`compute_inferred_times`, `inferred_trip_time_for_route`)
  remain unchanged and unaware of the toggle. The gate lives at the command boundary
  so the calculation core stays a pure function (testability preserved).

### R3 — Toast on apply (with undo)

- After a successful inference, [TripRow.svelte](../../src/lib/components/TripRow.svelte)
  shows a toast in Slovak:
  > *"Časy doplnené z poslednej trasy"* — with an action button **"Vrátiť"**.
- Toast lifetime: ~6 seconds (longer than the current 4s default; user needs time to
  notice and click).
- Clicking **Vrátiť**:
  1. Restores `formData.startDatetime` and `formData.endDatetime` to the values
     captured immediately before the overwrite (may be empty strings, the row's
     default, or whatever the user had typed).
  2. Clears the row's `inferredKey` so the user could re-trigger inference if they
     change their mind (e.g. by re-selecting the destination).
  3. Dismisses the toast.

### R4 — Toast store extension

- Extend [toast.ts](../../src/lib/stores/toast.ts) with a
  `withAction(message, actionLabel, onAction, durationMs?)` method. Existing
  `success/error/info` calls remain unchanged.
- Extend the `Toast` interface with optional `action: { label: string; onClick: () => void }`.
- Update [Toast.svelte](../../src/lib/components/Toast.svelte) to render the action
  button and stop the click from bubbling up to the toast's whole-element dismiss
  handler.

### R5 — i18n

- Add Slovak (primary) + English keys in
  [sk/index.ts](../../src/lib/i18n/sk/index.ts) and
  [en/index.ts](../../src/lib/i18n/en/index.ts) for:
  - Toast message
  - Undo button label
  - Settings toggle label + tooltip / description

### R6 — Tests

- **Backend unit tests** in
  [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs)
  (or a dedicated test file):
  - When `infer_trip_times = Some(false)` → command returns `Ok(None)` even when a
    matching historical trip exists.
  - When `infer_trip_times = Some(true)` → command returns the inferred times.
  - When `infer_trip_times = None` → command returns `Ok(None)` (default-OFF).
- **Settings round-trip test** in
  [settings_tests.rs](../../src-tauri/core/src/settings_tests.rs): load/save preserves
  the new field; default is `None`.
- **Integration test** in
  [tests/integration/specs/tier2](../../tests/integration/specs/tier2/) (WebdriverIO):
  - With setting off (default), entering origin+destination on a new row does NOT
    change the start/end datetime fields.
  - With setting on, it DOES change them and a toast appears with an action button.
  - Clicking the action button restores the prior values.

### R7 — Documentation

- [/changelog](../../.claude/skills/changelog-skill/) entry under [Unreleased] noting
  the new setting and the default-OFF behavior change.
- [/decision](../../.claude/skills/decision-skill/) (BIZ entry) recording why we chose
  default-OFF rather than default-ON-with-discovery-toast (preserves no-surprise
  principle for existing users; new toast still makes the feature discoverable when
  they enable it).

## Out of scope

- No changes to the jitter algorithm itself (still ±15 min / ±15%, per
  [time_inference.rs](../../src-tauri/core/src/calculations/time_inference.rs)).
- No "remember last action" persistence — undo is per-row, in-memory, and only valid
  while that row is still being edited.
- No multi-step undo history; one inference, one undo, one revert.
- No new toast positioning or styling beyond the action button.

## Technical Notes

- **Setting boundary:** read the toggle in `get_inferred_trip_time_for_route_internal`
  (in [trips.rs](../../src-tauri/core/src/commands_internal/trips.rs)) — the function
  already takes `db: &Database`, but it does NOT currently take `app_dir`. We'll need
  to pass it through the Tauri wrapper, mirroring how
  `get_theme_preference_internal(app_dir)` works in
  [settings_cmd.rs](../../src-tauri/core/src/commands_internal/settings_cmd.rs).
- **Capture-before-overwrite:** the natural place is in `tryInferTimes()` at
  [TripRow.svelte](../../src/lib/components/TripRow.svelte):170-191. Snapshot
  `formData.startDatetime` and `formData.endDatetime` BEFORE the assignment on lines
  184-185.
- **Reset of `inferredKey` on undo:** lets the user re-trigger inference deliberately;
  without it, undo would lock that row out of inference for the whole session.
- **No business-logic migration needed:** `compute_inferred_times` remains untouched;
  this is pure plumbing + a new config flag + a new UI affordance.
