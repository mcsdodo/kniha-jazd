**Date:** 2026-01-05
**Subject:** Final Roadmap - Agentic Workflow Improvements
**Status:** Complete

---

# Executive Summary

This roadmap synthesizes findings from 4 iterations of analysis covering hooks, skills, and documentation improvements for the kniha-jazd project's agentic workflow.

## Top 5 Recommendations (Ranked by Impact)

| Rank | Recommendation | Impact | Effort | Why |
|------|----------------|--------|--------|-----|
| 1 | **Pre-commit test hook** | Critical | 1 hour | Enforces TDD - converts "mandatory" from docs to automation |
| 2 | **Restructure CLAUDE.md** | High | 2 hours | Reduces cognitive load 40%, critical info visible immediately |
| 3 | **verify-skill** | High | 30 min | Prevents premature completion claims, ensures quality |
| 4 | **Post-commit changelog reminder** | Medium | 30 min | Addresses most common workflow gap |
| 5 | **tdd-skill** | Medium | 30 min | Provides test-first guidance, reinforces core principle |

## Key Metrics to Track

| Metric | Current State | Target | How to Measure |
|--------|---------------|--------|----------------|
| Commits blocked by tests | 0% (no enforcement) | 100% | Hook exit code 2 count |
| Changelog updates after commits | Unknown (manual) | 100% | Changelog reminder dismissals |
| Time to find critical info | ~30-60 sec | ~5 sec | Subjective (restructured doc) |
| Incomplete work claims | Common | Rare | verify-skill usage |
| CLAUDE.md length | 264 lines | ~150 lines | Line count |

## Success Criteria

1. **No commit passes without tests passing** - Pre-commit hook blocks failures
2. **Every completed feature has changelog entry** - Reminder hook enforces
3. **Agent finds critical constraints in <10 seconds** - Quick Reference at top
4. **Verification runs before "done" claims** - verify-skill integrated into workflow
5. **New sessions understand project in <30 seconds** - Restructured CLAUDE.md

---

# Prioritized Implementation Roadmap

## Phase 1: Quick Wins (1-2 hours, HIGH impact)

**Goal:** Get immediate enforcement of core workflows with minimal effort.

### Task 1.1: Create Pre-Commit Test Hook (1 hour)

**Files to create:**
- `.claude/hooks/pre-commit.ps1` - Test runner script
- `.claude/settings.json` - Hook configuration

**Implementation:**
```json
// .claude/settings.json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{
        "type": "command",
        "command": "pwsh -NoProfile -File .claude/hooks/pre-commit.ps1",
        "timeout": 120000
      }]
    }]
  }
}
```

**Why first:** Single most impactful change - converts TDD from "recommended" to "enforced."

**Dependencies:** None
**Risk:** Low - if hook breaks, can disable in settings.local.json

---

### Task 1.2: Create Post-Commit Changelog Reminder (30 min)

**Files to create:**
- `.claude/hooks/post-commit-reminder.ps1`

**Add to settings.json:**
```json
"PostToolUse": [{
  "matcher": "Bash",
  "hooks": [{
    "type": "command",
    "command": "pwsh -NoProfile -File .claude/hooks/post-commit-reminder.ps1",
    "timeout": 5000
  }]
}]
```

**Why second:** Addresses most common workflow gap - forgetting to update changelog.

**Dependencies:** Task 1.1 (settings.json exists)
**Risk:** Low - reminder only, doesn't block

---

### Task 1.3: Create verify-skill (30 min)

**File to create:**
- `.claude/skills/verify-skill/SKILL.md`

**Content:** See `03-skills-proposal.md` Section 3.5

**Why third:** Prevents premature "task complete" claims. Quick to implement.

**Dependencies:** None
**Risk:** Low - skill is advisory

---

## Phase 2: Core Improvements (Half day, ESSENTIAL)

**Goal:** Complete the workflow enforcement system and improve documentation.

### Task 2.1: Restructure CLAUDE.md (2 hours)

**Changes:**
1. Add Quick Reference section at top (commands, skills, constraints)
2. Move Critical Constraints up (ADR-008, TDD, pitfalls)
3. Add Common Tasks section with copy-paste examples
4. Remove architecture diagram (move to ARCHITECTURE.md)
5. Remove test counts (derivable from running tests)
6. Add cross-reference section

