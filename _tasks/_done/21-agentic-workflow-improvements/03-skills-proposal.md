**Date:** 2026-01-05
**Subject:** Skill Improvements & New Skills Proposal
**Status:** Proposal

## Overview

This document proposes improvements to existing skills and designs new skills to enhance the agentic workflow for kniha-jazd.

---

## Part 1: Analysis of Existing Skills

### 1.1 Skill Inventory

| Skill | Purpose | Strengths | Weaknesses |
|-------|---------|-----------|------------|
| `task-plan-skill` | Create planning folders | Integrates brainstorming, structured output | Missing cross-references to /decision, /changelog |
| `decision-skill` | Record ADR/BIZ decisions | Clear format, good examples | No link to task context, no integration prompts |
| `changelog-skill` | Update CHANGELOG.md | Simple workflow, Slovak language | No verification that work is actually complete |
| `release-skill` | Full release workflow | Comprehensive steps | No pre-release checklist, no test verification |

### 1.2 What Works Well

1. **Clear skill descriptions** - Each skill has a focused purpose
2. **Consistent format** - YAML frontmatter + markdown workflow
3. **Example-driven** - Good examples help understand usage
4. **Workflow steps** - Numbered steps are easy to follow
5. **Integration with superpowers** - `task-plan-skill` leverages brainstorming

### 1.3 What's Missing

1. **No cross-references** - Skills don't point to related skills
2. **No verification steps** - Skills assume work is complete, don't verify
3. **No workflow enforcement** - TDD not mentioned in any skill
4. **No project-specific context** - Skills don't reference ADR-008 or business rules
5. **No integration with proposed hooks** - Skills should work with hooks

### 1.4 Cross-Reference Opportunities

| Skill | Should Reference | Why |
|-------|------------------|-----|
| `task-plan-skill` | `/decision` | Architectural choices during planning |
| `task-plan-skill` | `/changelog` | Final step after implementation |
| `decision-skill` | Related task folder | Context for why decision was made |
| `changelog-skill` | Verification checklist | Ensure tests pass before claiming "complete" |
| `release-skill` | Pre-release checklist | ADR-008 compliance, all tests pass |

---

## Part 2: Proposed Skill Improvements

### 2.1 Enhanced task-plan-skill

Add these sections to existing skill:

```markdown
## Cross-References

**During planning, consider:**
- Use `/decision` if making architectural choices (ADR) or defining business rules (BIZ)
- Reference existing ADRs (especially ADR-008: backend-only calculations)

**After implementation, ensure:**
- Include `/changelog` as FINAL task in every plan
- Run `npm run test:backend` before marking complete

## Integration with Workflow Hooks

If hooks are enabled:
- Pre-commit hook will enforce tests pass before commit
- Post-commit hook will remind about changelog

## Example Plan Template (Enhanced)

```markdown
# {Feature} Implementation Plan

> **For Claude:** Use superpowers:executing-plans to implement.
> **Mandatory:** ADR-008 compliance - all calculations in Rust backend only.

**Goal:** {One-line summary}

---

## Task 1: Write failing tests

**Files:** `src-tauri/src/calculations.rs` (or appropriate test file)

**Steps:**
1. Write test for expected behavior
2. Run `npm run test:backend` - verify it fails

**Verification:** Test exists and fails for the right reason

---

## Task N: Update changelog

**Files:** `CHANGELOG.md`

**Steps:**
1. Run `/changelog` skill
2. Add entry under appropriate section (Pridane/Zmenene/Opravene)
3. Commit changelog update

**Verification:** [Unreleased] section updated
```
```

### 2.2 Enhanced decision-skill

Add context integration:

```markdown
## Linking Decisions to Context

When documenting a decision made during a task:

1. **Reference the task folder** (if applicable):
   ```markdown
   **Context:** During implementation of `_tasks/15-feature-name/`, we needed to choose...
   ```

2. **Cross-reference from task** - Add note in task file:
   ```markdown
   **Decisions:** See ADR-011 in `DECISIONS.md`
   ```

## Trigger Reminders

Consider using `/decision` when:
- During `/task-plan`: Architectural choices emerge
- During debugging: Non-obvious behavior discovered
- During code review: Alternative approaches discussed

## ADR-008 Compliance Check

When documenting calculation-related decisions, verify:
- Calculation lives in Rust backend? (ADR-008)
- No frontend duplication?
- Tests exist for the calculation?
```

