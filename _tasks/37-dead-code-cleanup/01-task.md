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

### Category 2: EV Feature Scaffolding (Reverted)

Full EV implementation was built then reverted (`7a0cd67`). Task 19 is in "Planning" status for future attempt.

| File | Item | Action |
|------|------|--------|
| `calculations_energy.rs` | All functions | SUPPRESS with `#[allow(dead_code)]` |
| `calculations_phev.rs` | `PhevTripConsumption` struct | SUPPRESS |
| `models.rs` | `uses_fuel()`, `uses_electricity()` | SUPPRESS |
| `models.rs` | `new()`, `new_ice()`, `new_bev()`, `new_phev()` | SUPPRESS |
| `models.rs` | `is_charge()`, `has_soc_override()` | SUPPRESS |

### Category 3: Route Feature (Planned, Not Wired)

BIZ-005 specified "Route Distance Memory". Backend CRUD exists but frontend never wired.

| File | Item | Action |
|------|------|--------|
| `db.rs` | `in_memory()` | SUPPRESS (useful for tests) |
| `db.rs` | `create_route`, `get_route`, etc. | SUPPRESS |
| `db.rs` | `populate_routes_from_trips()` | SUPPRESS |

### Category 4: Truly Dead Code

| File | Item | Action |
|------|------|--------|
| `commands.rs:472` | `vehicle_uuid` unused variable | FIX: prefix with `_` |
| `error.rs:4` | `AppError` enum | DELETE entire file |
| `export.rs:149` | `is_dummy_trip()` | DELETE |
| `models.rs` | `Receipt::new()`, `is_assigned()` | KEEP (useful helpers) |

### Category 5: Syntax/Style Issues

| File | Item | Action |
|------|------|--------|
| `db.rs:65` | Lifetime syntax | FIX: Add `'_` to `MutexGuard` |
| `VehicleModal.svelte:65` | a11y: click without keyboard | FIX |
| `+page.svelte:353` | Empty CSS ruleset | DELETE |
| `settings/+page.svelte:360` | Label without control | FIX |

### Category 6: Incomplete Implementation (Bug)

| File | Item | Issue |
|------|------|-------|
| `+page.svelte:47` | `bufferKm = 100` | Hardcoded placeholder |
| `calculations.rs:73` | `calculate_buffer_km()` | Exists but never called |
| `models.rs` | `TripStats` | Missing `buffer_km` field |

This is a **separate bug** - the compensation banner shows incorrect km.

## Acceptance Criteria

- [ ] All 17 warnings resolved
- [ ] Build output is clean (`cargo check` + `npm run check`)
- [ ] No regression in existing functionality
- [ ] Tech debt doc updated with resolution

## Implementation Order

1. **Phase 1:** Delete removed suggestion feature code
2. **Phase 2:** Suppress EV/Route scaffolding with comments
3. **Phase 3:** Fix truly dead code and syntax issues
4. **Phase 4:** Fix Svelte warnings

## Out of Scope

- Fixing the `bufferKm = 100` bug (separate task)
- Completing EV feature (task 19)
- Wiring Route feature (future task)