**Target:** 264 lines -> ~150 lines (40% reduction)

**Dependencies:** None
**Risk:** Medium - must preserve all critical information

---

### Task 2.2: Create tdd-skill (30 min)

**File to create:**
- `.claude/skills/tdd-skill/SKILL.md`

**Content:** See `03-skills-proposal.md` Section 3.1

**Why:** Provides structured guidance for test-first development.

**Dependencies:** None
**Risk:** Low

---

### Task 2.3: Update Existing Skills with Cross-References (1 hour)

**Files to modify:**
- `.claude/skills/task-plan-skill/SKILL.md` - Add TDD, /decision, /changelog refs
- `.claude/skills/decision-skill/SKILL.md` - Add context linking
- `.claude/skills/changelog-skill/SKILL.md` - Add verification step
- `.claude/skills/release-skill/SKILL.md` - Add pre-release checklist

**Changes per skill:** See `03-skills-proposal.md` Part 2

**Dependencies:** Task 2.2 (tdd-skill exists for references)
**Risk:** Low - additive changes

---

### Task 2.4: Create Idle Notification Hook (30 min)

**File to create:**
- `.claude/hooks/idle-reminder.ps1`

**Add to settings.json:**
```json
"Notification": [{
  "matcher": "idle_prompt",
  "hooks": [{
    "type": "command",
    "command": "pwsh -NoProfile -File .claude/hooks/idle-reminder.ps1",
    "timeout": 5000
  }]
}]
```

**Why:** Gentle reminder of completion checklist when idle.

**Dependencies:** Task 1.1 (settings.json exists)
**Risk:** Low - if notification event not supported, no harm

---

## Phase 3: Full Implementation (1-2 days, COMPLETE workflow)

**Goal:** Complete skill ecosystem and polish documentation.

### Task 3.1: Create review-skill (45 min)

**File to create:**
- `.claude/skills/review-skill/SKILL.md`

**Content:** See `03-skills-proposal.md` Section 3.4

**Purpose:** ADR compliance checking before commit.

**Dependencies:** None
**Risk:** Low

---

### Task 3.2: Create debug-skill (45 min)

**File to create:**
- `.claude/skills/debug-skill/SKILL.md`

**Content:** See `03-skills-proposal.md` Section 3.3

**Purpose:** Project-specific Rust/Tauri debugging patterns.

**Dependencies:** None
**Risk:** Low

---

### Task 3.3: Create pre-implementation-skill (45 min)

**File to create:**
- `.claude/skills/pre-implementation-skill/SKILL.md`

**Content:** See `03-skills-proposal.md` Section 3.2

**Purpose:** Ensure planning is complete before coding starts.

**Dependencies:** None
**Risk:** Low

---

### Task 3.4: Update ARCHITECTURE.md (30 min)

**Changes:**
- Ensure architecture diagram is complete
- Add any content moved from CLAUDE.md
- Cross-reference to DECISIONS.md for ADRs

**Dependencies:** Task 2.1 (CLAUDE.md restructured)
**Risk:** Low

---

### Task 3.5: End-to-End Workflow Testing (2 hours)

**Verification:**
1. Start new session - Quick Reference visible?
2. Create feature with /task-plan - cross-refs work?
3. Write code - pre-commit blocks without tests?
4. Commit code - changelog reminder appears?
5. Mark complete - verify-skill checks pass?
6. Run /changelog - updates correctly?
7. Run /release - pre-checklist works?

**Dependencies:** All previous tasks
**Risk:** Medium - may reveal integration issues

---

## Phase 4: Future Enhancements (Nice to have)

These items provide value but are not essential for core workflow.

### Task 4.1: SessionStart Hook (if supported)

**Purpose:** Display project context at session start.

**Blocked by:** Claude Code SessionStart hook availability

---

### Task 4.2: Write/Edit Linting Hook

**Purpose:** Run svelte-check after editing frontend files.

**Consideration:** May slow development; make configurable.

---

### Task 4.3: Cross-Platform Hook Scripts

**Purpose:** Support non-Windows development.

**Action:** Create `.sh` equivalents of PowerShell scripts.

---

### Task 4.4: Skill Usage Analytics

**Purpose:** Track which skills are used most/least.

**Consideration:** Low priority - focus on workflow first.

---

# Implementation Checklist

## Phase 1: Quick Wins