### 2.3 Enhanced changelog-skill

Add verification step:

```markdown
## Pre-Changelog Verification

Before updating changelog, verify work is complete:

1. **Tests pass:** `npm run test:backend`
2. **Code committed:** `git status` shows no uncommitted changes (except CHANGELOG.md)
3. **No pending TODOs:** Work is actually finished

**WARNING:** Do not update changelog for incomplete work. The changelog represents DONE features.

## Integration with Hooks

After committing code:
- Post-commit hook displays changelog reminder
- This skill fulfills that reminder

## Commit Pattern

Option 1: Separate commit (preferred for clarity)
```bash
# After code commit
git add CHANGELOG.md
git commit -m "docs: update changelog for {feature}"
```

Option 2: Include with final code commit
```bash
git add CHANGELOG.md src/...
git commit -m "feat: {description}"
```
```

### 2.4 Enhanced release-skill

Add pre-release checklist:

```markdown
## 0. Pre-Release Checklist (NEW - Run First)

Before starting release:

### Tests
- [ ] `npm run test:backend` passes (105 tests)
- [ ] `npm run test:integration` passes (if applicable)

### Code Quality
- [ ] No uncommitted changes: `git status` is clean
- [ ] ADR-008 compliance: No calculations in frontend

### Documentation
- [ ] CHANGELOG.md [Unreleased] has entries
- [ ] Any new ADR/BIZ decisions documented

### Review
- [ ] All planned features for this release are complete
- [ ] No known critical bugs

**Proceed only if all checks pass.**

## Integration with Hooks

If pre-commit hook is enabled:
- Tests will automatically run before release commit
- Hook provides additional safety net
```

---

## Part 3: New Skills

### 3.1 TDD Enforcement Skill

**Purpose:** Verify test exists before implementing new functionality.

**File:** `.claude/skills/tdd-skill/SKILL.md`

```markdown
---
name: tdd-skill
description: Use before implementing any new feature or fixing bugs - ensures test-first approach per ADR-003
---

# TDD Enforcement Skill

Ensures Test-Driven Development workflow is followed for all code changes.

## When to Use

- **BEFORE** writing any new implementation code
- **BEFORE** fixing any bug (write failing test that reproduces it first)
- After planning phase, before coding phase
- When superpowers:executing-plans starts a new task

## Core Principle (ADR-003)

```
1. WRITE failing test first
2. WRITE minimal code to pass
3. REFACTOR while tests pass
4. REPEAT
```

## Workflow

### 1. Identify What to Test

Based on the feature/fix:

| Change Type | Test Location | What to Test |
|-------------|---------------|--------------|
| Calculation logic | `calculations.rs` | Input/output scenarios |
| Suggestion logic | `suggestions.rs` | Route matching, compensation |
| Database behavior | `db.rs` | Only if non-trivial (ADR-003 says skip CRUD) |
| Command behavior | `commands.rs` | Business logic in commands |
| Export format | `export.rs` | Output correctness |

### 2. Write Failing Test First

```rust
#[test]
fn test_new_feature_behavior() {
    // Arrange
    let input = ...;

    // Act
    let result = new_feature(input);

    // Assert
    assert_eq!(result, expected);
}
```

### 3. Verify Test Fails

```bash
npm run test:backend
```

**Expected:** Test should fail with meaningful error (not compile error).

If test passes immediately → either test is wrong or feature already exists.

### 4. Implement Minimal Code

Write just enough code to make the test pass:

```rust
fn new_feature(input: Type) -> Output {
    // Minimal implementation
}
```

### 5. Verify Test Passes

```bash
npm run test:backend
```

### 6. Refactor (Optional)

If code is messy, clean it up while tests pass.

## What NOT to Test (ADR-003)

Do not write filler tests for:
- Trivial CRUD operations
- Getters/setters
- UI rendering (unless behavior-critical)

Focus on **business logic** that matters for legal compliance.

## ADR-008 Compliance

All calculations MUST be in Rust backend:
- `src-tauri/src/calculations.rs` - Core math
- `src-tauri/src/suggestions.rs` - Compensation logic
- `src-tauri/src/commands.rs` - Command handlers

**NEVER** add calculations to frontend (`src/lib/`).

## Integration with Other Skills

- Follows `/task-plan` → Execute plan with TDD per task
- Precedes `/changelog` → Only update changelog after tests pass
- Required before `/release` → All tests must pass

## Quick Reference

```bash
# Run tests
npm run test:backend           # Rust unit tests
npm run test:all               # All tests

