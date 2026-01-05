# Review Log - Agentic Workflow Improvements

**Goal:** Polished AI-agentic workflow that just works. No overengineering.

---

## Iterations

| Iteration | Date | Focus | Key Changes |
|-----------|------|-------|-------------|
| 1 | 2026-01-05 | Critical review - overengineering check | Major cuts, simplifications |
| 2 | Pending | Refinement | - |
| 3 | Pending | Refinement | - |
| 4 | Pending | Final polish | - |

---

## Iteration 1: Critical Review

**Status:** Complete

### Executive Summary

The proposals contain good ideas buried under significant overengineering. The 5 hooks, 5 new skills, and CLAUDE.md restructure are overkill for a single-developer project. The 80/20 rule applies strongly here.

**Verdict:** Keep 2 hooks, 1-2 new skills, skip the CLAUDE.md restructure. Focus on what actually enforces quality, not what looks comprehensive on paper.

---

### 1. HOOKS: Cut from 5 to 2

**Assessment:** Hooks are the most valuable proposal because they provide actual enforcement. But 5 hooks is overkill.

| Hook | Verdict | Reasoning |
|------|---------|-----------|
| Pre-commit test runner | **KEEP - ESSENTIAL** | Only item that actually enforces TDD. High value. |
| Post-commit changelog reminder | **KEEP - USEFUL** | Addresses real problem (forgotten changelogs). Low effort. |
| Idle notification | **CUT** | Annoying, will be ignored. Agents already have CLAUDE.md. |
| Session start guidance | **CUT** | CLAUDE.md already loaded at session start. Redundant. |
| Write/Edit linting | **CUT** | Slows development, Svelte errors visible anyway. |

**Specific Problems Found:**

1. **SessionStart hook doesn't exist** - The proposal admits "availability varies" but still plans for it. Cut it.

2. **Idle notification will be ignored** - Agents don't respond to reminders they can't act on. This just adds noise.

3. **Linting hook is premature optimization** - Svelte-check already runs during dev. Adding it to every Write/Edit adds latency for minimal benefit.

**Action:** Implement only pre-commit.ps1 and post-commit-reminder.ps1.

---

### 2. SKILLS: Cut from 5 new to 1

**Assessment:** The existing 4 skills are good and sufficient. Most proposed skills duplicate what's in CLAUDE.md or superpowers.

| Proposed Skill | Verdict | Reasoning |
|----------------|---------|-----------|
| verify-skill | **KEEP - USEFUL** | Consolidates checklist. Prevents "done" claims without verification. |
| tdd-skill | **CUT** | TDD workflow is in CLAUDE.md. Pre-commit hook enforces tests. Skill is redundant. |
| review-skill | **CUT** | This is what superpowers:requesting-code-review does. Don't duplicate. |
| debug-skill | **CUT** | superpowers:systematic-debugging exists. Project-specific tips belong in CLAUDE.md. |
| pre-implementation-skill | **CUT** | task-plan-skill + brainstorming already covers this. Adding another checklist is bureaucracy. |

**Why verify-skill is the only keeper:**

- It has a clear, narrow purpose: run `npm run test:backend`, check git status, check changelog
- It's actionable (runs commands, produces pass/fail)
- It prevents the actual problem (premature completion claims)

**Why the others fail:**

1. **tdd-skill** - The pre-commit hook already enforces tests. A skill that says "write test first" is just documentation disguised as a skill. If you need to remind the agent to do TDD, put it in CLAUDE.md (which already does).

2. **review-skill** - The proposal says "Check code against project ADRs" but ADR-008 is already mentioned 5+ times in CLAUDE.md. This is documentation, not a skill.

3. **debug-skill** - The proposal is 150 lines of debugging tips. That's documentation, not automation. Put Rust debugging tips in CLAUDE.md if needed.

4. **pre-implementation-skill** - This proposes a 20-item checklist before writing code. That's bureaucracy. The task-plan-skill already runs brainstorming and creates plans.

**Action:** Create verify-skill only. Update existing skills with lightweight cross-references (not the verbose additions proposed).

---

### 3. CLAUDE.md RESTRUCTURE: Skip It

**Assessment:** The proposal wants to reduce CLAUDE.md from 264 to 150 lines. This sounds good but the effort/benefit ratio is poor.

**Problems with the proposal:**

1. **The current CLAUDE.md is already well-structured** - It has clear sections, a table of contents via headers, and puts the most important info (ADR-008, TDD) near the top.

2. **"Quick Reference" is redundant** - Agents read the whole file. Adding a summary section means maintaining two versions of the same info.

3. **Moving architecture diagram is pointless** - The diagram is 15 lines and provides immediate visual context. ARCHITECTURE.md exists for deep dives.

4. **The proposed restructure is MORE work to maintain** - Cross-references between files means updates in multiple places.

5. **Line count is a vanity metric** - 264 lines is not "too long" for an agent. The issue isn't length, it's organization. The current organization is fine.

**What CLAUDE.md actually needs:**

Minor tweaks, not a restructure:

