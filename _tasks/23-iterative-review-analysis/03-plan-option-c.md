# Option C: Parameterized Review Skills - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a family of specialized review skills (`/plan-review`, `/code-review`, `/test-review`) that share a common base but have domain-specific configurations.

**Architecture:**
- Shared base template in `_templates/review-base.md`
- Three specialized skills, each with preset focus and quality gate
- Each skill imports base workflow but overrides parameters

**Tech Stack:** Claude Code skills (SKILL.md format) with shared template pattern

---

## Task 1: Create Shared Template Directory

**Files:**
- Create: `.claude/skills/_templates/review-base.md`

**Step 1: Create templates directory**

```bash
mkdir -p .claude/skills/_templates
```

**Step 2: Write the shared base template**

Create `.claude/skills/_templates/review-base.md`:

```markdown
# Review Base Template

This is a shared template for iterative review skills. Individual skills import this and override parameters.

## Shared Components

### Output Format

```
## Iteration N

**Date:** YYYY-MM-DD
**Reviewer:** {REVIEWER_TYPE}

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

### Review Agent Prompt Template

```
You are a {REVIEWER_TYPE} reviewing {TARGET}.

Focus: {FOCUS}
Reference: {REFERENCE}
Previous findings: {PRIOR_SUMMARY}

Your job:
1. Review everything fresh (don't trust prior reports)
2. Find NEW gaps not caught previously
3. Verify previous Critical/Important issues were fixed
4. Categorize findings:
   - Critical: Must fix (bugs, security, data loss)
   - Important: Should fix (missing features, poor handling)
   - Minor: Nice to have (style, docs)
5. Assess: Quality gate met?

{DOMAIN_SPECIFIC_INSTRUCTIONS}

Return structured findings using the output format.
```

### Anti-Patterns (All Reviews)

- Marking nitpicks as Critical
- Suggesting changes outside focus area
- Over-engineering beyond spec
- Trusting prior reports without verification
- Stopping early without quality gate check
```

**Step 3: Verify template created**

Run: `cat .claude/skills/_templates/review-base.md | head -10`

---

## Task 2: Create Plan Review Skill

**Files:**
- Create: `.claude/skills/plan-review-skill/SKILL.md`

**Step 1: Create skill directory**

```bash
mkdir -p .claude/skills/plan-review-skill
```

**Step 2: Write the plan-review skill**

Create `.claude/skills/plan-review-skill/SKILL.md`:

```markdown
---
name: plan-review-skill
description: Use to review plans, designs, or specifications for completeness and feasibility
---

# Plan Review Skill

Iterative review specialized for plans, designs, and specifications.

**Preset Configuration:**
- Focus: Plan completeness, feasibility, clarity
- Quality Gate: Early exit (no Critical/Important after 1 iteration)
- Reviewer Type: Plan Auditor
- Max Iterations: 4

## When to Use

- Reviewing implementation plans before coding
- Checking design documents for gaps
- Validating task specifications
- Assessing feasibility of proposed approaches

## Required Information

1. **Target** - path to plan/design document (e.g., `_tasks/15-feature/02-plan.md`)
2. **Reference** (optional) - requirements or constraints to compare against

## Workflow

### Step 1: Read Target Plan

Read the entire plan document to understand scope.

### Step 2: Create/Update Progress File

Create `{TARGET_DIR}/_plan-review.md`:

```markdown
# Plan Review

**Target:** {TARGET}
**Started:** YYYY-MM-DD
**Focus:** Completeness, feasibility, clarity
```

### Step 3: Execute Review Loop

For each iteration (max 4):

**Spawn Review Agent:**

```
Task tool (general-purpose):
  description: "Plan review iteration N"
  prompt: |
    You are a Plan Auditor reviewing {TARGET}.

    Your job is to assess this plan for:

    1. **Completeness**
       - Are all requirements addressed?
       - Are there missing tasks?
       - Are edge cases considered?

    2. **Feasibility**
       - Are the tasks achievable as described?
       - Are there hidden complexities?
       - Are dependencies identified?

    3. **Clarity**
       - Can an implementer follow this without questions?
       - Are file paths specific?
       - Are verification steps included?

    4. **YAGNI/DRY**
       - Is there unnecessary scope?
       - Is there duplication between tasks?

    Categorize findings as Critical/Important/Minor.
    Assess: Is this plan ready for implementation?
```

