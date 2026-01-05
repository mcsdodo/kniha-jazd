**Date:** 2026-01-05
**Subject:** Agentic Workflow Improvements Analysis
**Status:** Complete

## Goal

Analyze the kniha-jazd solution's agentic workflow and suggest improvements across:
1. Automation & Hooks
2. Skill Enhancement
3. Documentation & Onboarding
4. Workflow Enforcement

## Current State Summary

### Existing Skills (4)
| Skill | Purpose | Quality |
|-------|---------|---------|
| `task-plan-skill` | Create planning folders, runs brainstorming | Good |
| `decision-skill` | Record ADR/BIZ decisions | Good |
| `changelog-skill` | Update CHANGELOG.md in Slovak | Good |
| `release-skill` | Full release workflow | Good |

### Existing Commands (4)
All commands are simple wrappers: `/task-plan`, `/decision`, `/changelog`, `/release`

### Hooks
**None exist** - All workflow enforcement is manual/discipline-based.

### Documentation Structure
- `CLAUDE.md` - Main project instructions (comprehensive)
- `ARCHITECTURE.md` - System design overview
- `DECISIONS.md` - ADR/BIZ decision log
- `CHANGELOG.md` - Version history in Slovak
- `_tasks/` - 20 task folders with planning docs
- `_tasks/_TECH_DEBT/` - Tech debt tracking

---

## Gap Analysis

### 1. AUTOMATION & HOOKS (Critical Gap)

**Current:** No hooks exist. All workflow steps rely on memory and discipline.

**Problems:**
- TDD workflow is documented but not enforced
- Changelog updates are easily forgotten
- No pre-commit quality checks
- No automated reminders for decision documentation

**Opportunities:**
- Pre-commit hooks for lint/format checks
- Post-commit hooks for changelog reminders
- Notification hooks for workflow guidance
- Session-start hooks for context loading

### 2. SKILL ENHANCEMENT (Moderate Gap)

**Current:** 4 well-designed project skills, uses superpowers marketplace skills.

**Problems:**
- No project-specific debugging skill (uses generic superpowers:systematic-debugging)
- No code review skill tailored to project patterns
- No refactoring skill that knows backend-only calculation rule
- Skills don't cross-reference each other well

**Opportunities:**
- Project-specific debugging skill with Rust/Tauri patterns
- Code review skill that checks ADR-008 compliance
- TDD enforcement skill
- Pre-implementation checklist skill

### 3. DOCUMENTATION & ONBOARDING (Minor Gap)

**Current:** CLAUDE.md is comprehensive but dense.

**Problems:**
- CLAUDE.md is 260+ lines - information overload for new sessions
- Key patterns buried in prose (need better visual hierarchy)
- No "quick start" section for common tasks
- Settings permissions not documented in CLAUDE.md

**Opportunities:**
- Add quick reference section at top
- Create visual decision flowcharts
- Add common task examples section
- Document allowed superpowers skills

### 4. WORKFLOW ENFORCEMENT (Significant Gap)

**Current:** Relies on checklist in CLAUDE.md footer.

**Problems:**
- TDD "mandatory" but no verification
- Changelog "mandatory" but easily skipped
- Decision documentation is optional in practice
- Task completion has no automated verification

**Opportunities:**
- Pre-implementation checklist enforcement
- Post-completion verification skill
- Automated test run before commit
- Workflow state tracking

---

## Initial Recommendations

### Priority 1: Add Hooks (High Impact)
1. Create `.claude/hooks/pre-commit.md` - Run tests, lint checks
2. Create session-start hook - Load context, remind workflows
3. Create notification hook - Post-implementation changelog reminder

### Priority 2: New Skills (High Impact)
1. `tdd-enforcement-skill` - Verify test exists before implementation
2. `pre-implementation-checklist-skill` - Ensure planning is done
3. `project-debug-skill` - Rust/Tauri debugging patterns
4. `adr-compliance-skill` - Check code against ADRs

### Priority 3: Documentation Improvements (Medium Impact)
1. Add "Quick Reference" section to top of CLAUDE.md
2. Create visual workflow diagrams
3. Add "Common Tasks" examples section
4. Document superpowers skill integrations

### Priority 4: Workflow Enforcement (Medium Impact)
1. Add verification step to task-plan-skill
2. Create post-completion checklist skill
3. Add cross-skill references

---

## Iteration Log

| Iteration | Agent Focus | Key Findings |
|-----------|-------------|--------------|
| 0 | Initial exploration | Current state documented above |
| 1 | Hooks & workflow enforcement | See detailed findings below |
| 2 | Skill improvements | See `03-skills-proposal.md` - 5 new skills, 4 skill enhancements |
| 3 | Documentation & onboarding | See `04-documentation-proposal.md` - CLAUDE.md restructure, 40% reduction |
| 4 | **Final synthesis** | See `05-final-roadmap.md` - Prioritized implementation plan |

