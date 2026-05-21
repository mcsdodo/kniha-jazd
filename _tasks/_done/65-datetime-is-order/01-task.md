**Date:** 2026-05-21
**Subject:** Make `start_datetime` the single source of truth for trip order
**Status:** Planning

# Task 65: Datetime Is Order

## Problem

The [trips](../../src-tauri/core/src/schema.rs) table has a `sort_order` integer column that controls display order, manipulable independently from `start_datetime`. This causes a class of bugs where the two drift apart and the grid shows red "date-warning" rows even though the dates are chronologically correct.

**Concrete repro that surfaced the bug:**

A user created trip #66 (21.05), then #64 (18.05), then #65 (20.05) — all via the row-level `+` button. The `+` button calls `handleInsertAbove(targetTrip)` in [TripGrid.svelte](../../src/lib/components/TripGrid.svelte) which sets `insert_at_position = targetTrip.sortOrder`. Because the grid is displayed by `tripNumber` (date order) but `+` inserts at the clicked row's `sort_order` (manual order), once sort_order drifts from date order, every subsequent `+` click compounds the inconsistency.

Result in DB:
```
sort_order 0 → trip #65  (20.05)
sort_order 1 → trip #64  (18.05)
sort_order 2 → trip #66  (21.05)   ← 21 May AFTER 18 May in sort_order = warning
sort_order 3 → trip #63  (12.05)
sort_order 4 → trip #62  (05.05)
```

[calculate_date_warnings](../../src-tauri/core/src/commands_internal/statistics.rs) correctly flags trips #64 and #66 — but from the user's perspective the dates ARE in proper sequence, so the warning is confusing and the "fix" (manually reorder via up/down arrows) is hidden behind a sort-mode toggle.

## Goal

Eliminate the bug class entirely: **the `start_datetime` becomes the only source of truth for trip order.** Manual reordering ceases to exist as a concept. If a user wants a trip to appear earlier in the list, they edit its `start_datetime`.

## User Story

> As a user, when I add a trip with any date — past, present, or interleaved between existing trips — the app inserts it in the correct chronological position automatically. I never see "date warning" red rows for legitimate chronological data. I never need to manually drag, reorder, or switch sort modes. The trip's date IS its position.

## Approach (summary)

- **Drop `sort_order` column** from the `trips` table (Diesel migration).
- All ordering queries use `ORDER BY start_datetime DESC, created_at ASC` (`created_at` as tiebreaker for same-datetime trips; `id` as final tiebreaker if both match).
- [create_trip_internal](../../src-tauri/core/src/commands_internal/trips.rs) drops `insert_at_position` parameter — trip just inserts and is naturally ordered.
- [update_trip](../../src-tauri/core/src/commands_internal/trips.rs) no longer needs special handling for datetime changes — re-reads sort by `start_datetime`.
- Remove `reorder_trip` Tauri command and DB function.
- Remove `calculate_date_warnings` — drift becomes structurally impossible, no need for a guard.
- Frontend: drop manual sort mode, up/down arrows, `reorderTrip` API call, `dateWarnings` from `TripGridData`, `tr.date-warning` CSS, and the `insertAtSortOrder` plumbing through the `+` button. The `+` button keeps its UX role (insert form near the clicked row, pre-fill date) but no longer touches order.

**No data repair migration is needed** — because the order is now computed from `start_datetime` at query time, existing inconsistent data automatically displays in correct chronological order on next read.

## Acceptance Criteria

- [ ] Diesel migration drops `trips.sort_order` column
- [ ] [Trip](../../src-tauri/core/src/models.rs) model has no `sort_order` field
- [ ] `get_trips_for_vehicle` / `get_trips_for_vehicle_in_year` in [db.rs](../../src-tauri/core/src/db.rs) order by `start_datetime DESC, created_at ASC`
- [ ] [create_trip_internal](../../src-tauri/core/src/commands_internal/trips.rs) signature no longer accepts `insert_at_position`
- [ ] `reorder_trip` Tauri command removed; `reorderTrip` TS API removed from [api.ts](../../src/lib/api.ts)
- [ ] `calculate_date_warnings` and related infrastructure removed from [statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs)
- [ ] Frontend: up/down arrows removed from [TripRow.svelte](../../src/lib/components/TripRow.svelte); manual sort mode removed from [TripGrid.svelte](../../src/lib/components/TripGrid.svelte); `tr.date-warning` CSS + i18n strings removed
- [ ] Frontend `+` button uses simplified `createTrip(...)` call (no `insertAtSortOrder` arg)
- [ ] Integration test: reproduces the user's scenario (create trips out of date order via `+`) → no red rows, correct date display
- [ ] All existing backend tests updated (remove `sort_order: 0` from fixtures) and pass
- [ ] All integration tests pass (`npm run test:integration`)
- [ ] `npm run test:all` passes

