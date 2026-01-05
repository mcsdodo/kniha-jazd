**Date:** 2026-01-05
**Subject:** Documentation Structure & Onboarding Improvements
**Status:** Proposal

## Overview

This document proposes improvements to CLAUDE.md structure and project onboarding experience to make information more accessible and reduce cognitive load for Claude sessions.

---

## Part 1: CLAUDE.md Analysis

### 1.1 Current State

**File:** `CLAUDE.md` (264 lines)

**Strengths:**
- Comprehensive coverage of project requirements
- Architecture diagram is clear and helpful
- Business rules are well-documented
- Common pitfalls section exists
- Task completion checklist at the end

**Problems:**

| Issue | Impact | Location |
|-------|--------|----------|
| No quick reference at top | Agent must scan 260+ lines for basics | Entire file |
| TDD workflow buried in middle | Core principle not immediately visible | Lines 39-63 |
| Common Pitfalls buried | Easy to miss, causes repeated mistakes | Lines 114-121 |
| Completion checklist at bottom | Often not seen before claiming "done" | Lines 254-263 |
| Dense paragraphs | Hard to scan quickly | Throughout |
| Missing permission context | Skills in settings.local.json not documented | Not present |
| Redundant info with other files | ARCHITECTURE.md, DECISIONS.md overlap | Multiple sections |

### 1.2 Information Categories

I analyzed CLAUDE.md content into three categories:

**Category A: MUST KNOW (Every Session)**
- Project purpose (1 line)
- Tech stack (quick bullet list)
- ADR-008: Backend-only calculations (core constraint)
- TDD workflow (mandatory)
- Common commands (`npm run test:backend`, `npm run tauri dev`)
- Available skills (`/task-plan`, `/changelog`, `/decision`, `/release`)
- Common Pitfalls (top 3-4 critical ones)
- Task Completion Checklist

**Category B: REFERENCE (Look Up When Needed)**
- Detailed project structure
- Code patterns (Adding Tauri command, Adding calculation)
- Business rules formulas
- Database location
- Git guidelines details
- Test coverage breakdown

**Category C: CAN LIVE ELSEWHERE**
- Architecture diagram (in ARCHITECTURE.md)
- Full test coverage by file (derivable from running tests)
- CI/CD details (in workflow files)

### 1.3 Scannability Assessment

**Can an agent find the right information in under 30 seconds?**

| Information | Current State | Time to Find |
|-------------|---------------|--------------|
| "How to run tests?" | Line 67-75 | ~15 sec |
| "What's ADR-008?" | Line 13-37 (buried in prose) | ~20 sec |
| "What are Common Pitfalls?" | Line 114-121 | ~30 sec |
| "What skills are available?" | Line 227-233 | ~45 sec |
| "What do I do before marking complete?" | Line 254-263 (bottom) | ~60 sec |

**Verdict:** Too slow. Critical information should be at the TOP.

---

## Part 2: Proposed CLAUDE.md Restructuring

### 2.1 New Structure (Top-to-Bottom Priority)

```markdown
# CLAUDE.md

{One-line project description}

## Quick Reference

{Essential info in 20 lines - commands, skills, constraints}

## Critical Constraints (MUST READ)

{ADR-008, TDD, Common Pitfalls - prominently displayed}

## Common Tasks

{Copy-paste examples for frequent operations}

## Detailed Reference

{Everything else - structure, patterns, business rules}

## Task Completion Checklist

{Keep at end but reference it at top}
```

### 2.2 Proposed Quick Reference Section

```markdown
## Quick Reference

### Commands
| Command | Purpose |
|---------|---------|
| `npm run tauri dev` | Start dev server |
| `npm run test:backend` | Run Rust tests (105 tests) |
| `npm run tauri build` | Production build |

### Skills (invoke with /)
| Skill | When to Use |
|-------|-------------|
| `/task-plan` | Starting new feature (runs brainstorming first) |
| `/changelog` | After completing any work (MANDATORY) |
| `/decision` | When making architectural/business choices |
| `/release` | Bump version and publish |

### Critical Constraints
1. **ADR-008:** ALL calculations in Rust backend only - frontend is display-only
2. **TDD:** Write failing test BEFORE implementation
3. **Changelog:** Every feature/fix needs `/changelog` update

### Before Claiming Complete
- [ ] Tests pass (`npm run test:backend`)
- [ ] Code committed
- [ ] `/changelog` run and committed
```