### Iteration 4 Summary (Complete)

**Deliverable:** `05-final-roadmap.md` - Complete implementation roadmap

**Top 5 Recommendations:**
1. Pre-commit test hook (Critical impact, 1 hour effort)
2. Restructure CLAUDE.md (High impact, 2 hours effort)
3. verify-skill (High impact, 30 min effort)
4. Post-commit changelog reminder (Medium impact, 30 min effort)
5. tdd-skill (Medium impact, 30 min effort)

**Phase Summary:**
| Phase | Effort | Items | Impact |
|-------|--------|-------|--------|
| 1: Quick Wins | 1-2 hours | Pre-commit hook, changelog reminder, verify-skill | High |
| 2: Core Improvements | Half day | CLAUDE.md restructure, tdd-skill, skill updates | High |
| 3: Full Implementation | 1-2 days | review/debug/pre-impl skills, testing | Medium |
| 4: Future Enhancements | TBD | SessionStart hook, linting, cross-platform | Low |

**Success Metrics:**
- 100% commits blocked if tests fail
- 100% changelog updates after commits
- 5 second time-to-find critical info (down from 30-60 sec)
- 150 line CLAUDE.md (down from 264)

**Total Investment:** ~1 day across all phases
**Expected Return:** Automated quality enforcement, reduced mistakes, faster onboarding

---

## Iteration 1: Hooks & Workflow Enforcement

### Research Findings: Claude Code Hook System

Claude Code supports lifecycle hooks that execute shell commands at specific points. This is the key mechanism for enforcing workflows.

#### Available Hook Events

| Event | Trigger | Best Use Case |
|-------|---------|---------------|
| `PreToolUse` | Before Claude executes a tool | Block commits if tests fail |
| `PostToolUse` | After tool completion | Remind about changelog |
| `Notification` | Claude sends notifications | Show checklist on idle |
| `Stop` | Session ends | Final cleanup/reminders |
| `SessionStart` | Session begins | Load context, display guidance |
| `UserPromptSubmit` | User submits prompt | Pre-process requests |
| `PreCompact` | Before transcript compaction | Save state |

#### Configuration Location

Hooks are defined in settings files:
- `.claude/settings.json` - Project-wide (committed)
- `.claude/settings.local.json` - Local overrides (gitignored)

#### Matcher Syntax

- `"*"` - All tools
- `"Bash"` - Specific tool
- `"Write|Edit"` - Regex OR pattern
- `""` - Empty (for events without tool context)

#### Exit Codes

- `0` - Success, continue
- `1` - Error, display to user
- `2` - **Block the operation**

#### Environment Variables

- `$CLAUDE_PROJECT_DIR` - Project root
- `$CLAUDE_TOOL_INPUT` - JSON tool parameters (via stdin)
- `$CLAUDE_FILE_PATHS` - Affected files
- `$CLAUDE_NOTIFICATION` - Notification message

### Proposed Hook Strategy

**See: `02-hooks-proposal.md` for complete implementation details.**

#### Hook 1: Pre-Commit Test Runner (Critical)

**Purpose:** Enforce TDD by running `cargo test` before any git commit.

