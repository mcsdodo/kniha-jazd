**Date:** 2026-03-04
**Subject:** Fix odometer recalculation bugs — stale data + circular sort
**Status:** Complete

## Goal

Fix two related bugs that corrupt odometer values when editing trips (especially date/time changes):
1. `handleUpdate` runs `recalculateNewerTripsOdo` on stale `trips` data (before DB refresh)
2. `recalculateNewerTripsOdo` sorts same-day trips by odometer (the value being recalculated), creating a circular dependency

## Background

- **Commit `b13aa05`** (Jan 2026) fixed this same stale-data pattern for `handleSaveNew` and reorder operations, but **missed `handleUpdate`**
- `recalculateNewerTripsOdo` was introduced in `d3531da` (Dec 2025) as a performance optimization — only recalculate trips after the edited one instead of all trips
- The performance benefit is negligible (~34 trips/year, local Tauri IPC)
- The function also sorts by `tripDate()` which strips time (`YYYY-MM-DD` only), then uses `odometer` as tiebreaker — both are bugs

## User Impact

When adding/deleting/reordering trips with date changes, odometer values can reset to near-zero and cascade incorrectly through all subsequent trips. Confirmed in production dev DB where trips 31-34 had odometers of 382, 12, 386, 756 instead of ~59400-59800.

## Requirements

- `handleUpdate` must refresh trips before recalculating (same pattern as `handleSaveNew`)
- Remove `recalculateNewerTripsOdo` (dead code after fix, eliminates buggy sort)
- Backend sorts should use `sort_order` instead of `odometer` as tiebreaker for deterministic ordering
- Backend tests for same-datetime trip sort correctness

## Technical Notes

- ADR-008: business logic belongs in backend. Odometer recalculation is currently a frontend concern — this fix doesn't change that architecture, just fixes the bugs
- `recalculateAllOdo` already uses `sort_order` for ordering (correct pattern)
- Backend `calculate_odometer_start` already sorts by full datetime but uses `odometer` as final tiebreaker
- Backend `get_year_start_odometer` sorts by date only + odometer tiebreaker (missing time sort)