## Surface Area

**Backend (12 Rust files):**
- [models.rs](../../src-tauri/core/src/models.rs), [schema.rs](../../src-tauri/core/src/schema.rs) — drop field
- [db.rs](../../src-tauri/core/src/db.rs) — drop CRUD for sort_order, replace `ORDER BY sort_order` with `ORDER BY start_datetime`, remove `shift_trips_from_position` + `reorder_trip` DB methods
- [commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) — simplify `create_trip_internal`, remove `reorder_trip_internal`
- [commands_internal/statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs) — remove `calculate_date_warnings`, update `get_trip_grid_data` response shape
- [commands_internal/helpers.rs](../../src-tauri/core/src/commands_internal/helpers.rs) — `calculate_trip_numbers` already uses datetime; simplify tiebreaker from `sort_order` to `created_at`
- [commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs), [db_tests.rs](../../src-tauri/core/src/db_tests.rs), [invoice_tests.rs](../../src-tauri/core/src/invoice_tests.rs), [export_tests.rs](../../src-tauri/core/src/export_tests.rs), [calculations/tests.rs](../../src-tauri/core/src/calculations/tests.rs) — fixture cleanup
- [server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) — drop `reorder_trip` dispatch arm
- New Diesel migration in [migrations/](../../src-tauri/core/migrations/): `2026-05-21-XXXXXX_drop_sort_order`

**Frontend (6 files):**
- [types.ts](../../src/lib/types.ts) — drop `sortOrder` from `Trip`, drop `dateWarnings` from `TripGridData`
- [api.ts](../../src/lib/api.ts) — drop `reorderTrip` export, drop `insertAtPosition` from `createTrip`
- [constants.ts](../../src/lib/constants.ts) — drop `sortOrder` references in mock/seed data
- [TripGrid.svelte](../../src/lib/components/TripGrid.svelte) — drop manual sort mode, `handleMoveUp`/`Down`, `handleInsertAbove` simplification, drop `dateWarnings` Set
- [TripRow.svelte](../../src/lib/components/TripRow.svelte) — drop up/down arrow buttons, `canMoveUp`/`canMoveDown`/`onMoveUp`/`onMoveDown` props, `hasDateWarning` prop, `tr.date-warning` CSS
- [+page.svelte](../../src/routes/+page.svelte) — drop any `sortOrder` references in trip handling

**Integration tests (3 fixture files):**
- [utils/db.ts](../../tests/integration/utils/db.ts), [fixtures/trips.ts](../../tests/integration/fixtures/trips.ts), [fixtures/types.ts](../../tests/integration/fixtures/types.ts) — drop `sort_order` from seed fixtures
- New spec: [tests/integration/specs/tier2/datetime-is-order.spec.ts](../../tests/integration/specs/tier2/datetime-is-order.spec.ts) — repro the bug scenario

**i18n:**
- [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts), [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts) — remove `trips.legend.dateWarning`, `trips.moveUp`, `trips.moveDown`, `toast.errorMoveTrip` strings

## Out of Scope

- `consumption-warning` (orange rows for trips exceeding 120% TP rate) — separate concept, stays untouched.
- `tripNumber` calculation — already date-based, just simplifies its tiebreaker from `sort_order` to `created_at`.
- Calculations (`zostatok`, l/100km, margin) — they iterate chronologically; just switch ordering source from `sort_order` to `start_datetime`.
- Drag-and-drop reordering — never existed in this app.

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| Drop `sort_order` entirely (vs keep as cache) | User-confirmed: only ordering need is by date; cache adds no value, only failure modes. |
| No data-repair migration | Order is computed at read time; existing data displays correctly after schema drop. |
| Single task (not phased) | All changes are tightly coupled; phased rollout adds overhead without safety benefit. |
| `created_at` as tiebreaker (not new field) | Already in schema; deterministic; no migration needed. |

## Plan

See [02-plan.md](./02-plan.md) for step-by-step implementation.