**Configuration:**
```json
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

**Behavior:**
- Intercepts all Bash commands
- Checks if command is `git commit`
- Runs `cargo test` in src-tauri/
- Blocks commit (exit 2) if tests fail
- Allows commit if tests pass

**Impact:** Converts TDD from "mandatory in docs" to "enforced by automation."

#### Hook 2: Post-Commit Changelog Reminder (High Value)

**Purpose:** Never forget changelog after completing work.

**Behavior:**
- Triggers after successful `git commit`
- Displays prominent reminder to run `/changelog`
- Skips if commit already includes CHANGELOG

**Impact:** Addresses the most common workflow gap.

#### Hook 3: Idle Notification Checklist (Medium Value)

**Purpose:** Show completion checklist when Claude is waiting for input.

**Behavior:**
- Uses `Notification` event with `idle_prompt` matcher
- Displays task completion checklist:
  - [ ] Tests pass
  - [ ] Code committed
  - [ ] /changelog updated

**Impact:** Gentle reminder without blocking workflow.

#### Hook 4: Session Start Guidance (Nice to Have)

**Purpose:** Display project context at session start.

**Note:** SessionStart hook availability varies. May need alternative approach.

### Implementation Files Needed

| File | Purpose |
|------|---------|
| `.claude/settings.json` | Hook configuration (committed) |
| `.claude/hooks/pre-commit.ps1` | Test runner script |
| `.claude/hooks/post-commit-reminder.ps1` | Changelog reminder |
| `.claude/hooks/idle-reminder.ps1` | Completion checklist |

### Limitations Discovered

1. **No native PreCommit/PostCommit hooks** - Feature request open (#4834)
2. **Workaround required** - Use Bash matcher with command pattern matching
3. **Windows compatibility** - Need PowerShell scripts (project is Windows-based)
4. **Exit code 2 blocking** - May not work in all scenarios

### Integration with Existing Skills

The hooks complement existing skills:

| Skill | Hook Integration |
|-------|------------------|
| `/changelog` | Post-commit hook reminds to run this |
| `/decision` | Could add hook to detect architectural discussions |
| `/release` | Pre-commit tests ensure release quality |
| `/task-plan` | Session start could remind about planning |

### Recommendations for This Project

**Immediate (Phase 1):**
1. Create `.claude/hooks/` directory
2. Add `pre-commit.ps1` - test enforcement
3. Add `post-commit-reminder.ps1` - changelog reminders
4. Create `.claude/settings.json` with hook config

**Short-term (Phase 2):**
1. Add idle notification hook
2. Test and refine timeouts
3. Add skip mechanism for local development

**Future (Phase 3):**
1. Add SessionStart guidance (when/if supported)
2. Add Write/Edit linting hooks
3. Create cross-platform scripts

### References

- [Claude Code Hooks Reference](https://code.claude.com/docs/en/hooks)
- [Git Workflow Feature Request #4834](https://github.com/anthropics/claude-code/issues/4834)
- [GitButler Hook Integration](https://docs.gitbutler.com/features/ai-integration/claude-code-hooks)
- [Demystifying Hooks](https://www.brethorsting.com/blog/2025/08/demystifying-claude-code-hooks/)
- [Complete Hook Guide](https://suiteinsider.com/complete-guide-creating-claude-code-hooks/)

---

## Files to Create/Modify

### New Files (from Iteration 1 - Hooks)
- [ ] `.claude/settings.json` - Hook configuration (committed to repo)
- [ ] `.claude/hooks/pre-commit.ps1` - Test runner before commits
- [ ] `.claude/hooks/post-commit-reminder.ps1` - Changelog reminder
- [ ] `.claude/hooks/idle-reminder.ps1` - Completion checklist

### New Files (from Iteration 2 - Skills)
- [ ] `.claude/skills/verify-skill/SKILL.md` - Verification before completion (P1)
- [ ] `.claude/skills/tdd-skill/SKILL.md` - TDD enforcement (P1)
- [ ] `.claude/skills/review-skill/SKILL.md` - Code review checklist (P2)
- [ ] `.claude/skills/debug-skill/SKILL.md` - Project-specific debugging (P2)
- [ ] `.claude/skills/pre-implementation-skill/SKILL.md` - Pre-impl checklist (P2)

### Modified Files
- [ ] `CLAUDE.md` - Add quick reference, improve structure
- [ ] `.claude/settings.local.json` - May need hook permissions
- [ ] `.claude/skills/task-plan-skill/SKILL.md` - Add cross-references, TDD mention
- [ ] `.claude/skills/decision-skill/SKILL.md` - Add context linking
- [ ] `.claude/skills/changelog-skill/SKILL.md` - Add verification step
- [ ] `.claude/skills/release-skill/SKILL.md` - Add pre-release checklist

---

## Iteration 2: Skill Improvements & New Skills

**See: `03-skills-proposal.md` for complete skill specifications.**

### Analysis Summary

#### Existing Skills Assessment

| Skill | Strengths | Improvements Needed |
|-------|-----------|---------------------|
| `task-plan-skill` | Integrates brainstorming, structured output | Missing cross-refs to /decision, /changelog, TDD |
| `decision-skill` | Clear format, good examples | No link to task context, no integration prompts |
| `changelog-skill` | Simple workflow, Slovak language | No verification that work is complete |
| `release-skill` | Comprehensive steps | No pre-release checklist, no test verification |

#### Key Gaps Identified

1. **No verification before completion** - Skills don't verify tests pass before claiming done
2. **No TDD enforcement** - Despite being mandatory, no skill enforces test-first
3. **No ADR-008 compliance checking** - Skills don't verify backend-only calculations
4. **Missing cross-references** - Skills operate in isolation
5. **No project-specific debugging** - Uses generic superpowers:systematic-debugging

### Proposed New Skills (5)

| Skill | Purpose | Priority |
|-------|---------|----------|
| `verify-skill` | Final checks before claiming complete | P1 - Most impactful |
| `tdd-skill` | Enforce test-first development | P1 - Core principle |
| `review-skill` | Check ADR compliance before commit | P2 - Quality gate |
| `debug-skill` | Rust/Tauri debugging patterns | P2 - Project-specific |
| `pre-implementation-skill` | Ensure planning is done first | P2 - Workflow coverage |

### Skill Workflow Integration

```
Planning: /task-plan → brainstorming → pre-implementation-skill
     ↓