# Common test patterns
cargo test test_name           # Run specific test
cargo test -- --nocapture      # Show println output
```
```

---

### 3.2 Pre-Implementation Checklist Skill

**Purpose:** Ensure all preparation is done before coding starts.

**File:** `.claude/skills/pre-implementation-skill/SKILL.md`

```markdown
---
name: pre-implementation-skill
description: Use before starting any implementation - verifies planning is complete and branch is ready
---

# Pre-Implementation Checklist Skill

Ensures all preparation is complete before writing implementation code.

## When to Use

- After `/task-plan` creates planning docs
- Before `superpowers:executing-plans` starts
- When starting work on a new feature or fix
- After brainstorming is complete

## Checklist

### 1. Planning Complete?

- [ ] Task file exists (`_tasks/{NN}-{name}/01-task.md`)
- [ ] Plan file exists (`_tasks/{NN}-{name}/02-plan.md`)
- [ ] Requirements are clear and approved by user
- [ ] Plan has verification steps for each task

### 2. Architecture Decisions Made?

- [ ] ADR-008 understood: All calculations in Rust backend only
- [ ] Any new architectural choices documented with `/decision`?
- [ ] Reviewed relevant existing decisions in `DECISIONS.md`

### 3. Branch Strategy Decided?

Ask user:
> Should I create a feature branch for this work, or work on main?

Options:
- **Feature branch** (recommended for multi-session work):
  ```bash
  git checkout -b feature/{name}
  ```
- **Main branch** (OK for quick fixes)

### 4. Planning Docs Committed?

```bash
git add _tasks/{NN}-{name}/
git commit -m "docs: add task and plan for {feature-name}"
```

### 5. Test Environment Ready?

```bash
# Verify tests run
npm run test:backend

# Verify dev server works (optional)
npm run tauri dev
```

## Verification Script

Quick check before starting:

```bash
# Are there uncommitted changes?
git status

# Do current tests pass?
npm run test:backend

# Is planning folder present?
ls _tasks/
```

## Integration with Other Skills

| Phase | Skill |
|-------|-------|
| Before this | `/task-plan` (creates plan) |
| After this | TDD skill (write failing tests) |
| During impl | `/decision` (if choices arise) |
| After impl | `/changelog` (document changes) |

## Red Flags - Stop If:

- Requirements are unclear → Return to brainstorming
- No plan exists → Run `/task-plan` first
- Tests are broken → Fix tests before adding new code
- Uncommitted changes from another task → Commit or stash first
```

---

### 3.3 Project Debugging Skill

**Purpose:** Rust/Tauri-specific debugging patterns with ADR-008 compliance.

**File:** `.claude/skills/debug-skill/SKILL.md`

```markdown
---
name: debug-skill
description: Use when debugging issues - Rust/Tauri patterns and ADR-008 compliance verification
---

# Project-Specific Debugging Skill

Systematic debugging for kniha-jazd with focus on Rust backend and ADR-008 compliance.

## When to Use

- Bug reports or unexpected behavior
- Test failures
- IPC communication issues
- Calculation discrepancies

**Prefer this over** `superpowers:systematic-debugging` for project-specific context.

## Step 1: Classify the Bug

| Symptom | Likely Location | Check |
|---------|-----------------|-------|
| Wrong calculation | `calculations.rs` | Unit tests |
| Wrong UI data | `commands.rs` → frontend | IPC return values |
| Database issues | `db.rs` | SQL queries |
| Suggestion problems | `suggestions.rs` | Route matching |
| Export issues | `export.rs` | HTML generation |

## Step 2: Check ADR-008 Compliance

Before debugging, verify the bug isn't caused by architecture violation:

```
Question: Is there calculation logic in the frontend?
Location: src/lib/
Pattern: Any math operations on trip data
```

If found → This is the bug. Move calculation to backend.

## Step 3: Find Relevant Tests

```bash
# Search for related tests
cd src-tauri
cargo test {keyword} -- --nocapture