1. Move "Common Pitfalls" higher (it's currently at line 114, move to ~line 50)
2. Add skill trigger hints to the skills table
3. That's it.

**Action:** Make 2 minor edits to CLAUDE.md instead of restructuring.

---

### 4. SKILL ENHANCEMENTS: Simplify

The proposal wants to add verbose cross-references to all 4 existing skills. This is overkill.

**Proposed changes to task-plan-skill:**
- Add 30+ lines about TDD, ADR-008, changelog reminders
- **Problem:** This duplicates CLAUDE.md content in every skill

**Better approach:**
- Add ONE line to each skill: "See CLAUDE.md for project constraints."
- Skills should do their job, not repeat project documentation

**Action:** Add minimal cross-references, not verbose duplications.

---

### 5. WHAT'S ACTUALLY MISSING

The proposals miss some practical issues:

1. **No skip mechanism for hooks** - What if tests are slow and you want to commit a WIP? Need `--no-verify` equivalent or settings.local.json override.

2. **No error handling in hook scripts** - The PowerShell scripts don't handle edge cases (cargo not installed, wrong directory, etc.)

3. **No validation that hooks work** - Should test hooks before considering them done.

4. **Windows path issues** - PowerShell scripts use forward slashes in some places. Need to verify Windows compatibility.

---

### 6. REVISED PRIORITY LIST

**Do This (3-4 hours total):**

| Priority | Item | Effort | Why |
|----------|------|--------|-----|
| 1 | Pre-commit test hook | 1 hour | Actually enforces TDD |
| 2 | Post-commit changelog reminder | 30 min | Addresses real problem |
| 3 | verify-skill | 30 min | Prevents premature completion |
| 4 | Move "Common Pitfalls" in CLAUDE.md | 10 min | Quick visibility improvement |
| 5 | Add skip mechanism to hooks | 30 min | Practical necessity |

**Do Not Do:**

- SessionStart hook (doesn't exist)
- Idle notification hook (noise)
- Write/Edit linting hook (too slow)
- tdd-skill (redundant with hook)
- review-skill (redundant with superpowers)
- debug-skill (documentation, not a skill)
- pre-implementation-skill (bureaucracy)
- CLAUDE.md restructure (unnecessary)
- Verbose skill cross-references (duplicates CLAUDE.md)

---

### 7. SPECIFIC FILE CHANGES

**Create:**
- `.claude/hooks/pre-commit.ps1` - Test runner (with error handling)
- `.claude/hooks/post-commit-reminder.ps1` - Changelog reminder
- `.claude/settings.json` - Hook configuration
- `.claude/skills/verify-skill/SKILL.md` - Completion verification

**Modify:**
- `CLAUDE.md` - Move Common Pitfalls section higher (lines 114-121 to ~50)
- Existing skills - Add ONE line each: "See CLAUDE.md for project constraints"

**Do Not Create:**
- `.claude/hooks/idle-reminder.ps1`
- `.claude/hooks/session-start.ps1`
- `.claude/skills/tdd-skill/`
- `.claude/skills/review-skill/`
- `.claude/skills/debug-skill/`
- `.claude/skills/pre-implementation-skill/`

---

### 8. REVISED ROADMAP

**Phase 1: Essential Automation (2 hours)**

1. Create `.claude/hooks/` directory
2. Create `pre-commit.ps1` with:
   - Git commit detection
   - Test execution
   - Error handling for missing cargo/wrong directory
   - Exit code 2 on failure
3. Create `post-commit-reminder.ps1`
4. Create `.claude/settings.json`
5. Test both hooks work

**Phase 2: Minimal Documentation (30 min)**

1. Move Common Pitfalls in CLAUDE.md to line ~50
2. Add skill trigger hints to skills table
3. Create verify-skill

**Phase 3: Polish (30 min)**

1. Add skip mechanism (document settings.local.json override)
2. Update existing skills with single-line cross-reference
3. Test end-to-end workflow

**Total: ~3 hours** (down from ~1 day proposed)

---

### 9. SUCCESS CRITERIA (Simplified)

| Metric | Target |
|--------|--------|
| Commits blocked if tests fail | 100% |
| Changelog reminder shown after commits | 100% |
| Time to implement | <4 hours |
| New files created | 4 (not 10+) |
| CLAUDE.md changes | Minimal (one section move) |

---

### Changes Made to Proposal Files

Based on this review, the following updates should be made:

1. **02-hooks-proposal.md** - Reduce to 2 hooks only
2. **03-skills-proposal.md** - Reduce to 1 new skill only
3. **04-documentation-proposal.md** - Replace restructure with minor edit
4. **05-final-roadmap.md** - Simplify to 3 phases, ~3 hours

---

### Conclusion

The original proposals suffer from "comprehensive-itis" - the desire to cover every possible scenario. In practice:

- 5 hooks become 2
- 5 new skills become 1
- 264-line restructure becomes 10-line edit
- 1-day effort becomes 3-hour effort

The 80/20 rule wins. Focus on what actually enforces quality (pre-commit hook) and prevents mistakes (changelog reminder, verify-skill). Everything else is documentation that already exists or bureaucracy that won't be followed.
