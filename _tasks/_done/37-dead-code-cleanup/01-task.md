**Date:** 2026-01-13
**Subject:** Dead Code and Compiler Warnings Cleanup
**Status:** Planning
**Source:** `_TECH_DEBT/03-dead-code-and-warnings.md`

---

# Task: Dead Code Cleanup

## Overview

The codebase has 17 compiler warnings (14 Rust, 3 Svelte). Investigation revealed multiple categories of dead code with different root causes.

## Findings Summary

### Category 1: Removed Suggestion Feature (v0.12.0)

The "auto-suggest compensation trip" feature was **intentionally simplified** in v0.12.0. The app now only shows "you need X km" without suggesting specific trips. Related code was never removed.

| File | Item | Status |
|------|------|--------|
| `suggestions.rs:19` | `generate_target_margin()` | DELETE |
| `suggestions.rs:26` | `find_matching_route()` | DELETE |
| `suggestions.rs:47` | `build_compensation_suggestion()` | DELETE |
| `suggestions.rs:9` | `CompensationSuggestion` struct | DELETE |
| `commands.rs:413` | `get_compensation_suggestion` command | DELETE |
| `api.ts:181` | `getCompensationSuggestion()` | DELETE |
| `types.ts:62` | `CompensationSuggestion` type | DELETE |
| `lib.rs:62` | Command registration | DELETE |

**Note:** `calculate_buffer_km()` in `calculations.rs` should be KEPT - it's needed for the simplified feature but isn't wired up yet (separate bug).

### Category 2: EV Feature (Partially Implemented)

EV feature was merged (PR #1) but **integration incomplete**. DB schema, models, and UI exist but `get_trip_grid_data` doesn't call energy calculations. See [task 19](../19-electric-vehicles/03-status.md) for details.

| File | Item | Action |
|------|------|--------|
| `calculations_energy.rs` | All functions | KEEP - will be used when integration completed |
| `calculations_phev.rs` | `PhevTripConsumption` struct | KEEP - will be used |
| `models.rs` | `uses_fuel()`, `uses_electricity()` | KEEP - will be used |
| `models.rs` | `new()`, `new_ice()`, `new_bev()`, `new_phev()` | KEEP - convenience constructors |
| `models.rs` | `is_charge()`, `has_soc_override()` | KEEP - will be used |

**Note:** These warnings will resolve when task 19 is completed. No action needed in this task.

### Category 3: Unused Route CRUD (DELETE)

Route feature IS working via `get_routes_for_vehicle()` and `find_or_create_route()`. But manual CRUD operations were written "just in case" and never used.

| File | Item | Status | Action |
|------|------|--------|--------|
| `db.rs` | `get_routes_for_vehicle()` | ✅ Used | KEEP |
| `db.rs` | `find_or_create_route()` | ✅ Used | KEEP |
| `db.rs` | `in_memory()` | ❌ Never used | DELETE |
| `db.rs` | `create_route()` | ❌ Never used | DELETE |
| `db.rs` | `get_route()` | ❌ Never used | DELETE |
| `db.rs` | `update_route()` | ❌ Never used | DELETE |
| `db.rs` | `delete_route()` | ❌ Never used | DELETE |
| `db.rs` | `populate_routes_from_trips()` | ❌ Never used | DELETE |

### Category 4: Truly Dead Code

| File | Item | Analysis | Action |
|------|------|----------|--------|
| `error.rs` | `AppError` enum | Created for Diesel migration, never adopted | DELETE entire file |
| `export.rs:149` | `is_dummy_trip()` | Checks `distance_km <= 0`, never called | DELETE |
| `commands.rs:472` | `vehicle_uuid` | Parses UUID but uses string instead | FIX: prefix with `_` |
| `models.rs` | `Receipt::new()` | Used in tests (commands.rs:3520) | KEEP |
| `models.rs:439` | `Receipt::is_assigned()` | Never used, code uses `trip_id.is_some()` | DELETE |

### Category 5: Syntax/Style Issues

| File | Issue | Fix |
|------|-------|-----|
| `db.rs:65` | Elided lifetime confusing | `MutexGuard<'_, SqliteConnection>` |
| `VehicleModal.svelte:65` | Backdrop needs keyboard handler | Add `on:keydown` for Escape |
| `+page.svelte:353` | Empty `.trip-section {}` | DELETE ruleset |
| `settings/+page.svelte:360` | Theme label not associated | Use `<fieldset>` + `<legend>` |

### Category 6: Incomplete Implementation (Bug)

| File | Item | Issue |
|------|------|-------|
| `+page.svelte:47` | `bufferKm = 100` | Hardcoded placeholder |
| `calculations.rs:73` | `calculate_buffer_km()` | Exists but never called |
| `models.rs` | `TripStats` | Missing `buffer_km` field |

This is a **separate bug** - the compensation banner shows incorrect km.

## Acceptance Criteria

- [ ] Warnings reduced (EV warnings remain until task 19 completed)
- [ ] Build output cleaner (`cargo check` + `npm run check`)
- [ ] No regression in existing functionality
- [ ] Tech debt doc updated with resolution

## Implementation Order

1. **Phase 1:** Delete removed suggestion feature code (6 warnings)
2. **Phase 2:** Delete unused Route CRUD (6 functions, 1 warning group)
3. **Phase 3:** Fix truly dead code and syntax issues (3 warnings)
4. **Phase 4:** Fix Svelte warnings (3 warnings)

**Note:** EV-related warnings (5) will be resolved by task 19, not this task.

## Out of Scope

- Fixing the `bufferKm = 100` bug (separate task)
- Completing EV feature (task 19)
- Wiring Route feature (future task)
