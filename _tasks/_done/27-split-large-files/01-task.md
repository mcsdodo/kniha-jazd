**Date:** 2026-01-07
**Subject:** Split Large Rust Files (commands.rs, db.rs) into Modules
**Status:** Planning

## Goal

Split two large Rust files into organized module directories for better maintainability and easier AI context handling:
- `commands.rs` (3,044 lines) → `commands/` module (11 submodules)
- `db.rs` (1,806 lines) → `db/` module (7 submodules)

## Problem

The current large files exceed AI context limits (~2000 lines) and mix multiple domains in single files:
- **commands.rs**: 40 Tauri commands across 8 domains (vehicle, trip, receipt, backup, etc.)
- **db.rs**: CRUD operations for 5 entities + migrations + 650 lines of tests

## Requirements

### Functional
- No breaking changes to frontend - all Tauri commands keep same signatures
- All 108 tests must continue passing
- Maintain clear module boundaries (no circular dependencies)

### Technical
- Follow project test pattern: `#[path = "*_tests.rs"]` for test modules
- Use `pub use` re-exports so external code doesn't need changes
- Split db.rs first (no dependents), then commands.rs

## Success Criteria

- [ ] All tests pass after refactor
- [ ] No file exceeds ~850 lines
- [ ] `cargo clippy` reports no new warnings
- [ ] `npm run tauri dev` runs without issues

## Out of Scope

- Splitting Svelte components (TripGrid, settings page, etc.)
- Splitting export.rs (~560 lines - manageable size)
- Refactoring logic within functions