# Run all tests in a module
cargo test calculations::
cargo test suggestions::
```

## Step 4: Add Debugging Test

Write a test that reproduces the bug:

```rust
#[test]
fn test_bug_reproduction_issue_xxx() {
    // Setup: Exact conditions from bug report
    let input = ...;

    // Act
    let result = buggy_function(input);

    // Assert: What we expect (will fail initially)
    assert_eq!(result, expected);
}
```

## Step 5: Debug with Print Statements

Rust debugging in tests:

```rust
#[test]
fn test_debug_example() {
    let value = calculate_something();
    println!("DEBUG: value = {:?}", value);  // Visible with --nocapture

    // or use dbg! macro
    dbg!(&value);
}
```

Run with output:
```bash
cargo test test_debug_example -- --nocapture
```

## Step 6: Tauri-Specific Debugging

### IPC Issues

Frontend call:
```typescript
const result = await invoke("command_name", { args });
console.log("IPC result:", result);
```

Backend logging:
```rust
#[tauri::command]
fn command_name(args: Type) -> Result<Output, String> {
    println!("command_name called with: {:?}", args);
    // ...
}
```

### Database Issues

```rust
// In db.rs, add query logging
fn get_trips(conn: &Connection, vehicle_id: i64) -> Result<Vec<Trip>> {
    println!("Query: SELECT * FROM trips WHERE vehicle_id = {}", vehicle_id);
    // ...
}
```

## Step 7: Fix and Verify

1. Implement fix
2. Run reproduction test → should pass now
3. Run full test suite → no regressions
4. Manual verification if needed

```bash
npm run test:backend
npm run tauri dev  # Manual check
```

## Step 8: Document if Non-Obvious

If debugging revealed non-obvious behavior:
- Use `/decision` to document as BIZ-NNN
- Example: BIZ-010 (retroactive rate application) came from debugging

## Common Debugging Patterns

### Calculation Discrepancy
```bash
# Compare with expected Excel output
cargo test test_excel_verification -- --nocapture
```

### Margin Calculation Issues
```bash
cargo test margin -- --nocapture
cargo test legal_limit -- --nocapture
```

### Zostatok (Fuel Remaining) Issues
```bash
cargo test zostatok -- --nocapture
cargo test remaining -- --nocapture
```

## Integration with Workflow

After fixing:
1. Ensure reproduction test passes
2. Run full test suite
3. Commit fix with descriptive message
4. Run `/changelog` to document fix
```

---

### 3.4 Code Review Skill

**Purpose:** Check code against project ADRs and business rules before commit.

**File:** `.claude/skills/review-skill/SKILL.md`

```markdown
---
name: review-skill
description: Use before committing or when reviewing changes - checks ADR compliance and business rules
---

# Code Review Skill

Reviews code changes against project architectural decisions and business rules.

## When to Use

- Before committing significant changes
- After implementing a feature
- When reviewing PR or changes from another session
- As part of `superpowers:verification-before-completion`

## Review Checklist

### 1. ADR-008: Backend-Only Calculations

**Check:** No calculation logic in frontend

```
Files to scan: src/lib/**/*.ts, src/lib/**/*.svelte
Patterns to flag:
- Mathematical operations on trip data
- Consumption rate calculations
- Margin calculations
- Zostatok calculations
```

If found → Move to `src-tauri/src/calculations.rs`

### 2. ADR-003: TDD Compliance

**Check:** Tests exist for new business logic

Questions:
- Did new calculations get added? → Tests in `calculations.rs`?
- New suggestion logic? → Tests in `suggestions.rs`?
- Business rule changes? → Tests verify new behavior?

### 3. ADR-004: Code Language

**Check:** Code in English, UI in Slovak

```
Pattern to flag: Slovak variable names, function names, comments
OK: Slovak strings in i18n files
```

### 4. Business Rules Compliance

| Rule | Check |
|------|-------|
| BIZ-001 | Consumption = liters / km * 100 |
| BIZ-002 | Rate carries from last fill-up |
| BIZ-003 | Legal limit = TP rate * 1.2 |
| BIZ-010 | Rate applies retroactively to all trips since last fill-up |
| BIZ-011 | Legal check uses average consumption, not single fill-up |

### 5. Test Coverage

```bash
# Run tests to verify nothing broke
npm run test:backend

# Check test count didn't decrease
# Expected: 105+ tests
```

### 6. Changelog Prepared?

If feature/fix is complete:
- [ ] `/changelog` entry drafted
- [ ] Entry matches what was changed
- [ ] Entry is in Slovak

## Review Output Template

```markdown
## Code Review Summary

