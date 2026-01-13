# Tech Debt: Dead Code and Compiler Warnings Cleanup

**Date:** 2026-01-13
**Priority:** Low
**Effort:** Low (<2h)
**Component:** Multiple files (see inventory below)
**Status:** Open

## Problem

The codebase has 17 warnings (14 Rust, 3 Svelte) when building:
- 14 Rust warnings: unused variables, dead code, lifetime syntax
- 3 Svelte warnings: a11y issues, empty CSS ruleset

These warnings clutter build output and make it harder to spot real issues.

## Impact

- Build output noise obscures real warnings
- New developers may waste time investigating "unused" code
- Some dead code is truly orphaned, some is scaffolding for planned features

## Root Cause

Multiple factors contributed to this debt:

1. **EV Feature Revert (7a0cd67)**: Full EV implementation was built then reverted on 2026-01-07, leaving scaffolding methods in models.rs
2. **Route Feature Incomplete**: BIZ-005 planned route distance memory; backend CRUD exists but frontend never wired
3. **Abandoned Error Handling**: `AppError` enum created during Diesel migration but `Result<_, String>` pattern used instead
4. **Orphaned Functions**: `calculate_buffer_km`, `generate_target_margin` exist but never called

## Inventory

### Category A: Truly Dead Code (DELETE)

| File | Item | Reason |
|------|------|--------|
| `commands.rs:472` | `vehicle_uuid` unused variable | Simple fix: prefix with `_` |
| `calculations.rs:73` | `calculate_buffer_km()` | Never called; suggestions use different approach |
| `error.rs:4` | `AppError` enum | Never used; String errors used instead |
| `export.rs:149` | `is_dummy_trip()` | No dummy trips in current flow |
| `suggestions.rs:19` | `generate_target_margin()` | Never called; hardcoded range used |

### Category B: EV Scaffolding (KEEP + SUPPRESS)

| File | Item | Reason to Keep |
|------|------|----------------|
| `calculations_energy.rs` | All functions | Next EV attempt (task 19) will use this |
| `calculations_phev.rs` | `PhevTripConsumption` struct/fields | Part of EV implementation plan |
| `models.rs` | `uses_fuel()`, `uses_electricity()` | Part of VehicleType API |
| `models.rs` | `new()`, `new_ice()`, `new_bev()`, `new_phev()` | Convenience constructors for EV |
| `models.rs` | `is_charge()`, `has_soc_override()` | Trip helpers for EV support |
| `models.rs` | `Receipt::new()`, `is_assigned()` | Useful Receipt helpers |

### Category C: Route Feature (KEEP + SUPPRESS)

| File | Item | Reason to Keep |
|------|------|----------------|
| `db.rs` | `in_memory()` | Useful for testing |
| `db.rs` | Route CRUD methods | BIZ-005 planned feature |

### Category D: Syntax Fix (FIX)

| File | Item | Fix |
|------|------|-----|
| `db.rs:65` | Lifetime syntax | Add `'_` to `MutexGuard<'_, SqliteConnection>` |

### Category E: Svelte Warnings (FIX)

| File | Issue | Fix |
|------|-------|-----|
| `VehicleModal.svelte:65` | a11y: click without keyboard | Add `on:keydown` handler or use `<button>` |
| `+page.svelte:353` | Empty CSS ruleset | Remove `.trip-section {}` |
| `settings/+page.svelte:360` | Label without control | Add `for` attribute or wrap input |

## Recommended Solution

**Phase 1: Quick fixes (15 min)**
1. Delete truly dead code (Category A)
2. Fix lifetime syntax (Category D)
3. Fix Svelte warnings (Category E)

**Phase 2: Organized suppression (15 min)**
1. Add `#[allow(dead_code)]` to EV scaffolding with comment: `// EV support - task 19`
2. Add `#[allow(dead_code)]` to Route CRUD with comment: `// Route feature - BIZ-005`

## Alternative Options

1. **Delete all unused code**: Cleaner, but loses work. Can recreate from git history if needed.
2. **Leave warnings**: Status quo, but build noise continues.
3. **Disable warnings globally**: Bad practice - hides real issues.

## Related

- `_tasks/19-electric-vehicles/` - EV implementation plan (status: Planning)
- `DECISIONS.md` → BIZ-005: Route Distance Memory
- Revert commit: `7a0cd67`

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-13 | Created analysis | User noticed warnings during dev server startup |
| 2026-01-13 | Recommend keep EV/Route scaffolding | Active task 19 and BIZ-005 reference this code |
