**Date:** 2026-01-05
**Subject:** Final Roadmap - Agentic Workflow Improvements
**Status:** Revised after Iteration 1 Review

---

# Executive Summary

**REVISED:** After critical review (see `_review.md`), the original proposal was significantly simplified. The 80/20 rule applies - focus on automation that enforces quality, not comprehensive documentation.

## Final Recommendations (Post-Review)

| Rank | Recommendation | Impact | Effort | Why |
|------|----------------|--------|--------|-----|
| 1 | **Pre-commit test hook** | Critical | 1 hour | Only item that actually enforces TDD |
| 2 | **Post-commit changelog reminder** | High | 30 min | Addresses real problem (forgotten changelogs) |
| 3 | **verify-skill** | Medium | 30 min | Prevents premature "done" claims |
| 4 | **Move Common Pitfalls in CLAUDE.md** | Low | 10 min | Quick visibility improvement |

**CUT from original proposal:**
- ~~Restructure CLAUDE.md~~ (unnecessary - current structure is fine)
- ~~tdd-skill~~ (redundant - pre-commit hook enforces tests)
- ~~review-skill~~ (redundant - superpowers:requesting-code-review exists)
- ~~debug-skill~~ (documentation, not automation)
- ~~pre-implementation-skill~~ (bureaucracy - task-plan already covers this)
- ~~SessionStart hook~~ (doesn't exist in Claude Code)
- ~~Idle notification hook~~ (noise, will be ignored)
- ~~Write/Edit linting hook~~ (too slow, errors visible in dev anyway)

## Key Metrics

| Metric | Current State | Target |
|--------|---------------|--------|
| Commits blocked if tests fail | 0% | 100% |
| Changelog reminder after commits | No | Yes |
| Implementation time | N/A | <4 hours |
| New files created | N/A | 4 files |

## Success Criteria

1. **Commits blocked if tests fail** - Pre-commit hook works
2. **Changelog reminder shown after commits** - Post-commit hook works
3. **verify-skill usable** - Skill invocable and runs checks

---

# REVISED Implementation Roadmap (Post-Review)

> **Note:** This roadmap was significantly simplified after Iteration 1 review.
> See `_review.md` for full rationale. Original 4-phase plan reduced to 2 phases.

## Phase 1: Essential Automation (2-2.5 hours)

**Goal:** Implement the two hooks that actually enforce quality.

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

**pre-commit.ps1 requirements:**
- Detect `git commit` commands (not other Bash commands)
- Run `cargo test` in src-tauri directory
- Exit 2 to block commit on failure
- Handle edge cases: wrong directory, cargo not found
- Provide clear error messages

**Why:** Only item that actually enforces TDD.

---

### Task 1.2: Create Post-Commit Changelog Reminder (30 min)

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

**post-commit-reminder.ps1 requirements:**
- Detect successful `git commit` commands
- Skip if commit message mentions CHANGELOG
- Display prominent reminder to run /changelog

**Why:** Addresses most common workflow gap.

---

### Task 1.3: Create verify-skill (30 min)

**File to create:**
- `.claude/skills/verify-skill/SKILL.md`

**Content (simplified from original):**

```markdown
---
name: verify-skill
description: Use before claiming work is complete - runs tests, checks git status, verifies changelog
---

# Verification Before Completion

Run this before saying "task complete" or "done".

## Checklist

### 1. Tests Pass
```bash
npm run test:backend
```
Do NOT proceed if tests fail.

### 2. Code Committed
```bash
git status
```
All work-related files should be committed.

### 3. Changelog Updated
Check CHANGELOG.md [Unreleased] section has entry for this work.
If not, run /changelog.

## Quick Verification
```powershell
# Run tests
cd src-tauri && cargo test

# Check status
git status

# Check changelog
head -20 CHANGELOG.md
```

See CLAUDE.md for project constraints.
```

---

### Task 1.4: Add Hook Skip Mechanism (15 min)

Document in CLAUDE.md or create `.claude/settings.local.json`:

```json
{
  "hooks": {
    "PreToolUse": []
  }
}
```

This allows disabling hooks locally for WIP commits.

---

## Phase 2: Minimal Documentation Updates (30 min)

### Task 2.1: Move Common Pitfalls in CLAUDE.md (10 min)

Move lines 114-121 (Common Pitfalls section) to after line 50 (after TDD section).

**Current location:** Line 114
**New location:** Line ~50 (after "What to Test" section)

This is the only CLAUDE.md change needed.

---

### Task 2.2: Add Skill Hints to Skills Table (10 min)

Update the skills table in CLAUDE.md:

```markdown
| Skill | Purpose | Trigger |
|-------|---------|---------|
| `/task-plan` | Create planning folder | "plan feature", "new task" |
| `/changelog` | Update CHANGELOG.md | After completing work (MANDATORY) |
| `/decision` | Record ADR/BIZ | "should I use X or Y?" |
| `/release` | Version bump + build | "release", "publish" |
| `/verify` | Check before "done" | Before claiming complete |
```

---

### Task 2.3: Add Cross-Reference to Existing Skills (10 min)

Add ONE line to each existing skill file:

```markdown
## Project Context
See CLAUDE.md for project constraints (ADR-008, TDD, changelog requirements).
```

Files: task-plan-skill, decision-skill, changelog-skill, release-skill

---

## Implementation Checklist (Simplified)

### Phase 1: Essential Automation
- [ ] Create `.claude/hooks/` directory
- [ ] Create `.claude/hooks/pre-commit.ps1` with error handling
- [ ] Create `.claude/settings.json` with PreToolUse hook
- [ ] Test: try commit with failing test (should block)
- [ ] Test: try commit with passing tests (should succeed)
- [ ] Create `.claude/hooks/post-commit-reminder.ps1`
- [ ] Add PostToolUse hook to settings.json
- [ ] Test: commit non-changelog file (should show reminder)
- [ ] Test: commit changelog (should NOT show reminder)
- [ ] Create `.claude/skills/verify-skill/SKILL.md`
- [ ] Test: invoke /verify skill
- [ ] Document skip mechanism

### Phase 2: Documentation Updates
- [ ] Move Common Pitfalls section in CLAUDE.md
- [ ] Add trigger hints to skills table
- [ ] Add cross-reference line to 4 existing skills

---

## What NOT To Do

The following were in the original proposal but are CUT:

| Item | Reason |
|------|--------|
| CLAUDE.md restructure | Unnecessary - current structure is fine |
| tdd-skill | Redundant - pre-commit hook enforces tests |
| review-skill | Redundant - superpowers:requesting-code-review exists |
| debug-skill | Documentation, not automation |
| pre-implementation-skill | Bureaucracy - task-plan covers this |
| SessionStart hook | Doesn't exist in Claude Code |
| Idle notification hook | Noise, will be ignored |
| Write/Edit linting hook | Too slow |
| Verbose skill cross-references | Duplicates CLAUDE.md |
| ARCHITECTURE.md updates | Not needed |

---

## Summary

**Original proposal:** 4 phases, ~1 day, 10+ new files
**Revised plan:** 2 phases, ~3 hours, 4 new files

| Metric | Original | Revised |
|--------|----------|---------|
| Hooks | 5 | 2 |
| New skills | 5 | 1 |
| CLAUDE.md changes | Full restructure | One section move |
| Time estimate | 1 day | 3 hours |
| Files created | 10+ | 4 |

The 80/20 rule wins.

---

## References

- `_review.md` - Critical review explaining cuts
- `02-hooks-proposal.md` - Original hook specs (use selectively)
- `03-skills-proposal.md` - Original skill specs (verify-skill only)