Implementation: tdd-skill → debug-skill (if issues) → /decision (if choices)
     ↓
Completion: review-skill → verify-skill → /changelog → commit
     ↓
Release: /release (includes pre-release checklist)
```

### Hook Integration Points

| Skill | Hook Integration |
|-------|------------------|
| `verify-skill` | Called by idle notification hook |
| `tdd-skill` | Reinforced by pre-commit test runner |
| `/changelog` | Triggered by post-commit reminder |
| `/release` | Uses pre-commit hook for test enforcement |

### Improvements to Existing Skills

1. **task-plan-skill**
   - Add cross-references to /decision and /changelog
   - Include TDD as first task in plan template
   - Reference ADR-008 for calculation-related features

2. **decision-skill**
   - Add context linking to task folders
   - Add integration prompts during planning/debugging

3. **changelog-skill**
   - Add pre-changelog verification step (tests must pass)
   - Clarify that changelog = feature DONE, not in progress

4. **release-skill**
   - Add pre-release checklist (tests, compliance, changelog)
   - Reference ADR-008 verification

### Implementation Priority

**Phase 1 (Essential):**
1. Create `verify-skill` - Prevents premature completion claims
2. Create `tdd-skill` - Enforces core project principle
3. Update existing 4 skills with cross-references

**Phase 2 (Important):**
4. Create `review-skill` - ADR compliance checking
5. Create `debug-skill` - Project-specific debugging
6. Create `pre-implementation-skill` - Complete workflow coverage

**Phase 3 (Enhancement):**
7. Integrate skills with hooks from `02-hooks-proposal.md`
8. Test full workflow end-to-end

### Skill Trigger Matrix

| Trigger | Skill to Use |
|---------|--------------|
| Starting new feature | `/task-plan` |
| Before writing code | `pre-implementation-skill` |
| Before implementing task | `tdd-skill` |
| Bug or test failure | `debug-skill` |
| Architectural choice | `/decision` |
| Before committing | `review-skill` |
| Claiming "done" | `verify-skill` |
| After code committed | `/changelog` |
| Releasing version | `/release` |

---

---

## Iteration 3: Documentation & Onboarding

**See: `04-documentation-proposal.md` for complete restructuring proposal.**

### Analysis Summary

#### CLAUDE.md Current Problems

| Issue | Impact | Solution |
|-------|--------|----------|
| 264 lines - too long | Information overload | Target 150 lines (40% reduction) |
| No quick reference at top | Agent scans entire file | Add Quick Reference section first |
| Critical Constraints buried | Core rules missed | Move to top, add visual emphasis |
| Completion checklist at bottom | Often not seen | Reference at top, keep at bottom |
| Duplicates with other files | Maintenance burden | Remove, add cross-references |

#### Information Priority Analysis

**Category A (Every Session):**
- Project purpose (1 line)
- Core constraint (ADR-008)
- Available skills + triggers
- Common commands
- Completion checklist

**Category B (Reference):**
- Code patterns
- Project structure
- Business rules details
- Git guidelines

**Category C (Move Elsewhere):**
- Architecture diagram (ARCHITECTURE.md)
- CI/CD details (workflow files)
- Test counts by file (derivable)

### Proposed CLAUDE.md Structure

```
1. Quick Reference (commands, skills, constraint summary)
2. Critical Constraints (ADR-008, TDD, pitfalls)
3. Common Tasks (copy-paste examples)
4. Reference (structure, patterns, business rules)
5. More Information (cross-references)
6. Task Completion Checklist (emphasized)
```

### Scannability Improvements

| Information | Before | After |
|-------------|--------|-------|
| Test command | ~15 sec | ~5 sec |
| ADR-008 rule | ~20 sec | ~5 sec |
| Common pitfalls | ~30 sec | ~5 sec |
| Completion checklist | ~60 sec | ~5 sec |

### Key Recommendations

1. **Restructure top-to-bottom by priority**
2. **Add Quick Reference section at top**
3. **Move Critical Constraints up with visual emphasis**
4. **Add Common Tasks with copy-paste examples**
5. **Remove redundant content (architecture diagram, test counts)**
6. **Add cross-references to other files**
7. **Document allowed superpowers skills**

### Files to Modify

| File | Change | Priority |
|------|--------|----------|
| `CLAUDE.md` | Full restructure per proposal | P1 |
| `ARCHITECTURE.md` | Ensure has complete content | P2 |
| `.claude/skills/*.md` | Add trigger phrases | P3 |

---

*This analysis will be refined through 4 iterations with specialized agents.*
