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