### ADR Compliance
- [ ] ADR-008: No frontend calculations
- [ ] ADR-003: Tests exist for business logic
- [ ] ADR-004: Code in English, UI in Slovak

### Business Rules
- [ ] Calculations match BIZ-001 through BIZ-011

### Test Results
- Tests: PASS/FAIL (X tests)
- Coverage: {areas covered}

### Issues Found
1. {Issue description} - {File:line}

### Recommendations
1. {Recommendation}
```

## Integration with Other Skills

| Phase | Skill |
|-------|-------|
| Before review | Implementation complete |
| During review | This skill |
| After review | Fix issues, then `/changelog` |
| If decisions made | `/decision` to document |

## Self-Review vs Peer Review

**Self-review (before commit):**
- Quick checklist
- Focus on ADR-008 and tests

**Peer review (PR or handoff):**
- Full checklist
- Document findings in PR comments or task file
```

---

### 3.5 Verification Skill

**Purpose:** Run before marking work complete to ensure quality.

**File:** `.claude/skills/verify-skill/SKILL.md`

```markdown
---
name: verify-skill
description: Use before claiming work is complete - runs tests, checks changelog, verifies quality
---

# Verification Before Completion Skill

Final verification before marking any work as complete.

## When to Use

- Before saying "task complete" or "feature done"
- Before creating a PR
- Before final commit of a task
- As final step in `superpowers:executing-plans`

**MANDATORY:** Never claim completion without running this skill.

## Verification Checklist

### 1. Tests Pass

```bash
npm run test:backend
```

**Expected output:**
```
test result: ok. 105 passed; 0 failed; 0 ignored
```

Do NOT proceed if tests fail.

### 2. No Uncommitted Changes

```bash
git status
```

**Expected:** All work-related files committed.

If uncommitted:
```bash
git add {specific files}
git commit -m "{descriptive message}"
```

### 3. Changelog Updated

Check CHANGELOG.md [Unreleased] section:

```bash
head -30 CHANGELOG.md
```

If not updated:
- Run `/changelog` skill
- Add entry under appropriate section
- Commit changelog

### 4. Decisions Documented

Ask yourself:
- Were any architectural choices made? → `/decision` ADR-NNN
- Were any business rules defined? → `/decision` BIZ-NNN

### 5. Manual Verification (If Applicable)

For UI changes:
```bash
npm run tauri dev
```

Verify the feature works as expected.

## Verification Script

Run all checks at once:

```powershell
# Quick verification script
Write-Host "=== Verification Checklist ==="

# 1. Tests
Write-Host "`n[1/4] Running tests..."
Push-Location src-tauri
cargo test
$testResult = $LASTEXITCODE
Pop-Location

if ($testResult -ne 0) {
    Write-Host "FAIL: Tests failed" -ForegroundColor Red
    exit 1
}
Write-Host "PASS: Tests passed" -ForegroundColor Green

# 2. Git status
Write-Host "`n[2/4] Checking git status..."
git status --porcelain
if ($LASTEXITCODE -ne 0) {
    Write-Host "WARNING: Uncommitted changes" -ForegroundColor Yellow
}

# 3. Changelog
Write-Host "`n[3/4] Checking changelog..."
$unreleased = Select-String -Path "CHANGELOG.md" -Pattern "## \[Unreleased\]" -Context 0,5
Write-Host $unreleased.Context.PostContext

# 4. Summary
Write-Host "`n=== Verification Complete ==="
```

## Failure Handling

| Check | If Failed | Action |
|-------|-----------|--------|
| Tests | Fix failing tests | Do not proceed |
| Uncommitted | Commit or explain | Stage and commit |
| Changelog | Missing entry | Run `/changelog` |
| Decisions | Missing docs | Run `/decision` |

## Integration with Hooks

If hooks are enabled:
- Pre-commit hook runs tests automatically
- Post-commit hook reminds about changelog
- This skill verifies everything together

## Task Completion Template

After all checks pass:

```markdown
## Task Complete

### Summary
{What was implemented/fixed}

### Files Changed
- `path/to/file1.rs` - {change description}
- `path/to/file2.svelte` - {change description}

### Tests
- X tests passing
- New tests added: {list}

### Changelog
- Entry added under: Pridane/Zmenene/Opravene

### Commits
- `{hash}` - {message}
- `{hash}` - docs: update changelog
```

## Integration with Other Skills

This skill should be the FINAL step before:
- Marking TodoWrite task complete
- Responding "task is done"
- Creating PR
- Moving to next task
```