### 2.3 Proposed Critical Constraints Section

```markdown
## Critical Constraints

### ADR-008: Backend-Only Calculations

**NEVER** add calculation logic to frontend. All math lives in Rust:
- `calculations.rs` - Consumption, margin, zostatok
- `suggestions.rs` - Compensation trip logic
- `commands.rs` - Aggregates calculations for frontend

Frontend ONLY calls Tauri commands and displays results.

### TDD Workflow (Mandatory)

```
1. WRITE failing test first
2. WRITE minimal code to pass
3. REFACTOR while tests pass
4. REPEAT
```

Test business logic only. Skip: CRUD, UI rendering, getters/setters.

### Common Pitfalls

| Mistake | Consequence |
|---------|-------------|
| Calculations in frontend | ADR-008 violation, duplicate logic |
| `git add -A` | Stage unrelated files |
| Skip changelog | Undocumented work |
| Hardcode year | Breaks year picker |
| Slovak in code | ADR-004 violation |
```

### 2.4 Proposed Common Tasks Section

```markdown
## Common Tasks

### Adding a New Tauri Command

```rust
// 1. Add to src-tauri/src/commands.rs
#[tauri::command]
pub fn my_command(db: State<Database>, arg: String) -> Result<ReturnType, String> {
    // Implementation
}
```

```rust
// 2. Register in src-tauri/src/lib.rs invoke_handler
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    my_command,
])
```

```typescript
// 3. Call from frontend
const result = await invoke('my_command', { arg: 'value' });
```

### Adding a New Calculation

```rust
// 1. Write test in calculations.rs
#[test]
fn test_new_calculation() {
    assert_eq!(new_calculation(input), expected);
}

// 2. Implement in calculations.rs
pub fn new_calculation(input: Type) -> Output {
    // Minimal implementation
}

// 3. Expose via command in commands.rs
// 4. Call from frontend - receive pre-calculated value
```

### Adding UI Text

```typescript
// 1. src/lib/i18n/sk/index.ts (Slovak primary)
export default {
    newKey: 'Slovak text here',
};

// 2. src/lib/i18n/en/index.ts (English)
export default {
    newKey: 'English text here',
};

// 3. Use in Svelte
{LL.newKey()}
```
```

### 2.5 What to Remove/Move

| Content | Action | Reason |
|---------|--------|--------|
| Architecture diagram | Move to ARCHITECTURE.md | Already exists there |
| Full test count by file | Remove | Derivable, gets stale |
| CI/CD section | Remove | Lives in workflow files |
| Integration test details | Condense | Not needed every session |

---

## Part 3: Onboarding Experience Analysis

### 3.1 What a New Session Needs Immediately

**First 10 seconds:**
1. What is this project? (Vehicle logbook for Slovak legal compliance)
2. What tech stack? (Tauri + SvelteKit + Rust)
3. What's the core constraint? (ADR-008: backend-only calculations)

**First 30 seconds:**
1. How to run tests? (`npm run test:backend`)
2. What skills exist? (`/task-plan`, `/changelog`, `/decision`, `/release`)
3. What are the critical pitfalls?

**First 60 seconds:**
1. How to add a new feature? (Code pattern examples)
2. What's the completion checklist?

### 3.2 Current Gaps

| Gap | Impact | Solution |
|-----|--------|----------|
| No session start guidance | Agent misses context | Add to Quick Reference |
| Skills not linked to when | Agent doesn't know when to use | Add trigger descriptions |
| Permissions not documented | Unknown capabilities | Document in CLAUDE.md |
| No "start here" marker | Agent reads linearly | Restructure top-to-bottom |

### 3.3 Session Start Ideal Experience

When Claude starts a session, it should immediately see:

```
## This Session

Project: kniha-jazd (Vehicle logbook - Slovak legal compliance)
Core Rule: All calculations in Rust backend (ADR-008)
Workflow: TDD mandatory - test first, then implement

Quick Commands:
- Tests: npm run test:backend
- Dev: npm run tauri dev

Available Skills:
- /task-plan - Start new feature
- /changelog - Update after completing work (MANDATORY)
- /decision - Document architectural choices
- /release - Publish new version
```

