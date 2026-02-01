# Code Review

**Target:** HEAD~1..HEAD (constants migration refactoring)
**Reference:** _tasks/_done/38-magic-strings-refactoring/03-plan.md
**Started:** 2026-02-01
**Status:** Ready for User Review
**Focus:** Quality, correctness, best practices

**Baseline Test Status:** Pass (235 tests)

## Iteration 1

### What Was Done Well

1. **Clean constant organization** - The `constants.rs` module is well-structured with logical submodules
2. **Proper enum design** - New enums include `as_str()` methods for string conversion, maintaining API compatibility
3. **Gradual adoption approach** - Constants marked with `#[allow(dead_code)]` for future use
4. **Test verification** - Basic sanity tests added in `constants.rs`

### New Findings

#### Important

- [Important] `db_location.rs:80-90` - `resolve_db_paths` still uses hardcoded `"kniha-jazd.db"`, `"kniha-jazd.lock"`, `"backups"` strings instead of `paths::DB_FILENAME`, `paths::LOCK_FILENAME`, `paths::BACKUPS_DIR`

- [Important] `lib.rs:236` - Exit handler uses hardcoded `"kniha-jazd.lock"` instead of `paths::LOCK_FILENAME`

#### Minor

- [Minor] `commands/mod.rs:2280,2356` - Two occurrences of `"%Y-%m-%d"` remain; could use `date_formats::ISO_DATE`

- [Minor] `export.rs:237-238,454` - Several `"%d.%m."` patterns remain; consider adding `DISPLAY_DATE_SHORT` constant

- [Minor] Test files use magic strings directly - acceptable for test clarity but inconsistent

- [Minor] `#[allow(dead_code)]` annotations on infrastructure constants - intentional per plan, could be follow-up task

### Test Gaps

None identified - all 235 tests pass.

### Coverage Assessment

| Component | Status |
|-----------|--------|
| backup.rs | ✅ Complete |
| commands/mod.rs | ⚠️ Partial (2 date formats) |
| export.rs | ⚠️ Partial (short date format) |
| gemini.rs | ✅ Complete |
| receipts.rs | ✅ Complete |
| db_location.rs | ⚠️ Partial (hardcoded strings) |
| lib.rs | ⚠️ Partial (lock filename) |

---

## Review Summary

**Status:** Ready for User Review
**Iterations:** 1
**Total Findings:** 0 Critical, 2 Important, 4 Minor
**Test Status:** Pass (235 tests)

### All Findings (Consolidated)

#### Critical
(none)

#### Important
1. [ ] `db_location.rs:80-90` - Use `paths::DB_FILENAME`, `paths::LOCK_FILENAME`, `paths::BACKUPS_DIR` in `resolve_db_paths`
2. [ ] `lib.rs:236` - Use `paths::LOCK_FILENAME` in exit handler

#### Minor
1. [ ] `commands/mod.rs:2280,2356` - Use `date_formats::ISO_DATE` instead of `"%Y-%m-%d"`
2. [ ] `export.rs:237-238,454` - Consider adding `DISPLAY_DATE_SHORT` constant for `"%d.%m."`
3. [ ] Test files use magic strings - No action needed (clarity over consistency in tests)
4. [ ] `#[allow(dead_code)]` on infrastructure constants - Intentional, follow-up task later

### Recommendation

**Approved with minor issues.** The core infrastructure is solid and tests pass. The 2 Important issues should be fixed to maintain consistency with the refactoring goals.
