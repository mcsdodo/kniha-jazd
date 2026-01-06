---
name: code-review-skill
description: Use to review code implementations for quality, correctness, and best practices
---

# Code Review Skill

Iterative review specialized for code implementations.

**Preset Configuration:**
- Focus: Code quality, correctness, best practices
- Quality Gate: Tests pass + no Critical/Important issues
- Reviewer Type: Senior Code Reviewer
- Max Iterations: 4

## When to Use

- After completing implementation of a feature
- Before creating a pull request
- When refactoring significant code
- Post-merge quality verification

## Required Information

1. **Target** - code path or git range (e.g., `src-tauri/src/calculations.rs` or `HEAD~3..HEAD`)
2. **Reference** - plan or spec the code implements

## Workflow

### Step 1: Gather Code Context

If git range provided:
```bash
git diff --stat {BASE}..{HEAD}
git diff {BASE}..{HEAD}
```

If file path provided:
- Read the file(s)
- Identify recent changes

### Step 2: Run Tests First

```bash
npm run test:backend
```

If tests fail, note failures for review context.

### Step 3: Create Progress File

Create `_code-review.md` in project root or task folder:

```markdown
# Code Review

**Target:** {TARGET}
**Reference:** {REFERENCE}
**Started:** YYYY-MM-DD
**Focus:** Quality, correctness, best practices
```

### Step 4: Execute Review Loop

For each iteration (max 4):

**Spawn Review Agent:**

```
Task tool (superpowers:code-reviewer):
  Use template from requesting-code-review/code-reviewer.md

  WHAT_WAS_IMPLEMENTED: {description from reference}
  PLAN_OR_REQUIREMENTS: {REFERENCE}
  BASE_SHA: {BASE or N/A}
  HEAD_SHA: {HEAD or current}
```

**Apply Fixes:** Fix Critical issues, address Important.

**Run Tests Again:**
```bash
npm run test:backend
```

**Update Progress:** Append iteration.

**Commit:**
```bash
git add -A
git commit -m "review(code): iteration N fixes"
```

**Quality Gate:** Exit if tests pass AND no Critical/Important issues.

### Step 5: Final Assessment

```markdown
## Code Review Complete

**Ready to merge?** [Yes/No/With fixes]
**Test status:** [All passing / N failures]

### Remaining Items
[Any Minor issues deferred]
```

## Domain-Specific Checklist

- [ ] Tests pass
- [ ] No obvious bugs
- [ ] Error handling present
- [ ] No security vulnerabilities (input validation, etc.)
- [ ] Code matches plan/spec
- [ ] No scope creep (extra features not requested)
- [ ] Follows project patterns (see CLAUDE.md)

## Example

```
User: /code-review src-tauri/src/suggestions.rs against _tasks/19-electric/02-plan.md
Claude: [Runs tests, reviews code against plan, iterates until quality gate met]
```