---

## Part 4: Documentation Restructuring Proposal

### 4.1 File Responsibilities

| File | Purpose | Session Relevance |
|------|---------|-------------------|
| `CLAUDE.md` | Quick reference + critical constraints | Every session |
| `ARCHITECTURE.md` | Deep dive into system design | When needed |
| `DECISIONS.md` | ADR/BIZ decision log | When making decisions |
| `CONTRIBUTING.md` | External contributor guide | Not for Claude sessions |
| `.claude/skills/*/SKILL.md` | Skill-specific workflows | When skill invoked |

### 4.2 Proposed CLAUDE.md Length

**Target:** 150 lines (down from 264)

**Reduction strategy:**
- Move detailed architecture to ARCHITECTURE.md (saves ~30 lines)
- Remove CI/CD section (saves ~15 lines)
- Condense test coverage (saves ~20 lines)
- Remove duplicate info with DECISIONS.md (saves ~15 lines)
- Tighten prose throughout (saves ~30 lines)

### 4.3 Cross-Referencing Strategy

Add explicit pointers from CLAUDE.md:

```markdown
## More Information

- **Architecture details:** See `ARCHITECTURE.md`
- **ADR/BIZ decisions:** See `DECISIONS.md`
- **Task planning conventions:** See `_tasks/CLAUDE.md`
- **Skill workflows:** See `.claude/skills/`
```

### 4.4 Should There Be a CLAUDE-quickref.md?

**Considered:** Creating a separate `CLAUDE-quickref.md` for essential info.

**Decision:** NO - keep everything in CLAUDE.md but restructured.

**Reasoning:**
- Claude Code loads CLAUDE.md automatically
- Separate file requires explicit reading
- Better to restructure the main file with clear hierarchy

---

## Part 5: Redundancy Analysis

### 5.1 Overlap Between Files

| Topic | CLAUDE.md | ARCHITECTURE.md | DECISIONS.md |
|-------|-----------|-----------------|--------------|
| ADR-008 (backend calcs) | Full section | Full section | Entry exists |
| Architecture diagram | Yes | Yes (more detailed) | No |
| Business rules | Brief | Detailed | Full history |
| Test counts | Yes | Yes | No |

### 5.2 Proposed Deduplication

| Topic | Keep In | Remove From |
|-------|---------|-------------|
| ADR-008 | CLAUDE.md (short), DECISIONS.md (full) | ARCHITECTURE.md |
| Architecture diagram | ARCHITECTURE.md only | CLAUDE.md |
| Business rule formulas | ARCHITECTURE.md | CLAUDE.md (just reference) |
| Test counts | Neither (run tests to see) | Both |

---

## Part 6: Specific Improvements

### 6.1 Improve Common Pitfalls Visibility

**Current:** Buried at line 114, easy to miss.

**Proposed:** Move to top as "Critical Constraints" section with visual emphasis:

```markdown
## Critical Constraints

> **WARNING:** These are the most common mistakes. Read before coding.

| DO NOT | WHY |
|--------|-----|
| Add calculations to frontend | ADR-008 violation |
| Use `git add -A` | Stages unrelated files |
| Skip `/changelog` | Undocumented work |
| Hardcode year values | Breaks year picker |
| Use Slovak in code | ADR-004: code in English |
```

### 6.2 Improve Task Completion Checklist Visibility

**Current:** At the very bottom (line 254).

**Proposed:**
1. Keep at bottom (logical placement)
2. Reference at TOP in Quick Reference section
3. Add horizontal rule and visual emphasis

```markdown
---

## Before Claiming Complete

> **STOP.** Run this checklist before saying "done":

- [ ] `npm run test:backend` passes
- [ ] All code changes committed
- [ ] `/changelog` run to update [Unreleased]
- [ ] Changelog update committed

**For significant decisions:** Also run `/decision` to record ADR/BIZ entry.
```

### 6.3 Document Available Skills with Triggers

**Current:** Skills listed without clear when-to-use guidance.

**Proposed:**

```markdown
### Skills

| Skill | Trigger | What It Does |
|-------|---------|--------------|
| `/task-plan` | "I need to plan a feature" | Brainstorm, create _tasks folder |
| `/changelog` | After completing ANY work | Update CHANGELOG.md in Slovak |
| `/decision` | "Should I use X or Y?" | Add ADR/BIZ to DECISIONS.md |
| `/release` | "Ready to release" | Version bump, tag, build |

**MANDATORY:** Every completed feature/fix needs `/changelog` before done.
```

