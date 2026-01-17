# Plan Review

**Target:** `_tasks/19-electric-vehicles/02-plan.md`
**Reference:** `01-task.md`, `research.md`, `technical-analysis.md`
**Started:** 2026-01-06
**Focus:** Completeness, feasibility, clarity

---

## Iteration 1 - 2026-01-06

### Critical Issues (1)

| ID | Issue | Location | Resolution |
|----|-------|----------|------------|
| CRITICAL-1 | Year boundary contradiction | `02-plan.md` Phase 4.1 vs `01-task.md` lines 62-63 | Task says NO carryover (100% each year), Plan says carryover. Technical-analysis supports task. **Fix Plan Phase 4.1** |

### Important Issues (5)

| ID | Issue | Location | Resolution |
|----|-------|----------|------------|
| IMPORTANT-1 | i18n translations not explicit | Phase 2-3 UI tasks | Add explicit i18n task before UI work |
| IMPORTANT-2 | `trip_processor.rs` in tech-analysis but not plan | `technical-analysis.md` line 316 | Add to plan or update tech-analysis |
| IMPORTANT-3 | Vehicle type immutability needs explicit test | Phase 2.1, Testing Strategy | Add `test_update_vehicle_blocks_type_change_when_trips_exist` |
| IMPORTANT-4 | SoC override processing too late | Phase 4.2 vs Phase 2.6 | Move SoC processing to Phase 2.4 |
| IMPORTANT-5 | ICE year boundary fix is out of scope | Phase 4.1 | Remove or create separate tech debt |

### Minor Issues (5)

| ID | Issue | Resolution |
|----|-------|------------|
| MINOR-1 | Phase 4.4 Statistics vague/YAGNI | Defer or remove |
| MINOR-2 | Phase 4 Receipts explicitly out of scope | Remove from tech-analysis |
| MINOR-3 | Initial battery % default unclear | Clarify: default 100% in calculations, not DB |
| MINOR-4 | Test count minimums may miss edge cases | Add zero/large value tests |
| MINOR-5 | Rollback plan doesn't address behavior changes | Update after year boundary decision |

### Acceptance Criteria Coverage

| Criterion | Status | Notes |
|-----------|--------|-------|
| Create BEV vehicle | ✅ Covered | Phase 1.2, 2.1, 2.5 |
| Create PHEV vehicle | ✅ Covered | Phase 1.2, 3.3 |
| Initial battery % | ✅ Covered | Phase 1.2, 2.5 |
| BEV trip tracking | ✅ Covered | Phase 2.2-2.7 |
| PHEV consumption split | ✅ Covered | Phase 3.1 |
| No margin for electricity | ✅ Covered | Phase 2.4 |
| PHEV fuel margin only | ✅ Covered | Phase 3.2 |
| Vehicle type immutable | ⚠️ Needs test | Phase 2.1, 2.5 |
| SoC override | ⚠️ Wrong phase | Phase 4.2 too late |
| Year boundary | ❌ Contradiction | Phase 4.1 conflicts with task |
| Export for EV | ✅ Covered | Phase 2.8, 3.6 |
| ICE unchanged | ⚠️ At risk | Phase 4.1 changes ICE |
| Tests pass | ✅ Covered | Phase verification |

### Verdict

**Ready for implementation?** No - Critical contradiction must be resolved

---

## Iteration 2 - 2026-01-06

**User clarification:** Carryover IS correct behavior (already implemented for ICE). Plan is correct; task/tech-analysis need updating.

### Critical Issues - RESOLVED

| ID | Original Issue | Resolution |
|----|----------------|------------|
| CRITICAL-1 | Year boundary contradiction | **PLAN IS CORRECT.** Task and technical-analysis need updating to match carryover behavior. |

### Important Issues - Updated

| ID | Issue | Status | Resolution |
|----|-------|--------|------------|
| IMPORTANT-1 | i18n translations not explicit | **VALID** | Add i18n task before Phase 2.5 UI work |
| IMPORTANT-2 | `trip_processor.rs` orphaned | **VALID** | Decide: add to plan OR remove from tech-analysis |
| IMPORTANT-3 | Vehicle type immutability untested | **VALID** | Add explicit test to Testing Strategy |
| IMPORTANT-4 | SoC override processing too late | **VALID** | Move from Phase 4.2 to Phase 2.4 |
| IMPORTANT-5 | ICE year boundary fix out of scope | **INVALID** | Removed - ICE is already correct |

### New Issues Found

| ID | Issue | Resolution |
|----|-------|------------|
| NEW-1 | Phase 4.1 "ICE needs fix" is wrong | Remove ICE section from Phase 4.1 - already correct |
| NEW-2 | Year-end calculation missing SoC override | Add note that year-end must respect SoC overrides |
| NEW-3 | Acceptance criteria line 84 contradicts carryover | Update to reflect carryover behavior |

### Documents Requiring Updates

#### `01-task.md`
| Lines | Change |
|-------|--------|
| 62-63 | Change "no carry-over" to "carryover (matching ICE)" |
| 84 | Update year boundary acceptance criterion |

#### `technical-analysis.md`
| Lines | Change |
|-------|--------|
| 17 | Change "100% at year start" to "carries over" |
| 223-241 | Rewrite Year Boundary Logic section for carryover |
| 316 | Decide on `trip_processor.rs` |

#### `02-plan.md`
| Location | Change |
|----------|--------|
| Phase 4.1 ICE block | Remove - ICE is already correct |
| Phase 4.1 | Add SoC override consideration for year-end |
| Phase 4.2 | Move SoC override to Phase 2.4 |
| Phase 2-3 | Add explicit i18n task |
| Testing Strategy | Add vehicle type immutability test |

### Final Verdict

**Ready for implementation?** Yes - with document updates listed above

**Confidence:** High - plan structure is sound, changes are synchronization/refinement only

---

## Review Complete

**Iterations:** 2
**Exit reason:** All issues identified, no Critical blockers remaining
**Next step:** User reviews findings, then implements document updates

---

## Document Updates Applied - 2026-01-06

All identified issues have been resolved:

### `01-task.md`
- ✅ Lines 62-63: Changed "no carry-over" to "carryover (matching ICE)"
- ✅ Line 84: Updated year boundary acceptance criterion

### `technical-analysis.md`
- ✅ Line 17: Changed "100% at year start" to "carryover"
- ✅ Lines 221-247: Rewrote Year Boundary Logic section for carryover
- ✅ Line 316: Removed `trip_processor.rs` - routing handled in commands.rs

### `02-plan.md`
- ✅ Phase 4.1: Removed incorrect "ICE needs fix" block
- ✅ Phase 4.1: Added SoC override consideration to year-end calculation
- ✅ Phase 2.5: Added explicit i18n task with Slovak translations
- ✅ Phase 4.2: Added note about integrating core logic into Phase 2.4
- ✅ Testing Strategy: Added explicit vehicle type immutability test
- ✅ Renumbered sections 2.5-2.9

**Plan Status:** Ready for implementation