**Apply Fixes:** Update plan based on findings.

**Update Progress:** Append iteration to `_plan-review.md`.

**Commit:**
```bash
git add {TARGET_DIR}/
git commit -m "review(plan): iteration N for {PLAN_NAME}"
```

**Quality Gate:** Exit if no Critical/Important issues.

### Step 4: Final Assessment

```markdown
## Plan Review Complete

**Ready for implementation?** [Yes/No/With revisions]
**Confidence level:** [Low/Medium/High]

### Recommendations
[Specific next steps]
```

## Domain-Specific Checklist

- [ ] All requirements have corresponding tasks
- [ ] Tasks have specific file paths
- [ ] Verification steps are included
- [ ] Dependencies are in correct order
- [ ] No scope creep beyond requirements
- [ ] Complexity is appropriate (not over-engineered)

## Example

```
User: /plan-review _tasks/20-e2e-testing/02-plan.md
Claude: [Reviews plan for completeness, feasibility, clarity]
```
```

---

## Task 3: Create Code Review Skill

**Files:**
- Create: `.claude/skills/code-review-skill/SKILL.md`

**Step 1: Create skill directory**

```bash
mkdir -p .claude/skills/code-review-skill
```

**Step 2: Write the code-review skill**

Create `.claude/skills/code-review-skill/SKILL.md`:

```markdown
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
```

---

## Task 4: Create Test Review Skill

**Files:**
- Create: `.claude/skills/test-review-skill/SKILL.md`

**Step 1: Create skill directory**

```bash
mkdir -p .claude/skills/test-review-skill
```

**Step 2: Write the test-review skill**

Create `.claude/skills/test-review-skill/SKILL.md`:

```markdown
---
name: test-review-skill
description: Use to review test coverage for completeness and quality
---

# Test Review Skill

Iterative review specialized for test coverage analysis.

**Preset Configuration:**
- Focus: Test completeness, edge cases, quality
- Quality Gate: 2 consecutive iterations with no new issues (convergence)
- Reviewer Type: Test Coverage Analyst
- Max Iterations: 4

## When to Use

- After implementing a feature to verify test coverage
- Before release to check critical paths are tested
- When adding tests to existing code
- Periodic test health checks

## Required Information

1. **Target** - code module to check coverage for (e.g., `src-tauri/src/calculations.rs`)
2. **Reference** (optional) - business rules or plan defining what should be tested

## Workflow

### Step 1: Identify Test Files

Find tests for target:
```bash
# Rust
grep -r "test_" src-tauri/src/ --include="*.rs" | grep -i {module_name}

# Or check test modules
cat src-tauri/src/{module}.rs | grep -A5 "#\[cfg(test)\]"
```

### Step 2: Create Progress File

Create `{TARGET_DIR}/_test-review.md`:

```markdown
# Test Coverage Review

**Target:** {TARGET}
**Reference:** {REFERENCE}
**Started:** YYYY-MM-DD
**Focus:** Completeness, edge cases, test quality
```

### Step 3: Execute Review Loop

For each iteration (max 4):

**Spawn Review Agent:**

```
Task tool (general-purpose):
  description: "Test review iteration N"
  prompt: |
    You are a Test Coverage Analyst reviewing tests for {TARGET}.

    Reference (if provided): {REFERENCE}

    Your job:

    1. **Coverage Analysis**
       - What functions/methods exist in {TARGET}?
       - Which have corresponding tests?
       - Which are missing tests?

    2. **Edge Case Analysis**
       - Are boundary conditions tested?
       - Are error paths tested?
       - Are empty/null inputs tested?

    3. **Test Quality**
       - Do tests test actual logic (not mocks)?
       - Are assertions meaningful?
       - Are tests independent?

    4. **Business Logic**
       - Are calculation formulas verified?
       - Are business rules from {REFERENCE} tested?

    Focus on GAPS not on what's already well-tested.
    We don't want filler tests - only meaningful coverage.

    Categorize findings as Critical/Important/Minor.
```

**Apply Fixes:** Add missing Critical/Important tests.

**Run Tests:**
```bash
npm run test:backend
```

**Update Progress:** Append iteration.

