# Plan Review: Configurable Date Prefill

**Date:** 2026-01-26
**Reviewer:** Claude Opus 4.5
**Plan:** 03-plan.md
**Status:** Ready (Important findings addressed)

## Summary

| Category | Count |
|----------|-------|
| Critical | 0 |
| Important | 3 |
| Minor | 4 |

**Recommendation:** Ready to implement with minor adjustments.

---

## Findings

### Important (Should fix before implementing)

**1. [x] Missing TDD order - backend tests before implementation**

The plan lists "Add DatePrefillMode Enum" (Step 1.1) before "Add Backend Tests" (Step 1.2). Per CLAUDE.md, tests must come first.

**Recommendation:** Reorder to:
1. Write failing tests in `settings.rs` (Step 1.2)
2. Then add enum to make tests pass (Step 1.1)

---

**2. [x] Integration test placement unclear**

Plan says `tests/integration/specs/date-prefill.spec.ts` (new file). However, existing integration tests are organized by tier (tier1/tier2/tier3). This test should go in a tier folder.

**Recommendation:** Place in `tests/integration/specs/tier2/date-prefill.spec.ts` (settings-related, not critical path). Update Step 5.1 with correct path.

---

**3. [x] Design decision: Commands use AppState vs AppHandle**

Plan shows commands taking `State<AppState>`, but existing theme commands use `tauri::AppHandle`:

```rust
// Existing pattern (get_theme_preference)
pub fn get_theme_preference(app_handle: tauri::AppHandle) -> Result<String, String>
```

Using `AppState` won't work - `LocalSettings` needs the app data directory from `AppHandle`.

**Recommendation:** Update design in 02-design.md to use `tauri::AppHandle` parameter (not `State<AppState>`). Follow exact pattern from `get_theme_preference`/`set_theme_preference`.

---

### Minor (Nice to have, can skip)

**4. No explicit check_read_only clarification**

Step 1.3 notes "no check_read_only! needed" which is correct (these are local settings, not DB writes). However, this could be clearer.

**Recommendation:** Add brief explanation: "LocalSettings are per-machine, not database-dependent, so read-only mode doesn't apply."

---

**5. TripGrid header layout details missing**

Step 4.1 says "Add toggle to grid header (next to 'Novy zaznam' button)" but doesn't specify exact HTML structure or CSS considerations.

Looking at TripGrid.svelte lines 438-444:
```svelte
<div class="header">
  <h2>{$LL.trips.title()} ({trips.length})</h2>
  <button class="new-record" ...>
```

The toggle needs to go between h2 and button, or in a new wrapper div.

**Recommendation:** Add brief note: "Add toggle between h2 and button, or wrap both in a flex container with gap."

---

**6. defaultNewDate reactive block needs context**

The plan references "Update defaultNewDate reactive block to check mode" but doesn't show the current logic. For implementer clarity:

Current (lines 428-435):
```svelte
$: defaultNewDate = (() => {
  if (sortedTrips.length === 0) {
    return new Date().toISOString().split('T')[0];
  }
  const maxDate = new Date(sortedTrips[0].date);
  maxDate.setDate(maxDate.getDate() + 1);
  return maxDate.toISOString().split('T')[0];
})();
```

**Recommendation:** Add snippet showing the planned change:
```svelte
$: defaultNewDate = (() => {
  if (datePrefillMode === 'today' || sortedTrips.length === 0) {
    return new Date().toISOString().split('T')[0];
  }
  // Previous mode: +1 from last trip
  const maxDate = new Date(sortedTrips[0].date);
  maxDate.setDate(maxDate.getDate() + 1);
  return maxDate.toISOString().split('T')[0];
})();
```

---

**7. Backend test file location**

Step 1.2 says tests go "in existing mod tests" in settings.rs. The file already has `#[cfg(test)] mod tests` at line 55. This is fine, but inconsistent with CLAUDE.md which says "Write tests in `*_tests.rs` companion file."

For a simple enum with 3 tests, inline is acceptable. But for consistency with project patterns, a separate file could be used.

**Recommendation:** Keep inline for simplicity (only 3 tests), but note this is acceptable for small test counts.

---

## Checklist Verification

- [x] All tasks have specific file paths
- [x] Verification steps included (tests, manual checks)
- [x] Steps in correct dependency order (with adjustment needed for TDD)
- [x] No scope creep beyond task requirements

## Edge Cases Covered

From 02-design.md:
- [x] No trips exist: Both modes return today (handled in defaultNewDate logic)
- [x] Insert between trips: Uses existing behavior (not affected)
- [x] Year change: Setting is global (correct - LocalSettings is global)

## Alignment with Project Conventions

- [x] ADR-008: Backend-only calculations (N/A - this is a UI preference, no calculations)
- [x] TDD workflow: Tests specified (order needs adjustment)
- [x] i18n: Translation keys specified for SK and EN
- [x] Changelog: Step 6.1 included

---

## Final Recommendation

**Ready to implement** after addressing:
1. ~~Reorder Step 1.1 and 1.2 for TDD compliance~~ ✅ Fixed
2. ~~Update command signature to use `AppHandle` not `AppState`~~ ✅ Fixed
3. ~~Specify correct integration test tier path~~ ✅ Fixed

The plan is well-structured and covers all requirements from 01-task.md. Implementation complexity is low (simple setting persistence + UI toggle).

---

## Resolution

**Date:** 2026-01-26

All 3 Important findings addressed:
- Plan reordered: tests (1.1) → implementation (1.2) for TDD compliance
- Design updated: commands use `tauri::AppHandle` parameter
- Test path updated: `tests/integration/specs/tier2/date-prefill.spec.ts`

Minor findings (4-7) skipped as acceptable.