### 6.4 Document Permissions

**Current:** `.claude/settings.local.json` has permissions but not documented.

**Proposed:** Add to CLAUDE.md:

```markdown
### Allowed Superpowers Skills

The following external skills are pre-approved for this project:
- `superpowers:using-git-worktrees` - Create isolated feature branches
- `superpowers:systematic-debugging` - Debug with project context
- `superpowers:writing-skills` - Create/edit project skills
```

---

## Part 7: Implementation Plan

### Phase 1: Restructure CLAUDE.md (High Impact)

1. Add Quick Reference section at top
2. Move Critical Constraints up (after Quick Reference)
3. Add Common Tasks section with examples
4. Condense Detailed Reference section
5. Enhance Task Completion Checklist visibility
6. Remove redundant content

**Estimated reduction:** 264 lines -> ~150 lines

### Phase 2: Cross-Reference Cleanup (Medium Impact)

1. Remove architecture diagram from CLAUDE.md (keep in ARCHITECTURE.md)
2. Add explicit "More Information" section with file pointers
3. Update ARCHITECTURE.md if needed

### Phase 3: Skill Integration (Low Impact)

1. Document allowed superpowers skills in CLAUDE.md
2. Add skill trigger phrases to skill descriptions
3. Cross-reference skills in Quick Reference

---

## Part 8: Proposed New CLAUDE.md Structure

```markdown
# CLAUDE.md

Vehicle logbook (Kniha jazd) for Slovak legal compliance.

## Quick Reference

### Commands
{Table: dev, test, build}

### Skills
{Table: /task-plan, /changelog, /decision, /release with triggers}

### Before Claiming Complete
{Checklist preview - full version at end}

## Critical Constraints (MUST READ)

### ADR-008: Backend-Only Calculations
{Concise explanation}

### TDD Workflow
{The 4-step cycle}

### Common Pitfalls
{Table format - top 5}

## Common Tasks

### Adding a Tauri Command
{Copy-paste example}

### Adding a Calculation
{Copy-paste example}

### Adding UI Text
{Copy-paste example}

## Reference

### Tech Stack
{Brief bullets}

### Project Structure
{Tree, condensed}

### Key Files
{Table}

### Business Rules
{Brief - point to ARCHITECTURE.md for details}

### Git Guidelines
{Condensed}

## More Information

- Architecture: ARCHITECTURE.md
- Decisions: DECISIONS.md
- Task planning: _tasks/CLAUDE.md
- Skills: .claude/skills/

---

## Task Completion Checklist

{Full checklist with emphasis}
```

---

## Summary

### Key Recommendations

1. **Restructure CLAUDE.md top-to-bottom by priority** - Quick Reference first, detailed reference later
2. **Move Critical Constraints up** - ADR-008, TDD, Pitfalls should be immediately visible
3. **Add Common Tasks section** - Copy-paste examples reduce friction
4. **Reduce length 40%** - Target 150 lines from 264
5. **Remove duplicates** - Architecture diagram, test counts, CI details
6. **Reference checklist at top** - Don't hide completion requirements at bottom
7. **Document permissions** - Skills in settings.local.json should be visible

### Expected Outcomes

| Metric | Before | After |
|--------|--------|-------|
| Time to find test command | ~15 sec | ~5 sec |
| Time to find ADR-008 | ~20 sec | ~5 sec |
| Time to find pitfalls | ~30 sec | ~5 sec |
| Time to find completion checklist | ~60 sec | ~5 sec (referenced at top) |
| CLAUDE.md line count | 264 | ~150 |

### Files to Modify

| File | Change |
|------|--------|
| `CLAUDE.md` | Full restructure per proposal |
| `ARCHITECTURE.md` | Minor updates, ensure it has full diagram |
| `.claude/skills/*.md` | Add trigger phrases |

---

## References

- `01-analysis.md` - Gap analysis
- `02-hooks-proposal.md` - Hook specifications
- `03-skills-proposal.md` - Skill improvements
- Current `CLAUDE.md` - 264 lines to restructure