---

## Part 4: Skill Integration Map

### 4.1 Workflow Integration

```
┌─────────────────────────────────────────────────────────────┐
│                    PLANNING PHASE                            │
├─────────────────────────────────────────────────────────────┤
│  /task-plan ──→ brainstorming ──→ writing-plans             │
│       │                                                      │
│       └──→ /decision (if architectural choices)              │
│       └──→ pre-implementation-skill (checklist)              │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  IMPLEMENTATION PHASE                        │
├─────────────────────────────────────────────────────────────┤
│  For each task:                                              │
│    tdd-skill ──→ Write failing test                          │
│         │                                                    │
│         └──→ Implement minimal code                          │
│         └──→ Verify test passes                              │
│         └──→ Refactor                                        │
│                                                              │
│  During work:                                                │
│    debug-skill ──→ If issues arise                           │
│    /decision ──→ If choices need documentation               │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  COMPLETION PHASE                            │
├─────────────────────────────────────────────────────────────┤
│  review-skill ──→ ADR compliance check                       │
│       │                                                      │
│       └──→ verify-skill (final checks)                       │
│                 │                                            │
│                 └──→ /changelog (document changes)           │
│                 └──→ Commit                                  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    RELEASE PHASE                             │
├─────────────────────────────────────────────────────────────┤
│  /release ──→ Pre-release checklist                          │
│       │                                                      │
│       └──→ Version bump, changelog, tag, build               │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 Hook Integration

| Hook | Skills Involved |
|------|-----------------|
| Pre-commit (Bash) | Enforces `tdd-skill` test verification |
| Post-commit (Bash) | Reminds to run `/changelog` |
| Notification (idle) | Shows `verify-skill` checklist |

### 4.3 Skill Trigger Summary

| Skill | Trigger Phrase | Auto-Triggered By |
|-------|----------------|-------------------|
| `/task-plan` | "plan", "new feature" | Manual |
| `/decision` | "decide", "architecture" | During planning/debugging |
| `/changelog` | "changelog", "update log" | Post-commit hook reminder |
| `/release` | "release", "/release" | Manual |
| `tdd-skill` | Before implementing | Plan execution |
| `pre-implementation-skill` | After planning | Before executing-plans |
| `debug-skill` | Bug, failure, issue | When debugging |
| `review-skill` | Before commit | Before verify-skill |
| `verify-skill` | Before "complete" | Final step of any task |

---

## Part 5: Implementation Priority

### Phase 1: Essential (Immediate)

1. **verify-skill** - Most impactful, prevents incomplete work
2. **tdd-skill** - Enforces core project principle
3. Update existing skills with cross-references

### Phase 2: Important (Short-term)

4. **review-skill** - ADR compliance checking
5. **debug-skill** - Project-specific debugging
6. **pre-implementation-skill** - Complete workflow coverage

### Phase 3: Enhancement (Later)

7. Integrate skills with hooks from `02-hooks-proposal.md`
8. Add skill usage analytics/tracking
9. Create skill documentation index

---

## Part 6: Files to Create

| File | Purpose | Priority |
|------|---------|----------|
| `.claude/skills/verify-skill/SKILL.md` | Verification before completion | P1 |
| `.claude/skills/tdd-skill/SKILL.md` | TDD enforcement | P1 |
| `.claude/skills/review-skill/SKILL.md` | Code review checklist | P2 |
| `.claude/skills/debug-skill/SKILL.md` | Project debugging | P2 |
| `.claude/skills/pre-implementation-skill/SKILL.md` | Pre-impl checklist | P2 |

### Existing Files to Modify

| File | Changes | Priority |
|------|---------|----------|
| `.claude/skills/task-plan-skill/SKILL.md` | Add cross-references, TDD mention | P1 |
| `.claude/skills/decision-skill/SKILL.md` | Add context linking | P1 |
| `.claude/skills/changelog-skill/SKILL.md` | Add verification step | P1 |
| `.claude/skills/release-skill/SKILL.md` | Add pre-release checklist | P1 |

---

## References

- `01-analysis.md` - Gap analysis and initial recommendations
- `02-hooks-proposal.md` - Hook specifications
- `CLAUDE.md` - Project instructions and requirements
- `DECISIONS.md` - Existing ADR/BIZ decisions
