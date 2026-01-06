---
name: plan-review-skill
description: Use to review plans, designs, or specifications for completeness and feasibility
---

# Plan Review Skill

Two-phase review: First analyze and document findings, then apply approved changes.

**Preset Configuration:**
- Focus: Plan completeness, feasibility, clarity
- Quality Gate: No new findings for 1 iteration (review comprehensive)
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

---

## Phase 1: Review (Findings Only)

### Step 1: Read Target Plan

Read the entire plan document to understand scope.

### Step 2: Create Review Document

Create `{TARGET_DIR}/_plan-review.md`:

```markdown
# Plan Review

**Target:** {TARGET}
**Started:** YYYY-MM-DD
**Status:** In Progress
**Focus:** Completeness, feasibility, clarity

## Iteration 1

### Findings

[To be filled by review agent]
```

### Step 3: Execute Review Loop

For each iteration (max 4):

**Spawn Review Agent:**

```
Task tool (general-purpose):
  description: "Plan review iteration N"
  prompt: |
    You are a Plan Auditor reviewing {TARGET}.

    Previous findings (if any): [summary from _plan-review.md]

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
    Note any NEW findings not in previous iterations.
    Assess: Is the review comprehensive or are there areas unexplored?
```

**Update Review Document:** Append findings to `_plan-review.md`:
```markdown
## Iteration N

### New Findings
- [Critical] ...
- [Important] ...
- [Minor] ...

### Refined Analysis
[Any updates to previous findings]

### Coverage Assessment
[Areas reviewed / Areas remaining]
```

**Commit Review Only:**
```bash
git add {TARGET_DIR}/_plan-review.md
git commit -m "review(plan): iteration N findings for {PLAN_NAME}"
```

**Quality Gate:** Exit when no new findings for 1 iteration (review is comprehensive).

### Step 4: Finalize Review Document

Update `_plan-review.md` with summary:

```markdown
## Review Summary

**Status:** Ready for User Review
**Iterations:** N
**Total Findings:** X Critical, Y Important, Z Minor

### All Findings (Consolidated)

#### Critical
1. [ ] Finding description - Location/context

#### Important
1. [ ] Finding description - Location/context

#### Minor
1. [ ] Finding description - Location/context

### Recommendation
[Ready for implementation / Needs revisions / Major rework needed]
```

**Commit:**
```bash
git add {TARGET_DIR}/_plan-review.md
git commit -m "review(plan): complete findings for {PLAN_NAME}"
```

### Step 5: Present Review for Approval

Inform user:

> **Plan review complete.**
>
> Please review `{TARGET_DIR}/_plan-review.md` for findings.
>
> After your review, let me know:
> - Which findings to address
> - Which to skip (with reason)
> - Any questions about findings

**STOP and wait for user direction.**

---

## Phase 2: Apply Approved Changes

*Only proceed after user approval.*

### Step 6: Apply Approved Fixes

For each user-approved finding:

1. Update the source plan to address the finding
2. Check the finding as addressed in `_plan-review.md`: `[x]`

### Step 7: Commit Changes

```bash
git add {TARGET_DIR}/
git commit -m "plan: apply review feedback for {PLAN_NAME}

Addressed:
- [list of addressed findings]"
```

### Step 8: Final Assessment

Update `_plan-review.md`:

```markdown
## Resolution

**Addressed:** N findings
**Skipped:** M findings (user decision)
**Status:** Complete

### Applied Changes
- Finding 1: [how resolved]
- Finding 2: [how resolved]

### Skipped Items
- Finding X: [user's reason]
```

---

## Domain-Specific Checklist

Review should verify:
- [ ] All requirements have corresponding tasks
- [ ] Tasks have specific file paths
- [ ] Verification steps are included
- [ ] Dependencies are in correct order
- [ ] No scope creep beyond requirements
- [ ] Complexity is appropriate (not over-engineered)

## Example

```
User: /plan-review _tasks/20-e2e-testing/02-plan.md

Claude: [Executes Phase 1 - creates _plan-review.md, iterates until comprehensive]
Claude: Plan review complete. Please review _plan-review.md for findings.

User: Address all Critical and Important. Skip Minor items.

Claude: [Executes Phase 2 - applies approved changes to plan]
```
