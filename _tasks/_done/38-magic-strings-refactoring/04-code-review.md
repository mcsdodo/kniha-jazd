# Code Review

**Target:** HEAD~1..HEAD (constants migration refactoring)
**Reference:** _tasks/_done/38-magic-strings-refactoring/03-plan.md
**Started:** 2026-02-01
**Status:** Complete
**Focus:** Quality, correctness, best practices

**Baseline Test Status:** Pass (235 tests)

## Iteration 1

### What Was Done Well

1. **Clean constant organization** - The `constants.rs` module is well-structured with logical submodules
2. **Proper enum design** - New enums include `as_str()` methods for string conversion, maintaining API compatibility
3. **Gradual adoption approach** - Constants marked with `#[allow(dead_code)]` for future use
4. **Test verification** - Basic sanity tests added in `constants.rs`

### Findings

#### Important
- [x] `db_location.rs:80-90` - Use `paths::DB_FILENAME`, `paths::LOCK_FILENAME`, `paths::BACKUPS_DIR` in `resolve_db_paths`
- [x] `lib.rs:236` - Use `paths::LOCK_FILENAME` in exit handler

#### Minor
- [x] `commands/mod.rs:2280,2356` - Use `date_formats::ISO_DATE` instead of `"%Y-%m-%d"`
- [ ] `export.rs:237-238,454` - Consider adding `DISPLAY_DATE_SHORT` constant for `"%d.%m."` (skipped - minor)
- [N/A] Test files use magic strings - No action needed (clarity over consistency in tests)
- [N/A] `#[allow(dead_code)]` on infrastructure constants - Intentional, follow-up task later

---

## Resolution

**Addressed:** 3 findings (2 Important, 1 Minor)
**Skipped:** 1 Minor finding (DISPLAY_DATE_SHORT - low value)
**Test Status:** All 235 tests passing
**Status:** Complete

### Applied Fixes

1. **`db_location.rs`** - Refactored `resolve_db_paths` to use `DbPaths::from_dir()` helper (which already uses constants), eliminating code duplication

2. **`lib.rs`** - Updated exit handler to use `paths::LOCK_FILENAME` instead of hardcoded string

3. **`commands/mod.rs`** - Replaced 2 occurrences of `"%Y-%m-%d"` with `date_formats::ISO_DATE`

### Skipped Items

- `export.rs` short date format (`"%d.%m."`) - Creating a new constant for this pattern adds complexity for minimal benefit; the format is display-specific and unlikely to change