**Commit:**
```bash
git add -A
git commit -m "test: add coverage for {TARGET} (iteration N)"
```

**Quality Gate (Convergence):**
- Track issues found per iteration
- Exit when 2 consecutive iterations find no new issues
- OR max iterations reached

### Step 4: Final Assessment

```markdown
## Test Review Complete

**Coverage assessment:** [Sparse/Adequate/Comprehensive]
**Exit reason:** [Convergence/Max iterations]

### Tests Added
[Summary of new tests]

### Remaining Gaps (if any)
[Minor items deferred]
```

## Domain-Specific Checklist

- [ ] Core business logic has tests
- [ ] Edge cases covered (empty, null, boundary)
- [ ] Error paths tested
- [ ] No tests that only test mocks
- [ ] Tests are independent (can run in any order)
- [ ] Tests have meaningful assertions

## Example

```
User: /test-review src-tauri/src/calculations.rs against DECISIONS.md
Claude: [Analyzes test coverage, identifies gaps, adds tests until convergence]
```
```

---

## Task 5: Verify All Skills

**Step 1: List all skills**

```bash
ls -la .claude/skills/
```

Expected output includes:
- `_templates/`
- `plan-review-skill/`
- `code-review-skill/`
- `test-review-skill/`

**Step 2: Verify each skill has valid SKILL.md**

```bash
for skill in plan-review code-review test-review; do
  echo "=== $skill ==="
  head -5 .claude/skills/${skill}-skill/SKILL.md
done
```

---

## Task 6: Commit All Skills

**Step 1: Stage all new files**

```bash
git add .claude/skills/_templates/
git add .claude/skills/plan-review-skill/
git add .claude/skills/code-review-skill/
git add .claude/skills/test-review-skill/
```

**Step 2: Commit**

```bash
git commit -m "$(cat <<'EOF'
feat: add parameterized review skills (Option C)

Three specialized iterative review skills:
- /plan-review - completeness, feasibility, clarity
- /code-review - quality, correctness, tests pass
- /test-review - coverage, edge cases, convergence-based

Shared template in _templates/review-base.md
Each skill has domain-specific:
- Focus area
- Quality gate
- Reviewer persona
- Checklist

Based on research in _tasks/23-iterative-review-analysis/

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Update CLAUDE.md Documentation

**Files:**
- Modify: `CLAUDE.md` (Documentation section)

**Step 1: Add skills to documentation table**

Add to the skill table in CLAUDE.md:

```markdown
| `/plan-review` | Plan/design review | Check completeness, feasibility before coding |
| `/code-review` | Code quality review | Review implementations with test validation |
| `/test-review` | Test coverage review | Identify gaps, add meaningful tests |
```

**Step 2: Commit documentation**

```bash
git add CLAUDE.md
git commit -m "docs: add review skills to CLAUDE.md"
```

---

## Verification Checklist

- [ ] `_templates/review-base.md` exists with shared components
- [ ] `/plan-review` skill exists and prompts for target
- [ ] `/code-review` skill exists and runs tests first
- [ ] `/test-review` skill uses convergence-based quality gate
- [ ] Each skill has domain-specific checklist
- [ ] All skills appear in `/` autocomplete
- [ ] CLAUDE.md updated with new skills

---

## Comparison: Option B vs Option C

| Aspect | Option B (Single Skill) | Option C (Parameterized) |
|--------|-------------------------|--------------------------|
| Invocation | `/iterative-review` | `/plan-review`, `/code-review`, `/test-review` |
| Configuration | User provides focus | Preset per domain |
| Quality Gate | User chooses | Optimized per domain |
| Flexibility | High (any focus) | Medium (3 presets) |
| Ease of Use | Requires parameters | Just target path |
| Maintenance | 1 file | 4 files |

**When to use Option C over B:**
- Team has standardized review workflows
- Want one-command invocation without configuration
- Different domains need different quality gates
- Want discoverable domain-specific checklists

---

## Rollback Plan

If skills don't work:

```bash
git log --oneline -3  # Find commit hash
git revert <commit-hash>  # Revert the skills commit
rm -rf .claude/skills/_templates/
rm -rf .claude/skills/plan-review-skill/
rm -rf .claude/skills/code-review-skill/
rm -rf .claude/skills/test-review-skill/
```