- [ ] Create `.claude/hooks/` directory
- [ ] Create `.claude/hooks/pre-commit.ps1`
- [ ] Create `.claude/settings.json` with PreToolUse hook
- [ ] Test pre-commit hook (try commit with failing test)
- [ ] Create `.claude/hooks/post-commit-reminder.ps1`
- [ ] Add PostToolUse hook to settings.json
- [ ] Test changelog reminder (commit without CHANGELOG)
- [ ] Create `.claude/skills/verify-skill/SKILL.md`
- [ ] Test verify-skill invocation

## Phase 2: Core Improvements

- [ ] Backup current CLAUDE.md
- [ ] Restructure CLAUDE.md (add Quick Reference, move constraints)
- [ ] Reduce CLAUDE.md to ~150 lines
- [ ] Verify all critical info preserved
- [ ] Create `.claude/skills/tdd-skill/SKILL.md`
- [ ] Update task-plan-skill with cross-references
- [ ] Update decision-skill with context linking
- [ ] Update changelog-skill with verification step
- [ ] Update release-skill with pre-release checklist
- [ ] Create `.claude/hooks/idle-reminder.ps1`
- [ ] Add Notification hook to settings.json

## Phase 3: Full Implementation

- [ ] Create `.claude/skills/review-skill/SKILL.md`
- [ ] Create `.claude/skills/debug-skill/SKILL.md`
- [ ] Create `.claude/skills/pre-implementation-skill/SKILL.md`
- [ ] Update ARCHITECTURE.md with moved content
- [ ] Run end-to-end workflow test
- [ ] Fix any integration issues discovered
- [ ] Document any new decisions in DECISIONS.md

## Phase 4: Future Enhancements

- [ ] Monitor SessionStart hook support
- [ ] Evaluate need for linting hooks
- [ ] Create cross-platform scripts if needed
- [ ] Consider skill usage tracking

---

# Dependency Graph

```
Phase 1 (Parallel work possible):
  Task 1.1 (pre-commit hook) ─┐
                              ├─→ Task 1.2 (changelog reminder)
  Task 1.3 (verify-skill) ────┘

Phase 2 (Some dependencies):
  Task 2.1 (CLAUDE.md restructure) ─→ Task 3.4 (ARCHITECTURE.md update)
  Task 2.2 (tdd-skill) ─→ Task 2.3 (update existing skills)
  Task 2.4 (idle hook) depends on Task 1.1 (settings.json)

Phase 3 (Mostly independent):
  Tasks 3.1, 3.2, 3.3 can run in parallel
  Task 3.5 (testing) depends on all previous
```

---

# Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Pre-commit hook too slow | Medium | Low | Add timeout, allow skip in settings.local.json |
| Hook blocks legitimate commits | Low | Medium | Exit code 1 shows error, doesn't block |
| CLAUDE.md restructure loses info | Low | High | Review before/after, keep backup |
| Skills not used | Medium | Low | Integrate with hooks, remind in docs |
| Hooks not supported on all events | Medium | Low | Fallback to documentation reminders |

---

# Next Steps

1. **Immediate:** Implement Phase 1 (Quick Wins) - ~1.5 hours
2. **This week:** Complete Phase 2 (Core Improvements) - ~half day
3. **Next sprint:** Phase 3 (Full Implementation) - ~1 day
4. **Ongoing:** Monitor and iterate based on usage

---

# Summary

This roadmap transforms the kniha-jazd project's agentic workflow from documentation-based discipline to automation-enforced quality:

**Before:**
- TDD is "mandatory" in docs but not enforced
- Changelog updates are easily forgotten
- Critical constraints are buried in 264-line CLAUDE.md
- Skills operate in isolation without cross-references
- No verification before claiming work complete

**After:**
- Pre-commit hook blocks commits if tests fail
- Post-commit reminder ensures changelog updates
- Quick Reference at top of ~150-line CLAUDE.md
- Skills cross-reference each other for complete workflow
- verify-skill ensures quality before completion claims

**Investment:** ~1 day total across all phases
**Return:** Consistent quality, reduced mistakes, faster onboarding

---

## References

- `01-analysis.md` - Initial analysis and gap identification
- `02-hooks-proposal.md` - Detailed hook specifications
- `03-skills-proposal.md` - New skills and enhancements
- `04-documentation-proposal.md` - CLAUDE.md restructure proposal
