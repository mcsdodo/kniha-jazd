**Date:** 2026-04-15
**Subject:** Smart defaults for trip row entry — ODO clamp and time inference from known routes
**Status:** Planning

## Goal

Reduce manual data entry friction in the trip grid by:

1. Preventing accidental ODO regressions (typos, off-by-one) by clamping any entered ODO below the previous row's ODO up to `previousOdometer + 1`.
2. Auto-populating start/end time on new trip rows when the (origin, destination) pair matches a previously logged trip — with small randomised jitter so entries don't look machine-identical.

## Requirements

### 1. ODO clamp (applies to ALL trip rows — new and existing edits)

- **Trigger:** user changes the ODO field on any trip row.
- **Logic:** if `entered_value < previousOdometer`, silently set field to `previousOdometer + 1`.
- **Scope:** universal data-integrity rule — applies to both newly added rows and edits of existing rows (ODO monotonicity is a calculation invariant, not a convenience).
- **UX:** silent clamp (no toast/warning). Mirrors auto-calc behaviour already present in `handleOdoChange`.

### 2. Time inference from known origin/destination combos (NEW rows only)

- **Trigger:** on a new (unsaved) trip row, once both `origin` and `destination` are filled.
- **Lookup:** most recent trip for the same `(vehicle_id, origin, destination)` triple with a non-null `end_datetime`.
- **Algorithm:**
  1. `base_start_hhmm` = HH:MM of the matched trip's `start_datetime`.
  2. `base_duration_minutes` = (end_datetime − start_datetime) of matched trip, in minutes.
  3. `new_start_hhmm` = `base_start_hhmm + uniform_int(-15, +15)` minutes.
  4. `new_duration_minutes` = `base_duration_minutes × uniform(0.85, 1.15)`.
  5. `new_end_hhmm` = `new_start_hhmm + new_duration_minutes`.
  6. The row's **date** (already selected/prefilled) is preserved. Only HH:MM is written on both `start_datetime` and `end_datetime`.
- **Scope:** new rows only. Editing an existing row must never re-apply this logic or overwrite user-entered times.
- **Fallback:** no match → leave time fields as-is (don't clear or default).

## Technical Notes

- **Architecture (ADR-008):** all business logic — lookup *and* jitter — stays in Rust. The Tauri command takes `(vehicle_id, origin, destination, row_date)` and returns the final `{start_datetime, end_datetime}`. Frontend just `invoke()`s and writes the returned values. Testability is solved entirely in Rust by splitting into two functions: a pure `compute_inferred_times(row_date, base_start, base_duration_mins, jitter: &mut dyn Jitter)` driven by a trait, plus a thin wrapper that constructs a real `ThreadRngJitter`. Tests supply a `StubJitter` for determinism — no cross-language seam needed.
- **Existing foundation:** `TripRow.svelte` already has `handleOdoChange` (line 186) and route-matching logic via `routes` prop (line 131). ODO clamp extends `handleOdoChange`; time inference hooks into the existing origin/destination change path.
- **Data model:** "fillup" is not a distinct row — any `Trip` with `fuel_liters > 0` is a fillup. The user's wording "fillup row" refers to trip rows generally; no schema changes needed.
- **Testability:** the randomness boundary must be separable for tests. Frontend helper `computeInferredTimes(baseStart, baseDurationMin, jitterFn)` accepts an injectable jitter function so tests can pass a deterministic stub.
- **Edge cases to cover in tests:**
  - New row with no matching route → no time written.
  - Matched trip has null `end_datetime` → treat as no match.
  - ODO clamp on existing row edit (user sets lower value → clamps).
  - ODO clamp when `previousOdometer == 0` (first trip) → no clamp possible; leave untouched.
  - Day boundary: `new_start + duration` crosses midnight → end_datetime rolls to next day correctly.
