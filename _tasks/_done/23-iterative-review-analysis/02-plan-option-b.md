# Option B: Dedicated Iterative Review Skill - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a single `/iterative-review` skill that provides structured multi-iteration review with quality gates.

**Architecture:** One skill folder with SKILL.md containing the full template from analysis. User invokes with `/iterative-review`, then provides target and focus when prompted.

**Tech Stack:** Claude Code skill (SKILL.md format)

---

## Task 1: Create Skill Folder Structure

**Files:**
- Create: `.claude/skills/iterative-review-skill/SKILL.md`

**Step 1: Create the skill directory**

```bash
mkdir -p .claude/skills/iterative-review-skill
```

**Step 2: Verify directory exists**

Run: `ls .claude/skills/`
Expected: `iterative-review-skill` appears in output

---

## Task 2: Write the SKILL.md File

**Files:**
- Create: `.claude/skills/iterative-review-skill/SKILL.md`

**Step 1: Write the skill content**

Create `.claude/skills/iterative-review-skill/SKILL.md` with this exact content:

```markdown
---
name: iterative-review-skill
description: Use for multi-iteration review of plans, code, or tests with structured quality gates
---

# Iterative Review Skill

Multi-pass review workflow with severity categorization and early-exit quality gates.

## When to Use

- Reviewing plans or designs for completeness
- Code review for quality and correctness
- Test coverage review
- Security or edge-case analysis
- Any review needing multiple passes with structured output

## Required Information

Before starting, you need:
1. **Target** - folder/file path to review (e.g., `_tasks/22-test-completeness/`)
2. **Focus** - what aspect to review (e.g., "test completeness", "security", "performance")
3. **Reference** (optional) - plan/spec to compare against

If not provided, ask user for these before proceeding.

## Workflow

### Step 1: Confirm Parameters

```
Iterative Review Configuration:
- Target: {TARGET}
- Focus: {FOCUS}
- Reference: {REFERENCE or "None"}
- Max iterations: 4
- Quality gate: No Critical or Important issues

Proceed? (y/n)
```

### Step 2: Create Progress File

Create `{TARGET}/_review.md` if it doesn't exist:

```markdown
# Iterative Review: {FOCUS}

**Target:** {TARGET}
**Started:** YYYY-MM-DD
**Status:** In Progress
```

### Step 3: Execute Review Loop

For each iteration (1 to 4, or until quality gate met):

**3a. Spawn Review Agent**

```
Task tool (general-purpose):
  description: "Review iteration N for {TARGET}"
  prompt: |
    You are reviewing {TARGET} for iteration N.

    Focus: {FOCUS}
    Reference: {REFERENCE}
    Previous findings: [summary from _review.md]

    Your job:
    1. Review everything fresh (don't trust prior reports)
    2. Find NEW gaps not caught previously
    3. Verify previous Critical/Important issues were fixed
    4. Categorize findings:
       - Critical: Must fix (bugs, security, data loss)
       - Important: Should fix (missing features, poor handling)
       - Minor: Nice to have (style, docs)
    5. Assess: Quality gate met?

    Return structured findings.
```

**3b. Apply Fixes**

- Fix all Critical issues immediately
- Address Important issues
- Note Minor issues for later

**3c. Update Progress File**

Append to `{TARGET}/_review.md`:

```markdown
---

## Iteration N

**Date:** YYYY-MM-DD
**Reviewer:** Review Agent

### Critical Issues
[List or "None found"]

### Important Issues
[List or "None found"]

### Minor Issues
[List - may defer]

### Changes Made
- [Bullet list of fixes]

### Assessment
**Quality:** [Poor/Acceptable/Good/Excellent]
**Continue?** [Yes/No - quality gate met]
```

**3d. Commit Changes**

```bash
git add -A
git commit -m "review({FOCUS}): iteration N complete"
```

**3e. Check Quality Gate**

- If no Critical and no Important issues: **EXIT LOOP** (quality gate met)
- If max iterations reached: Summarize remaining issues
- Otherwise: Continue to next iteration

### Step 4: Final Summary

After loop completes, provide summary:

```markdown
## Review Complete

**Iterations:** N of 4
**Exit reason:** [Quality gate met / Max iterations]
**Final quality:** [Assessment]

### Remaining Issues
[Any Minor issues not addressed]

### Recommendations
[Any follow-up work suggested]
```

## Quality Gate Options

Default is "early exit" - stop when no Critical/Important issues.

For different use cases, adjust:

| Use Case | Quality Gate |
|----------|--------------|
| Plan review | Early exit (default) |
| Code review | Tests pass + early exit |
| Test coverage | 2 consecutive clean iterations |
| Security review | Minimum 2 iterations + early exit |

## Anti-Patterns to Avoid

- Marking nitpicks as Critical
- Suggesting changes outside {FOCUS}
- Over-engineering beyond spec
- Trusting prior reports without verification
- Stopping early without explicit quality gate check

## Example Invocation

```
User: /iterative-review
Claude: What would you like to review?
User: Review _tasks/22-test-completeness for test coverage gaps
Claude: [Executes workflow with TARGET=_tasks/22-test-completeness, FOCUS=test coverage]
```
```

**Step 2: Verify file was created correctly**

Run: `head -20 .claude/skills/iterative-review-skill/SKILL.md`
Expected: Frontmatter with `name: iterative-review-skill`

---

## Task 3: Test the Skill

**Files:**
- Read: `.claude/skills/iterative-review-skill/SKILL.md`

**Step 1: Verify skill is recognized**

In a new Claude Code session, type `/iterative-review` and verify it appears in skill list.

**Step 2: Dry-run test**

Invoke the skill and verify it:
1. Asks for target and focus
2. Creates `_review.md` structure
3. Spawns review agent correctly

---

## Task 4: Commit the Skill

**Step 1: Stage files**

```bash
git add .claude/skills/iterative-review-skill/
```

**Step 2: Commit**

```bash
git commit -m "$(cat <<'EOF'
feat: add iterative-review skill (Option B)

Single-purpose skill for multi-iteration review with:
- Severity categorization (Critical/Important/Minor)
- Early-exit quality gate
- Structured progress tracking via _review.md
- Configurable for plan/code/test/security review

Based on research in _tasks/23-iterative-review-analysis/

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Update CLAUDE.md (Optional)

**Files:**
- Modify: `CLAUDE.md` (Documentation section)

**Step 1: Add skill to documentation table**

Add to the skill table in CLAUDE.md:

```markdown
| `/iterative-review` | Multi-pass review | Review plans/code/tests with quality gates |
```

**Step 2: Commit documentation update**

```bash
git add CLAUDE.md
git commit -m "docs: add iterative-review skill to CLAUDE.md"
```

---

## Verification Checklist

- [ ] Skill folder exists at `.claude/skills/iterative-review-skill/`
- [ ] SKILL.md has correct frontmatter (name, description)
- [ ] Skill appears when typing `/iterative-review`
- [ ] Workflow prompts for target and focus
- [ ] Review agent spawns correctly
- [ ] Progress tracked in `_review.md`
- [ ] Quality gate exits early when met
- [ ] Commits after each iteration

---

## Rollback Plan

If skill doesn't work:

```bash
git log --oneline -3  # Find commit hash
git revert <commit-hash>  # Revert the skill commit
```
